// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Process lifecycle management for the PTY multiplexer with per-process buffers.
//!
//! This module implements true terminal multiplexing where each process maintains
//! its own virtual terminal (`OffscreenBuffer`) and ANSI parser. Process switching
//! is instant with no delays or hacks - just display a different buffer.

use std::fmt::{Debug, Formatter, Result};

use portable_pty::PtySize;

use super::output_renderer::STATUS_BAR_HEIGHT;
use crate::{OffscreenBuffer, Size,
            core::{osc::OscEvent,
                   pty::{PtyCommandBuilder, PtyInputEvent, PtyReadWriteOutputEvent,
                         PtyReadWriteSession}},
            height};

/// Manages multiple PTY processes and handles switching between them.
#[derive(Debug)]
pub struct ProcessManager {
    processes: Vec<Process>,
    active_index: usize,
    terminal_size: Size,
}

/// Represents a single process that can be managed by the multiplexer.
///
/// Each process maintains its own virtual terminal state through an [`OffscreenBuffer`]
/// and [`ANSI parser`](vte::Parser), enabling true terminal multiplexing where switching
/// between processes is instant and preserves the complete terminal state.
pub struct Process {
    /// Display name for this process (shown in status bar)
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Command line arguments
    pub args: Vec<String>,
    /// Optional PTY session (None if not yet spawned)
    session: Option<PtyReadWriteSession>,
    /// Whether the process is currently running
    is_running: bool,
    /// Virtual terminal buffer for this process (per-process buffer architecture)
    ofs_buf: OffscreenBuffer,

    /// Tracks if this process has unrendered output since last render
    has_unrendered_output: bool,
    /// Terminal title set by OSC sequences (None if not set)
    pub terminal_title: Option<String>,
}

impl Process {
    /// Create a new process definition with virtual terminal buffer.
    ///
    /// The buffer is sized to (height-1, width) to reserve space for the status bar.
    /// Each process gets its own virtual terminal that persists when switching.
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        terminal_size: Size,
    ) -> Self {
        // Reserve bottom row for status bar - buffer gets reduced height.
        let buffer_size = Size {
            row_height: height(
                terminal_size.row_height.saturating_sub(STATUS_BAR_HEIGHT),
            ),
            col_width: terminal_size.col_width,
        };

        Self {
            name: name.into(),
            command: command.into(),
            args,
            session: None,
            is_running: false,
            ofs_buf: OffscreenBuffer::new_empty(buffer_size),

            has_unrendered_output: false,
            terminal_title: None,
        }
    }

    /// Returns whether this process is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool { self.is_running }

    /// Update the process's virtual terminal buffer with new PTY output.
    ///
    /// This is the core of the per-process virtual terminal architecture:
    /// Each process maintains its own complete terminal state through an
    /// `OffscreenBuffer`. Raw PTY bytes are processed through the ANSI parser and
    /// converted into `PixelChar` updates in the virtual terminal buffer.
    ///
    /// This allows each process to maintain its complete screen state independently,
    /// enabling instant switching without any delays or resizing tricks.
    pub fn process_pty_output_and_update_buffer(&mut self, output: Vec<u8>) {
        if !output.is_empty() {
            // Process bytes and extract any OSC and DSR events.
            let (osc_events, dsr_requests) = self.ofs_buf.apply_ansi_bytes(&output);

            // Handle any OSC events that were detected.
            for event in osc_events {
                match event {
                    OscEvent::SetTitleAndTab(title) => {
                        self.terminal_title = Some(title.clone());
                        tracing::debug!(
                            "Process '{}' set terminal title: {}",
                            self.name,
                            title
                        );
                    }
                    _ => {
                        // Other OSC events can be handled here in the future.
                    }
                }
            }

            // Handle any DSR response events - send them back through PTY.
            if !dsr_requests.is_empty()
                && let Some(session) = &self.session
            {
                for dsr_event in dsr_requests {
                    let response_bytes = dsr_event.to_string().into_bytes();
                    tracing::debug!(
                        "Process '{}' sending DSR response: {:?}",
                        self.name,
                        dsr_event
                    );
                    // Send the response back through the PTY input channel.
                    let _unused = session
                        .input_event_ch_tx_half
                        .send(crate::PtyInputEvent::Write(response_bytes));
                }
            }
            self.has_unrendered_output = true;

            tracing::trace!(
                "Process '{}' updated buffer with {} bytes, cursor at {:?}",
                self.name,
                output.len(),
                self.ofs_buf.my_pos
            );
        }
    }

    /// Try to get output from this process's PTY session without blocking.
    ///
    /// Returns None if no output is immediately available, or Some(output) if
    /// there is new data to process.
    pub fn try_get_output(&mut self) -> Option<Vec<u8>> {
        if let Some(session) = &mut self.session
            && let Ok(event) = session.output_event_receiver_half.try_recv()
        {
            match event {
                PtyReadWriteOutputEvent::Output(data) => {
                    tracing::trace!(
                        "Process '{}' received {} bytes of output",
                        self.name,
                        data.len()
                    );
                    return Some(data);
                }
                PtyReadWriteOutputEvent::Exit(_status) => {
                    self.is_running = false;
                    tracing::debug!("Process '{}' has exited", self.name);
                    return None;
                }
                _ => {}
            }
        }
        None
    }

    /// Mark this process as having been rendered (clear unrendered output flag).
    pub fn mark_as_rendered(&mut self) { self.has_unrendered_output = false; }
}

impl Debug for Process {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Process")
            .field("name", &self.name)
            .field("command", &self.command)
            .field("args", &self.args)
            .field("session", &self.session)
            .field("is_running", &self.is_running)
            .field("offscreen_buffer", &self.ofs_buf)
            .field("has_unrendered_output", &self.has_unrendered_output)
            .field("terminal_title", &self.terminal_title)
            .finish()
    }
}

impl ProcessManager {
    /// Create a new process manager with the given processes and terminal size.
    #[must_use]
    pub fn new(processes: Vec<Process>, terminal_size: Size) -> Self {
        Self {
            processes,
            active_index: 0,
            terminal_size,
        }
    }

    /// Start all processes at startup.
    ///
    /// This spawns all configured processes immediately so they're ready when the user
    /// switches to them. This ensures faster switching and eliminates the delay of
    /// on-demand spawning. Fails if any process cannot be started.
    ///
    /// # Errors
    ///
    /// Returns an error if any process cannot be started or spawned.
    pub fn start_all_processes(&mut self) -> miette::Result<()> {
        for i in 0..self.processes.len() {
            if self.processes[i].session.is_none()
                && let Err(e) = self.spawn_process(i)
            {
                // Fail immediately if any process can't be started
                miette::bail!(
                    "Failed to start process '{}' ({}): {}. Please ensure it's installed and in PATH.",
                    self.processes[i].name,
                    self.processes[i].command,
                    e
                );
            }
        }
        Ok(())
    }

    /// Switch to the process at the given index.
    ///
    /// **Instant switching with per-process virtual terminals**:
    /// This is where the per-process buffer architecture shines - switching
    /// between processes is truly instant because each process maintains its
    /// complete terminal state independently.
    ///
    /// **What happens**:
    /// 1. Change the `active_index` to point to a different process
    /// 2. That's it! No delays, no resize tricks, no screen clearing
    /// 3. The next render will display the target process's virtual terminal
    ///
    /// **Why this works universally**:
    /// - TUI apps: Their complete screen state is preserved in the `OffscreenBuffer`
    /// - bash: Your command history and current prompt state remain intact
    /// - CLI tools: All their output is preserved exactly as they generated it
    pub fn switch_to(&mut self, index: usize) -> Option<usize> {
        if index >= self.processes.len() {
            return None;
        }

        let old_index = self.active_index;
        self.active_index = index;

        tracing::debug!(
            "Switched from process {} ('{}') to process {} ('{}') - instant switch with per-process buffers",
            old_index,
            self.processes[old_index].name,
            index,
            self.processes[index].name
        );

        Some(old_index)
    }

    /// Spawn a process at the given index.
    fn spawn_process(&mut self, index: usize) -> miette::Result<()> {
        let process = &mut self.processes[index];
        tracing::debug!("Spawning process: {} ({})", process.name, process.command);

        // Reserve bottom row for status bar - PTY gets reduced height
        let pty_size = PtySize {
            rows: self
                .terminal_size
                .row_height
                .saturating_sub(STATUS_BAR_HEIGHT),
            cols: self.terminal_size.col_width.into(),
            pixel_width: 0,
            pixel_height: 0,
        };

        // Use existing PtyCommandBuilder with reduced size
        let session = PtyCommandBuilder::new(&process.command)
            .args(&process.args)
            .spawn_read_write(pty_size)?;

        process.session = Some(session);
        process.is_running = true;
        Ok(())
    }

    /// Poll all processes and update their virtual terminal buffers.
    ///
    /// This is the heart of the per-process virtual terminal architecture:
    ///
    /// **Key Innovation**: ALL processes are polled continuously, not just the active
    /// one. Each process maintains its own complete virtual terminal state through
    /// its `OffscreenBuffer`. When you switch between processes, you're instantly
    /// seeing their maintained terminal state - no delays, no fake resize tricks
    /// needed.
    ///
    /// **How it works**:
    /// 1. Poll each process for new PTY output (non-blocking)
    /// 2. If output exists, process it through the ANSI parser
    /// 3. Update the process's virtual terminal buffer
    /// 4. Track if the currently active process had updates
    ///
    /// **Why this enables universal compatibility**:
    /// - bash: Command history and prompt state persist in the virtual terminal
    /// - TUI apps: Complete screen state maintained perfectly
    /// - CLI tools: Output preserved exactly as generated
    ///
    /// Returns true if the active process had new output (triggers rendering).
    pub fn poll_all_processes(&mut self) -> bool {
        let mut active_had_output = false;

        for (i, process) in self.processes.iter_mut().enumerate() {
            if let Some(output) = process.try_get_output() {
                // Update this process's virtual terminal buffer
                process.process_pty_output_and_update_buffer(output);

                // Track if the active process had output
                if i == self.active_index {
                    active_had_output = true;
                }

                tracing::trace!(
                    "Process {} ('{}') updated its virtual terminal buffer (active: {})",
                    i,
                    process.name,
                    i == self.active_index
                );
            }
        }

        active_had_output
    }

    /// Send input to the currently active process.
    ///
    /// # Errors
    ///
    /// Returns an error if sending input to the process fails.
    pub fn send_input(&mut self, event: PtyInputEvent) -> miette::Result<()> {
        if let Some(session) = &self.processes[self.active_index].session {
            let _unused = session.input_event_ch_tx_half.send(event);
        }
        Ok(())
    }

    /// Get the name of the currently active process.
    #[must_use]
    pub fn active_name(&self) -> &str { &self.processes[self.active_index].name }

    /// Get a slice of all processes.
    #[must_use]
    pub fn processes(&self) -> &[Process] { &self.processes }

    /// Get the index of the currently active process.
    #[must_use]
    pub fn active_index(&self) -> usize { self.active_index }

    /// Get the terminal title of the currently active process (if any).
    #[must_use]
    pub fn active_terminal_title(&self) -> Option<&str> {
        self.processes[self.active_index].terminal_title.as_deref()
    }

    /// Get read-only access to the active process's virtual terminal buffer.
    #[must_use]
    pub fn get_active_buffer(&self) -> &OffscreenBuffer {
        &self.processes[self.active_index].ofs_buf
    }

    /// Mark the active process as having been rendered.
    pub fn mark_active_as_rendered(&mut self) {
        self.processes[self.active_index].mark_as_rendered();
    }
    /// Handle terminal resize with per-process buffer architecture.
    ///
    /// This creates fresh buffers at the new size for all processes and resets
    /// their parsers for a clean state. Each PTY is notified of the resize
    /// for natural reflow.
    pub fn handle_terminal_resize(&mut self, new_size: Size) {
        self.terminal_size = new_size;

        // Calculate PTY size (reserve status bar)
        let pty_size = PtySize {
            rows: new_size.row_height.saturating_sub(STATUS_BAR_HEIGHT),
            cols: new_size.col_width.into(),
            pixel_width: 0,
            pixel_height: 0,
        };

        let buffer_size = Size {
            row_height: height(new_size.row_height.saturating_sub(STATUS_BAR_HEIGHT)),
            col_width: new_size.col_width,
        };

        tracing::debug!(
            "Handling terminal resize to {:?}, PTY size: {:?}, buffer size: {:?}",
            new_size,
            pty_size,
            buffer_size
        );

        // Update all processes with new buffers and parsers
        for (i, process) in self.processes.iter_mut().enumerate() {
            // Create fresh buffer at new size
            process.ofs_buf = OffscreenBuffer::new_empty(buffer_size);

            // Clear unrendered output flag since we're starting fresh
            process.has_unrendered_output = false;

            // Send resize event to PTY session
            if let Some(session) = &process.session {
                let _unused = session
                    .input_event_ch_tx_half
                    .send(PtyInputEvent::Resize(pty_size));

                tracing::debug!(
                    "Sent resize event to process {} ('{}') with PTY size {:?}",
                    i,
                    process.name,
                    pty_size
                );
            }
        }

        tracing::debug!(
            "Terminal resize handling completed for {} processes",
            self.processes.len()
        );
    }

    /// Shutdown all running processes.
    ///
    /// This method kills all active PTY sessions to ensure clean exit.
    /// Called when the multiplexer is shutting down.
    pub fn shutdown_all_processes(&mut self) {
        tracing::debug!(
            "Shutting down all processes - starting cleanup of {} processes",
            self.processes.len()
        );

        for (index, process) in self.processes.iter_mut().enumerate() {
            tracing::debug!(
                "Processing shutdown for process {}: '{}' (running: {})",
                index,
                process.name,
                process.is_running
            );

            if let Some(mut session) = process.session.take() {
                tracing::debug!(
                    "Process '{}' has active session, forcefully terminating",
                    process.name
                );

                // CRITICAL SHUTDOWN PATTERN: Kill child process THEN send Close event
                // 1. First, kill the child process to ensure immediate termination
                match session.child_process_terminate_handle.kill() {
                    Ok(()) => tracing::debug!(
                        "Successfully killed child process for '{}'",
                        process.name
                    ),
                    Err(e) => tracing::warn!(
                        "Failed to kill child process for '{}': {:?}",
                        process.name,
                        e
                    ),
                }

                // 2. Then, send Close event to stop input writer and signal EOF
                // Note: Close alone is insufficient - it only stops input, doesn't kill
                // the process
                match session.input_event_ch_tx_half.send(PtyInputEvent::Close) {
                    Ok(()) => tracing::debug!(
                        "Successfully sent Close event to process '{}'",
                        process.name
                    ),
                    Err(e) => tracing::warn!(
                        "Failed to send Close event to process '{}': {:?}",
                        process.name,
                        e
                    ),
                }

                tracing::debug!("Dropping PTY session for process '{}'", process.name);
                // Drop the session to clean up resources
                drop(session);
                tracing::debug!("PTY session dropped for process '{}'", process.name);

                process.is_running = false;
                tracing::debug!("Process '{}' marked as not running", process.name);
            } else {
                tracing::debug!(
                    "Process '{}' has no active session, skipping",
                    process.name
                );
            }
        }

        tracing::debug!("Finished shutting down all processes");
    }
}

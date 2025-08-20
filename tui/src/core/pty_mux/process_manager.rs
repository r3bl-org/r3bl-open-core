// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Process lifecycle management for the PTY multiplexer.
//!
//! This module manages multiple PTY sessions, handles process switching,
//! and implements the "fake resize" technique for proper TUI app repainting.

use portable_pty::PtySize;
use tokio::time::{Duration, sleep};

use super::output_renderer::STATUS_BAR_HEIGHT;
use crate::{Size,
            core::pty::{PtyCommandBuilder, PtyInputEvent, PtyReadWriteOutputEvent,
                        PtyReadWriteSession}};

/// Manages multiple PTY processes and handles switching between them.
#[derive(Debug)]
pub struct ProcessManager {
    processes: Vec<Process>,
    active_index: usize,
    terminal_size: Size,
}

/// Represents a single process that can be managed by the multiplexer.
#[derive(Debug)]
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
}

/// Output events from the process manager.
#[derive(Debug)]
pub enum ProcessOutput {
    /// Output from the currently active process
    Active(Vec<u8>),
    /// Notification that processes were switched
    ProcessSwitch { from: usize, to: usize },
}

impl Process {
    /// Create a new process definition.
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            session: None,
            is_running: false,
        }
    }

    /// Returns whether this process is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool { self.is_running }
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

    /// Switch to the process at the given index with enhanced repaint strategy.
    ///
    /// Uses aggressive terminal reset sequences and enhanced fake resize to ensure
    /// complete repaints when switching between TUI processes.
    ///
    /// # Errors
    ///
    /// Returns an error if sending terminal sequences or resize events fails.
    pub async fn switch_to(&mut self, index: usize) -> miette::Result<()> {
        if index >= self.processes.len() {
            return Ok(());
        }

        let old_index = self.active_index;
        self.active_index = index;
        tracing::debug!("Switching from process {} to {}", old_index, index);

        if let Some(session) = &mut self.processes[index].session {
            // Only use fake resize - this is the correct and sufficient approach
            // The fake resize sends SIGWINCH, causing TUI apps to repaint themselves
            
            // 1. Fake resize sequence (tiny -> actual size)
            let tiny_size = PtySize {
                rows: 10,
                cols: 10,
                pixel_width: 0,
                pixel_height: 0,
            };
            let _unused = session
                .input_event_ch_tx_half
                .send(PtyInputEvent::Resize(tiny_size));
            sleep(Duration::from_millis(50)).await;

            // 2. Resize to actual size - this triggers SIGWINCH and full repaint
            let real_size = PtySize {
                rows: self
                    .terminal_size
                    .row_height
                    .0
                    .value
                    .saturating_sub(STATUS_BAR_HEIGHT),
                cols: self.terminal_size.col_width.0.value,
                pixel_width: 0,
                pixel_height: 0,
            };
            let _unused = session
                .input_event_ch_tx_half
                .send(PtyInputEvent::Resize(real_size));
        }

        Ok(())
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
                .0
                .value
                .saturating_sub(STATUS_BAR_HEIGHT),
            cols: self.terminal_size.col_width.0.value,
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

    /// Try to get output from the active process without blocking.
    ///
    /// Returns None if no output is immediately available.
    pub fn try_get_output(&mut self) -> Option<ProcessOutput> {
        // First drain ALL background processes to prevent their buffers from filling up
        // This is critical - if we don't drain them, they'll block and stop working
        for (i, process) in self.processes.iter_mut().enumerate() {
            if i != self.active_index
                && let Some(session) = &mut process.session
            {
                // Drain all pending output from background processes
                while session.output_event_receiver_half.try_recv().is_ok() {
                    // Discard - we're not displaying this process right now
                }
            }
        }

        // Now try to get output from the active process
        if let Some(session) = &mut self.processes[self.active_index].session
            && let Ok(event) = session.output_event_receiver_half.try_recv()
        {
            match event {
                PtyReadWriteOutputEvent::Output(data) => {
                    return Some(ProcessOutput::Active(data));
                }
                PtyReadWriteOutputEvent::Exit(_status) => {
                    self.processes[self.active_index].is_running = false;
                    return None;
                }
                _ => {}
            }
        }

        None
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

    /// Update the terminal size and propagate to all active PTY sessions.
    pub fn update_terminal_size(&mut self, new_size: Size) {
        self.terminal_size = new_size;

        // Send resize events to all active PTY sessions
        let pty_size = PtySize {
            rows: new_size
                .row_height
                .0
                .value
                .saturating_sub(STATUS_BAR_HEIGHT),
            cols: new_size.col_width.0.value,
            pixel_width: 0,
            pixel_height: 0,
        };

        for process in &self.processes {
            if let Some(session) = &process.session {
                let _unused = session
                    .input_event_ch_tx_half
                    .send(PtyInputEvent::Resize(pty_size));
            }
        }
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

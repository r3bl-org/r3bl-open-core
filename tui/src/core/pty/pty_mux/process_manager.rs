// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Process lifecycle management for the [`PTY`] multiplexer. See [`ProcessManager`] and
//! [`Process`].
//!
//! Each process maintains its own virtual terminal ([`OfsBuf`]) and [[`ANSI`]
//! parser]. Process switching is instant - just display a different buffer.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`OfsBuf`]: crate::tui::OfsBuf
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [ANSI parser]: crate::core::ansi::AnsiToOfsBufPerformer

use super::STATUS_BAR_HEIGHT;
#[allow(unused_imports, reason = "Allows short rustdoc ref def links")]
use crate::core::pty::pty_mux;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, DEBUG_TUI_PTY_PROCESS_MANAGER,
            DefaultPtySessionConfig, LengthOps, OfsBufVT100, PtyInputEvent,
            PtySessionConfigOption, ScrollbackAmount, VPSize, VPWidth,
            core::{osc::OscEvent,
                   pty::{PtyOutputEvent, PtySession, PtySessionBuilder}},
            ok, vp_height};
use std::fmt::{Debug, Formatter};

/// Manages multiple [`PTY`] processes and handles switching between them.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessStatus {
    Running,
    #[default]
    NotRunning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnrenderedOutput {
    Available,
    #[default]
    NotAvailable,
}

#[derive(Debug)]
pub struct ProcessManager {
    processes: Vec<Process>,
    focused_index: usize,
    terminal_size: VPSize,
}

/// Represents a single process that can be managed by the multiplexer.
///
/// Semantically, this is a **virtual terminal**. For a detailed explanation of the
/// architecture, see the [Virtual Terminal Architecture].
///
/// Each process maintains its own virtual terminal emulator through an [`OfsBufVT100`],
/// enabling true terminal multiplexing where switching between processes is instant and
/// preserves the complete terminal state (including scrollback).
///
/// [Virtual Terminal Architecture]:
///     mod@pty_mux#virtual-terminal-architecture
pub struct Process {
    /// Display name for this process (shown in status bar).
    pub name: String,

    /// Command to execute.
    pub command: String,

    /// Command line arguments.
    pub args: Vec<String>,

    /// Optional [`PTY`] session ([`None`] if not yet spawned).
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    session: Option<PtySession>,

    /// Virtual terminal buffer for this process (per-process buffer architecture).
    pub terminal_state: OfsBufVT100,

    /// Whether the process is currently running
    pub status: ProcessStatus,

    /// Tracks if this process has unrendered output since last render
    pub unrendered_output: UnrenderedOutput,

    /// Terminal title set by [`OSC`] sequences ([`None`] if not set).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub terminal_title: Option<String>,

    /// Optional vertical viewport origin.
    ///
    /// - [`None`] means the viewport is locked to the live output (bottom).
    /// - [`Some`] means the user has detached and scrolled up into the history.
    pub maybe_scroll_offset: Option<ScrollbackAmount>,
}

impl Process {
    /// Creates a new process definition with virtual terminal buffer.
    ///
    /// The buffer is sized to (height-1, width) to reserve space for the status bar. Each
    /// process gets its own virtual terminal that persists when switching.
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        terminal_size: VPSize,
    ) -> Self {
        Self::new_with_virtual_width(name, command, args, terminal_size, None)
    }

    /// Creates a new process definition with an optional custom virtual terminal width.
    ///
    /// The buffer height is sized to (height-1) to reserve space for the status bar.
    /// The buffer width is sized to `maybe_virtual_width` if provided, otherwise
    /// defaulting to `terminal_size.col_width`.
    pub fn new_with_virtual_width(
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        terminal_size: VPSize,
        maybe_virtual_width: Option<VPWidth>,
    ) -> Self {
        let col_width = maybe_virtual_width.unwrap_or(terminal_size.col_width);
        // Reserve bottom row for status bar - buffer gets reduced height.
        let buffer_size = VPSize {
            row_height: terminal_size.row_height - vp_height(STATUS_BAR_HEIGHT),
            col_width,
        };

        Self {
            name: name.into(),
            command: command.into(),
            args,
            session: None,
            status: ProcessStatus::NotRunning,
            terminal_state: OfsBufVT100::new_empty(buffer_size),
            unrendered_output: UnrenderedOutput::NotAvailable,
            terminal_title: None,
            maybe_scroll_offset: None,
        }
    }

    /// Scrolls the viewport back into the history buffer by the specified amount.
    pub fn scroll_back_by(&mut self, amount: ScrollbackAmount) {
        let max_scrollback_amt = self.terminal_state.get_history_len();
        if max_scrollback_amt == 0 {
            // Nowhere to scroll back to.
            return;
        }

        let new_offset = match self.maybe_scroll_offset {
            // If not scrolling yet, start by scrolling back by the specified amount.
            None => amount,
            // If already scrolling, add to the current offset.
            Some(current_offset) => current_offset.saturating_add(amount),
        };

        self.maybe_scroll_offset = match new_offset.overflows(max_scrollback_amt) {
            // Clip the viewport origin to the top of the history buffer.
            ArrayOverflowResult::Overflowed => Some(max_scrollback_amt.into()),
            // Otherwise, keep the new offset.
            ArrayOverflowResult::Within => Some(new_offset),
        };
    }

    /// Scrolls the viewport forward towards the live output by the specified amount.
    pub fn scroll_forward_by(&mut self, amount: ScrollbackAmount) {
        let Some(current_offset) = self.maybe_scroll_offset else {
            // Already at the bottom.
            return;
        };

        // Subtract the scroll amount from the current offset.
        let new_offset = current_offset.saturating_sub(amount);

        if new_offset.is_empty() {
            // If the result of subtracting is 0, then we've scrolled all the way forward
            // to the live boundary.
            self.maybe_scroll_offset = None;
        } else {
            // Otherwise, keep the new offset.
            self.maybe_scroll_offset = Some(new_offset);
        }
    }

    /// Pans the virtual terminal viewport left by the specified amount.
    pub fn pan_left_by(&mut self, amount: VPWidth) {
        self.terminal_state.pan_viewport_left(amount);
    }

    /// Pans the virtual terminal viewport right by the specified amount.
    pub fn pan_right_by(&mut self, amount: VPWidth) {
        self.terminal_state.pan_viewport_right(amount);
    }

    /// Returns whether this process is currently running.
    #[must_use]
    pub fn status(&self) -> ProcessStatus { self.status }

    /// Updates the process's virtual terminal buffer with new [`PTY`] output.
    ///
    /// This is the core of the per-process virtual terminal architecture: Each process
    /// maintains its own complete terminal state through a [`OfsBufVT100`]. Raw
    /// [`PTY`] bytes are processed through the [`ANSI`] parser and converted into
    /// [`PixelChar`] updates in the virtual terminal buffer.
    ///
    /// This allows each process to maintain its complete screen state independently,
    /// enabling instant switching without any delays or resizing tricks.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`PixelChar`]: crate::tui::PixelChar
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn process_pty_output_and_update_buffer(&mut self, output: Vec<u8>) {
        if !output.is_empty() {
            // Process bytes and extract any OSC and DSR events.
            let (osc_events, pty_response_events) =
                self.terminal_state.apply_ansi_bytes(&output);

            // Handle any OSC events that were detected.
            for event in osc_events {
                match event {
                    OscEvent::SetTitleAndTab(title) => {
                        self.terminal_title = Some(title.clone());
                        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug! {
                                message = "PtyProcess::process_pty_output_and_update_buffer",
                                process_name = %self.name,
                                title = %title,
                                "Process set terminal title"
                            };
                        });
                    }
                    _ => {
                        // Other OSC events can be handled here in the future.
                    }
                }
            }

            // Handle any DSR response events - send them back through PTY.
            if !pty_response_events.is_empty()
                && let Some(session) = &self.session
            {
                for response_event in pty_response_events {
                    let response_bytes = response_event.to_string().into_bytes();
                    DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug! {
                            message = "PtyProcess::process_pty_output_and_update_buffer",
                            process_name = %self.name,
                            response = ?response_event,
                            "Sending DSR response"
                        };
                    });
                    // Send the response back through the PTY input channel.
                    let _unused = session
                        .tx_input_event
                        .try_send(PtyInputEvent::Write(response_bytes));
                }
            }
            self.unrendered_output = UnrenderedOutput::Available;

            DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                // % is Display, ? is Debug.
                tracing::trace! {
                    message = "PtyProcess::process_pty_output_and_update_buffer",
                    process_name = %self.name,
                    bytes = output.len(),
                    cursor = ?self.terminal_state.get_active_screen_buffer().get_cursor_pos(),
                    "Process updated buffer"
                };
            });
        }
    }

    /// Tries to get output from this process's [`PTY`] session without blocking.
    ///
    /// # Returns
    ///
    /// - [`None`] if no output is immediately available
    /// - [`Some(output)`] if there is new data to process
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`Some(output)`]: Some
    pub fn try_get_output(&mut self) -> Option<Vec<u8>> {
        if let Some(session) = &mut self.session
            && let Ok(event) = session.rx_output_event.try_recv()
        {
            match event {
                PtyOutputEvent::Output(data) => {
                    DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                        // % is Display, ? is Debug.
                        tracing::trace! {
                            message = "PtyProcess::try_get_output",
                            process_name = %self.name,
                            bytes = data.len(),
                            "Yielding accumulated output bytes"
                        };
                    });
                    return Some(data);
                }
                PtyOutputEvent::Exit(_status) => {
                    self.status = ProcessStatus::NotRunning;
                    DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug! {
                            message = "PtyProcess::try_get_output",
                            process_name = %self.name,
                            "Process has exited"
                        };
                    });
                    return None;
                }
                _ => {}
            }
        }
        None
    }

    /// Marks this process as having been rendered (clear unrendered output flag).
    pub fn mark_as_rendered(&mut self) {
        self.unrendered_output = UnrenderedOutput::NotAvailable;
    }
}

impl Debug for Process {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Process")
            .field("name", &self.name)
            .field("command", &self.command)
            .field("args", &self.args)
            .field("session", &self.session)
            .field("status", &self.status)
            .field("terminal_state", &self.terminal_state)
            .field("unrendered_output", &self.unrendered_output)
            .field("terminal_title", &self.terminal_title)
            .field("maybe_scroll_offset", &self.maybe_scroll_offset)
            .finish()
    }
}

impl ProcessManager {
    /// Creates a new process manager with the given processes and terminal size.
    #[must_use]
    pub fn new(processes: Vec<Process>, terminal_size: VPSize) -> Self {
        Self {
            processes,
            focused_index: 0,
            terminal_size,
        }
    }

    /// Starts all processes at startup.
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
                // Fail immediately if any process can't be started.
                return Err(miette::miette!(
                    "Failed to start process '{}' ({}): {}. Please ensure it's installed and in PATH.",
                    self.processes[i].name,
                    self.processes[i].command,
                    e
                ));
            }
        }
        ok!()
    }

    /// Switch to the process at the given index.
    ///
    /// **Instant switching with per-process virtual terminals**: This is where the
    /// per-process buffer architecture shines - switching between processes is truly
    /// instant because each process maintains its complete terminal state independently.
    ///
    /// **What happens**:
    /// 1. Change the `focused_index` to point to a different process
    /// 2. That's it! No delays, no resize tricks, no screen clearing
    /// 3. The next render will display the target process's virtual terminal
    ///
    /// **Why this works universally**:
    /// - TUI apps: Their complete screen state is preserved in the [`OfsBufVT100`]
    /// - bash: Your command history and current prompt state remain intact
    /// - CLI tools: All their output is preserved exactly as they generated it
    pub fn switch_to(&mut self, index: usize) -> Option<usize> {
        if index >= self.processes.len() {
            return None;
        }

        let old_index = self.focused_index;
        self.focused_index = index;

        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "ProcessManager::switch_to",
                old_index = %old_index,
                old_name = %self.processes[old_index].name,
                new_index = %index,
                new_name = %self.processes[index].name,
                "Instant switch with per-process buffers"
            };
        });

        Some(old_index)
    }

    /// Spawns a process at the given index.
    fn spawn_process(&mut self, index: usize) -> miette::Result<()> {
        let process = &mut self.processes[index];
        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "ProcessManager::spawn_process",
                process_name = %process.name,
                command = %process.command,
                "Spawning process"
            };
        });

        // Reserve bottom row for status bar - PTY gets reduced height and
        // process-configured width.
        let pty_size = VPSize {
            row_height: self.terminal_size.row_height - vp_height(STATUS_BAR_HEIGHT),
            col_width: process.terminal_state.get_window_size().col_width,
        };

        // Use existing PtySessionBuilder with reduced size.
        let session = PtySessionBuilder::new(&process.command)
            .cli_args(&process.args)
            .with_config(DefaultPtySessionConfig + PtySessionConfigOption::Size(pty_size))
            .start()?;

        process.session = Some(session);
        process.status = ProcessStatus::Running;
        ok!()
    }

    /// Poll all processes and update their virtual terminal buffers.
    ///
    /// This is the heart of the per-process virtual terminal architecture:
    ///
    /// **Key Innovation**: ALL processes are polled continuously, not just the focused
    /// one. Each process maintains its own complete virtual terminal state through its
    /// [`OfsBufVT100`]. When you switch between processes, you're instantly seeing
    /// their maintained terminal state - no delays, no fake resize tricks needed.
    ///
    /// **How it works**:
    /// 1. Poll each process for new [`PTY`] output (non-blocking)
    /// 2. If output exists, process it through the [`ANSI`] parser
    /// 3. Update the process's virtual terminal buffer
    /// 4. Track if the currently focused process had updates
    ///
    /// **Why this enables universal compatibility**:
    /// - bash: Command history and prompt state persist in the virtual terminal
    /// - TUI apps: Complete screen state maintained perfectly
    /// - CLI tools: Output preserved exactly as generated
    ///
    /// Returns true if the focused process had new output (triggers rendering).
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn poll_all_processes(&mut self) -> bool {
        let mut focused_had_output = false;

        for (i, process) in self.processes.iter_mut().enumerate() {
            if let Some(output) = process.try_get_output() {
                // Update this process's virtual terminal buffer.
                process.process_pty_output_and_update_buffer(output);

                // Track if the focused process had output.
                if i == self.focused_index {
                    focused_had_output = true;
                }

                DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::trace! {
                        message = "ProcessManager::poll_all_processes",
                        process_index = %i,
                        process_name = %process.name,
                        focused = %(i == self.focused_index),
                        "Updated virtual terminal buffer"
                    };
                });
            }
        }

        focused_had_output
    }

    /// Sends input to the currently focused process.
    pub fn send_input(&mut self, event: PtyInputEvent) {
        if let Some(session) = &self.processes[self.focused_index].session
            && let Err(err) = session.tx_input_event.try_send(event)
        {
            DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                tracing::warn!("Ignored input for dead process: {}", err);
            });
        }
    }

    /// Gets the name of the currently focused process.
    #[must_use]
    pub fn focused_name(&self) -> &str { &self.processes[self.focused_index].name }

    /// Gets a slice of all processes.
    #[must_use]
    pub fn processes(&self) -> &[Process] { &self.processes }

    /// Gets the index of the currently focused process.
    #[must_use]
    pub fn focused_index(&self) -> usize { self.focused_index }

    /// Gets the terminal title of the currently focused process (if any).
    #[must_use]
    pub fn focused_terminal_title(&self) -> Option<&str> {
        self.processes[self.focused_index].terminal_title.as_deref()
    }

    /// Gets read-only access to the focused process's virtual terminal buffer.
    #[must_use]
    pub fn focused_process_buffer(&self) -> &OfsBufVT100 {
        &self.processes[self.focused_index].terminal_state
    }

    /// Gets immutable access to the focused process.
    #[must_use]
    pub fn focused_process(&self) -> &Process { &self.processes[self.focused_index] }

    /// Gets mutable access to the focused process.
    pub fn focused_process_mut(&mut self) -> &mut Process {
        &mut self.processes[self.focused_index]
    }

    /// Marks the focused process as having been rendered.
    pub fn mark_focused_as_rendered(&mut self) {
        self.processes[self.focused_index].mark_as_rendered();
    }

    /// Handles terminal resize with per-process buffer architecture.
    ///
    /// This creates fresh buffers at the new size for all processes and resets their
    /// parsers for a clean state. Each [`PTY`] is notified of the resize for natural
    /// reflow.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn handle_terminal_resize(&mut self, new_size: VPSize) {
        self.terminal_size = new_size;

        // Calculate PTY/buffer size (reserve status bar).
        let pty_size = VPSize {
            row_height: new_size.row_height - vp_height(STATUS_BAR_HEIGHT),
            col_width: new_size.col_width,
        };

        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "ProcessManager::handle_terminal_resize",
                new_size = ?new_size,
                pty_size = ?pty_size,
                "Handling terminal resize"
            };
        });

        // Update all processes with new buffers and parsers.
        for (i, process) in self.processes.iter_mut().enumerate() {
            // Create fresh buffer at new size.
            process.terminal_state = OfsBufVT100::new_empty(pty_size);

            // Clear unrendered output flag since we're starting fresh.
            process.unrendered_output = UnrenderedOutput::NotAvailable;

            // Send resize event to PTY session.
            if let Some(session) = &process.session {
                let _unused = session
                    .tx_input_event
                    .try_send(PtyInputEvent::Resize(pty_size));

                DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "ProcessManager::handle_terminal_resize",
                        process_index = %i,
                        process_name = %process.name,
                        pty_size = ?pty_size,
                        "Sent resize event to process"
                    };
                });
            }
        }

        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "ProcessManager::handle_terminal_resize",
                num_processes = %self.processes.len(),
                "Terminal resize handling completed"
            };
        });
    }

    /// Shuts down all running processes.
    ///
    /// This method kills all active [`PTY`] sessions to ensure clean exit. Called when
    /// the multiplexer is shutting down.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[allow(clippy::too_many_lines)]
    pub fn shutdown_all_processes(&mut self) {
        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "ProcessManager::shutdown_all_processes",
                num_processes = %self.processes.len(),
                "Shutting down all processes - starting cleanup"
            };
        });

        for (index, process) in self.processes.iter_mut().enumerate() {
            DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "ProcessManager::shutdown_all_processes",
                    process_index = %index,
                    process_name = %process.name,
                    running = %(process.status == ProcessStatus::Running),
                    "Processing shutdown for process"
                };
            });

            if let Some(mut session) = process.session.take() {
                DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "ProcessManager::shutdown_all_processes",
                        process_name = %process.name,
                        "Process has active session, forcefully terminating"
                    };
                });

                // CRITICAL SHUTDOWN PATTERN: Kill child process THEN send Close event
                // 1. First, kill the child process to ensure immediate termination.
                match session.child_process_termination_handle.kill() {
                    Ok(()) => {
                        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug! {
                                message = "ProcessManager::shutdown_all_processes",
                                process_name = %process.name,
                                "Successfully killed child process"
                            };
                        });
                    }
                    Err(e) => {
                        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                            // % is Display, ? is Debug.
                            tracing::warn! {
                                message = "ProcessManager::shutdown_all_processes",
                                process_name = %process.name,
                                error = ?e,
                                "Failed to kill child process"
                            };
                        });
                    }
                }

                // 2. Then, send Close event to stop input writer and signal EOF.
                // Note: Close alone is insufficient - it only stops input, doesn't kill
                // the process.
                match session.tx_input_event.try_send(PtyInputEvent::Close) {
                    Ok(()) => {
                        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug! {
                                message = "ProcessManager::shutdown_all_processes",
                                process_name = %process.name,
                                "Successfully sent Close event"
                            };
                        });
                    }
                    Err(e) => {
                        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                            // % is Display, ? is Debug.
                            tracing::warn! {
                                message = "ProcessManager::shutdown_all_processes",
                                process_name = %process.name,
                                error = ?e,
                                "Failed to send Close event"
                            };
                        });
                    }
                }

                // Drop the session to clean up resources.
                DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "ProcessManager::shutdown_all_processes",
                        process_name = %process.name,
                        "Dropping PTY session"
                    };
                });
                drop(session);
                DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "ProcessManager::shutdown_all_processes",
                        process_name = %process.name,
                        "PTY session dropped"
                    };
                });

                // Mark process as not running.
                process.status = ProcessStatus::NotRunning;
                DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "ProcessManager::shutdown_all_processes",
                        process_name = %process.name,
                        "Process marked as not running"
                    };
                });
            } else {
                DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "ProcessManager::shutdown_all_processes",
                        process_name = %process.name,
                        "Process has no active session, skipping"
                    };
                });
            }
        }

        DEBUG_TUI_PTY_PROCESS_MANAGER.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "ProcessManager::shutdown_all_processes",
                "Finished shutting down all processes"
            };
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vp_col, vp_height, vp_size, vp_width};

    #[test]
    fn test_process_virtual_width_sizing() {
        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let proc_default = Process::new("bash", "bash", vec![], terminal_size);
        assert_eq!(
            proc_default.terminal_state.get_window_size(),
            vp_size(vp_width(80), vp_height(24))
        );

        let proc_wide = Process::new_with_virtual_width(
            "bash",
            "bash",
            vec![],
            terminal_size,
            Some(vp_width(1000)),
        );
        assert_eq!(
            proc_wide.terminal_state.get_window_size(),
            vp_size(vp_width(1000), vp_height(24))
        );
    }

    #[test]
    fn test_process_horizontal_panning() {
        let terminal_size = vp_size(vp_width(80), vp_height(25));
        let mut proc = Process::new_with_virtual_width(
            "bash",
            "bash",
            vec![],
            terminal_size,
            Some(vp_width(1000)),
        );

        assert_eq!(proc.terminal_state.get_viewport_col_offset(), vp_col(0));

        proc.pan_right_by(vp_width(50));
        assert_eq!(proc.terminal_state.get_viewport_col_offset(), vp_col(50));

        proc.pan_left_by(vp_width(20));
        assert_eq!(proc.terminal_state.get_viewport_col_offset(), vp_col(30));

        // Saturates at col 0 when panning left further than current offset.
        proc.pan_left_by(vp_width(100));
        assert_eq!(proc.terminal_state.get_viewport_col_offset(), vp_col(0));
    }
}

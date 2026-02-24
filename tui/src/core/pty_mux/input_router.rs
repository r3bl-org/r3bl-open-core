// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Dynamic input event routing for the [PTY] multiplexer.
//!
//! This module handles keyboard input routing, including dynamic process switching
//! shortcuts (F1 through F9 based on the number of processes) and terminal resize events.
//!
//! [PTY]: https://en.wikipedia.org/wiki/Pseudoterminal

use super::ProcessManager;
use crate::{AnsiSequenceGenerator, Continuation, InputEvent, Key, KeyPress, KeyState,
            ModifierKeysMask, Size, col,
            core::{osc::OscController,
                   pty::{PtyInputEvent, pty_core::pty_sessions::show_notification},
                   terminal_io::OutputDevice},
            lock_output_device_as_mut, row};

/// Routes input events to appropriate handlers and manages dynamic keyboard shortcuts.
#[derive(Debug)]
pub struct InputRouter;

impl InputRouter {
    /// Creates a new input router.
    #[must_use]
    pub fn new() -> Self { Self }

    /// Handles an input event, routing it appropriately.
    ///
    /// # Returns
    ///
    /// - [`Continuation::Stop`] if the application should exit (Ctrl+Q or input shutdown)
    /// - [`Continuation::Continue`] otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if terminal operations or process switching fails.
    pub fn handle_input(
        &mut self,
        event: InputEvent,
        process_manager: &mut ProcessManager,
        osc: &mut OscController<'_>,
        output_device: &OutputDevice,
    ) -> miette::Result<Continuation> {
        match event {
            InputEvent::Keyboard(key) => {
                match key {
                    // Process switching: Handle F1 through F12 for switching processes
                    KeyPress::Plain {
                        key: Key::FunctionKey(fn_key),
                    } => {
                        let fn_number = u8::from(fn_key);
                        let process_index = (fn_number - 1) as usize;

                        tracing::debug!("Received F{} for process switching", fn_number);

                        // Only switch if the process index is valid for current process
                        // count.
                        if process_index < process_manager.processes().len() {
                            let old_index = process_manager.active_index();
                            if old_index == process_index {
                                tracing::debug!(
                                    "F{} pressed but already on process {}",
                                    fn_number,
                                    process_index
                                );
                            } else {
                                tracing::debug!(
                                    "Process switch: {} -> {} (triggered by F{})",
                                    old_index,
                                    process_index,
                                    fn_number
                                );

                                // Show notification for process switching.
                                let process_name =
                                    &process_manager.processes()[process_index].command;
                                show_notification(
                                    "PTY Mux - Process Switch",
                                    &format!("Switching to {process_name}"),
                                );

                                // Clear the screen before switching.
                                {
                                    let out = lock_output_device_as_mut!(output_device);
                                    let _unused = out.write_all(
                                        AnsiSequenceGenerator::clear_screen().as_bytes(),
                                    );
                                    let _unused = out.write_all(
                                        AnsiSequenceGenerator::cursor_position(
                                            row(0),
                                            col(0),
                                        )
                                        .as_bytes(),
                                    );
                                    let _unused = out.flush();
                                }

                                process_manager.switch_to(process_index);
                                Self::update_terminal_title(process_manager, osc)?;
                                tracing::debug!("Process switch completed successfully");
                            }
                        } else {
                            tracing::warn!(
                                "Invalid process index {} for current process count {} (F{} pressed)",
                                process_index,
                                process_manager.processes().len(),
                                fn_number
                            );
                        }
                    }
                    // Exit shortcut: Ctrl+Q
                    KeyPress::WithModifiers {
                        key: Key::Character('q'),
                        mask:
                            ModifierKeysMask {
                                ctrl_key_state: KeyState::Pressed,
                                shift_key_state: _, // Don't care about shift state
                                alt_key_state: _,   // Don't care about alt state
                            },
                    } => {
                        tracing::debug!("Exit requested (Ctrl+Q)");

                        // Show notification for exit.
                        show_notification("PTY Mux - Exit", "Exiting PTY Mux");

                        return Ok(Continuation::Stop); // Exit requested
                    }
                    _ => {
                        // Show notification for other key presses (useful for debugging)
                        show_notification(
                            "PTY Mux - Key Press",
                            &format!("Key pressed: {key:?}"),
                        );

                        // Forward all other input to active PTY using proper conversion.
                        if let Some(pty_event) = Option::<PtyInputEvent>::from(key) {
                            tracing::debug!("Forwarding input to PTY: {:?}", pty_event);
                            process_manager.send_input(pty_event)?;
                        }
                    }
                }
            }
            InputEvent::Resize(new_size) => {
                // Handle terminal resize - forward to all active PTYs.
                Self::handle_resize(process_manager, new_size);
            }
            InputEvent::Shutdown(_) => {
                // Input thread died - signal exit so the mux doesn't hang waiting for
                // events that will never come.
                return Ok(Continuation::Stop);
            }
            _ => {
                // Other input events (Mouse, Focus, BracketedPaste) are
                // ignored for now.
            }
        }

        Ok(Continuation::Continue)
    }

    /// Updates the terminal title based on the currently active process.
    fn update_terminal_title(
        process_manager: &ProcessManager,
        osc: &mut OscController<'_>,
    ) -> miette::Result<()> {
        // Check if the active process has set a custom terminal title.
        let title = if let Some(custom_title) = process_manager.active_terminal_title() {
            // Use the process's custom title.
            format!(
                "PTYMux - {} - {}",
                process_manager.active_name(),
                custom_title
            )
        } else {
            // Use default title with just process name.
            format!("PTYMux - {}", process_manager.active_name())
        };
        osc.set_title_and_tab(&title)?;
        Ok(())
    }

    /// Handles terminal resize events.
    ///
    /// Updates the process manager's size and forwards reduced size to all PTY sessions
    /// to reserve space for the status bar.
    fn handle_resize(process_manager: &mut ProcessManager, new_size: Size) {
        // Update manager's size (full terminal size)
        process_manager.handle_terminal_resize(new_size);

        // Forward reduced size to all PTY sessions to reserve status bar space.
        // Note: The process manager handles the actual PTY size conversion
        // We just need to update the manager with the full terminal size.

        // The process manager handles forwarding resize events to PTY sessions
        // so we don't need to do it here explicitly.
    }
}

impl Default for InputRouter {
    fn default() -> Self { Self::new() }
}

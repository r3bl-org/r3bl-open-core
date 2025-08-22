// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Dynamic input event routing for the PTY multiplexer.
//!
//! This module handles keyboard input routing, including dynamic process switching
//! shortcuts (F1 through F9 based on the number of processes) and
//! terminal resize events.

use super::ProcessManager;
use crate::{Size,
            ansi::terminal_output,
            core::{osc::OscController, pty::{PtyInputEvent, pty_core::pty_sessions::show_notification}, terminal_io::OutputDevice},
            tui::terminal_lib_backends::{FunctionKey, InputEvent, Key, KeyPress, KeyState,
                                         ModifierKeysMask}};

/// Routes input events to appropriate handlers and manages dynamic keyboard shortcuts.
#[derive(Debug)]
pub struct InputRouter;

impl InputRouter {
    /// Create a new input router.
    #[must_use]
    pub fn new() -> Self { Self }

    /// Handle an input event, routing it appropriately.
    ///
    /// Returns `Ok(true)` if the application should exit, `Ok(false)` otherwise.
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
    ) -> miette::Result<bool> {
        match event {
            InputEvent::Keyboard(key) => {
                match key {
                    // Process switching: Handle F1 through F9 for switching processes
                    KeyPress::Plain { key: Key::FunctionKey(fn_key) } => {
                        let (fn_number, process_index) = match fn_key {
                            FunctionKey::F1 => (1, 0),
                            FunctionKey::F2 => (2, 1),
                            FunctionKey::F3 => (3, 2),
                            FunctionKey::F4 => (4, 3),
                            FunctionKey::F5 => (5, 4),
                            FunctionKey::F6 => (6, 5),
                            FunctionKey::F7 => (7, 6),
                            FunctionKey::F8 => (8, 7),
                            FunctionKey::F9 => (9, 8),
                            _ => return Ok(false), // F10-F12 not handled
                        };

                        tracing::debug!("Received F{} for process switching", fn_number);

                        // Only switch if the process index is valid for current process count
                        if process_index < process_manager.processes().len() {
                            let old_index = process_manager.active_index();
                            if old_index == process_index {
                                tracing::debug!("F{} pressed but already on process {}", fn_number, process_index);
                            } else {
                                tracing::debug!(
                                    "Process switch: {} -> {} (triggered by F{})",
                                    old_index,
                                    process_index,
                                    fn_number
                                );
                                
                                // Show notification for process switching
                                let process_name = &process_manager.processes()[process_index].command;
                                show_notification("PTY Mux - Process Switch", &format!("Switching to {process_name}"));
                                
                                // Clear the screen before switching
                                terminal_output::clear_screen_and_home_cursor(
                                    output_device,
                                );

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
                        
                        // Show notification for exit
                        show_notification("PTY Mux - Exit", "Exiting PTY Mux");
                        
                        return Ok(true); // Exit requested
                    }
                    _ => {
                        // Show notification for other key presses (useful for debugging)
                        show_notification("PTY Mux - Key Press", &format!("Key pressed: {key:?}"));
                        
                        // Forward all other input to active PTY using proper conversion
                        if let Some(pty_event) = Option::<PtyInputEvent>::from(key) {
                            tracing::debug!("Forwarding input to PTY: {:?}", pty_event);
                            process_manager.send_input(pty_event)?;
                        }
                    }
                }
            }
            InputEvent::Resize(new_size) => {
                // Handle terminal resize - forward to all active PTYs
                Self::handle_resize(process_manager, new_size);
            }
            _ => {
                // Other input events are ignored for now
            }
        }

        Ok(false)
    }

    /// Update the terminal title based on the currently active process.
    fn update_terminal_title(
        process_manager: &ProcessManager,
        osc: &mut OscController<'_>,
    ) -> miette::Result<()> {
        // Check if the active process has set a custom terminal title
        let title = if let Some(custom_title) = process_manager.active_terminal_title() {
            // Use the process's custom title
            format!("PTYMux - {} - {}", process_manager.active_name(), custom_title)
        } else {
            // Use default title with just process name
            format!("PTYMux - {}", process_manager.active_name())
        };
        osc.set_title_and_tab(&title)?;
        Ok(())
    }

    /// Handle terminal resize events.
    ///
    /// Updates the process manager's size and forwards reduced size to all PTY sessions
    /// to reserve space for the status bar.
    fn handle_resize(process_manager: &mut ProcessManager, new_size: Size) {
        // Update manager's size (full terminal size)
        process_manager.handle_terminal_resize(new_size);

        // Forward reduced size to all PTY sessions to reserve status bar space
        // Note: The process manager handles the actual PTY size conversion
        // We just need to update the manager with the full terminal size

        // The process manager handles forwarding resize events to PTY sessions
        // so we don't need to do it here explicitly
    }
}

impl Default for InputRouter {
    fn default() -> Self { Self::new() }
}

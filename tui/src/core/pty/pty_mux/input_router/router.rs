// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::ProcessManager, keyboard_command::KeyboardCommand,
            mouse_command::MouseCommand};
use crate::{Continuation, DEBUG_TUI_PTY_MUX, InputEvent, Size};

/// Dynamic input event routing for the [`PTY`] multiplexer.
///
/// This intercepts and routes all user input (keyboard, mouse, terminal resize) to either
/// the multiplexer's control layer or the currently active background process.
///
/// ## Core Responsibilities
///
/// 1. _Multiplexer Control_: Handles global multiplexer management commands, such as
///    exiting the session and dynamically switching focus between concurrent processes.
/// 2. _Keyboard Input_: Forwards all other keyboard inputs safely to the active [`PTY`]
///    session.
/// 3. _Mouse Tracking_: Intercepts terminal mouse events, respects the active buffer's
///    [`MouseTrackingMode`], and translates clicks, scrolls, and motion into [`VT-100`]
///    compliant [`SGR`] sequences via [`mouse_sgr`].
/// 4. _Resize Events_: Propagates terminal resize events to the process manager to update
///    all underlying sessions.
///
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
///
/// [`mouse_sgr`]: crate::mouse_sgr
/// [`MouseTrackingMode`]: crate::MouseTrackingMode
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`SGR`]: crate::SgrCode
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
pub fn handle_input(
    event: InputEvent,
    process_manager: &mut ProcessManager,
) -> miette::Result<Continuation> {
    let active_buffer = process_manager.active_buffer();

    match event {
        InputEvent::Keyboard(key) => {
            // Decision - convert key press to command.
            let keyboard_command = KeyboardCommand::from((key, active_buffer));
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "handle_input",
                    keyboard_command = ?keyboard_command,
                };
            });

            // Execute the command.
            match keyboard_command {
                KeyboardCommand::ScrollHistoryBack(amount) => {
                    process_manager.active_process_mut().scroll_back_by(amount);
                }
                KeyboardCommand::ScrollHistoryForward(amount) => {
                    process_manager
                        .active_process_mut()
                        .scroll_forward_by(amount);
                }
                KeyboardCommand::ForwardToProcess(pty_event) => {
                    process_manager.active_process_mut().maybe_scroll_offset = None;
                    process_manager.send_input(pty_event);
                }
                KeyboardCommand::Ignore => {}
            }
        }
        InputEvent::Mouse(mouse_input) => {
            // Decision - convert mouse input to command.
            let mouse_command = MouseCommand::from((&mouse_input, active_buffer));
            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "handle_input",
                    mouse_command = ?mouse_command,
                };
            });

            // Execute the command.
            match mouse_command {
                MouseCommand::ScrollHistoryBack(amount) => {
                    process_manager.active_process_mut().scroll_back_by(amount);
                }
                MouseCommand::ScrollHistoryForward(amount) => {
                    process_manager
                        .active_process_mut()
                        .scroll_forward_by(amount);
                }
                MouseCommand::ForwardToProcess(pty_event) => {
                    process_manager.send_input(pty_event);
                }
                MouseCommand::Ignore => {}
            }
        }
        InputEvent::Resize(new_size) => {
            // Handle terminal resize - forward to all active PTYs.
            handle_resize(process_manager, new_size);
        }
        InputEvent::Shutdown(_) => {
            // Input thread died - signal exit so the mux doesn't hang waiting for
            // events that will never come.
            return Ok(Continuation::Stop);
        }
        _ => {
            // Other input events (Focus, BracketedPaste) are
            // ignored for now.
        }
    }

    Ok(Continuation::Continue)
}

/// Handles terminal resize events.
///
/// Updates the process manager's size and forwards reduced size to all [`PTY`]
/// sessions to reserve space for the status bar.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn handle_resize(process_manager: &mut ProcessManager, new_size: Size) {
    // Update manager's size (full terminal size)
    process_manager.handle_terminal_resize(new_size);

    // Forward reduced size to all PTY sessions to reserve status bar space.
    // Note: The process manager handles the actual PTY size conversion
    // We just need to update the manager with the full terminal size.

    // The process manager handles forwarding resize events to PTY sessions
    // so we don't need to do it here explicitly.
}

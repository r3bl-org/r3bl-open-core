// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Dynamic input event routing for the [`PTY`] multiplexer.
//!
//! This module intercepts and routes all user input (keyboard, mouse, terminal resize) to
//! either the multiplexer's control layer or the currently active background process.
//!
//! ## Core Responsibilities
//!
//! 1. _Multiplexer Control_: Handles global multiplexer management commands, such as
//!    exiting the session and dynamically switching focus between concurrent processes.
//! 2. _Keyboard Input_: Forwards all other keyboard inputs safely to the active [`PTY`]
//!    session.
//! 3. _Mouse Tracking_: Intercepts terminal mouse events, respects the active buffer's
//!    [`MouseTrackingState`], and translates clicks, scrolls, and motion into [`VT-100`]
//!    compliant [`SGR`] sequences via [`SgrMouseSequence`].
//! 4. _Resize Events_: Propagates terminal resize events to the process manager to update
//!    all underlying sessions.
//!
//! [`MouseTrackingState`]: crate::MouseTrackingState
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SGR`]: crate::SgrCode
//! [`SgrMouseSequence`]: crate::SgrMouseSequence
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

use super::ProcessManager;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ColIndex, Continuation,
            DEBUG_TUI_PTY_MUX, InputEvent, MouseTrackingMode, RowHeight, RowIndex, Size,
            TermCol, TermRow};

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
    ) -> miette::Result<Continuation> {
        match event {
            InputEvent::Keyboard(key) => {
                // Forward all keyboard input to active PTY using proper conversion.
                if let Some(pty_event) = key.into() {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug! {
                            message = "InputRouter::handle_input",
                            status = "Forwarding input to PTY",
                            pty_event = ?pty_event,
                        };
                    });
                    process_manager.send_input(pty_event)?;
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
            InputEvent::Mouse(mouse_input) => {
                let active_buffer = process_manager.get_active_buffer();

                // We use a simplified "firehose" approach. If the app requested *any*
                // tracking protocol (1000/1002/1003), `mouse.mode` becomes `Enabled`.
                // When enabled, we unconditionally route all events (clicks, drags,
                // motion) back to the app using the modern SGR (1006)
                // format, ignoring `mouse.format`.
                match active_buffer.terminal_mode.mouse_tracking {
                    MouseTrackingMode::Enabled => {
                        let mouse_col: ColIndex = mouse_input.pos.col_index;
                        let mouse_row: RowIndex = mouse_input.pos.row_index;

                        let pty_height: RowHeight = active_buffer.window_size.row_height;
                        if mouse_row.overflows(pty_height)
                            == ArrayOverflowResult::Overflowed
                        {
                            return Ok(Continuation::Continue);
                        }

                        let term_col: TermCol = mouse_col.into();
                        let term_row: TermRow = mouse_row.into();

                        let sgr_bytes: Option<Vec<u8>> =
                            crate::SgrMouseSequence::generate(
                                &mouse_input,
                                term_col,
                                term_row,
                            );
                        if let Some(bytes) = sgr_bytes {
                            let _unused = process_manager
                                .send_input(crate::PtyInputEvent::Write(bytes));
                        } else {
                            DEBUG_TUI_PTY_MUX.then(|| {
                                // % is Display, ? is Debug.
                                tracing::error! {
                                    message = "InputRouter::handle_input",
                                    status = "Unsupported mouse event for SGR translation",
                                    mouse_event = ?mouse_input,
                                };
                            });
                        }
                    }
                    MouseTrackingMode::Disabled => {
                        // Do nothing.
                    }
                }
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
}

impl Default for InputRouter {
    fn default() -> Self { Self::new() }
}

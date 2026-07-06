// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::MOUSE_SCROLL_BY_AMOUNT;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ColIndex, DEBUG_TUI_PTY_MUX,
            MouseInput, MouseInputKind, MouseTrackingFormat, MouseTrackingMode,
            OfsBufVT100, PtyInputEvent, RowHeight, RowIndex, ScrollbackAmount, TermCol,
            TermRow, mouse_x10, mouse_sgr};

/// Represents the explicit command to take for a mouse event.
#[derive(Debug)]
pub enum MouseCommand {
    /// Scroll the virtual terminal viewport history up (intercepted).
    ScrollHistoryBack(ScrollbackAmount),

    /// Scroll the virtual terminal viewport history down (intercepted).
    ScrollHistoryForward(ScrollbackAmount),

    /// Forward the mouse event to the child process as an [`SGR`] sequence.
    ///
    /// [`SGR`]: crate::SgrCode
    ForwardToProcess(PtyInputEvent),

    /// The mouse event is out of bounds or unsupported and should be dropped.
    Ignore,
}

impl From<(&MouseInput, &OfsBufVT100)> for MouseCommand {
    /// Evaluates a raw mouse input event against the active buffer's [virtual terminal
    /// tab] terminal mode to determine the appropriate semantic command. The actual
    /// execution of this command is delegated to the [virtual terminal tab].
    ///
    /// - If mouse tracking is [`Disabled`], we evaluate scroll events against the active
    ///   screen buffer state to potentially yield a [`MouseCommand::ScrollHistoryBack`]
    ///   or [`MouseCommand::ScrollHistoryForward`] command.
    /// - If mouse tracking is [`Enabled`], we evaluate all events to yield a
    ///   [`MouseCommand::ForwardToProcess`] command. See the [`MouseTrackingFormat`]
    ///   implementation note for exact details on how the byte sequence payload is
    ///   formatted based on the app's requested protocols.
    ///
    /// [`Disabled`]: crate::MouseTrackingMode::Disabled
    /// [`Enabled`]: crate::MouseTrackingMode::Enabled
    /// [`mouse.format`]: crate::MouseTrackingMode
    /// [`mouse.mode`]: crate::TerminalModeState::mouse_tracking
    /// [`MouseTrackingFormat`]: crate::MouseTrackingFormat
    /// [`SGR`]: crate::SgrCode
    /// [virtual terminal tab]:
    ///     pty_mux#virtual-terminal-architecture-the-virtual-tab-mental-model
    fn from(args: (&MouseInput, &OfsBufVT100)) -> Self {
        let (mouse_input, active_buffer) = args;
        match active_buffer.terminal_mode.mouse_tracking_mode {
            MouseTrackingMode::Disabled => {
                // If mouse tracking is disabled and we're in the primary screen,
                // intercept scroll wheel events to scroll the buffer.
                if active_buffer.is_in_primary_screen() {
                    match mouse_input.kind {
                        MouseInputKind::ScrollUp => {
                            MouseCommand::ScrollHistoryBack(MOUSE_SCROLL_BY_AMOUNT.into())
                        }
                        MouseInputKind::ScrollDown => MouseCommand::ScrollHistoryForward(
                            MOUSE_SCROLL_BY_AMOUNT.into(),
                        ),
                        _ => {
                            DEBUG_TUI_PTY_MUX.then(|| {
                                tracing::debug!("Ignoring mouse event: {:?}", mouse_input.kind);
                            });
                            MouseCommand::Ignore
                        }
                    }
                } else {
                    MouseCommand::Ignore
                }
            }
            MouseTrackingMode::Enabled => {
                let mouse_col: ColIndex = mouse_input.pos.col_index;
                let mouse_row: RowIndex = mouse_input.pos.row_index;

                let pty_height: RowHeight = active_buffer.ofs_buf.get_window_size().row_height;
                if mouse_row.overflows(pty_height) == ArrayOverflowResult::Overflowed {
                    return MouseCommand::Ignore;
                }

                let term_col: TermCol = mouse_col.into();
                let term_row: TermRow = mouse_row.into();

                let mouse_tracking_format =
                    active_buffer.terminal_mode.mouse_tracking_format;
                let generated_bytes: Option<Vec<u8>> = match mouse_tracking_format {
                    MouseTrackingFormat::X10 => {
                        mouse_x10::generate(mouse_input, term_col, term_row)
                    }
                    MouseTrackingFormat::Sgr => {
                        mouse_sgr::generate(mouse_input, term_col, term_row)
                    }
                };

                if let Some(bytes) = generated_bytes {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        tracing::debug!(
                            "Forwarding mouse event ({:?}) as format {:?} bytes: {:?}",
                            mouse_input.kind,
                            mouse_tracking_format,
                            String::from_utf8_lossy(&bytes)
                        );
                    });
                    MouseCommand::ForwardToProcess(PtyInputEvent::Write(bytes))
                } else {
                    DEBUG_TUI_PTY_MUX.then(|| {
                        // % is Display, ? is Debug.
                        tracing::error! {
                            message = "MouseCommand::from",
                            status = "Unsupported mouse event for format",
                            format = ?mouse_tracking_format,
                            mouse_event = ?mouse_input,
                        };
                    });
                    MouseCommand::Ignore
                }
            }
        }
    }
}

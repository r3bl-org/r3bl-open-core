// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::MOUSE_SCROLL_BY_AMOUNT;
use crate::{ActiveScreenBuffer, DEBUG_TUI_PTY_MUX, KeyState, MouseInput, MouseInputKind,
            MouseTrackingFormat, MouseTrackingMode, OfsBufVT100, PtyInputEvent,
            RangeBoundsResult, ScrollbackAmount, VPCol, VPRow, VPWidth, mouse_sgr,
            mouse_x10};

/// Represents the explicit command to take for a mouse event.
#[derive(Debug)]
pub enum MouseCommand {
    /// Scroll the virtual terminal viewport history up (intercepted).
    ScrollHistoryBack(ScrollbackAmount),

    /// Scroll the virtual terminal viewport history down (intercepted).
    ScrollHistoryForward(ScrollbackAmount),

    /// Pan the virtual terminal viewport horizontally to the left (intercepted).
    PanHistoryLeft(VPWidth),

    /// Pan the virtual terminal viewport horizontally to the right (intercepted).
    PanHistoryRight(VPWidth),

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
    /// [`Disabled`]: MouseTrackingMode::Disabled
    /// [`Enabled`]: MouseTrackingMode::Enabled
    /// [`mouse.format`]: MouseTrackingMode
    /// [`mouse.mode`]: TerminalModeState::mouse_tracking
    /// [`MouseTrackingFormat`]: MouseTrackingFormat
    /// [`SGR`]: SgrCode
    /// [virtual terminal tab]:
    ///     pty_mux#virtual-terminal-architecture
    fn from(args: (&MouseInput, &OfsBufVT100)) -> MouseCommand {
        let (mouse_input, active_buffer) = args;
        match active_buffer.get_terminal_mode().mouse_tracking_mode {
            MouseTrackingMode::Disabled => {
                Self::handle_disabled_mouse_tracking(mouse_input, active_buffer)
            }
            MouseTrackingMode::Enabled => {
                Self::handle_enabled_mouse_tracking(mouse_input, active_buffer)
            }
        }
    }
}

impl MouseCommand {
    fn handle_disabled_mouse_tracking(
        mouse_input: &MouseInput,
        active_buffer: &OfsBufVT100,
    ) -> Self {
        // If mouse tracking is disabled and we're in the primary screen,
        // intercept scroll wheel events to scroll the buffer.
        if active_buffer.get_terminal_mode().active_screen_buffer
            == ActiveScreenBuffer::Primary
        {
            match mouse_input.kind {
                MouseInputKind::ScrollUp => {
                    if mouse_input
                        .maybe_modifier_keys
                        .is_some_and(|m| m.shift_key_state == KeyState::Pressed)
                    {
                        MouseCommand::PanHistoryLeft(MOUSE_SCROLL_BY_AMOUNT.into())
                    } else {
                        MouseCommand::ScrollHistoryBack(MOUSE_SCROLL_BY_AMOUNT.into())
                    }
                }
                MouseInputKind::ScrollDown => {
                    if mouse_input
                        .maybe_modifier_keys
                        .is_some_and(|m| m.shift_key_state == KeyState::Pressed)
                    {
                        MouseCommand::PanHistoryRight(MOUSE_SCROLL_BY_AMOUNT.into())
                    } else {
                        MouseCommand::ScrollHistoryForward(MOUSE_SCROLL_BY_AMOUNT.into())
                    }
                }
                MouseInputKind::ScrollLeft => {
                    MouseCommand::PanHistoryLeft(MOUSE_SCROLL_BY_AMOUNT.into())
                }
                MouseInputKind::ScrollRight => {
                    MouseCommand::PanHistoryRight(MOUSE_SCROLL_BY_AMOUNT.into())
                }
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

    fn handle_enabled_mouse_tracking(
        mouse_input: &MouseInput,
        active_buffer: &OfsBufVT100,
    ) -> Self {
        let mouse_col: VPCol = mouse_input.pos.col_index;
        let mouse_row: VPRow = mouse_input.pos.row_index;

        let viewport = active_buffer.get_active_screen_buffer().get_viewport();
        if viewport.contains_row(mouse_row) != RangeBoundsResult::Within
            || viewport.contains_col(mouse_col) != RangeBoundsResult::Within
        {
            return MouseCommand::Ignore;
        }

        let mouse_tracking_format =
            active_buffer.get_terminal_mode().mouse_tracking_format;

        let generated_bytes: Option<Vec<u8>> = {
            let mouse_col = mouse_col.into();
            let mouse_row = mouse_row.into();
            match mouse_tracking_format {
                MouseTrackingFormat::X10 => {
                    mouse_x10::generate(mouse_input, mouse_col, mouse_row)
                }
                MouseTrackingFormat::Sgr => {
                    mouse_sgr::generate(mouse_input, mouse_col, mouse_row)
                }
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

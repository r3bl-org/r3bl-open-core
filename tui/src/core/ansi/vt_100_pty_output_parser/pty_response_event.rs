// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#[allow(unused_imports, reason = "Used for shorter rustdoc link ref defs")]
use crate::core::ansi::vt_100_pty_output_parser::ParserGlobalState;
use crate::{TermCol, TermRow, DaSequence, DsrSequence};
use std::fmt::{self, Display};

/// Represents a query written by the child process (running in a [`PTY`] controlled end)
/// to its [`stdout`] that requires the "terminal emulator" (the [`pty_mux`] engine) to
/// write a response back to the child process's [`stdin`].
///
/// This is the generic event type that can wrap:
/// - Device Status Reports ([`DSR`]),
/// - Device Attributes ([`DA`]),
/// - and other terminal responses (in the future).
///
/// ## Data Flow
///
/// 1. **Child process → [`PTY`] → Parser**: Child process sends a query (e.g. `CSI c` or
///    `CSI 6n`). The bytes are routed through the [`PTY`] output channel into
///    [`apply_ansi_bytes()`].
/// 2. **Parser → Event**: [`AnsiToOfsBufPerformer::csi_dispatch()`] recognizes the query
///    and creates a variant of [`PtyResponseEvent`]. It pushes this into
///    [`pending_pty_response_events`].
/// 3. **Event → Manager**: [`Process::process_pty_output_and_update_buffer()`] (managed
///    by [`ProcessManager`]) drains this vector and receives the request event.
/// 4. **Manager → [`PTY`] → Child process**: [`Process`] calls `.to_string()` on the
///    event and sends the resulting bytes back through the [`session.tx_input_event`]
///    channel, which writes directly to the child process's `stdin`.
///
/// This type implements [`Display`] which delegates to the appropriate sequence generator
/// (like [`DsrSequence`] or [`DaSequence`]) for formatting.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer::csi_dispatch()`]: crate::AnsiToOfsBufPerformer
/// [`apply_ansi_bytes()`]: crate::OfsBufVT100::apply_ansi_bytes
/// [`DA`]: crate::DaSequence
/// [`DaSequence`]: crate::DaSequence
/// [`DSR`]: crate::DsrSequence
/// [`DsrSequence`]: crate::DsrSequence
/// [`pending_pty_response_events`]: ParserGlobalState::pending_pty_response_events
/// [`Process::process_pty_output_and_update_buffer()`]:
///     crate::pty_mux::Process::process_pty_output_and_update_buffer
/// [`Process`]: crate::pty_mux::Process
/// [`ProcessManager`]: crate::pty_mux::ProcessManager
/// [`pty_mux`]: crate::pty_mux
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`session.tx_input_event`]: crate::core::pty::PtySession::tx_input_event
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
#[derive(Debug, Clone, PartialEq)]
pub enum PtyResponseEvent {
    /// Terminal status report requested (`CSI 5n` received).
    ///
    /// Should respond with `ESC [ 0 n` (terminal OK).
    TerminalStatus,

    /// Cursor position report requested (`CSI 6n` received).
    ///
    /// Should respond with `ESC [ row ; col R` (1-based coordinates).
    CursorPosition { row: TermRow, col: TermCol },

    /// Primary Device Attributes requested (`CSI c` or `CSI 0 c` received).
    ///
    /// Should respond with `ESC [ ? 62 ; 22 c` (`VT220`-family terminal with [`ANSI`]
    /// color).
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    PrimaryDeviceAttributes,
}

impl Display for PtyResponseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PtyResponseEvent::TerminalStatus => DsrSequence::StatusOkResponse.fmt(f),
            PtyResponseEvent::CursorPosition { row, col } => {
                DsrSequence::CursorPositionResponse {
                    row: *row,
                    col: *col,
                }
                .fmt(f)
            }
            PtyResponseEvent::PrimaryDeviceAttributes => {
                DaSequence::PrimaryDeviceAttributes.fmt(f)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::constants::{DA1_VT220_COLOR_RESPONSE_STR, DSR_STATUS_OK_FULL_RESPONSE};
    use crate::vt_100_pty_output_conformance_tests::{
        nz, test_sequence_generators::dsr_builders::dsr_cursor_position_response,
    };
    use crate::{term_col, term_row};

    #[test]
    fn test_dsr_request_status_display() {
        let request = PtyResponseEvent::TerminalStatus;
        assert_eq!(request.to_string(), DSR_STATUS_OK_FULL_RESPONSE);
    }

    #[test]
    fn test_dsr_request_cursor_position_display() {
        let request = PtyResponseEvent::CursorPosition {
            row: term_row(nz(10)),
            col: term_col(nz(25)),
        };
        let expected = dsr_cursor_position_response(term_row(nz(10)), term_col(nz(25)));
        assert_eq!(request.to_string(), expected);
    }

    #[test]
    fn test_da1_request_display() {
        let request = PtyResponseEvent::PrimaryDeviceAttributes;
        assert_eq!(request.to_string(), DA1_VT220_COLOR_RESPONSE_STR);
    }

    #[test]
    fn test_to_bytes_conversion() {
        let request = PtyResponseEvent::TerminalStatus;
        let bytes = request.to_string().into_bytes();
        assert_eq!(bytes, DSR_STATUS_OK_FULL_RESPONSE.as_bytes());
    }
}

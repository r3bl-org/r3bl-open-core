// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) sequence builders.
//!
//! This module provides types and builders for handling DSR requests and responses
//! in terminal emulation.
//!
//! ## DSR Codes and Constants
//!
//! DSR sequences are used for bidirectional communication between terminals and
//! applications:
//! - **Requests** (INCOMING): Applications send CSI sequences to request information
//! - **Responses** (OUTGOING): Terminal emulator sends back ESC sequences with the
//!   requested data
//!
//! ### Request Format (from application)
//! - `CSI 5 n` - Request terminal status
//! - `CSI 6 n` - Request cursor position
//!
//! ### Response Format (from terminal)
//! - `ESC [ 0 n` - Terminal OK status
//! - `ESC [ row ; col R` - Cursor position (1-based)

use crate::{ParamsExt, TermCol, TermRow,
            core::{ansi::constants::{CSI_PARAM_SEPARATOR,
                                     DSR_CURSOR_POSITION_RESPONSE_END,
                                     DSR_RESPONSE_START, DSR_STATUS_OK_CODE,
                                     DSR_STATUS_RESPONSE_END},
                   common::fast_stringify::{BufTextStorage, FastStringify}},
            generate_impl_display_for_fast_stringify};
use std::fmt::{self, Display};

// DSR request types for parsing incoming DSR CSI sequences.

/// Device Status Report types for CSI n sequences.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DsrRequestType {
    /// Request terminal status (5)
    RequestStatus,
    /// Request cursor position (6)
    RequestCursorPosition,
    /// Unknown/unsupported DSR type
    Other(u16),
}

mod dsr_request_type_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl DsrRequestType {
        #[must_use]
        pub fn as_u16(&self) -> u16 {
            match self {
                Self::RequestStatus => 5,
                Self::RequestCursorPosition => 6,
                Self::Other(n) => *n,
            }
        }
    }

    impl From<&vte::Params> for DsrRequestType {
        fn from(params: &vte::Params) -> Self {
            let first_param_or_zero = params.extract_nth_single_opt_raw(0).unwrap_or(0);
            first_param_or_zero.into()
        }
    }

    impl From<u16> for DsrRequestType {
        fn from(n: u16) -> Self {
            match n {
                5 => Self::RequestStatus,
                6 => Self::RequestCursorPosition,
                other => Self::Other(other),
            }
        }
    }
}

/// Builder for formatting DSR response sequences.
///
/// This is the formatting layer that converts semantic DSR requests
/// into properly formatted ANSI escape sequences.
///
/// `DsrSequence` is part of the OUTGOING sequence builder family:
/// - **`CsiSequence`**: Builds CSI sequences (cursor movement, colors, etc.)
/// - **`OscSequence`**: Builds OSC sequences (titles, hyperlinks, notifications)
/// - **`DsrSequence`**: Builds DSR response sequences (status reports)
///
/// All implement `FastStringify` for efficient formatting and `Display` for convenience.
///
/// ## Examples
///
/// ```rust
/// use r3bl_tui::{DsrSequence, term_row, term_col};
/// use std::num::NonZeroU16;
///
/// let status = DsrSequence::StatusOkResponse;
/// assert_eq!(status.to_string(), "\x1b[0n");
///
/// let cursor = DsrSequence::CursorPositionResponse {
///     row: term_row(NonZeroU16::new(5).unwrap()),
///     col: term_col(NonZeroU16::new(10).unwrap())
/// };
/// assert_eq!(cursor.to_string(), "\x1b[5;10R");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DsrSequence {
    /// Terminal OK status response - ESC[0n
    StatusOkResponse,

    /// Cursor position response - ESC[row;colR (1-based)
    CursorPositionResponse { row: TermRow, col: TermCol },
}

mod dsr_sequence_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl FastStringify for DsrSequence {
        fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
            acc.push_str(DSR_RESPONSE_START);
            match self {
                DsrSequence::StatusOkResponse => {
                    acc.push_str(DSR_STATUS_OK_CODE);
                    acc.push(DSR_STATUS_RESPONSE_END);
                }
                DsrSequence::CursorPositionResponse { row, col } => {
                    acc.push_str(&row.as_u16().to_string());
                    acc.push(CSI_PARAM_SEPARATOR);
                    acc.push_str(&col.as_u16().to_string());
                    acc.push(DSR_CURSOR_POSITION_RESPONSE_END);
                }
            }
            Ok(())
        }

        fn write_buf_to_fmt(
            &self,
            acc: &BufTextStorage,
            f: &mut fmt::Formatter<'_>,
        ) -> fmt::Result {
            f.write_str(&acc.clone())
        }
    }
}

generate_impl_display_for_fast_stringify!(DsrSequence);

/// Represents a Device Status Report (DSR) request event received FROM the PTY child
/// process that requires a response to be sent back TO the PTY.
///
/// ## Data Flow
///
/// 1. **Child process → PTY → Parser**: Child process sends `CSI 6n` (request cursor
///    position)
/// 2. **Parser → Event**: Parser creates `DsrRequestFromPtyEvent::CursorPosition { row,
///    col }`
/// 3. **Event → Manager**: Process manager receives the request event
/// 4. **Manager → PTY → Child process**: Manager sends response bytes back:
///    `ESC[row;colR`
///
/// This type implements `Display` which delegates to `DsrSequence` for formatting.
///
/// ## Usage Example
///
/// ```rust
/// use r3bl_tui::{DsrRequestFromPtyEvent, term_row, term_col};
/// use std::num::NonZeroU16;
///
/// let request = DsrRequestFromPtyEvent::CursorPosition {
///     row: term_row(NonZeroU16::new(10).unwrap()),
///     col: term_col(NonZeroU16::new(25).unwrap())
/// };
/// let response_bytes = request.to_string().into_bytes();
/// // The response_bytes would be sent back through the PTY input channel
/// assert_eq!(response_bytes, b"\x1b[10;25R");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum DsrRequestFromPtyEvent {
    /// Terminal status report requested (CSI 5n received)
    /// Should respond with ESC[0n (terminal OK)
    TerminalStatus,

    /// Cursor position report requested (CSI 6n received)
    /// Should respond with ESC[row;colR (1-based coordinates)
    CursorPosition { row: TermRow, col: TermCol },
}

mod dsr_request_from_pty_event_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<&DsrRequestFromPtyEvent> for DsrSequence {
        fn from(event: &DsrRequestFromPtyEvent) -> Self {
            match event {
                DsrRequestFromPtyEvent::TerminalStatus => DsrSequence::StatusOkResponse,
                DsrRequestFromPtyEvent::CursorPosition { row, col } => {
                    DsrSequence::CursorPositionResponse {
                        row: *row,
                        col: *col,
                    }
                }
            }
        }
    }

    impl Display for DsrRequestFromPtyEvent {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // Delegate to DsrSequence for formatting.
            DsrSequence::from(self).fmt(f)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::constants::DSR_STATUS_OK_FULL_RESPONSE;
    use crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::test_sequence_builders::dsr_builders::dsr_cursor_position_response;
    use crate::{term_col, term_row,
                core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::test_fixtures_vt_100_ansi_conformance::nz};

    #[test]
    fn test_dsr_sequence_status_ok_response() {
        let sequence = DsrSequence::StatusOkResponse;
        assert_eq!(sequence.to_string(), DSR_STATUS_OK_FULL_RESPONSE);
    }

    #[test]
    fn test_dsr_sequence_cursor_position_response() {
        let sequence = DsrSequence::CursorPositionResponse {
            row: term_row(nz(10)),
            col: term_col(nz(25)),
        };
        let expected = dsr_cursor_position_response(term_row(nz(10)), term_col(nz(25)));
        assert_eq!(sequence.to_string(), expected);
    }

    #[test]
    fn test_dsr_sequence_clone_and_debug() {
        let original = DsrSequence::CursorPositionResponse {
            row: term_row(nz(5)),
            col: term_col(nz(10)),
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);

        let debug_output = format!("{original:?}");
        assert!(debug_output.contains("CursorPositionResponse"));
        assert!(debug_output.contains("TermRow(5)"));
        assert!(debug_output.contains("TermCol(10)"));
    }

    #[test]
    fn test_write_to_buf_produces_correct_ansi_sequence() {
        let sequence = DsrSequence::CursorPositionResponse {
            row: term_row(nz(42)),
            col: term_col(nz(84)),
        };
        let mut acc = BufTextStorage::new();

        // Test that write_to_buf works correctly.
        sequence.write_to_buf(&mut acc).unwrap();
        let expected = dsr_cursor_position_response(term_row(nz(42)), term_col(nz(84)));
        assert_eq!(acc.clone(), expected);
    }

    #[test]
    fn test_dsr_request_status_display() {
        let request = DsrRequestFromPtyEvent::TerminalStatus;
        assert_eq!(request.to_string(), DSR_STATUS_OK_FULL_RESPONSE);
    }

    #[test]
    fn test_dsr_request_cursor_position_display() {
        let request = DsrRequestFromPtyEvent::CursorPosition {
            row: term_row(nz(10)),
            col: term_col(nz(25)),
        };
        let expected = dsr_cursor_position_response(term_row(nz(10)), term_col(nz(25)));
        assert_eq!(request.to_string(), expected);
    }

    #[test]
    fn test_from_trait_conversion() {
        let request = DsrRequestFromPtyEvent::CursorPosition {
            row: term_row(nz(3)),
            col: term_col(nz(7)),
        };
        let expected_sequence = DsrSequence::CursorPositionResponse {
            row: term_row(nz(3)),
            col: term_col(nz(7)),
        };

        assert_eq!(DsrSequence::from(&request), expected_sequence);
    }

    #[test]
    fn test_to_bytes_conversion() {
        let request = DsrRequestFromPtyEvent::TerminalStatus;
        let bytes = request.to_string().into_bytes();
        assert_eq!(bytes, DSR_STATUS_OK_FULL_RESPONSE.as_bytes());
    }
}

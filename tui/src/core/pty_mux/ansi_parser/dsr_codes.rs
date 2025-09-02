// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) handling for terminal emulation.
//!
//! This module provides a two-layer architecture for handling DSR requests and responses:
//!
//! 1. **Semantic Layer** (`DsrRequestFromPty`): Represents what response is needed
//! 2. **Formatting Layer** (`DsrSequence`): Formats the response as ANSI sequences
//!
//! ## Architecture Overview
//!
//! DSR processing follows this flow:
//! 1. PTY child process sends DSR request (e.g., `CSI 6n` for cursor position)
//! 2. ANSI parser detects request and creates `DsrRequestFromPty` event
//! 3. Process manager receives event and sends formatted response back to PTY
//! 4. Response is formatted using `DsrSequence` (via `Display` trait delegation)
//!
//! ## DSR Codes and Constants
//!
//! DSR sequences are used for bidirectional communication between terminals and
//! applications:
//! - **Requests** (INCOMING): Applications send CSI sequences to request information
//! - **Responses** (OUTGOING): Terminal emulator sends back ESC sequences with the
//!   requested data
//!
//! ## Request Format (from application)
//! - `CSI 5 n` - Request terminal status
//! - `CSI 6 n` - Request cursor position
//!
//! ## Response Format (from terminal)
//! - `ESC [ 0 n` - Terminal OK status
//! - `ESC [ row ; col R` - Cursor position (1-based)

use std::fmt::{self, Display};

use super::csi_codes::CSI_PARAM_SEPARATOR;
use crate::core::common::write_to_buf::{BufTextStorage, WriteToBuf};

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

impl From<u16> for DsrRequestType {
    fn from(value: u16) -> Self {
        match value {
            5 => Self::RequestStatus,
            6 => Self::RequestCursorPosition,
            n => Self::Other(n),
        }
    }
}

// DSR response sequence components.

/// CSI sequence start for DSR responses: ESC [
pub const DSR_RESPONSE_START: &str = "\x1b[";

/// Status OK code: 0
pub const DSR_STATUS_OK_CODE: &str = "0";

/// Status response terminator: n
pub const DSR_STATUS_RESPONSE_END: char = 'n';

/// Cursor position response terminator: R
pub const DSR_CURSOR_POSITION_RESPONSE_END: char = 'R';

// Complete response sequences for testing.

/// Complete status OK response: ESC[0n
pub const DSR_STATUS_OK_FULL_RESPONSE: &str = "\x1b[0n";

/// Builder for formatting DSR response sequences.
///
/// This is the formatting layer that converts semantic DSR requests
/// into properly formatted ANSI escape sequences.
///
/// ## Architecture
///
/// `DsrSequence` is part of the OUTGOING sequence builder family:
/// - **`CsiSequence`**: Builds CSI sequences (cursor movement, colors, etc.)
/// - **`OscSequence`**: Builds OSC sequences (titles, hyperlinks, notifications)
/// - **`DsrSequence`**: Builds DSR response sequences (status reports)
///
/// All implement `WriteToBuf` for efficient formatting and `Display` for convenience.
///
/// ## Usage
///
/// Typically not used directly - instead use `DsrRequestFromPty` which
/// delegates to this type via its Display implementation.
///
/// ## Examples
///
/// ```rust
/// use r3bl_tui::DsrSequence;
///
/// let status = DsrSequence::StatusOkResponse;
/// assert_eq!(status.to_string(), "\x1b[0n");
///
/// let cursor = DsrSequence::CursorPositionResponse { row: 5, col: 10 };
/// assert_eq!(cursor.to_string(), "\x1b[5;10R");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DsrSequence {
    /// Terminal OK status response - ESC[0n
    StatusOkResponse,

    /// Cursor position response - ESC[row;colR (1-based)
    CursorPositionResponse { row: u16, col: u16 },
}

impl WriteToBuf for DsrSequence {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
        acc.push_str(DSR_RESPONSE_START);
        match self {
            DsrSequence::StatusOkResponse => {
                acc.push_str(DSR_STATUS_OK_CODE);
                acc.push(DSR_STATUS_RESPONSE_END);
            }
            DsrSequence::CursorPositionResponse { row, col } => {
                acc.push_str(&row.to_string());
                acc.push(CSI_PARAM_SEPARATOR);
                acc.push_str(&col.to_string());
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

impl fmt::Display for DsrSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut acc = BufTextStorage::new();
        self.write_to_buf(&mut acc)?;
        self.write_buf_to_fmt(&acc, f)
    }
}

/// Represents a Device Status Report (DSR) request event received FROM the PTY child
/// process that requires a response to be sent back TO the PTY.
///
/// ## Architecture Overview
///
/// When a PTY child process sends DSR request sequences (e.g., `CSI 5n` or `CSI 6n`),
/// the ANSI parser detects these and creates `DsrRequestFromPtyEvent` events. The process
/// manager then sends responses back through the PTY input channel.
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
/// ## Implementation Pattern
///
/// This type implements `Display` which delegates to `DsrSequence` for formatting.
/// This provides a clean separation between the semantic layer (what was requested)
/// and the formatting layer (how to encode the response).
///
/// ## Usage Example
///
/// ```rust
/// use r3bl_tui::DsrRequestFromPtyEvent;
///
/// let request = DsrRequestFromPtyEvent::CursorPosition { row: 10, col: 25 };
/// let response_bytes = request.to_string().into_bytes();
/// // The response_bytes would be sent back through the PTY input channel
/// assert_eq!(response_bytes, b"\x1b[10;25R");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum DsrRequestFromPtyEvent {
    /// Terminal status report requested (CSI 5n received)
    /// Should respond with ESC[0n (terminal OK)
    TerminalStatus,

    /// Cursor position report requested (CSI 6n received)\
    /// Should respond with ESC[row;colR (1-based coordinates)
    CursorPosition { row: u16, col: u16 },
}

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

/// Test helper functions for DSR sequences.
#[cfg(test)]
pub mod dsr_test_helpers {
    use super::*;

    #[must_use]
    pub fn dsr_cursor_position_response(row: u16, col: u16) -> String {
        format!(
            "{DSR_RESPONSE_START}{row}{CSI_PARAM_SEPARATOR}{col}{DSR_CURSOR_POSITION_RESPONSE_END}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{dsr_test_helpers::dsr_cursor_position_response, *};

    #[test]
    fn test_dsr_sequence_status_ok_response() {
        let sequence = DsrSequence::StatusOkResponse;
        assert_eq!(sequence.to_string(), DSR_STATUS_OK_FULL_RESPONSE);
    }

    #[test]
    fn test_dsr_sequence_cursor_position_response() {
        let sequence = DsrSequence::CursorPositionResponse { row: 10, col: 25 };
        let expected = dsr_cursor_position_response(10, 25);
        assert_eq!(sequence.to_string(), expected);
    }

    #[test]
    fn test_dsr_sequence_cursor_position_single_digits() {
        let sequence = DsrSequence::CursorPositionResponse { row: 1, col: 1 };
        let expected = dsr_cursor_position_response(1, 1);
        assert_eq!(sequence.to_string(), expected);
    }

    #[test]
    fn test_dsr_sequence_clone_and_debug() {
        let original = DsrSequence::CursorPositionResponse { row: 5, col: 10 };
        let cloned = original.clone();
        assert_eq!(original, cloned);

        let debug_output = format!("{original:?}");
        assert!(debug_output.contains("CursorPositionResponse"));
        assert!(debug_output.contains("row: 5"));
        assert!(debug_output.contains("col: 10"));
    }

    #[test]
    fn test_write_to_buf_efficiency() {
        let sequence = DsrSequence::CursorPositionResponse { row: 42, col: 84 };
        let mut acc = BufTextStorage::new();

        // Test that write_to_buf works correctly.
        sequence.write_to_buf(&mut acc).unwrap();
        let expected = dsr_cursor_position_response(42, 84);
        assert_eq!(acc.clone(), expected);
    }

    #[test]
    fn test_helper_function_consistency() {
        let sequence = DsrSequence::CursorPositionResponse { row: 7, col: 14 };
        let helper_result = dsr_cursor_position_response(7, 14);
        assert_eq!(sequence.to_string(), helper_result);
    }

    #[test]
    fn test_dsr_request_status_display() {
        let request = DsrRequestFromPtyEvent::TerminalStatus;
        assert_eq!(request.to_string(), DSR_STATUS_OK_FULL_RESPONSE);
    }

    #[test]
    fn test_dsr_request_cursor_position_display() {
        let request = DsrRequestFromPtyEvent::CursorPosition { row: 10, col: 25 };
        let expected = dsr_cursor_position_response(10, 25);
        assert_eq!(request.to_string(), expected);
    }

    #[test]
    fn test_dsr_request_cursor_position_single_digits() {
        let request = DsrRequestFromPtyEvent::CursorPosition { row: 1, col: 1 };
        let expected = dsr_cursor_position_response(1, 1);
        assert_eq!(request.to_string(), expected);
    }

    #[test]
    fn test_display_delegation() {
        // Verify that Display impl correctly uses From<&DsrRequestFromPtyEvent> for
        // DsrSequence.
        let request = DsrRequestFromPtyEvent::CursorPosition { row: 3, col: 7 };
        let sequence = DsrSequence::CursorPositionResponse { row: 3, col: 7 };
        assert_eq!(request.to_string(), sequence.to_string());

        // Test From trait directly.
        assert_eq!(DsrSequence::from(&request), sequence);
    }

    #[test]
    fn test_to_bytes_conversion() {
        let request = DsrRequestFromPtyEvent::TerminalStatus;
        let bytes = request.to_string().into_bytes();
        assert_eq!(bytes, DSR_STATUS_OK_FULL_RESPONSE.as_bytes());
    }
}
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) operations for VT100/ANSI terminal emulation.
//!
//! This module implements DSR operations that correspond to ANSI DSR
//! sequences handled by the `vt_100_ansi_parser::operations::dsr_ops` module. These
//! include:
//!
//! - **DSR 5** (Device Status Report) - `handle_status_report_request`
//! - **DSR 6** (Cursor Position Report) - `handle_cursor_position_request`
//!
//! All operations maintain VT100 compliance and handle proper response
//! queueing for later transmission back to the PTY.
//!
//! This module implements the business logic for DSR operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See [parser module docs](crate::core::pty_mux::vt_100_ansi_parser)
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//! - **Shim**: [`dsr_ops`] - Parameter translation and delegation (no direct tests)
//! - **Integration Tests**: [`test_dsr_ops`] - Full ANSI pipeline testing
//!
//! [`dsr_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::dsr_ops
//! [`test_dsr_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::test_dsr_ops

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{DsrRequestFromPtyEvent,
            core::pty_mux::vt_100_ansi_parser::term_units::{TermCol, TermRow}};

impl OffscreenBuffer {
    /// Handle device status report request.
    /// Queues a response indicating terminal is OK (ESC[0n).
    pub fn handle_status_report_request(&mut self) {
        self.ansi_parser_support
            .pending_dsr_responses
            .push(DsrRequestFromPtyEvent::TerminalStatus);
    }

    /// Handle cursor position report request.
    /// Queues a response with current cursor position (ESC[row;colR).
    /// Converts 0-based internal position to 1-based terminal position.
    pub fn handle_cursor_position_request(&mut self) {
        // Convert 0-based internal position to 1-based terminal position.
        let row: TermRow = (self.cursor_pos.row_index.as_u16() + 1).into();
        let col: TermCol = (self.cursor_pos.col_index.as_u16() + 1).into();
        self.ansi_parser_support
            .pending_dsr_responses
            .push(DsrRequestFromPtyEvent::CursorPosition { row, col });
    }
}

#[cfg(test)]
mod tests_dsr_ops {
    use super::*;
    use crate::{col, height, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_handle_status_report_request() {
        let mut buffer = create_test_buffer();

        // Initially no pending responses.
        assert!(buffer.ansi_parser_support.pending_dsr_responses.is_empty());

        buffer.handle_status_report_request();

        // Should have one terminal status response.
        assert_eq!(buffer.ansi_parser_support.pending_dsr_responses.len(), 1);
        assert!(matches!(
            buffer.ansi_parser_support.pending_dsr_responses[0],
            DsrRequestFromPtyEvent::TerminalStatus
        ));
    }

    #[test]
    fn test_handle_cursor_position_request() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(5);

        buffer.handle_cursor_position_request();

        // Should have one cursor position response.
        assert_eq!(buffer.ansi_parser_support.pending_dsr_responses.len(), 1);
        if let DsrRequestFromPtyEvent::CursorPosition { row, col } =
            &buffer.ansi_parser_support.pending_dsr_responses[0]
        {
            // 0-based internal (2,5) becomes 1-based terminal (3,6)
            assert_eq!(row.as_u16(), 3);
            assert_eq!(col.as_u16(), 6);
        } else {
            panic!("Expected CursorPosition response");
        }
    }

    #[test]
    fn test_multiple_dsr_requests() {
        let mut buffer = create_test_buffer();

        buffer.handle_status_report_request();
        buffer.handle_cursor_position_request();

        // Should have both responses queued.
        assert_eq!(buffer.ansi_parser_support.pending_dsr_responses.len(), 2);
        assert!(matches!(
            buffer.ansi_parser_support.pending_dsr_responses[0],
            DsrRequestFromPtyEvent::TerminalStatus
        ));
        assert!(matches!(
            buffer.ansi_parser_support.pending_dsr_responses[1],
            DsrRequestFromPtyEvent::CursorPosition { .. }
        ));
    }
}

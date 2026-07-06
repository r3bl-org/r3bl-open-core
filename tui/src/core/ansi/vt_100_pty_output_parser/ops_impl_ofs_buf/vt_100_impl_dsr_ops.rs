// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report ([`DSR`]) operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements [`DSR`] operations that correspond to [`ANSI`] [`DSR`]
//! sequences handled by the [`vt_100_pty_output_parser::ops::dsr_ops`] module. These
//! include:
//!
//! - **[`DSR`] 5** (Device Status Report) - [`handle_status_report_request`]
//! - **[`DSR`] 6** (Cursor Position Report) - [`handle_cursor_position_request`]
//!
//! All operations maintain [`VT-100`] compliance and handle proper response queueing for
//! later transmission back to the [`PTY`].
//!
//! This module implements the business logic for [`DSR`] operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`DSR`]: crate::DsrSequence
//! [`handle_cursor_position_request`]: crate::OfsBufVT100::handle_cursor_position_request
//! [`handle_status_report_request`]: crate::OfsBufVT100::handle_status_report_request
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_pty_output_parser::ops::dsr_ops`]:
//!     crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_dsr_ops

use crate::{PtyResponseEvent, OfsBufVT100, TermCol, TermRow};

impl OfsBufVT100 {
    /// Handles device status report request.
    ///
    /// Queues a response indicating terminal is OK (`ESC [ 0 n`).
    pub fn handle_status_report_request(&mut self) {
        self.parser_global_state
            .pending_pty_response_events
            .push(PtyResponseEvent::TerminalStatus);
    }

    /// Handles cursor position report request.
    ///
    /// Queues a response with current cursor position (`ESC [ row ; col R`).
    /// Converts 0-based internal position to 1-based terminal position.
    pub fn handle_cursor_position_request(&mut self) {
        // Convert 0-based internal position to 1-based terminal position.
        // Uses type-safe From<RowIndex>/From<ColIndex> conversions.
        let row = TermRow::from(self.get_cursor_pos().row_index);
        let col = TermCol::from(self.get_cursor_pos().col_index);
        self.parser_global_state
            .pending_pty_response_events
            .push(PtyResponseEvent::CursorPosition { row, col });
    }
}

#[cfg(test)]
mod tests_dsr_ops {
    use super::*;
    use crate::{OfsBufVT100, col, height, row, width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = width(10) + height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_handle_status_report_request() {
        let mut buffer = create_test_buffer();

        // Initially no pending responses.
        assert!(buffer.parser_global_state.pending_pty_response_events.is_empty());

        buffer.handle_status_report_request();

        // Should have one terminal status response.
        assert_eq!(buffer.parser_global_state.pending_pty_response_events.len(), 1);
        assert!(matches!(
            buffer.parser_global_state.pending_pty_response_events[0],
            PtyResponseEvent::TerminalStatus
        ));
    }

    #[test]
    fn test_handle_cursor_position_request() {
        let mut buffer = create_test_buffer();
        buffer.set_cursor_pos(row(2) + col(5));

        buffer.handle_cursor_position_request();

        // Should have one cursor position response.
        assert_eq!(buffer.parser_global_state.pending_pty_response_events.len(), 1);
        if let PtyResponseEvent::CursorPosition { row, col } =
            &buffer.parser_global_state.pending_pty_response_events[0]
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
        assert_eq!(buffer.parser_global_state.pending_pty_response_events.len(), 2);
        assert!(matches!(
            buffer.parser_global_state.pending_pty_response_events[0],
            PtyResponseEvent::TerminalStatus
        ));
        assert!(matches!(
            buffer.parser_global_state.pending_pty_response_events[1],
            PtyResponseEvent::CursorPosition { .. }
        ));
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for line insertion and deletion operations (IL/DL).
//!
//! Tests the complete pipeline from ANSI sequences through the shim to implementation
//! using the public [`apply_ansi_bytes`] API. This provides integration testing coverage
//! for the [`line_ops`] shim layer. The `test_` prefix follows our naming convention.
//! See [parser module docs] for the complete testing philosophy.
//!
//! **Related Files:**
//! - **Shim**: [`line_ops`] - Parameter translation (tested indirectly by this module)
//! - **Implementation**: [`impl_line_ops`] - Business logic (has separate unit tests)
//!
//! [`apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes
//! [`line_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::vt_100_shim_line_ops
//! [`impl_line_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_line_ops
//! [parser module docs]: super::super

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{col, row, term_col, term_row,
            vt_100_ansi_parser::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                                 CsiSequence}};

/// Tests for Insert Line (IL) operations.
pub mod insert_line {
    use super::*;

    #[test]
    fn test_insert_single_line() {
        let mut ofs_buf = create_numbered_buffer(5, 10);

        // Move cursor to row 2 (0-based) and insert one line
        let move_cursor = term_row(nz(3)) + term_col(nz(1)); // Move to row 3, col 1 (1-based)
        let insert_line = CsiSequence::InsertLine(1);
        let sequence = format!("{move_cursor}{insert_line}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify lines have shifted down.
        assert_blank_line(&ofs_buf, 2); // New blank line at cursor
        assert_line_content(&ofs_buf, 0, "Line00"); // Line 0 unchanged
        assert_line_content(&ofs_buf, 1, "Line01"); // Line 1 unchanged
        assert_line_content(&ofs_buf, 3, "Line02"); // Line 2 shifted to 3
        assert_line_content(&ofs_buf, 4, "Line03"); // Line 3 shifted to 4
        // Line 4 ("Line04") was lost (shifted off bottom)
    }

    #[test]
    fn test_insert_multiple_lines() {
        let mut ofs_buf = create_numbered_buffer(5, 10);

        // Move cursor to row 1 (0-based) and insert three lines
        let move_cursor = term_row(nz(2)) + term_col(nz(1)); // Move to row 2, col 1 (1-based)
        let insert_lines = CsiSequence::InsertLine(3);
        let sequence = format!("{move_cursor}{insert_lines}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify lines have shifted down by 3.
        assert_line_content(&ofs_buf, 0, "Line00"); // Line 0 unchanged
        assert_blank_line(&ofs_buf, 1); // New blank lines
        assert_blank_line(&ofs_buf, 2);
        assert_blank_line(&ofs_buf, 3);
        assert_line_content(&ofs_buf, 4, "Line01"); // Line 1 shifted to 4
        // Lines 2, 3, 4 were lost.
    }

    #[test]
    fn test_insert_lines_with_margins() {
        let mut ofs_buf = create_numbered_buffer(10, 10);

        // Set scroll margins: rows 3-7 (1-based)
        let set_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(nz(3))),
            bottom: Some(term_row(nz(7))),
        };
        let _result = ofs_buf.apply_ansi_bytes(format!("{set_margins}"));

        // Move cursor to row 5 (1-based, within margins) and insert one line
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(nz(5)),
            col: term_col(nz(1)),
        };
        let insert_line = CsiSequence::InsertLine(1);
        let sequence = format!("{move_cursor}{insert_line}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify only lines within margins are affected.
        assert_line_content(&ofs_buf, 0, "Line00"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 1, "Line01"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 2, "Line02"); // Top margin, unchanged
        assert_line_content(&ofs_buf, 3, "Line03"); // Within margins, unchanged
        assert_blank_line(&ofs_buf, 4); // Inserted blank line
        assert_line_content(&ofs_buf, 5, "Line04"); // Shifted within margins
        assert_line_content(&ofs_buf, 6, "Line05"); // Shifted within margins
        // Line06 was lost at bottom of scroll region.
        assert_line_content(&ofs_buf, 7, "Line07"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 8, "Line08"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 9, "Line09"); // Outside margins, unchanged
    }

    #[test]
    fn test_insert_line_outside_margins_ignored() {
        let mut ofs_buf = create_numbered_buffer(5, 10);
        let performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set scroll margins: rows 2-4 (1-based) = 1-3 (0-based)
        performer.ofs_buf.ansi_parser_support.scroll_region_top = Some(term_row(nz(2)));
        performer.ofs_buf.ansi_parser_support.scroll_region_bottom =
            Some(term_row(nz(4)));

        // Move cursor to row 0 (outside margins)
        performer.ofs_buf.cursor_pos = row(0) + col(0);

        // Try to insert line: ESC[L (should be ignored)
        let insert_line_sequence = format!("{}", CsiSequence::InsertLine(1));
        let _result = ofs_buf.apply_ansi_bytes(insert_line_sequence);

        // Verify no changes occurred.
        for r in 0..5 {
            assert_line_content(&ofs_buf, r, &format!("Line{r:02}"));
        }
    }
}

/// Tests for Delete Line (DL) operations.
pub mod delete_line {
    use super::*;

    #[test]
    fn test_delete_single_line() {
        let mut ofs_buf = create_numbered_buffer(5, 10);

        // Move cursor to row 3 (1-based) and delete one line
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(nz(3)),
            col: term_col(nz(1)),
        };
        let delete_line = CsiSequence::DeleteLine(1);
        let sequence = format!("{move_cursor}{delete_line}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify lines have shifted up.
        assert_line_content(&ofs_buf, 0, "Line00"); // Line 0 unchanged
        assert_line_content(&ofs_buf, 1, "Line01"); // Line 1 unchanged
        assert_line_content(&ofs_buf, 2, "Line03"); // Line 3 shifted to 2
        assert_line_content(&ofs_buf, 3, "Line04"); // Line 4 shifted to 3
        assert_blank_line(&ofs_buf, 4); // New blank line at bottom
        // Line 2 ("Line02") was deleted
    }

    #[test]
    fn test_delete_multiple_lines() {
        let mut ofs_buf = create_numbered_buffer(5, 10);

        // Move cursor to row 2 (1-based) and delete three lines
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(nz(2)),
            col: term_col(nz(1)),
        };
        let delete_lines = CsiSequence::DeleteLine(3);
        let sequence = format!("{move_cursor}{delete_lines}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify lines have shifted up by 3.
        assert_line_content(&ofs_buf, 0, "Line00"); // Line 0 unchanged
        assert_line_content(&ofs_buf, 1, "Line04"); // Line 4 shifted to 1
        assert_blank_line(&ofs_buf, 2); // New blank lines at bottom
        assert_blank_line(&ofs_buf, 3);
        assert_blank_line(&ofs_buf, 4);
        // Lines 1, 2, 3 were deleted.
    }

    #[test]
    fn test_delete_lines_with_margins() {
        let mut ofs_buf = create_numbered_buffer(10, 10);

        // Set scroll margins: rows 3-7 (1-based)
        let set_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(nz(3))),
            bottom: Some(term_row(nz(7))),
        };
        let _result = ofs_buf.apply_ansi_bytes(format!("{set_margins}"));

        // Move cursor to row 5 (1-based, within margins) and delete one line
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(nz(5)),
            col: term_col(nz(1)),
        };
        let delete_line = CsiSequence::DeleteLine(1);
        let sequence = format!("{move_cursor}{delete_line}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify only lines within margins are affected.
        assert_line_content(&ofs_buf, 0, "Line00"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 1, "Line01"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 2, "Line02"); // Top margin, unchanged
        assert_line_content(&ofs_buf, 3, "Line03"); // Within margins, unchanged
        assert_line_content(&ofs_buf, 4, "Line05"); // Line05 shifted to 4
        assert_line_content(&ofs_buf, 5, "Line06"); // Line06 shifted to 5
        assert_blank_line(&ofs_buf, 6); // New blank line at bottom of region
        assert_line_content(&ofs_buf, 7, "Line07"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 8, "Line08"); // Outside margins, unchanged
        assert_line_content(&ofs_buf, 9, "Line09"); // Outside margins, unchanged
        // Line04 was deleted.
    }

    #[test]
    fn test_delete_line_outside_margins_ignored() {
        let mut ofs_buf = create_numbered_buffer(5, 10);
        let performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set scroll margins: rows 2-4 (1-based) = 1-3 (0-based)
        performer.ofs_buf.ansi_parser_support.scroll_region_top = Some(term_row(nz(2)));
        performer.ofs_buf.ansi_parser_support.scroll_region_bottom =
            Some(term_row(nz(4)));

        // Move cursor to row 4 (outside margins)
        performer.ofs_buf.cursor_pos = row(4) + col(0);

        // Try to delete line: ESC[M (should be ignored)
        let delete_line_sequence = format!("{}", CsiSequence::DeleteLine(1));
        let _result = ofs_buf.apply_ansi_bytes(delete_line_sequence);

        // Verify no changes occurred.
        for r in 0..5 {
            assert_line_content(&ofs_buf, r, &format!("Line{r:02}"));
        }
    }
}

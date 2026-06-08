// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for Erase Display (ED) and Erase Line (EL) operations.
//!
//! Tests the complete pipeline from [`ANSI`] sequences through the shim to implementation
//! using the public [`apply_ansi_bytes`] API. This provides integration testing coverage
//! for the [`clear_ops`] shim layer. The `test_` prefix follows our naming convention.
//! See [parser module docs] for the complete testing philosophy.
//!
//! **Related Files:**
//! - **Shim**: [`clear_ops`] - Parameter translation (tested indirectly by this module)
//! - **Implementation**: [`impl_clear_ops`] - Business logic (has separate unit tests)
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes
//! [`clear_ops`]: crate::vt_100_pty_output_parser::operations::vt_100_shim_clear_ops
//! [`impl_clear_ops`]: crate::vt_100_ansi_impl::vt_100_impl_clear_ops
//! [parser module docs]: super::super

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{EraseDisplayMode, EraseLineMode, OffscreenBuffer, PixelChar, TuiStyle, col, core::ansi::{constants::{CSI_START, ED_ERASE_DISPLAY, EL_ERASE_LINE},
                         vt_100_pty_output_parser::CsiSequence}, row};

/// Helper to create a fully filled 5x5 offscreen buffer with 'X' characters.
fn create_filled_5r_by_5c_buffer() -> OffscreenBuffer {
    let mut buf = OffscreenBuffer::new_empty(crate::height(5) + crate::width(5));
    let style = TuiStyle::default();
    let char_x = PixelChar::PlainText {
        display_char: 'X',
        style,
    };
    for r in 0..5 {
        for c in 0..5 {
            buf.buffer[r][c] = char_x;
        }
    }
    buf
}

#[test]
fn test_parameter_defaults() {
    // Missing arguments should default to 0 (FromCursorToEnd).

    // CSI J should be parsed as CSI 0 J (EraseDisplay FromCursorToEnd)
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(2) + col(2);

        // Apply CSI J (no param)
        let csi_j_no_param = format!("{CSI_START}{ED_ERASE_DISPLAY}");
        let _result = ofs_buf.apply_ansi_bytes(csi_j_no_param);

        // Line 2: cursor is at col 2. So cols 2, 3, 4 should be cleared.
        assert_line_content(&ofs_buf, 2, "XX   ");
        // Lines 3 and 4 should be fully cleared.
        assert_line_content(&ofs_buf, 3, "     ");
        assert_line_content(&ofs_buf, 4, "     ");
        // Lines 0 and 1 should remain untouched.
        assert_line_content(&ofs_buf, 0, "XXXXX");
        assert_line_content(&ofs_buf, 1, "XXXXX");
    }

    // CSI K should be parsed as CSI 0 K (EraseLine FromCursorToEnd)
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(2) + col(2);

        // Apply CSI K (no param)
        let csi_k_no_param = format!("{CSI_START}{EL_ERASE_LINE}");
        let _result = ofs_buf.apply_ansi_bytes(csi_k_no_param);

        // Line 2: cursor is at col 2. So cols 2, 3, 4 of Line 2 should be cleared.
        assert_line_content(&ofs_buf, 2, "XX   ");
        // All other lines remain untouched.
        assert_line_content(&ofs_buf, 0, "XXXXX");
        assert_line_content(&ofs_buf, 1, "XXXXX");
        assert_line_content(&ofs_buf, 3, "XXXXX");
        assert_line_content(&ofs_buf, 4, "XXXXX");
    }
}

#[test]
fn test_erase_display_modes() {
    // Mode 0: From Cursor to End
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(1) + col(3);

        // CSI 0 J
        let sequence =
            CsiSequence::EraseDisplay(EraseDisplayMode::FromCursorToEnd).to_string();
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf, 0, "XXXXX");
        assert_line_content(&ofs_buf, 1, "XXX  ");
        assert_line_content(&ofs_buf, 2, "     ");
        assert_line_content(&ofs_buf, 3, "     ");
        assert_line_content(&ofs_buf, 4, "     ");
    }

    // Mode 1: From Start to Cursor
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(2) + col(2);

        // CSI 1 J
        let sequence =
            CsiSequence::EraseDisplay(EraseDisplayMode::FromStartToCursor).to_string();
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf, 0, "     ");
        assert_line_content(&ofs_buf, 1, "     ");
        assert_line_content(&ofs_buf, 2, "   XX");
        assert_line_content(&ofs_buf, 3, "XXXXX");
        assert_line_content(&ofs_buf, 4, "XXXXX");
    }

    // Mode 2: Entire Screen
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(2) + col(2);

        // CSI 2 J
        let sequence =
            CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen).to_string();
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf, 0, "     ");
        assert_line_content(&ofs_buf, 1, "     ");
        assert_line_content(&ofs_buf, 2, "     ");
        assert_line_content(&ofs_buf, 3, "     ");
        assert_line_content(&ofs_buf, 4, "     ");
    }
}

#[test]
fn test_erase_line_modes() {
    // Mode 0: From Cursor to End
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(2) + col(1);

        // CSI 0 K
        let sequence = CsiSequence::EraseLine(EraseLineMode::FromCursorToEnd).to_string();
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf, 2, "X    ");
        assert_line_content(&ofs_buf, 1, "XXXXX");
    }

    // Mode 1: From Start to Cursor
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(2) + col(3);

        // CSI 1 K
        let sequence =
            CsiSequence::EraseLine(EraseLineMode::FromStartToCursor).to_string();
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf, 2, "    X");
        assert_line_content(&ofs_buf, 1, "XXXXX");
    }

    // Mode 2: Entire Line
    {
        let mut ofs_buf = create_filled_5r_by_5c_buffer();
        ofs_buf.cursor_pos = row(2) + col(2);

        // CSI 2 K
        let sequence = CsiSequence::EraseLine(EraseLineMode::EntireLine).to_string();
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf, 2, "     ");
        assert_line_content(&ofs_buf, 1, "XXXXX");
    }
}

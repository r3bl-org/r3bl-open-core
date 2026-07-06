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
//! - **Implementation**: [`vt_100_impl_clear_ops`] - Business logic (has separate unit
//!   tests)
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`apply_ansi_bytes`]: crate::OfsBufVT100::apply_ansi_bytes
//! [`clear_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_clear_ops
//! [`vt_100_impl_clear_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_clear_ops
//! [parser module docs]: super::super

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{EraseDisplayMode, EraseLineMode, OfsBufVT100, PixelChar, TuiStyle, col, height, width, PixelCharLine,
            core::ansi::{constants::{CSI_START, ED_ERASE_DISPLAY, EL_ERASE_LINE},
                         vt_100_pty_output_parser::CsiSequence},
            row};

/// Helper to create a fully filled 5x5 offscreen buffer with 'X' characters.
fn create_filled_5r_by_5c_buffer() -> OfsBufVT100 {
    let mut buf = OfsBufVT100::new_empty(height(5) + width(5));
    let style = TuiStyle::default();
    let char_x = PixelChar::PlainText {
        display_char: 'X',
        style,
    };
    for r in 0..5 {
        for c in 0..5 {
            buf.ofs_buf.get_row_mut(r).unwrap()[c] = char_x;
        }
    }
    buf
}

mod test_parameter_defaults {
    use super::*;

    #[test]
    fn csi_j_defaults_to_mode_0() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(2));

        // Apply CSI J (no param)
        let csi_j_no_param = format!("{CSI_START}{ED_ERASE_DISPLAY}");
        let _result = ofs_buf_vt_100.apply_ansi_bytes(csi_j_no_param);

        // Line 2: cursor is at col 2. So cols 2, 3, 4 should be cleared.
        assert_line_content(&ofs_buf_vt_100, 2, "XX   ");
        // Lines 3 and 4 should be fully cleared.
        assert_line_content(&ofs_buf_vt_100, 3, "     ");
        assert_line_content(&ofs_buf_vt_100, 4, "     ");
        // Lines 0 and 1 should remain untouched.
        assert_line_content(&ofs_buf_vt_100, 0, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 1, "XXXXX");
    }

    #[test]
    fn csi_k_defaults_to_mode_0() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(2));

        // Apply CSI K (no param)
        let csi_k_no_param = format!("{CSI_START}{EL_ERASE_LINE}");
        let _result = ofs_buf_vt_100.apply_ansi_bytes(csi_k_no_param);

        // Line 2: cursor is at col 2. So cols 2, 3, 4 of Line 2 should be cleared.
        assert_line_content(&ofs_buf_vt_100, 2, "XX   ");
        // All other lines remain untouched.
        assert_line_content(&ofs_buf_vt_100, 0, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 1, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 3, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 4, "XXXXX");
    }
}

mod test_erase_display {
    use super::*;

    #[test]
    fn mode_0_from_cursor_to_end() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(1) + col(3));

        // CSI 0 J
        let sequence =
            CsiSequence::EraseDisplay(EraseDisplayMode::FromCursorToEnd).to_string();
        let _result = ofs_buf_vt_100.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf_vt_100, 0, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 1, "XXX  ");
        assert_line_content(&ofs_buf_vt_100, 2, "     ");
        assert_line_content(&ofs_buf_vt_100, 3, "     ");
        assert_line_content(&ofs_buf_vt_100, 4, "     ");
    }

    #[test]
    fn mode_1_from_start_to_cursor() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(2));

        // CSI 1 J
        let sequence =
            CsiSequence::EraseDisplay(EraseDisplayMode::FromStartToCursor).to_string();
        let _result = ofs_buf_vt_100.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf_vt_100, 0, "     ");
        assert_line_content(&ofs_buf_vt_100, 1, "     ");
        assert_line_content(&ofs_buf_vt_100, 2, "   XX");
        assert_line_content(&ofs_buf_vt_100, 3, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 4, "XXXXX");
    }

    #[test]
    fn mode_2_entire_screen() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(2));

        // CSI 2 J
        let sequence =
            CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen).to_string();
        let _result = ofs_buf_vt_100.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf_vt_100, 0, "     ");
        assert_line_content(&ofs_buf_vt_100, 1, "     ");
        assert_line_content(&ofs_buf_vt_100, 2, "     ");
        assert_line_content(&ofs_buf_vt_100, 3, "     ");
        assert_line_content(&ofs_buf_vt_100, 4, "     ");
    }

    #[test]
    fn mode_3_entire_screen_and_scrollback() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(2));

        // Add some scrollback history
        ofs_buf_vt_100.scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0));
        assert_eq!(ofs_buf_vt_100.scrollback_buffer.lines.len(), 1);

        // CSI 3 J
        let sequence =
            CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreenAndScrollback).to_string();
        let _result = ofs_buf_vt_100.apply_ansi_bytes(sequence);

        // Buffer should be UNTOUCHED
        assert_line_content(&ofs_buf_vt_100, 0, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 1, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 2, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 3, "XXXXX");
        assert_line_content(&ofs_buf_vt_100, 4, "XXXXX");

        // Scrollback should be cleared
        assert_eq!(ofs_buf_vt_100.scrollback_buffer.lines.len(), 0);
    }
}

mod test_erase_line {
    use super::*;

    #[test]
    fn mode_0_from_cursor_to_end() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(1));

        // CSI 0 K
        let sequence = CsiSequence::EraseLine(EraseLineMode::FromCursorToEnd).to_string();
        let _result = ofs_buf_vt_100.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf_vt_100, 2, "X    ");
        assert_line_content(&ofs_buf_vt_100, 1, "XXXXX");
    }

    #[test]
    fn mode_1_from_start_to_cursor() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(3));

        // CSI 1 K
        let sequence =
            CsiSequence::EraseLine(EraseLineMode::FromStartToCursor).to_string();
        let _result = ofs_buf_vt_100.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf_vt_100, 2, "    X");
        assert_line_content(&ofs_buf_vt_100, 1, "XXXXX");
    }

    #[test]
    fn mode_2_entire_line() {
        let mut ofs_buf_vt_100 = create_filled_5r_by_5c_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(2));

        // CSI 2 K
        let sequence = CsiSequence::EraseLine(EraseLineMode::EntireLine).to_string();
        let _result = ofs_buf_vt_100.apply_ansi_bytes(sequence);

        assert_line_content(&ofs_buf_vt_100, 2, "     ");
        assert_line_content(&ofs_buf_vt_100, 1, "XXXXX");
    }
}
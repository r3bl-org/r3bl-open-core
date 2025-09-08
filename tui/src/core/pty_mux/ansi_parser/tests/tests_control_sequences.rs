// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for control sequences and edge cases.

use vte::Perform;

use super::tests_fixtures::*;
use crate::{AnsiToOfsBufPerformer, col, core::pty_mux::ansi_parser::esc_codes,
            offscreen_buffer::test_fixtures_offscreen_buffer::*, row};

/// Tests for C0 control characters (CR, LF, Tab, Backspace, etc.).
pub mod control_chars {
    use super::*;

    #[test]
    fn test_control_characters() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Test various control characters
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Print some text
        performer.print('A');
        performer.print('B');
        performer.print('C');

        // Carriage return should move to start of line
        performer.execute(esc_codes::CARRIAGE_RETURN);
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(0) + col(0),
            "Cursor should be at start of line after CR"
        );
        performer.print('X'); // Should overwrite 'A'

        // Line feed should move to next line, but same column
        performer.execute(esc_codes::LINE_FEED);
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(1) + col(1),
            "Cursor should move to next row after LF, but same column"
        );

        // Reset column for next test
        performer.ofs_buf.my_pos.col_index = col(0);
        performer.print('Y');

        // Tab should advance cursor
        performer.execute(esc_codes::TAB);
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(1) + col(8),
            "Cursor should move to col 8 after tab"
        );
        performer.print('Z');

        // Backspace should move cursor back
        performer.ofs_buf.my_pos.col_index = col(3);
        performer.print('M');
        performer.execute(esc_codes::BACKSPACE); // Backspace
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(1) + col(3),
            "Cursor should move back one column after BS, to col 3"
        );
        performer.print('N'); // Should overwrite 'M' at col 3
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(1) + col(4),
            "Cursor should move to col 4 after printing 'N', same row"
        );

        // Verify final ofs_buf cursor position
        assert_eq!(
            ofs_buf.my_pos,
            row(1) + col(4),
            "Final cursor position should be row 1, col 4"
        );

        // Verify buffer contents
        assert_plain_char_at(&ofs_buf, 0, 0, 'X'); // 'A' was overwritten by 'X' after CR
        assert_plain_char_at(&ofs_buf, 0, 1, 'B');
        assert_plain_char_at(&ofs_buf, 0, 2, 'C');

        assert_plain_char_at(&ofs_buf, 1, 0, 'Y'); // After line feed
        assert_plain_char_at(&ofs_buf, 1, 8, 'Z'); // After tab
        assert_plain_char_at(&ofs_buf, 1, 3, 'N'); // N overwrote M at position 3
    }
}

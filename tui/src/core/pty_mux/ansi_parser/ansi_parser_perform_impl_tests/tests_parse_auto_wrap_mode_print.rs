// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for auto-wrap mode functionality in print operations.

use vte::Perform;

use super::tests_parse_common::create_test_offscreen_buffer_10r_by_10c;
use crate::{Pos,
            ansi_parser::{ansi_parser_perform_impl::new,
                          ansi_parser_public_api::AnsiToBufferProcessor,
                          csi_codes::{CSI_PRIVATE_MODE_PREFIX, CSI_START,
                                      DECAWM_AUTO_WRAP, RM_RESET_PRIVATE_MODE,
                                      SM_SET_PRIVATE_MODE}},
            col,
            offscreen_buffer::test_fixtures_offscreen_buffer::*,
            row};

/// Helper to set DECAWM mode via ANSI sequence
fn test_decawm_with_sequence(processor: &mut AnsiToBufferProcessor, enable: bool) {
    let command = if enable {
        SM_SET_PRIVATE_MODE
    } else {
        RM_RESET_PRIVATE_MODE
    };
    let sequence = format!(
        "{a}{b}{c}{d}",
        a = CSI_START,
        b = CSI_PRIVATE_MODE_PREFIX,
        c = DECAWM_AUTO_WRAP,
        d = command
    );
    let mut parser = vte::Parser::new();
    for byte in sequence.bytes() {
        parser.advance(processor, byte);
    }
}

#[test]
fn test_decawm_default_enabled() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Auto-wrap mode default enabled test:
    //
    // Column:  0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │ ← First 10 chars fill line
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 1:  │ K │   │   │   │   │   │   │   │   │   │ ← 11th char wraps to next line
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    //             ╰─ cursor ends here (1,1)
    //
    // Sequence: Write 11 characters A-K (DECAWM enabled by default)

    {
        let mut processor = new(&mut ofs_buf);

        // DECAWM should be enabled by default
        assert!(processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Position at end of line and write 11 characters
        processor.cursor_pos = row(0) + col(0);

        for i in 0..11 {
            let ch = (b'A' + i) as char;
            processor.print(ch);
        }

        // Should wrap to next line after 10 characters
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 1);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 1);
    }

    // Verify first line has A-J
    assert_plain_text_at(&ofs_buf, 0, 0, "ABCDEFGHIJ");
    // Verify K wrapped to next line
    assert_plain_char_at(&ofs_buf, 1, 0, 'K');
}

#[test]
fn test_decawm_disabled_overwrites() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Auto-wrap mode disabled overwrite test:
    //
    // Column:  0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │ A │ B │ C │ D │ E │ F │ G │ H │ I │ L │ ← chars J,K,L overwrite at col 9
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   cursor stays at right margin
    // Row 1:  │   │   │   │   │   │   │   │   │   │   │ ← second line remains empty
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    //                                                 ╰─ cursor stays here (0,9)
    //
    // Sequence: ESC[?7l (disable wrap) → Write 12 characters A-L

    {
        let mut processor = new(&mut ofs_buf);

        // Disable auto-wrap mode
        test_decawm_with_sequence(&mut processor, false);
        assert!(!processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Position at end of line and write 12 characters
        processor.cursor_pos = row(0) + col(0);

        for i in 0..12 {
            let ch = (b'A' + i) as char;
            processor.print(ch);
        }

        // Should stay at right margin (column 9)
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 0);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);
    }

    // Verify first 9 characters are A-I
    assert_plain_text_at(&ofs_buf, 0, 0, "ABCDEFGHI");
    // Verify last character is L (overwrote J and K)
    assert_plain_char_at(&ofs_buf, 0, 9, 'L');

    // Verify second line is empty
    for col in 0..10 {
        assert_empty_at(&ofs_buf, 1, col);
    }
}

#[test]
fn test_decawm_re_enable() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Auto-wrap mode re-enable test:
    //
    // Column:  0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 2:  │   │   │   │   │   │   │   │   │ X │ Y │ ← start at col 8, write X,Y
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 3:  │ Z │   │   │   │   │   │   │   │   │   │ ← Z wraps to next line
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    //             ╰─ cursor ends here (3,1)
    //
    // Sequence: ESC[?7l → ESC[?7h → Move to (2,8) → Write X,Y,Z

    {
        let mut processor = new(&mut ofs_buf);

        // Disable then re-enable auto-wrap mode
        test_decawm_with_sequence(&mut processor, false);
        test_decawm_with_sequence(&mut processor, true);
        assert!(processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Test wrapping works again
        processor.cursor_pos = row(2) + col(8);

        // Write 3 characters (should wrap after 2)
        processor.print('X');
        processor.print('Y');
        processor.print('Z');

        // Should wrap to next line
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 1);
    }

    // Verify X and Y on line 2
    assert_plain_char_at(&ofs_buf, 2, 8, 'X');
    assert_plain_char_at(&ofs_buf, 2, 9, 'Y');
    // Verify Z wrapped to line 3
    assert_plain_char_at(&ofs_buf, 3, 0, 'Z');
}

#[test]
fn test_decawm_mode_switching() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Auto-wrap mode switching test:
    //
    // Column:  0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │   │   │   │   │   │   │   │   │ A │ B │ ← start at col 8, write A,B
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 1:  │ C │   │   │   │   │   │   │   │   │ E │ ← C wraps, then disable wrap
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘   D,E overwrite at col 9
    //
    // Sequence:
    // 1. Start at (0,8) → Write A,B,C (C wraps due to DECAWM enabled)
    // 2. ESC[?7l (disable wrap) → Move to (1,9) → Write D,E (E overwrites D)

    {
        let mut processor = new(&mut ofs_buf);

        // Start with enabled (default), write some chars
        processor.cursor_pos = Pos {
            row_index: row(0),
            col_index: col(8),
        };
        processor.print('A');
        processor.print('B');
        processor.print('C'); // Should wrap

        assert_eq!(processor.cursor_pos.row_index.as_usize(), 1);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 1);

        // Disable wrap mode
        test_decawm_with_sequence(&mut processor, false);

        // Move to end of line 1 and write more
        processor.cursor_pos = Pos {
            row_index: row(1),
            col_index: col(9),
        };
        processor.print('D');
        processor.print('E'); // Should overwrite D

        assert_eq!(processor.cursor_pos.row_index.as_usize(), 1);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);
    }

    // Verify line 0 has A, B at end
    assert_plain_char_at(&ofs_buf, 0, 8, 'A');
    assert_plain_char_at(&ofs_buf, 0, 9, 'B');
    // Verify C wrapped to line 1
    assert_plain_char_at(&ofs_buf, 1, 0, 'C');
    // Verify E overwrote D at end of line 1
    assert_plain_char_at(&ofs_buf, 1, 9, 'E');
}

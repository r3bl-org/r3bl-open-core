// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for cursor movement operations.

use vte::Perform;

use super::tests_parse_common::create_test_offscreen_buffer_10r_by_10c;
use crate::{ansi_parser::ansi_parser_perform_impl::{cursor_ops, new},
            col,
            offscreen_buffer::test_fixtures_offscreen_buffer::*,
            row};

#[test]
fn test_cursor_movement_up() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
    // 1-based index.
    //
    // Cursor up movement pattern:
    //
    // Column:   0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │   │   │   │ C │   │   │   │   │   │   │ ← 3. final position (up
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   beyond boundary)
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 3:  │   │   │   │ B │   │   │   │   │   │   │ ← 2. up 2 rows from row 5
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 5:  │   │   │   │ A │   │   │   │   │   │   │ ← 1. start position
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘

    // Test cursor up movement with buffer verification
    {
        let mut processor = new(&mut ofs_buf);

        // 1. Start at row 5, write a character
        processor.cursor_pos = row(5) + col(3);
        processor.print('A');

        // 2. Move up 2 rows and write another character
        cursor_ops::cursor_up(&mut processor, 2);
        assert_eq!(processor.cursor_pos.row_index, row(3));
        assert_eq!(processor.cursor_pos.col_index, col(4)); // Column should be after 'A'

        processor.cursor_pos.col_index = col(3); // Reset column to same position
        processor.print('B');

        // 3. Try to move up beyond boundary
        cursor_ops::cursor_up(&mut processor, 10);
        assert_eq!(processor.cursor_pos.row_index, row(0)); // Should stop at row 0
        processor.cursor_pos.col_index = col(3);
        processor.print('C');
    }

    // Verify characters are in correct positions
    assert_plain_char_at(&ofs_buf, 5, 3, 'A');
    assert_plain_char_at(&ofs_buf, 3, 3, 'B');
    assert_plain_char_at(&ofs_buf, 0, 3, 'C');
}

#[test]
fn test_cursor_movement_down() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
    // 1-based index.
    //
    // Cursor down movement pattern:
    //
    // Column:   0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 2:  │   │   │   │   │ X │   │   │   │   │   │ ← 1. start position
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 5:  │   │   │   │ Y │   │   │   │   │   │   │ ← 2. down 3 rows from row 2
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 9:  │   │   │   │ Z │   │   │   │   │   │   │ ← 3. final position (down
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘   beyond boundary)

    // Test cursor down movement with buffer verification
    {
        let mut processor = new(&mut ofs_buf);

        // 1. Start at row 2, write a character
        processor.cursor_pos = row(2) + col(4);
        processor.print('X');

        // 2. Move down 3 rows and write another character
        cursor_ops::cursor_down(&mut processor, 3);
        assert_eq!(processor.cursor_pos.row_index, row(5));
        assert_eq!(processor.cursor_pos.col_index, col(5)); // Column should be after 'X'

        processor.cursor_pos.col_index = col(3); // Reset column to same position
        processor.print('Y');

        // 3. Try to move down beyond buffer area
        cursor_ops::cursor_down(&mut processor, 10);
        assert_eq!(processor.cursor_pos.row_index, row(9)); // Should stop at row 9 (last row)
        processor.cursor_pos.col_index = col(3);
        processor.print('Z');
    }

    // Verify characters are in correct positions
    assert_plain_char_at(&ofs_buf, 2, 4, 'X');
    assert_plain_char_at(&ofs_buf, 5, 3, 'Y'); // col 3 because we reset it
    assert_plain_char_at(&ofs_buf, 9, 3, 'Z'); // col 3 because we reset it, row 9 is now valid
}

#[test]
fn test_cursor_movement_forward() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
    // 1-based index.

    // Cursor forward movement with gaps:
    //
    // Column:   0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 3:  │ 1 │   │   │   │ 2 │   │   │   │ 3 │ 4 │ ← cursor wraps to (4,0)
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   after '4'
    // Row 4:  │ ← cursor here after line wrap ┼───┼───┤
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    //
    // Sequence: '1' → forward(3) → '2' → forward(3) → '3' → forward(5) → '4'

    // Test cursor forward movement with buffer verification
    {
        let mut processor = new(&mut ofs_buf);

        // 1. Write some text at row 3
        processor.cursor_pos = row(3) + col(0);
        processor.print('1');

        // 2. Move forward 3 columns and write
        cursor_ops::cursor_forward(&mut processor, 3);
        assert_eq!(processor.cursor_pos.col_index, col(4));
        processor.print('2');

        // 3. Move forward to near end of line
        cursor_ops::cursor_forward(&mut processor, 3);
        assert_eq!(processor.cursor_pos.col_index, col(8));
        processor.print('3');

        // 4. Try to move beyond line boundary
        cursor_ops::cursor_forward(&mut processor, 5);
        assert_eq!(processor.cursor_pos.col_index, col(9)); // Should stop at last column
        processor.print('4');
    }

    // Verify characters are in correct positions
    assert_plain_char_at(&ofs_buf, 3, 0, '1');
    assert_plain_char_at(&ofs_buf, 3, 4, '2');
    assert_plain_char_at(&ofs_buf, 3, 8, '3');
    assert_plain_char_at(&ofs_buf, 3, 9, '4');

    // Verify gaps are empty
    for col in [1, 2, 3, 5, 6, 7] {
        assert_empty_at(&ofs_buf, 3, col);
    }
}

#[test]
fn test_cursor_movement_backward() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
    // 1-based index.
    //
    // Cursor backward movement with gaps:
    //
    // Column:   0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 4:  │ D │   │   │ C │   │   │ B │   │ A │   │
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    //
    // Sequence: 'A'@8 → back(3) → 'B'@6 → back(4) → 'C'@3 → back(10) → 'D'@0

    // Test cursor backward movement with buffer verification
    {
        let mut processor = new(&mut ofs_buf);

        // 1. Start at column 8, row 4 and write
        processor.cursor_pos = row(4) + col(8);
        processor.print('A');

        // 2. Move backward 3 columns and write
        cursor_ops::cursor_backward(&mut processor, 3);
        assert_eq!(processor.cursor_pos.col_index, col(6)); // 9 - 3 = 6
        processor.print('B');

        // 3. Move backward more
        cursor_ops::cursor_backward(&mut processor, 4);
        assert_eq!(processor.cursor_pos.col_index, col(3)); // 7 - 4 = 3
        processor.print('C');

        // 4. Try to move beyond start of line
        cursor_ops::cursor_backward(&mut processor, 10);
        assert_eq!(processor.cursor_pos.col_index, col(0)); // Should stop at column 0
        processor.print('D');
    }

    // Verify characters are in correct positions
    assert_plain_char_at(&ofs_buf, 4, 8, 'A');
    assert_plain_char_at(&ofs_buf, 4, 6, 'B');
    assert_plain_char_at(&ofs_buf, 4, 3, 'C');
    assert_plain_char_at(&ofs_buf, 4, 0, 'D');

    // Verify gaps are empty
    for col in [1, 2, 4, 5, 7, 9] {
        assert_empty_at(&ofs_buf, 4, col);
    }
}

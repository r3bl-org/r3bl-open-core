// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for CSI absolute positioning commands.

use vte::Perform;

use super::tests_parse_common::create_test_offscreen_buffer_10r_by_10c;
use crate::{ansi_parser::{ansi_parser_perform_impl::{new, process_bytes},
                          csi_codes::CsiSequence},
            col,
            offscreen_buffer::test_fixtures_offscreen_buffer::*,
            row};

#[test]
fn test_csi_h_home_position() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Start at a non-home position
        processor.cursor_pos = row(5) + col(5);
        processor.print('X');

        // Send ESC[H to move to home position (1,1)
        let sequence = CsiSequence::CursorPosition { row: 1, col: 1 }.to_string();
        process_bytes(&mut processor, &mut parser, sequence);

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        // Verify cursor is at home position (0,0 in 0-based indexing)
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 0);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);

        processor.print('H'); // Mark home position
    }

    // Verify characters are at correct positions
    assert_plain_char_at(&ofs_buf, 5, 5, 'X');
    assert_plain_char_at(&ofs_buf, 0, 0, 'H');

    // Verify final cursor position in buffer
    assert_eq!(ofs_buf.my_pos.row_index.as_usize(), 0);
    assert_eq!(ofs_buf.my_pos.col_index.as_usize(), 1); // After writing 'H'
}

#[test]
fn test_csi_h_specific_position() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Send ESC[5;10H to move to row 5, column 10 (1-based)
        let sequence = CsiSequence::CursorPosition { row: 5, col: 10 }.to_string();
        process_bytes(&mut processor, &mut parser, sequence);

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        // Verify cursor is at (4,9) in 0-based indexing
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 4);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);

        processor.print('A');
    }

    // Verify character was written at correct position
    assert_plain_char_at(&ofs_buf, 4, 9, 'A');
}

#[test]
fn test_csi_h_with_boundary_clamping() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
    // 1-based index.
    //
    // Boundary clamping test: ESC[999;999H should clamp to bottom-right corner
    //
    // Column:   0   1   2   3   4   5   6   7   8   9
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    // Row 0:  │   │   │   │   │   │   │   │   │   │   │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
    // Row 9:  │   │   │   │   │   │   │   │   │   │ E │ ← ESC[999;999H clamped here
    //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘

    {
        let mut processor = new(&mut ofs_buf);

        // Try to position beyond buffer bounds: ESC[999;999H
        let sequence = CsiSequence::CursorPosition { row: 999, col: 999 }.to_string();
        process_bytes(&mut processor, &mut parser, sequence);

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        // Should be clamped to bottom-right corner
        // Buffer is 10x10, so max is row 10 (index 9), col 10 (index 9)
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 9); // Row 10 in 1-based, 9 in 0-based
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 9);

        processor.print('E'); // Mark edge position
    }

    // Verify character at clamped position
    assert_plain_char_at(&ofs_buf, 9, 9, 'E');
}

#[test]
fn test_csi_f_alternate_form() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // ESC[f is alternate form of ESC[H (should go to home)
        let sequence = CsiSequence::CursorPositionAlt { row: 1, col: 1 }.to_string();
        process_bytes(&mut processor, &mut parser, sequence);

        assert_eq!(processor.cursor_pos.row_index.as_usize(), 0);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 0);

        processor.print('F');

        // ESC[3;7f should position at row 3, col 7
        let sequence = CsiSequence::CursorPositionAlt { row: 3, col: 7 }.to_string();
        process_bytes(&mut processor, &mut parser, sequence);

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 2); // Row 3 in 1-based
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 6); // Col 7 in 1-based

        processor.print('G');
    }

    assert_plain_char_at(&ofs_buf, 0, 0, 'F');
    assert_plain_char_at(&ofs_buf, 2, 6, 'G');
}

#[test]
fn test_csi_position_with_missing_params() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Start at non-home position
        processor.cursor_pos = row(3) + col(3);

        // ESC[;5H - missing row param, should default to 1
        // Note: Using raw string as CsiSequence doesn't support missing params (it is
        // always valid)
        let sequence = "\x1b[;5H";
        process_bytes(&mut processor, &mut parser, sequence);

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 0); // Row 1 (default)
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 4); // Col 5

        processor.print('M');

        // ESC[3;H - missing col param, should default to 1
        // Note: Using raw string as CsiSequence doesn't support missing params (it is
        // always valid)
        let sequence = "\x1b[3;H";
        process_bytes(&mut processor, &mut parser, sequence);

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 2); // Row 3
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 0); // Col 1 (default)

        processor.print('N');
    }

    assert_plain_char_at(&ofs_buf, 0, 4, 'M');
    assert_plain_char_at(&ofs_buf, 2, 0, 'N');
}

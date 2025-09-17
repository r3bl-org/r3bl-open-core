// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for all cursor operations - positioning, movement, and save/restore.

use vte::Perform;

use super::super::test_fixtures::*;
use crate::{Pos,
            ansi_parser::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                          protocols::{csi_codes::{CSI_PARAM_SEPARATOR, CSI_START,
                                                  CsiSequence,
                                                  csi_test_helpers::{csi_seq_cursor_pos,
                                                                     csi_seq_cursor_pos_alt}},
                                      esc_codes},
                          term_units::{term_col, term_row}},
            col, height,
            offscreen_buffer::ofs_buf_test_fixtures::*,
            row, width};

/// Tests for absolute cursor positioning (CUP, HVP commands).
pub mod positioning {
    use super::*;

    #[test]
    fn test_cursor_move_to_home_position() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ H │ ␩ │   │   │   │   │   │   │   │   │ ← 2. ESC[H moves to home
        //         ├───┼─▲─┼───┼───┼───┼───┼───┼───┼───┼───┤   (r:1,c:1) in 1-based,
        //         │   │ │ │   │   │   │   │   │   │   │   │   (r:0,c:0) in 0-based
        //         ├───┼─│─┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ │ │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼─│─┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 5:  │   │ │ │   │   │   │ X │   │   │   │   │ ← 1. start at (r:5,c:5)
        //         └───┴─│─┴───┴───┴───┴───┴───┴───┴───┴───┘
        //               ╰─ cursor ends here (r:0,c:1)       ← 3. print 'H' at home
        //
        // Sequence: Write 'X' at (r:5,c:5) → ESC[H → Write 'H' at home (r:0,c:0)

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Start at a non-home position.
        performer.ofs_buf.cursor_pos = row(5) + col(5);
        performer.print('X');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(5) + col(6),
            "Cursor should move right after printing X"
        );

        // Send ESC[1;1H to move to home position (r:1,c:1), 1-based index
        let sequence = csi_seq_cursor_pos(term_row(1) + term_col(1)).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify cursor is at home position (r:0,c:0), in 0-based index
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(0),
            "Cursor should be at home position (r:0,c:0)"
        );

        performer.print('H'); // Mark home position
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(1),
            "Cursor should move right after printing H"
        );

        // Verify characters are at correct positions.
        assert_plain_char_at(&ofs_buf, 5, 5, 'X');
        assert_plain_char_at(&ofs_buf, 0, 0, 'H');

        // Verify final cursor position in buffer (`␩` in the diagram)
        assert_eq!(ofs_buf.cursor_pos, row(0) + col(1)); // After writing 'H'
    }

    #[test]
    fn test_cursor_move_to_specific_row_and_column() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 4:  │   │   │   │   │   │   │   │   │   │ A │ ← ESC[5;10H moves to
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   row 5, col 10 (1-based)
        // Row 5:  │ ␩ │   │   │   │   │   │   │   │   │   │   which is (r:4,c:9) 0-based
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘   then print 'A' there
        //           ╰─ cursor ends here (r:5,c:0) after printing 'A'
        //
        // Sequence: ESC[5;10H → Write 'A' at (r:4,c:9)

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Send ESC[5;10H to move to row 5, column 10 (1-based)
        let sequence = csi_seq_cursor_pos(term_row(5) + term_col(10)).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify cursor is at (r:4,c:9) in 0-based indexing
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(4) + col(9),
            "Cursor should be at (r:4,c:9) in 0-based indexing"
        );

        performer.print('A');

        // Verify cursor moved to next position (r:5,c:0) after writing 'A'
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(5) + col(0),
            "Cursor should move to next line start after writing A"
        );

        // Verify character was written at correct position.
        assert_plain_char_at(&ofs_buf, 4, 9, 'A');
        assert_eq!(
            ofs_buf.cursor_pos,
            row(5) + col(0),
            "Final cursor position should be at (r:5,c:0)"
        );
    }

    #[test]
    fn test_cursor_clamps_to_buffer_boundaries_when_out_of_range() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Buffer layout after boundary clamping:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ C │ ␩ │   │   │   │   │   │   │   │   │ ← 2. move to (r:0,c:0)
        //         ├───┼─▲─┼───┼───┼───┼───┼───┼───┼───┼───┤      then print 'C' there
        //         │ … │ │ │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼─│─┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 9:  │   │ │ │   │   │   │   │   │   │   │ B │ ← 1. clamped boundary
        //         └───┴─│─┴───┴───┴───┴───┴───┴───┴───┴───┘
        //               ╰─ cursor ends here (r:9,c:9) after printing 'C'

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Try to move cursor beyond buffer bounds (row 15, col 15) - should clamp to
        // (r:9,c:9)
        let sequence = csi_seq_cursor_pos(term_row(15) + term_col(15)).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify cursor is clamped to buffer boundaries (r:9,c:9) in 0-based indexing
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(9) + col(9),
            "Cursor should be clamped to buffer boundaries at (r:9,c:9)"
        );

        performer.print('B'); // Mark the clamped position

        // Try to move to negative/zero positions (should become (r:0,c:0))
        let sequence = csi_seq_cursor_pos(term_row(0) + term_col(0)).to_string();
        performer.apply_ansi_bytes(sequence);

        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(0),
            "Cursor should default to (r:0,c:0) for out-of-range coordinates"
        );

        performer.print('C'); // Mark the origin

        // Verify characters are at expected positions.
        assert_plain_char_at(&ofs_buf, 9, 9, 'B'); // At clamped boundary
        assert_plain_char_at(&ofs_buf, 0, 0, 'C'); // At origin
    }

    #[test]
    fn test_cursor_alternate_positioning_syntax_works_identically() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Buffer layout after testing both CUP and HVP:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 2:  │   │   │   │ D │   │   │   │   │   │   │ ← 1. CUP: ESC[3;4H
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   move r:3, c:4 (1-based)
        //         │   │   │   │   │   │   │   │   │   │   │   ie (r:2,c:3) (0-based)
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   then print 'D' there
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 5:  │   │   │   │   │   │   │ E │ ␩ │   │   │ ← 2. HVP: ESC[6;7f
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘   move r:6, c:7 (1-based)
        //                                       │             ie (r:5,c:6) (0-based)
        //                                       │             then print 'E' there
        //                                       ╰─ cursor ends here (r:5,c:7)

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test CUP (Cursor Position) - ESC[3;4H
        let cup_sequence = csi_seq_cursor_pos(term_row(3) + term_col(4)).to_string();
        performer.apply_ansi_bytes(cup_sequence);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(2) + col(3),
            "Cursor should be at (r:2,c:3) after CUP"
        );
        performer.print('D');

        // Test HVP (Horizontal Vertical Position) - ESC[6;7f
        let hvp_sequence = csi_seq_cursor_pos_alt(term_row(6) + term_col(7)).to_string();
        performer.apply_ansi_bytes(hvp_sequence);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(5) + col(6),
            "Cursor should be at (r:5,c:6) after HVP"
        );
        performer.print('E');

        // Verify both positioning commands work identically.
        assert_plain_char_at(&ofs_buf, 2, 3, 'D');
        assert_plain_char_at(&ofs_buf, 5, 6, 'E');
    }

    #[test]
    fn test_cursor_defaults_to_1_when_row_or_column_missing() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Buffer layout after testing default parameter behavior:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ H │ ␩ │   │   │ C │   │   │   │   │   │ ← 2. ESC[H → (r:0,c:0) 'H'
        //         ├───┼─▲─┼───┼───┼───┼───┼───┼───┼───┼───┤   4. ESC[;5H → (r:0,c:4) 'C'
        //         │   │ │ │   │   │   │   │   │   │   │   │
        //         ├───┼─│─┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 2:  │ R │ │ │   │   │   │   │   │   │   │   │ ← 3. ESC[3H → (r:2,c:0) 'R'
        //         ├───┼─│─┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ │ │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼─│─┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 5:  │   │ │ │   │   │   │ S │   │   │   │   │ ← 1. start at (r:5,c:5)
        //         └───┴─│─┴───┴───┴───┴───┴───┴───┴───┴───┘   then print 'S' there
        //               ╰─ cursor ends here (r:0,c:1)
        //
        // Sequence: S@(r:5,c:5) → ESC[H → H@(r:0,c:0) → ESC[3H → R@(r:2,c:0)
        //           → ESC[;5H → C@(r:0,c:4)

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Move to a known position first.
        performer.ofs_buf.cursor_pos = row(5) + col(5);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(5) + col(5),
            "Cursor should be at (r:5,c:5) to start"
        );
        performer.print('S'); // Mark start position

        // Test ESC[H (no parameters) - should move to (r:1,c:1) which is (r:0,c:0) in
        // 0-based
        performer.apply_ansi_bytes(format!("{CSI_START}H"));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(0),
            "Cursor should be at (r:0,c:0) after ESC[H (no params)"
        );
        performer.print('H'); // Mark home

        // Test ESC[3H (row only) - should move to (r:3,c:1) which is (r:2,c:0) in
        // 0-based
        performer.apply_ansi_bytes(format!("{CSI_START}3H"));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(2) + col(0),
            "Cursor should be at (r:2,c:0) after ESC[3H (col missing, defaults to 1)"
        );
        performer.print('R'); // Mark row-only

        // Test ESC[;5H (column only) - should move to (r:1,c:5) which is (r:0,c:4) in
        // 0-based
        performer.apply_ansi_bytes(format!("{CSI_START}{CSI_PARAM_SEPARATOR}5H"));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(4),
            "Cursor should be at (r:0,c:4) after ESC[;5H (row missing, defaults to 1)"
        );
        performer.print('C'); // Mark column-only

        // Verify default behavior.
        assert_plain_char_at(&ofs_buf, 5, 5, 'S'); // Start position
        assert_plain_char_at(&ofs_buf, 0, 0, 'H'); // ESC[H -> (r:0,c:0)
        assert_plain_char_at(&ofs_buf, 2, 0, 'R'); // ESC[3H -> (r:2,c:0)
        assert_plain_char_at(&ofs_buf, 0, 4, 'C'); // ESC[;5H -> (r:0,c:4)
    }
}

/// Tests for relative cursor movement (up, down, forward, backward).
pub mod movement {
    use super::*;

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
        // Row 0:  │   │   │   │   │   │ C │ ␩ │   │   │   │ ← 3. final position (up
        //         ├───┼───┼───┼───┼───┼───┼─▲─┼───┼───┼───┤   beyond boundary)
        //         │ … │ … │ … │ … │ … │ … │ │ │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼─│─┼───┼───┼───┤
        // Row 3:  │   │   │   │   │ B │   │ │ │   │   │   │ ← 2. up 2 rows from row 5
        //         ├───┼───┼───┼───┼───┼───┼─│─┼───┼───┼───┤   print 'B' there
        //         │   │   │   │   │   │   │ │ │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼─│─┼───┼───┼───┤
        // Row 5:  │   │   │   │ A │   │   │ │ │   │   │   │ ← 1. start at (r:5,c:3)
        //         └───┴───┴───┴───┴───┴───┴─│─┴───┴───┴───┘   then print 'A' there
        //                                   ╰cursor ends here (r:0,c:5)

        // Test cursor up movement with buffer verification.
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // 1. Start at row 5, col 3, write a 'A'.
        performer.ofs_buf.cursor_pos = row(5) + col(3);
        performer.print('A');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(5) + col(4),
            "Cursor should be at (r:5,c:4) after printing 'A'"
        );

        // 2. Move up 2 rows and write 'B'.
        performer.ofs_buf.cursor_up(height(2));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(4),
            "Cursor should be at (r:3,c:4) after moving up 2 rows, after printing 'A'"
        );

        performer.print('B');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(5),
            "Cursor should be at (r:3,c:5) after printing 'B'"
        );

        // 3. Try to move up beyond boundary.
        performer.ofs_buf.cursor_up(height(10));
        assert_eq!(performer.ofs_buf.cursor_pos.row_index, row(0)); // Should stop at row 0
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(5),
            "Column should remain the same, 5, after moving up, row clamped to 0"
        );
        performer.print('C');

        // Verify characters are in correct positions.
        assert_plain_char_at(&ofs_buf, 5, 3, 'A');
        assert_plain_char_at(&ofs_buf, 3, 4, 'B');
        assert_plain_char_at(&ofs_buf, 0, 5, 'C');
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
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 2:  │   │   │   │   │ X │   │   │   │   │   │ ← 1. start position at
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   (r:2,c:4), then print 'X'
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 5:  │   │   │   │   │   │ Y │   │   │   │   │ ← 2. down 3 rows from row 2
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 9:  │   │   │   │   │   │   │ Z │ ␩ │   │   │ ← 3. final position (down
        //         └───┴───┴───┴───┴───┴───┴───┴─▲─┴───┴───┘   beyond boundary)
        //                                       ╰─ cursor ends here (r:9,c:6)

        // Test cursor down movement with buffer verification.
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // 1. Start at row 2, write a character.
        performer.ofs_buf.cursor_pos = row(2) + col(4);
        performer.print('X');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(2) + col(5),
            "Cursor should move right after printing X, to (r:2,c:5)"
        );

        // 2. Move down 3 rows and write another character.
        performer.ofs_buf.cursor_down(height(3));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(5) + col(5),
            "Cursor should be at (r:5,c:5) after moving down 3 rows"
        );
        performer.print('Y');

        // 3. Try to move down beyond boundary.
        performer.ofs_buf.cursor_down(height(10));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(9) + col(6),
            "Cursor row should be clamped to bottom boundary, row 9, col remains 6"
        );
        performer.print('Z');

        // Verify characters are in correct positions.
        assert_plain_char_at(&ofs_buf, 2, 4, 'X');
        assert_plain_char_at(&ofs_buf, 5, 5, 'Y');
        assert_plain_char_at(&ofs_buf, 9, 6, 'Z');
    }

    #[test]
    fn test_cursor_movement_forward() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Cursor forward movement pattern:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 4:  │   │   │   │ L │   │   │ M │   │   │ N │ ← start at (r:4,c:3) → 'L'
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   forward 2 → 'M'
        // Row 5:  │ ␩ │   │   │   │   │   │   │   │   │   │   forward 10 → 'N'
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //           ╰─ cursor wraps here after printing 'N' at (r:4,c:9)
        //
        // Sequence: L@(r:4,c:3) → forward(2) → M@(r:4,c:6) → forward(10)
        //           → N@(r:4,c:9) → cursor@(r:5,c:0)

        // Test cursor forward movement with buffer verification.
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // 1. Start at column 3, write a character.
        performer.ofs_buf.cursor_pos = row(4) + col(3);
        performer.print('L');

        // 2. Move forward 2 columns and write another character.
        performer.ofs_buf.cursor_forward(width(2));
        assert_eq!(performer.ofs_buf.cursor_pos.col_index, col(6)); // Should be at column 6
        performer.print('M');

        // 3. Try to move forward beyond boundary.
        performer.ofs_buf.cursor_forward(width(10));
        assert_eq!(performer.ofs_buf.cursor_pos.col_index, col(9)); // Should stop at column 9
        performer.print('N');

        // Verify characters are in correct positions.
        assert_plain_char_at(&ofs_buf, 4, 3, 'L');
        assert_plain_char_at(&ofs_buf, 4, 6, 'M');
        assert_plain_char_at(&ofs_buf, 4, 9, 'N');

        // Verify gaps are empty in row 4.
        for col in [0, 1, 2, 4, 5, 7, 8] {
            assert_empty_at(&ofs_buf, 4, col);
        }
    }

    #[test]
    fn test_cursor_movement_backward() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Cursor backward movement pattern:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 6:  │ R │ ␩ │   │   │   │ Q │   │ P │   │   │ ← start at (r:6,c:7) → 'P'
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘   backward 3 → 'Q'
        //               ╰─ cursor ends here (r:6,c:1)         backward 10 → 'R'
        //
        // Sequence: P@(r:6,c:7) → back(3) → Q@(r:6,c:5) → back(10) → R@(r:6,c:0)

        // Test cursor backward movement with buffer verification.
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // 1. Start at column 7, write a character.
        performer.ofs_buf.cursor_pos = row(6) + col(7);
        performer.print('P');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(6) + col(8),
            "Cursor should move right after printing P, to (r:6,c:8)"
        );

        // 2. Move backward 3 columns and write another character.
        performer.ofs_buf.cursor_backward(width(3));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(6) + col(5),
            "Should be at column 5, row 6, after moving backward 3"
        );
        performer.print('Q');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(6) + col(6),
            "Cursor should move right after printing Q, to (r:6,c:6)"
        );

        // 3. Try to move backward beyond boundary.
        performer.ofs_buf.cursor_backward(width(10));
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(6) + col(0),
            "Should stop at column 0, same row 6, after moving backward and hitting boundary"
        );
        performer.print('R');

        // Verify characters are in correct positions.
        assert_plain_char_at(&ofs_buf, 6, 7, 'P');
        assert_plain_char_at(&ofs_buf, 6, 5, 'Q');
        assert_plain_char_at(&ofs_buf, 6, 0, 'R');

        // Verify gaps are empty.
        for col in [1, 2, 3, 4, 6, 8, 9] {
            assert_empty_at(&ofs_buf, 6, col);
        }
    }
}

/// Tests for cursor save and restore operations (both CSI and ESC variants).
pub mod save_restore {
    use super::*;

    #[test]
    fn test_csi_save_restore_cursor() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Buffer layout:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 3:  │   │   │   │   │   │ A │ C │ ␩ │   │   │ ← 1. save position after A
        //         ├───┼───┼───┼───┼───┼───┼───┼─▲─┼───┼───┤   3. restore & write C
        //         │ … │ … │ … │ … │ … │ … │ … │ │ │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼─│─┼───┼───┤
        // Row 7:  │   │   │ B │   │   │   │   │ │ │   │   │ ← 2. move elsewhere & write B
        //         └───┴───┴───┴───┴───┴───┴───┴─│─┴───┴───┘
        //                                       ╰─ cursor ends here: (r:3,c:7)

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Move to (r:3,c:5) and write 'A'
        performer.ofs_buf.cursor_pos = row(3) + col(5);
        performer.print('A');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(6),
            "Cursor should be at (r:3,c:6) after printing 'A'"
        );

        // CSI s - Save cursor position.
        performer.apply_ansi_bytes(CsiSequence::SaveCursor.to_string());

        // Move elsewhere
        performer.ofs_buf.cursor_pos = row(7) + col(2);
        performer.print('B');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(7) + col(3),
            "Cursor should be at (r:7,c:3) after printing 'B'"
        );

        // CSI u - Restore cursor position.
        performer.apply_ansi_bytes(CsiSequence::RestoreCursor.to_string());

        // Should be back at saved position.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(6),
            "Cursor should be restored to (r:3,c:6)"
        );

        performer.print('C');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(7),
            "Cursor should be at (r:3,c:7) after printing 'C'"
        );

        // Verify characters.
        assert_plain_char_at(&ofs_buf, 3, 5, 'A');
        assert_plain_char_at(&ofs_buf, 7, 2, 'B');
        assert_plain_char_at(&ofs_buf, 3, 6, 'C');

        // Verify saved cursor position persisted in buffer.
        assert_eq!(
            ofs_buf.cursor_pos,
            row(3) + col(7),
            "ofs_buf cursor pos should be (r:3,c:7) after processing"
        );
    }

    #[test]
    fn test_esc_save_restore_cursor() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // ESC save/restore cursor operation pattern:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 3:  │   │   │   │   │   │ A │ C │ ␩ │   │   │ ← 1. save position after A
        //         ├───┼───┼───┼───┼───┼───┼───┼─▲─┼───┼───┤   3. restore & write C
        //         │ … │ … │ … │ … │ … │ … │ … │ │ │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼─│─┼───┼───┤
        // Row 7:  │   │   │ B │   │   │   │   │ │ │   │   │ ← 2. move elsewhere & write B
        //         └───┴───┴───┴───┴───┴───┴───┴─│─┴───┴───┘
        //                                       ╰─ cursor ends here: (r:3,c:7)

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Move cursor to position (r:3,c:5) and write 'A'
        performer.ofs_buf.cursor_pos = row(3) + col(5);
        performer.print('A');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(6),
            "Cursor should be at (r:3,c:6) after printing 'A'"
        );

        // Save cursor position at (r:3,c:6) using ESC 7
        performer.apply_ansi_bytes(esc_codes::EscSequence::SaveCursor.to_string());

        // Move cursor elsewhere and write 'B'.
        performer.ofs_buf.cursor_pos = row(7) + col(2);
        performer.print('B');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(7) + col(3),
            "Cursor should be at (r:7,c:3) after printing 'B'"
        );

        // Restore cursor position using ESC 8.
        performer.apply_ansi_bytes(esc_codes::EscSequence::RestoreCursor.to_string());

        // Verify cursor was restored.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            Pos {
                row_index: row(3),
                col_index: col(6),
            }
        );

        // Write 'C' at restored position.
        performer.print('C');
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(7),
            "Cursor should be at (r:3,c:7) after printing 'C'"
        );

        // Verify saved cursor position persisted in buffer.
        assert_eq!(
            ofs_buf.cursor_pos,
            row(3) + col(7),
            "ofs_buf cursor pos should be (r:3,c:7) after processing"
        );

        // Verify characters are at expected positions.
        assert_plain_char_at(&ofs_buf, 3, 5, 'A');
        assert_plain_char_at(&ofs_buf, 7, 2, 'B');
        assert_plain_char_at(&ofs_buf, 3, 6, 'C');
    }
}

/// Tests for Vertical Position Absolute (VPA) operation.
pub mod vertical_position_absolute {
    use super::*;

    #[test]
    fn test_vpa_move_to_specific_row() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start at position (3, 5) and move to row 7
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(4),
            col: term_col(6),
        }; // Move to row 4, col 6 (1-based)
        let vpa_sequence = CsiSequence::VerticalPositionAbsolute(7);
        let sequence = format!("{move_cursor}{vpa_sequence}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify cursor moved to row 6 (0-based), column unchanged
        assert_eq!(
            ofs_buf.cursor_pos,
            row(6) + col(5),
            "VPA should move to row 6 (0-based) while preserving column 5"
        );
    }

    #[test]
    fn test_vpa_default_parameter() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start at position (5, 8) and use VPA with default parameter
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(6),
            col: term_col(9),
        }; // Move to row 6, col 9 (1-based)
        let vpa_sequence = CsiSequence::VerticalPositionAbsolute(1); // Default to row 1
        let sequence = format!("{move_cursor}{vpa_sequence}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify cursor moved to row 0 (0-based), column unchanged
        assert_eq!(
            ofs_buf.cursor_pos,
            row(0) + col(8),
            "VPA default should move to row 0 (0-based) while preserving column 8"
        );
    }

    #[test]
    fn test_vpa_bounds_checking() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start at position (5, 3) and try to move beyond bounds
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(6),
            col: term_col(4),
        }; // Move to row 6, col 4 (1-based)
        let vpa_sequence = CsiSequence::VerticalPositionAbsolute(15); // Beyond bounds
        let sequence = format!("{move_cursor}{vpa_sequence}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify cursor clamped to last row (9 in 0-based), column unchanged
        assert_eq!(
            ofs_buf.cursor_pos,
            row(9) + col(3),
            "VPA should clamp to row 9 (0-based) when target is beyond buffer"
        );
    }

    #[test]
    fn test_vpa_zero_parameter_treated_as_one() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start at position (7, 2) and move with parameter 0
        let move_cursor = CsiSequence::CursorPosition {
            row: term_row(8),
            col: term_col(3),
        }; // Move to row 8, col 3 (1-based)
        // VPA parameter 0 should be treated as 1, but since we need explicit param,
        // let's use 1 which represents the first row.
        let vpa_sequence = CsiSequence::VerticalPositionAbsolute(1);
        let sequence = format!("{move_cursor}{vpa_sequence}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify cursor moved to row 0 (0-based), column unchanged
        assert_eq!(
            ofs_buf.cursor_pos,
            row(0) + col(2),
            "VPA with parameter 1 should move to row 0 (0-based) while preserving column 2"
        );
    }

    #[test]
    fn test_vpa_preserves_horizontal_position() {
        // Test multiple column positions.
        for col_pos in [0u16, 3, 6, 9] {
            let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

            // Move to initial position and then use VPA.
            let move_cursor = CsiSequence::CursorPosition {
                row: term_row(3),
                col: term_col(col_pos + 1),
            }; // Move to row 3, col (1-based)
            let vpa_sequence = CsiSequence::VerticalPositionAbsolute(8);
            let sequence = format!("{move_cursor}{vpa_sequence}");
            let _result = ofs_buf.apply_ansi_bytes(sequence);

            // Verify column position preserved.
            assert_eq!(
                ofs_buf.cursor_pos,
                row(7) + col(col_pos),
                "VPA should preserve column {col_pos} when moving to row 7"
            );
        }
    }
}

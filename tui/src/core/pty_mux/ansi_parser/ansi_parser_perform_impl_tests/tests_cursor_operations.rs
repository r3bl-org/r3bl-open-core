// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for all cursor operations - positioning, movement, and save/restore.

use vte::Perform;

use super::create_test_offscreen_buffer_10r_by_10c;
use crate::{ansi_parser::{ansi_parser_perform_impl::cursor_ops,
                          ansi_parser_public_api::AnsiToBufferProcessor,
                          csi_codes::{CSI_PARAM_SEPARATOR, CSI_START,
                                      csi_seq_cursor_pos, csi_seq_cursor_pos_alt},
                          term_units::{term_col, term_row},
                          csi_codes::CsiSequence, esc_codes},
            Pos, col, row,
            offscreen_buffer::test_fixtures_offscreen_buffer::*};

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
        // Row 0:  │ H │   │   │   │   │   │   │   │   │   │ ← 2. ESC[H moves to home (1,1)
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   in 1-based, (0,0) in 0-based
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 5:  │   │   │   │   │   │ X │   │   │   │   │ ← 1. start at (5,5)
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //
        // Sequence: Write 'X' at (5,5) → ESC[H → Write 'H' at home (0,0)

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Start at a non-home position
            processor.cursor_pos = row(5) + col(5);
            processor.print('X');
            assert_eq!(
                processor.cursor_pos,
                row(5) + col(6),
                "Cursor should move right after printing X"
            ); // After writing 'X', cursor moves right
            assert_eq!(processor.cursor_pos, row(5) + col(6));

            // Send ESC[H to move to home position (1,1), 1-based index
            let sequence = csi_seq_cursor_pos(term_row(1) + term_col(1)).to_string();
            processor.process_bytes(sequence);

            // Verify cursor is at home position (0,0), in 0-based indexing)
            assert_eq!(processor.cursor_pos, row(0) + col(0));

            processor.print('H'); // Mark home position
        }

        // Verify characters are at correct positions
        assert_plain_char_at(&ofs_buf, 5, 5, 'X');
        assert_plain_char_at(&ofs_buf, 0, 0, 'H');

        // Verify final cursor position in buffer
        assert_eq!(ofs_buf.my_pos, row(0) + col(1)); // After writing 'H'
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
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 4:  │   │   │   │   │   │   │   │   │   │ A │ ← ESC[5;10H moves to
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   row 5, col 10 (1-based)
        // Row 5:  │ ⋯ │   │   │   │   │   │   │   │   │   │   which is(4,9) 0-based
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //           └─ cursor ends here (row 5, col 0)
        //
        // Sequence: ESC[5;10H → Write 'A' at (4,9)

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Send ESC[5;10H to move to row 5, column 10 (1-based)
            let sequence = csi_seq_cursor_pos(term_row(5) + term_col(10)).to_string();
            processor.process_bytes(sequence);

            // Verify cursor is at (4,9) in 0-based indexing
            assert_eq!(
                processor.cursor_pos,
                row(4) + col(9),
                "Cursor should be at (4,9) in 0-based indexing"
            );

            processor.print('A');

            // Verify cursor moved to next position (5,0) after writing 'A'
            assert_eq!(
                processor.cursor_pos,
                row(5) + col(0),
                "Cursor should move to next line start after writing A"
            );
        }

        // Verify character was written at correct position
        assert_plain_char_at(&ofs_buf, 4, 9, 'A');
        assert_eq!(
            ofs_buf.my_pos,
            row(5) + col(0),
            "Final cursor position should be at (5,0)"
        );
    }

    #[test]
    fn test_cursor_clamps_to_buffer_boundaries_when_out_of_range() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Try to move cursor beyond buffer bounds (row 15, col 15) - should clamp to (9,9)
            let sequence = csi_seq_cursor_pos(term_row(15) + term_col(15)).to_string();
            processor.process_bytes(sequence);

            // Verify cursor is clamped to buffer boundaries (9,9) in 0-based indexing
            assert_eq!(
                processor.cursor_pos,
                row(9) + col(9),
                "Cursor should be clamped to buffer boundaries at (9,9)"
            );

            processor.print('B'); // Mark the clamped position

            // Try to move to negative/zero positions (should become (0,0))
            let sequence = csi_seq_cursor_pos(term_row(0) + term_col(0)).to_string();
            processor.process_bytes(sequence);

            assert_eq!(
                processor.cursor_pos,
                row(0) + col(0),
                "Cursor should default to (0,0) for out-of-range coordinates"
            );

            processor.print('C'); // Mark the origin
        }

        // Verify characters are at expected positions
        assert_plain_char_at(&ofs_buf, 9, 9, 'B'); // At clamped boundary
        assert_plain_char_at(&ofs_buf, 0, 0, 'C'); // At origin
    }

    #[test]
    fn test_cursor_alternate_positioning_syntax_works_identically() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Test CUP (Cursor Position) - ESC[3;4H
            let cup_sequence = csi_seq_cursor_pos(term_row(3) + term_col(4)).to_string();
            processor.process_bytes(cup_sequence);
            processor.print('D');

            // Test HVP (Horizontal Vertical Position) - ESC[6;7f
            let hvp_sequence = csi_seq_cursor_pos_alt(term_row(6) + term_col(7)).to_string();
            processor.process_bytes(hvp_sequence);
            processor.print('E');
        }

        // Verify both positioning commands work identically
        assert_plain_char_at(&ofs_buf, 2, 3, 'D'); // CUP result (3,4) -> (2,3)
        assert_plain_char_at(&ofs_buf, 5, 6, 'E'); // HVP result (6,7) -> (5,6)
    }

    #[test]
    fn test_cursor_defaults_to_1_when_row_or_column_missing() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Move to a known position first
            processor.cursor_pos = row(5) + col(5);
            processor.print('S'); // Mark start position

            // Test ESC[H (no parameters) - should move to (1,1) which is (0,0) in 0-based
            processor.process_bytes(format!("{CSI_START}H"));
            processor.print('H'); // Mark home

            // Test ESC[3H (row only) - should move to (3,1) which is (2,0) in 0-based
            processor.process_bytes(format!("{CSI_START}3H"));
            processor.print('R'); // Mark row-only

            // Test ESC[;5H (column only) - should move to (1,5) which is (0,4) in 0-based
            processor.process_bytes(format!("{CSI_START}{CSI_PARAM_SEPARATOR}5H"));
            processor.print('C'); // Mark column-only
        }

        // Verify default behavior
        assert_plain_char_at(&ofs_buf, 5, 5, 'S'); // Start position
        assert_plain_char_at(&ofs_buf, 0, 0, 'H'); // ESC[H -> (0,0)
        assert_plain_char_at(&ofs_buf, 2, 0, 'R'); // ESC[3H -> (2,0)
        assert_plain_char_at(&ofs_buf, 0, 4, 'C'); // ESC[;5H -> (0,4)
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
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

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
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // 1. Start at row 2, write a character
            processor.cursor_pos = row(2) + col(4);
            processor.print('X');

            // 2. Move down 3 rows and write another character
            cursor_ops::cursor_down(&mut processor, 3);
            assert_eq!(processor.cursor_pos.row_index, row(5));
            assert_eq!(processor.cursor_pos.col_index, col(5)); // Column should be after 'X'

            processor.cursor_pos.col_index = col(3); // Reset column to different position
            processor.print('Y');

            // 3. Try to move down beyond boundary
            cursor_ops::cursor_down(&mut processor, 10);
            assert_eq!(processor.cursor_pos.row_index, row(9)); // Should stop at row 9
            processor.cursor_pos.col_index = col(3);
            processor.print('Z');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 2, 4, 'X');
        assert_plain_char_at(&ofs_buf, 5, 3, 'Y');
        assert_plain_char_at(&ofs_buf, 9, 3, 'Z');
    }

    #[test]
    fn test_cursor_movement_forward() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Test cursor forward movement with buffer verification
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // 1. Start at column 3, write a character
            processor.cursor_pos = row(4) + col(3);
            processor.print('L');

            // 2. Move forward 2 columns and write another character
            cursor_ops::cursor_forward(&mut processor, 2);
            assert_eq!(processor.cursor_pos.col_index, col(6)); // Should be at column 6
            processor.print('M');

            // 3. Try to move forward beyond boundary
            cursor_ops::cursor_forward(&mut processor, 10);
            assert_eq!(processor.cursor_pos.col_index, col(9)); // Should stop at column 9
            processor.print('N');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 4, 3, 'L');
        assert_plain_char_at(&ofs_buf, 4, 6, 'M');
        assert_plain_char_at(&ofs_buf, 4, 9, 'N');
    }

    #[test]
    fn test_cursor_movement_backward() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Test cursor backward movement with buffer verification
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // 1. Start at column 7, write a character
            processor.cursor_pos = row(6) + col(7);
            processor.print('P');

            // 2. Move backward 3 columns and write another character
            cursor_ops::cursor_backward(&mut processor, 3);
            assert_eq!(processor.cursor_pos.col_index, col(5)); // Should be at column 5
            processor.print('Q');

            // 3. Try to move backward beyond boundary
            cursor_ops::cursor_backward(&mut processor, 10);
            assert_eq!(processor.cursor_pos.col_index, col(0)); // Should stop at column 0
            processor.print('R');
        }

        // Verify characters are in correct positions
        assert_plain_char_at(&ofs_buf, 6, 7, 'P');
        assert_plain_char_at(&ofs_buf, 6, 5, 'Q');
        assert_plain_char_at(&ofs_buf, 6, 0, 'R');
    }
}

/// Tests for cursor save and restore operations (both CSI and ESC variants).
pub mod save_restore {
    use super::*;

    #[test]
    fn test_csi_save_restore_cursor() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Move to position and save with CSI s
            processor.cursor_pos = row(3) + col(5);
            processor.print('A');

            // CSI s - Save cursor position
            let sequence = CsiSequence::SaveCursor.to_string();
            processor.process_bytes(sequence);

            // Move elsewhere
            processor.cursor_pos = row(7) + col(2);
            processor.print('B');

            // CSI u - Restore cursor position
            let sequence = CsiSequence::RestoreCursor.to_string();
            processor.process_bytes(sequence);

            // Should be back at saved position
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 6); // After 'A'

            processor.print('C');
        }

        // Verify characters
        assert_plain_char_at(&ofs_buf, 3, 5, 'A');
        assert_plain_char_at(&ofs_buf, 7, 2, 'B');
        assert_plain_char_at(&ofs_buf, 3, 6, 'C'); // At restored position
    }

    #[test]
    fn test_esc_save_restore_cursor() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Move cursor to position (3, 5) and write 'A'
            processor.cursor_pos = row(3) + col(5);
            processor.print('A');

            // Save cursor position at (3, 6) using ESC 7
            let seq = esc_codes::EscSequence::SaveCursor.to_string();
            processor.process_bytes(&seq);

            // Move cursor elsewhere and write 'B'
            processor.cursor_pos = row(7) + col(2);
            processor.print('B');

            // Restore cursor position using ESC 8
            let seq = esc_codes::EscSequence::RestoreCursor.to_string();
            processor.process_bytes(&seq);

            // Verify cursor was restored
            assert_eq!(
                processor.cursor_pos,
                Pos {
                    row_index: row(3),
                    col_index: col(6),
                }
            );

            // Write 'C' at restored position
            processor.print('C');
        }

        // Verify saved cursor position persisted in buffer
        assert_eq!(
            ofs_buf.ansi_parser_support.cursor_pos_for_esc_save_and_restore,
            Some(row(3) + col(6))
        );

        // Verify characters are at expected positions
        assert_plain_char_at(&ofs_buf, 3, 5, 'A');
        assert_plain_char_at(&ofs_buf, 7, 2, 'B');
        assert_plain_char_at(&ofs_buf, 3, 6, 'C'); // Should be right after 'A'
    }
}
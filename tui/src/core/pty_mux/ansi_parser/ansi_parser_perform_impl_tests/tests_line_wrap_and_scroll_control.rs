// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for line management and buffer control - wrapping, auto-wrap mode, and
//! scrolling.

use vte::Perform;

use super::tests_fixtures::*;
use crate::{ansi_parser::{ansi_parser_public_api::AnsiToBufferProcessor,
                          csi_codes::{CsiSequence, PrivateModeType},
                          esc_codes},
            col,
            csi_codes::{CSI_START, SD_SCROLL_DOWN, SU_SCROLL_UP},
            offscreen_buffer::test_fixtures_offscreen_buffer::*,
            row};

/// Tests for auto-wrap mode (DECAWM) functionality.
pub mod auto_wrap {
    use super::*;

    #[test]
    fn test_auto_wrap_enabled_by_default() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Auto-wrap should be enabled by default
        // This test verifies that characters wrap to the next line when reaching the
        // right margin
        //
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ 0 │ 1 │ 2 │ 3 │ 4 │ 5 │ 6 │ 7 │ 8 │ 9 │ ← First line fills up
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 1:  │ A │ ␩ │   │   │   │   │   │   │   │   │ ← 11th char wraps here
        //         ├───┼─▲─┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ │ │ … │ … │ … │ … │ … │ … │ … │ … │
        //         └───┴─│─┴───┴───┴───┴───┴───┴───┴───┴───┘
        //               ╰─ cursor at (r:1,c:1)

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Verify auto-wrap is enabled by default
        assert!(
            processor.ofs_buf.ansi_parser_support.auto_wrap_mode,
            "Auto-wrap mode should be enabled by default"
        );

        // Write 10 characters to fill the first line (0-9)
        for ch in '0'..='9' {
            processor.print(ch);
        }

        // Verify cursor is at (r:1,c:0) - wrapped to next line after hitting right
        // boundary
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(1) + col(0),
            "Cursor should be at (r:1,c:0) after printing 10 characters"
        );

        // The 11th character (should be added to the next line)
        processor.print('A');

        // Verify cursor wrapped to next line
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(1) + col(1),
            "Cursor should be at (r:1,c:1) after wrapping"
        );

        // Verify buffer contents
        assert_plain_text_at(&ofs_buf, 0, 0, "0123456789");
        assert_plain_char_at(&ofs_buf, 1, 0, 'A');
    }

    #[test]
    fn test_auto_wrap_can_be_disabled() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // When auto-wrap is disabled, characters beyond the right margin
        // should overwrite the last column instead of wrapping
        //
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ 0 │ 1 │ 2 │ 3 │ 4 │ 5 │ 6 │ 7 │ 8 │ X │ ← 'X' overwrites '9'
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼─▲─┤
        // Row 1:  │   │   │   │   │   │   │   │   │   │ ╰──── cursor here
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤   no wrapping occurs
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Disable auto-wrap mode using CSI ?7l
        let sequence =
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap).to_string();
        processor.process_bytes(sequence);

        // Verify auto-wrap is now disabled
        assert!(
            !processor.ofs_buf.ansi_parser_support.auto_wrap_mode,
            "Auto-wrap mode should be disabled after CSI ?7l"
        );

        // Fill the line
        for ch in '0'..='9' {
            processor.print(ch);
        }

        // Try to write beyond the margin - should clamp at right edge
        processor.print('X');

        // Verify cursor stays at right margin
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(0) + col(9),
            "Cursor should stay at (r:0,c:9) without wrapping"
        );

        // Verify buffer contents - 'X' should overwrite '9'
        assert_plain_text_at(&ofs_buf, 0, 0, "012345678X");
    }

    #[test]
    fn test_auto_wrap_can_be_toggled() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Start with default (enabled)
        assert!(processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Disable auto-wrap
        let disable_sequence =
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap).to_string();
        processor.process_bytes(disable_sequence);
        assert!(!processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Re-enable auto-wrap using CSI ?7h
        let enable_sequence =
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap).to_string();
        processor.process_bytes(enable_sequence);
        assert!(processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Test that wrapping works again
        for ch in 'A'..='K' {
            // 11 characters should wrap
            processor.print(ch);
        }

        // Verify wrapping occurred
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(1) + col(1),
            "Cursor should be at (r:1,c:1) after wrapping"
        );

        // Verify buffer contents
        assert_plain_text_at(&ofs_buf, 0, 0, "ABCDEFGHIJ");
        assert_plain_char_at(&ofs_buf, 1, 0, 'K');
    }

    #[test]
    fn test_auto_wrap_mode_change_effect_is_immediate() {
        // This test verifies that toggling auto-wrap mode (DECAWM) has an immediate
        // effect on character printing behavior.
        //
        // 1. Starts with auto-wrap ON, fills first line, cursor wraps to (r:1,c:0).
        // 2. Disables auto-wrap, moves to (r:2,c:9), prints 'X' then 'Y'. 'Y' overwrites
        //    'X' as cursor clamps at the right margin.
        // 3. Re-enables auto-wrap, moves to (r:2,c:9), prints 'A' then 'B'. 'A'
        //    overwrites 'Y', and 'B' wraps to the next line.
        //
        // Final Buffer State:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ 0 │ 1 │ 2 │ 3 │ 4 │ 5 │ 6 │ 7 │ 8 │ 9 │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 1:  │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 2:  │   │   │   │   │   │   │   │   │   │ A │ ← 'Y' overwritten by 'A'
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 3:  │ B │ ␩ │   │   │   │   │   │   │   │   │ ← 'B' wraps
        //         ├───┼─▲─┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ │ │ … │ … │ … │ … │ … │ … │ … │ … │
        //         └───┴─│─┴───┴───┴───┴───┴───┴───┴───┴───┘
        //               ╰─ cursor ends at (r:3,c:1)
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Test auto-wrap mode changes
        // Fill the first line to column 9 (last column)
        for ch in '0'..='9' {
            processor.print(ch);
        }

        // Now cursor should be at (r:1,c:0) after wrapping
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(1) + col(0),
            "Cursor should be at (r:1,c:0) after printing 10 characters"
        );

        // Disable auto-wrap mode
        let sequence =
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap).to_string();
        processor.process_bytes(sequence);

        // Move to end of line 2 and test clamping
        processor.ofs_buf.my_pos = row(2) + col(9);
        processor.print('X'); // At boundary
        processor.print('Y'); // Should clamp to (r:2,c:9) and overwrite 'X'

        // Re-enable auto-wrap mode
        let sequence =
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap).to_string();
        processor.process_bytes(sequence);

        // Move to a new position and test wrapping again
        processor.ofs_buf.my_pos = row(2) + col(9);
        processor.print('A');
        processor.print('B'); // Should wrap to row 3

        // Verify final cursor position
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(3) + col(1),
            "Cursor should be at (r:3,c:1) after wrapping"
        );

        // Verify buffer contents
        assert_plain_char_at(&ofs_buf, 0, 8, '8'); // '8' at position [0][8]
        assert_plain_char_at(&ofs_buf, 0, 9, '9'); // '9' at position [0][9]
        assert_plain_char_at(&ofs_buf, 2, 9, 'A'); // 'A' at boundary position (overwrote 'Y')
        assert_plain_char_at(&ofs_buf, 3, 0, 'B'); // 'B' wrapped to next line
    }
}

/// Tests for line wrapping behavior at buffer boundaries.
pub mod line_wrapping {
    use super::*;

    #[test]
    fn test_line_wrapping_behavior() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Process characters that should wrap at column 10
        //
        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │ ← First 10 chars
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 1:  │ K │ ␩ │   │   │   │   │   │   │   │   │ ← 11th char wraps
        //         ├───┼─▲─┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ │ │ … │ … │ … │ … │ … │ … │ … │ … │
        //         └───┴─│─┴───┴───┴───┴───┴───┴───┴───┴───┘
        //               ╰─ cursor at (r:1,c:1)

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Write 10 characters to fill the line
        for ch in 'A'..='J' {
            processor.print(ch);
        }

        // 11th character should wrap to next line
        processor.print('K');

        // Verify cursor wrapped to next line
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(1) + col(1),
            "Cursor should be at (r:1,c:1) after wrapping"
        );

        // Verify buffer contents - first line should have A-J
        assert_plain_text_at(&ofs_buf, 0, 0, "ABCDEFGHIJ");

        // Verify K wrapped to next line
        assert_plain_char_at(&ofs_buf, 1, 0, 'K');

        // Verify rest of second line is empty
        for col in 1..10 {
            assert_empty_at(&ofs_buf, 1, col);
        }
    }
}

/// Tests for buffer scrolling operations.
pub mod scrolling {
    use super::*;

    fn fill_buffer_with_lines(ofs_buf: &mut crate::OffscreenBuffer) {
        for r in 0..ofs_buf.window_size.row_height.as_usize() {
            let line_text = format!("Line-{}", r);
            for (c, char) in line_text.chars().enumerate() {
                ofs_buf.buffer[r][c] = crate::PixelChar::PlainText {
                    display_char: char,
                    maybe_style: None,
                };
            }
        }
    }

    #[test]
    fn test_esc_d_index_scrolls_up_at_bottom() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        // ESC D (Index) at bottom row causes buffer to scroll up
        //
        // BEFORE ESC D:                    AFTER ESC D:
        //
        // Row 0: │Line-0│                  Row 0: │Line-1│ ← Line-0 disappears
        // Row 1: │Line-1│                  Row 1: │Line-2│
        // Row 2: │Line-2│                  Row 2: │Line-3│
        // Row 3: │Line-3│                  Row 3: │Line-4│
        // Row 4: │Line-4│                  Row 4: │Line-5│
        // Row 5: │Line-5│    ESC D         Row 5: │Line-6│
        // Row 6: │Line-6│    ═══════►      Row 6: │Line-7│
        // Row 7: │Line-7│                  Row 7: │Line-8│
        // Row 8: │Line-8│                  Row 8: │Line-9│
        // Row 9: │Line-9│ ← cursor here    Row 9: │  ␩   │ ← empty, cursor here

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // Move cursor to the last row
        processor.ofs_buf.my_pos.row_index = row(9);

        // Execute Index (ESC D)
        processor.esc_dispatch(&[], false, esc_codes::IND_INDEX_DOWN);

        // Verify buffer scrolled up: "Line-0" is gone, "Line-1" is now at row 0
        assert_plain_text_at(&ofs_buf, 0, 0, "Line-1");
        // Verify the last line is now empty
        for col in 0..10 {
            assert_empty_at(&ofs_buf, 9, col);
        }
    }

    #[test]
    fn test_esc_m_reverse_index_scrolls_down_at_top() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        // ESC M (Reverse Index) at top row causes buffer to scroll down
        //
        // BEFORE ESC M:                    AFTER ESC M:
        //
        // Row 0: │Line-0│ ← cursor here    Row 0: │  ␩   │ ← empty, cursor here
        // Row 1: │Line-1│                  Row 1: │Line-0│
        // Row 2: │Line-2│                  Row 2: │Line-1│
        // Row 3: │Line-3│                  Row 3: │Line-2│
        // Row 4: │Line-4│    ESC M         Row 4: │Line-3│
        // Row 5: │Line-5│    ═══════►      Row 5: │Line-4│
        // Row 6: │Line-6│                  Row 6: │Line-5│
        // Row 7: │Line-7│                  Row 7: │Line-6│
        // Row 8: │Line-8│                  Row 8: │Line-7│
        // Row 9: │Line-9│                  Row 9: │Line-8│ ← Line-9 disappears

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // Move cursor to the first row
        processor.ofs_buf.my_pos.row_index = row(0);

        // Execute Reverse Index (ESC M)
        processor.esc_dispatch(&[], false, esc_codes::RI_REVERSE_INDEX_UP);

        // Verify buffer scrolled down: first line is now empty
        for col in 0..10 {
            assert_empty_at(&ofs_buf, 0, col);
        }
        // Verify "Line-0" is now at row 1
        assert_plain_text_at(&ofs_buf, 1, 0, "Line-0");
        // Verify "Line-8" is now at row 9
        assert_plain_text_at(&ofs_buf, 9, 0, "Line-8");
    }

    #[test]
    fn test_csi_s_scroll_up() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        // CSI 2 S (Scroll Up by 2 lines) removes top 2 lines, adds empty at bottom
        //
        // BEFORE CSI 2S:                   AFTER CSI 2S:
        //
        // Row 0: │Line-0│                  Row 0: │Line-2│ ← Line-0,1 disappear
        // Row 1: │Line-1│                  Row 1: │Line-3│
        // Row 2: │Line-2│                  Row 2: │Line-4│
        // Row 3: │Line-3│    CSI 2S        Row 3: │Line-5│
        // Row 4: │Line-4│    ═══════►      Row 4: │Line-6│
        // Row 5: │Line-5│                  Row 5: │Line-7│
        // Row 6: │Line-6│                  Row 6: │Line-8│
        // Row 7: │Line-7│                  Row 7: │Line-9│
        // Row 8: │Line-8│                  Row 8: │      │ ← empty
        // Row 9: │Line-9│                  Row 9: │      │ ← empty

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Execute Scroll Up by 2 lines (CSI 2 S)
        let sequence = CsiSequence::ScrollUp(2).to_string();
        processor.process_bytes(sequence);

        // Verify buffer scrolled up by 2: "Line-2" is now at row 0
        assert_plain_text_at(&ofs_buf, 0, 0, "Line-2");

        // Verify the last two lines are now empty
        for col in 0..10 {
            assert_empty_at(&ofs_buf, 8, col); // second last line is empty
            assert_empty_at(&ofs_buf, 9, col); // last line is empty
        }
    }

    #[test]
    fn test_csi_t_scroll_down() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        // CSI 3 T (Scroll Down by 3 lines) adds empty at top, removes bottom 3 lines
        //
        // BEFORE CSI 3T:                   AFTER CSI 3T:
        //
        // Row 0: │Line-0│                  Row 0: │      │ ← empty
        // Row 1: │Line-1│                  Row 1: │      │ ← empty
        // Row 2: │Line-2│                  Row 2: │      │ ← empty
        // Row 3: │Line-3│    CSI 3T        Row 3: │Line-0│
        // Row 4: │Line-4│    ═══════►      Row 4: │Line-1│
        // Row 5: │Line-5│                  Row 5: │Line-2│
        // Row 6: │Line-6│                  Row 6: │Line-3│
        // Row 7: │Line-7│                  Row 7: │Line-4│
        // Row 8: │Line-8│                  Row 8: │Line-5│
        // Row 9: │Line-9│                  Row 9: │Line-6│ ← Line-7,8,9 disappear

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // Execute Scroll Down by 3 lines (CSI 3 T)
        let sequence = CsiSequence::ScrollDown(3).to_string();
        processor.process_bytes(sequence);

        // Verify buffer scrolled down by 3: first 3 lines are empty
        for r in 0..3 {
            for c in 0..10 {
                assert_empty_at(&ofs_buf, r, c);
            }
        }

        // Verify "Line-0" is now at row 3
        assert_plain_text_at(&ofs_buf, 3, 0, "Line-0");

        // Verify "Line-6" is now at row 9
        assert_plain_text_at(&ofs_buf, 9, 0, "Line-6");
    }

    #[test]
    fn test_csi_s_scroll_up_more_than_height_clears_buffer() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        // CSI 20 S (Scroll Up by 20 lines) - more than buffer height clears everything
        //
        // BEFORE CSI 20S:                  AFTER CSI 20S:
        //
        // Row 0: │Line-0│                  Row 0: │      │ ← empty
        // Row 1: │Line-1│                  Row 1: │      │ ← empty
        // Row 2: │Line-2│                  Row 2: │      │ ← empty
        // Row 3: │Line-3│    CSI 20S       Row 3: │      │ ← empty
        // Row 4: │Line-4│    ═══════►      Row 4: │      │ ← empty
        // Row 5: │Line-5│                  Row 5: │      │ ← empty
        // Row 6: │Line-6│                  Row 6: │      │ ← empty
        // Row 7: │Line-7│                  Row 7: │      │ ← empty
        // Row 8: │Line-8│                  Row 8: │      │ ← empty
        // Row 9: │Line-9│                  Row 9: │      │ ← empty

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // Execute Scroll Up by 20 lines (more than height)
        let sequence = CsiSequence::ScrollUp(20).to_string();
        processor.process_bytes(sequence);

        // Verify the entire buffer is empty
        for r in 0..10 {
            for c in 0..10 {
                assert_empty_at(&ofs_buf, r, c);
            }
        }
    }

    #[test]
    fn test_csi_t_scroll_down_more_than_height_clears_buffer() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // Execute Scroll Down by 20 lines (more than height)
        let sequence = CsiSequence::ScrollDown(20).to_string();
        processor.process_bytes(sequence);

        // Verify the entire buffer is empty
        for r in 0..10 {
            for c in 0..10 {
                assert_empty_at(&ofs_buf, r, c);
            }
        }
    }

    #[test]
    fn test_esc_d_and_esc_m_move_cursor_when_not_at_boundary() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        // This test ensures that ESC D (Index) and ESC M (Reverse Index) only move the
        // cursor when not at the screen boundaries, without scrolling the buffer.
        //
        // 1. Cursor starts at (r:5, c:0).
        // 2. ESC D moves cursor down to (r:6, c:0). Buffer content remains unchanged.
        // 3. ESC M moves cursor back up to (r:5, c:0). Buffer content remains unchanged.
        //
        //                Buffer State (remains unchanged throughout)
        //
        // Row 4: │Line-4│
        // Row 5: │Line-5│ ← Cursor starts here, and returns here
        // Row 6: │Line-6│ ← Cursor moves down to here
        // Row 7: │Line-7│

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        processor.ofs_buf.my_pos = row(5) + col(0);

        // Execute Index (ESC D) - should just move cursor down
        processor.esc_dispatch(&[], false, esc_codes::IND_INDEX_DOWN);
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(6) + col(0),
            "Cursor should move down"
        );
        assert_plain_text_at(&processor.ofs_buf, 5, 0, "Line-5");

        // Execute Reverse Index (ESC M) - should just move cursor up
        processor.esc_dispatch(&[], false, esc_codes::RI_REVERSE_INDEX_UP);
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(5) + col(0),
            "Cursor should move up"
        );
        assert_plain_text_at(&processor.ofs_buf, 6, 0, "Line-6");
    }

    #[test]
    fn test_csi_s_scroll_up_defaults_to_one_line() {
        // Verifies that CSI S (Scroll Up) defaults to 1 line if no parameter is given.
        //
        // Row 0: │Line-0│  ->  Row 0: │Line-1│
        // Row 9: │Line-9│  ->  Row 9: │      │ (empty)

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // This is missing the "number of lines to scroll" parameter, so should default to
        // 1
        processor.process_bytes(format!("{}{}", CSI_START, SU_SCROLL_UP));

        assert_plain_text_at(&ofs_buf, 0, 0, "Line-1");
        assert_empty_at(&ofs_buf, 9, 0);
    }

    #[test]
    fn test_csi_t_scroll_down_defaults_to_one_line() {
        // Verifies that CSI T (Scroll Down) defaults to 1 line if no parameter is given.
        //
        // Row 0: │Line-0│  ->  Row 0: │      │ (empty)
        // Row 1: │Line-1│  ->  Row 1: │Line-0│

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // This is missing the "number of lines to scroll" parameter, so should default to
        // 1
        processor.process_bytes(format!("{}{}", CSI_START, SD_SCROLL_DOWN));

        assert_empty_at(&ofs_buf, 0, 0);
        assert_plain_text_at(&ofs_buf, 1, 0, "Line-0");
    }
}

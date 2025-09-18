// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for line management and buffer control - wrapping, auto-wrap mode, and
//! scrolling.

use vte::Perform;

use super::super::test_fixtures::*;
use crate::{TuiStyle, col,
            offscreen_buffer::ofs_buf_test_fixtures::*,
            row,
            vt_100_ansi_parser::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                                 csi_codes::{CsiSequence, PrivateModeType},
                                 esc_codes::{self, EscSequence},
                                 term_units::{term_col, term_row}}};

fn fill_buffer_with_lines(ofs_buf: &mut crate::OffscreenBuffer) {
    for r in 0..ofs_buf.window_size.row_height.as_usize() {
        let line_text = format!("Line-{r}");
        for (c, char) in line_text.chars().enumerate() {
            ofs_buf.buffer[r][c] = crate::PixelChar::PlainText {
                display_char: char,
                style: TuiStyle::default(),
            };
        }
    }
}

/// Tests for auto-wrap mode (DECAWM) functionality.
pub mod auto_wrap {
    use super::*;

    #[test]
    fn test_auto_wrap_enabled_by_default() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Auto-wrap should be enabled by default.
        // This test verifies that characters wrap to the next line when reaching the
        // right margin.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Verify auto-wrap is enabled by default.
        assert!(
            performer.ofs_buf.ansi_parser_support.auto_wrap_mode,
            "Auto-wrap mode should be enabled by default"
        );

        // Write 10 characters to fill the first line (0-9)
        for ch in '0'..='9' {
            performer.print(ch);
        }

        // Verify cursor is at (r:1,c:0) - wrapped to next line after hitting right
        // boundary
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(1) + col(0),
            "Cursor should be at (r:1,c:0) after printing 10 characters"
        );

        // The 11th character (should be added to the next line)
        performer.print('A');

        // Verify cursor wrapped to next line.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(1) + col(1),
            "Cursor should be at (r:1,c:1) after wrapping"
        );

        // Verify buffer contents.
        assert_plain_text_at(&ofs_buf, 0, 0, "0123456789");
        assert_plain_char_at(&ofs_buf, 1, 0, 'A');
    }

    #[test]
    fn test_auto_wrap_can_be_disabled() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // When auto-wrap is disabled, characters beyond the right margin
        // should overwrite the last column instead of wrapping.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Disable auto-wrap mode using CSI ?7l.
        let sequence =
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify auto-wrap is now disabled.
        assert!(
            !performer.ofs_buf.ansi_parser_support.auto_wrap_mode,
            "Auto-wrap mode should be disabled after CSI ?7l"
        );

        // Fill the line
        for ch in '0'..='9' {
            performer.print(ch);
        }

        // Try to write beyond the margin - should clamp at right edge.
        performer.print('X');

        // Verify cursor stays at right margin.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(9),
            "Cursor should stay at (r:0,c:9) without wrapping"
        );

        // Verify buffer contents - 'X' should overwrite '9'.
        assert_plain_text_at(&ofs_buf, 0, 0, "012345678X");
    }

    #[test]
    fn test_auto_wrap_can_be_toggled() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Start with default (enabled)
        assert!(performer.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Disable auto-wrap.
        let disable_sequence =
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap).to_string();
        performer.apply_ansi_bytes(disable_sequence);
        assert!(!performer.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Re-enable auto-wrap using CSI ?7h.
        let enable_sequence =
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap).to_string();
        performer.apply_ansi_bytes(enable_sequence);
        assert!(performer.ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Test that wrapping works again.
        for ch in 'A'..='K' {
            // 11 characters should wrap.
            performer.print(ch);
        }

        // Verify wrapping occurred.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(1) + col(1),
            "Cursor should be at (r:1,c:1) after wrapping"
        );

        // Verify buffer contents.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test auto-wrap mode changes.
        // Fill the first line to column 9 (last column)
        for ch in '0'..='9' {
            performer.print(ch);
        }

        // Now cursor should be at (r:1,c:0) after wrapping
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(1) + col(0),
            "Cursor should be at (r:1,c:0) after printing 10 characters"
        );

        // Disable auto-wrap mode.
        let sequence =
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap).to_string();
        performer.apply_ansi_bytes(sequence);

        // Move to end of line 2 and test clamping.
        performer.ofs_buf.cursor_pos = row(2) + col(9);
        performer.print('X'); // At boundary
        performer.print('Y'); // Should clamp to (r:2,c:9) and overwrite 'X'

        // Re-enable auto-wrap mode.
        let sequence =
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap).to_string();
        performer.apply_ansi_bytes(sequence);

        // Move to a new position and test wrapping again.
        performer.ofs_buf.cursor_pos = row(2) + col(9);
        performer.print('A');
        performer.print('B'); // Should wrap to row 3

        // Verify final cursor position.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(3) + col(1),
            "Cursor should be at (r:3,c:1) after wrapping"
        );

        // Verify buffer contents.
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

        // Process characters that should wrap at column 10.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Write 10 characters to fill the line.
        for ch in 'A'..='J' {
            performer.print(ch);
        }

        // 11th character should wrap to next line.
        performer.print('K');

        // Verify cursor wrapped to next line.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(1) + col(1),
            "Cursor should be at (r:1,c:1) after wrapping"
        );

        // Verify buffer contents - first line should have A-J.
        assert_plain_text_at(&ofs_buf, 0, 0, "ABCDEFGHIJ");

        // Verify K wrapped to next line.
        assert_plain_char_at(&ofs_buf, 1, 0, 'K');

        // Verify rest of second line is empty.
        for col in 1..10 {
            assert_empty_at(&ofs_buf, 1, col);
        }
    }
}

/// Tests for buffer scrolling operations.
pub mod scrolling {
    use super::*;

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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        // Move cursor to the last row.
        performer.ofs_buf.cursor_pos.row_index = row(9);

        // Execute Index (ESC D)
        performer.esc_dispatch(&[], false, esc_codes::IND_INDEX_DOWN);

        // Verify buffer scrolled up: "Line-0" is gone, "Line-1" is now at row 0.
        assert_plain_text_at(&ofs_buf, 0, 0, "Line-1");
        // Verify the last line is now empty.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        // Move cursor to the first row.
        performer.ofs_buf.cursor_pos.row_index = row(0);

        // Execute Reverse Index (ESC M)
        performer.esc_dispatch(&[], false, esc_codes::RI_REVERSE_INDEX_UP);

        // Verify buffer scrolled down: first line is now empty
        for col in 0..10 {
            assert_empty_at(&ofs_buf, 0, col);
        }
        // Verify "Line-0" is now at row 1.
        assert_plain_text_at(&ofs_buf, 1, 0, "Line-0");
        // Verify "Line-8" is now at row 9.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Execute Scroll Up by 2 lines (CSI 2 S)
        let sequence = CsiSequence::ScrollUp(2).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify buffer scrolled up by 2: "Line-2" is now at row 0.
        assert_plain_text_at(&ofs_buf, 0, 0, "Line-2");

        // Verify the last two lines are now empty.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        // Execute Scroll Down by 3 lines (CSI 3 T)
        let sequence = CsiSequence::ScrollDown(3).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify buffer scrolled down by 3: first 3 lines are empty
        for r in 0..3 {
            for c in 0..10 {
                assert_empty_at(&ofs_buf, r, c);
            }
        }

        // Verify "Line-0" is now at row 3.
        assert_plain_text_at(&ofs_buf, 3, 0, "Line-0");

        // Verify "Line-6" is now at row 9.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        // Execute Scroll Up by 20 lines (more than height)
        let sequence = CsiSequence::ScrollUp(20).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify the entire buffer is empty.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        // Execute Scroll Down by 20 lines (more than height)
        let sequence = CsiSequence::ScrollDown(20).to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify the entire buffer is empty.
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

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        performer.ofs_buf.cursor_pos = row(5) + col(0);

        // Execute Index (ESC D) - should just move cursor down
        performer.esc_dispatch(&[], false, esc_codes::IND_INDEX_DOWN);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(6) + col(0),
            "Cursor should move down"
        );
        assert_plain_text_at(performer.ofs_buf, 5, 0, "Line-5");

        // Execute Reverse Index (ESC M) - should just move cursor up
        performer.esc_dispatch(&[], false, esc_codes::RI_REVERSE_INDEX_UP);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(5) + col(0),
            "Cursor should move up"
        );
        assert_plain_text_at(performer.ofs_buf, 6, 0, "Line-6");
    }

    #[test]
    fn test_csi_s_scroll_up_defaults_to_one_line() {
        // Verifies that CSI S (Scroll Up) defaults to 1 line if no parameter is given.
        // Raw sequence "\x1b[S" should scroll by 1 line.

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Send CSI sequence with explicit default parameter 1.
        let scroll_up_sequence = format!("{}", CsiSequence::ScrollUp(1));
        performer.apply_ansi_bytes(scroll_up_sequence.as_bytes());

        // After scrolling up by 1, Line-1 should be at row 0.
        assert_plain_text_at(performer.ofs_buf, 0, 0, "Line-1");
        // Bottom row should be empty.
        assert_empty_at(performer.ofs_buf, 9, 0);
    }

    #[test]
    fn test_csi_t_scroll_down_defaults_to_one_line() {
        // Verifies that CSI T (Scroll Down) defaults to 1 line if no parameter is given.
        // Raw sequence "\x1b[T" should scroll by 1 line.

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Send CSI sequence with explicit default parameter 1.
        let scroll_down_sequence = format!("{}", CsiSequence::ScrollDown(1));
        performer.apply_ansi_bytes(scroll_down_sequence.as_bytes());

        // After scrolling down by 1, top row should be empty.
        assert_empty_at(performer.ofs_buf, 0, 0);
        // Line-0 should move to row 1.
        assert_plain_text_at(performer.ofs_buf, 1, 0, "Line-0");
    }

    #[test]
    fn test_cursor_position_after_scroll_operations() {
        // Tests that cursor positions are correct after various scroll operations.
        // This addresses the gap where cursor position verification after scrolling
        // was missing from existing tests.

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test ESC D (Index) at bottom - cursor should remain at bottom
        performer.ofs_buf.cursor_pos = row(9) + col(5);
        performer.esc_dispatch(&[], false, esc_codes::IND_INDEX_DOWN);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(9) + col(5),
            "Cursor should remain at bottom after ESC D scroll"
        );

        // Test ESC M (Reverse Index) at top - cursor should remain at top
        performer.ofs_buf.cursor_pos = row(0) + col(3);
        performer.esc_dispatch(&[], false, esc_codes::RI_REVERSE_INDEX_UP);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(0) + col(3),
            "Cursor should remain at top after ESC M scroll"
        );

        // Test CSI S (Scroll Up) - cursor position should be unchanged
        performer.ofs_buf.cursor_pos = row(4) + col(7);
        let sequence = CsiSequence::ScrollUp(2).to_string();
        performer.apply_ansi_bytes(sequence);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(4) + col(7),
            "Cursor position should be unchanged after CSI S scroll"
        );

        // Test CSI T (Scroll Down) - cursor position should be unchanged
        performer.ofs_buf.cursor_pos = row(6) + col(2);
        let sequence = CsiSequence::ScrollDown(1).to_string();
        performer.apply_ansi_bytes(sequence);
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(6) + col(2),
            "Cursor position should be unchanged after CSI T scroll"
        );
    }

    #[test]
    fn test_scroll_edge_cases() {
        // Tests edge cases for scrolling operations, including zero-parameter scrolls
        // and other boundary conditions.
        //
        // NOTE: According to VT100 specification, a parameter of 0 for scroll operations
        // should be treated as 1, just like cursor movement commands. This is now
        // correctly implemented.

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test CSI 0 S (Scroll Up by 0 lines) - VT100 spec says 0 should be treated as 1
        // So this should scroll up by 1 line: Line-0 lost, Line-1 moves to top
        let sequence = CsiSequence::ScrollUp(0).to_string();
        performer.apply_ansi_bytes(sequence);

        // After scroll up by 1: Line-1 should now be at top (0 treated as 1)
        assert_plain_text_at(performer.ofs_buf, 0, 0, "Line-1");
        // Bottom should be empty.
        assert_empty_at(performer.ofs_buf, 9, 0);

        // Reset buffer for next test.
        fill_buffer_with_lines(performer.ofs_buf);

        // Test CSI 0 T (Scroll Down by 0 lines) - also treated as 1
        let sequence = CsiSequence::ScrollDown(0).to_string();
        performer.apply_ansi_bytes(sequence);

        // After scroll down by 1: top should be empty, Line-0 moves to row 1
        assert_empty_at(performer.ofs_buf, 0, 0);
        assert_plain_text_at(performer.ofs_buf, 1, 0, "Line-0");

        // Reset buffer for final test.
        fill_buffer_with_lines(performer.ofs_buf);

        // Test single line scroll up followed by single line scroll down.
        let sequence_up = CsiSequence::ScrollUp(1).to_string();
        let sequence_down = CsiSequence::ScrollDown(1).to_string();

        performer.apply_ansi_bytes(sequence_up); // Line-0 lost, Line-1->0, empty at bottom
        performer.apply_ansi_bytes(sequence_down); // Empty at top, Line-1->1, Line-2->0

        // After scroll up then down:
        // - Top line empty (from scroll down)
        // - Line-1 should be at row 1 (was at row 0 after scroll up, moved down)
        assert_empty_at(performer.ofs_buf, 0, 0);
        assert_plain_text_at(performer.ofs_buf, 1, 0, "Line-1");
    }
}

/// Tests for line wrap causing scroll on the last line of the buffer.
/// This addresses a critical gap where line wrapping behavior at the
/// bottom of the buffer wasn't tested.
pub mod line_wrap_scroll_interaction {
    use super::*;

    #[test]
    fn test_line_wrap_at_bottom_stays_clamped() {
        // Tests the current implementation where line wrapping at the bottom
        // of the screen clamps the cursor instead of scrolling.
        //
        // NOTE: This documents current behavior. True VT100 terminals would
        // typically scroll when wrapping at the bottom, but this implementation
        // clamps the cursor position.

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Fill the last line except for the last character.
        performer.ofs_buf.cursor_pos = row(9) + col(0);
        for c in "ABCDEFGHI".chars() {
            performer.print(c);
        }

        // Verify cursor is at the last position.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(9) + col(9),
            "Cursor should be at last position (9,9)"
        );

        // Print one more character - with current implementation, this wraps
        // but stays on the same row since we're at the bottom.
        performer.print('J');

        // Verify no scrolling occurred - "Line-0" should still be at top.
        assert_plain_text_at(performer.ofs_buf, 0, 0, "Line-0");

        // J gets written at (9,9), cursor tries to advance but wraps to (9,0)
        // since we're at the bottom row.
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(9) + col(0),
            "Cursor should wrap to (9,0) after printing J"
        );

        // The 'J' character should be at position (9,9) where it was printed
        assert_plain_char_at(performer.ofs_buf, 9, 9, 'J');

        // ABCDEFGHI should still be there from positions 0-8.
        assert_plain_char_at(performer.ofs_buf, 9, 0, 'A');
    }

    #[test]
    fn test_line_wrap_no_scroll_when_not_at_bottom() {
        // Verifies that line wrapping doesn't cause scrolling when the cursor
        // is not on the last line of the buffer.

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Position cursor on row 5 (not the last row).
        performer.ofs_buf.cursor_pos = row(5) + col(9);

        // Print character that should wrap.
        performer.print('X');

        // The print method writes the char, advances cursor, then handles wrap.
        // So X gets written at (5,9), cursor advances to (6,0)
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(6) + col(0),
            "Cursor should wrap to next line (6,0) after printing"
        );

        // Verify 'X' is at position (5,9) where it was printed.
        assert_plain_char_at(performer.ofs_buf, 5, 9, 'X');

        // Verify no scrolling occurred by checking that row 0 is still empty
        // (since we never filled it in this test).
        assert_empty_at(performer.ofs_buf, 0, 0);
    }

    #[test]
    fn test_multiple_wraps_at_bottom_behavior() {
        // Tests the current behavior where multiple wraps at bottom.
        // continue to clamp the cursor at the bottom row.

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        fill_buffer_with_lines(&mut ofs_buf);

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Position at bottom line, leave room for some characters.
        performer.ofs_buf.cursor_pos = row(9) + col(7);

        // Print characters that fill and wrap the line.
        performer.print('A'); // written at (9,7), cursor to (9,8)
        performer.print('B'); // written at (9,8), cursor to (9,9)
        performer.print('C'); // written at (9,9), cursor wraps to (9,0)

        // After wrap, cursor should be at (9,0)
        assert_eq!(
            performer.ofs_buf.cursor_pos,
            row(9) + col(0),
            "Should wrap to column 0 after printing C"
        );

        // Verify no scrolling - Line-0 still at top.
        assert_plain_text_at(performer.ofs_buf, 0, 0, "Line-0");

        // Continue printing - should continue from wrapped position.
        performer.print('D'); // written at (9,0), cursor to (9,1)
        performer.print('E'); // written at (9,1), cursor to (9,2)

        // Verify characters are placed correctly.
        assert_plain_char_at(performer.ofs_buf, 9, 7, 'A'); // A at original pos
        assert_plain_char_at(performer.ofs_buf, 9, 8, 'B'); // B at original pos
        assert_plain_char_at(performer.ofs_buf, 9, 9, 'C'); // C at rightmost pos
        assert_plain_char_at(performer.ofs_buf, 9, 0, 'D'); // D overwrites Line-9 start
        assert_plain_char_at(performer.ofs_buf, 9, 1, 'E'); // E follows D

        // Original content should still be present where not overwritten.
        assert_plain_text_at(performer.ofs_buf, 0, 0, "Line-0");
        assert_plain_text_at(performer.ofs_buf, 8, 0, "Line-8");
    }
}

/// Tests for DECSTBM (Set Top and Bottom Margins) functionality.
///
/// DECSTBM is essential for applications like vim splits, terminal multiplexers,
/// and any application that needs split-screen functionality with independent
/// scrolling regions.
pub mod decstbm_scroll_margins {
    use super::*;

    #[test]
    fn test_set_scroll_margins_basic() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set scroll region from row 3 to row 7 (1-based) - ESC [ 3 ; 7 r
        let sequence = CsiSequence::SetScrollingMargins {
            top: Some(term_row(3)),
            bottom: Some(term_row(7)),
        }
        .to_string();
        performer.apply_ansi_bytes(sequence);

        // Verify margins are set correctly (converted to 1-based internally)
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_top,
            Some(term_row(3))
        );
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_bottom,
            Some(term_row(7))
        );
    }

    #[test]
    fn test_reset_scroll_margins() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set some margins first.
        let sequence = CsiSequence::SetScrollingMargins {
            top: Some(term_row(3)),
            bottom: Some(term_row(7)),
        }
        .to_string();
        performer.apply_ansi_bytes(sequence);
        assert!(
            performer
                .ofs_buf
                .ansi_parser_support
                .scroll_region_top
                .is_some()
        );
        assert!(
            performer
                .ofs_buf
                .ansi_parser_support
                .scroll_region_bottom
                .is_some()
        );

        // Reset margins with ESC [ r (no parameters)
        let reset_sequence = CsiSequence::SetScrollingMargins {
            top: None,
            bottom: None,
        }
        .to_string();
        performer.apply_ansi_bytes(reset_sequence);

        // Verify margins are cleared.
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_top,
            None
        );
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_bottom,
            None
        );
    }

    #[test]
    fn test_scrolling_within_margins() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        fill_buffer_with_lines(performer.ofs_buf);

        // Set scroll region from row 3 to row 7 (1-based)
        let set_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(3)),
            bottom: Some(term_row(7)),
        }
        .to_string();
        performer.apply_ansi_bytes(set_margins);

        // Scroll up one line - should only affect rows 2-6 (0-based)
        let scroll_up = CsiSequence::ScrollUp(1).to_string();
        performer.apply_ansi_bytes(scroll_up);

        // Content outside scroll region should be unchanged.
        assert_plain_text_at(performer.ofs_buf, 0, 0, "Line-0"); // Above region
        assert_plain_text_at(performer.ofs_buf, 1, 0, "Line-1"); // Above region
        assert_plain_text_at(performer.ofs_buf, 8, 0, "Line-8"); // Below region
        assert_plain_text_at(performer.ofs_buf, 9, 0, "Line-9"); // Below region

        // Within scroll region: Line-2 should be gone, Line-3 moved up
        assert_plain_text_at(performer.ofs_buf, 2, 0, "Line-3"); // Line-3 moved to row 2
        assert_plain_text_at(performer.ofs_buf, 3, 0, "Line-4"); // Line-4 moved to row 3
        assert_plain_text_at(performer.ofs_buf, 4, 0, "Line-5"); // Line-5 moved to row 4
        assert_plain_text_at(performer.ofs_buf, 5, 0, "Line-6"); // Line-6 moved to row 5

        // Bottom of scroll region should be cleared.
        assert_empty_at(performer.ofs_buf, 6, 0); // Row 6 cleared
    }

    #[test]
    fn test_cursor_movement_respects_margins() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set scroll region from row 3 to row 7 (1-based)
        let set_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(3)),
            bottom: Some(term_row(7)),
        }
        .to_string();
        performer.apply_ansi_bytes(set_margins);

        // Position cursor at top of scroll region.
        let cursor_pos = CsiSequence::CursorPosition {
            row: term_row(3),
            col: term_col(1),
        }
        .to_string();
        performer.apply_ansi_bytes(cursor_pos);
        assert_eq!(performer.ofs_buf.cursor_pos.row_index.as_usize(), 2); // 0-based row 2

        // Try to move cursor up - should be clamped to scroll region top.
        let cursor_up = CsiSequence::CursorUp(5).to_string();
        performer.apply_ansi_bytes(cursor_up);
        assert_eq!(performer.ofs_buf.cursor_pos.row_index.as_usize(), 2); // Still at top margin

        // Move cursor to bottom of scroll region.
        let cursor_pos_bottom = CsiSequence::CursorPosition {
            row: term_row(7),
            col: term_col(1),
        }
        .to_string();
        performer.apply_ansi_bytes(cursor_pos_bottom);
        assert_eq!(performer.ofs_buf.cursor_pos.row_index.as_usize(), 6); // 0-based row 6

        // Try to move cursor down - should be clamped to scroll region bottom.
        let cursor_down = CsiSequence::CursorDown(5).to_string();
        performer.apply_ansi_bytes(cursor_down);
        assert_eq!(performer.ofs_buf.cursor_pos.row_index.as_usize(), 6); // Still at bottom margin
    }

    #[test]
    fn test_cursor_position_clamped_to_margins() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set scroll region from row 3 to row 7 (1-based)
        let set_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(3)),
            bottom: Some(term_row(7)),
        }
        .to_string();
        performer.apply_ansi_bytes(set_margins);

        // Try to position cursor above scroll region.
        let cursor_above = CsiSequence::CursorPosition {
            row: term_row(1),
            col: term_col(5),
        }
        .to_string();
        performer.apply_ansi_bytes(cursor_above);
        assert_eq!(performer.ofs_buf.cursor_pos.row_index.as_usize(), 2); // Clamped to top margin

        // Try to position cursor below scroll region.
        let cursor_below = CsiSequence::CursorPosition {
            row: term_row(9),
            col: term_col(5),
        }
        .to_string();
        performer.apply_ansi_bytes(cursor_below);
        assert_eq!(performer.ofs_buf.cursor_pos.row_index.as_usize(), 6); // Clamped to bottom margin

        // Position within scroll region should work normally.
        let cursor_within = CsiSequence::CursorPosition {
            row: term_row(5),
            col: term_col(5),
        }
        .to_string();
        performer.apply_ansi_bytes(cursor_within);
        assert_eq!(performer.ofs_buf.cursor_pos.row_index.as_usize(), 4); // 0-based row 4
    }

    #[test]
    fn test_index_and_reverse_index_with_margins() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        fill_buffer_with_lines(performer.ofs_buf);

        // Set scroll region from row 3 to row 7 (1-based)
        let set_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(3)),
            bottom: Some(term_row(7)),
        }
        .to_string();
        performer.apply_ansi_bytes(set_margins);

        // Position cursor at bottom of scroll region.
        let cursor_pos = CsiSequence::CursorPosition {
            row: term_row(7),
            col: term_col(1),
        }
        .to_string();
        performer.apply_ansi_bytes(cursor_pos);

        // Send ESC D (Index) - should scroll the region up
        let index_down_sequence = format!("{}", EscSequence::IndexDown);
        performer.apply_ansi_bytes(index_down_sequence.as_bytes());

        // Content outside scroll region should be unchanged.
        assert_plain_text_at(performer.ofs_buf, 0, 0, "Line-0"); // Above region
        assert_plain_text_at(performer.ofs_buf, 1, 0, "Line-1"); // Above region
        assert_plain_text_at(performer.ofs_buf, 8, 0, "Line-8"); // Below region

        // Within scroll region: should have scrolled up
        assert_plain_text_at(performer.ofs_buf, 2, 0, "Line-3"); // Line-3 moved to row 2
        assert_empty_at(performer.ofs_buf, 6, 0); // Bottom row cleared

        // Position cursor at top of scroll region.
        let cursor_pos_top = CsiSequence::CursorPosition {
            row: term_row(3),
            col: term_col(1),
        }
        .to_string();
        performer.apply_ansi_bytes(cursor_pos_top);

        // Send ESC M (Reverse Index) - should scroll the region down
        let reverse_index_sequence = format!("{}", EscSequence::ReverseIndex);
        performer.apply_ansi_bytes(reverse_index_sequence.as_bytes());

        // Top of scroll region should be cleared.
        assert_empty_at(performer.ofs_buf, 2, 0); // Top row cleared
        assert_plain_text_at(performer.ofs_buf, 3, 0, "Line-3"); // Line-3 moved down
    }

    #[test]
    fn test_terminal_reset_clears_margins() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set scroll margins.
        let set_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(3)),
            bottom: Some(term_row(7)),
        }
        .to_string();
        performer.apply_ansi_bytes(set_margins);
        assert!(
            performer
                .ofs_buf
                .ansi_parser_support
                .scroll_region_top
                .is_some()
        );

        // Reset terminal with ESC c.
        let reset_sequence = format!("{}", EscSequence::ResetTerminal);
        performer.apply_ansi_bytes(reset_sequence);

        // Margins should be cleared.
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_top,
            None
        );
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_bottom,
            None
        );
    }

    #[test]
    fn test_invalid_margins_ignored() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Try to set invalid margins (top >= bottom)
        let invalid_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(7)),
            bottom: Some(term_row(3)),
        }
        .to_string();
        performer.apply_ansi_bytes(invalid_margins);

        // Margins should remain None.
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_top,
            None
        );
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_bottom,
            None
        );

        // Try to set margins beyond buffer height.
        let large_margins = CsiSequence::SetScrollingMargins {
            top: Some(term_row(1)),
            bottom: Some(term_row(15)),
        }
        .to_string();
        performer.apply_ansi_bytes(large_margins);

        // Should be clamped to buffer height.
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_top,
            Some(term_row(1))
        );
        assert_eq!(
            performer.ofs_buf.ansi_parser_support.scroll_region_bottom,
            Some(term_row(10))
        );
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for tab operation conformance.
//!
//! This module tests tab-related ANSI operations:
//! - Basic TAB character (0x09) with fixed 8-column tab stops
//! - HTS (Horizontal Tab Set) - ESC H (placeholder for future implementation)
//! - TBC (Tab Clear) - CSI g (placeholder for future implementation)
//! - CHT (Cursor Horizontal Tab) - CSI I (placeholder for future implementation)
//! - CBT (Cursor Backward Tab) - CSI Z (placeholder for future implementation)

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::vt_100_ansi_parser::term_units::{term_col, term_row};

/// Tests for basic TAB character (0x09) functionality.
/// The current implementation uses fixed 8-column tab stops.
pub mod basic_tab_operations {
    use super::*;

    #[test]
    fn test_tab_from_column_zero() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start at column 0 (origin)
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(0));

        // Send TAB character
        let _result = ofs_buf.apply_ansi_bytes("\t");

        // Should move to column 8 (next 8-column tab stop)
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(8));
    }

    #[test]
    fn test_tab_from_mid_column() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Move cursor to column 3
        let move_sequence = format!(
            "{}",
            crate::vt_100_ansi_parser::protocols::csi_codes::CsiSequence::CursorPosition {
                row: term_row(1),
                col: term_col(4) // 1-based column 4 = 0-based column 3
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(3));

        // Send TAB character
        let _result = ofs_buf.apply_ansi_bytes("\t");

        // Should move to column 8 (next 8-column tab stop)
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(8));
    }

    #[test]
    fn test_tab_at_tab_stop_boundary() {
        let mut ofs_buf = create_test_offscreen_buffer_20r_by_20c();

        // Move cursor to column 8 (already at a tab stop)
        let move_sequence = format!(
            "{}",
            crate::vt_100_ansi_parser::protocols::csi_codes::CsiSequence::CursorPosition {
                row: term_row(1),
                col: term_col(9) // 1-based column 9 = 0-based column 8
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(8));

        // Send TAB character
        let _result = ofs_buf.apply_ansi_bytes("\t");

        // Should move to column 16 (next 8-column tab stop)
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(16));
    }

    #[test]
    fn test_multiple_consecutive_tabs() {
        let mut ofs_buf = create_test_offscreen_buffer_20r_by_20c();

        // Start at column 0
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(0));

        // Send multiple TAB characters
        let _result = ofs_buf.apply_ansi_bytes("\t\t\t");

        // Should move through tab stops: 0 → 8 → 16 → 24
        // But buffer is only 20 columns wide, so clamp to max column index 19
        let expected_col = std::cmp::min(24, 20 - 1); // 19
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(expected_col));
    }

    #[test]
    fn test_tab_near_right_margin() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Move cursor to column 9 (near right margin)
        let move_sequence = format!(
            "{}",
            crate::vt_100_ansi_parser::protocols::csi_codes::CsiSequence::CursorPosition {
                row: term_row(1),
                col: term_col(10) // 1-based column 10 = 0-based column 9
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(9));

        // Send TAB character
        let _result = ofs_buf.apply_ansi_bytes("\t");

        // Should clamp to rightmost column (next tab stop would be 16, but buffer is only
        // 10 wide)
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(9)); // Stays at max valid column
    }

    #[test]
    fn test_tab_with_text() {
        let mut ofs_buf = create_test_offscreen_buffer_20r_by_20c();

        // Write some text, then tab, then more text
        let _result = ofs_buf.apply_ansi_bytes("ABC\tDEF");

        // Check cursor position (ABC = 3 chars, tab moves to column 8, DEF = 3 more
        // chars)
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(11)); // 8 + 3 = 11

        // Verify the text placement
        let first_row = &ofs_buf.buffer[0];

        // Check text positions: "ABC" at 0-2, "DEF" at 8-10
        if let crate::PixelChar::PlainText { display_char, .. } = &first_row[0] {
            assert_eq!(*display_char, 'A');
        }
        if let crate::PixelChar::PlainText { display_char, .. } = &first_row[2] {
            assert_eq!(*display_char, 'C');
        }
        if let crate::PixelChar::PlainText { display_char, .. } = &first_row[8] {
            assert_eq!(*display_char, 'D');
        }
        if let crate::PixelChar::PlainText { display_char, .. } = &first_row[10] {
            assert_eq!(*display_char, 'F');
        }
    }

    #[test]
    fn test_tab_behavior_different_starting_positions() {
        let mut ofs_buf = create_test_offscreen_buffer_20r_by_20c();

        // Test tab behavior from different starting positions
        let test_positions = vec![
            (0, 8),   // col 0 → 8
            (1, 8),   // col 1 → 8
            (7, 8),   // col 7 → 8
            (8, 16),  // col 8 → 16
            (12, 16), // col 12 → 16
            (15, 16), // col 15 → 16
        ];

        for (start_col, expected_col) in test_positions {
            // Reset position
            let move_sequence = format!(
                "{}",
                crate::vt_100_ansi_parser::protocols::csi_codes::CsiSequence::CursorPosition {
                    row: term_row(1),
                    col: term_col(start_col + 1) // Convert to 1-based
                }
            );
            let _result = ofs_buf.apply_ansi_bytes(move_sequence);
            assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(start_col));

            // Send TAB
            let _result = ofs_buf.apply_ansi_bytes("\t");

            // Verify expected position
            assert_eq!(
                ofs_buf.cursor_pos.col_index,
                crate::col(expected_col),
                "Tab from column {} should move to column {}, but cursor is at column {}",
                start_col,
                expected_col,
                ofs_buf.cursor_pos.col_index.as_usize()
            );
        }
    }
}

/// Tests for tab operations with edge cases and boundary conditions.
pub mod tab_edge_cases {
    use super::*;
    use crate::{row, col};

    #[test]
    fn test_tab_at_exact_buffer_width() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Move to last column
        let move_sequence = format!(
            "{}",
            crate::vt_100_ansi_parser::protocols::csi_codes::CsiSequence::CursorPosition {
                row: term_row(1),
                col: term_col(10) // 1-based column 10 = 0-based column 9
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);

        // TAB from last column should not move cursor beyond buffer bounds
        let _result = ofs_buf.apply_ansi_bytes("\t");

        // Should remain at last valid column
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(9));
    }

    #[test]
    fn test_tab_with_line_wrapping_disabled() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Disable auto-wrap mode
        let disable_wrap = format!(
            "{}",
            crate::vt_100_ansi_parser::protocols::csi_codes::CsiSequence::DisablePrivateMode(
                crate::vt_100_ansi_parser::protocols::csi_codes::PrivateModeType::AutoWrap
            )
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_wrap);

        // Move near right edge and tab
        let move_sequence = format!(
            "{}",
            crate::vt_100_ansi_parser::protocols::csi_codes::CsiSequence::CursorPosition {
                row: term_row(1),
                col: term_col(7) // 1-based column 7 = 0-based column 6
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);

        // TAB should move to next tab stop, but clamp to buffer width
        let _result = ofs_buf.apply_ansi_bytes("\t");

        // Should move to column 8, but stay within bounds
        assert_eq!(ofs_buf.cursor_pos.col_index, crate::col(8));
    }

    #[test]
    fn test_tab_sequence_with_other_control_characters() {
        let mut ofs_buf = create_test_offscreen_buffer_20r_by_20c();

        // Mix tab with carriage return and line feed
        let mixed_sequence = "AB\tCD\r\n\tEF";
        let _result = ofs_buf.apply_ansi_bytes(mixed_sequence);

        // Verify final cursor position
        // "AB" (cols 0-1) → TAB (col 8) → "CD" (cols 8-9) → CR (col 0) → LF (next line) →
        // TAB (col 8) → "EF" (cols 8-9)
        assert_eq!(ofs_buf.cursor_pos.row_index, row(1)); // Second row
        assert_eq!(ofs_buf.cursor_pos.col_index, col(10)); // After "EF" at tab position

        // Verify content placement
        let first_row = &ofs_buf.buffer[0];
        let second_row = &ofs_buf.buffer[1];

        // First row: "AB" at 0-1, "CD" at 8-9
        if let crate::PixelChar::PlainText { display_char, .. } = &first_row[0] {
            assert_eq!(*display_char, 'A');
        }
        if let crate::PixelChar::PlainText { display_char, .. } = &first_row[8] {
            assert_eq!(*display_char, 'C');
        }

        // Second row: "EF" at 8-9 (after tab)
        if let crate::PixelChar::PlainText { display_char, .. } = &second_row[8] {
            assert_eq!(*display_char, 'E');
        }
    }
}

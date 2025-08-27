// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for line management and buffer control - wrapping, auto-wrap mode, and scrolling.

use vte::Perform;

use super::create_test_offscreen_buffer_10r_by_10c;
use crate::{ansi_parser::{ansi_parser_public_api::AnsiToBufferProcessor,
                          csi_codes::{CsiSequence, DECAWM_AUTO_WRAP}},
            col, row,
            offscreen_buffer::test_fixtures_offscreen_buffer::*};

/// Tests for auto-wrap mode (DECAWM) functionality.
pub mod auto_wrap {
    use super::*;

    #[test]
    fn test_auto_wrap_enabled_by_default() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Auto-wrap should be enabled by default
        // This test verifies that characters wrap to the next line when reaching the right margin

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Verify auto-wrap is enabled by default
            assert!(
                processor.ofs_buf.ansi_parser_support.auto_wrap_mode,
                "Auto-wrap mode should be enabled by default"
            );

            // Write 10 characters to fill the first line (0-9)
            for i in 0..10 {
                let ch = (b'0' + i) as char;
                processor.print(ch);
            }

            // Verify cursor is at (0, 10) - beyond the last column
            // The 11th character should wrap to the next line
            processor.print('A');

            // Verify cursor wrapped to next line
            assert_eq!(processor.cursor_pos.row_index, row(1));
            assert_eq!(processor.cursor_pos.col_index, col(1)); // After 'A'
        }

        // Verify buffer contents
        assert_plain_text_at(&ofs_buf, 0, 0, "0123456789");
        assert_plain_char_at(&ofs_buf, 1, 0, 'A');
    }

    #[test]
    fn test_auto_wrap_can_be_disabled() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Disable auto-wrap mode using CSI ?7l
            let sequence = CsiSequence::DisablePrivateMode(DECAWM_AUTO_WRAP).to_string();
            processor.process_bytes(sequence);

            // Verify auto-wrap is now disabled
            assert!(
                !processor.ofs_buf.ansi_parser_support.auto_wrap_mode,
                "Auto-wrap mode should be disabled after CSI ?7l"
            );

            // Fill the line
            for i in 0..10 {
                let ch = (b'0' + i) as char;
                processor.print(ch);
            }

            // Try to write beyond the margin - should clamp at right edge
            processor.print('X');

            // Verify cursor stays at right margin
            assert_eq!(processor.cursor_pos.row_index, row(0));
            assert_eq!(processor.cursor_pos.col_index, col(9)); // Clamped at last column
        }

        // Verify buffer contents - 'X' should overwrite '9'
        assert_plain_text_at(&ofs_buf, 0, 0, "012345678X");
    }

    #[test]
    fn test_auto_wrap_can_be_toggled() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Start with default (enabled)
            assert!(processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

            // Disable auto-wrap
            let disable_sequence = CsiSequence::DisablePrivateMode(DECAWM_AUTO_WRAP).to_string();
            processor.process_bytes(disable_sequence);
            assert!(!processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

            // Re-enable auto-wrap using CSI ?7h
            let enable_sequence = CsiSequence::EnablePrivateMode(DECAWM_AUTO_WRAP).to_string();
            processor.process_bytes(enable_sequence);
            assert!(processor.ofs_buf.ansi_parser_support.auto_wrap_mode);

            // Test that wrapping works again
            for i in 0..11 { // 11 characters should wrap
                let ch = (b'A' + i) as char;
                processor.print(ch);
            }

            // Verify wrapping occurred
            assert_eq!(processor.cursor_pos.row_index, row(1));
            assert_eq!(processor.cursor_pos.col_index, col(1)); // After 'K'
        }

        // Verify buffer contents
        assert_plain_text_at(&ofs_buf, 0, 0, "ABCDEFGHIJ");
        assert_plain_char_at(&ofs_buf, 1, 0, 'K');
    }

    #[test]
    fn test_auto_wrap_mode_change_effect_is_immediate() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Test auto-wrap mode changes
            // Fill the first line to column 9 (last column)
            for i in 0..10 {
                let ch = (b'0' + i) as char;
                processor.print(ch);
            }
            
            // Now cursor should be at (1, 0) after wrapping
            assert_eq!(processor.cursor_pos, row(1) + col(0));
            
            // Disable auto-wrap mode
            let sequence = CsiSequence::DisablePrivateMode(DECAWM_AUTO_WRAP).to_string();
            processor.process_bytes(sequence);
            
            // Move to end of line 2 and test clamping
            processor.cursor_pos = row(2) + col(9);
            processor.print('X'); // At boundary
            processor.print('Y'); // Should clamp to (2, 9) and overwrite 'X'

            // Re-enable auto-wrap mode
            let sequence = CsiSequence::EnablePrivateMode(DECAWM_AUTO_WRAP).to_string();
            processor.process_bytes(sequence);

            // Move to a new position and test wrapping again
            processor.cursor_pos = row(2) + col(9);
            processor.print('A');
            processor.print('B'); // Should wrap to row 3

            // Verify final cursor position
            assert_eq!(processor.cursor_pos.row_index, row(3));
            assert_eq!(processor.cursor_pos.col_index, col(1));
        }

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
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Write 10 characters to fill the line
            for i in 0..10 {
                let ch = (b'A' + i) as char;
                processor.print(ch);
            }

            // 11th character should wrap to next line
            processor.print('K');

            // Verify cursor wrapped to next line
            assert_eq!(processor.cursor_pos.row_index.as_usize(), 1);
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 1);
        }

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

/// Tests for buffer scrolling operations (placeholder for future scrolling tests).
pub mod scrolling {
    use super::*;

    // Note: Scrolling operations (ESC D, ESC M, CSI S, CSI T) are implemented
    // in the main implementation but not yet extensively tested here.
    // This module is a placeholder for future scrolling tests.

    #[test]
    fn test_scrolling_placeholder() {
        let _ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        // TODO: Add comprehensive scrolling tests when scrolling operations
        // are moved from other test files or when new scrolling functionality
        // is added.
    }
}
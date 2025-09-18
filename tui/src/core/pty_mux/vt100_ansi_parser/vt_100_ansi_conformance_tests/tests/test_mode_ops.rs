// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for terminal mode operations (SM/RM).
//!
//! This module tests mode setting and resetting operations including:
//! - DECAWM (Auto Wrap Mode) - CSI ? 7 h/l
//! - Future IRM (Insert/Replace Mode) - CSI 4 h/l (placeholder tests)
//! - Future DECOM (Origin Mode) - CSI ? 6 h/l (placeholder tests)

use super::super::test_fixtures::*;
use crate::vt100_ansi_parser::{protocols::csi_codes::{CsiSequence, PrivateModeType},
                               term_units::{term_col, term_row}};

/// Tests for DECAWM (Auto Wrap Mode) operations.
pub mod auto_wrap_mode {
    use super::*;

    #[test]
    fn test_decawm_enable() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Auto wrap is enabled by default
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Disable first to test enable
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Enable auto wrap mode
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);

        // Verify mode is enabled
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);
    }

    #[test]
    fn test_decawm_disable() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Auto wrap is enabled by default
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Disable auto wrap mode
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);

        // Verify mode is disabled
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);
    }

    #[test]
    fn test_decawm_behavior_with_text_wrapping() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Enable auto wrap (default)
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);

        // Write text that exceeds line width
        let long_text = "ABCDEFGHIJKLMNOP"; // 16 chars, buffer is 10 wide
        let _result = ofs_buf.apply_ansi_bytes(long_text);

        // Should wrap to next line
        assert_line_content(&ofs_buf, 0, "ABCDEFGHIJ");
        assert_line_content(&ofs_buf, 1, "KLMNOP");
    }

    #[test]
    fn test_decawm_behavior_without_wrapping() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Disable auto wrap
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);

        // Write text that exceeds line width
        let long_text = "ABCDEFGHIJKLMNOP"; // 16 chars, buffer is 10 wide
        let _result = ofs_buf.apply_ansi_bytes(long_text);

        // Should not wrap, last character should overwrite at right margin
        assert_line_content(&ofs_buf, 0, "ABCDEFGHIP"); // Last 'P' overwrites 'J'
        assert_blank_line(&ofs_buf, 1); // Second line should be blank
    }

    #[test]
    fn test_decawm_mode_persistence() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Disable auto wrap
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Perform other operations
        let move_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(3),
                col: term_col(5)
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);
        let _result = ofs_buf.apply_ansi_bytes("Test");

        // Mode should persist
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Re-enable and verify
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);
    }
}

/// Tests for mode state combinations and interactions.
pub mod mode_interactions {
    use super::*;

    #[test]
    fn test_multiple_mode_changes() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start with defaults
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Toggle auto wrap multiple times
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);

        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);

        let disable_sequence2 = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence2);
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);
    }

    #[test]
    fn test_mode_with_cursor_save_restore() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Disable auto wrap
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Save cursor
        let save_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save_sequence);

        // Enable auto wrap
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Restore cursor
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        // Mode should persist (not affected by cursor restore)
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);
    }
}

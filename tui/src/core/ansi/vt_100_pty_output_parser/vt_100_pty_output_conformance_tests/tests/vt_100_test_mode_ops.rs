// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words KLMNOP ABCDEFGHIP

//! Tests for terminal mode operations (SM/RM).
//!
//! Tests the complete pipeline from [`ANSI`] sequences through the shim to implementation
//! using the public [`apply_ansi_bytes`] API. This provides integration testing coverage
//! for the [`mode_ops`] shim layer. The `test_` prefix follows our naming convention.
//! See [parser module docs] for the complete testing philosophy.
//!
//! This module tests mode setting and resetting operations including:
//! - DECAWM (Auto Wrap Mode) - [`CSI`] ? 7 h/l
//! - Future IRM (Insert/Replace Mode) - [`CSI`] 4 h/l (placeholder tests)
//! - Future DECOM (Origin Mode) - [`CSI`] ? 6 h/l (placeholder tests)
//!
//! **Related Files:**
//! - **Shim**: [`mode_ops`] - Parameter translation (tested indirectly by this module)
//! - **Implementation**: [`impl_mode_ops`] - Business logic (has separate unit tests)
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes
//! [`CSI`]: crate::CsiSequence
//! [`impl_mode_ops`]: crate::vt_100_ansi_impl::vt_100_impl_mode_ops
//! [`mode_ops`]: crate::vt_100_pty_output_parser::operations::vt_100_shim_mode_ops
//! [parser module docs]: super::super

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{core::ansi::vt_100_pty_output_parser::{CsiSequence, PrivateModeType},
            term_col, term_row};

/// Tests for DECAWM (Auto Wrap Mode) operations.
pub mod auto_wrap_mode {
    use crate::AutoWrapState;

use super::*;

    #[test]
    fn test_decawm_enable() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Auto wrap is enabled by default
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);

        // Disable first to test enable
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);

        // Enable auto wrap mode
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);

        // Verify mode is enabled
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);
    }

    #[test]
    fn test_decawm_disable() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Auto wrap is enabled by default
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);

        // Disable auto wrap mode
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);

        // Verify mode is disabled
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);
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
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);

        // Perform other operations
        let move_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(3)),
                col: term_col(nz(5))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);
        let _result = ofs_buf.apply_ansi_bytes("Test");

        // Mode should persist
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);

        // Re-enable and verify
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);
    }
}

/// Tests for mode state combinations and interactions.
pub mod mode_interactions {
    use crate::AutoWrapState;

use super::*;

    #[test]
    fn test_multiple_mode_changes() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start with defaults
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);

        // Toggle auto wrap multiple times
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);

        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);

        let disable_sequence2 = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence2);
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);
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
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);

        // Save cursor
        let save_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save_sequence);

        // Enable auto wrap
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);

        // Restore cursor
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        // Mode should persist (not affected by cursor restore)
        assert_eq!(ofs_buf.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);
    }
}

/// Tests for the Alternate Screen Buffer (?1049) mode operations.
pub mod alt_screen_mode {
    use super::*;
    use crate::AlternateScreenState;

    #[test]
    fn test_alt_screen_enable_and_disable_via_ansi() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Initially inactive.
        assert_eq!(
            ofs_buf.terminal_mode.alternate_screen,
            AlternateScreenState::Inactive
        );

        // Enable alternate screen buffer (?1049h)
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AlternateScreenBuffer)
        );
        let _result = ofs_buf.apply_ansi_bytes(enable_sequence);
        assert_eq!(
            ofs_buf.terminal_mode.alternate_screen,
            AlternateScreenState::Active
        );

        // Disable alternate screen buffer (?1049l)
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::AlternateScreenBuffer)
        );
        let _result = ofs_buf.apply_ansi_bytes(disable_sequence);
        assert_eq!(
            ofs_buf.terminal_mode.alternate_screen,
            AlternateScreenState::Inactive
        );
    }
}

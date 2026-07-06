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
//! - **Implementation**: [`vt_100_impl_mode_ops`] - Business logic (has separate unit
//!   tests)
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`apply_ansi_bytes`]: crate::OfsBufVT100::apply_ansi_bytes
//! [`CSI`]: crate::CsiSequence
//! [`mode_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_mode_ops
//! [`vt_100_impl_mode_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_mode_ops
//! [parser module docs]: super::super

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{core::ansi::vt_100_pty_output_parser::{CsiSequence, PrivateModeType},
            term_col, term_row};

/// Tests for DECAWM (Auto Wrap Mode) operations.
pub mod auto_wrap_mode {
    use super::*;
    use crate::AutoWrapMode;

    #[test]
    fn test_decawm_enable() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Auto wrap is enabled by default
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        // Disable first to test enable
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );

        // Enable auto wrap mode
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(enable_sequence);

        // Verify mode is enabled
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );
    }

    #[test]
    fn test_decawm_disable() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Auto wrap is enabled by default
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        // Disable auto wrap mode
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence);

        // Verify mode is disabled
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );
    }

    #[test]
    fn test_decawm_behavior_with_text_wrapping() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Enable auto wrap (default)
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(enable_sequence);

        // Write text that exceeds line width
        let long_text = "ABCDEFGHIJKLMNOP"; // 16 chars, buffer is 10 wide
        let _result = ofs_buf_vt_100.apply_ansi_bytes(long_text);

        // Should wrap to next line
        assert_line_content(&ofs_buf_vt_100, 0, "ABCDEFGHIJ");
        assert_line_content(&ofs_buf_vt_100, 1, "KLMNOP");
    }

    #[test]
    fn test_decawm_behavior_without_wrapping() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Disable auto wrap
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence);

        // Write text that exceeds line width
        let long_text = "ABCDEFGHIJKLMNOP"; // 16 chars, buffer is 10 wide
        let _result = ofs_buf_vt_100.apply_ansi_bytes(long_text);

        // Should not wrap, last character should overwrite at right margin
        assert_line_content(&ofs_buf_vt_100, 0, "ABCDEFGHIP"); // Last 'P' overwrites 'J'
        assert_blank_line(&ofs_buf_vt_100, 1); // Second line should be blank
    }

    #[test]
    fn test_decawm_mode_persistence() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Disable auto wrap
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );

        // Perform other operations
        let move_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(3)),
                col: term_col(nz(5))
            }
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(move_sequence);
        let _result = ofs_buf_vt_100.apply_ansi_bytes("Test");

        // Mode should persist
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );

        // Re-enable and verify
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(enable_sequence);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );
    }
}

/// Tests for mode state combinations and interactions.
pub mod mode_interactions {
    use super::*;
    use crate::AutoWrapMode;

    #[test]
    fn test_multiple_mode_changes() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Start with defaults
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        // Toggle auto wrap multiple times
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );

        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(enable_sequence);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        let disable_sequence2 = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence2);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );
    }

    #[test]
    fn test_mode_with_cursor_save_restore() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Disable auto wrap
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );

        // Save cursor
        let save_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf_vt_100.apply_ansi_bytes(save_sequence);

        // Enable auto wrap
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::AutoWrap])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(enable_sequence);
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        // Restore cursor
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf_vt_100.apply_ansi_bytes(restore_sequence);

        // Mode should persist (not affected by cursor restore)
        assert_eq!(
            ofs_buf_vt_100.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );
    }
}

/// Tests for the Alternate Screen Buffer (`?1049`) mode operations.
pub mod alt_screen_mode {
    use super::*;
    use crate::ActiveScreenBuffer;

    #[test]
    fn test_alt_screen_enable_and_disable_via_ansi() {
        let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

        // Initially inactive.
        assert_eq!(
            ofs_buf_vt_100.terminal_mode.active_screen_buffer,
            ActiveScreenBuffer::Primary
        );

        // Enable alternate screen buffer (`?1049h`)
        let enable_sequence = format!(
            "{}",
            CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::AlternateScreenBuffer])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(enable_sequence);
        assert_eq!(
            ofs_buf_vt_100.terminal_mode.active_screen_buffer,
            ActiveScreenBuffer::Alternate
        );

        // Disable alternate screen buffer (`?1049l`)
        let disable_sequence = format!(
            "{}",
            CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::AlternateScreenBuffer])
        );
        let _result = ofs_buf_vt_100.apply_ansi_bytes(disable_sequence);
        assert_eq!(
            ofs_buf_vt_100.terminal_mode.active_screen_buffer,
            ActiveScreenBuffer::Primary
        );
    }
}

/// Tests for Mouse Tracking Mode operations.
pub mod mouse_tracking_mode {
    use super::*;
    use crate::{MouseTrackingMode, MouseTrackingFormat};

    #[test]
    fn test_mouse_tracking_enable_and_disable() {
        let mut ofs_buf = create_test_ofs_buf_10r_by_10c();

        // Initially disabled.
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Disabled);

        // Enable legacy mouse tracking (1000)
        let enable_1000 = format!("{}", CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::X11MouseTracking]));
        let _unused = ofs_buf.apply_ansi_bytes(enable_1000);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Enabled);

        // Disable legacy mouse tracking
        let disable_1000 = format!("{}", CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::X11MouseTracking]));
        let _unused = ofs_buf.apply_ansi_bytes(disable_1000);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Disabled);

        // Enable legacy mouse tracking (1002)
        let enable_1002 = format!("{}", CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::CellMotionMouseTracking]));
        let _unused = ofs_buf.apply_ansi_bytes(enable_1002);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Enabled);
        let _unused = ofs_buf.apply_ansi_bytes(format!("{}", CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::CellMotionMouseTracking])));

        // Enable legacy mouse tracking (1003)
        let enable_1003 = format!("{}", CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::ApplicationMouseTracking]));
        let _unused = ofs_buf.apply_ansi_bytes(enable_1003);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Enabled);
        
        // Enable SGR mouse tracking (1006)
        let enable_1006 = format!("{}", CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::SgrMouseMode]));
        let _unused = ofs_buf.apply_ansi_bytes(enable_1006);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Enabled);

        // Disable SGR mouse tracking
        let disable_1006 = format!("{}", CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::SgrMouseMode]));
        let _unused = ofs_buf.apply_ansi_bytes(disable_1006);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Enabled);

        // Finally disable tracking
        let _unused = ofs_buf.apply_ansi_bytes(format!("{}", CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::ApplicationMouseTracking])));
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Disabled);
    }

    #[test]
    fn test_mouse_tracking_chained_modes() {
        let mut ofs_buf = create_test_ofs_buf_10r_by_10c();

        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Disabled);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_format, MouseTrackingFormat::X10);

        // Test htop's chained initialization sequence: enable both SGR (1006) and X11 (1000)
        let _unused = ofs_buf.apply_ansi_bytes("\x1b[?1006;1000h");

        // Both mode AND format should be correctly updated
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Enabled);
        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_format, MouseTrackingFormat::Sgr);

        // Test chained de-initialization: disable both SGR (1006) and X11 (1000)
        let _unused = ofs_buf.apply_ansi_bytes("\x1b[?1006;1000l");

        assert_eq!(ofs_buf.terminal_mode.mouse_tracking_mode, MouseTrackingMode::Disabled);
    }
}

/// Tests for Cursor Key Mode operations.
pub mod cursor_key_mode {
    use super::*;
    use crate::CursorKeyMode;

    #[test]
    fn test_cursor_key_mode_enable_and_disable() {
        let mut ofs_buf = create_test_ofs_buf_10r_by_10c();

        // Reset to normal mode first to ensure a known state
        let disable = format!("{}", CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::CursorKeys]));
        let _unused = ofs_buf.apply_ansi_bytes(disable);
        assert_eq!(ofs_buf.terminal_mode.cursor_key_mode, CursorKeyMode::Normal);

        // Enable cursor key application mode (?1h)
        let enable = format!("{}", CsiSequence::EnablePrivateMode(smallvec::smallvec![PrivateModeType::CursorKeys]));
        let _unused = ofs_buf.apply_ansi_bytes(enable);
        assert_eq!(ofs_buf.terminal_mode.cursor_key_mode, CursorKeyMode::Application);

        // Disable cursor key application mode (?1l)
        let disable = format!("{}", CsiSequence::DisablePrivateMode(smallvec::smallvec![PrivateModeType::CursorKeys]));
        let _unused = ofs_buf.apply_ansi_bytes(disable);
        assert_eq!(ofs_buf.terminal_mode.cursor_key_mode, CursorKeyMode::Normal);
    }
}

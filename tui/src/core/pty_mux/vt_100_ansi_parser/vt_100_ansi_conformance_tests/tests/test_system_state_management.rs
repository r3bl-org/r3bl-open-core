// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for complex terminal state management operations.
//!
//! This module tests the preservation and interaction of various terminal states:
//! - Cursor save/restore with active SGR attributes
//! - Character set preservation across save/restore operations
//! - Mode state interactions with cursor operations
//! - Scroll region effects on cursor save/restore

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{ANSIBasicColor, SgrCode, col, row,
            vt_100_ansi_parser::{protocols::csi_codes::{CsiSequence, PrivateModeType},
                                 term_units::{term_col, term_row}}};

/// Tests for cursor save/restore with active SGR styling attributes.
pub mod cursor_save_restore_with_attributes {
    use super::*;

    #[test]
    fn test_cursor_save_restore_preserves_position_only() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Move cursor to specific position
        let move_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(3)),
                col: term_col(nz(5))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);

        // Set some SGR attributes
        let style_sequence = format!(
            "{}{}{}",
            SgrCode::Bold,
            SgrCode::ForegroundBasic(ANSIBasicColor::Red),
            SgrCode::BackgroundBasic(ANSIBasicColor::Blue)
        );
        let _result = ofs_buf.apply_ansi_bytes(style_sequence);

        // Save cursor (should only save position, not attributes)
        let save_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save_sequence);

        // Move cursor and change attributes
        let move_sequence2 = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(7)),
                col: term_col(nz(8))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence2);

        let style_sequence2 = format!(
            "{}{}",
            SgrCode::Reset,
            SgrCode::ForegroundBasic(ANSIBasicColor::Green)
        );
        let _result = ofs_buf.apply_ansi_bytes(style_sequence2);

        // Restore cursor
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        // Position should be restored
        assert_eq!(ofs_buf.cursor_pos, row(2) + col(4)); // 0-based

        // Attributes should remain as they were changed (not restored)
        // Current style should be green foreground from the change above
        let current_style = ofs_buf.ansi_parser_support.current_style;
        assert_eq!(
            current_style.color_fg,
            Some(crate::TuiColor::Basic(ANSIBasicColor::Green))
        );
    }

    #[test]
    fn test_cursor_save_restore_multiple_times() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // First save at origin
        let save1_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save1_sequence);

        // Move to position 1
        let move1_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(2)),
                col: term_col(nz(3))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move1_sequence);

        // Second save at position 1
        let save2_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save2_sequence);

        // Move to position 2
        let move2_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(5)),
                col: term_col(nz(7))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move2_sequence);

        // Restore should go to most recently saved position (position 1)
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        assert_eq!(ofs_buf.cursor_pos, row(1) + col(2)); // 0-based position 1
    }

    #[test]
    fn test_cursor_save_restore_with_styling_operations() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Position cursor and set initial style
        let move_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(2)),
                col: term_col(nz(4))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);

        let initial_style = format!("{}", SgrCode::Bold);
        let _result = ofs_buf.apply_ansi_bytes(initial_style);

        // Save cursor
        let save_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save_sequence);

        // Move and do complex styling
        let move_and_style = format!(
            "{}{}{}{}Text",
            CsiSequence::CursorPosition {
                row: term_row(nz(5)),
                col: term_col(nz(1))
            },
            SgrCode::ForegroundBasic(ANSIBasicColor::Red),
            SgrCode::BackgroundBasic(ANSIBasicColor::Yellow),
            SgrCode::Underline
        );
        let _result = ofs_buf.apply_ansi_bytes(move_and_style);

        // Restore cursor (position only)
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        // Verify cursor position restored
        assert_eq!(ofs_buf.cursor_pos, row(1) + col(3)); // 0-based

        // Write text to verify current attributes are maintained
        let _result = ofs_buf.apply_ansi_bytes("Test");

        // Should be written with the complex styling (red on yellow, underlined)
        let char_at_restore_pos = &ofs_buf.buffer[1][3];
        if let crate::PixelChar::PlainText { style, .. } = char_at_restore_pos {
            assert_eq!(
                style.color_fg,
                Some(crate::TuiColor::Basic(ANSIBasicColor::Red))
            );
            assert_eq!(
                style.color_bg,
                Some(crate::TuiColor::Basic(ANSIBasicColor::Yellow))
            );
            assert!(style.attribs.underline.is_some());
        } else {
            panic!("Expected PlainText with styling");
        }
    }
}

/// Tests for character set state preservation across operations.
pub mod character_set_state_management {
    use super::*;

    #[test]
    fn test_character_set_persistence_across_cursor_operations() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Switch to DEC Graphics character set (ESC ( 0)
        let graphics_mode = b"\x1b(0";
        let _result = ofs_buf.apply_ansi_bytes(graphics_mode);

        // Verify DEC Graphics mode is active
        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::DECGraphics);

        // Perform cursor save/restore operations
        let cursor_ops = format!(
            "{}{}{}",
            CsiSequence::SaveCursor,
            CsiSequence::CursorPosition {
                row: term_row(nz(3)),
                col: term_col(nz(5))
            },
            CsiSequence::RestoreCursor
        );
        let _result = ofs_buf.apply_ansi_bytes(cursor_ops);

        // Character set should persist
        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::DECGraphics);

        // Print a character that gets translated in DEC Graphics mode
        let _result = ofs_buf.apply_ansi_bytes("q"); // Should become horizontal line

        // Verify the character was translated
        let char_at_cursor = &ofs_buf.buffer[0][0];
        if let crate::PixelChar::PlainText { display_char, .. } = char_at_cursor {
            assert_eq!(*display_char, '─'); // DEC Graphics 'q' → horizontal line
        } else {
            panic!("Expected translated DEC Graphics character");
        }
    }

    #[test]
    fn test_character_set_switching_with_mode_changes() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start in ASCII mode (default)
        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::Ascii);

        // Switch to DEC Graphics and disable auto-wrap
        let combined_sequence = format!(
            "{}{}",
            "\x1b(0", // DEC Graphics
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(combined_sequence);

        // Both states should be active
        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::DECGraphics);
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Change modes but keep character set
        let mode_change = format!(
            "{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap)
        );
        let _result = ofs_buf.apply_ansi_bytes(mode_change);

        // Auto-wrap should change but character set should persist
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);
        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::DECGraphics);

        // Switch back to ASCII
        let ascii_mode = b"\x1b(B";
        let _result = ofs_buf.apply_ansi_bytes(ascii_mode);

        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::Ascii);
    }
}

/// Tests for scroll region interactions with cursor and state management.
pub mod scroll_region_state_interactions {
    use super::*;

    #[test]
    fn test_cursor_save_restore_with_scroll_regions() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Set scroll region (rows 3-7)
        let margins_sequence = format!(
            "{}",
            CsiSequence::SetScrollingMargins {
                top: Some(term_row(nz(3))),
                bottom: Some(term_row(nz(7)))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(margins_sequence);

        // Position cursor within scroll region
        let move_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(5)),
                col: term_col(nz(6))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);

        // Save cursor
        let save_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save_sequence);

        // Move outside scroll region
        let move_outside = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(1)),
                col: term_col(nz(2))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_outside);

        // Change scroll region
        let new_margins = format!(
            "{}",
            CsiSequence::SetScrollingMargins {
                top: Some(term_row(nz(2))),
                bottom: Some(term_row(nz(8)))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(new_margins);

        // Restore cursor (should restore to absolute position)
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        // Should restore to original absolute position (5,6)
        assert_eq!(ofs_buf.cursor_pos, row(4) + col(5)); // 0-based

        // Verify scroll region changed
        assert_eq!(
            ofs_buf.ansi_parser_support.scroll_region_top,
            Some(term_row(nz(2)))
        );
        assert_eq!(
            ofs_buf.ansi_parser_support.scroll_region_bottom,
            Some(term_row(nz(8)))
        );
    }

    #[test]
    fn test_scroll_region_reset_with_saved_cursor() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Set initial scroll region and position cursor
        let setup_sequence = format!(
            "{}{}{}",
            CsiSequence::SetScrollingMargins {
                top: Some(term_row(nz(4))),
                bottom: Some(term_row(nz(8)))
            },
            CsiSequence::CursorPosition {
                row: term_row(nz(6)),
                col: term_col(nz(3))
            },
            CsiSequence::SaveCursor
        );
        let _result = ofs_buf.apply_ansi_bytes(setup_sequence);

        // Reset scroll region (ESC [ r with no parameters)
        let reset_margins = format!(
            "{}",
            CsiSequence::SetScrollingMargins {
                top: None,
                bottom: None
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(reset_margins);

        // Verify scroll region reset
        assert_eq!(ofs_buf.ansi_parser_support.scroll_region_top, None);
        assert_eq!(ofs_buf.ansi_parser_support.scroll_region_bottom, None);

        // Move cursor somewhere else
        let move_sequence = format!(
            "{}",
            CsiSequence::CursorPosition {
                row: term_row(nz(2)),
                col: term_col(nz(8))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(move_sequence);

        // Restore cursor (should still work with reset scroll region)
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        // Should restore to saved position (6,3)
        assert_eq!(ofs_buf.cursor_pos, row(5) + col(2)); // 0-based
    }
}

/// Tests for complex state combinations and edge cases.
pub mod complex_state_combinations {
    use super::*;

    #[test]
    fn test_full_state_combination() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Set up complex state: scroll region + DEC graphics + auto-wrap off + styling
        let complex_setup = format!(
            "{}{}{}{}{}{}",
            CsiSequence::SetScrollingMargins {
                top: Some(term_row(nz(2))),
                bottom: Some(term_row(nz(8)))
            },
            "\x1b(0", // DEC Graphics
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap),
            SgrCode::Bold,
            SgrCode::ForegroundBasic(ANSIBasicColor::Cyan),
            CsiSequence::CursorPosition {
                row: term_row(nz(5)),
                col: term_col(nz(4))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(complex_setup);

        // Save cursor
        let save_sequence = format!("{}", CsiSequence::SaveCursor);
        let _result = ofs_buf.apply_ansi_bytes(save_sequence);

        // Modify all states
        let state_changes = format!(
            "{}{}{}{}{}",
            CsiSequence::SetScrollingMargins {
                top: Some(term_row(nz(1))),
                bottom: Some(term_row(nz(10)))
            },
            "\x1b(B", // ASCII
            CsiSequence::EnablePrivateMode(PrivateModeType::AutoWrap),
            SgrCode::Reset,
            CsiSequence::CursorPosition {
                row: term_row(nz(9)),
                col: term_col(nz(1))
            }
        );
        let _result = ofs_buf.apply_ansi_bytes(state_changes);

        // Restore cursor (position only)
        let restore_sequence = format!("{}", CsiSequence::RestoreCursor);
        let _result = ofs_buf.apply_ansi_bytes(restore_sequence);

        // Verify cursor position restored
        assert_eq!(ofs_buf.cursor_pos, row(4) + col(3)); // 0-based

        // Verify other states changed (not restored)
        assert_eq!(
            ofs_buf.ansi_parser_support.scroll_region_top,
            Some(term_row(nz(1)))
        );
        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::Ascii);
        assert!(ofs_buf.ansi_parser_support.auto_wrap_mode);
    }

    #[test]
    fn test_state_persistence_across_buffer_operations() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Set up initial state
        let initial_state = format!(
            "{}{}{}",
            "\x1b(0", // DEC Graphics
            CsiSequence::DisablePrivateMode(PrivateModeType::AutoWrap),
            SgrCode::Bold
        );
        let _result = ofs_buf.apply_ansi_bytes(initial_state);

        // Perform various buffer operations
        let buffer_ops = format!(
            "{}{}{}{}{}",
            "Text1 ",
            CsiSequence::CursorPosition {
                row: term_row(nz(2)),
                col: term_col(nz(1))
            },
            "Text2 ",
            CsiSequence::EraseDisplay(0), // Clear from cursor to end
            "Text3"
        );
        let _result = ofs_buf.apply_ansi_bytes(buffer_ops);

        // States should persist through buffer operations
        assert_eq!(ofs_buf.ansi_parser_support.character_set,
                  crate::tui::terminal_lib_backends::offscreen_buffer::ofs_buf_core::CharacterSet::DECGraphics);
        assert!(!ofs_buf.ansi_parser_support.auto_wrap_mode);

        // Current style should still have bold
        assert!(
            ofs_buf
                .ansi_parser_support
                .current_style
                .attribs
                .bold
                .is_some()
        );
    }
}

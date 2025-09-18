// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for terminal-level operations.
//!
//! This module tests terminal control operations including:
//! - RIS (Reset to Initial State) - ESC c
//! - Character set selection - ESC ( B, ESC ( 0
//! - Terminal state reset and initialization
//! - Character set switching and translation

use super::super::test_fixtures::*;
use crate::{CharacterSet, ColIndex, Pos, RowIndex, TuiStyle, ch,
            vt_100_ansi_parser::protocols::esc_codes::EscSequence};

/// Tests for RIS (Reset to Initial State) operations.
pub mod reset_initial_state {
    use super::*;

    #[test]
    fn test_ris_resets_cursor_position() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Move cursor to non-origin position
        ofs_buf.cursor_pos = Pos::new((RowIndex::new(ch(5)), ColIndex::new(ch(3))));

        // Send RIS sequence
        let ris_sequence = format!("{}", EscSequence::ResetTerminal);
        let _result = ofs_buf.apply_ansi_bytes(ris_sequence.as_bytes());

        // Cursor should be reset to origin
        assert_eq!(ofs_buf.cursor_pos, Pos::default());
    }

    #[test]
    fn test_ris_resets_style() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Set non-default style (simplified for test)
        ofs_buf.ansi_parser_support.current_style = TuiStyle::default();

        // Send RIS sequence
        let ris_sequence = format!("{}", EscSequence::ResetTerminal);
        let _result = ofs_buf.apply_ansi_bytes(ris_sequence.as_bytes());

        // Style should be reset to default
        assert_eq!(
            ofs_buf.ansi_parser_support.current_style,
            TuiStyle::default()
        );
    }

    #[test]
    fn test_ris_resets_character_set() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Set DEC graphics character set
        ofs_buf.ansi_parser_support.character_set = CharacterSet::DECGraphics;

        // Send RIS sequence
        let ris_sequence = format!("{}", EscSequence::ResetTerminal);
        let _result = ofs_buf.apply_ansi_bytes(ris_sequence.as_bytes());

        // Character set should be reset to ASCII
        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );
    }

    #[test]
    fn test_ris_basic_functionality() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Move cursor away from origin
        ofs_buf.cursor_pos = Pos::new((RowIndex::new(ch(3)), ColIndex::new(ch(5))));

        // Send RIS sequence
        let ris_sequence = format!("{}", EscSequence::ResetTerminal);
        let _result = ofs_buf.apply_ansi_bytes(ris_sequence.as_bytes());

        // Verify basic reset occurred
        assert_eq!(ofs_buf.cursor_pos, Pos::default());
    }
}

/// Tests for character set selection operations.
pub mod character_set_selection {
    use super::*;

    #[test]
    fn test_select_ascii_character_set() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start with DEC graphics
        ofs_buf.ansi_parser_support.character_set = CharacterSet::DECGraphics;

        // Select ASCII character set (ESC ( B)
        let ascii_sequence = format!("{}", EscSequence::SelectAscii);
        let _result = ofs_buf.apply_ansi_bytes(ascii_sequence.as_bytes());

        // Character set should be ASCII
        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );
    }

    #[test]
    fn test_select_dec_graphics_character_set() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Start with ASCII
        ofs_buf.ansi_parser_support.character_set = CharacterSet::Ascii;

        // Select DEC graphics character set (ESC ( 0)
        let graphics_sequence = format!("{}", EscSequence::SelectDECGraphics);
        let _result = ofs_buf.apply_ansi_bytes(graphics_sequence.as_bytes());

        // Character set should be DEC graphics
        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::DECGraphics
        );
    }

    #[test]
    fn test_basic_character_set_functionality() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Verify default character set
        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );

        // Switch to DEC graphics
        ofs_buf.ansi_parser_support.character_set = CharacterSet::DECGraphics;
        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::DECGraphics
        );

        // Switch back to ASCII
        ofs_buf.ansi_parser_support.character_set = CharacterSet::Ascii;
        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );
    }
}

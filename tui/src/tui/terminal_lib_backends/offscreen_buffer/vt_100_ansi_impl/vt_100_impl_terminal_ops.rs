// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI character set selection operations for `OffscreenBuffer`.
//!
//! This module provides methods for selecting different character sets
//! as required by ANSI terminal emulation standards, particularly for
//! ESC ( B (ASCII) and ESC ( 0 (DEC Special Graphics) sequences.
//!
//! This module implements the business logic for terminal operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See the architecture documentation above
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//!

#[allow(clippy::wildcard_imports)]
use super::super::*;

impl OffscreenBuffer {
    /// Select ASCII character set for normal text rendering.
    ///
    /// Used by ESC ( B sequence to switch to normal ASCII character set.
    /// This is the default character set for most text operations.
    ///
    /// # Example
    ///
    /// ```text
    /// Before ESC ( B: character_set = DECGraphics
    /// After ESC ( B:  character_set = Ascii
    /// ```
    pub fn select_ascii_character_set(&mut self) {
        self.ansi_parser_support.character_set = CharacterSet::Ascii;
    }

    /// Select DEC Special Graphics character set for box-drawing characters.
    ///
    /// Used by ESC ( 0 sequence to switch to DEC Special Graphics character set.
    /// This enables rendering of box-drawing and line-drawing characters commonly
    /// used for terminal-based user interfaces.
    ///
    /// # Example
    ///
    /// ```text
    /// Before ESC ( 0: character_set = Ascii
    /// After ESC ( 0:  character_set = DECGraphics
    /// ```
    pub fn select_dec_graphics_character_set(&mut self) {
        self.ansi_parser_support.character_set = CharacterSet::DECGraphics;
    }

    /// Translate DEC Special Graphics characters to Unicode box-drawing characters.
    /// Used when `character_set` is [`DECGraphics`] (after ESC ( 0).
    ///
    /// [`DECGraphics`]: crate::CharacterSet::DECGraphics
    #[must_use]
    pub fn translate_dec_graphics(c: char) -> char {
        match c {
            'j' => '┘', // Lower right corner.
            'k' => '┐', // Upper right corner.
            'l' => '┌', // Upper left corner.
            'm' => '└', // Lower left corner.
            'n' => '┼', // Crossing lines.
            'q' => '─', // Horizontal line.
            't' => '├', // Left "T".
            'u' => '┤', // Right "T".
            'v' => '┴', // Bottom "T".
            'w' => '┬', // Top "T".
            'x' => '│', // Vertical line.
            _ => c,     // Pass through unmapped characters.
        }
    }
}

#[cfg(test)]
mod tests_char_set_ops {
    use super::*;
    use crate::{height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_select_ascii_character_set() {
        let mut buffer = create_test_buffer();

        // Start with DEC graphics character set.
        buffer.ansi_parser_support.character_set = CharacterSet::DECGraphics;

        buffer.select_ascii_character_set();

        assert_eq!(
            buffer.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );
    }

    #[test]
    fn test_select_dec_graphics_character_set() {
        let mut buffer = create_test_buffer();

        // Start with ASCII character set (default).
        buffer.ansi_parser_support.character_set = CharacterSet::Ascii;

        buffer.select_dec_graphics_character_set();

        assert_eq!(
            buffer.ansi_parser_support.character_set,
            CharacterSet::DECGraphics
        );
    }

    #[test]
    fn test_translate_dec_graphics_corners() {
        // Test corner characters.
        assert_eq!(OffscreenBuffer::translate_dec_graphics('j'), '┘'); // Lower right
        assert_eq!(OffscreenBuffer::translate_dec_graphics('k'), '┐'); // Upper right
        assert_eq!(OffscreenBuffer::translate_dec_graphics('l'), '┌'); // Upper left
        assert_eq!(OffscreenBuffer::translate_dec_graphics('m'), '└'); // Lower left
    }

    #[test]
    fn test_translate_dec_graphics_lines() {
        // Test line characters.
        assert_eq!(OffscreenBuffer::translate_dec_graphics('q'), '─'); // Horizontal line
        assert_eq!(OffscreenBuffer::translate_dec_graphics('x'), '│'); // Vertical line
        assert_eq!(OffscreenBuffer::translate_dec_graphics('n'), '┼'); // Crossing lines
    }

    #[test]
    fn test_translate_dec_graphics_tees() {
        // Test T-junction characters.
        assert_eq!(OffscreenBuffer::translate_dec_graphics('t'), '├'); // Left "T"
        assert_eq!(OffscreenBuffer::translate_dec_graphics('u'), '┤'); // Right "T"
        assert_eq!(OffscreenBuffer::translate_dec_graphics('v'), '┴'); // Bottom "T"
        assert_eq!(OffscreenBuffer::translate_dec_graphics('w'), '┬'); // Top "T"
    }

    #[test]
    fn test_translate_dec_graphics_unmapped_characters() {
        // Test that unmapped characters pass through unchanged.
        assert_eq!(OffscreenBuffer::translate_dec_graphics('a'), 'a');
        assert_eq!(OffscreenBuffer::translate_dec_graphics('Z'), 'Z');
        assert_eq!(OffscreenBuffer::translate_dec_graphics('1'), '1');
        assert_eq!(OffscreenBuffer::translate_dec_graphics('@'), '@');
        assert_eq!(OffscreenBuffer::translate_dec_graphics(' '), ' ');
    }

    #[test]
    fn test_translate_dec_graphics_complete_mapping() {
        // Test all mapped characters at once to ensure completeness.
        let mappings = [
            ('j', '┘'),
            ('k', '┐'),
            ('l', '┌'),
            ('m', '└'),
            ('n', '┼'),
            ('q', '─'),
            ('t', '├'),
            ('u', '┤'),
            ('v', '┴'),
            ('w', '┬'),
            ('x', '│'),
        ];

        for (input, expected) in mappings {
            assert_eq!(
                OffscreenBuffer::translate_dec_graphics(input),
                expected,
                "Translation failed for character '{input}'"
            );
        }
    }

    #[test]
    fn test_character_set_state_persistence() {
        let mut buffer = create_test_buffer();

        // Verify initial state is ASCII (default).
        assert_eq!(
            buffer.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );

        // Switch to DEC graphics and verify persistence.
        buffer.select_dec_graphics_character_set();
        assert_eq!(
            buffer.ansi_parser_support.character_set,
            CharacterSet::DECGraphics
        );

        // Switch back to ASCII and verify persistence.
        buffer.select_ascii_character_set();
        assert_eq!(
            buffer.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );
    }

    #[test]
    fn test_character_set_toggle_behavior() {
        let mut buffer = create_test_buffer();

        // Test multiple toggles between character sets
        for _ in 0..3 {
            buffer.select_dec_graphics_character_set();
            assert_eq!(
                buffer.ansi_parser_support.character_set,
                CharacterSet::DECGraphics
            );

            buffer.select_ascii_character_set();
            assert_eq!(
                buffer.ansi_parser_support.character_set,
                CharacterSet::Ascii
            );
        }
    }

    #[test]
    fn test_dec_graphics_box_drawing_pattern() {
        // Test a common box-drawing pattern using DEC graphics
        let box_chars = ['l', 'q', 'k', 'x', 'x', 'm', 'q', 'j'];
        let expected_unicode = ['┌', '─', '┐', '│', '│', '└', '─', '┘'];

        for (dec_char, expected_unicode_char) in
            box_chars.iter().zip(expected_unicode.iter())
        {
            assert_eq!(
                OffscreenBuffer::translate_dec_graphics(*dec_char),
                *expected_unicode_char,
                "Failed to translate box drawing character '{dec_char}'"
            );
        }
    }
}

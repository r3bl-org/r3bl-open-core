// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI character set selection operations for `OffscreenBuffer`.
//!
//! This module provides methods for selecting different character sets
//! as required by ANSI terminal emulation standards, particularly for
//! ESC ( B (ASCII) and ESC ( 0 (DEC Special Graphics) sequences.

#[allow(clippy::wildcard_imports)]
use super::*;

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
    /// Used when `character_set` is `DECGraphics` (after ESC ( 0).
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

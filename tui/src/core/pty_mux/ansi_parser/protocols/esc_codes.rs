// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Direct ESC (Escape) codes for terminal control.
//!
//! ESC sequences are simple, non-parameterized terminal control codes that predate
//! the more advanced CSI sequences. They provide fundamental terminal operations
//! without the flexibility of parameters.
//!
//! ## Relationship to CSI Sequences
//!
//! ESC sequences are the predecessors to the more modern CSI sequences:
//!
//! - **ESC sequences** (this module): Original, simple commands used in early terminals
//!   like the VT100. Each does one specific thing: `ESC 7` saves cursor, `ESC 8` restores
//!   it.
//! - **CSI sequences** (the successors): Modern, parameterized commands that evolved from
//!   ESC to provide greater flexibility. See [`csi_codes`] for the modern equivalents.
//!
//! Both approaches coexist for backward compatibility. For example:
//! - `ESC 7` / `ESC 8` (this module) vs `ESC[s` / `ESC[u` (CSI equivalent)
//! - `ESC D` (move down 1 line) vs `ESC[1B` or `ESC[5B` (move down N lines)
//!
//! [`csi_codes`]: crate::ansi_parser::csi_codes
//!
//! ## Structure
//! ESC sequences follow simpler patterns than CSI:
//! - Single character: `ESC character` (e.g., `ESC c` for reset)
//! - With intermediate: `ESC intermediate final` (e.g., `ESC ( B` for charset selection)
//!
//! ## Common Uses
//! - **Cursor Save/Restore**: Save and restore cursor position without parameters
//! - **Character Sets**: Switch between ASCII and special graphics character sets
//! - **Line Operations**: Move cursor with automatic scrolling at boundaries
//! - **Terminal Reset**: Full terminal initialization
//!
//! ## Examples
//! - `ESC 7` - Save cursor position and attributes
//! - `ESC 8` - Restore saved cursor position
//! - `ESC ( 0` - Switch to line-drawing character set
//! - `ESC c` - Reset terminal to initial state

use std::fmt;

use crate::{BufTextStorage, WriteToBuf};

// Cursor Save/Restore Operations

/// ESC 7 (DECSC): Save cursor position and attributes
/// Saves the current cursor position and SGR attributes
pub const DECSC_SAVE_CURSOR: u8 = b'7';

/// ESC 8 (DECRC): Restore cursor position and attributes
/// Restores the previously saved cursor position and SGR attributes
pub const DECRC_RESTORE_CURSOR: u8 = b'8';

// Scrolling Operations.

/// ESC D (IND): Index - move cursor down one line
/// If at bottom of scroll region, scrolls the screen up
pub const IND_INDEX_DOWN: u8 = b'D';

/// ESC M (RI): Reverse Index - move cursor up one line
/// If at top of scroll region, scrolls the screen down
pub const RI_REVERSE_INDEX_UP: u8 = b'M';

// Terminal Control.

/// ESC c (RIS): Reset to Initial State
/// Performs a full terminal reset, clearing the screen and resetting all modes
pub const RIS_RESET_TERMINAL: u8 = b'c';

// Character Set Selection Intermediates.

/// ESC ( - G0 character set designation intermediate
/// Used before the final character to select character sets for G0
pub const G0_CHARSET_INTERMEDIATE: &[u8] = b"(";

/// ESC ) - G1 character set designation intermediate
/// Used before the final character to select character sets for G1
pub const G1_CHARSET_INTERMEDIATE: &[u8] = b")";

// Character Set Selection Final Bytes (used after intermediates)

/// Select ASCII character set (normal text mode)
/// Used as: ESC ( B
pub const CHARSET_ASCII: u8 = b'B';

/// Select DEC Special Graphics character set (line drawing)
/// Used as: ESC ( 0
/// Maps ASCII characters to box-drawing Unicode characters
pub const CHARSET_DEC_GRAPHICS: u8 = b'0';

// Other Character Sets (for future extension)

/// Select United Kingdom (UK) character set
/// Used as: ESC ( A
pub const CHARSET_UK: u8 = b'A';

/// Select DEC Supplemental Graphics character set
/// Used as: ESC ( <
pub const CHARSET_DEC_SUPPLEMENTAL: u8 = b'<';

// Miscellaneous ESC Sequences.

/// ESC E (NEL): Next Line
/// Moves cursor to beginning of next line
pub const NEL_NEXT_LINE: u8 = b'E';

/// ESC H (HTS): Horizontal Tab Set
/// Sets a tab stop at the current cursor position
pub const HTS_TAB_SET: u8 = b'H';

/// ESC Z (DECID): Identify Terminal
/// Requests terminal identification
pub const DECID_IDENTIFY: u8 = b'Z';

/// ESC = : Application Keypad Mode (DECKPAM)
/// Enables application keypad mode
pub const DECKPAM_APP_KEYPAD: u8 = b'=';

/// ESC > : Normal Keypad Mode (DECKPNM)
/// Disables application keypad mode
pub const DECKPNM_NORMAL_KEYPAD: u8 = b'>';

// C0 Control Characters (handled by execute() method)
// These are not ESC sequences but basic control characters.

/// Backspace control character (BS)
/// Moves cursor one position to the left
pub const BACKSPACE: u8 = 0x08;

/// Horizontal Tab control character (HT)
/// Moves cursor to next tab stop
pub const TAB: u8 = b'\t';

/// Line Feed control character (LF)
/// Moves cursor to next line
pub const LINE_FEED: u8 = b'\n';

/// Carriage Return control character (CR)
/// Moves cursor to beginning of current line
pub const CARRIAGE_RETURN: u8 = b'\r';

// ESC sequence builder following the same pattern as SgrCode.

/// Builder for ESC (direct escape) sequences.
/// Similar to `SgrCode` but for direct escape sequences.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EscSequence {
    /// ESC 7 - Save cursor position (DECSC)
    SaveCursor,
    /// ESC 8 - Restore cursor position (DECRC)
    RestoreCursor,
    /// ESC D - Index down (IND)
    IndexDown,
    /// ESC M - Reverse index up (RI)
    ReverseIndex,
    /// ESC c - Reset terminal (RIS)
    ResetTerminal,
    /// ESC ( B - Select ASCII character set
    SelectAscii,
    /// ESC ( 0 - Select DEC graphics character set
    SelectDECGraphics,
}

impl fmt::Display for EscSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut acc = BufTextStorage::new();
        self.write_to_buf(&mut acc)?;
        self.write_buf_to_fmt(&acc, f)
    }
}

impl WriteToBuf for EscSequence {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
        acc.push('\x1b');
        match self {
            EscSequence::SaveCursor => acc.push(DECSC_SAVE_CURSOR as char),
            EscSequence::RestoreCursor => acc.push(DECRC_RESTORE_CURSOR as char),
            EscSequence::IndexDown => acc.push(IND_INDEX_DOWN as char),
            EscSequence::ReverseIndex => acc.push(RI_REVERSE_INDEX_UP as char),
            EscSequence::ResetTerminal => acc.push(RIS_RESET_TERMINAL as char),
            EscSequence::SelectAscii => {
                acc.push('(');
                acc.push(CHARSET_ASCII as char);
            }
            EscSequence::SelectDECGraphics => {
                acc.push('(');
                acc.push(CHARSET_DEC_GRAPHICS as char);
            }
        }
        Ok(())
    }

    fn write_buf_to_fmt(
        &self,
        acc: &BufTextStorage,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&acc.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esc_sequence_save_cursor() {
        let sequence = EscSequence::SaveCursor;
        let result = sequence.to_string();
        let expected = "\x1b7";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_esc_sequence_restore_cursor() {
        let sequence = EscSequence::RestoreCursor;
        let result = sequence.to_string();
        let expected = "\x1b8";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_esc_sequence_index_down() {
        let sequence = EscSequence::IndexDown;
        let result = sequence.to_string();
        let expected = "\x1bD";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_esc_sequence_reverse_index() {
        let sequence = EscSequence::ReverseIndex;
        let result = sequence.to_string();
        let expected = "\x1bM";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_esc_sequence_reset_terminal() {
        let sequence = EscSequence::ResetTerminal;
        let result = sequence.to_string();
        let expected = "\x1bc";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_esc_sequence_select_ascii() {
        let sequence = EscSequence::SelectAscii;
        let result = sequence.to_string();
        let expected = "\x1b(B";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_esc_sequence_select_dec_graphics() {
        let sequence = EscSequence::SelectDECGraphics;
        let result = sequence.to_string();
        let expected = "\x1b(0";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_esc_sequence_write_to_buf_save_cursor() {
        let sequence = EscSequence::SaveCursor;
        let mut buffer = BufTextStorage::new();
        let result = sequence.write_to_buf(&mut buffer);

        assert!(result.is_ok());
        assert_eq!(buffer, "\x1b7");
    }

    #[test]
    fn test_esc_sequence_write_to_buf_select_ascii() {
        let sequence = EscSequence::SelectAscii;
        let mut buffer = BufTextStorage::new();
        let result = sequence.write_to_buf(&mut buffer);

        assert!(result.is_ok());
        assert_eq!(buffer, "\x1b(B");
    }

    #[test]
    fn test_esc_sequence_write_to_buf_select_dec_graphics() {
        let sequence = EscSequence::SelectDECGraphics;
        let mut buffer = BufTextStorage::new();
        let result = sequence.write_to_buf(&mut buffer);

        assert!(result.is_ok());
        assert_eq!(buffer, "\x1b(0");
    }

    #[test]
    fn test_esc_sequence_debug() {
        let sequence = EscSequence::SaveCursor;
        let debug_output = format!("{sequence:?}");
        assert!(debug_output.contains("SaveCursor"));
    }

    #[test]
    fn test_esc_sequence_equality() {
        let seq1 = EscSequence::SaveCursor;
        let seq2 = EscSequence::SaveCursor;
        let seq3 = EscSequence::RestoreCursor;

        assert_eq!(seq1, seq2);
        assert_ne!(seq1, seq3);
    }

    #[test]
    fn test_all_escape_sequences_generate_unique_outputs() {
        let sequences = [
            EscSequence::SaveCursor,
            EscSequence::RestoreCursor,
            EscSequence::IndexDown,
            EscSequence::ReverseIndex,
            EscSequence::ResetTerminal,
            EscSequence::SelectAscii,
            EscSequence::SelectDECGraphics,
        ];

        let mut outputs = std::collections::HashSet::new();

        for sequence in &sequences {
            let output = sequence.to_string();
            // Each sequence should produce a unique output.
            assert!(
                outputs.insert(output.clone()),
                "Duplicate output found: {output}"
            );
        }

        // Should have 7 unique outputs.
        assert_eq!(outputs.len(), 7);
    }

    #[test]
    fn test_escape_sequences_start_with_escape() {
        let sequences = [
            EscSequence::SaveCursor,
            EscSequence::RestoreCursor,
            EscSequence::IndexDown,
            EscSequence::ReverseIndex,
            EscSequence::ResetTerminal,
            EscSequence::SelectAscii,
            EscSequence::SelectDECGraphics,
        ];

        for sequence in &sequences {
            let output = sequence.to_string();
            assert!(
                output.starts_with('\x1b'),
                "Sequence {sequence:?} should start with ESC character, got: {output:?}"
            );
        }
    }

    #[test]
    fn test_character_set_sequences_have_correct_format() {
        let ascii_seq = EscSequence::SelectAscii;
        let dec_graphics_seq = EscSequence::SelectDECGraphics;

        let ascii_output = ascii_seq.to_string();
        let dec_graphics_output = dec_graphics_seq.to_string();

        // Both should be ESC + ( + character.
        assert_eq!(ascii_output.len(), 3);
        assert_eq!(dec_graphics_output.len(), 3);

        assert_eq!(ascii_output.chars().nth(1), Some('('));
        assert_eq!(dec_graphics_output.chars().nth(1), Some('('));

        assert_eq!(ascii_output.chars().nth(2), Some('B'));
        assert_eq!(dec_graphics_output.chars().nth(2), Some('0'));
    }
}

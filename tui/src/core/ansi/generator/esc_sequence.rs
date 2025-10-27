// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Direct ESC (Escape) sequence builder.
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
//!   ESC to provide greater flexibility.
//!
//! Both approaches coexist for backward compatibility. For example:
//! - `ESC 7` / `ESC 8` (this module) vs `ESC[s` / `ESC[u` (CSI equivalent)
//! - `ESC D` (move down 1 line) vs `ESC[1B` or `ESC[5B` (move down N lines)
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

use crate::{BufTextStorage, FastStringify,
            core::ansi::constants::{CHARSET_ASCII, CHARSET_DEC_GRAPHICS,
                                    CHARSET_SELECTOR_G0, DECRC_RESTORE_CURSOR,
                                    DECSC_SAVE_CURSOR, ESC_START, IND_INDEX_DOWN,
                                    RI_REVERSE_INDEX_UP, RIS_RESET_TERMINAL},
            generate_impl_display_for_fast_stringify};
use std::fmt;

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

impl FastStringify for EscSequence {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
        acc.push(ESC_START);
        match self {
            EscSequence::SaveCursor => acc.push(DECSC_SAVE_CURSOR as char),
            EscSequence::RestoreCursor => acc.push(DECRC_RESTORE_CURSOR as char),
            EscSequence::IndexDown => acc.push(IND_INDEX_DOWN as char),
            EscSequence::ReverseIndex => acc.push(RI_REVERSE_INDEX_UP as char),
            EscSequence::ResetTerminal => acc.push(RIS_RESET_TERMINAL as char),
            EscSequence::SelectAscii => {
                acc.push(CHARSET_SELECTOR_G0);
                acc.push(CHARSET_ASCII as char);
            }
            EscSequence::SelectDECGraphics => {
                acc.push(CHARSET_SELECTOR_G0);
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

generate_impl_display_for_fast_stringify!(EscSequence);

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

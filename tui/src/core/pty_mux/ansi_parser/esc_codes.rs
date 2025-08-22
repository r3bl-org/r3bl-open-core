// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Direct ESC (Escape) codes for terminal control.
//!
//! ESC sequences are simple, non-parameterized terminal control codes that predate
//! the more advanced CSI sequences. They provide fundamental terminal operations
//! without the flexibility of parameters.
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

// Cursor Save/Restore Operations

/// ESC 7 (DECSC): Save cursor position and attributes
/// Saves the current cursor position and SGR attributes
pub const DECSC_SAVE_CURSOR: u8 = b'7';

/// ESC 8 (DECRC): Restore cursor position and attributes
/// Restores the previously saved cursor position and SGR attributes
pub const DECRC_RESTORE_CURSOR: u8 = b'8';

// Scrolling Operations

/// ESC D (IND): Index - move cursor down one line
/// If at bottom of scroll region, scrolls the screen up
pub const IND_INDEX_DOWN: u8 = b'D';

/// ESC M (RI): Reverse Index - move cursor up one line
/// If at top of scroll region, scrolls the screen down
pub const RI_REVERSE_INDEX_UP: u8 = b'M';

// Terminal Control

/// ESC c (RIS): Reset to Initial State
/// Performs a full terminal reset, clearing the screen and resetting all modes
pub const RIS_RESET_TERMINAL: u8 = b'c';

// Character Set Selection Intermediates

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

// Miscellaneous ESC Sequences

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

// ESC sequence builder following the same pattern as SgrCode

use std::fmt;
use crate::{BufTextStorage, WriteToBuf};

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
    SelectGraphics,
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
            EscSequence::SelectGraphics => {
                acc.push('(');
                acc.push(CHARSET_DEC_GRAPHICS as char);
            }
        }
        Ok(())
    }
    
    fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&acc.clone())
    }
}
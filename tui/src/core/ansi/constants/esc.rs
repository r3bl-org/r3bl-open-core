// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Direct ESC (Escape) sequence constants for terminal control.
//!
//! ESC sequences are simple, non-parameterized terminal control codes that predate
//! the more advanced CSI sequences. They provide fundamental terminal operations
//! without the flexibility of parameters.

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

// ESC sequence start and selection characters.

/// ESC sequence start: ESC (0x1B)
pub const ESC_START: char = '\x1b';

/// G0 character set selector intermediate
pub const CHARSET_SELECTOR_G0: char = '(';

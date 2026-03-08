// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Direct [`ESC`] (Escape) sequence constants for terminal control.
//!
//! These are simple, non-parameterized terminal control codes that predate the more
//! advanced [`CSI`] sequences. They provide fundamental terminal operations without the
//! flexibility of parameters. They are used by [`EscSequence`] for building [`ESC`]
//! output sequences.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! [`CSI`]: crate::CsiSequence
//! [`ESC`]: crate::EscSequence
//! [`EscSequence`]: crate::EscSequence
//! [constants module design]: mod@crate::constants#design

use crate::define_ansi_const;

// Cursor Save/Restore Operations

/// Save Cursor (DECSC): Saves the current cursor position and attributes.
///
/// Value: `55` dec, `37` hex.
///
/// Sequence: `ESC 7`.
///
/// [`ESC`]: crate::EscSequence
/// [VT-100]: https://vt100.net/docs/vt100-ug/chapter3.html
pub const DECSC_SAVE_CURSOR: u8 = b'7';

/// Restore Cursor (DECRC): Restores the previously saved cursor position and attributes.
///
/// Value: `56` dec, `38` hex.
///
/// Sequence: `ESC 8`.
///
/// [`ESC`]: crate::EscSequence
/// [VT-100]: https://vt100.net/docs/vt100-ug/chapter3.html
pub const DECRC_RESTORE_CURSOR: u8 = b'8';

// Scrolling Operations.

/// Index (IND): Move cursor down one line.
/// If at bottom of scroll region, scrolls the screen up.
///
/// Value: `68` dec, `44` hex.
///
/// Sequence: `ESC D`.
///
/// [`ESC`]: crate::EscSequence
/// [VT-100]: https://vt100.net/docs/vt100-ug/chapter3.html
pub const IND_INDEX_DOWN: u8 = b'D';

/// Reverse Index (RI): Move cursor up one line.
/// If at top of scroll region, scrolls the screen down.
///
/// Value: `77` dec, `4D` hex.
///
/// Sequence: `ESC M`.
///
/// [`ESC`]: crate::EscSequence
/// [VT-100]: https://vt100.net/docs/vt100-ug/chapter3.html
pub const RI_REVERSE_INDEX_UP: u8 = b'M';

// Terminal Control.

/// Reset to Initial State (RIS): Performs a full terminal reset.
/// Clears the screen and resetting all modes.
///
/// Value: `99` dec, `63` hex.
///
/// Sequence: `ESC c`.
///
/// [`ESC`]: crate::EscSequence
/// [VT-100]: https://vt100.net/docs/vt100-ug/chapter3.html
pub const RIS_RESET_TERMINAL: u8 = b'c';

// Character Set Selection Intermediates.

/// G0 character set designation intermediate.
/// Used before the final character to select character sets for G0.
///
/// Value: `(` dec, `28` hex.
///
/// Sequence part: `ESC (`.
///
/// [`ESC`]: crate::EscSequence
pub const G0_CHARSET_INTERMEDIATE: &[u8] = b"(";

/// G1 character set designation intermediate.
/// Used before the final character to select character sets for G1.
///
/// Value: `)` dec, `29` hex.
///
/// Sequence part: `ESC )`.
///
/// [`ESC`]: crate::EscSequence
pub const G1_CHARSET_INTERMEDIATE: &[u8] = b")";

// Character Set Selection Final Bytes (used after intermediates)

/// Select [`ASCII`] character set (normal text mode).
///
/// Value: `66` dec, `42` hex.
///
/// Used as: `ESC ( B`.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`ESC`]: crate::EscSequence
pub const CHARSET_ASCII: u8 = b'B';

/// Select [`DEC`] Special Graphics character set (line drawing).
///
/// Value: `48` dec, `30` hex.
///
/// Used as: `ESC ( 0`. Maps [`ASCII`] characters to box-drawing Unicode characters.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub const CHARSET_DEC_GRAPHICS: u8 = b'0';

// Other Character Sets (for future extension)

/// Select United Kingdom (UK) character set.
///
/// Value: `65` dec, `41` hex.
///
/// Used as: `ESC ( A`.
///
/// [`ESC`]: crate::EscSequence
pub const CHARSET_UK: u8 = b'A';

/// Select [`DEC`] Supplemental Graphics character set.
///
/// Value: `60` dec, `3C` hex.
///
/// Used as: `ESC ( <`.
///
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub const CHARSET_DEC_SUPPLEMENTAL: u8 = b'<';

// Miscellaneous ESC Sequences.

/// Next Line (NEL): Moves cursor to beginning of next line.
///
/// Value: `69` dec, `45` hex.
///
/// Sequence: `ESC E`.
///
/// [`ESC`]: crate::EscSequence
pub const NEL_NEXT_LINE: u8 = b'E';

/// Horizontal Tab Set (HTS): Sets a tab stop at the current cursor position.
///
/// Value: `72` dec, `48` hex.
///
/// Sequence: `ESC H`.
///
/// [`ESC`]: crate::EscSequence
pub const HTS_TAB_SET: u8 = b'H';

/// Identify Terminal (DECID): Requests terminal identification.
///
/// Value: `90` dec, `5A` hex.
///
/// Sequence: `ESC Z`.
///
/// [`ESC`]: crate::EscSequence
pub const DECID_IDENTIFY: u8 = b'Z';

/// Application Keypad Mode (DECKPAM): Enables application keypad mode.
///
/// Value: `61` dec, `3D` hex.
///
/// Sequence: `ESC =`.
///
/// [`ESC`]: crate::EscSequence
pub const DECKPAM_APP_KEYPAD: u8 = b'=';

/// Normal Keypad Mode (DECKPNM): Disables application keypad mode.
///
/// Value: `62` dec, `3E` hex.
///
/// Sequence: `ESC >`.
///
/// [`ESC`]: crate::EscSequence
pub const DECKPNM_NORMAL_KEYPAD: u8 = b'>';

// C0 Control Characters (handled by execute() method)
// These are not ESC sequences but basic control characters.

/// Backspace control character (BS).
///
/// Value: `8` dec, `08` hex.
///
/// Moves cursor one position to the left.
pub const BACKSPACE: u8 = 8;

/// Horizontal Tab control character (HT).
///
/// Value: `9` dec, `09` hex.
///
/// Moves cursor to next tab stop.
pub const TAB: u8 = b'\t';

/// Line Feed control character (LF).
///
/// Value: `10` dec, `0A` hex.
///
/// Moves cursor to next line.
pub const LINE_FEED: u8 = b'\n';

/// Carriage Return control character (CR).
///
/// Value: `13` dec, `0D` hex.
///
/// Moves cursor to beginning of current line.
pub const CARRIAGE_RETURN: u8 = b'\r';

// ESC sequence start and selection characters.

/// Start byte: the escape character ([`ESC`]).
///
/// Value: `27` dec, `1B` hex.
///
/// [`ESC`]: crate::ANSI_ESC
pub const ESC_START: char = '\x1b';

define_ansi_const!(@esc_str : ESC_STR = [""] =>
    "Escape Start" : "Start string: the escape character (`27` dec, `1B` hex)."
);

define_ansi_const!(@esc_str : ESC_SAVE_CURSOR_STR = ["7"] =>
    "Save Cursor (DECSC)" : "Full string sequence."
);

define_ansi_const!(@esc_str : ESC_RESTORE_CURSOR_STR = ["8"] =>
    "Restore Cursor (DECRC)" : "Full string sequence."
);

define_ansi_const!(@esc_str : ESC_INDEX_DOWN_STR = ["D"] =>
    "Index (IND)" : "Full string sequence."
);

define_ansi_const!(@esc_str : ESC_REVERSE_INDEX_STR = ["M"] =>
    "Reverse Index (RI)" : "Full string sequence."
);

define_ansi_const!(@esc_str : ESC_RESET_TERMINAL_STR = ["c"] =>
    "Reset Terminal (RIS)" : "Full string sequence."
);

define_ansi_const!(@esc_str : ESC_SELECT_ASCII_STR = ["(B"] =>
    "Select ASCII" : "Full string sequence."
);

define_ansi_const!(@esc_str : ESC_SELECT_DEC_GRAPHICS_STR = ["(0"] =>
    "Select DEC Graphics" : "Full string sequence."
);

/// G0 character set selector intermediate.
///
/// Value: `'('` dec, `28` hex.
pub const CHARSET_SELECTOR_G0: char = '(';

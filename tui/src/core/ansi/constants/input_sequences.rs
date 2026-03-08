// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`]/[`VT-100`] keyboard input sequence constants.
//!
//! This module contains constant values for keyboard input sequences (arrow keys,
//! function keys, modifiers, control characters). Mouse constants are in the [`mouse`]
//! module and re-exported here.
//!
//! For [`VT-100`] keyboard encoding history and design decisions. See the [keyboard]
//! module documentation for more details.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`mouse`]: crate::constants::mouse
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [constants module design]: mod@crate::constants#design
//! [keyboard]:
//!     mod@crate::vt_100_terminal_input_parser::keyboard#keyboard-encoding-explained

// ==================== ANSI Sequence Components ====================

/// Escape ([`ESC`]): Start byte for all [`ANSI`] escape sequences.
///
/// Value: `27` dec, `1B` hex.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ESC`]: crate::ANSI_ESC
pub const ANSI_ESC: u8 = 27;

/// Control Sequence Introducer ([`CSI`]): Bracket byte `[`.
///
/// Sequence: `ESC [` (second byte of [`CSI`] prefix).
///
/// Value: `91` dec, `5B` hex.
///
/// [`CSI`]: crate::CsiSequence
pub const ANSI_CSI_BRACKET: u8 = b'[';

/// Single Shift 3 (SS3): `O` byte for `ESC O` sequences.
///
/// Sequence: `ESC O` (introduces SS3 function key and numpad sequences).
///
/// Value: `79` dec, `4F` hex.
pub const ANSI_SS3_O: u8 = b'O';

/// Parameter Separator: Semicolon `;` delimiter between [`CSI`] parameters.
///
/// Sequence: `CSI param1 ; param2 ...` (separates numeric parameters).
///
/// Value: `59` dec, `3B` hex.
///
/// [`CSI`]: crate::CsiSequence
pub const ANSI_PARAM_SEPARATOR: u8 = b';';

/// Function Key Terminator: Tilde `~` that ends function key and special key sequences.
///
/// Sequence: `CSI n ~` (terminates function key codes like `CSI 11~` for F1).
///
/// Value: `126` dec, `7E` hex.
pub const ANSI_FUNCTION_KEY_TERMINATOR: u8 = b'~';

// ==================== Arrow Keys (CSI A/B/C/D) ====================

/// Cursor Up (CUU): Up arrow final byte.
///
/// Sequence: `CSI A`
///
/// Value: `65` dec, `41` hex.
pub const ARROW_UP_FINAL: u8 = b'A';

/// Cursor Down (CUD): Down arrow final byte.
///
/// Sequence: `CSI B`
///
/// Value: `66` dec, `42` hex.
pub const ARROW_DOWN_FINAL: u8 = b'B';

/// Cursor Forward (CUF): Right arrow final byte.
///
/// Sequence: `CSI C`
///
/// Value: `67` dec, `43` hex.
pub const ARROW_RIGHT_FINAL: u8 = b'C';

/// Cursor Back (CUB): Left arrow final byte.
///
/// Sequence: `CSI D`
///
/// Value: `68` dec, `44` hex.
pub const ARROW_LEFT_FINAL: u8 = b'D';

// ==================== Tab Keys ====================

/// Cursor Backward Tabulation (CBT): `BackTab` / `Shift+Tab` final byte.
///
/// Sequence: `CSI Z`
///
/// Value: `90` dec, `5A` hex.
pub const BACKTAB_FINAL: u8 = b'Z';

// ==================== Special Keys (CSI H/F) ====================

/// Cursor Position (Home): Home key final byte.
///
/// Sequence: `CSI H`
///
/// Value: `72` dec, `48` hex.
pub const SPECIAL_HOME_FINAL: u8 = b'H';

/// Cursor Preceding Line (End): End key final byte.
///
/// Sequence: `CSI F`
///
/// Value: `70` dec, `46` hex.
pub const SPECIAL_END_FINAL: u8 = b'F';

// ==================== Special Keys (CSI n~) ====================

/// Insert Key ([`ANSI`]): Parameter code for the Insert key.
///
/// Value: `2`.
///
/// Sequence: `CSI 2~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_INSERT_CODE: u16 = 2;

/// Delete Key ([`ANSI`]): Parameter code for the Delete key.
///
/// Value: `3`.
///
/// Sequence: `CSI 3~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_DELETE_CODE: u16 = 3;

/// Page Up Key ([`ANSI`]): Parameter code for the Page Up key.
///
/// Value: `5`.
///
/// Sequence: `CSI 5~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_PAGE_UP_CODE: u16 = 5;

/// Page Down Key ([`ANSI`]): Parameter code for the Page Down key.
///
/// Value: `6`.
///
/// Sequence: `CSI 6~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_PAGE_DOWN_CODE: u16 = 6;

/// Home Key (Alternative) ([`ANSI`]): Parameter code for the Home key variant.
///
/// Value: `1`.
///
/// Sequence: `CSI 1~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_HOME_ALT1_CODE: u16 = 1;

/// End Key (Alternative) ([`ANSI`]): Parameter code for the End key variant.
///
/// Value: `4`.
///
/// Sequence: `CSI 4~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_END_ALT1_CODE: u16 = 4;

/// Home Key (rxvt) ([`ANSI`]): Parameter code for the Home key in rxvt terminals.
///
/// Value: `7`.
///
/// Sequence: `CSI 7~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_HOME_ALT2_CODE: u16 = 7;

/// End Key (rxvt) ([`ANSI`]): Parameter code for the End key in rxvt terminals.
///
/// Value: `8`.
///
/// Sequence: `CSI 8~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SPECIAL_END_ALT2_CODE: u16 = 8;

// ==================== Function Keys (CSI n~) ====================
//
// Function key codes have gaps: F1-F5 are 11-15, F6-F10 are 17-21, F11-F12 are 23-24.

/// Function Key F1 ([`ANSI`]): Parameter code for F1.
///
/// Value: `11`.
///
/// Sequence: `CSI 11~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F1_CODE: u16 = 11;

/// Function Key F2 ([`ANSI`]): Parameter code for F2.
///
/// Value: `12`.
///
/// Sequence: `CSI 12~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F2_CODE: u16 = 12;

/// Function Key F3 ([`ANSI`]): Parameter code for F3.
///
/// Value: `13`.
///
/// Sequence: `CSI 13~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F3_CODE: u16 = 13;

/// Function Key F4 ([`ANSI`]): Parameter code for F4.
///
/// Value: `14`.
///
/// Sequence: `CSI 14~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F4_CODE: u16 = 14;

/// Function Key F5 ([`ANSI`]): Parameter code for F5.
///
/// Value: `15`.
///
/// Sequence: `CSI 15~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F5_CODE: u16 = 15;

/// Function Key F6 ([`ANSI`]): Parameter code for F6.
///
/// Value: `17`.
///
/// Sequence: `CSI 17~` (gap at 16).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F6_CODE: u16 = 17;

/// Function Key F7 ([`ANSI`]): Parameter code for F7.
///
/// Value: `18`.
///
/// Sequence: `CSI 18~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F7_CODE: u16 = 18;

/// Function Key F8 ([`ANSI`]): Parameter code for F8.
///
/// Value: `19`.
///
/// Sequence: `CSI 19~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F8_CODE: u16 = 19;

/// Function Key F9 ([`ANSI`]): Parameter code for F9.
///
/// Value: `20`.
///
/// Sequence: `CSI 20~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F9_CODE: u16 = 20;

/// Function Key F10 ([`ANSI`]): Parameter code for F10.
///
/// Value: `21`.
///
/// Sequence: `CSI 21~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F10_CODE: u16 = 21;

/// Function Key F11 ([`ANSI`]): Parameter code for F11.
///
/// Value: `23`.
///
/// Sequence: `CSI 23~` (gap at 22).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F11_CODE: u16 = 23;

/// Function Key F12 ([`ANSI`]): Parameter code for F12.
///
/// Value: `24`.
///
/// Sequence: `CSI 24~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FUNCTION_F12_CODE: u16 = 24;

// ==================== SS3 Function Keys (SS3 P/Q/R/S) ====================
//
// SS3 sequences (ESC O) are used in application mode for F1-F4.

/// SS3 F1 ([`ANSI`]): Application mode F1 final byte.
///
/// Value: `'P'` dec, `50` hex.
///
/// Sequence: `SS3 P`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_F1_FINAL: u8 = b'P';

/// SS3 F2 ([`ANSI`]): Application mode F2 final byte.
///
/// Value: `'Q'` dec, `51` hex.
///
/// Sequence: `SS3 Q`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_F2_FINAL: u8 = b'Q';

/// SS3 F3 ([`ANSI`]): Application mode F3 final byte.
///
/// Value: `'R'` dec, `52` hex.
///
/// Sequence: `SS3 R`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_F3_FINAL: u8 = b'R';

/// SS3 F4 ([`ANSI`]): Application mode F4 final byte.
///
/// Value: `'S'` dec, `53` hex.
///
/// Sequence: `SS3 S`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_F4_FINAL: u8 = b'S';

// ==================== SS3 Numpad Keys (Application Mode) ====================
//
// In application mode (DECPAM), numpad keys send SS3 sequences instead of digits.

/// SS3 Numpad Enter ([`ANSI`]): Application mode numpad Enter final byte.
///
/// Value: `'M'` dec, `4D` hex.
///
/// Sequence: `SS3 M`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_ENTER: u8 = b'M';

/// SS3 Numpad Multiply ([`ANSI`]): Application mode numpad `*` final byte.
///
/// Value: `'j'` dec, `6A` hex.
///
/// Sequence: `SS3 j`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_MULTIPLY: u8 = b'j';

/// SS3 Numpad Plus ([`ANSI`]): Application mode numpad `+` final byte.
///
/// Value: `'k'` dec, `6B` hex.
///
/// Sequence: `SS3 k`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_PLUS: u8 = b'k';

/// SS3 Numpad Comma ([`ANSI`]): Application mode numpad `,` final byte.
/// Not all terminals support this.
///
/// Value: `'l'` dec, `6C` hex.
///
/// Sequence: `SS3 l`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_COMMA: u8 = b'l';

/// SS3 Numpad Minus ([`ANSI`]): Application mode numpad `-` final byte.
///
/// Value: `'m'` dec, `6D` hex.
///
/// Sequence: `SS3 m`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_MINUS: u8 = b'm';

/// SS3 Numpad Decimal ([`ANSI`]): Application mode numpad `.` final byte.
///
/// Value: `'n'` dec, `6E` hex.
///
/// Sequence: `SS3 n`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_DECIMAL: u8 = b'n';

/// SS3 Numpad Divide ([`ANSI`]): Application mode numpad `/` final byte.
///
/// Value: `'o'` dec, `6F` hex.
///
/// Sequence: `SS3 o`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_DIVIDE: u8 = b'o';

/// SS3 Numpad 0 ([`ANSI`]): Application mode numpad `0` final byte.
///
/// Value: `'p'` dec, `70` hex.
///
/// Sequence: `SS3 p`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_0: u8 = b'p';

/// SS3 Numpad 1 ([`ANSI`]): Application mode numpad `1` final byte.
///
/// Value: `'q'` dec, `71` hex.
///
/// Sequence: `SS3 q`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_1: u8 = b'q';

/// SS3 Numpad 2 ([`ANSI`]): Application mode numpad `2` final byte.
///
/// Value: `'r'` dec, `72` hex.
///
/// Sequence: `SS3 r`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_2: u8 = b'r';

/// SS3 Numpad 3 ([`ANSI`]): Application mode numpad `3` final byte.
///
/// Value: `'s'` dec, `73` hex.
///
/// Sequence: `SS3 s`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_3: u8 = b's';

/// SS3 Numpad 4 ([`ANSI`]): Application mode numpad `4` final byte.
///
/// Value: `'t'` dec, `74` hex.
///
/// Sequence: `SS3 t`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_4: u8 = b't';

/// SS3 Numpad 5 ([`ANSI`]): Application mode numpad `5` final byte.
///
/// Value: `'u'` dec, `75` hex.
///
/// Sequence: `SS3 u`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_5: u8 = b'u';

/// SS3 Numpad 6 ([`ANSI`]): Application mode numpad `6` final byte.
///
/// Value: `'v'` dec, `76` hex.
///
/// Sequence: `SS3 v`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_6: u8 = b'v';

/// SS3 Numpad 7 ([`ANSI`]): Application mode numpad `7` final byte.
///
/// Value: `'w'` dec, `77` hex.
///
/// Sequence: `SS3 w`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_7: u8 = b'w';

/// SS3 Numpad 8 ([`ANSI`]): Application mode numpad `8` final byte.
///
/// Value: `'x'` dec, `78` hex.
///
/// Sequence: `SS3 x`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_8: u8 = b'x';

/// SS3 Numpad 9 ([`ANSI`]): Application mode numpad `9` final byte.
///
/// Value: `'y'` dec, `79` hex.
///
/// Sequence: `SS3 y`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const SS3_NUMPAD_9: u8 = b'y';

// ==================== Modifier Masks ====================
//
// Bitwise flags: bit 0 = Shift, bit 1 = Alt, bit 2 = Ctrl.

/// Shift Modifier ([`ANSI`]): Bit 0 of the modifier bitfield.
///
/// Value: `1`.
///
/// Encoding: `CSI 1 ; 2 X` (modifier parameter = 1 + bitfield).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_SHIFT: u8 = 1;

/// Alt Modifier ([`ANSI`]): Bit 1 of the modifier bitfield.
///
/// Value: `2`.
///
/// Encoding: `CSI 1 ; 3 X` (modifier parameter = 1 + bitfield).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_ALT: u8 = 2;

/// Ctrl Modifier ([`ANSI`]): Bit 2 of the modifier bitfield.
///
/// Value: `4`.
///
/// Encoding: `CSI 1 ; 5 X` (modifier parameter = 1 + bitfield).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_CTRL: u8 = 4;

/// No Modifier ([`ANSI`]): Empty modifier bitfield.
///
/// Value: `0`.
///
/// Encoding: `CSI X` (no modifier parameter).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_NONE: u8 = 0;

/// Alt+Shift Modifier ([`ANSI`]): Combined Alt and Shift bits.
///
/// Value: `3`.
///
/// Encoding: `CSI 1 ; 4 X` (modifier parameter = 1 + bitfield).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_ALT_SHIFT: u8 = MODIFIER_ALT | MODIFIER_SHIFT;

/// Ctrl+Shift Modifier ([`ANSI`]): Combined Ctrl and Shift bits.
///
/// Value: `5`.
///
/// Encoding: `CSI 1 ; 6 X` (modifier parameter = 1 + bitfield).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_CTRL_SHIFT: u8 = MODIFIER_CTRL | MODIFIER_SHIFT;

/// Ctrl+Alt Modifier ([`ANSI`]): Combined Ctrl and Alt bits.
///
/// Value: `6`.
///
/// Encoding: `CSI 1 ; 7 X` (modifier parameter = 1 + bitfield).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_CTRL_ALT: u8 = MODIFIER_CTRL | MODIFIER_ALT;

/// Ctrl+Alt+Shift Modifier ([`ANSI`]): All three modifier bits set.
///
/// Value: `7`.
///
/// Encoding: `CSI 1 ; 8 X` (modifier parameter = 1 + bitfield).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_CTRL_ALT_SHIFT: u8 = MODIFIER_CTRL | MODIFIER_ALT | MODIFIER_SHIFT;

// ==================== Arrow Key Modifiers ====================
//
// Format: `CSI 1 ; modifier A/B/C/D`. Modifier parameter = 1 + bitfield.

/// Arrow Key Modifier Base ([`ANSI`]): Base value for modified arrow key sequences.
///
/// Value: `1`.
///
/// Sequence: `CSI 1 ; modifier A/B/C/D`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const ARROW_KEY_MODIFIER_BASE: u16 = 1;

/// Modifier Parameter Base Character ([`ANSI`]): The `'1'` character in modified key
/// sequences.
///
/// Value: `49` dec, `31` hex.
///
/// Sequence: `CSI 1 ; modifier X` (this is the leading `1`).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const MODIFIER_PARAMETER_BASE_CHAR: u8 = b'1';

/// Modifier Parameter Offset ([`CSI`]): Subtract 1 from [`CSI`] parameter to get
/// bitfield.
///
/// Value: `1`.
///
/// Encoding: modifier bitfield = parameter - 1.
///
/// [`CSI`]: crate::CsiSequence
pub const MODIFIER_PARAMETER_OFFSET: u8 = 1;

// ==================== Control Characters ====================
//
// Ctrl+letter → letter & 0x1F. Reverse: byte | 0x60 → lowercase letter.

/// Control Character Range Maximum ([`ANSI`]): Highest control character byte.
///
/// Value: `31` dec, `1F` hex.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const CTRL_CHAR_RANGE_MAX: u8 = 31;

/// Null (NUL) ([`ANSI`]): The null control character.
///
/// Value: `0` dec, `00` hex.
///
/// Key combo: `Ctrl+Space` or `Ctrl+@`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const CONTROL_NUL: u8 = 0;

/// Horizontal Tab (HT) ([`ANSI`]): The tab control character.
///
/// Value: `9` dec, `09` hex.
///
/// Key combo: `Ctrl+I` or Tab key.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const CONTROL_TAB: u8 = b'\t';

/// Line Feed (LF) ([`ANSI`]): The newline control character.
///
/// Value: `10` dec, `0A` hex.
///
/// Key combo: `Ctrl+J` or Enter (Unix).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const CONTROL_LF: u8 = b'\n';

/// Carriage Return (CR) ([`ANSI`]): The Enter key control character.
///
/// Value: `13` dec, `0D` hex.
///
/// Key combo: `Ctrl+M` or Enter (Windows/Mac).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const CONTROL_ENTER: u8 = b'\r';

/// Escape ([`ESC`]): The escape control character.
///
/// Value: `27` dec, `1B` hex.
///
/// Key combo: `Ctrl+[` or Esc key.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ESC`]: crate::EscSequence
pub const CONTROL_ESC: u8 = 27;

/// Backspace (BS) ([`ANSI`]): The backspace control character.
///
/// Value: `8` dec, `08` hex.
///
/// Key combo: `Ctrl+H` or Backspace key.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const CONTROL_BACKSPACE: u8 = 8;

/// End of Text (ETX) ([`ANSI`]): The `Ctrl+C` control character.
///
/// Value: `3` dec, `03` hex.
///
/// Key combo: `Ctrl+C`. Sends [`SIGINT`] in [cooked mode].
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`SIGINT`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [cooked mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
pub const CONTROL_C: u8 = 3;

/// End of Transmission (EOT) ([`ANSI`]): The `Ctrl+D` control character.
///
/// Value: `4` dec, `04` hex.
///
/// Key combo: `Ctrl+D`. Sends [`EOF`] in [cooked mode].
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [cooked mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
pub const CONTROL_D: u8 = 4;

/// Control-to-Lowercase Mask ([`ANSI`]): OR with control byte to get lowercase letter.
///
/// Value: `96` dec, `60` hex.
///
/// Conversion: `control_byte | 0x60` yields lowercase [`ASCII`] letter.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0b0110_0000;

/// Control-to-Uppercase Mask ([`ANSI`]): OR with control byte to get uppercase letter.
///
/// Value: `64` dec, `40` hex.
///
/// Conversion: `control_byte | 0x40` yields uppercase [`ASCII`] letter.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const CTRL_TO_UPPERCASE_MASK: u8 = 0b0100_0000;

/// Printable [`ASCII`] Minimum ([`ASCII`]): Space character, start of printable range.
///
/// Value: `32` dec, `20` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const PRINTABLE_ASCII_MIN: u8 = b' ';

/// Printable [`ASCII`] Maximum ([`ASCII`]): Tilde character, end of printable range.
///
/// Value: `126` dec, `7E` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const PRINTABLE_ASCII_MAX: u8 = b'~';

/// Delete (DEL) ([`ASCII`]): The delete character, used for Backspace or `Alt+Backspace`.
///
/// Value: `127` dec, `7F` hex.
///
/// Key combo: Backspace key, or `Alt+Backspace` when prefixed with [`ESC`].
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`ESC`]: crate::ANSI_ESC
pub const ASCII_DEL: u8 = 127;

// ==================== ASCII Character Constants ====================
//
// ASCII byte values for digits (0x30-0x39) and letters (A-Z, a-z).
// Note: Cannot use these in match arms due to RFC 1445; use if/else or matches!.

/// [`ASCII`] Digit 0 ([`ASCII`]): Byte value for character `'0'`.
///
/// Value: `48` dec, `30` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_0: u8 = b'0';

/// [`ASCII`] Digit 1 ([`ASCII`]): Byte value for character `'1'`.
///
/// Value: `49` dec, `31` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_1: u8 = b'1';

/// [`ASCII`] Digit 2 ([`ASCII`]): Byte value for character `'2'`.
///
/// Value: `50` dec, `32` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_2: u8 = b'2';

/// [`ASCII`] Digit 3 ([`ASCII`]): Byte value for character `'3'`.
///
/// Value: `51` dec, `33` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_3: u8 = b'3';

/// [`ASCII`] Digit 4 ([`ASCII`]): Byte value for character `'4'`.
///
/// Value: `52` dec, `34` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_4: u8 = b'4';

/// [`ASCII`] Digit 5 ([`ASCII`]): Byte value for character `'5'`.
///
/// Value: `53` dec, `35` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_5: u8 = b'5';

/// [`ASCII`] Digit 6 ([`ASCII`]): Byte value for character `'6'`.
///
/// Value: `54` dec, `36` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_6: u8 = b'6';

/// [`ASCII`] Digit 7 ([`ASCII`]): Byte value for character `'7'`.
///
/// Value: `55` dec, `37` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_7: u8 = b'7';

/// [`ASCII`] Digit 8 ([`ASCII`]): Byte value for character `'8'`.
///
/// Value: `56` dec, `38` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_8: u8 = b'8';

/// [`ASCII`] Digit 9 ([`ASCII`]): Byte value for character `'9'`.
///
/// Value: `57` dec, `39` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_DIGIT_9: u8 = b'9';

/// [`ASCII`] Upper A ([`ASCII`]): Byte value for character `'A'`.
///
/// Value: `65` dec, `41` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_UPPER_A: u8 = b'A';

/// [`ASCII`] Upper Z ([`ASCII`]): Byte value for character `'Z'`.
///
/// Value: `90` dec, `5A` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_UPPER_Z: u8 = b'Z';

/// [`ASCII`] Lower a ([`ASCII`]): Byte value for character `'a'`.
///
/// Value: `97` dec, `61` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_LOWER_A: u8 = b'a';

/// [`ASCII`] Lower z ([`ASCII`]): Byte value for character `'z'`.
///
/// Value: `122` dec, `7A` hex.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ASCII_LOWER_Z: u8 = b'z';

// ==================== CSI Prefix ====================

/// Control Sequence Introducer ([`CSI`]) Prefix: Two-byte introducer for all [`CSI`]
/// sequences.
///
/// Sequence: `ESC [` (`1B 5B` hex).
///
/// [`CSI`]: crate::CsiSequence
pub const CSI_PREFIX: &[u8] = b"\x1b[";

/// [`CSI`] Prefix Length: Number of bytes in the [`CSI_PREFIX`].
///
/// Derived from [`CSI_PREFIX`].
///
/// [`CSI`]: crate::CsiSequence
pub const CSI_PREFIX_LEN: usize = CSI_PREFIX.len();

// ==================== DECCKM Cursor Key Mode Sequences ====================
//
// Complete byte sequences for detecting DECCKM mode changes in PTY output.
// Used by `CursorModeDetector::scan_for_mode_change()`.

/// [`DEC`] Cursor Key Mode (DECCKM) Enable: Switch to application mode cursor keys.
///
/// Sequence: `ESC [ ? 1 h`
///
/// Byte representation of `CsiSequence::EnablePrivateMode(PrivateModeType::CursorKeys)`.
/// See `DECCKM_CURSOR_KEYS` for the mode number.
///
/// [`CSI`]: crate::CsiSequence
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub const DECCKM_ENABLE_BYTES: &[u8] = b"\x1b[?1h";

/// [`DEC`] Cursor Key Mode (DECCKM) Disable: Switch to normal mode cursor keys.
///
/// Sequence: `ESC [ ? 1 l`
///
/// Byte representation of `CsiSequence::DisablePrivateMode(PrivateModeType::CursorKeys)`.
/// See `DECCKM_CURSOR_KEYS` for the mode number.
///
/// [`CSI`]: crate::CsiSequence
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub const DECCKM_DISABLE_BYTES: &[u8] = b"\x1b[?1l";

/// DECCKM Sequence Length: Number of bytes in DECCKM enable/disable sequences.
///
/// Sequence: 5 bytes (`ESC [ ? 1 h` or `ESC [ ? 1 l`).
pub const DECCKM_SEQ_LEN: usize = 5;

// ==================== Terminal Focus Events ====================

/// Focus Gained ([`ANSI`]): Terminal window gained focus final byte.
///
/// Value: `73` dec, `49` hex.
///
/// Sequence: `CSI I`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FOCUS_GAINED_FINAL: u8 = b'I';

/// Focus Lost ([`ANSI`]): Terminal window lost focus final byte.
///
/// Value: `79` dec, `4F` hex.
///
/// Sequence: `CSI O`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const FOCUS_LOST_FINAL: u8 = b'O';

// ==================== Resize Event Constants ====================
//
// Format: `CSI 8 ; rows ; cols t`

/// Resize Event Parameter ([`ANSI`]): Numeric parameter for parsing resize events.
///
/// Value: `8`.
///
/// Sequence: `CSI 8 ; rows ; cols t`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const RESIZE_EVENT_PARSE_PARAM: u16 = 8;

/// Resize Event Code ([`ANSI`]): The `'8'` character for generating resize sequences.
///
/// Value: `56` dec, `38` hex.
///
/// Sequence: `CSI 8 ; rows ; cols t`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const RESIZE_EVENT_GENERATE_CODE: u8 = b'8';

/// Resize Event Terminator ([`ANSI`]): The `'t'` character that ends resize sequences.
///
/// Value: `116` dec, `74` hex.
///
/// Sequence: `CSI 8 ; rows ; cols t`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const RESIZE_TERMINATOR: u8 = b't';

// ==================== Bracketed Paste Mode Constants ====================
//
// Start: `CSI 200 ~`, End: `CSI 201 ~`

/// Bracketed Paste Start Parameter ([`ANSI`]): Numeric parameter for parsing paste start.
///
/// Value: `200`.
///
/// Sequence: `CSI 200~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const PASTE_START_PARSE_PARAM: u16 = 200;

/// Bracketed Paste End Parameter ([`ANSI`]): Numeric parameter for parsing paste end.
///
/// Value: `201`.
///
/// Sequence: `CSI 201~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const PASTE_END_PARSE_PARAM: u16 = 201;

/// Bracketed Paste Start Code ([`ANSI`]): String `"200"` for generating paste start
/// sequences.
///
/// Value: `"200"`.
///
/// Sequence: `CSI 200~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const PASTE_START_GENERATE_CODE: &str = "200";

/// Bracketed Paste End Code ([`ANSI`]): String `"201"` for generating paste end
/// sequences.
///
/// Value: `"201"`.
///
/// Sequence: `CSI 201~`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const PASTE_END_GENERATE_CODE: &str = "201";

/// Tests that verify [`ANSI`]/[`VT-100`] protocol constants match their specification
/// values.
///
/// These tests:
/// - prevent subtle input parsing bugs that would be difficult to diagnose.
/// - serve as **living documentation** and **regression guards** for magic numbers
///   defined by the terminal protocol.
///
/// The constants are unlikely to change accidentally, but these tests:
/// - Document the expected values explicitly (easier to discover than reading hex).
/// - Catch refactoring mistakes if constants are reorganized.
/// - Highlight protocol quirks like function key code gaps (F6=17 skips 16, F11=23 skips
///   22).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_sequence_components() {
        // Verify ESC and CSI bracket constants.
        assert_eq!(ANSI_ESC, 0x1B);
        assert_eq!(ANSI_CSI_BRACKET, 0x5B);
        assert_eq!(ANSI_PARAM_SEPARATOR, b';');
        assert_eq!(ANSI_FUNCTION_KEY_TERMINATOR, b'~');
    }

    #[test]
    fn test_arrow_key_final_bytes() {
        assert_eq!(ARROW_UP_FINAL, b'A');
        assert_eq!(ARROW_DOWN_FINAL, b'B');
        assert_eq!(ARROW_RIGHT_FINAL, b'C');
        assert_eq!(ARROW_LEFT_FINAL, b'D');
    }

    #[test]
    fn test_special_key_codes() {
        // Verify special key codes
        assert_eq!(SPECIAL_INSERT_CODE, 2);
        assert_eq!(SPECIAL_DELETE_CODE, 3);
        assert_eq!(SPECIAL_PAGE_UP_CODE, 5);
        assert_eq!(SPECIAL_PAGE_DOWN_CODE, 6);
    }

    #[test]
    fn test_function_key_codes() {
        // Verify function key codes are correct with gaps
        assert_eq!(FUNCTION_F1_CODE, 11);
        assert_eq!(FUNCTION_F5_CODE, 15);
        assert_eq!(FUNCTION_F6_CODE, 17); // Gap at 16
        assert_eq!(FUNCTION_F10_CODE, 21);
        assert_eq!(FUNCTION_F11_CODE, 23); // Gap at 22
        assert_eq!(FUNCTION_F12_CODE, 24);
    }

    #[test]
    fn test_modifier_masks() {
        // Verify modifier encoding
        assert_eq!(MODIFIER_SHIFT, 1);
        assert_eq!(MODIFIER_ALT, 2);
        assert_eq!(MODIFIER_CTRL, 4);
        assert_eq!(MODIFIER_ALT_SHIFT, 3);
        assert_eq!(MODIFIER_CTRL_SHIFT, 5);
        assert_eq!(MODIFIER_CTRL_ALT, 6);
        assert_eq!(MODIFIER_CTRL_ALT_SHIFT, 7);
    }

    #[test]
    fn test_control_characters() {
        assert_eq!(CONTROL_TAB, b'\t');
        assert_eq!(CONTROL_ENTER, b'\r');
        assert_eq!(CONTROL_ESC, 0x1B);
        assert_eq!(CONTROL_BACKSPACE, 0x08);
    }
}

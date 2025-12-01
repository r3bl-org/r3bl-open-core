// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT100 keyboard input sequence constants.
//!
//! This module contains constant values for keyboard input sequences (arrow keys,
//! function keys, modifiers, control characters). Mouse constants are in the [`mouse`]
//! module and re-exported here.
//!
//! For VT-100 keyboard encoding history and design decisions, see the [keyboard module
//! documentation].
//!
//! [`mouse`]: crate::core::ansi::constants::mouse
//! [keyboard module documentation]: crate::core::ansi::vt_100_terminal_input_parser::keyboard#vt-100-keyboard-input-encoding-explained

// ==================== ANSI Sequence Components ====================

/// ESC byte (27 dec, 1B hex).
pub const ANSI_ESC: u8 = 27;

/// CSI bracket byte `[` (91 dec, 5B hex).
pub const ANSI_CSI_BRACKET: u8 = b'[';

/// SS3 `O` byte for `ESC O` sequences (79 dec, 4F hex).
pub const ANSI_SS3_O: u8 = b'O';

/// Parameter separator `;` (59 dec, 3B hex).
pub const ANSI_PARAM_SEPARATOR: u8 = b';';

/// Function key terminator `~` (126 dec, 7E hex).
pub const ANSI_FUNCTION_KEY_TERMINATOR: u8 = b'~';

// ==================== Arrow Keys (CSI A/B/C/D) ====================

/// `CSI A`: Up arrow (65 dec, 41 hex).
pub const ARROW_UP_FINAL: u8 = b'A';

/// `CSI B`: Down arrow (66 dec, 42 hex).
pub const ARROW_DOWN_FINAL: u8 = b'B';

/// `CSI C`: Right arrow (67 dec, 43 hex).
pub const ARROW_RIGHT_FINAL: u8 = b'C';

/// `CSI D`: Left arrow (68 dec, 44 hex).
pub const ARROW_LEFT_FINAL: u8 = b'D';

// ==================== Tab Keys ====================

/// `CSI Z`: BackTab / Shift+Tab (90 dec, 5A hex).
pub const BACKTAB_FINAL: u8 = b'Z';

// ==================== Special Keys (CSI H/F) ====================

/// `CSI H`: Home key (72 dec, 48 hex).
pub const SPECIAL_HOME_FINAL: u8 = b'H';

/// `CSI F`: End key (70 dec, 46 hex).
pub const SPECIAL_END_FINAL: u8 = b'F';

// ==================== Special Keys (CSI n~) ====================

/// `CSI 2~`: Insert key.
pub const SPECIAL_INSERT_CODE: u16 = 2;

/// `CSI 3~`: Delete key.
pub const SPECIAL_DELETE_CODE: u16 = 3;

/// `CSI 5~`: Page Up key.
pub const SPECIAL_PAGE_UP_CODE: u16 = 5;

/// `CSI 6~`: Page Down key.
pub const SPECIAL_PAGE_DOWN_CODE: u16 = 6;

/// `CSI 1~`: Home key (alternative).
pub const SPECIAL_HOME_ALT1_CODE: u16 = 1;

/// `CSI 4~`: End key (alternative).
pub const SPECIAL_END_ALT1_CODE: u16 = 4;

/// `CSI 7~`: Home key (rxvt).
pub const SPECIAL_HOME_ALT2_CODE: u16 = 7;

/// `CSI 8~`: End key (rxvt).
pub const SPECIAL_END_ALT2_CODE: u16 = 8;

// ==================== Function Keys (CSI n~) ====================
//
// Function key codes have gaps: F1-F5 are 11-15, F6-F10 are 17-21, F11-F12 are 23-24.

/// `CSI 11~`: F1.
pub const FUNCTION_F1_CODE: u16 = 11;

/// `CSI 12~`: F2.
pub const FUNCTION_F2_CODE: u16 = 12;

/// `CSI 13~`: F3.
pub const FUNCTION_F3_CODE: u16 = 13;

/// `CSI 14~`: F4.
pub const FUNCTION_F4_CODE: u16 = 14;

/// `CSI 15~`: F5.
pub const FUNCTION_F5_CODE: u16 = 15;

/// `CSI 17~`: F6.
pub const FUNCTION_F6_CODE: u16 = 17;

/// `CSI 18~`: F7.
pub const FUNCTION_F7_CODE: u16 = 18;

/// `CSI 19~`: F8.
pub const FUNCTION_F8_CODE: u16 = 19;

/// `CSI 20~`: F9.
pub const FUNCTION_F9_CODE: u16 = 20;

/// `CSI 21~`: F10.
pub const FUNCTION_F10_CODE: u16 = 21;

/// `CSI 23~`: F11.
pub const FUNCTION_F11_CODE: u16 = 23;

/// `CSI 24~`: F12.
pub const FUNCTION_F12_CODE: u16 = 24;

// ==================== SS3 Function Keys (SS3 P/Q/R/S) ====================
//
// SS3 sequences (ESC O) are used in application mode for F1-F4.

/// `SS3 P`: F1 (80 dec, 50 hex).
pub const SS3_F1_FINAL: u8 = b'P';

/// `SS3 Q`: F2 (81 dec, 51 hex).
pub const SS3_F2_FINAL: u8 = b'Q';

/// `SS3 R`: F3 (82 dec, 52 hex).
pub const SS3_F3_FINAL: u8 = b'R';

/// `SS3 S`: F4 (83 dec, 53 hex).
pub const SS3_F4_FINAL: u8 = b'S';

// ==================== SS3 Numpad Keys (Application Mode) ====================
//
// In application mode (DECPAM), numpad keys send SS3 sequences instead of digits.

/// `SS3 M`: Numpad Enter (77 dec, 4D hex).
pub const SS3_NUMPAD_ENTER: u8 = b'M';

/// `SS3 j`: Numpad `*` (106 dec, 6A hex).
pub const SS3_NUMPAD_MULTIPLY: u8 = b'j';

/// `SS3 k`: Numpad `+` (107 dec, 6B hex).
pub const SS3_NUMPAD_PLUS: u8 = b'k';

/// `SS3 l`: Numpad `,` (108 dec, 6C hex). Not all terminals support this.
pub const SS3_NUMPAD_COMMA: u8 = b'l';

/// `SS3 m`: Numpad `-` (109 dec, 6D hex).
pub const SS3_NUMPAD_MINUS: u8 = b'm';

/// `SS3 n`: Numpad `.` (110 dec, 6E hex).
pub const SS3_NUMPAD_DECIMAL: u8 = b'n';

/// `SS3 o`: Numpad `/` (111 dec, 6F hex).
pub const SS3_NUMPAD_DIVIDE: u8 = b'o';

/// `SS3 p`: Numpad 0 (112 dec, 70 hex).
pub const SS3_NUMPAD_0: u8 = b'p';

/// `SS3 q`: Numpad 1 (113 dec, 71 hex).
pub const SS3_NUMPAD_1: u8 = b'q';

/// `SS3 r`: Numpad 2 (114 dec, 72 hex).
pub const SS3_NUMPAD_2: u8 = b'r';

/// `SS3 s`: Numpad 3 (115 dec, 73 hex).
pub const SS3_NUMPAD_3: u8 = b's';

/// `SS3 t`: Numpad 4 (116 dec, 74 hex).
pub const SS3_NUMPAD_4: u8 = b't';

/// `SS3 u`: Numpad 5 (117 dec, 75 hex).
pub const SS3_NUMPAD_5: u8 = b'u';

/// `SS3 v`: Numpad 6 (118 dec, 76 hex).
pub const SS3_NUMPAD_6: u8 = b'v';

/// `SS3 w`: Numpad 7 (119 dec, 77 hex).
pub const SS3_NUMPAD_7: u8 = b'w';

/// `SS3 x`: Numpad 8 (120 dec, 78 hex).
pub const SS3_NUMPAD_8: u8 = b'x';

/// `SS3 y`: Numpad 9 (121 dec, 79 hex).
pub const SS3_NUMPAD_9: u8 = b'y';

// ==================== Modifier Masks ====================
//
// Bitwise flags: bit 0 = Shift, bit 1 = Alt, bit 2 = Ctrl.

/// Shift modifier (bit 0).
pub const MODIFIER_SHIFT: u8 = 1;

/// Alt modifier (bit 1).
pub const MODIFIER_ALT: u8 = 2;

/// Ctrl modifier (bit 2).
pub const MODIFIER_CTRL: u8 = 4;

/// No modifiers.
pub const MODIFIER_NONE: u8 = 0;

/// Alt+Shift.
pub const MODIFIER_ALT_SHIFT: u8 = MODIFIER_ALT | MODIFIER_SHIFT;

/// Ctrl+Shift.
pub const MODIFIER_CTRL_SHIFT: u8 = MODIFIER_CTRL | MODIFIER_SHIFT;

/// Ctrl+Alt.
pub const MODIFIER_CTRL_ALT: u8 = MODIFIER_CTRL | MODIFIER_ALT;

/// Ctrl+Alt+Shift.
pub const MODIFIER_CTRL_ALT_SHIFT: u8 = MODIFIER_CTRL | MODIFIER_ALT | MODIFIER_SHIFT;

// ==================== Arrow Key Modifiers ====================
//
// Format: `CSI 1 ; modifier A/B/C/D`. Modifier parameter = 1 + bitfield.

/// Arrow key modifier base value.
pub const ARROW_KEY_MODIFIER_BASE: u16 = 1;

/// Modifier parameter base character `'1'` (49 dec, 31 hex).
pub const MODIFIER_PARAMETER_BASE_CHAR: u8 = b'1';

/// Offset to convert CSI parameter to bitfield (subtract 1).
pub const MODIFIER_PARAMETER_OFFSET: u8 = 1;

// ==================== Control Characters ====================
//
// Ctrl+letter → letter & 0x1F. Reverse: byte | 0x60 → lowercase letter.

/// Control character range maximum (31 dec, 1F hex).
pub const CTRL_CHAR_RANGE_MAX: u8 = 31;

/// NUL (0 dec, 00 hex). Ctrl+Space or Ctrl+@.
pub const CONTROL_NUL: u8 = 0;

/// Tab (9 dec, 09 hex). Ctrl+I or Tab key.
pub const CONTROL_TAB: u8 = b'\t';

/// Line Feed (10 dec, 0A hex). Ctrl+J or Enter (Unix).
pub const CONTROL_LF: u8 = b'\n';

/// Carriage Return (13 dec, 0D hex). Ctrl+M or Enter (Windows/Mac).
pub const CONTROL_ENTER: u8 = b'\r';

/// Escape (27 dec, 1B hex). Ctrl+\[ or Esc key.
pub const CONTROL_ESC: u8 = 27;

/// Backspace (8 dec, 08 hex). Ctrl+H or Backspace key.
pub const CONTROL_BACKSPACE: u8 = 8;

/// Ctrl+C / ETX (3 dec, 03 hex). SIGINT in cooked mode.
pub const CONTROL_C: u8 = 3;

/// Ctrl+D / EOT (4 dec, 04 hex). EOF in cooked mode.
pub const CONTROL_D: u8 = 4;

/// Mask to convert control byte to lowercase (96 dec, 60 hex).
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0b0110_0000;

/// Mask to convert control byte to uppercase (64 dec, 40 hex).
pub const CTRL_TO_UPPERCASE_MASK: u8 = 0b0100_0000;

/// Printable ASCII minimum: space (32 dec, 20 hex).
pub const PRINTABLE_ASCII_MIN: u8 = b' ';

/// Printable ASCII maximum: tilde (126 dec, 7E hex).
pub const PRINTABLE_ASCII_MAX: u8 = b'~';

/// DEL character (127 dec, 7F hex). Backspace key or Alt+Backspace with ESC.
pub const ASCII_DEL: u8 = 127;

// ==================== ASCII Character Constants ====================
//
// ASCII byte values for digits (0x30-0x39) and letters (A-Z, a-z).
// Note: Cannot use these in match arms due to RFC 1445; use if/else or matches!.

/// ASCII `'0'` (48 dec, 30 hex).
pub const ASCII_DIGIT_0: u8 = b'0';

/// ASCII `'1'` (49 dec, 31 hex).
pub const ASCII_DIGIT_1: u8 = b'1';

/// ASCII `'2'` (50 dec, 32 hex).
pub const ASCII_DIGIT_2: u8 = b'2';

/// ASCII `'3'` (51 dec, 33 hex).
pub const ASCII_DIGIT_3: u8 = b'3';

/// ASCII `'4'` (52 dec, 34 hex).
pub const ASCII_DIGIT_4: u8 = b'4';

/// ASCII `'5'` (53 dec, 35 hex).
pub const ASCII_DIGIT_5: u8 = b'5';

/// ASCII `'6'` (54 dec, 36 hex).
pub const ASCII_DIGIT_6: u8 = b'6';

/// ASCII `'7'` (55 dec, 37 hex).
pub const ASCII_DIGIT_7: u8 = b'7';

/// ASCII `'8'` (56 dec, 38 hex).
pub const ASCII_DIGIT_8: u8 = b'8';

/// ASCII `'9'` (57 dec, 39 hex).
pub const ASCII_DIGIT_9: u8 = b'9';

/// ASCII `'A'` (65 dec, 41 hex).
pub const ASCII_UPPER_A: u8 = b'A';

/// ASCII `'Z'` (90 dec, 5A hex).
pub const ASCII_UPPER_Z: u8 = b'Z';

/// ASCII `'a'` (97 dec, 61 hex).
pub const ASCII_LOWER_A: u8 = b'a';

/// ASCII `'z'` (122 dec, 7A hex).
pub const ASCII_LOWER_Z: u8 = b'z';

// ==================== Mouse Protocol Markers ====================

// Mouse constants are now in the dedicated `mouse` module.
// Re-export them for backward compatibility with existing imports.
pub use crate::core::ansi::constants::mouse::*;

// ==================== CSI Prefix ====================

/// CSI prefix `ESC [`.
pub const CSI_PREFIX: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET];

/// CSI prefix length (2 bytes).
pub const CSI_PREFIX_LEN: usize = 2;

// ==================== Terminal Focus Events ====================

/// `CSI I`: Focus gained (73 dec, 49 hex).
pub const FOCUS_GAINED_FINAL: u8 = b'I';

/// `CSI O`: Focus lost (79 dec, 4F hex).
pub const FOCUS_LOST_FINAL: u8 = b'O';

// ==================== Resize Event Constants ====================
//
// Format: `CSI 8 ; rows ; cols t`

/// Resize event parameter for parsing.
pub const RESIZE_EVENT_PARSE_PARAM: u16 = 8;

/// Resize event code `'8'` for generation (56 dec, 38 hex).
pub const RESIZE_EVENT_GENERATE_CODE: u8 = b'8';

/// Resize event terminator `'t'` (116 dec, 74 hex).
pub const RESIZE_TERMINATOR: u8 = b't';

// ==================== Bracketed Paste Mode Constants ====================
//
// Start: `CSI 200 ~`, End: `CSI 201 ~`

/// Paste start parameter for parsing.
pub const PASTE_START_PARSE_PARAM: u16 = 200;

/// Paste end parameter for parsing.
pub const PASTE_END_PARSE_PARAM: u16 = 201;

/// Paste start code `"200"` for generation.
pub const PASTE_START_GENERATE_CODE: &str = "200";

/// Paste end code `"201"` for generation.
pub const PASTE_END_GENERATE_CODE: &str = "201";

/// Tests that verify ANSI/VT100 protocol constants match their specification values.
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT100 input sequence constants.
//!
//! This module contains all constant values used in keyboard and mouse input sequences,
//! organized by functional category.
//!
//! # ANSI Input Sequence Format
//!
//! Input sequences follow the CSI (Control Sequence Introducer) format:
//! - `CSI` = ESC [  (0x1B 0x5B)
//! - Keyboard sequences: `CSI final_byte` or `CSI param; modifier final_byte`
//! - Mouse sequences: `CSI param; param M` or similar
//!
//! # Keyboard Sequences
//!
//! ## Arrow Keys (CSI A/B/C/D)
//! - Up: `ESC[A`
//! - Down: `ESC[B`
//! - Right: `ESC[C`
//! - Left: `ESC[D`
//!
//! ## Function Keys (CSI n~)
//! - F1-F5: codes 11-15
//! - F6-F10: codes 17-21
//! - F11-F12: codes 23-24
//!
//! ## Special Keys (CSI n~)
//! - Insert: `CSI 2~`
//! - Delete: `CSI 3~`
//! - Page Up: `CSI 5~`
//! - Page Down: `CSI 6~`
//! - Home: `CSI H`
//! - End: `CSI F`
//!
//! ## Modifiers (CSI 1; m `final_byte`)
//! Modifier encoding:
//! - 0 = no modifiers
//! - 1 = Shift
//! - 2 = Alt
//! - 3 = Alt+Shift
//! - 4 = Ctrl
//! - 5 = Ctrl+Shift
//! - 6 = Ctrl+Alt
//! - 7 = Ctrl+Alt+Shift

// ==================== ANSI Sequence Components ====================

/// ESC byte (27 in decimal, 0x1B in hex)
pub const ANSI_ESC: u8 = 0x1B;

/// CSI bracket byte: `[` (91 in decimal, 0x5B in hex)
pub const ANSI_CSI_BRACKET: u8 = 0x5B;

/// SS3 'O' byte: Second byte of SS3 sequences (0x4F)
/// SS3 sequences format: ESC O `command_char` (used in application mode).
pub const ANSI_SS3_O: u8 = b'O';

/// Parameter separator byte: `;` (59 in decimal, 0x3B in hex)
pub const ANSI_PARAM_SEPARATOR: u8 = b';';

/// Function key terminator: `~` (126 in decimal, 0x7E in hex)
pub const ANSI_FUNCTION_KEY_TERMINATOR: u8 = b'~';

// ==================== Arrow Keys (CSI A/B/C/D) ====================

/// CSI A: Up arrow key final byte
pub const ARROW_UP_FINAL: u8 = b'A';

/// CSI B: Down arrow key final byte
pub const ARROW_DOWN_FINAL: u8 = b'B';

/// CSI C: Right arrow key final byte
pub const ARROW_RIGHT_FINAL: u8 = b'C';

/// CSI D: Left arrow key final byte
pub const ARROW_LEFT_FINAL: u8 = b'D';

// ==================== Tab Keys ====================

/// CSI Z: `BackTab` (Shift+Tab) final byte
pub const BACKTAB_FINAL: u8 = b'Z';

// ==================== Special Keys (CSI H/F) ====================

/// CSI H: Home key final byte
pub const SPECIAL_HOME_FINAL: u8 = b'H';

/// CSI F: End key final byte
pub const SPECIAL_END_FINAL: u8 = b'F';

// ==================== Special Keys (CSI n~) ====================

/// CSI 2~: Insert key code
pub const SPECIAL_INSERT_CODE: u16 = 2;

/// CSI 3~: Delete key code
pub const SPECIAL_DELETE_CODE: u16 = 3;

/// CSI 5~: Page Up key code
pub const SPECIAL_PAGE_UP_CODE: u16 = 5;

/// CSI 6~: Page Down key code
pub const SPECIAL_PAGE_DOWN_CODE: u16 = 6;

/// CSI 1~: Home key code (alternative)
pub const SPECIAL_HOME_ALT1_CODE: u16 = 1;

/// CSI 4~: End key code (alternative)
pub const SPECIAL_END_ALT1_CODE: u16 = 4;

/// CSI 7~: Home key code (alternative, rxvt)
pub const SPECIAL_HOME_ALT2_CODE: u16 = 7;

/// CSI 8~: End key code (alternative, rxvt)
pub const SPECIAL_END_ALT2_CODE: u16 = 8;

// ==================== Function Keys (CSI n~) ====================
//
// ANSI function key codes (with gaps - non-sequential):
// - F1: 11, F2: 12, F3: 13, F4: 14, F5: 15
// - [gap at 16]
// - F6: 17, F7: 18, F8: 19, F9: 20, F10: 21
// - [gap at 22]
// - F11: 23, F12: 24

/// CSI 11~: Function key F1
pub const FUNCTION_F1_CODE: u16 = 11;

/// CSI 12~: Function key F2
pub const FUNCTION_F2_CODE: u16 = 12;

/// CSI 13~: Function key F3
pub const FUNCTION_F3_CODE: u16 = 13;

/// CSI 14~: Function key F4
pub const FUNCTION_F4_CODE: u16 = 14;

/// CSI 15~: Function key F5
pub const FUNCTION_F5_CODE: u16 = 15;

/// CSI 17~: Function key F6
pub const FUNCTION_F6_CODE: u16 = 17;

/// CSI 18~: Function key F7
pub const FUNCTION_F7_CODE: u16 = 18;

/// CSI 19~: Function key F8
pub const FUNCTION_F8_CODE: u16 = 19;

/// CSI 20~: Function key F9
pub const FUNCTION_F9_CODE: u16 = 20;

/// CSI 21~: Function key F10
pub const FUNCTION_F10_CODE: u16 = 21;

/// CSI 23~: Function key F11
pub const FUNCTION_F11_CODE: u16 = 23;

/// CSI 24~: Function key F12
pub const FUNCTION_F12_CODE: u16 = 24;

// ==================== SS3 Function Keys (SS3 P/Q/R/S) ====================
//
// SS3 sequences (ESC O) are used in application mode for F1-F4.
// Format: ESC O `command_char`

/// SS3 P: Function key F1 (application mode)
pub const SS3_F1_FINAL: u8 = b'P';

/// SS3 Q: Function key F2 (application mode)
pub const SS3_F2_FINAL: u8 = b'Q';

/// SS3 R: Function key F3 (application mode)
pub const SS3_F3_FINAL: u8 = b'R';

/// SS3 S: Function key F4 (application mode)
pub const SS3_F4_FINAL: u8 = b'S';

// ==================== SS3 Numpad Keys (Application Mode) ====================
//
// When numpad is in application mode (DECPAM), numpad keys send SS3 sequences
// instead of their literal digits. Format: ESC O `command_char`
//
// This allows applications to distinguish numpad from regular number keys.
// For example, vim can use numpad for navigation while regular numbers set counts.

/// SS3 M: Numpad Enter (application mode)
pub const SS3_NUMPAD_ENTER: u8 = b'M';

/// SS3 j: Numpad * (multiply)
pub const SS3_NUMPAD_MULTIPLY: u8 = b'j';

/// SS3 k: Numpad + (plus)
pub const SS3_NUMPAD_PLUS: u8 = b'k';

/// SS3 l: Numpad , (comma/separator) - Note: not all terminals support this
pub const SS3_NUMPAD_COMMA: u8 = b'l';

/// SS3 m: Numpad - (minus)
pub const SS3_NUMPAD_MINUS: u8 = b'm';

/// SS3 n: Numpad . (decimal point)
pub const SS3_NUMPAD_DECIMAL: u8 = b'n';

/// SS3 o: Numpad / (divide)
pub const SS3_NUMPAD_DIVIDE: u8 = b'o';

/// SS3 p: Numpad 0
pub const SS3_NUMPAD_0: u8 = b'p';

/// SS3 q: Numpad 1
pub const SS3_NUMPAD_1: u8 = b'q';

/// SS3 r: Numpad 2
pub const SS3_NUMPAD_2: u8 = b'r';

/// SS3 s: Numpad 3
pub const SS3_NUMPAD_3: u8 = b's';

/// SS3 t: Numpad 4
pub const SS3_NUMPAD_4: u8 = b't';

/// SS3 u: Numpad 5
pub const SS3_NUMPAD_5: u8 = b'u';

/// SS3 v: Numpad 6
pub const SS3_NUMPAD_6: u8 = b'v';

/// SS3 w: Numpad 7
pub const SS3_NUMPAD_7: u8 = b'w';

/// SS3 x: Numpad 8
pub const SS3_NUMPAD_8: u8 = b'x';

/// SS3 y: Numpad 9
pub const SS3_NUMPAD_9: u8 = b'y';

// ==================== Modifier Masks ====================
//
// Modifier encoding for CSI sequences: CSI base; modifier final_byte
// Modifiers are bitwise flags:
// - bit 0 (value 1): Shift
// - bit 1 (value 2): Alt
// - bit 2 (value 4): Ctrl
//
// Common combinations:
// - 0 = no modifiers
// - 1 = Shift
// - 2 = Alt
// - 3 = Alt+Shift
// - 4 = Ctrl
// - 5 = Ctrl+Shift
// - 6 = Ctrl+Alt
// - 7 = Ctrl+Alt+Shift

/// Modifier mask for Shift key (bit 0)
pub const MODIFIER_SHIFT: u8 = 1;

/// Modifier mask for Alt key (bit 1)
pub const MODIFIER_ALT: u8 = 2;

/// Modifier mask for Ctrl key (bit 2)
pub const MODIFIER_CTRL: u8 = 4;

/// Combined modifier: Alt+Shift
pub const MODIFIER_ALT_SHIFT: u8 = MODIFIER_ALT | MODIFIER_SHIFT;

/// Combined modifier: Ctrl+Shift
pub const MODIFIER_CTRL_SHIFT: u8 = MODIFIER_CTRL | MODIFIER_SHIFT;

/// Combined modifier: Ctrl+Alt
pub const MODIFIER_CTRL_ALT: u8 = MODIFIER_CTRL | MODIFIER_ALT;

/// Combined modifier: Ctrl+Alt+Shift
pub const MODIFIER_CTRL_ALT_SHIFT: u8 = MODIFIER_CTRL | MODIFIER_ALT | MODIFIER_SHIFT;

// ==================== Arrow Key Modifiers ====================
//
// Arrow keys with modifiers use the format: CSI 1; modifier A/B/C/D
// Where 1 is the "base" value and modifier is the mask above

/// Arrow key modifier base value (always 1 for arrow keys with modifiers)
pub const ARROW_KEY_MODIFIER_BASE: u16 = 1;

/// Arrow key modifier parameter base character: `'1'` (ASCII 49)
///
/// Used in VT-100 keyboard sequences with modifiers: `CSI 1; modifier [A/B/C/D/...]`
/// The parameter is computed as: `'1'` + `modifier_mask`, producing `'1'`-`'8'`
///
/// ## VT-100 Modifier Encoding
/// - '1' = no modifiers (1 + 0)
/// - '2' = Shift (1 + 1)
/// - '3' = Alt (1 + 2)
/// - '4' = Alt+Shift (1 + 3)
/// - '5' = Ctrl (1 + 4)
/// - '6' = Ctrl+Shift (1 + 5)
/// - '7' = Ctrl+Alt (1 + 6)
/// - '8' = Ctrl+Alt+Shift (1 + 7)
pub const MODIFIER_PARAMETER_BASE_CHAR: u8 = b'1';

// ==================== Control Characters ====================
//
// Control characters (0x00-0x1F) are generated when Ctrl is held while typing.
// For example: Ctrl+A → 0x01, Ctrl+D → 0x04, Ctrl+W → 0x17
//
// The transformation is: letter & 0x1F = control_byte
// Reverse: control_byte | 0x60 = lowercase letter

/// Control character range maximum (0x1F).
///
/// Control characters occupy bytes 0x00-0x1F in ASCII.
/// This constant marks the upper bound of this range.
pub const CTRL_CHAR_RANGE_MAX: u8 = 0x1F;

/// ASCII NUL character (0x00)
/// Can be generated via Ctrl+Space or Ctrl+@.
pub const CONTROL_NUL: u8 = 0x00;

/// ASCII Tab character (0x09)
/// Can be generated via Ctrl+I or Tab key.
pub const CONTROL_TAB: u8 = b'\t';

/// ASCII Line Feed (0x0A)
/// Can be generated via Ctrl+J or Enter key (Unix).
pub const CONTROL_LF: u8 = b'\n';

/// ASCII Enter/Carriage Return character (0x0D)
/// Can be generated via Ctrl+M or Enter key (Windows/Mac).
pub const CONTROL_ENTER: u8 = b'\r';

/// ASCII Escape character (0x1B)
/// Can be generated via Ctrl+[ or Esc key.
pub const CONTROL_ESC: u8 = 0x1B;

/// ASCII Backspace character (0x08)
/// Can be generated via Ctrl+H or Backspace key.
pub const CONTROL_BACKSPACE: u8 = 0x08;

/// Ctrl+C character (0x03) - End of Text (ETX)
/// Can be generated via Ctrl+C.
/// In cooked mode, typically triggers SIGINT. In raw mode, passed as byte 0x03.
pub const CONTROL_C: u8 = 0x03;

/// Ctrl+D character (0x04) - End of Transmission (EOT)
/// Can be generated via Ctrl+D.
/// In cooked mode, typically signals EOF. In raw mode, passed as byte 0x04.
pub const CONTROL_D: u8 = 0x04;

/// Mask to convert control character to lowercase letter (0x60).
///
/// To reverse the Ctrl transformation (letter & 0x1F → byte),
/// we can compute: byte | 0x60 = lowercase letter.
///
/// Example: 0x01 | 0x60 = 0x61 = 'a'
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0x60;

/// Mask to convert control character to uppercase letter (0x40).
///
/// Alternative reverse transformation: byte | 0x40 = uppercase letter.
///
/// Example: 0x01 | 0x40 = 0x41 = 'A'
pub const CTRL_TO_UPPERCASE_MASK: u8 = 0x40;

/// Printable ASCII minimum (space, 0x20).
///
/// First printable ASCII character. Used to validate Alt+letter sequences,
/// which must be ESC + printable character (0x20-0x7E).
pub const PRINTABLE_ASCII_MIN: u8 = 0x20;

/// Printable ASCII maximum (tilde, 0x7E).
///
/// Last printable ASCII character. Used to validate Alt+letter sequences,
/// which must be ESC + printable character (0x20-0x7E).
pub const PRINTABLE_ASCII_MAX: u8 = 0x7E;

/// ASCII DEL character (0x7F).
///
/// This is the ASCII delete character, typically sent by the Backspace key.
/// When combined with ESC (0x1B), it represents Alt+Backspace: ESC DEL (0x1B 0x7F).
pub const ASCII_DEL: u8 = 0x7F;

// ==================== ASCII Character Constants ====================
//
// These constants define the ASCII byte values for digits and letters used in
// ANSI escape sequence parsing and generation.
//
// ## Why These Constants Are Necessary
//
// ANSI escape sequences are text-based protocols that use ASCII characters to
// represent numeric values. For example:
// - The digit '1' is represented as byte 0x31 (ASCII 49), NOT byte value 1
// - Parameters like "15" are two ASCII bytes: [0x31, 0x35] ('1', '5')
//
// These constants prevent off-by-one errors when converting between:
// - Numeric values (0-9) and their ASCII representations (0x30-0x39)
// - Character ranges used in parsing (b'A'-b'Z', b'a'-b'z')
//
// ## Rust Pattern Matching Limitation
//
// We cannot use these constants in match arms due to RFC 1445. In match patterns,
// constants are treated as variable bindings, not value comparisons. Instead, use:
// - `if/else` chains with direct comparisons
// - `matches!` macro for range checking.
// See keyboard.rs and terminal_events.rs for examples.

/// ASCII digit '0' (48 in decimal, 0x30 in hex)
///
/// Used to convert numeric values to ASCII digits: `ASCII_DIGIT_0 + value`
/// Example: To represent number 5 in ASCII: `ASCII_DIGIT_0 + 5 = 0x35 = '5'`
pub const ASCII_DIGIT_0: u8 = b'0';

/// ASCII digit '1' (49 in decimal, 0x31 in hex)
pub const ASCII_DIGIT_1: u8 = b'1';

/// ASCII digit '2' (50 in decimal, 0x32 in hex)
pub const ASCII_DIGIT_2: u8 = b'2';

/// ASCII digit '3' (51 in decimal, 0x33 in hex)
pub const ASCII_DIGIT_3: u8 = b'3';

/// ASCII digit '4' (52 in decimal, 0x34 in hex)
pub const ASCII_DIGIT_4: u8 = b'4';

/// ASCII digit '5' (53 in decimal, 0x35 in hex)
pub const ASCII_DIGIT_5: u8 = b'5';

/// ASCII digit '6' (54 in decimal, 0x36 in hex)
pub const ASCII_DIGIT_6: u8 = b'6';

/// ASCII digit '7' (55 in decimal, 0x37 in hex)
pub const ASCII_DIGIT_7: u8 = b'7';

/// ASCII digit '8' (56 in decimal, 0x38 in hex)
pub const ASCII_DIGIT_8: u8 = b'8';

/// ASCII digit '9' (57 in decimal, 0x39 in hex)
///
/// Upper bound for digit range checking: `b'0'..=b'9'` becomes
/// `ASCII_DIGIT_0..=ASCII_DIGIT_9`
pub const ASCII_DIGIT_9: u8 = b'9';

/// ASCII uppercase letter 'A' (65 in decimal, 0x41 in hex)
///
/// Lower bound for uppercase letter range: `b'A'..=b'Z'` becomes
/// `ASCII_UPPER_A..=ASCII_UPPER_Z`
pub const ASCII_UPPER_A: u8 = b'A';

/// ASCII uppercase letter 'Z' (90 in decimal, 0x5A in hex)
///
/// Upper bound for uppercase letter range
pub const ASCII_UPPER_Z: u8 = b'Z';

/// ASCII lowercase letter 'a' (97 in decimal, 0x61 in hex)
///
/// Lower bound for lowercase letter range: `b'a'..=b'z'` becomes
/// `ASCII_LOWER_A..=ASCII_LOWER_Z`
pub const ASCII_LOWER_A: u8 = b'a';

/// ASCII lowercase letter 'z' (122 in decimal, 0x7A in hex)
///
/// Upper bound for lowercase letter range
pub const ASCII_LOWER_Z: u8 = b'z';

// ==================== Mouse Protocol Markers ====================

// Mouse constants are now in the dedicated `mouse` module.
// Re-export them for backward compatibility with existing imports.
pub use crate::core::ansi::constants::mouse::*;

// ==================== CSI Prefix ====================

/// Basic CSI sequence prefix: ESC [
///
/// Used for general CSI sequence detection and as base for mouse/focus sequences.
pub const CSI_PREFIX: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET];

// ==================== Terminal Focus Events ====================

/// CSI I: Terminal focus gained event final byte
/// Format: ESC [ I
pub const FOCUS_GAINED_FINAL: u8 = b'I';

/// CSI O: Terminal focus lost event final byte
/// Format: ESC [ O
pub const FOCUS_LOST_FINAL: u8 = b'O';

// ==================== Resize Event Constants ====================

/// CSI 8: Window resize event parameter for parsing (numeric).
///
/// The numeric parameter value (8) used when PARSING resize sequences.
/// When we receive `CSI 8 ; rows ; cols t` and parse it, `params[0]` will equal this value.
/// Format: `CSI 8 ; rows ; cols t`
pub const RESIZE_EVENT_PARSE_PARAM: u16 = 8;

/// CSI 8: Window resize event code for generation (ASCII character).
///
/// The ASCII character '8' (0x38) used when GENERATING resize sequences.
/// When we create `CSI 8 ; rows ; cols t`, we push this byte value.
/// Format: `CSI 8 ; rows ; cols t`
pub const RESIZE_EVENT_GENERATE_CODE: u8 = b'8';

/// CSI t: Resize event terminator.
///
/// Final byte in the window resize sequence.
pub const RESIZE_TERMINATOR: u8 = b't';

// ==================== Bracketed Paste Mode Constants ====================

/// Bracketed paste start parameter for parsing: numeric value 200.
///
/// The numeric parameter value used when PARSING paste start sequences.
/// When we receive `CSI 200 ~` and parse it, `params[0]` will equal this value.
/// Full sequence: `CSI 200 ~`
pub const PASTE_START_PARSE_PARAM: u16 = 200;

/// Bracketed paste end parameter for parsing: numeric value 201.
///
/// The numeric parameter value used when PARSING paste end sequences.
/// When we receive `CSI 201 ~` and parse it, params\[0\] will equal this value.
/// Full sequence: `CSI 201 ~`
pub const PASTE_END_PARSE_PARAM: u16 = 201;

/// Bracketed paste start code for generation: `200` (string).
///
/// String representation used when GENERATING paste start sequences.
/// When we create `CSI 200 ~`, we use this string.
/// Full sequence: `CSI 200 ~`
pub const PASTE_START_GENERATE_CODE: &str = "200";

/// Bracketed paste end code for generation: `201` (string).
///
/// String representation used when GENERATING paste end sequences.
/// When we create `CSI 201 ~`, we use this string.
/// Full sequence: `CSI 201 ~`
pub const PASTE_END_GENERATE_CODE: &str = "201";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_sequence_components() {
        // Verify ESC and CSI bracket constants
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

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
/// SS3 sequences format: ESC O `command_char` (used in application mode)
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

// ==================== Control Characters ====================

/// ASCII Tab character (0x09)
pub const CONTROL_TAB: u8 = b'\t';

/// ASCII Enter character (0x0D)
pub const CONTROL_ENTER: u8 = b'\r';

/// ASCII Escape character (0x1B)
pub const CONTROL_ESC: u8 = 0x1B;

/// ASCII Backspace character (0x08)
pub const CONTROL_BACKSPACE: u8 = 0x08;

// ==================== Mouse Protocol Markers ====================

/// SGR mouse protocol marker: `<` (60 in decimal, 0x3C in hex)
/// Used in SGR extended mouse tracking sequences: ESC [ < Cb ; Cx ; Cy M/m
pub const MOUSE_SGR_MARKER: u8 = b'<';

/// X10/Normal mouse protocol marker: `M` (77 in decimal, 0x4D in hex)
/// Used in X10 mouse tracking sequences: ESC [ M Cb Cx Cy
pub const MOUSE_X10_MARKER: u8 = b'M';

// ==================== Mouse Protocol Sequence Prefixes ====================

/// SGR mouse protocol sequence prefix: ESC [ <
/// Used to identify SGR extended mouse tracking sequences
pub const MOUSE_SGR_PREFIX: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, MOUSE_SGR_MARKER];

/// X10/Normal mouse protocol sequence prefix: ESC [ M
/// Used to identify X10 mouse tracking sequences
pub const MOUSE_X10_PREFIX: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, MOUSE_X10_MARKER];

/// Basic CSI sequence prefix: ESC [
/// Used for general CSI sequence detection
pub const CSI_PREFIX: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET];

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

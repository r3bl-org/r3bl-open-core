// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input event generator - converts high-level input events to ANSI sequences.
//!
//! This module provides the inverse operation to the input parsers in
//! [`vt_100_terminal_input_parser`](crate::core::ansi::vt_100_terminal_input_parser).
//!
//! ## Purpose
//!
//! The primary use cases are:
//! 1. **Round-trip testing**: Parse ANSI → InputEvent → Generate ANSI → Parse again
//! 2. **Test helpers**: Build keyboard sequences without hardcoding magic bytes
//! 3. **Symmetric architecture**: Input and output paths have corresponding generators
//!
//! ## Example
//!
//! ```ignore
//! use r3bl_tui::{InputEvent, KeyCode, KeyModifiers, generate_keyboard_sequence};
//!
//! // Generate ANSI for Up arrow with no modifiers
//! let event = InputEvent::Keyboard {
//!     code: KeyCode::Up,
//!     modifiers: KeyModifiers::default(),
//! };
//! let bytes = generate_keyboard_sequence(&event)?;
//! assert_eq!(bytes, b"\x1b[A");
//! ```

use crate::core::ansi::{
    constants::{
        ANSI_CSI_BRACKET, ANSI_ESC, ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
        ARROW_DOWN_FINAL, ARROW_LEFT_FINAL, ARROW_RIGHT_FINAL, ARROW_UP_FINAL,
        FUNCTION_F1_CODE, FUNCTION_F10_CODE, FUNCTION_F11_CODE, FUNCTION_F12_CODE,
        FUNCTION_F2_CODE, FUNCTION_F3_CODE, FUNCTION_F4_CODE, FUNCTION_F5_CODE,
        FUNCTION_F6_CODE, FUNCTION_F7_CODE, FUNCTION_F8_CODE, FUNCTION_F9_CODE,
        MODIFIER_ALT, MODIFIER_CTRL, MODIFIER_SHIFT, SPECIAL_DELETE_CODE,
        SPECIAL_END_FINAL, SPECIAL_HOME_FINAL, SPECIAL_INSERT_CODE, SPECIAL_PAGE_DOWN_CODE,
        SPECIAL_PAGE_UP_CODE,
    },
    vt_100_terminal_input_parser::{InputEvent, KeyCode, KeyModifiers},
};

/// Generate ANSI bytes for a keyboard input event.
///
/// Converts a keyboard input event back into the ANSI CSI sequence format.
///
/// ## Returns
///
/// - `Some(Vec<u8>)` for recognized key combinations
/// - `None` for unrecognized or unsupported key codes
///
/// ## Examples
///
/// ```ignore
/// // Arrow key with no modifiers
/// let event = InputEvent::Keyboard {
///     code: KeyCode::Up,
///     modifiers: KeyModifiers::default(),
/// };
/// assert_eq!(generate_keyboard_sequence(&event)?, b"\x1b[A");
///
/// // Arrow key with modifier
/// let event = InputEvent::Keyboard {
///     code: KeyCode::Right,
///     modifiers: KeyModifiers { shift: true, ctrl: false, alt: false },
/// };
/// assert_eq!(generate_keyboard_sequence(&event)?, b"\x1b[1;1C");
/// ```
pub fn generate_keyboard_sequence(event: &InputEvent) -> Option<Vec<u8>> {
    match event {
        InputEvent::Keyboard { code, modifiers } => {
            generate_key_sequence(*code, *modifiers)
        }
        _ => None,
    }
}

/// Generate ANSI bytes for a specific key code and modifiers.
fn generate_key_sequence(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    // Build the base sequence
    let mut bytes = vec![ANSI_ESC, ANSI_CSI_BRACKET];

    let has_modifiers = modifiers.shift || modifiers.ctrl || modifiers.alt;

    match code {
        // ==================== Arrow Keys ====================
        KeyCode::Up => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_UP_FINAL);
            Some(bytes)
        }
        KeyCode::Down => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_DOWN_FINAL);
            Some(bytes)
        }
        KeyCode::Right => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_RIGHT_FINAL);
            Some(bytes)
        }
        KeyCode::Left => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_LEFT_FINAL);
            Some(bytes)
        }

        // ==================== Special Keys (CSI H/F) ====================
        KeyCode::Home => {
            bytes.push(SPECIAL_HOME_FINAL);
            Some(bytes)
        }
        KeyCode::End => {
            bytes.push(SPECIAL_END_FINAL);
            Some(bytes)
        }

        // ==================== Special Keys (CSI n~) ====================
        KeyCode::Insert => {
            generate_special_key_sequence(&mut bytes, SPECIAL_INSERT_CODE, modifiers)
        }
        KeyCode::Delete => {
            generate_special_key_sequence(&mut bytes, SPECIAL_DELETE_CODE, modifiers)
        }
        KeyCode::PageUp => {
            generate_special_key_sequence(&mut bytes, SPECIAL_PAGE_UP_CODE, modifiers)
        }
        KeyCode::PageDown => {
            generate_special_key_sequence(&mut bytes, SPECIAL_PAGE_DOWN_CODE, modifiers)
        }

        // ==================== Function Keys (CSI n~) ====================
        KeyCode::Function(n) => {
            let code = match n {
                1 => FUNCTION_F1_CODE,
                2 => FUNCTION_F2_CODE,
                3 => FUNCTION_F3_CODE,
                4 => FUNCTION_F4_CODE,
                5 => FUNCTION_F5_CODE,
                6 => FUNCTION_F6_CODE,
                7 => FUNCTION_F7_CODE,
                8 => FUNCTION_F8_CODE,
                9 => FUNCTION_F9_CODE,
                10 => FUNCTION_F10_CODE,
                11 => FUNCTION_F11_CODE,
                12 => FUNCTION_F12_CODE,
                _ => return None, // Invalid function key number
            };
            generate_special_key_sequence(&mut bytes, code, modifiers)
        }

        // ==================== Other Keys ====================
        // Tab, Enter, Escape, Backspace are typically raw control characters,
        // not CSI sequences. Not implemented in generator as they're handled
        // differently in the input parsing layer.
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Enter | KeyCode::Escape | KeyCode::Backspace => None,

        // Char events are also handled differently (UTF-8 text)
        KeyCode::Char(_) => None,
    }
}

/// Generate a special key or function key sequence (CSI n~).
fn generate_special_key_sequence(
    bytes: &mut Vec<u8>,
    code: u16,
    modifiers: KeyModifiers,
) -> Option<Vec<u8>> {
    // Format: CSI code~ or CSI code; modifier~
    let code_str = code.to_string();
    bytes.extend_from_slice(code_str.as_bytes());

    if modifiers.shift || modifiers.ctrl || modifiers.alt {
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.push(encode_modifiers(modifiers));
    }

    bytes.push(ANSI_FUNCTION_KEY_TERMINATOR);
    Some(bytes.clone())
}

/// Encode modifier flags into a single byte following ANSI convention.
///
/// Modifier encoding (bitwise flags):
/// - bit 0 (value 1): Shift
/// - bit 1 (value 2): Alt
/// - bit 2 (value 4): Ctrl
///
/// ## Examples
///
/// - `0` → no modifiers
/// - `1` → Shift
/// - `2` → Alt
/// - `3` → Alt+Shift
/// - `4` → Ctrl
/// - `5` → Ctrl+Shift
/// - `6` → Ctrl+Alt
/// - `7` → Ctrl+Alt+Shift
fn encode_modifiers(modifiers: KeyModifiers) -> u8 {
    let mut mask: u8 = 0;
    if modifiers.shift {
        mask |= MODIFIER_SHIFT;
    }
    if modifiers.alt {
        mask |= MODIFIER_ALT;
    }
    if modifiers.ctrl {
        mask |= MODIFIER_CTRL;
    }
    // ASCII digit for the mask (0-7 as '0'-'7')
    b'0' + mask
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Arrow Keys ====================

    #[test]
    fn test_generate_arrow_up() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Up,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[A");
    }

    #[test]
    fn test_generate_arrow_down() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Down,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[B");
    }

    #[test]
    fn test_generate_arrow_right() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Right,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[C");
    }

    #[test]
    fn test_generate_arrow_left() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Left,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[D");
    }

    // ==================== Arrow Keys with Modifiers ====================

    #[test]
    fn test_generate_shift_up() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Up,
            modifiers: KeyModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            },
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[1;1A");
    }

    #[test]
    fn test_generate_alt_right() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Right,
            modifiers: KeyModifiers {
                shift: false,
                alt: true,
                ctrl: false,
            },
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[1;2C");
    }

    #[test]
    fn test_generate_ctrl_down() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Down,
            modifiers: KeyModifiers {
                shift: false,
                alt: false,
                ctrl: true,
            },
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[1;4B");
    }

    #[test]
    fn test_generate_ctrl_alt_shift_left() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Left,
            modifiers: KeyModifiers {
                shift: true,
                alt: true,
                ctrl: true,
            },
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[1;7D");
    }

    // ==================== Special Keys ====================

    #[test]
    fn test_generate_home_key() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Home,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[H");
    }

    #[test]
    fn test_generate_end_key() {
        let event = InputEvent::Keyboard {
            code: KeyCode::End,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[F");
    }

    #[test]
    fn test_generate_insert_key() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Insert,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[2~");
    }

    #[test]
    fn test_generate_delete_key() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Delete,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[3~");
    }

    #[test]
    fn test_generate_page_up() {
        let event = InputEvent::Keyboard {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[5~");
    }

    #[test]
    fn test_generate_page_down() {
        let event = InputEvent::Keyboard {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[6~");
    }

    // ==================== Function Keys ====================

    #[test]
    fn test_generate_f1_key() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Function(1),
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[11~");
    }

    #[test]
    fn test_generate_f6_key() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Function(6),
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[17~");
    }

    #[test]
    fn test_generate_f12_key() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Function(12),
            modifiers: KeyModifiers::default(),
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[24~");
    }

    // ==================== Function Keys with Modifiers ====================

    #[test]
    fn test_generate_shift_f5() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Function(5),
            modifiers: KeyModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            },
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[15;1~");
    }

    #[test]
    fn test_generate_ctrl_alt_f10() {
        let event = InputEvent::Keyboard {
            code: KeyCode::Function(10),
            modifiers: KeyModifiers {
                shift: false,
                alt: true,
                ctrl: true,
            },
        };
        let bytes = generate_keyboard_sequence(&event).unwrap();
        assert_eq!(bytes, b"\x1b[21;6~");
    }

    // ==================== Unsupported Keys ====================

    #[test]
    fn test_generate_unsupported_keys() {
        // Tab, Enter, Escape, Backspace are not generated as CSI sequences
        let tab_event = InputEvent::Keyboard {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::default(),
        };
        assert_eq!(generate_keyboard_sequence(&tab_event), None);

        let enter_event = InputEvent::Keyboard {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::default(),
        };
        assert_eq!(generate_keyboard_sequence(&enter_event), None);

        let escape_event = InputEvent::Keyboard {
            code: KeyCode::Escape,
            modifiers: KeyModifiers::default(),
        };
        assert_eq!(generate_keyboard_sequence(&escape_event), None);

        let backspace_event = InputEvent::Keyboard {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::default(),
        };
        assert_eq!(generate_keyboard_sequence(&backspace_event), None);
    }

    // ==================== Round-Trip Tests ====================

    #[test]
    fn test_roundtrip_arrow_up() {
        use crate::core::ansi::vt_100_terminal_input_parser::parse_keyboard_sequence;

        let original_event = InputEvent::Keyboard {
            code: KeyCode::Up,
            modifiers: KeyModifiers::default(),
        };

        let bytes = generate_keyboard_sequence(&original_event).unwrap();
        let parsed_event = parse_keyboard_sequence(&bytes);

        assert_eq!(parsed_event, Some(original_event));
    }

    #[test]
    fn test_roundtrip_ctrl_alt_f10() {
        use crate::core::ansi::vt_100_terminal_input_parser::parse_keyboard_sequence;

        let original_event = InputEvent::Keyboard {
            code: KeyCode::Function(10),
            modifiers: KeyModifiers {
                shift: false,
                alt: true,
                ctrl: true,
            },
        };

        let bytes = generate_keyboard_sequence(&original_event).unwrap();
        let parsed_event = parse_keyboard_sequence(&bytes);

        assert_eq!(parsed_event, Some(original_event));
    }

    #[test]
    fn test_roundtrip_insert_key_with_shift() {
        use crate::core::ansi::vt_100_terminal_input_parser::parse_keyboard_sequence;

        let original_event = InputEvent::Keyboard {
            code: KeyCode::Insert,
            modifiers: KeyModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            },
        };

        let bytes = generate_keyboard_sequence(&original_event).unwrap();
        let parsed_event = parse_keyboard_sequence(&bytes);

        assert_eq!(parsed_event, Some(original_event));
    }
}

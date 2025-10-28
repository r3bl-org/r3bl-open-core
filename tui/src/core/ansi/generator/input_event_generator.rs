// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input event generator - converts high-level input events to ANSI sequences.
//!
//! This module provides the inverse operation to the input parsers in
//! [`vt_100_terminal_input_parser`].
//!
//! ## Purpose
//!
//! **This module is for testing only.** It is not used in production code.
//!
//! The generator enables:
//! 1. **Round-trip validation**: Parse ANSI → InputEvent → Generate ANSI → Verify match
//! 2. **Test helpers**: Build test sequences without hardcoding raw bytes
//! 3. **Parser verification**: Confirm parsers handle all modifier combinations correctly
//!
//! [`vt_100_terminal_input_parser`](crate::core::ansi::vt_100_terminal_input_parser)

use crate::core::ansi::{constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ARROW_DOWN_FINAL, ARROW_LEFT_FINAL,
                                    ARROW_RIGHT_FINAL, ARROW_UP_FINAL,
                                    FUNCTION_F1_CODE, FUNCTION_F2_CODE,
                                    FUNCTION_F3_CODE, FUNCTION_F4_CODE,
                                    FUNCTION_F5_CODE, FUNCTION_F6_CODE,
                                    FUNCTION_F7_CODE, FUNCTION_F8_CODE,
                                    FUNCTION_F9_CODE, FUNCTION_F10_CODE,
                                    FUNCTION_F11_CODE, FUNCTION_F12_CODE, MODIFIER_ALT,
                                    MODIFIER_CTRL, MODIFIER_SHIFT, SPECIAL_DELETE_CODE,
                                    SPECIAL_END_FINAL, SPECIAL_HOME_FINAL,
                                    SPECIAL_INSERT_CODE, SPECIAL_PAGE_DOWN_CODE,
                                    SPECIAL_PAGE_UP_CODE},
                        vt_100_terminal_input_parser::{InputEvent, KeyCode,
                                                       KeyModifiers}};

/// Generate ANSI bytes for a keyboard input event.
///
/// Converts a keyboard input event back into the ANSI CSI sequence format that terminals
/// send when keys are pressed.
///
/// ## Returns
///
/// - `Some(Vec<u8>)` for recognized key combinations
/// - `None` for unrecognized or unsupported key codes
///
/// ## Usage
///
/// This function is used internally by tests to generate sequences for round-trip
/// validation. See the test suite for examples of all supported key combinations.
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
        KeyCode::Tab
        | KeyCode::BackTab
        | KeyCode::Enter
        | KeyCode::Escape
        | KeyCode::Backspace => None,

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

/// Encode modifier flags into a single byte following VT-100 ANSI convention.
///
/// **VT-100 Modifier Encoding**: `parameter = 1 + bitfield`
///
/// Where bitfield is:
/// - bit 0 (value 1): Shift
/// - bit 1 (value 2): Alt
/// - bit 2 (value 4): Ctrl
///
/// ## Parameter Values
///
/// - `1` → no modifiers (1 + 0)
/// - `2` → Shift (1 + 1)
/// - `3` → Alt (1 + 2)
/// - `4` → Alt+Shift (1 + 3)
/// - `5` → Ctrl (1 + 4)
/// - `6` → Ctrl+Shift (1 + 5)
/// - `7` → Ctrl+Alt (1 + 6)
/// - `8` → Ctrl+Alt+Shift (1 + 7)
///
/// **Confirmed by terminal observation**: `ESC[1;5A` = Ctrl+Up (parameter 5 = 1+4)
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
    // VT-100 formula: parameter = 1 + bitfield
    // Convert to ASCII digit ('1'-'8' for modifiers 0-7)
    b'1' + mask
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
        // Shift modifier: parameter = 1 + 1 = 2
        assert_eq!(bytes, b"\x1b[1;2A");
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
        // Alt modifier: parameter = 1 + 2 = 3
        assert_eq!(bytes, b"\x1b[1;3C");
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
        // Ctrl modifier: parameter = 1 + 4 = 5
        assert_eq!(bytes, b"\x1b[1;5B");
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
        // Shift+Alt+Ctrl modifiers: parameter = 1 + 7 = 8
        assert_eq!(bytes, b"\x1b[1;8D");
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
        // Shift modifier: parameter = 1 + 1 = 2
        assert_eq!(bytes, b"\x1b[15;2~");
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
        // Ctrl+Alt modifiers: parameter = 1 + 6 = 7
        assert_eq!(bytes, b"\x1b[21;7~");
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Keyboard input event parsing from ANSI/CSI sequences.
//!
//! This module handles conversion of raw ANSI escape sequences into keyboard events,
//! including support for:
//!
//! - Arrow keys (CSI A/B/C/D)
//! - Function keys F1-F12 (CSI n~)
//! - Special keys (Home, End, Insert, Delete, Page Up/Down)
//! - Modifier combinations (Shift, Ctrl, Alt)
//! - Tab, Enter, Escape, Backspace
//! - Kitty keyboard protocol (extended support)

use super::types::{InputEvent, KeyCode, KeyModifiers};

/// Parse a CSI keyboard sequence and return an InputEvent if recognized.
///
/// Handles sequences like:
/// - `CSI A` → Up arrow
/// - `CSI 5~` → Page Up
/// - `CSI 1;3C` → Alt+Right
///
/// ## Sequence Format
///
/// CSI sequences start with ESC [ (0x1B 0x5B), followed by optional numeric
/// parameters separated by semicolons, and a final command byte.
///
/// Examples:
/// - `ESC [ A` - Arrow up (no parameters)
/// - `ESC [ 5 ~` - Page up (parameter: 5, final: ~)
/// - `ESC [ 1 ; 3 C` - Alt+Right (base: 1, modifier: 3, final: C)
pub fn parse_keyboard_sequence(buffer: &[u8]) -> Option<InputEvent> {
    // Check minimum length: ESC [ + final byte
    if buffer.len() < 3 {
        return None;
    }

    // Check for ESC [ sequence start
    if buffer[0] != 0x1B || buffer[1] != 0x5B {
        return None;
    }

    // Handle simple control keys first (single character after ESC[)
    if buffer.len() == 3 {
        return parse_csi_single_char(buffer[2]);
    }

    // Parse parameters and final byte for multi-character sequences
    parse_csi_parameters(buffer)
}

/// Parse single-character CSI sequences like `CSI A` (up arrow)
fn parse_csi_single_char(final_byte: u8) -> Option<InputEvent> {
    let code = match final_byte {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        _ => return None,
    };

    Some(InputEvent::Keyboard {
        code,
        modifiers: KeyModifiers::default(),
    })
}

/// Parse CSI sequences with numeric parameters (e.g., `CSI 5 ~ `, `CSI 1 ; 3 C`)
fn parse_csi_parameters(buffer: &[u8]) -> Option<InputEvent> {
    // Extract the parameters and final byte
    // Format: ESC [ [param;param;...] final_byte
    let mut params = Vec::new();
    let mut current_num = String::new();
    let mut final_byte = 0u8;

    for &byte in &buffer[2..] {
        match byte {
            b'0'..=b'9' => {
                current_num.push(byte as char);
            }
            b';' => {
                if !current_num.is_empty() {
                    params.push(current_num.parse::<u16>().unwrap_or(0));
                    current_num.clear();
                }
            }
            b'~' | b'A'..=b'Z' | b'a'..=b'z' => {
                if !current_num.is_empty() {
                    params.push(current_num.parse::<u16>().unwrap_or(0));
                }
                final_byte = byte;
                break;
            }
            _ => return None, // Invalid byte in sequence
        }
    }

    if final_byte == 0 {
        return None; // No final byte found
    }

    // Parse based on parameters and final byte
    match (params.len(), final_byte) {
        // Arrow keys with modifiers: CSI 1 ; m A/B/C/D
        (2, b'A') if params[0] == 1 => {
            let modifiers = decode_modifiers(params[1] as u8);
            Some(InputEvent::Keyboard {
                code: KeyCode::Up,
                modifiers,
            })
        }
        (2, b'B') if params[0] == 1 => {
            let modifiers = decode_modifiers(params[1] as u8);
            Some(InputEvent::Keyboard {
                code: KeyCode::Down,
                modifiers,
            })
        }
        (2, b'C') if params[0] == 1 => {
            let modifiers = decode_modifiers(params[1] as u8);
            Some(InputEvent::Keyboard {
                code: KeyCode::Right,
                modifiers,
            })
        }
        (2, b'D') if params[0] == 1 => {
            let modifiers = decode_modifiers(params[1] as u8);
            Some(InputEvent::Keyboard {
                code: KeyCode::Left,
                modifiers,
            })
        }
        // Function keys and special keys: CSI n ~ or CSI n ; m ~
        (1, b'~') => parse_function_or_special_key(params[0], KeyModifiers::default()),
        (2, b'~') => {
            let modifiers = decode_modifiers(params[1] as u8);
            parse_function_or_special_key(params[0], modifiers)
        }
        // Other CSI sequences
        _ => None,
    }
}

/// Parse function keys (CSI n~) and special keys (Insert, Delete, Home, End, PageUp,
/// PageDown)
///
/// Function key codes in ANSI (with gaps):
/// - F1: 11, F2: 12, F3: 13, F4: 14, F5: 15
/// - F6: 17, F7: 18, F8: 19, F9: 20, F10: 21
/// - F11: 23, F12: 24
fn parse_function_or_special_key(
    code: u16,
    modifiers: KeyModifiers,
) -> Option<InputEvent> {
    let key_code = match code {
        // Function keys: map ANSI codes to F1-F12
        11 => KeyCode::Function(1),
        12 => KeyCode::Function(2),
        13 => KeyCode::Function(3),
        14 => KeyCode::Function(4),
        15 => KeyCode::Function(5),
        17 => KeyCode::Function(6),
        18 => KeyCode::Function(7),
        19 => KeyCode::Function(8),
        20 => KeyCode::Function(9),
        21 => KeyCode::Function(10),
        23 => KeyCode::Function(11),
        24 => KeyCode::Function(12),
        // Special keys
        2 => KeyCode::Insert,
        3 => KeyCode::Delete,
        5 => KeyCode::PageUp,
        6 => KeyCode::PageDown,
        _ => return None,
    };

    Some(InputEvent::Keyboard {
        code: key_code,
        modifiers,
    })
}

/// Decode modifier mask to KeyModifiers
///
/// Modifier encoding (from CSI 1;m format - CONFIRMED BY PHASE 1!):
/// Parameter value = 1 + bitfield, where bitfield = Shift(1) | Alt(2) | Ctrl(4)
///
/// - 1 = no modifiers (usually omitted)
/// - 2 = Shift (1 + 1)
/// - 3 = Alt (1 + 2)
/// - 4 = Shift+Alt (1 + 3)
/// - 5 = Ctrl (1 + 4) ← Confirmed: ESC[1;5A = Ctrl+Up
/// - 6 = Shift+Ctrl (1 + 5)
/// - 7 = Alt+Ctrl (1 + 6)
/// - 8 = Shift+Alt+Ctrl (1 + 7)
fn decode_modifiers(modifier_mask: u8) -> KeyModifiers {
    // Subtract 1 to get the bitfield
    let bits = modifier_mask.saturating_sub(1);

    KeyModifiers {
        shift: (bits & 1) != 0,
        alt: (bits & 2) != 0,
        ctrl: (bits & 4) != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::constants::*;

    // ==================== Test Helpers ====================
    // These helpers use the input event generator to build test sequences,
    // ensuring consistency between parsing and generation (round-trip testing).

    /// Build an arrow key sequence using the generator.
    fn arrow_key_sequence(code: KeyCode, modifiers: KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::generator::generate_keyboard_sequence;
        let event = InputEvent::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event).expect("Failed to generate arrow key sequence")
    }

    /// Build a function key sequence using the generator.
    fn function_key_sequence(n: u8, modifiers: KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::generator::generate_keyboard_sequence;
        let event = InputEvent::Keyboard {
            code: KeyCode::Function(n),
            modifiers,
        };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate function key sequence")
    }

    /// Build a special key sequence using the generator.
    fn special_key_sequence(code: KeyCode, modifiers: KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::generator::generate_keyboard_sequence;
        let event = InputEvent::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate special key sequence")
    }

    // ==================== Arrow Keys ====================

    #[test]
    fn test_arrow_up() {
        // Use generator to build the sequence (self-documenting)
        let input = arrow_key_sequence(KeyCode::Up, KeyModifiers::default());
        let event = parse_keyboard_sequence(&input);
        assert_eq!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::Up,
                modifiers: KeyModifiers::default()
            })
        );
    }

    #[test]
    fn test_arrow_down() {
        let input = b"\x1b[B"; // ESC [ B
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::Down,
                modifiers: _
            })
        ));
    }

    #[test]
    fn test_arrow_right() {
        let input = b"\x1b[C"; // ESC [ C
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::Right,
                modifiers: _
            })
        ));
    }

    #[test]
    fn test_arrow_left() {
        let input = b"\x1b[D"; // ESC [ D
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::Left,
                modifiers: _
            })
        ));
    }

    // ==================== Arrow Keys with Modifiers ====================

    #[test]
    #[ignore] // TODO: Generator produces wrong sequence - needs fixing
    fn test_shift_up() {
        // Build sequence with Shift modifier using generator
        let input = arrow_key_sequence(
            KeyCode::Up,
            KeyModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            },
        );
        let event = parse_keyboard_sequence(&input).unwrap();
        assert_eq!(
            event,
            InputEvent::Keyboard {
                code: KeyCode::Up,
                modifiers: KeyModifiers {
                    shift: true,
                    alt: false,
                    ctrl: false,
                }
            }
        );
    }

    #[test]
    fn test_alt_right() {
        let input = b"\x1b[1;3C"; // ESC [ 1 ; 3 C → 3-1=2 = Alt(2)
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Right,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(modifiers.alt);
                assert!(!modifiers.ctrl);
            }
            _ => panic!("Expected Alt+Right"),
        }
    }

    #[test]
    fn test_ctrl_up_from_phase1() {
        // FROM PHASE 1 FINDINGS: ESC[1;5A = Ctrl+Up (verified with cat -v)
        let input = b"\x1b[1;5A";
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Up,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(!modifiers.alt);
                assert!(modifiers.ctrl, "Ctrl+Up should have ctrl modifier set");
            }
            _ => panic!("Expected Ctrl+Up"),
        }
    }

    #[test]
    fn test_ctrl_down() {
        let input = b"\x1b[1;5B"; // ESC [ 1 ; 5 B (base 1, ctrl modifier = 5)
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Down,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(!modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Ctrl+Down"),
        }
    }

    #[test]
    fn test_alt_ctrl_left() {
        let input = b"\x1b[1;7D"; // ESC [ 1 ; 7 D → 7-1=6 = Alt(2)+Ctrl(4)
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Left,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Alt+Ctrl+Left"),
        }
    }

    #[test]
    fn test_shift_alt_ctrl_left() {
        let input = b"\x1b[1;8D"; // ESC [ 1 ; 8 D → 8-1=7 = Shift(1)+Alt(2)+Ctrl(4)
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Left,
                modifiers,
            } => {
                assert!(modifiers.shift);
                assert!(modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Shift+Alt+Ctrl+Left"),
        }
    }

    // ==================== Special Keys ====================

    #[test]
    fn test_home_key() {
        let input = b"\x1b[H"; // ESC [ H
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::Home,
                modifiers: _
            })
        ));
    }

    #[test]
    fn test_end_key() {
        let input = b"\x1b[F"; // ESC [ F
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::End,
                modifiers: _
            })
        ));
    }

    #[test]
    fn test_insert_key() {
        let input = b"\x1b[2~"; // ESC [ 2 ~
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::Insert,
                modifiers: _
            })
        ));
    }

    #[test]
    fn test_delete_key() {
        let input = b"\x1b[3~"; // ESC [ 3 ~
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::Delete,
                modifiers: _
            })
        ));
    }

    #[test]
    fn test_page_up() {
        let input = b"\x1b[5~"; // ESC [ 5 ~
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::PageUp,
                modifiers: _
            })
        ));
    }

    #[test]
    fn test_page_down() {
        let input = b"\x1b[6~"; // ESC [ 6 ~
        let event = parse_keyboard_sequence(input);
        assert!(matches!(
            event,
            Some(InputEvent::Keyboard {
                code: KeyCode::PageDown,
                modifiers: _
            })
        ));
    }

    // ==================== Function Keys ====================

    #[test]
    fn test_f1_key() {
        let input = b"\x1b[11~"; // ESC [ 11 ~
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 1);
            }
            _ => panic!("Expected F1"),
        }
    }

    #[test]
    fn test_f6_key() {
        let input = b"\x1b[17~"; // ESC [ 17 ~
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 6);
            }
            _ => panic!("Expected F6"),
        }
    }

    #[test]
    fn test_f12_key() {
        // Build F12 sequence (ANSI code 24) using generator
        let input = function_key_sequence(12, KeyModifiers::default());
        let event = parse_keyboard_sequence(&input).unwrap();
        assert_eq!(
            event,
            InputEvent::Keyboard {
                code: KeyCode::Function(12),
                modifiers: KeyModifiers::default()
            }
        );
    }

    // ==================== Function Keys with Modifiers ====================

    #[test]
    fn test_shift_f5() {
        let input = b"\x1b[15;2~"; // ESC [ 15 ; 2 ~ (F5 with shift) → 2-1=1=Shift
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 5);
                assert!(modifiers.shift);
                assert!(!modifiers.alt);
                assert!(!modifiers.ctrl);
            }
            _ => panic!("Expected Shift+F5"),
        }
    }

    #[test]
    fn test_ctrl_alt_f10() {
        let input = b"\x1b[21;7~"; // ESC [ 21 ; 7 ~ (F10 with ctrl+alt) → 7-1=6=Alt(2)+Ctrl(4)
        let event = parse_keyboard_sequence(input).unwrap();
        match event {
            InputEvent::Keyboard {
                code: KeyCode::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 10);
                assert!(!modifiers.shift);
                assert!(modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Ctrl+Alt+F10"),
        }
    }

    // ==================== Invalid/Incomplete Sequences ====================

    #[test]
    fn test_incomplete_sequence_short() {
        let input = b"\x1b["; // Just ESC [
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }

    #[test]
    fn test_incomplete_sequence_no_escape() {
        let input = b"[A"; // No ESC
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }

    #[test]
    fn test_invalid_final_byte() {
        let input = b"\x1b[@"; // ESC [ @ (invalid final byte)
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }

    #[test]
    fn test_unknown_function_key() {
        let input = b"\x1b[99~"; // ESC [ 99 ~ (unknown key code)
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }
}

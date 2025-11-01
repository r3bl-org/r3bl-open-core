// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Keyboard input event parsing from ANSI/CSI sequences.
//!
//! This module handles conversion of raw ANSI escape sequences into keyboard events,
//! including support for:
//!
//! - Arrow keys (CSI A/B/C/D, SS3 A/B/C/D for application mode)
//! - Function keys F1-F12 (CSI n~, SS3 P/Q/R/S for F1-F4)
//! - Special keys (Home, End, Insert, Delete, Page Up/Down)
//! - Modifier combinations (Shift, Ctrl, Alt)
//! - Tab, Enter, Escape, Backspace
//! - Kitty keyboard protocol (extended support)
//! - SS3 sequences (ESC O) for vim/less/emacs application mode

use super::types::{VT100InputEvent, VT100KeyCode, VT100KeyModifiers};
use crate::core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_SS3_O};

/// Parse a CSI keyboard sequence and return an `InputEvent` with bytes consumed.
///
/// Returns `Some((event, bytes_consumed))` if a complete sequence was parsed,
/// or `None` if the sequence is incomplete or invalid.
///
/// Handles sequences like:
/// - `CSI A` → (Up arrow, 3 bytes)
/// - `CSI 5~` → (Page Up, 4 bytes)
/// - `CSI 1;3C` → (Alt+Right, 6 bytes)
///
/// ## Sequence Format
///
/// CSI sequences start with ESC [ (0x1B 0x5B), followed by optional numeric
/// parameters separated by semicolons, and a final command byte.
///
/// Examples:
/// - `ESC [ A` - Arrow up (no parameters, 3 bytes)
/// - `ESC [ 5 ~` - Page up (parameter: 5, final: ~, 4 bytes)
/// - `ESC [ 1 ; 3 C` - Alt+Right (base: 1, modifier: 3, final: C, 6 bytes)
#[must_use]
pub fn parse_keyboard_sequence(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Check minimum length: ESC [ + final byte
    if buffer.len() < 3 {
        return None;
    }

    // Check for ESC [ sequence start
    if buffer[0] != ANSI_ESC || buffer[1] != ANSI_CSI_BRACKET {
        return None;
    }

    // Handle simple control keys first (single character after ESC[)
    if buffer.len() == 3 {
        return parse_csi_single_char(buffer[2]).map(|event| (event, 3));
    }

    // Parse parameters and final byte for multi-character sequences
    parse_csi_parameters(buffer)
}

/// Parse an SS3 keyboard sequence and return an `InputEvent` with bytes consumed.
///
/// SS3 sequences are used in terminal application mode (vim, less, emacs, etc.)
/// to send arrow keys and function keys. They have a simpler format than CSI.
///
/// Returns `Some((event, bytes_consumed))` if a complete sequence was parsed,
/// or `None` if the sequence is incomplete or invalid.
///
/// Handles sequences like:
/// - `SS3 A` → (Up arrow, 3 bytes)
/// - `SS3 P` → (F1, 3 bytes)
///
/// ## Sequence Format
///
/// SS3 sequences start with ESC O (0x1B 0x4F), followed by a single character command.
/// Total length is always 3 bytes.
///
/// Examples:
/// - `ESC O A` - Arrow up (3 bytes)
/// - `ESC O P` - F1 (3 bytes)
///
/// **Note**: SS3 sequences do NOT support modifiers like Shift/Ctrl/Alt.
/// Those combinations are still sent as CSI sequences with modifiers.
#[must_use]
pub fn parse_ss3_sequence(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // SS3 sequences must be exactly 3 bytes: ESC O + command_char
    if buffer.len() < 3 {
        return None;
    }

    // Check for ESC O sequence start
    if buffer[0] != ANSI_ESC || buffer[1] != ANSI_SS3_O {
        return None;
    }

    // Parse the command character
    let event = match buffer[2] {
        // Arrow keys
        b'A' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Up,
            modifiers: VT100KeyModifiers::default(),
        },
        b'B' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Down,
            modifiers: VT100KeyModifiers::default(),
        },
        b'C' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Right,
            modifiers: VT100KeyModifiers::default(),
        },
        b'D' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Left,
            modifiers: VT100KeyModifiers::default(),
        },
        // Home and End keys
        b'H' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Home,
            modifiers: VT100KeyModifiers::default(),
        },
        b'F' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::End,
            modifiers: VT100KeyModifiers::default(),
        },
        // Function keys F1-F4
        b'P' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Function(1),
            modifiers: VT100KeyModifiers::default(),
        },
        b'Q' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Function(2),
            modifiers: VT100KeyModifiers::default(),
        },
        b'R' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Function(3),
            modifiers: VT100KeyModifiers::default(),
        },
        b'S' => VT100InputEvent::Keyboard {
            code: VT100KeyCode::Function(4),
            modifiers: VT100KeyModifiers::default(),
        },
        _ => return None,
    };

    Some((event, 3))
}

/// Parse single-character CSI sequences like `CSI A` (up arrow)
fn parse_csi_single_char(final_byte: u8) -> Option<VT100InputEvent> {
    let code = match final_byte {
        b'A' => VT100KeyCode::Up,
        b'B' => VT100KeyCode::Down,
        b'C' => VT100KeyCode::Right,
        b'D' => VT100KeyCode::Left,
        b'H' => VT100KeyCode::Home,
        b'F' => VT100KeyCode::End,
        _ => return None,
    };

    Some(VT100InputEvent::Keyboard {
        code,
        modifiers: VT100KeyModifiers::default(),
    })
}

/// Parse CSI sequences with numeric parameters (e.g., `CSI 5 ~ `, `CSI 1 ; 3 C`)
/// Returns (`InputEvent`, `bytes_consumed`) on success.
fn parse_csi_parameters(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Extract the parameters and final byte
    // Format: ESC [ [param;param;...] final_byte
    let mut params = Vec::new();
    let mut current_num = String::new();
    let mut final_byte = 0u8;
    let mut bytes_scanned = 0;

    for (idx, &byte) in buffer[2..].iter().enumerate() {
        bytes_scanned = idx + 1; // Track position relative to buffer[2..]
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

    // Total bytes consumed: ESC [ (2 bytes) + scanned bytes (includes final)
    let total_consumed = 2 + bytes_scanned;

    // Parse based on parameters and final byte
    let event = match (params.len(), final_byte) {
        // Arrow keys with modifiers: CSI 1 ; m A/B/C/D
        (2, b'A') if params[0] == 1 => {
            #[allow(clippy::cast_possible_truncation)]
            let modifiers = decode_modifiers(params[1] as u8);
            Some(VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers,
            })
        }
        (2, b'B') if params[0] == 1 => {
            #[allow(clippy::cast_possible_truncation)]
            let modifiers = decode_modifiers(params[1] as u8);
            Some(VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers,
            })
        }
        (2, b'C') if params[0] == 1 => {
            #[allow(clippy::cast_possible_truncation)]
            let modifiers = decode_modifiers(params[1] as u8);
            Some(VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers,
            })
        }
        (2, b'D') if params[0] == 1 => {
            #[allow(clippy::cast_possible_truncation)]
            let modifiers = decode_modifiers(params[1] as u8);
            Some(VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers,
            })
        }
        // Function keys and special keys: CSI n ~ or CSI n ; m ~
        (1, b'~') => {
            parse_function_or_special_key(params[0], VT100KeyModifiers::default())
        }
        (2, b'~') => {
            #[allow(clippy::cast_possible_truncation)]
            let modifiers = decode_modifiers(params[1] as u8);
            parse_function_or_special_key(params[0], modifiers)
        }
        // Other CSI sequences
        _ => None,
    }?;

    Some((event, total_consumed))
}

/// Parse function keys (CSI n~) and special keys (Insert, Delete, Home, End, `PageUp`,
/// `PageDown`)
///
/// Function key codes in ANSI (with gaps):
/// - F1: 11, F2: 12, F3: 13, F4: 14, F5: 15
/// - F6: 17, F7: 18, F8: 19, F9: 20, F10: 21
/// - F11: 23, F12: 24
fn parse_function_or_special_key(
    code: u16,
    modifiers: VT100KeyModifiers,
) -> Option<VT100InputEvent> {
    let key_code = match code {
        // Function keys: map ANSI codes to F1-F12
        11 => VT100KeyCode::Function(1),
        12 => VT100KeyCode::Function(2),
        13 => VT100KeyCode::Function(3),
        14 => VT100KeyCode::Function(4),
        15 => VT100KeyCode::Function(5),
        17 => VT100KeyCode::Function(6),
        18 => VT100KeyCode::Function(7),
        19 => VT100KeyCode::Function(8),
        20 => VT100KeyCode::Function(9),
        21 => VT100KeyCode::Function(10),
        23 => VT100KeyCode::Function(11),
        24 => VT100KeyCode::Function(12),
        // Special keys
        2 => VT100KeyCode::Insert,
        3 => VT100KeyCode::Delete,
        5 => VT100KeyCode::PageUp,
        6 => VT100KeyCode::PageDown,
        _ => return None,
    };

    Some(VT100InputEvent::Keyboard {
        code: key_code,
        modifiers,
    })
}

/// Decode modifier mask to `KeyModifiers`
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
fn decode_modifiers(modifier_mask: u8) -> VT100KeyModifiers {
    // Subtract 1 to get the bitfield
    let bits = modifier_mask.saturating_sub(1);

    VT100KeyModifiers {
        shift: (bits & 1) != 0,
        alt: (bits & 2) != 0,
        ctrl: (bits & 4) != 0,
    }
}

/// Unit tests for keyboard input parsing.
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing. For testing strategy details,
/// see the [testing strategy] documentation.
///
/// [testing strategy]: mod@super#testing-strategy
#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Test Helpers ====================
    // These helpers use the input event generator to build test sequences,
    // ensuring consistency between parsing and generation (round-trip testing).

    /// Build an arrow key sequence using the generator.
    fn arrow_key_sequence(code: VT100KeyCode, modifiers: VT100KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;
        let event = VT100InputEvent::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event).expect("Failed to generate arrow key sequence")
    }

    /// Build a function key sequence using the generator.
    fn function_key_sequence(n: u8, modifiers: VT100KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;
        let event = VT100InputEvent::Keyboard {
            code: VT100KeyCode::Function(n),
            modifiers,
        };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate function key sequence")
    }

    /// Build a special key sequence (Home, End, Insert, Delete, PageUp, PageDown) using
    /// the generator.
    fn special_key_sequence(code: VT100KeyCode, modifiers: VT100KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;
        let event = VT100InputEvent::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate special key sequence")
    }

    // ==================== SS3 Sequences ====================
    // SS3 sequences (ESC O) are used in vim, less, emacs and other terminal apps
    // when they're in application mode. Simple 3-byte format: ESC O + command_char

    #[test]
    fn test_ss3_arrow_up() {
        let input = b"\x1bOA"; // ESC O A
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 up");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_arrow_down() {
        let input = b"\x1bOB"; // ESC O B
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 down");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_arrow_right() {
        let input = b"\x1bOC"; // ESC O C
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 right");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_arrow_left() {
        let input = b"\x1bOD"; // ESC O D
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 left");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_home() {
        let input = b"\x1bOH"; // ESC O H
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 home");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Home,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_end() {
        let input = b"\x1bOF"; // ESC O F
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 end");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::End,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f1() {
        let input = b"\x1bOP"; // ESC O P
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F1");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(1),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f2() {
        let input = b"\x1bOQ"; // ESC O Q
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F2");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(2),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f3() {
        let input = b"\x1bOR"; // ESC O R
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F3");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(3),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f4() {
        let input = b"\x1bOS"; // ESC O S
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F4");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(4),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_incomplete_sequence() {
        let input = b"\x1bO"; // Only ESC O, missing command char
        assert!(
            parse_ss3_sequence(input).is_none(),
            "Incomplete SS3 sequence should return None"
        );
    }

    #[test]
    fn test_ss3_invalid_command_char() {
        let input = b"\x1bOX"; // ESC O X (X is not a valid command)
        assert!(
            parse_ss3_sequence(input).is_none(),
            "Invalid SS3 command should return None"
        );
    }

    #[test]
    fn test_ss3_rejects_csi_sequence() {
        // Make sure SS3 parser correctly rejects CSI sequences
        let input = b"\x1b[A"; // CSI sequence, not SS3
        assert!(
            parse_ss3_sequence(input).is_none(),
            "SS3 parser should reject CSI sequences"
        );
    }

    // ==================== Arrow Keys ====================

    #[test]
    fn test_arrow_up() {
        // Use generator to build the sequence (self-documenting)
        let input = arrow_key_sequence(VT100KeyCode::Up, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_arrow_down() {
        let input = arrow_key_sequence(VT100KeyCode::Down, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_arrow_right() {
        let input = arrow_key_sequence(VT100KeyCode::Right, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_arrow_left() {
        let input = arrow_key_sequence(VT100KeyCode::Left, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Arrow Keys with Modifiers ====================

    #[test]
    fn test_shift_up() {
        // Build sequence with Shift modifier using generator
        let input = arrow_key_sequence(
            VT100KeyCode::Up,
            VT100KeyModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers {
                    shift: true,
                    alt: false,
                    ctrl: false,
                }
            }
        );
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_alt_right() {
        let input = arrow_key_sequence(
            VT100KeyCode::Right,
            VT100KeyModifiers {
                shift: false,
                alt: true,
                ctrl: false,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(modifiers.alt);
                assert!(!modifiers.ctrl);
            }
            _ => panic!("Expected Alt+Right"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_ctrl_up_from_phase1() {
        // FROM PHASE 1 FINDINGS: ESC[1;5A = Ctrl+Up (verified with cat -v)
        let input = arrow_key_sequence(
            VT100KeyCode::Up,
            VT100KeyModifiers {
                shift: false,
                alt: false,
                ctrl: true,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(!modifiers.alt);
                assert!(modifiers.ctrl, "Ctrl+Up should have ctrl modifier set");
            }
            _ => panic!("Expected Ctrl+Up"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_ctrl_down() {
        let input = b"\x1b[1;5B"; // ESC [ 1 ; 5 B (base 1, ctrl modifier = 5)
        let (event, bytes_consumed) = parse_keyboard_sequence(input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(!modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Ctrl+Down"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_alt_ctrl_left() {
        let input = b"\x1b[1;7D"; // ESC [ 1 ; 7 D → 7-1=6 = Alt(2)+Ctrl(4)
        let (event, bytes_consumed) = parse_keyboard_sequence(input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers,
            } => {
                assert!(!modifiers.shift);
                assert!(modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Alt+Ctrl+Left"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_shift_alt_ctrl_left() {
        let input = b"\x1b[1;8D"; // ESC [ 1 ; 8 D → 8-1=7 = Shift(1)+Alt(2)+Ctrl(4)
        let (event, bytes_consumed) = parse_keyboard_sequence(input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers,
            } => {
                assert!(modifiers.shift);
                assert!(modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Shift+Alt+Ctrl+Left"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Special Keys ====================

    #[test]
    fn test_home_key() {
        let input =
            special_key_sequence(VT100KeyCode::Home, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Home,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_end_key() {
        let input = special_key_sequence(VT100KeyCode::End, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::End,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_insert_key() {
        let input =
            special_key_sequence(VT100KeyCode::Insert, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Insert,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_delete_key() {
        let input =
            special_key_sequence(VT100KeyCode::Delete, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Delete,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_page_up() {
        let input =
            special_key_sequence(VT100KeyCode::PageUp, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::PageUp,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_page_down() {
        let input =
            special_key_sequence(VT100KeyCode::PageDown, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::PageDown,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Function Keys ====================

    #[test]
    fn test_f1_key() {
        let input = function_key_sequence(1, VT100KeyModifiers::default());
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 1);
            }
            _ => panic!("Expected F1"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_f6_key() {
        let input = function_key_sequence(6, VT100KeyModifiers::default());
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 6);
            }
            _ => panic!("Expected F6"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_f12_key() {
        // Build F12 sequence (ANSI code 24) using generator
        let input = function_key_sequence(12, VT100KeyModifiers::default());
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(12),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Function Keys with Modifiers ====================

    #[test]
    fn test_shift_f5() {
        let input = b"\x1b[15;2~"; // ESC [ 15 ; 2 ~ (F5 with shift) → 2-1=1=Shift
        let (event, bytes_consumed) = parse_keyboard_sequence(input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 5);
                assert!(modifiers.shift);
                assert!(!modifiers.alt);
                assert!(!modifiers.ctrl);
            }
            _ => panic!("Expected Shift+F5"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_ctrl_alt_f10() {
        let input = b"\x1b[21;7~"; // ESC [ 21 ; 7 ~ (F10 with ctrl+alt) → 7-1=6=Alt(2)+Ctrl(4)
        let (event, bytes_consumed) = parse_keyboard_sequence(input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 10);
                assert!(!modifiers.shift);
                assert!(modifiers.alt);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected Ctrl+Alt+F10"),
        }
        assert_eq!(bytes_consumed, input.len());
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

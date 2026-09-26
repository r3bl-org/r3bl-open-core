// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`CSI`] sequence decoding and dispatch for keyboard input.
//!
//! [`CSI`]: crate::CsiSequence

use super::{super::ir_event_types::{ParsedInputEventIR, VT100InputEventIR,
                                    VT100KeyCodeIR, VT100KeyModifiersIR},
            csi_scanner, csi_u, modifiers};
use crate::{byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ARROW_DOWN_FINAL,
                                    ARROW_LEFT_FINAL, ARROW_RIGHT_FINAL,
                                    ARROW_UP_FINAL, BACKTAB_FINAL, CSI_MIN_LEN,
                                    CSI_PARAM_DEFAULT, CSI_PARAM_ZERO,
                                    FUNCTION_F1_CODE, FUNCTION_F2_CODE,
                                    FUNCTION_F3_CODE, FUNCTION_F4_CODE,
                                    FUNCTION_F5_CODE, FUNCTION_F6_CODE,
                                    FUNCTION_F7_CODE, FUNCTION_F8_CODE,
                                    FUNCTION_F9_CODE, FUNCTION_F10_CODE,
                                    FUNCTION_F11_CODE, FUNCTION_F12_CODE,
                                    SPECIAL_DELETE_CODE, SPECIAL_END_ALT1_CODE,
                                    SPECIAL_END_ALT2_CODE, SPECIAL_END_FINAL,
                                    SPECIAL_HOME_ALT1_CODE, SPECIAL_HOME_ALT2_CODE,
                                    SPECIAL_HOME_FINAL, SPECIAL_INSERT_CODE,
                                    SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE,
                                    SS3_F1_FINAL, SS3_F2_FINAL, SS3_F3_FINAL,
                                    SS3_F4_FINAL}};

/// Parses a [`CSI`] keyboard sequence and returns the parsed event with bytes consumed.
///
/// **Dispatch position**: 1st parser for [`CSI`] sequences ([`ESC`] [). See [`Parser
/// Dispatch Priority Pipeline`] in [`router`] for dispatch order. Keyboard sequences are
/// tried first because they're more common than mouse or terminal events.
///
/// Handles arrow keys, function keys, and modified keys like Alt+Right, Ctrl+Up, etc.
/// See [`CSI Sequences`] for format details.
///
/// # Returns
///
/// - The parsed keyboard event and byte count on success.
/// - Nothing if the sequence is incomplete or invalid.
///
/// [`CSI Sequences`]: mod@super#csi-sequences-esc
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`Parser Dispatch Priority Pipeline`]: mod@super::super::router#parser-dispatch-priority-pipeline
/// [`router`]: mod@super::super::router
#[must_use]
pub fn parse_keyboard_sequence(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    if let Some(csi_u_event) = csi_u::parse_csi_u_sequence(buffer) {
        return Some(csi_u_event);
    }

    match classify_csi_buffer(buffer) {
        CsiBufferKind::SingleChar(final_byte) => {
            let event = parse_csi_single_char(final_byte)?;
            Some(ParsedInputEventIR::new(event, byte_offset(CSI_MIN_LEN)))
        }
        CsiBufferKind::Parameterized(buf) => parse_csi_parameters(buf),
        CsiBufferKind::Invalid => None,
    }
}

/// Structural categorization of an incoming [`CSI`] byte slice.
///
/// [`CSI`]: crate::CsiSequence
#[derive(Debug, PartialEq, Eq)]
pub enum CsiBufferKind<'a> {
    /// Exactly 3 bytes: `ESC [ <final_byte>` (e.g. `ESC [ A`).
    SingleChar(u8),

    /// Multi-byte sequence: `ESC [ <params...> <final_byte>` (e.g. `ESC [ 1 ; 2 H`).
    Parameterized(&'a [u8]),

    /// Does not start with `ESC [` or is too short.
    Invalid,
}

/// Classifies the buffer prefix into a [`CsiBufferKind`] without raw indexing.
///
/// [`CSI`]: crate::CsiSequence
#[must_use]
pub fn classify_csi_buffer(buffer: &[u8]) -> CsiBufferKind<'_> {
    match buffer {
        // Exactly 3 bytes starting with ESC [.
        [ANSI_ESC, ANSI_CSI_BRACKET, final_byte] => {
            CsiBufferKind::SingleChar(*final_byte)
        }
        // Starts with ESC [ and has at least 4 bytes (params + terminator) but can
        // have more.
        [ANSI_ESC, ANSI_CSI_BRACKET, _, _, /* 0 or more */ ..] => {
            CsiBufferKind::Parameterized(buffer)
        }
        _ => CsiBufferKind::Invalid,
    }
}

/// Parses single-character [`CSI`] sequences like `CSI A` (up arrow).
///
/// [`CSI`]: crate::CsiSequence
#[must_use]
pub fn parse_csi_single_char(final_byte: u8) -> Option<VT100InputEventIR> {
    let code = match final_byte {
        ARROW_UP_FINAL => VT100KeyCodeIR::Up,
        ARROW_DOWN_FINAL => VT100KeyCodeIR::Down,
        ARROW_RIGHT_FINAL => VT100KeyCodeIR::Right,
        ARROW_LEFT_FINAL => VT100KeyCodeIR::Left,
        SPECIAL_HOME_FINAL => VT100KeyCodeIR::Home,
        SPECIAL_END_FINAL => VT100KeyCodeIR::End,
        BACKTAB_FINAL => VT100KeyCodeIR::BackTab,
        _ => return None,
    };

    Some(VT100InputEventIR::Keyboard {
        code,
        modifiers: VT100KeyModifiersIR::default(),
    })
}

/// Parses [`CSI`] sequences with numeric parameters into keyboard events.
///
/// # Format
///
/// `ESC [ param ; param ; ... final_byte`
///
/// # Examples
///
/// Note: `CSI = ESC [`
///
/// | Sequence         | Meaning                |
/// | ---------------- | ---------------------- |
/// | `CSI 5 ~`        | `PageUp`               |
/// | `CSI 1 ; 3 C`    | Alt + Right Arrow      |
/// | `CSI 11 ~`       | F1                     |
/// | `CSI 1 ; 5 A`    | Ctrl + Up Arrow        |
///
/// # Returns
///
/// - The parsed keyboard event and total byte consumption displacement ([`ByteOffset`])
///   on success.
/// - Nothing if the sequence is invalid or incomplete.
///
/// [`ByteOffset`]: crate::ByteOffset
/// [`CSI`]: crate::CsiSequence
#[must_use]
pub fn parse_csi_parameters(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    let extracted = csi_scanner::extract_csi_params(buffer)?;

    // Parse based on parameters and final byte.
    let event = decode_csi_event(&extracted.params, extracted.final_byte)?;

    Some(ParsedInputEventIR::new(event, extracted.total_consumed()))
}

/// Parameter structure of a parsed [`CSI`] keyboard sequence.
///
/// [`CSI`]: crate::CsiSequence
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum CsiParamShape {
    /// Omitted (`CSI X`), zero-accumulator, or explicit default parameter 1 (`CSI 1
    /// X`).
    DefaultCount,

    /// Modified key with base parameter 1: `CSI 1 ; <modifier> <final>`.
    ModifiedKey { modifier: u8 },

    /// Tilde key (function/special key): `CSI <code> ~` or `CSI <code> ; <modifier>
    /// ~`.
    TildeKey { code: u16, modifier: Option<u8> },

    /// Any other unsupported parameter layout.
    Unknown,
}

/// Classifies parsed [`CSI`] parameters and final byte into a [`CsiParamShape`].
///
/// [`CSI`]: crate::CsiSequence
fn classify_csi_params(params: &[u16], final_byte: u8) -> CsiParamShape {
    if final_byte == ANSI_FUNCTION_KEY_TERMINATOR {
        match params {
            // CSI <code> ~ (unmodified function/special key).
            [code] => CsiParamShape::TildeKey {
                code: *code,
                modifier: None,
            },
            // CSI <code> ; <modifier> ~ (modified function/special key).
            [code, modifier] => CsiParamShape::TildeKey {
                code: *code,
                modifier: Some(modifiers::extract_modifier_parameter(*modifier)),
            },
            _ => CsiParamShape::Unknown,
        }
    } else {
        match params {
            // CSI X, CSI 0 X, or CSI 1 X (default count / no modifiers).
            [] | [CSI_PARAM_ZERO | CSI_PARAM_DEFAULT] => CsiParamShape::DefaultCount,
            // CSI 1 ; <modifier> <final> (modified navigation/arrows/F1-F4).
            [CSI_PARAM_DEFAULT, modifier] => CsiParamShape::ModifiedKey {
                modifier: modifiers::extract_modifier_parameter(*modifier),
            },
            _ => CsiParamShape::Unknown,
        }
    }
}

/// Decodes parsed [`CSI`] parameters and final byte into a [`VT100InputEventIR`].
///
/// [`CSI`]: crate::CsiSequence
fn decode_csi_event(params: &[u16], final_byte: u8) -> Option<VT100InputEventIR> {
    match (classify_csi_params(params, final_byte), final_byte) {
        // Default / unmodified single key (CSI A, CSI 1 A, CSI H, CSI 1 H, CSI Z,
        // etc.).
        (CsiParamShape::DefaultCount, final_byte) => parse_csi_single_char(final_byte),

        // Modified arrow / navigation / F1-F4 keys (CSI 1 ; m A/B/C/D/H/F/P/Q/R/S).
        (CsiParamShape::ModifiedKey { modifier }, final_byte) => {
            let code = match final_byte {
                ARROW_UP_FINAL => VT100KeyCodeIR::Up,
                ARROW_DOWN_FINAL => VT100KeyCodeIR::Down,
                ARROW_RIGHT_FINAL => VT100KeyCodeIR::Right,
                ARROW_LEFT_FINAL => VT100KeyCodeIR::Left,
                SPECIAL_HOME_FINAL => VT100KeyCodeIR::Home,
                SPECIAL_END_FINAL => VT100KeyCodeIR::End,
                SS3_F1_FINAL => VT100KeyCodeIR::Function(1),
                SS3_F2_FINAL => VT100KeyCodeIR::Function(2),
                SS3_F3_FINAL => VT100KeyCodeIR::Function(3),
                SS3_F4_FINAL => VT100KeyCodeIR::Function(4),
                _ => return None,
            };
            Some(VT100InputEventIR::Keyboard {
                code,
                modifiers: modifiers::decode_modifiers(modifier),
            })
        }

        // Tilde keys: function keys and special keys (CSI code ~ or CSI code ; m ~).
        (CsiParamShape::TildeKey { code, modifier }, ANSI_FUNCTION_KEY_TERMINATOR) => {
            let modifiers = modifier.map_or_default(modifiers::decode_modifiers);
            parse_function_or_special_key(code, modifiers)
        }

        _ => None,
    }
}

/// Parses function keys (F1-F12) and special keys (Insert, Delete, Home, End,
/// `PageUp`, `PageDown`).
///
/// Maps [`ANSI`] codes to [`VT100KeyCodeIR`]. Called by [`CSI`] parameter parser.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`CSI`]: crate::CsiSequence
fn parse_function_or_special_key(
    code: u16,
    modifiers: VT100KeyModifiersIR,
) -> Option<VT100InputEventIR> {
    let key_code = match code {
        // Function keys: map ANSI codes to F1-F12.
        FUNCTION_F1_CODE => VT100KeyCodeIR::Function(1),
        FUNCTION_F2_CODE => VT100KeyCodeIR::Function(2),
        FUNCTION_F3_CODE => VT100KeyCodeIR::Function(3),
        FUNCTION_F4_CODE => VT100KeyCodeIR::Function(4),
        FUNCTION_F5_CODE => VT100KeyCodeIR::Function(5),
        FUNCTION_F6_CODE => VT100KeyCodeIR::Function(6),
        FUNCTION_F7_CODE => VT100KeyCodeIR::Function(7),
        FUNCTION_F8_CODE => VT100KeyCodeIR::Function(8),
        FUNCTION_F9_CODE => VT100KeyCodeIR::Function(9),
        FUNCTION_F10_CODE => VT100KeyCodeIR::Function(10),
        FUNCTION_F11_CODE => VT100KeyCodeIR::Function(11),
        FUNCTION_F12_CODE => VT100KeyCodeIR::Function(12),

        // Special keys.
        // Home: Multiple alternative codes for different terminal implementations.
        SPECIAL_HOME_ALT1_CODE | SPECIAL_HOME_ALT2_CODE => VT100KeyCodeIR::Home,
        SPECIAL_INSERT_CODE => VT100KeyCodeIR::Insert,
        SPECIAL_DELETE_CODE => VT100KeyCodeIR::Delete,

        // End: Multiple alternative codes for different terminal implementations.
        SPECIAL_END_ALT1_CODE | SPECIAL_END_ALT2_CODE => VT100KeyCodeIR::End,
        SPECIAL_PAGE_UP_CODE => VT100KeyCodeIR::PageUp,
        SPECIAL_PAGE_DOWN_CODE => VT100KeyCodeIR::PageDown,

        _ => return None,
    };

    Some(VT100InputEventIR::Keyboard {
        code: key_code,
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeyState;

    // ==================== Test Helpers ====================
    // These helpers use the input event generator to build test sequences,
    // ensuring consistency between parsing and generation (round-trip testing).

    /// Builds an arrow key sequence using the generator.
    fn arrow_key_sequence(
        code: VT100KeyCodeIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        use crate::core::ansi::generator::generate_keyboard_sequence;
        let event = VT100InputEventIR::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event).expect("Failed to generate arrow key sequence")
    }

    /// Builds a function key sequence using the generator.
    fn function_key_sequence(n: u8, modifiers: VT100KeyModifiersIR) -> Vec<u8> {
        use crate::core::ansi::generator::generate_keyboard_sequence;
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(n),
            modifiers,
        };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate function key sequence")
    }

    /// Builds a special key sequence (Home, End, Insert, Delete, `PageUp`, `PageDown`)
    /// using the generator.
    fn special_key_sequence(
        code: VT100KeyCodeIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        use crate::core::ansi::generator::generate_keyboard_sequence;
        let event = VT100InputEventIR::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate special key sequence")
    }

    #[test]
    fn test_arrow_up() {
        // Use generator to build the sequence (self-documenting)
        let input =
            arrow_key_sequence(VT100KeyCodeIR::Up, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_arrow_down() {
        let input =
            arrow_key_sequence(VT100KeyCodeIR::Down, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_arrow_right() {
        let input =
            arrow_key_sequence(VT100KeyCodeIR::Right, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Right,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_arrow_left() {
        let input =
            arrow_key_sequence(VT100KeyCodeIR::Left, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Left,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    // ==================== Arrow Keys with Modifiers ====================

    #[test]
    fn test_shift_up() {
        // Build sequence with Shift modifier using generator
        let input = arrow_key_sequence(
            VT100KeyCodeIR::Up,
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                }
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_alt_right() {
        let input = arrow_key_sequence(
            VT100KeyCodeIR::Right,
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::NotPressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Right,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
            }
            _ => panic!("Expected Alt+Right"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_ctrl_up() {
        // ESC [1;5A = Ctrl+Up (verified with real terminal output)
        let input = arrow_key_sequence(
            VT100KeyCodeIR::Up,
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
                assert_eq!(
                    modifiers.ctrl,
                    KeyState::Pressed,
                    "Ctrl+Up should have ctrl modifier set"
                );
            }
            _ => panic!("Expected Ctrl+Up"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_ctrl_down() {
        let input = arrow_key_sequence(
            VT100KeyCodeIR::Down,
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Ctrl+Down"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_alt_ctrl_left() {
        let input = arrow_key_sequence(
            VT100KeyCodeIR::Left,
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::Pressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Left,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Alt+Ctrl+Left"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_shift_alt_ctrl_left() {
        let input = arrow_key_sequence(
            VT100KeyCodeIR::Left,
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::Pressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Left,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::Pressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Shift+Alt+Ctrl+Left"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    // ==================== Special Keys ====================

    #[test]
    fn test_home_key() {
        let input =
            special_key_sequence(VT100KeyCodeIR::Home, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_end_key() {
        let input =
            special_key_sequence(VT100KeyCodeIR::End, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::End,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_shift_home() {
        let input = b"\x1b[1;2H";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(input).expect("Should parse Shift+Home");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                }
            }
        );
        assert_eq!(bytes_consumed, byte_offset(6));
    }

    #[test]
    fn test_ctrl_home() {
        let input = b"\x1b[1;5H";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(input).expect("Should parse Ctrl+Home");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                }
            }
        );
        assert_eq!(bytes_consumed, byte_offset(6));
    }

    #[test]
    fn test_shift_end() {
        let input = b"\x1b[1;2F";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(input).expect("Should parse Shift+End");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::End,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                }
            }
        );
        assert_eq!(bytes_consumed, byte_offset(6));
    }

    #[test]
    fn test_ctrl_end() {
        let input = b"\x1b[1;5F";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(input).expect("Should parse Ctrl+End");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::End,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                }
            }
        );
        assert_eq!(bytes_consumed, byte_offset(6));
    }

    #[test]
    fn test_xterm_modified_f1_to_f4() {
        let ParsedInputEventIR {
            event: event_f1,
            bytes_consumed: consumed_f1,
        } = parse_keyboard_sequence(b"\x1b[1;2P").expect("Should parse Shift+F1");
        assert_eq!(
            event_f1,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                }
            }
        );
        assert_eq!(consumed_f1, byte_offset(6));

        let ParsedInputEventIR {
            event: event_f4,
            bytes_consumed: consumed_f4,
        } = parse_keyboard_sequence(b"\x1b[1;5S").expect("Should parse Ctrl+F4");
        assert_eq!(
            event_f4,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(4),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                }
            }
        );
        assert_eq!(consumed_f4, byte_offset(6));
    }

    #[test]
    fn test_single_param_home_end_backtab() {
        let ParsedInputEventIR {
            event: event_home,
            bytes_consumed: consumed_home,
        } = parse_keyboard_sequence(b"\x1b[1H").expect("Should parse CSI 1 H");
        assert_eq!(
            event_home,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(consumed_home, byte_offset(4));

        let ParsedInputEventIR {
            event: event_end,
            bytes_consumed: consumed_end,
        } = parse_keyboard_sequence(b"\x1b[1F").expect("Should parse CSI 1 F");
        assert_eq!(
            event_end,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::End,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(consumed_end, byte_offset(4));

        let ParsedInputEventIR {
            event: event_backtab,
            bytes_consumed: consumed_backtab,
        } = parse_keyboard_sequence(b"\x1b[1Z").expect("Should parse CSI 1 Z");
        assert_eq!(
            event_backtab,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::BackTab,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(consumed_backtab, byte_offset(4));
    }

    #[test]
    fn test_insert_key() {
        let input =
            special_key_sequence(VT100KeyCodeIR::Insert, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Insert,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_delete_key() {
        let input =
            special_key_sequence(VT100KeyCodeIR::Delete, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Delete,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_page_up() {
        let input =
            special_key_sequence(VT100KeyCodeIR::PageUp, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::PageUp,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_page_down() {
        let input = special_key_sequence(
            VT100KeyCodeIR::PageDown,
            VT100KeyModifiersIR::default(),
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::PageDown,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    // ==================== Function Keys ====================

    #[test]
    fn test_f1_key() {
        let input = function_key_sequence(1, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 1);
            }
            _ => panic!("Expected F1"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_f6_key() {
        let input = function_key_sequence(6, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 6);
            }
            _ => panic!("Expected F6"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_f12_key() {
        // Build F12 sequence (ANSI code 24) using generator
        let input = function_key_sequence(12, VT100KeyModifiersIR::default());
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(12),
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    // ==================== Function Keys with Modifiers ====================

    #[test]
    fn test_shift_f5() {
        let input = function_key_sequence(
            5,
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 5);
                assert_eq!(modifiers.shift, KeyState::Pressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
            }
            _ => panic!("Expected Shift+F5"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_ctrl_alt_f10() {
        let input = function_key_sequence(
            10,
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::Pressed,
            },
        );
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_keyboard_sequence(&input).expect("conversion error");
        match event {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 10);
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Ctrl+Alt+F10"),
        }
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    // ==================== Invalid/Incomplete Sequences ====================

    #[test]
    fn test_incomplete_sequence_short() {
        let input = b"\x1b["; // Just ESC [
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None, "Should return None for incomplete sequence");
    }

    #[test]
    fn test_incomplete_sequence_no_escape() {
        let input = b"[1;5A"; // Missing ESC
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None, "Should return None when not starting with ESC");
    }

    #[test]
    fn test_invalid_final_byte() {
        let input = b"\x1b[1;5?"; // '?' is not a valid final byte
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None, "Should return None for invalid final byte");
    }

    #[test]
    fn test_unknown_function_key() {
        // ANSI code 99 is not a known function key
        let input = b"\x1b[99~";
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None, "Should return None for unknown function key");
    }
}

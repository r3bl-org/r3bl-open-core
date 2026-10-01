// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Alt+key parsing ([`ESC`] followed by printable [`ASCII`] or DEL).
//!
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`ESC`]: crate::EscSequence

use super::super::ir_event_types::{ParsedInputEventIR, VT100InputEventIR,
                                   VT100KeyCodeIR, VT100KeyModifiersIR};
use crate::{KeyState, byte_offset,
            core::ansi::constants::{ANSI_ESC, ASCII_DEL, PRINTABLE_ASCII_MAX,
                                    PRINTABLE_ASCII_MIN}};

/// Parse Alt+key combination ([`ESC`] followed by printable [`ASCII`] or DEL).
///
/// **Dispatch position**: Only parser for [`ESC`] + unknown byte. See [`Parser Dispatch
/// Priority Pipeline`] in [`router`] for dispatch order.
///
/// Terminals send Alt+key as [`ESC`] (`0x1B`) + key byte. This parses two-byte sequences
/// like Alt+B → (`0x1B`, `0x62`) or Alt+Backspace → (`0x1B`, `0x7F`).
///
/// For design rationale on why Alt uses [`ESC`] prefix vs [`CSI`] sequences, see module
/// docs [`Why Alt Uses ESC Prefix`].
///
/// # Returns
///
/// - The parsed Alt+key event and byte count (always 2) on success.
/// - Nothing if the buffer doesn't start with [`ESC`] + printable [`ASCII`] or DEL.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`Parser Dispatch Priority Pipeline`]: mod@super::super::router#parser-dispatch-priority-pipeline
/// [`router`]: mod@super::super::router
/// [`Why Alt Uses ESC Prefix`]: mod@super#why-alt-uses-esc-prefix-not-csi
#[must_use]
pub fn parse_alt_letter(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    match buffer {
        [ANSI_ESC, ASCII_DEL, ..] => {
            // Handle Alt+Backspace (ESC + DEL).
            Some(ParsedInputEventIR::new(
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Backspace,
                    modifiers: VT100KeyModifiersIR {
                        shift: KeyState::NotPressed,
                        ctrl: KeyState::NotPressed,
                        alt: KeyState::Pressed,
                    },
                },
                byte_offset(2), // Consume both ESC and DEL.
            ))
        }
        [
            ANSI_ESC,
            second @ PRINTABLE_ASCII_MIN..=PRINTABLE_ASCII_MAX,
            ..,
        ] => {
            // Second byte is printable ASCII (space through ~).
            // Range: 0x20 (space) to 0x7E (~).
            let ch = char::from(*second);

            Some(ParsedInputEventIR::new(
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Char(ch),
                    modifiers: VT100KeyModifiersIR {
                        shift: KeyState::NotPressed,
                        ctrl: KeyState::NotPressed,
                        alt: KeyState::Pressed,
                    },
                },
                byte_offset(2), // Consume both ESC and letter.
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alt_letter_b() {
        let input = &[ANSI_ESC, b'b']; // ESC b → Alt+b
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_alt_letter(input).expect("Should parse Alt+b");
        match event {
            VT100InputEventIR::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCodeIR::Char('b'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, byte_offset(2));
    }

    #[test]
    fn test_alt_letter_f() {
        let input = &[ANSI_ESC, b'f']; // `ESC f → Alt+f`
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_alt_letter(input).expect("Should parse Alt+f");
        match event {
            VT100InputEventIR::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCodeIR::Char('f'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, byte_offset(2));
    }

    #[test]
    fn test_alt_letter_uppercase() {
        let input = &[ANSI_ESC, b'B']; // ESC B → Alt+B (uppercase)
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_alt_letter(input).expect("Should parse Alt+B");
        match event {
            VT100InputEventIR::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCodeIR::Char('B'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, byte_offset(2));
    }

    #[test]
    fn test_alt_digit() {
        let input = &[ANSI_ESC, b'3']; // ESC 3 → Alt+3
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_alt_letter(input).expect("Should parse Alt+3");
        match event {
            VT100InputEventIR::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCodeIR::Char('3'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, byte_offset(2));
    }

    #[test]
    fn test_alt_space() {
        let input = &[ANSI_ESC, b' ']; // ESC space → Alt+space
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_alt_letter(input).expect("Should parse Alt+space");
        match event {
            VT100InputEventIR::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCodeIR::Char(' '));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, byte_offset(2));
    }

    #[test]
    fn test_alt_backspace() {
        let input = &[ANSI_ESC, ASCII_DEL]; // ESC DEL → Alt+Backspace
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_alt_letter(input).expect("Should parse Alt+Backspace");
        match event {
            VT100InputEventIR::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCodeIR::Backspace);
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, byte_offset(2));
    }

    #[test]
    fn test_alt_letter_incomplete() {
        let input = &[ANSI_ESC]; // Just ESC, no second byte
        let event = parse_alt_letter(input);
        assert_eq!(event, None, "Should return None for incomplete sequence");
    }

    #[test]
    fn test_alt_letter_not_esc() {
        let input = b"Ab"; // 'A' 'b' (not ESC prefix)
        let event = parse_alt_letter(input);
        assert_eq!(event, None, "Should return None when first byte is not ESC");
    }

    #[test]
    fn test_alt_letter_control_char() {
        let input = &[ANSI_ESC, 0x01]; // ESC Ctrl+A (0x01 is control char)
        let event = parse_alt_letter(input);
        assert_eq!(
            event, None,
            "Should return None for control characters (below 0x20)"
        );
    }

    #[test]
    fn test_alt_letter_above_del() {
        let input = &[ANSI_ESC, 0x80]; // ESC + 0x80 (above DEL)
        let event = parse_alt_letter(input);
        assert_eq!(event, None, "Should return None for bytes above DEL (0x7F)");
    }
}

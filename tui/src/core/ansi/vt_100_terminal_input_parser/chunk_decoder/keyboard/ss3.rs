// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! SS3 sequence parsing ([`ESC`] O + command byte).
//!
//! [`ESC`]: crate::EscSequence

use super::super::super::ir_event_types::{ParsedInputEventIR, VT100InputEventIR,
                                          VT100KeyCodeIR, VT100KeyModifiersIR};
use crate::{byte_offset,
            core::ansi::constants::{ANSI_ESC, ANSI_SS3_O, ARROW_DOWN_FINAL,
                                    ARROW_LEFT_FINAL, ARROW_RIGHT_FINAL,
                                    ARROW_UP_FINAL, SPECIAL_END_FINAL,
                                    SPECIAL_HOME_FINAL, SS3_F1_FINAL, SS3_F2_FINAL,
                                    SS3_F3_FINAL, SS3_F4_FINAL, SS3_NUMPAD_0,
                                    SS3_NUMPAD_1, SS3_NUMPAD_2, SS3_NUMPAD_3,
                                    SS3_NUMPAD_4, SS3_NUMPAD_5, SS3_NUMPAD_6,
                                    SS3_NUMPAD_7, SS3_NUMPAD_8, SS3_NUMPAD_9,
                                    SS3_NUMPAD_COMMA, SS3_NUMPAD_DECIMAL,
                                    SS3_NUMPAD_DIVIDE, SS3_NUMPAD_ENTER,
                                    SS3_NUMPAD_MINUS, SS3_NUMPAD_MULTIPLY,
                                    SS3_NUMPAD_PLUS, SS3_SEQ_LEN}};

/// Parses an SS3 keyboard sequence.
///
/// **Dispatch position**: Only parser for SS3 sequences ([`ESC`] O). See [`Parser
/// Dispatch Priority Pipeline`] in [`chunk_decoder`] for dispatch order.
///
/// SS3 sequences ([`ESC`] O + single char) are used in terminal application mode (vim,
/// less, emacs) for arrow keys, function keys (F1-F4), Home, End, and numpad keys. Always
/// 3 bytes. See [`SS3 Sequences`] for format details.
///
/// **Note**: SS3 sequences do NOT support modifiers. Modified arrow keys use [`CSI`]
/// format.
///
/// # Returns
///
/// - `Some(ParsedInputEventIR)` on success.
/// - `None` if the sequence is incomplete or invalid.
///
/// [`chunk_decoder`]: mod@super::super
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`Parser Dispatch Priority Pipeline`]:
///     mod@super::super#parser-dispatch-priority-pipeline
/// [`SS3 Sequences`]: mod@super#ss3-sequences-esc-o
#[must_use]
pub fn parse_ss3_sequence(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    let code = Ss3BufferKind::classify(buffer).decode()?;
    Some(ParsedInputEventIR::new(
        VT100InputEventIR::Keyboard {
            code,
            modifiers: VT100KeyModifiersIR::default(),
        },
        byte_offset(SS3_SEQ_LEN),
    ))
}

/// Structural categorization of an incoming [`SS3`] byte slice.
///
/// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Ss3BufferKind {
    /// Starts with `ESC O` and contains a command byte.
    Command(u8),
    /// Does not start with `ESC O` or is shorter than [`SS3_SEQ_LEN`].
    Invalid,
}

impl Ss3BufferKind {
    /// Classifies the buffer prefix and extracts the command character without raw
    /// indexing.
    #[must_use]
    pub fn classify(buffer: &[u8]) -> Self {
        match buffer {
            // Starts with ESC O and has a command byte, i.e., at least 3 bytes, but it
            // can have more trailing stream buffer bytes.
            [ANSI_ESC, ANSI_SS3_O, command_char, /* 0 or more */ ..] => {
                Self::Command(*command_char)
            }
            _ => Self::Invalid,
        }
    }

    /// Decodes this [`Ss3BufferKind`] into a [`VT100KeyCodeIR`].
    #[must_use]
    pub fn decode(self) -> Option<VT100KeyCodeIR> {
        match self {
            Self::Command(command_byte) => Self::parse_command(command_byte),
            Self::Invalid => None,
        }
    }

    /// Parse SS3 command character and return the corresponding [`VT100KeyCodeIR`].
    #[must_use]
    pub fn parse_command(byte: u8) -> Option<VT100KeyCodeIR> {
        match byte {
            // Arrow keys.
            ARROW_UP_FINAL => Some(VT100KeyCodeIR::Up),
            ARROW_DOWN_FINAL => Some(VT100KeyCodeIR::Down),
            ARROW_RIGHT_FINAL => Some(VT100KeyCodeIR::Right),
            ARROW_LEFT_FINAL => Some(VT100KeyCodeIR::Left),

            // Home and End keys.
            SPECIAL_HOME_FINAL => Some(VT100KeyCodeIR::Home),
            SPECIAL_END_FINAL => Some(VT100KeyCodeIR::End),

            // Function keys F1-F4 (SS3 mode).
            SS3_F1_FINAL => Some(VT100KeyCodeIR::Function(1)),
            SS3_F2_FINAL => Some(VT100KeyCodeIR::Function(2)),
            SS3_F3_FINAL => Some(VT100KeyCodeIR::Function(3)),
            SS3_F4_FINAL => Some(VT100KeyCodeIR::Function(4)),

            // Numpad keys in application mode.
            // Note: These send SS3 sequences instead of literal digits to allow
            // applications to distinguish numpad from regular number keys.
            SS3_NUMPAD_0 => Some(VT100KeyCodeIR::Char('0')),
            SS3_NUMPAD_1 => Some(VT100KeyCodeIR::Char('1')),
            SS3_NUMPAD_2 => Some(VT100KeyCodeIR::Char('2')),
            SS3_NUMPAD_3 => Some(VT100KeyCodeIR::Char('3')),
            SS3_NUMPAD_4 => Some(VT100KeyCodeIR::Char('4')),
            SS3_NUMPAD_5 => Some(VT100KeyCodeIR::Char('5')),
            SS3_NUMPAD_6 => Some(VT100KeyCodeIR::Char('6')),
            SS3_NUMPAD_7 => Some(VT100KeyCodeIR::Char('7')),
            SS3_NUMPAD_8 => Some(VT100KeyCodeIR::Char('8')),
            SS3_NUMPAD_9 => Some(VT100KeyCodeIR::Char('9')),

            // Numpad operators and special keys.
            SS3_NUMPAD_ENTER => Some(VT100KeyCodeIR::Enter),
            SS3_NUMPAD_PLUS => Some(VT100KeyCodeIR::Char('+')),
            SS3_NUMPAD_MINUS => Some(VT100KeyCodeIR::Char('-')),
            SS3_NUMPAD_MULTIPLY => Some(VT100KeyCodeIR::Char('*')),
            SS3_NUMPAD_DIVIDE => Some(VT100KeyCodeIR::Char('/')),
            SS3_NUMPAD_DECIMAL => Some(VT100KeyCodeIR::Char('.')),
            SS3_NUMPAD_COMMA => Some(VT100KeyCodeIR::Char(',')),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ss3_arrow_up() {
        let input = b"\x1bOA"; // ESC O A
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 up");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_arrow_down() {
        let input = b"\x1bOB"; // ESC O B
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 down");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_arrow_right() {
        let input = b"\x1bOC"; // ESC O C
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 right");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Right,
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_arrow_left() {
        let input = b"\x1bOD"; // ESC O D
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 left");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Left,
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_home() {
        let input = b"\x1bOH"; // ESC O H
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 home");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_end() {
        let input = b"\x1bOF"; // ESC O F
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 end");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::End,
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_f1() {
        let input = b"\x1bOP"; // ESC O P
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 F1");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_f2() {
        let input = b"\x1bOQ"; // ESC O Q
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 F2");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(2),
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_f3() {
        let input = b"\x1bOR"; // ESC O R
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 F3");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(3),
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_f4() {
        let input = b"\x1bOS"; // ESC O S
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_ss3_sequence(input).expect("Should parse SS3 F4");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(4),
                modifiers: VT100KeyModifiersIR::default()
            }
        );
        assert_eq!(bytes_consumed, byte_offset(SS3_SEQ_LEN));
    }

    #[test]
    fn test_ss3_numpad() {
        assert_eq!(
            Ss3BufferKind::parse_command(SS3_NUMPAD_0),
            Some(VT100KeyCodeIR::Char('0'))
        );
        assert_eq!(
            Ss3BufferKind::parse_command(SS3_NUMPAD_ENTER),
            Some(VT100KeyCodeIR::Enter)
        );
        assert_eq!(
            Ss3BufferKind::parse_command(SS3_NUMPAD_PLUS),
            Some(VT100KeyCodeIR::Char('+'))
        );
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
}

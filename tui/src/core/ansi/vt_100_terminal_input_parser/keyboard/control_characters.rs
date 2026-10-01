// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control character parsing ([`ASCII`] `0x00`-`0x1F` and DEL `0x7F`).
//!
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII

use super::super::ir_event_types::{ParsedInputEventIR, VT100InputEventIR,
                                   VT100KeyCodeIR, VT100KeyModifiersIR};
use crate::{SPACE_CHAR, byte_offset,
            core::ansi::constants::{ASCII_DEL, CONTROL_BACKSPACE, CONTROL_ENTER,
                                    CONTROL_ESC, CONTROL_LF, CONTROL_NUL, CONTROL_TAB,
                                    CTRL_CHAR_RANGE_MAX, CTRL_TO_LOWERCASE_MASK}};

/// Parse a control character (bytes `0x00`-`0x1F`) and convert to a Ctrl+key event.
///
/// **Dispatch position**: 3rd parser in non-[`ESC`] priority. Must be tried before
/// [`UTF-8`] text because control bytes are valid [`UTF-8`] but represent Ctrl+letter
/// combinations.
///
/// See [`Parser Dispatch Priority Pipeline`] in [`router`] for dispatch order and
/// [`Control Key Combinations`] for complete byte mappings. Note: some bytes are treated
/// as dedicated keys (Tab, Enter, Backspace, Escape) - see [`Ambiguous Control Character
/// Handling`] for details.
///
/// # Returns
///
/// - The parsed control key event and byte count (always 1) on success.
/// - Nothing if the byte is not a control character.
///
/// [`Ambiguous Control Character Handling`]:
///     mod@super#ambiguous-control-character-handling
/// [`Control Key Combinations`]: mod@super#control-key-combinations-ctrlletter
/// [`ESC`]: crate::EscSequence
/// [`Parser Dispatch Priority Pipeline`]: mod@super::super::router#parser-dispatch-priority-pipeline
/// [`router`]: mod@super::super::router
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn parse_control_character(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    match buffer {
        // Handle ASCII DEL (0x7F) - common Backspace encoding.
        [ASCII_DEL, ..] => Some(ParsedInputEventIR::new(
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::default(),
            },
            byte_offset(1),
        )),

        // Handle control character range (0x00-0x1F).
        [byte @ ..=CTRL_CHAR_RANGE_MAX, ..] => {
            // Handle special control characters as dedicated keys (not Ctrl+letter).
            match *byte {
                CONTROL_NUL => {
                    // Ctrl+Space (or Ctrl+@) generates NUL.
                    // Treat as Ctrl+Space for better usability.
                    Some(ParsedInputEventIR::new(
                        VT100InputEventIR::Keyboard {
                            code: VT100KeyCodeIR::Char(SPACE_CHAR),
                            modifiers: VT100KeyModifiersIR::CTRL,
                        },
                        byte_offset(1),
                    ))
                }
                CONTROL_TAB => {
                    // Tab key (0x09) - treated as Tab, not `Ctrl+I`.
                    Some(ParsedInputEventIR::new(
                        VT100InputEventIR::Keyboard {
                            code: VT100KeyCodeIR::Tab,
                            modifiers: VT100KeyModifiersIR::default(),
                        },
                        byte_offset(1),
                    ))
                }
                CONTROL_LF | CONTROL_ENTER => {
                    // Enter key sends CR (0x0D) or LF (0x0A) depending on terminal.
                    Some(ParsedInputEventIR::new(
                        VT100InputEventIR::Keyboard {
                            code: VT100KeyCodeIR::Enter,
                            modifiers: VT100KeyModifiersIR::default(),
                        },
                        byte_offset(1),
                    ))
                }
                CONTROL_BACKSPACE => {
                    // Backspace can send BS (0x08) or DEL (0x7F).
                    Some(ParsedInputEventIR::new(
                        VT100InputEventIR::Keyboard {
                            code: VT100KeyCodeIR::Backspace,
                            modifiers: VT100KeyModifiersIR::default(),
                        },
                        byte_offset(1),
                    ))
                }
                CONTROL_ESC => None, // Escape - handled in try_parse() routing.
                _ => {
                    // Convert control character to Ctrl+letter.
                    // Control characters are generated as: letter & 0x1F.
                    // Reverse: (byte | 0x40) gives uppercase letter, (byte | 0x60) gives
                    // lowercase. Example: 0x01 | 0x60 = 0x61 = 'a'.
                    let letter = char::from(*byte | CTRL_TO_LOWERCASE_MASK);

                    Some(ParsedInputEventIR::new(
                        VT100InputEventIR::Keyboard {
                            code: VT100KeyCodeIR::Char(letter),
                            modifiers: VT100KeyModifiersIR::CTRL,
                        },
                        byte_offset(1),
                    ))
                }
            }
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::constants::{ASCII_LOWER_A, ASCII_LOWER_Z, ASCII_UPPER_A,
                                       PRINTABLE_ASCII_MIN};

    #[test]
    fn test_del_is_backspace() {
        let ParsedInputEventIR {
            event,
            bytes_consumed: len,
        } = parse_control_character(&[ASCII_DEL]).unwrap();
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(len, byte_offset(1));
    }

    #[test]
    fn test_control_nul_is_ctrl_space() {
        let ParsedInputEventIR {
            event,
            bytes_consumed: len,
        } = parse_control_character(&[CONTROL_NUL]).unwrap();
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(SPACE_CHAR),
                modifiers: VT100KeyModifiersIR::CTRL,
            }
        );
        assert_eq!(len, byte_offset(1));
    }

    #[test]
    fn test_control_tab() {
        let ParsedInputEventIR {
            event,
            bytes_consumed: len,
        } = parse_control_character(&[CONTROL_TAB]).unwrap();
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Tab,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(len, byte_offset(1));
    }

    #[test]
    fn test_control_enter_lf_and_cr() {
        let ParsedInputEventIR {
            event: event_lf, ..
        } = parse_control_character(&[CONTROL_LF]).unwrap();
        assert_eq!(
            event_lf,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Enter,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );

        let ParsedInputEventIR {
            event: event_cr, ..
        } = parse_control_character(&[CONTROL_ENTER]).unwrap();
        assert_eq!(
            event_cr,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Enter,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
    }

    #[test]
    fn test_control_backspace() {
        let ParsedInputEventIR { event, .. } =
            parse_control_character(&[CONTROL_BACKSPACE]).unwrap();
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
    }

    #[test]
    fn test_control_esc_returns_none() {
        assert!(parse_control_character(&[CONTROL_ESC]).is_none());
    }

    #[test]
    fn test_ctrl_letters() {
        let ParsedInputEventIR { event: event_a, .. } =
            parse_control_character(&[0x01]).unwrap(); // Ctrl+A
        assert_eq!(
            event_a,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(char::from(ASCII_LOWER_A)),
                modifiers: VT100KeyModifiersIR::CTRL,
            }
        );

        let ParsedInputEventIR { event: event_z, .. } =
            parse_control_character(&[0x1A]).unwrap(); // Ctrl+Z
        assert_eq!(
            event_z,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(char::from(ASCII_LOWER_Z)),
                modifiers: VT100KeyModifiersIR::CTRL,
            }
        );
    }

    #[test]
    fn test_empty_and_non_control_returns_none() {
        assert!(parse_control_character(&[]).is_none());
        assert!(parse_control_character(&[PRINTABLE_ASCII_MIN]).is_none());
        assert!(parse_control_character(&[ASCII_UPPER_A]).is_none());
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control character and dedicated key parsing ([`ASCII`] `0x00`-`0x1F` and DEL `0x7F`).
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
/// [`Ambiguous Control Character Handling`]: mod@super#ambiguous-control-character-handling
/// [`Control Key Combinations`]: mod@super#control-key-combinations-ctrlletter
/// [`ESC`]: crate::EscSequence
/// [`Parser Dispatch Priority Pipeline`]: mod@super::super::router#parser-dispatch-priority-pipeline
/// [`router`]: mod@super::super::router
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn parse_control_character(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    let first_byte = *buffer.first()?;

    // Check dedicated keys (Backspace, Tab, Enter, Ctrl+Space) first. Any other byte in
    // the 0..=31 range is handled as a Ctrl+letter combination at the bottom.
    match first_byte {
        //
        // Handle special control characters as dedicated keys (not Ctrl+letter).
        //

        // Backspace can send DEL (0x7F) or BS (0x08).
        ASCII_DEL | CONTROL_BACKSPACE => Some(ParsedInputEventIR::new(
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::default(),
            },
            byte_offset(1),
        )),
        // Ctrl+Space (or Ctrl+@) generates NUL.
        // Treat as Ctrl+Space for better usability.
        CONTROL_NUL => Some(ParsedInputEventIR::new(
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(SPACE_CHAR),
                modifiers: VT100KeyModifiersIR::CTRL,
            },
            byte_offset(1),
        )),
        // Tab key (0x09) - treated as Tab, not `Ctrl+I`.
        CONTROL_TAB => Some(ParsedInputEventIR::new(
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Tab,
                modifiers: VT100KeyModifiersIR::default(),
            },
            byte_offset(1),
        )),
        // Enter key sends CR (0x0D) or LF (0x0A) depending on terminal.
        CONTROL_LF | CONTROL_ENTER => Some(ParsedInputEventIR::new(
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Enter,
                modifiers: VT100KeyModifiersIR::default(),
            },
            byte_offset(1),
        )),
        // Escape - handled in try_parse() routing.
        CONTROL_ESC => None,

        // Remaining unhandled control characters in the 0x00-0x1F range:
        // primarily Ctrl+letter combinations (Ctrl+A through Ctrl+Z).
        byte @ 0..=CTRL_CHAR_RANGE_MAX => Some(parse_ctrl_key(byte)),

        // Fallback arm for any other byte.
        _ => None,
    }
}

/// Convert a control byte (`0x00`-`0x1F`) into a Ctrl+letter keyboard event.
///
/// # Terminal Transmission & Case Destruction
///
/// When a user presses a Ctrl+letter combination, standard terminal emulators transmit a
/// single control byte where bits 5 and 6 have been zeroed out (`& 0x1F`).
///
/// Because bits 5 and 6 are stripped, case information is permanently destroyed by the
/// terminal:
/// - `'A'` (`0b100_0001`) `& 0x1F` = `0x01`
/// - `'a'` (`0b110_0001`) `& 0x1F` = `0x01`
///
/// A standard terminal emulator sends the exact same byte (`0x01`) whether the user types
/// `Ctrl+a` or `Ctrl+Shift+A`. The parser has no way to know whether Shift was pressed or
/// which case the user intended.
///
/// Control bytes are transmitted as:
/// - `Ctrl+A`: sends byte value `1` (`0x01`).
/// - `Ctrl+B`: sends byte value `2` (`0x02`).
/// - `Ctrl+C`: sends byte value `3` (`0x03`).
/// - ...
/// - `Ctrl+Z`: sends byte value `26` (`0x1A`).
///
/// # Reconstruction & Why We Choose Lowercase (i.e., `| 0x60`)
///
/// When reconstructing the key from that single byte, we have to choose which case to
/// represent:
/// - Setting bit 6 (`| 0x40`) yields uppercase `'A'`.
/// - Setting bits 5 and 6 (`| 0x60`, [`CTRL_TO_LOWERCASE_MASK`]) yields lowercase
///   [`ASCII`] `'a'`.
///
/// We normalize to lowercase by bitwise OR-ing the control byte with
/// [`CTRL_TO_LOWERCASE_MASK`] (`0x60`):
/// - Byte value `1` (`0x01`) | `0x60` = `0x61` -> `'a'`.
/// - Byte value `2` (`0x02`) | `0x60` = `0x62` -> `'b'`.
///
/// # Routing: Legacy [`VT-100`] vs. [`Kitty`] Keyboard Protocol
///
/// This function only handles legacy [`VT-100`] single-byte control inputs
/// (`0x00`-`0x1F`). The top-level [`router`] automatically separates these paths based on
/// byte prefix:
///
/// - **Legacy inputs** arrive as single non-[`ESC`] bytes (e.g., `0x01` for `Ctrl+A`) and
///   are routed directly to [`parse_control_character`] and this helper.
/// - **Modern inputs** (terminals supporting the [Kitty Keyboard Protocol]) arrive as
///   multi-byte [`CSI`] sequences starting with `ESC [` and are routed to
///   [`parse_csi_u_sequence`], preserving both letter case and Shift modifiers without
///   ambiguity. Here are examples:
///   - `\x1b[97;5u` for `Ctrl+A`.
///   - `\x1b[97;6u` for `Ctrl+Shift+A`.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CSI`]: crate::CsiSequence
/// [`CTRL_TO_LOWERCASE_MASK`]: crate::core::ansi::constants::CTRL_TO_LOWERCASE_MASK
/// [`ESC`]: crate::EscSequence
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`parse_csi_u_sequence`]: super::parse_csi_u_sequence
/// [`router`]: mod@super::super::router
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [Kitty Keyboard Protocol]: super::csi_u
fn parse_ctrl_key(byte: u8) -> ParsedInputEventIR {
    let letter = char::from(byte | CTRL_TO_LOWERCASE_MASK);
    ParsedInputEventIR::new(
        VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(letter),
            modifiers: VT100KeyModifiersIR::CTRL,
        },
        byte_offset(1),
    )
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

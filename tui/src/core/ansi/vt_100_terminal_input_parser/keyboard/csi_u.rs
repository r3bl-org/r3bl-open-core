// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [Kitty Keyboard Protocol] sequence (`CSI u`) parsing.
//!
//! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/

use super::{super::ir_event_types::{ParsedInputEventIR, VT100InputEventIR,
                                    VT100KeyCodeIR},
            modifiers};
use crate::{NarrowingCastToU8, NarrowingCastToU16, WideningCastToU32, byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_CSI_U, ANSI_ESC,
                                    ANSI_PARAM_SEPARATOR, ANSI_SUBPARAM_SEPARATOR,
                                    ASCII_DEL, ASCII_DIGIT_0, ASCII_DIGIT_9,
                                    CONTROL_BACKSPACE, CONTROL_ENTER, CONTROL_ESC,
                                    CONTROL_TAB, CSI_PREFIX_LEN, KITTY_EVENT_PRESS,
                                    KITTY_EVENT_RELEASE, KITTY_PUA_DELETE,
                                    KITTY_PUA_DOWN, KITTY_PUA_END, KITTY_PUA_F1,
                                    KITTY_PUA_F12, KITTY_PUA_HOME, KITTY_PUA_INSERT,
                                    KITTY_PUA_LEFT, KITTY_PUA_PAGE_DOWN,
                                    KITTY_PUA_PAGE_UP, KITTY_PUA_RIGHT, KITTY_PUA_UP,
                                    MODIFIER_PARAMETER_OFFSET}};

/// Parses a [Kitty Keyboard Protocol] sequence (`CSI u`) into a [`VT100InputEventIR`].
///
/// **Syntax**:
///
/// | Variant                 | Syntax                                            | Description                                     |
/// | :---------------------- | :------------------------------------------------ | :---------------------------------------------- |
/// | Key only                | `ESC [ <codepoint> u`                             | Base key without modifiers                      |
/// | With modifier           | `ESC [ <codepoint> ; <modifier> u`                | Key with modifiers (Shift, Alt, Ctrl)           |
/// | With modifier and event | `ESC [ <codepoint> ; <modifier> : <event_type> u` | Event types: 1 = press, 2 = repeat, 3 = release |
///
/// # Examples
///
/// | Sequence             | Parsed Event                   | Description                  |
/// | :------------------- | :----------------------------- | :--------------------------- |
/// | `ESC [ 91 ; 3 u`     | `Alt+[`                        | Codepoint 91 (`[`), Alt (3)  |
/// | `ESC [ 13 ; 2 u`     | `Shift+Enter`                  | Codepoint 13 (CR), Shift (2) |
/// | `ESC [ 9 ; 5 u`      | `Ctrl+Tab`                     | Codepoint 9 (Tab), Ctrl (5)  |
/// | `ESC [ 27 ; 3 u`     | `Alt+Escape`                   | Codepoint 27 (Esc), Alt (3)  |
/// | `ESC [ 91 ; 3 : 1 u` | `Alt+[`                        | Press event (`:1`)           |
/// | `ESC [ 91 ; 3 : 3 u` | [`VT100InputEventIR::Ignored`] | Release event (`:3`)         |
///
/// [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
#[must_use]
pub fn parse_csi_u_sequence(chunk: &[u8]) -> Option<ParsedInputEventIR> {
    // 1. Early return: Must start with "ESC [ <digit>".
    let [ANSI_ESC, ANSI_CSI_BRACKET, first_byte, ..] = *chunk else {
        return None;
    };
    if !(ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&first_byte) {
        return None;
    }

    // 2. Early return: Must contain the terminating `u`.
    let payload = chunk.get(CSI_PREFIX_LEN..)?;
    let u_pos = payload.iter().position(|&byte| byte == ANSI_CSI_U)?;
    let parameter_bytes = payload.get(..u_pos)?;

    // 3. Early return: All parameter bytes must be digits, ';', or ':'.
    if !parameter_bytes.iter().all(is_valid_csi_u_param_byte) {
        return None;
    }

    // Parse parameters: "<codepoint>" or "<codepoint>;<modifiers>".
    let mut parameter_parts = parameter_bytes.split(|&byte| byte == ANSI_PARAM_SEPARATOR);
    let codepoint = parse_codepoint_parameter(parameter_parts.next()?)?;
    let CsiUModifierInfo {
        modifier_param,
        event_type,
    } = parse_modifier_parameter(parameter_parts.next())?;

    // Bytes consumed = prefix ("ESC [") + parameter bytes + terminator (`u`).
    let bytes_consumed = byte_offset(CSI_PREFIX_LEN + parameter_bytes.len() + 1);

    // Event type: 1 = press, 2 = repeat, 3 = release (consumed and ignored).
    if event_type == KITTY_EVENT_RELEASE {
        return Some(ParsedInputEventIR::new(
            VT100InputEventIR::Ignored,
            bytes_consumed,
        ));
    }

    let key_modifiers = modifiers::decode_modifiers(modifier_param);
    let key_code = decode_csi_u_codepoint(codepoint)?;

    Some(ParsedInputEventIR::new(
        VT100InputEventIR::Keyboard {
            code: key_code,
            modifiers: key_modifiers,
        },
        bytes_consumed,
    ))
}

/// Parsed modifier parameter and event type from the modifier portion of `CSI u`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CsiUModifierInfo {
    modifier_param: u8,
    event_type: u8,
}

/// Returns `true` if the byte is an [`ASCII`] digit (`'0'`..=`'9'`), semicolon (`;`),
/// or colon (`:`).
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
fn is_valid_csi_u_param_byte(byte: &u8) -> bool {
    (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(byte)
        || *byte == ANSI_PARAM_SEPARATOR
        || *byte == ANSI_SUBPARAM_SEPARATOR
}

/// Parses the codepoint parameter, ignoring any colon-separated alternate keys (e.g.
/// `"91:93"` -> `91`).
fn parse_codepoint_parameter(codepoint_param_slice: &[u8]) -> Option<u32> {
    let codepoint_digit_bytes = codepoint_param_slice
        .split(|&byte| byte == ANSI_SUBPARAM_SEPARATOR)
        .next()?;
    parse_decimal_digits(codepoint_digit_bytes)
}

/// Parses the optional modifier parameter slice (e.g. `"3"`, `"3:1"`, or `""`).
fn parse_modifier_parameter(
    modifier_param_slice: Option<&[u8]>,
) -> Option<CsiUModifierInfo> {
    let mut modifier_param: u8 = MODIFIER_PARAMETER_OFFSET;
    let mut event_type: u8 = KITTY_EVENT_PRESS;

    let Some(modifier_slice) = modifier_param_slice else {
        return Some(CsiUModifierInfo {
            modifier_param,
            event_type,
        });
    };

    if modifier_slice.is_empty() {
        return Some(CsiUModifierInfo {
            modifier_param,
            event_type,
        });
    }

    let mut modifier_sub_parts =
        modifier_slice.split(|&byte| byte == ANSI_SUBPARAM_SEPARATOR);

    if let Some(modifier_digit_bytes) = modifier_sub_parts.next()
        && !modifier_digit_bytes.is_empty()
    {
        let raw_modifier_value =
            parse_decimal_digits(modifier_digit_bytes)?.as_u16_narrowing();
        modifier_param = modifiers::extract_modifier_parameter(raw_modifier_value);
    }

    if let Some(event_type_digit_bytes) = modifier_sub_parts.next()
        && !event_type_digit_bytes.is_empty()
    {
        event_type = parse_decimal_digits(event_type_digit_bytes)?.as_u8_narrowing();
    }

    Some(CsiUModifierInfo {
        modifier_param,
        event_type,
    })
}

fn decode_csi_u_codepoint(codepoint: u32) -> Option<VT100KeyCodeIR> {
    // Standard ASCII control characters.
    if let Ok(ascii_byte) = u8::try_from(codepoint) {
        match ascii_byte {
            CONTROL_ENTER => return Some(VT100KeyCodeIR::Enter),
            CONTROL_TAB => return Some(VT100KeyCodeIR::Tab),
            CONTROL_ESC => return Some(VT100KeyCodeIR::Escape),
            ASCII_DEL | CONTROL_BACKSPACE => return Some(VT100KeyCodeIR::Backspace),
            _ => {}
        }
    }

    match codepoint {
        // Kitty functional key codepoints in Private Use Area (PUA).
        KITTY_PUA_INSERT => Some(VT100KeyCodeIR::Insert),
        KITTY_PUA_DELETE => Some(VT100KeyCodeIR::Delete),
        KITTY_PUA_LEFT => Some(VT100KeyCodeIR::Left),
        KITTY_PUA_RIGHT => Some(VT100KeyCodeIR::Right),
        KITTY_PUA_UP => Some(VT100KeyCodeIR::Up),
        KITTY_PUA_DOWN => Some(VT100KeyCodeIR::Down),
        KITTY_PUA_PAGE_UP => Some(VT100KeyCodeIR::PageUp),
        KITTY_PUA_PAGE_DOWN => Some(VT100KeyCodeIR::PageDown),
        KITTY_PUA_HOME => Some(VT100KeyCodeIR::Home),
        KITTY_PUA_END => Some(VT100KeyCodeIR::End),
        KITTY_PUA_F1..=KITTY_PUA_F12 => {
            let function_key_number = (codepoint - KITTY_PUA_F1 + 1).as_u8_narrowing();
            Some(VT100KeyCodeIR::Function(function_key_number))
        }

        // Any printable character or Unicode codepoint.
        _ => char::from_u32(codepoint).map(VT100KeyCodeIR::Char),
    }
}

/// Parses a text-formatted number in a byte slice into an integer ([`u32`]).
///
/// This is a zero-allocation, byte-level equivalent of [`str::parse::<u32>()`] (or C's
/// [`atoi`]).
///
/// # Context: Numbers as Text in Terminal Streams
///
/// The terminal transmits escape sequences as human-readable [`ASCII`] text over the
/// wire. For example, in the [`Kitty`] sequence `ESC [ 9 1 ; 3 u`, the number `91` is
/// sent as the text characters `'9'` and `'1'`.
///
/// Rather than allocating or converting the slice into a `&str` (which requires a
/// [`UTF-8`] validation pass), this function parses the text representation of the number
/// directly from the raw byte stream:
///
/// ```text
/// Text string in stream:     "91"
///                             │└───┐
///                             ▼    ▼
/// ASCII character bytes:    ['9', '1']  (decimal byte values: [57, 49])
///
/// Resulting u32 integer:      91
/// ```
///
/// `b"91"` is simply string-syntax shorthand for the byte slice `&[b'9', b'1']`.
///
/// # Arguments
///
/// - `digit_bytes`: Slice containing the text-formatted number, e.g. `&[b'9', b'1']`
///
/// # Returns
///
/// - `Some(u32)`: If the text contains only valid [`ASCII`] digits (`'0'`..=`'9'`).
/// - `None`: If `digit_bytes` is empty or contains non-digit text (e.g. `;`, `:`, or
///   letters).
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::vt_100_terminal_input_parser::parse_decimal_digits;
/// // Text "91" parses to integer 91:
/// assert_eq!(parse_decimal_digits(b"91"), Some(91));
///
/// // Text "57366" (Home key) parses to integer 57366:
/// assert_eq!(parse_decimal_digits(b"57366"), Some(57366));
///
/// // Empty text slice returns None:
/// assert_eq!(parse_decimal_digits(b""), None);
///
/// // Non-digit characters (e.g. ';' in "12;3") return None:
/// assert_eq!(parse_decimal_digits(b"12;3"), None);
/// ```
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`atoi`]: https://man7.org/linux/man-pages/man3/atoi.3.html
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`str::parse::<u32>()`]: str::parse
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn parse_decimal_digits(digit_bytes: &[u8]) -> Option<u32> {
    const DECIMAL_RADIX: u32 = 10;

    if digit_bytes.is_empty() {
        return None;
    }
    let mut accumulated_value: u32 = 0;
    for byte in digit_bytes.iter().copied() {
        if !(ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
            return None;
        }
        let digit = (byte - ASCII_DIGIT_0).as_u32_widening();
        accumulated_value = accumulated_value
            .saturating_mul(DECIMAL_RADIX)
            .saturating_add(digit);
    }
    Some(accumulated_value)
}

#[cfg(test)]
mod tests {
    use super::{super::super::ir_event_types::VT100KeyModifiersIR, *};

    #[test]
    fn test_parse_csi_u_alt_bracket() {
        let input = b"\x1b[91;3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Alt+[");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_shift_enter() {
        let input = b"\x1b[13;2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Shift+Enter");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Enter,
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_ctrl_tab() {
        let input = b"\x1b[9;5u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Ctrl+Tab");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Tab,
                modifiers: VT100KeyModifiersIR::CTRL,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_alt_escape() {
        let input = b"\x1b[27;3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Alt+Escape");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Escape,
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_event_types() {
        // Event type 1 (press): accepted
        let input_press = b"\x1b[91;3:1u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_press).expect("Should parse press event");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_press.len());

        // Event type 2 (repeat): accepted
        let input_repeat = b"\x1b[91;3:2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_repeat).expect("Should parse repeat event");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_repeat.len());

        // Event type 3 (release): emitted as Ignored
        let input_release = b"\x1b[91;3:3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_release).expect("Should parse release event");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(bytes_consumed.as_usize(), input_release.len());
    }

    #[test]
    fn test_parse_csi_u_omitted_modifiers() {
        let input = b"\x1b[91u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input).expect("Should parse plain CSI u");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::NONE,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_pua_functional_keys() {
        let input = b"\x1b[57366;2u"; // Home + Shift
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input).expect("Should parse Shift+Home PUA");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_backspace() {
        // Codepoint 127 (DEL) + Shift -> Backspace.
        let input_del_key = b"\x1b[127;2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_del_key)
            .expect("Should parse Shift+Backspace (127)");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_del_key.len());

        // Codepoint 8 (BS) + Ctrl -> Backspace.
        let input_backspace_key = b"\x1b[8;5u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_backspace_key)
            .expect("Should parse Ctrl+Backspace (8)");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::CTRL,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_backspace_key.len());
    }

    #[test]
    fn test_parse_csi_u_pua_navigation_and_editing() {
        let cases = [
            (b"\x1b[57358;1u".as_slice(), VT100KeyCodeIR::Insert),
            (b"\x1b[57359;1u".as_slice(), VT100KeyCodeIR::Delete),
            (b"\x1b[57360;1u".as_slice(), VT100KeyCodeIR::Left),
            (b"\x1b[57361;1u".as_slice(), VT100KeyCodeIR::Right),
            (b"\x1b[57362;1u".as_slice(), VT100KeyCodeIR::Up),
            (b"\x1b[57363;1u".as_slice(), VT100KeyCodeIR::Down),
            (b"\x1b[57364;1u".as_slice(), VT100KeyCodeIR::PageUp),
            (b"\x1b[57365;1u".as_slice(), VT100KeyCodeIR::PageDown),
            (b"\x1b[57367;1u".as_slice(), VT100KeyCodeIR::End),
        ];

        for (input, expected_code) in cases {
            let ParsedInputEventIR {
                event,
                bytes_consumed,
            } = parse_csi_u_sequence(input)
                .unwrap_or_else(|| panic!("Failed parsing {input:?}"));
            assert_eq!(
                event,
                VT100InputEventIR::Keyboard {
                    code: expected_code,
                    modifiers: VT100KeyModifiersIR::NONE,
                }
            );
            assert_eq!(bytes_consumed.as_usize(), input.len());
        }
    }

    #[test]
    fn test_parse_csi_u_function_keys_range() {
        // F1 (57376) and F12 (57387) boundary testing.
        let input_f1 = b"\x1b[57376;1u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_f1).expect("Should parse F1");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: VT100KeyModifiersIR::NONE,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_f1.len());

        let input_f12 = b"\x1b[57387;2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_f12).expect("Should parse Shift+F12");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(12),
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_f12.len());
    }

    #[test]
    fn test_parse_csi_u_alternate_key_and_empty_modifier() {
        // Sub-parameter alternate key: 91:93 (codepoint 91, shifted 93) -> ignores
        // alternate key.
        let input_alt_key = b"\x1b[91:93;3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_alt_key)
            .expect("Should parse codepoint with alternate key sub-param");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_alt_key.len());

        // Empty modifier after semicolon: `\x1b[91;u` -> default modifier 1 (NONE).
        let input_empty_modifier = b"\x1b[91;u";
        let ParsedInputEventIR {
            event,
            bytes_consumed,
        } = parse_csi_u_sequence(input_empty_modifier)
            .expect("Should parse sequence with trailing semicolon");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::NONE,
            }
        );
        assert_eq!(bytes_consumed.as_usize(), input_empty_modifier.len());
    }

    #[test]
    fn test_parse_csi_u_invalid_sequences() {
        assert!(parse_csi_u_sequence(b"").is_none());
        assert!(parse_csi_u_sequence(b"\x1b[").is_none());
        assert!(parse_csi_u_sequence(b"\x1b[u").is_none());
        assert!(parse_csi_u_sequence(b"\x1b[91;3").is_none());
        assert!(parse_csi_u_sequence(b"\x1b[91x").is_none());
        assert!(parse_csi_u_sequence(b"\x1b[;3u").is_none());
        assert!(parse_csi_u_sequence(b"other prefix").is_none());
        // Surrogate codepoint (0xD800 = 55296) is not a valid Unicode scalar.
        assert!(parse_csi_u_sequence(b"\x1b[55296;1u").is_none());
    }
}

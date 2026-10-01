// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [Kitty Keyboard Protocol] sequence (`CSI u`) parsing.
//!
//! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/

#[cfg(test)]
use super::super::ir_event_types::VT100KeyModifiersIR;
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
/// Syntax: `ESC [ <codepoint> [; <modifiers> [: <event_type>]] u`
///
/// # Examples
///
/// - `\x1b[91;3u` -> `Alt+[`
/// - `\x1b[13;2u` -> `Shift+Enter`
/// - `\x1b[9;5u`  -> `Ctrl+Tab`
/// - `\x1b[27;3u` -> `Alt+Escape`
/// - `\x1b[91;3:1u` -> `Alt+[` (press)
/// - `\x1b[91;3:3u` -> [`VT100InputEventIR::Ignored`] (release)
///
/// [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
#[must_use]
pub fn parse_csi_u_sequence(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    let [ANSI_ESC, ANSI_CSI_BRACKET, _, _, ..] = *buffer else {
        return None;
    };

    // Find the terminal 'u'.
    let mut u_pos: Option<usize> = None;
    for (i, b) in buffer.iter().copied().enumerate().skip(CSI_PREFIX_LEN) {
        if b == ANSI_CSI_U {
            u_pos = Some(i);
            break;
        }
        // In CSI parameter bytes: valid ASCII range is digits, semicolon, and colon.
        if !(b == ANSI_PARAM_SEPARATOR
            || b == ANSI_SUBPARAM_SEPARATOR
            || (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&b))
        {
            return None;
        }
    }

    let u_idx = u_pos?;
    let param_slice = &buffer[CSI_PREFIX_LEN..u_idx];
    if param_slice.is_empty() {
        return None;
    }

    // Split parameters by ';'.
    let mut parts = param_slice.split(|b| *b == ANSI_PARAM_SEPARATOR);
    let codepoint_part = parts.next()?;
    let modifier_part = parts.next();

    // Part 0: codepoint (and optional colon-separated alternate keys, e.g. 91:93).
    let codepoint_raw = codepoint_part
        .split(|b| *b == ANSI_SUBPARAM_SEPARATOR)
        .next()?;
    let codepoint = parse_decimal_digits(codepoint_raw)?;

    // Part 1: modifiers and optional event_type (e.g. "3", "3:1", "3:3").
    let mut modifier_param: u8 = MODIFIER_PARAMETER_OFFSET;
    let mut event_type: u8 = KITTY_EVENT_PRESS; // Default is press.

    if let Some(mod_slice) = modifier_part
        && !mod_slice.is_empty()
    {
        let mut mod_sub_parts = mod_slice.split(|b| *b == ANSI_SUBPARAM_SEPARATOR);
        if let Some(m_raw) = mod_sub_parts.next()
            && !m_raw.is_empty()
        {
            let m = parse_decimal_digits(m_raw)?.as_u16_narrowing();
            modifier_param = modifiers::extract_modifier_parameter(m);
        }
        if let Some(ev_raw) = mod_sub_parts.next()
            && !ev_raw.is_empty()
        {
            event_type = parse_decimal_digits(ev_raw)?.as_u8_narrowing();
        }
    }

    let consumed = byte_offset(u_idx + 1);

    // Event type: 1 = press, 2 = repeat, 3 = release.
    // Releases (event_type == KITTY_EVENT_RELEASE) are consumed and ignored.
    if event_type == KITTY_EVENT_RELEASE {
        return Some(ParsedInputEventIR::new(
            VT100InputEventIR::Ignored,
            consumed,
        ));
    }

    let key_modifiers = modifiers::decode_modifiers(modifier_param);
    let key_code = decode_csi_u_codepoint(codepoint)?;

    Some(ParsedInputEventIR::new(
        VT100InputEventIR::Keyboard {
            code: key_code,
            modifiers: key_modifiers,
        },
        consumed,
    ))
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
            let fn_num = (codepoint - KITTY_PUA_F1 + 1).as_u8_narrowing();
            Some(VT100KeyCodeIR::Function(fn_num))
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
/// - `raw`: A byte slice containing the text-formatted number, e.g. `&[b'9', b'1']`
///
/// # Returns
///
/// - `Some(u32)`: If the text contains only valid [`ASCII`] digits (`'0'`..=`'9'`).
/// - `None`: If `raw` is empty or contains non-digit text (e.g. `;`, `:`, or letters).
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
pub fn parse_decimal_digits(raw: &[u8]) -> Option<u32> {
    const DECIMAL_RADIX: u32 = 10;

    if raw.is_empty() {
        return None;
    }
    let mut acc: u32 = 0;
    for &byte in raw {
        if !(ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
            return None;
        }
        let digit = (byte - ASCII_DIGIT_0).as_u32_widening();
        acc = acc.saturating_mul(DECIMAL_RADIX).saturating_add(digit);
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csi_u_alt_bracket() {
        let input = b"\x1b[91;3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Alt+[");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_shift_enter() {
        let input = b"\x1b[13;2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Shift+Enter");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Enter,
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_ctrl_tab() {
        let input = b"\x1b[9;5u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Ctrl+Tab");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Tab,
                modifiers: VT100KeyModifiersIR::CTRL,
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_alt_escape() {
        let input = b"\x1b[27;3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input).expect("Should parse CSI u Alt+Escape");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Escape,
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_event_types() {
        // Event type 1 (press): accepted
        let input_press = b"\x1b[91;3:1u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_press).expect("Should parse press event");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(consumed.as_usize(), input_press.len());

        // Event type 2 (repeat): accepted
        let input_repeat = b"\x1b[91;3:2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_repeat).expect("Should parse repeat event");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(consumed.as_usize(), input_repeat.len());

        // Event type 3 (release): emitted as Ignored
        let input_release = b"\x1b[91;3:3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_release).expect("Should parse release event");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(consumed.as_usize(), input_release.len());
    }

    #[test]
    fn test_parse_csi_u_omitted_modifiers() {
        let input = b"\x1b[91u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input).expect("Should parse plain CSI u");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::NONE,
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_pua_functional_keys() {
        let input = b"\x1b[57366;2u"; // Home + Shift
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input).expect("Should parse Shift+Home PUA");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_backspace() {
        // Codepoint 127 (DEL) + Shift -> Backspace.
        let input_del = b"\x1b[127;2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_del).expect("Should parse Shift+Backspace (127)");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(consumed.as_usize(), input_del.len());

        // Codepoint 8 (BS) + Ctrl -> Backspace.
        let input_bs = b"\x1b[8;5u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_bs).expect("Should parse Ctrl+Backspace (8)");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Backspace,
                modifiers: VT100KeyModifiersIR::CTRL,
            }
        );
        assert_eq!(consumed.as_usize(), input_bs.len());
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
                bytes_consumed: consumed,
            } = parse_csi_u_sequence(input)
                .unwrap_or_else(|| panic!("Failed parsing {input:?}"));
            assert_eq!(
                event,
                VT100InputEventIR::Keyboard {
                    code: expected_code,
                    modifiers: VT100KeyModifiersIR::NONE,
                }
            );
            assert_eq!(consumed.as_usize(), input.len());
        }
    }

    #[test]
    fn test_parse_csi_u_function_keys_range() {
        // F1 (57376) and F12 (57387) boundary testing.
        let input_f1 = b"\x1b[57376;1u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_f1).expect("Should parse F1");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: VT100KeyModifiersIR::NONE,
            }
        );
        assert_eq!(consumed.as_usize(), input_f1.len());

        let input_f12 = b"\x1b[57387;2u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_f12).expect("Should parse Shift+F12");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(12),
                modifiers: VT100KeyModifiersIR::SHIFT,
            }
        );
        assert_eq!(consumed.as_usize(), input_f12.len());
    }

    #[test]
    fn test_parse_csi_u_alternate_key_and_empty_modifier() {
        // Sub-parameter alternate key: 91:93 (codepoint 91, shifted 93) -> ignores
        // alternate key.
        let input_alt_key = b"\x1b[91:93;3u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_alt_key)
            .expect("Should parse codepoint with alternate key sub-param");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::ALT,
            }
        );
        assert_eq!(consumed.as_usize(), input_alt_key.len());

        // Empty modifier after semicolon: \x1b[91;u -> default modifier 1 (NONE).
        let input_empty_mod = b"\x1b[91;u";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_csi_u_sequence(input_empty_mod)
            .expect("Should parse sequence with trailing semicolon");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::NONE,
            }
        );
        assert_eq!(consumed.as_usize(), input_empty_mod.len());
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

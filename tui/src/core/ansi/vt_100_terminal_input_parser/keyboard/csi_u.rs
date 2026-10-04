// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [Kitty Keyboard Protocol] sequence (`CSI u`) parsing.
//!
//! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/

use super::{super::{csi_scanner::{parse_decimal_digits, strip_csi_numeric_prefix},
                    ir_event_types::{ParsedInputEventIR, VT100InputEventIR,
                                     VT100KeyCodeIR}},
            modifiers};
use crate::{ByteOffset, NarrowingCastToU8, NarrowingCastToU16, byte_offset,
            core::ansi::constants::{ANSI_CSI_U, ANSI_PARAM_SEPARATOR,
                                    ANSI_SUBPARAM_SEPARATOR, ASCII_DEL,
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
/// # Where These Sequences Come From
///
/// These `CSI u` sequences are emitted by modern terminal emulators ([`Kitty`],
/// [`Ghostty`], [`WezTerm`], etc.) after [`OutputDevice::setup_full_screen_tui()`]
/// activates progressive keyboard enhancement via [`enable_keyboard_enhancement()`][enh]
/// (by writing `CSI > 1 u` to [`stdout`]).
///
/// # Syntax
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
/// All decimal parameters (codepoints, modifiers, and event types) are text-formatted
/// [`ASCII`] digit slices parsed directly from the byte stream via
/// [`parse_decimal_digits()`].
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`Ghostty`]: https://ghostty.org/
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`OutputDevice::setup_full_screen_tui()`]: crate::OutputDevice::setup_full_screen_tui
/// [`parse_decimal_digits()`]: super::super::csi_scanner::parse_decimal_digits
/// [`stdout`]: std::io::stdout
/// [`WezTerm`]: https://wezfurlong.org/wezterm/
/// [enh]: crate::TerminalModeController::enable_keyboard_enhancement
/// [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
/// [no-ack]: mod@crate::vt_100_terminal_input_parser#progressive-keyboard-enhancement
#[must_use]
pub fn parse_csi_u_sequence(chunk: &[u8]) -> Option<ParsedInputEventIR> {
    let frame = Frame::try_extract(chunk)?;
    let params = Params::try_parse(frame.parameter_bytes)?;
    let event = params.try_decode()?;

    Some(ParsedInputEventIR::new(event, frame.bytes_consumed))
}

/// Frame information containing the extracted parameter slice and total bytes consumed
/// by a `CSI u` sequence.
#[derive(Debug, PartialEq, Eq)]
struct Frame<'a> {
    parameter_bytes: &'a [u8],
    bytes_consumed: ByteOffset,
}

impl<'a> Frame<'a> {
    /// Validates sequence framing and extracts the raw parameter byte slice.
    ///
    /// Ensures the sequence begins with `ESC [ <digit>`, contains a terminating `u`
    /// ([`ANSI_CSI_U`]), and that all parameter bytes are valid decimal digits or
    /// separators.
    fn try_extract(chunk: &'a [u8]) -> Option<Self> {
        let payload = strip_csi_numeric_prefix(chunk)?;
        let final_byte_index = payload.iter().position(|byte| *byte == ANSI_CSI_U)?;
        let parameter_bytes = payload.get(..final_byte_index)?;

        if !parameter_bytes
            .iter()
            .copied()
            .all(Self::is_valid_param_byte)
        {
            return None;
        }

        let full_seq = chunk.get(..CSI_PREFIX_LEN + final_byte_index + 1)?;

        Some(Self {
            parameter_bytes,
            bytes_consumed: byte_offset(full_seq.len()),
        })
    }

    /// Returns `true` if the byte is an [`ASCII`] digit (`'0'`..=`'9'`), semicolon (`;`),
    /// or colon (`:`).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    fn is_valid_param_byte(byte: u8) -> bool {
        byte.is_ascii_digit()
            || byte == ANSI_PARAM_SEPARATOR
            || byte == ANSI_SUBPARAM_SEPARATOR
    }
}

/// Parsed parameters of a `CSI u` sequence:
/// `ESC [ <codepoint> ; <modifier> : <event_type> u`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Params {
    codepoint: u32,
    modifier_param: u8,
    event_type: u8,
}

impl Params {
    /// Parses the raw parameter slice into codepoint and modifier information.
    ///
    /// The parameter slice is separated by `;` into:
    /// 1. Codepoint parameter (mandatory, e.g. `"91"` or `"91:93"`).
    /// 2. Modifier parameter (optional, e.g. `"3"`, `"3:1"`, `""`, or omitted as in
    ///    `"91"`).
    fn try_parse(parameter_bytes: &[u8]) -> Option<Self> {
        let mut parameter_parts =
            parameter_bytes.split(|byte| *byte == ANSI_PARAM_SEPARATOR);
        let codepoint_param_slice = parameter_parts.next()?;
        let maybe_modifier_param_slice = parameter_parts.next();

        let codepoint = Self::try_parse_codepoint(codepoint_param_slice)?;
        let mut params = Self {
            codepoint,
            modifier_param: MODIFIER_PARAMETER_OFFSET,
            event_type: KITTY_EVENT_PRESS,
        };

        if let Some(modifier_slice) = maybe_modifier_param_slice
            && !modifier_slice.is_empty()
        {
            params.try_apply_modifier_slice(modifier_slice)?;
        }

        Some(params)
    }

    /// Decodes the parsed [`Params`] into a [`VT100InputEventIR`].
    fn try_decode(self) -> Option<VT100InputEventIR> {
        if self.event_type == KITTY_EVENT_RELEASE {
            return Some(VT100InputEventIR::Ignored);
        }

        let key_modifiers = modifiers::decode_modifiers(self.modifier_param);
        let key_code = Self::try_decode_codepoint(self.codepoint)?;

        Some(VT100InputEventIR::Keyboard {
            code: key_code,
            modifiers: key_modifiers,
        })
    }

    /// Parses the mandatory codepoint parameter, ignoring any colon-separated alternate
    /// keys (e.g. `"91:93"` -> `91`).
    fn try_parse_codepoint(codepoint_param_slice: &[u8]) -> Option<u32> {
        let codepoint_digit_bytes = codepoint_param_slice
            .split(|&byte| byte == ANSI_SUBPARAM_SEPARATOR)
            .next()?;
        parse_decimal_digits(codepoint_digit_bytes)
    }

    /// Parses the optional modifier parameter slice (e.g. `"3"`, `"3:1"`, or `""`).
    ///
    /// Under the [Kitty Keyboard Protocol], when the modifier section is omitted or empty
    /// (e.g. `ESC [ 91 u` or `ESC [ 91 ; u`):
    /// - `modifier_param` defaults to [`MODIFIER_PARAMETER_OFFSET`] (`1`), which encodes
    ///   zero modifier bits (no Shift, Alt, or Ctrl).
    /// - `event_type` defaults to [`KITTY_EVENT_PRESS`] (`1`), indicating a key press
    ///   event.
    ///
    /// [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
    fn try_apply_modifier_slice(&mut self, modifier_param_slice: &[u8]) -> Option<()> {
        let mut modifier_sub_parts =
            modifier_param_slice.split(|&byte| byte == ANSI_SUBPARAM_SEPARATOR);

        if let Some(modifier_digit_bytes) = modifier_sub_parts.next()
            && !modifier_digit_bytes.is_empty()
        {
            let raw_modifier_value =
                parse_decimal_digits(modifier_digit_bytes)?.as_u16_narrowing();
            self.modifier_param =
                modifiers::extract_modifier_parameter(raw_modifier_value);
        }

        if let Some(event_type_digit_bytes) = modifier_sub_parts.next()
            && !event_type_digit_bytes.is_empty()
        {
            self.event_type =
                parse_decimal_digits(event_type_digit_bytes)?.as_u8_narrowing();
        }

        Some(())
    }

    fn try_decode_codepoint(codepoint: u32) -> Option<VT100KeyCodeIR> {
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
                let function_key_number =
                    (codepoint - KITTY_PUA_F1 + 1).as_u8_narrowing();
                Some(VT100KeyCodeIR::Function(function_key_number))
            }

            // Any printable character or Unicode codepoint.
            _ => char::from_u32(codepoint).map(VT100KeyCodeIR::Char),
        }
    }
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
    fn test_try_extract_csi_u_frame() {
        let input = b"\x1b[91;3u_trailing";
        let frame = Frame::try_extract(input).expect("Should extract valid CSI u frame");
        assert_eq!(frame.parameter_bytes, b"91;3");
        assert_eq!(frame.bytes_consumed, byte_offset(7));
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

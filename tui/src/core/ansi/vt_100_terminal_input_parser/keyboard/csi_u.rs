// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [Kitty Keyboard Protocol] sequence (`CSI u`) parsing.
//!
//! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/

#[cfg(test)]
use super::super::ir_event_types::VT100KeyModifiersIR;
use super::{super::ir_event_types::{VT100InputEventIR, VT100KeyCodeIR},
            modifiers};
use crate::{ByteOffset, NarrowingCastToU8, WideningCastToU16, WideningCastToU32,
            byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_CSI_U, ANSI_ESC,
                                    ANSI_PARAM_SEPARATOR, ASCII_DIGIT_0, ASCII_DIGIT_9,
                                    CSI_PREFIX_LEN}};

/// Parses a [Kitty Keyboard Protocol] sequence (`CSI u`) into a [`VT100InputEventIR`].
///
/// Grammar: `ESC [ <codepoint> [; <modifiers> [: <event_type>]] u`
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
pub fn parse_csi_u_sequence(buffer: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    if buffer.len() < 4 || buffer[0] != ANSI_ESC || buffer[1] != ANSI_CSI_BRACKET {
        return None;
    }

    // Find the terminal 'u'.
    let mut u_pos: Option<usize> = None;
    for (i, &b) in buffer.iter().enumerate().skip(CSI_PREFIX_LEN) {
        if b == ANSI_CSI_U {
            u_pos = Some(i);
            break;
        }
        // In CSI parameter bytes: valid ASCII range is digits, semicolon, and colon.
        if !(b == ANSI_PARAM_SEPARATOR
            || b == b':'
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
    let mut parts = param_slice.split(|&b| b == ANSI_PARAM_SEPARATOR);
    let codepoint_part = parts.next()?;
    let modifier_part = parts.next();

    // Part 0: codepoint (and optional colon-separated alternate keys, e.g. 91:93).
    let codepoint_raw = codepoint_part.split(|&b| b == b':').next()?;
    if codepoint_raw.is_empty() {
        return None;
    }
    let mut codepoint: u32 = 0;
    for &b in codepoint_raw {
        if !(ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&b) {
            return None;
        }
        codepoint = codepoint
            .saturating_mul(10)
            .saturating_add((b - ASCII_DIGIT_0).as_u32_widening());
    }

    // Part 1: modifiers and optional event_type (e.g. "3", "3:1", "3:3").
    let mut modifier_param: u8 = 1;
    let mut event_type: u8 = 1; // 1 = press (default).

    if let Some(mod_slice) = modifier_part
        && !mod_slice.is_empty()
    {
        let mut mod_sub_parts = mod_slice.split(|&b| b == b':');
        if let Some(m_raw) = mod_sub_parts.next()
            && !m_raw.is_empty()
        {
            let mut m: u16 = 0;
            for &b in m_raw {
                if !(ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&b) {
                    return None;
                }
                m = m
                    .saturating_mul(10)
                    .saturating_add((b - ASCII_DIGIT_0).as_u16_widening());
            }
            modifier_param = modifiers::extract_modifier_parameter(m);
        }
        if let Some(ev_raw) = mod_sub_parts.next()
            && !ev_raw.is_empty()
        {
            let mut ev: u8 = 0;
            for &b in ev_raw {
                if !(ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&b) {
                    return None;
                }
                ev = ev.saturating_mul(10).saturating_add(b - ASCII_DIGIT_0);
            }
            event_type = ev;
        }
    }

    let consumed = byte_offset(u_idx + 1);

    // Event type: 1 = press, 2 = repeat, 3 = release.
    // Releases (event_type == 3) are consumed and ignored.
    if event_type == 3 {
        return Some((VT100InputEventIR::Ignored, consumed));
    }

    let key_modifiers = modifiers::decode_modifiers(modifier_param);
    let key_code = decode_csi_u_codepoint(codepoint)?;

    Some((
        VT100InputEventIR::Keyboard {
            code: key_code,
            modifiers: key_modifiers,
        },
        consumed,
    ))
}

fn decode_csi_u_codepoint(codepoint: u32) -> Option<VT100KeyCodeIR> {
    match codepoint {
        // Standard ASCII control characters.
        13 => Some(VT100KeyCodeIR::Enter),
        9 => Some(VT100KeyCodeIR::Tab),
        27 => Some(VT100KeyCodeIR::Escape),
        127 | 8 => Some(VT100KeyCodeIR::Backspace),

        // Kitty functional key codepoints in Private Use Area (PUA).
        57358 => Some(VT100KeyCodeIR::Insert),
        57359 => Some(VT100KeyCodeIR::Delete),
        57360 => Some(VT100KeyCodeIR::Left),
        57361 => Some(VT100KeyCodeIR::Right),
        57362 => Some(VT100KeyCodeIR::Up),
        57363 => Some(VT100KeyCodeIR::Down),
        57364 => Some(VT100KeyCodeIR::PageUp),
        57365 => Some(VT100KeyCodeIR::PageDown),
        57366 => Some(VT100KeyCodeIR::Home),
        57367 => Some(VT100KeyCodeIR::End),
        57376..=57387 => {
            let fn_num = (codepoint - 57376 + 1).as_u8_narrowing();
            Some(VT100KeyCodeIR::Function(fn_num))
        }

        // Any printable character or Unicode codepoint.
        _ => char::from_u32(codepoint).map(VT100KeyCodeIR::Char),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeyState;

    #[test]
    fn test_parse_csi_u_alt_bracket() {
        let input = b"\x1b[91;3u";
        let (event, consumed) =
            parse_csi_u_sequence(input).expect("Should parse CSI u Alt+[");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                },
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_shift_enter() {
        let input = b"\x1b[13;2u";
        let (event, consumed) =
            parse_csi_u_sequence(input).expect("Should parse CSI u Shift+Enter");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Enter,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                },
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_ctrl_tab() {
        let input = b"\x1b[9;5u";
        let (event, consumed) =
            parse_csi_u_sequence(input).expect("Should parse CSI u Ctrl+Tab");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Tab,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                },
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_alt_escape() {
        let input = b"\x1b[27;3u";
        let (event, consumed) =
            parse_csi_u_sequence(input).expect("Should parse CSI u Alt+Escape");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Escape,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                },
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_event_types() {
        // Event type 1 (press): accepted
        let input_press = b"\x1b[91;3:1u";
        let (event, consumed) =
            parse_csi_u_sequence(input_press).expect("Should parse press event");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                },
            }
        );
        assert_eq!(consumed.as_usize(), input_press.len());

        // Event type 2 (repeat): accepted
        let input_repeat = b"\x1b[91;3:2u";
        let (event, consumed) =
            parse_csi_u_sequence(input_repeat).expect("Should parse repeat event");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                },
            }
        );
        assert_eq!(consumed.as_usize(), input_repeat.len());

        // Event type 3 (release): emitted as Ignored
        let input_release = b"\x1b[91;3:3u";
        let (event, consumed) =
            parse_csi_u_sequence(input_release).expect("Should parse release event");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(consumed.as_usize(), input_release.len());
    }

    #[test]
    fn test_parse_csi_u_omitted_modifiers() {
        let input = b"\x1b[91u";
        let (event, consumed) =
            parse_csi_u_sequence(input).expect("Should parse plain CSI u");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('['),
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }

    #[test]
    fn test_parse_csi_u_pua_functional_keys() {
        let input = b"\x1b[57366;2u"; // Home + Shift
        let (event, consumed) =
            parse_csi_u_sequence(input).expect("Should parse Shift+Home PUA");
        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Home,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                },
            }
        );
        assert_eq!(consumed.as_usize(), input.len());
    }
}

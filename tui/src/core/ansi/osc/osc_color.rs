// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Color specification parsing and report types for [`OSC`] color sequences.
//!
//! Defines [`TerminalColorReport`] and [`TerminalColorRole`], and handles terminal color
//! specifications in both:
//! - Standard X11/XTerm format: `rgb:rrrr/gggg/bbbb` (1 to 4 hex digits per channel)
//! - Hex formats: `#rrggbb` and `#rgb`
//!
//! [`OSC`]: crate::OscSequence
//! [`TerminalColorReport`]: TerminalColorReport
//! [`TerminalColorRole`]: TerminalColorRole

use crate::{HexSlice, OSC_CODE_COLOR_REPORT_BACKGROUND, OSC_CODE_COLOR_REPORT_CURSOR,
            OSC_CODE_COLOR_REPORT_FOREGROUND, OSC_CODE_COLOR_REPORT_HIGHLIGHT,
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND,
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND,
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND, RgbValue,
            core::ansi::constants::{OSC_COLOR_SPEC_CHANNEL_SEPARATOR,
                                    OSC_COLOR_SPEC_HASH_PREFIX,
                                    OSC_COLOR_SPEC_RGB_PREFIX}};
use std::{fmt::{Display, Formatter},
          str::FromStr};

/// Parses a terminal color payload, supporting both the standard X11/XTerm
/// `rgb:rrrr/gggg/bbbb` format and the hash hex `#rrggbb` / `#rgb` format.
///
/// # Supported Formats
/// - `rgb:rrrr/gggg/bbbb` (1 to 4 hex digits per channel)
/// - `#rrggbb`
/// - `#rgb`
#[must_use]
pub fn try_parse_color_spec(payload: &[u8]) -> Option<RgbValue> {
    try_parse_rgb_spec_color(payload).or_else(|| try_parse_hash_hex_color(payload))
}

/// Parses the X11/XTerm `rgb:rrrr/gggg/bbbb` color specification format into an
/// [`RgbValue`].
fn try_parse_rgb_spec_color(payload: &[u8]) -> Option<RgbValue> {
    let prefix_len = OSC_COLOR_SPEC_RGB_PREFIX.len();
    if payload.len() < prefix_len {
        return None;
    }

    let (prefix, rest) = payload.split_at(prefix_len);
    if !prefix.eq_ignore_ascii_case(OSC_COLOR_SPEC_RGB_PREFIX.as_bytes()) {
        return None;
    }

    let mut channels = rest.split(|&b| b == OSC_COLOR_SPEC_CHANNEL_SEPARATOR);
    let (r_slice, g_slice, b_slice) =
        (channels.next()?, channels.next()?, channels.next()?);
    if channels.next().is_some() {
        return None;
    }

    let red = try_parse_hex_channel(r_slice)?;
    let green = try_parse_hex_channel(g_slice)?;
    let blue = try_parse_hex_channel(b_slice)?;

    Some(RgbValue::from_u8(red, green, blue))
}

/// Parses 1, 2, 3, or 4 hex digits into an 8-bit color channel (`0..=255`).
fn try_parse_hex_channel(slice: &[u8]) -> Option<u8> {
    match *slice {
        // 1 hex digit (4-bit): duplicate into both nibbles (e.g., b'f' -> "ff" = 0xff).
        [d] => HexSlice::try_decode_byte(d, d),
        // 2 hex digits (8-bit): full byte value (e.g., b"8a" -> 0x8a).
        [hi, lo] => HexSlice::try_decode_byte(hi, lo),
        // 3 hex digits (12-bit): validate trailing digit, keep upper 8 bits.
        [hi, lo, d2] => {
            HexSlice::try_decode_nibble(d2)?;
            HexSlice::try_decode_byte(hi, lo)
        }
        // 4 hex digits (16-bit): validate trailing pair, keep upper 8 bits.
        [hi, lo, d2, d3] => {
            HexSlice::try_decode_byte(d2, d3)?;
            HexSlice::try_decode_byte(hi, lo)
        }
        _ => None,
    }
}

/// Parses `#RRGGBB` or `#RGB` hex colors into an [`RgbValue`].
fn try_parse_hash_hex_color(payload: &[u8]) -> Option<RgbValue> {
    let hex_slice = payload.strip_prefix(OSC_COLOR_SPEC_HASH_PREFIX)?;
    let step = match hex_slice.len() {
        6 => 2,
        3 => 1,
        _ => return None,
    };

    let mut chunks = hex_slice.chunks_exact(step);
    let red = try_parse_hex_channel(chunks.next()?)?;
    let green = try_parse_hex_channel(chunks.next()?)?;
    let blue = try_parse_hex_channel(chunks.next()?)?;

    Some(RgbValue::from_u8(red, green, blue))
}

/// Structured terminal color response decoded from an [`OSC`] 10, 11, 12, 13, 14, 17, or
/// 19 sequence.
///
/// Contains the reported [`RgbValue`] and the functional [`TerminalColorRole`] it applies
/// to.
///
/// [`OSC`]: crate::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalColorReport {
    pub role: TerminalColorRole,
    pub color: RgbValue,
}

/// Functional role for a terminal color report sequence ([`OSC`] 10, 11, 12, 13, 14, 17,
/// or 19).
///
/// # Omitted Sequences ([`OSC`] 15, 16, and 18)
///
/// In the [XTerm Control Sequences] specification for dynamic window colors ([`OSC`]
/// 10-19), parameter codes 15, 16, and 18 are historically allocated to Tektronix 4014
/// mode:
/// - [`OSC`] 15: Tektronix foreground color.
/// - [`OSC`] 16: Tektronix background color.
/// - [`OSC`] 18: Tektronix cursor color.
///
/// Because modern terminal emulators do not implement Tektronix vector graphics
/// emulation, these sequences are obsolete and omitted here. Only the [`VT-100`] text,
/// cursor, pointer, and highlight color roles are supported.
///
/// [`OSC`]: crate::OscSequence
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [XTerm Control Sequences]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalColorRole {
    /// Foreground color report ([`OSC`] 10).
    ///
    /// [`OSC`]: crate::OscSequence
    Foreground,

    /// Background color report ([`OSC`] 11).
    ///
    /// [`OSC`]: crate::OscSequence
    Background,

    /// Text cursor color report ([`OSC`] 12).
    ///
    /// [`OSC`]: crate::OscSequence
    Cursor,

    /// Mouse pointer foreground color report ([`OSC`] 13).
    ///
    /// [`OSC`]: crate::OscSequence
    MouseForeground,

    /// Mouse pointer background color report ([`OSC`] 14).
    ///
    /// [`OSC`]: crate::OscSequence
    MouseBackground,

    /// Highlight / selection background color report ([`OSC`] 17).
    ///
    /// [`OSC`]: crate::OscSequence
    Highlight,

    /// Highlight / selection foreground color report ([`OSC`] 19).
    ///
    /// [`OSC`]: crate::OscSequence
    HighlightForeground,
}

impl TerminalColorRole {
    /// Returns the [`OSC`] parameter code for this color role.
    ///
    /// [`OSC`]: crate::OscSequence
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Foreground => OSC_CODE_COLOR_REPORT_FOREGROUND,
            Self::Background => OSC_CODE_COLOR_REPORT_BACKGROUND,
            Self::Cursor => OSC_CODE_COLOR_REPORT_CURSOR,
            Self::MouseForeground => OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND,
            Self::MouseBackground => OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND,
            Self::Highlight => OSC_CODE_COLOR_REPORT_HIGHLIGHT,
            Self::HighlightForeground => OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND,
        }
    }
}

impl FromStr for TerminalColorRole {
    type Err = ();

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        match code {
            OSC_CODE_COLOR_REPORT_FOREGROUND => Ok(Self::Foreground),
            OSC_CODE_COLOR_REPORT_BACKGROUND => Ok(Self::Background),
            OSC_CODE_COLOR_REPORT_CURSOR => Ok(Self::Cursor),
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND => Ok(Self::MouseForeground),
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND => Ok(Self::MouseBackground),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT => Ok(Self::Highlight),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND => Ok(Self::HighlightForeground),
            _ => Err(()),
        }
    }
}

impl Display for TerminalColorRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OscSequence, RgbValue, core::ansi::constants::*};

    #[test]
    fn test_terminal_color_role_display() {
        assert_eq!(
            TerminalColorRole::Foreground.as_str(),
            OSC_CODE_COLOR_REPORT_FOREGROUND
        );
        assert_eq!(
            TerminalColorRole::Background.as_str(),
            OSC_CODE_COLOR_REPORT_BACKGROUND
        );
        assert_eq!(
            TerminalColorRole::Cursor.as_str(),
            OSC_CODE_COLOR_REPORT_CURSOR
        );
        assert_eq!(
            TerminalColorRole::MouseForeground.as_str(),
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND
        );
        assert_eq!(
            TerminalColorRole::MouseBackground.as_str(),
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND
        );
        assert_eq!(
            TerminalColorRole::Highlight.as_str(),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT
        );
        assert_eq!(
            TerminalColorRole::HighlightForeground.as_str(),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND
        );

        assert_eq!(
            format!("{}", TerminalColorRole::Foreground),
            OSC_CODE_COLOR_REPORT_FOREGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::Background),
            OSC_CODE_COLOR_REPORT_BACKGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::Cursor),
            OSC_CODE_COLOR_REPORT_CURSOR
        );
        assert_eq!(
            format!("{}", TerminalColorRole::MouseForeground),
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::MouseBackground),
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::Highlight),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT
        );
        assert_eq!(
            format!("{}", TerminalColorRole::HighlightForeground),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND
        );
    }

    #[test]
    fn test_terminal_color_role_from_str() {
        assert_eq!(
            OSC_CODE_COLOR_REPORT_FOREGROUND.parse(),
            Ok(TerminalColorRole::Foreground)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_BACKGROUND.parse(),
            Ok(TerminalColorRole::Background)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_CURSOR.parse(),
            Ok(TerminalColorRole::Cursor)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND.parse(),
            Ok(TerminalColorRole::MouseForeground)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND.parse(),
            Ok(TerminalColorRole::MouseBackground)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_HIGHLIGHT.parse(),
            Ok(TerminalColorRole::Highlight)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND.parse(),
            Ok(TerminalColorRole::HighlightForeground)
        );

        assert!("".parse::<TerminalColorRole>().is_err());
        assert!("0".parse::<TerminalColorRole>().is_err());
        assert!("20".parse::<TerminalColorRole>().is_err());
        assert!("invalid".parse::<TerminalColorRole>().is_err());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_parse_color_report_various_formats() {
        let cases = [
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1e1e/2a2a/3b3b{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Foreground.as_str()
                ),
                TerminalColorRole::Foreground,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1e/2a/3b{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Foreground.as_str()
                ),
                TerminalColorRole::Foreground,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1/2/3{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(17, 34, 51),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}111/222/333{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(0x11, 0x22, 0x33),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#1e2a3b{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#123{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(17, 34, 51),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}ffff/0000/0000{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Cursor.as_str()
                ),
                TerminalColorRole::Cursor,
                RgbValue::from_u8(255, 0, 0),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}00/ff/00{OSC_TERMINATOR_ST}",
                    TerminalColorRole::MouseForeground.as_str()
                ),
                TerminalColorRole::MouseForeground,
                RgbValue::from_u8(0, 255, 0),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#0000ff{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::MouseBackground.as_str()
                ),
                TerminalColorRole::MouseBackground,
                RgbValue::from_u8(0, 0, 255),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}3333/4444/5555{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Highlight.as_str()
                ),
                TerminalColorRole::Highlight,
                RgbValue::from_u8(0x33, 0x44, 0x55),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#ffffff{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::HighlightForeground.as_str()
                ),
                TerminalColorRole::HighlightForeground,
                RgbValue::from_u8(255, 255, 255),
            ),
            (
                format!(
                    "{OSC_START}{}?{OSC_COLOR_SPEC_RGB_PREFIX}00/00/00{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(0, 0, 0),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}RGB:1e/2a/3b{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Foreground.as_str()
                ),
                TerminalColorRole::Foreground,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
        ];

        for (seq, expected_role, expected_color) in &cases {
            let parsed = OscSequence::try_parse(seq.as_bytes())
                .expect("Failed to parse valid color report");
            assert_eq!(
                parsed,
                OscSequence::ColorReport(TerminalColorReport {
                    role: *expected_role,
                    color: *expected_color,
                })
            );
        }
    }

    #[test]
    fn test_parse_color_query() {
        let cases = [
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Foreground.as_str()
                ),
                TerminalColorRole::Foreground,
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Cursor.as_str()
                ),
                TerminalColorRole::Cursor,
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::MouseForeground.as_str()
                ),
                TerminalColorRole::MouseForeground,
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::MouseBackground.as_str()
                ),
                TerminalColorRole::MouseBackground,
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Highlight.as_str()
                ),
                TerminalColorRole::Highlight,
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::HighlightForeground.as_str()
                ),
                TerminalColorRole::HighlightForeground,
            ),
        ];

        for (seq, expected_role) in &cases {
            assert_eq!(
                OscSequence::try_parse(seq.as_bytes()),
                Some(OscSequence::ColorQuery(*expected_role))
            );
        }

        let not_query = format!(
            "{OSC_START}{}{OSC_DELIMITER}notquery{OSC_TERMINATOR_BEL}",
            TerminalColorRole::Background.as_str()
        );
        assert_eq!(OscSequence::try_parse(not_query.as_bytes()), None);

        let invalid_code =
            format!("{OSC_START}99{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}");
        assert_eq!(OscSequence::try_parse(invalid_code.as_bytes()), None);
    }

    #[test]
    fn test_parse_color_report_invalid() {
        let invalid_cases = [
            format!(
                "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}xyz/12/34{OSC_TERMINATOR_BEL}",
                TerminalColorRole::Foreground.as_str()
            ),
            format!(
                "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}12/34{OSC_TERMINATOR_BEL}",
                TerminalColorRole::Background.as_str()
            ),
            format!(
                "{OSC_START}{}{OSC_DELIMITER}#12345{OSC_TERMINATOR_BEL}",
                TerminalColorRole::Foreground.as_str()
            ),
            format!(
                "{OSC_START}{}{OSC_DELIMITER}notacolor{OSC_TERMINATOR_BEL}",
                TerminalColorRole::Background.as_str()
            ),
            format!(
                "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1/2/3/4{OSC_TERMINATOR_BEL}",
                TerminalColorRole::Background.as_str()
            ),
            format!(
                "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}12345/00/00{OSC_TERMINATOR_BEL}",
                TerminalColorRole::Background.as_str()
            ),
        ];

        for seq in &invalid_cases {
            assert_eq!(OscSequence::try_parse(seq.as_bytes()), None);
        }
    }

    #[test]
    fn test_parse_color_spec_direct() {
        assert_eq!(
            try_parse_color_spec(b"rgb:1e1e/2a2a/3b3b"),
            Some(RgbValue::from_u8(0x1e, 0x2a, 0x3b))
        );
        assert_eq!(
            try_parse_color_spec(b"#1e2a3b"),
            Some(RgbValue::from_u8(0x1e, 0x2a, 0x3b))
        );
        assert_eq!(
            try_parse_color_spec(b"#123"),
            Some(RgbValue::from_u8(17, 34, 51))
        );
        assert_eq!(try_parse_color_spec(b"notacolor"), None);
        // Short payload (< prefix_len "rgb:".len() == 4).
        assert_eq!(try_parse_color_spec(b"rg"), None);
        assert_eq!(try_parse_color_spec(b""), None);
        // 5 hex digits that parse as u16 but exceed 4-digit channel limit.
        assert_eq!(try_parse_color_spec(b"rgb:00000/00/00"), None);
        // Leading '+' sign (accepted by u16::from_str_radix) is rejected.
        assert_eq!(try_parse_color_spec(b"rgb:+f/+f/+f"), None);
        assert_eq!(try_parse_color_spec(b"#+f0000"), None);
        // Invalid trailing hex digits in 3- and 4-digit channels are rejected.
        assert_eq!(try_parse_color_spec(b"rgb:12g/00/00"), None);
        assert_eq!(try_parse_color_spec(b"rgb:123g/00/00"), None);
    }
}

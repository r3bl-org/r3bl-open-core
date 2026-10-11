// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Color specification parsing for [`OSC`] color sequences.
//!
//! Handles terminal color specifications in both:
//! - Standard X11/XTerm format: `rgb:rrrr/gggg/bbbb` (1 to 4 hex digits per channel)
//! - Hex formats: `#rrggbb` and `#rgb`
//!
//! [`OSC`]: crate::osc_codes::OscSequence

use crate::{LossyConvertToByte, RgbValue,
            core::ansi::constants::{OSC_COLOR_SPEC_CHANNEL_SEPARATOR,
                                    OSC_COLOR_SPEC_HASH_PREFIX,
                                    OSC_COLOR_SPEC_RGB_PREFIX}};

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
    let hex_str = std::str::from_utf8(slice).ok()?;
    let value = u16::from_str_radix(hex_str, 16).ok()?;
    let channel = match hex_str.len() {
        1 => value * 17, // 4-bit to 8-bit duplication (e.g., 0xF -> 0xFF).
        2 => value,      // 8-bit full value.
        3 => value >> 4, // 12-bit to top 8 bits.
        4 => value >> 8, // 16-bit to top 8 bits.
        _ => return None,
    };

    Some(LossyConvertToByte::to_u8_lossy(channel))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OscSequence, TerminalColorReport, TerminalColorRole,
                core::ansi::constants::*};

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
    }
}

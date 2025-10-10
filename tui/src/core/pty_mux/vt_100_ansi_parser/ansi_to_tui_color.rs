// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI color code to `TuiColor` conversion utility.
//!
//! This module provides conversion functions from various ANSI color formats to the
//! `TuiColor` type used throughout the TUI framework.
//!
//! # Supported Color Formats
//!
//! - **Basic 16 colors** (SGR 30-37, 40-47, 90-97, 100-107) - [`ansi_to_tui_color`]
//! - **256-color palette** (SGR 38;5;n / 48;5;n) - [`ansi256_to_tui_color`]
//! - **RGB true color** (SGR 38;2;r;g;b / 48;2;r;g;b) - [`rgb_to_tui_color`]

use crate::{ANSIBasicColor, AnsiValue, RgbValue, TuiColor};

/// Convert ANSI color code to `TuiColor`.
///
/// Supports both standard (30-37, 40-47) and bright (90-97, 100-107) colors.
/// This is used for basic 16-color ANSI sequences.
#[must_use]
pub fn ansi_to_tui_color(ansi_code: i64) -> TuiColor {
    match ansi_code {
        // Standard colors (30-37, 40-47).
        30 | 40 => TuiColor::Basic(ANSIBasicColor::Black),
        31 | 41 => TuiColor::Basic(ANSIBasicColor::DarkRed),
        32 | 42 => TuiColor::Basic(ANSIBasicColor::DarkGreen),
        33 | 43 => TuiColor::Basic(ANSIBasicColor::DarkYellow),
        34 | 44 => TuiColor::Basic(ANSIBasicColor::DarkBlue),
        35 | 45 => TuiColor::Basic(ANSIBasicColor::DarkMagenta),
        36 | 46 => TuiColor::Basic(ANSIBasicColor::DarkCyan),
        37 | 47 => TuiColor::Basic(ANSIBasicColor::Gray),

        // Bright colors (90-97, 100-107).
        90 | 100 => TuiColor::Basic(ANSIBasicColor::DarkGray),
        91 | 101 => TuiColor::Basic(ANSIBasicColor::Red),
        92 | 102 => TuiColor::Basic(ANSIBasicColor::Green),
        93 | 103 => TuiColor::Basic(ANSIBasicColor::Yellow),
        94 | 104 => TuiColor::Basic(ANSIBasicColor::Blue),
        95 | 105 => TuiColor::Basic(ANSIBasicColor::Magenta),
        96 | 106 => TuiColor::Basic(ANSIBasicColor::Cyan),
        97 | 107 => TuiColor::Basic(ANSIBasicColor::White),

        _ => TuiColor::Reset,
    }
}

/// Convert 256-color palette index to `TuiColor`.
///
/// This function handles extended color sequences using the 256-color palette.
/// The palette is structured as follows:
///
/// - **0-15**: Standard ANSI colors (same as basic 16 colors)
/// - **16-231**: 6×6×6 RGB cube (216 colors)
/// - **232-255**: Grayscale ramp (24 shades from dark to light)
///
/// # Arguments
///
/// * `index` - Palette index (0-255)
///
/// # Returns
///
/// `TuiColor::Ansi` variant containing the palette index
///
/// # Examples
///
/// ```
/// use r3bl_tui::ansi256_to_tui_color;
///
/// // Red from the 216-color cube
/// let color = ansi256_to_tui_color(196);
///
/// // Grayscale value
/// let gray = ansi256_to_tui_color(240);
/// ```
///
/// # VT100 Sequences
///
/// This is typically used with:
/// - `ESC[38;5;nm` - Set foreground to palette index n
/// - `ESC[48;5;nm` - Set background to palette index n
#[must_use]
pub fn ansi256_to_tui_color(index: u8) -> TuiColor {
    TuiColor::Ansi(AnsiValue::new(index))
}

/// Convert RGB color values to `TuiColor`.
///
/// This function handles true color (24-bit) RGB sequences, providing access to
/// 16.7 million colors. Each component (red, green, blue) can range from 0-255.
///
/// # Arguments
///
/// * `r` - Red component (0-255)
/// * `g` - Green component (0-255)
/// * `b` - Blue component (0-255)
///
/// # Returns
///
/// `TuiColor::Rgb` variant containing the RGB values
///
/// # Examples
///
/// ```
/// use r3bl_tui::rgb_to_tui_color;
///
/// // Orange color
/// let orange = rgb_to_tui_color(255, 128, 0);
///
/// // Pure cyan
/// let cyan = rgb_to_tui_color(0, 255, 255);
/// ```
///
/// # VT100 Sequences
///
/// This is typically used with:
/// - `ESC[38;2;r;g;bm` - Set foreground to RGB(r, g, b)
/// - `ESC[48;2;r;g;bm` - Set background to RGB(r, g, b)
#[must_use]
pub fn rgb_to_tui_color(r: u8, g: u8, b: u8) -> TuiColor {
    TuiColor::Rgb(RgbValue::from_u8(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_ansi_to_tui_color() {
        // Note: Codes 0-7 are SGR attributes (bold, underline, etc), not colors.
        // They should not map to colors.
        assert_eq!(ansi_to_tui_color(0), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(1), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(2), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(3), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(4), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(5), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(6), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(7), TuiColor::Reset);

        // Standard foreground colors (30-37).
        assert_eq!(
            ansi_to_tui_color(30),
            TuiColor::Basic(ANSIBasicColor::Black)
        );
        assert_eq!(
            ansi_to_tui_color(31),
            TuiColor::Basic(ANSIBasicColor::DarkRed)
        );
        assert_eq!(
            ansi_to_tui_color(32),
            TuiColor::Basic(ANSIBasicColor::DarkGreen)
        );
        assert_eq!(
            ansi_to_tui_color(33),
            TuiColor::Basic(ANSIBasicColor::DarkYellow)
        );
        assert_eq!(
            ansi_to_tui_color(34),
            TuiColor::Basic(ANSIBasicColor::DarkBlue)
        );
        assert_eq!(
            ansi_to_tui_color(35),
            TuiColor::Basic(ANSIBasicColor::DarkMagenta)
        );
        assert_eq!(
            ansi_to_tui_color(36),
            TuiColor::Basic(ANSIBasicColor::DarkCyan)
        );
        assert_eq!(ansi_to_tui_color(37), TuiColor::Basic(ANSIBasicColor::Gray));

        // Standard background colors (40-47).
        assert_eq!(
            ansi_to_tui_color(40),
            TuiColor::Basic(ANSIBasicColor::Black)
        );
        assert_eq!(
            ansi_to_tui_color(41),
            TuiColor::Basic(ANSIBasicColor::DarkRed)
        );
        assert_eq!(
            ansi_to_tui_color(42),
            TuiColor::Basic(ANSIBasicColor::DarkGreen)
        );
        assert_eq!(
            ansi_to_tui_color(43),
            TuiColor::Basic(ANSIBasicColor::DarkYellow)
        );
        assert_eq!(
            ansi_to_tui_color(44),
            TuiColor::Basic(ANSIBasicColor::DarkBlue)
        );
        assert_eq!(
            ansi_to_tui_color(45),
            TuiColor::Basic(ANSIBasicColor::DarkMagenta)
        );
        assert_eq!(
            ansi_to_tui_color(46),
            TuiColor::Basic(ANSIBasicColor::DarkCyan)
        );
        assert_eq!(ansi_to_tui_color(47), TuiColor::Basic(ANSIBasicColor::Gray));

        // Bright foreground colors (90-97).
        // Note the proper gradation: Black (30) < DarkGray (90) < Gray (37) < White (97).
        assert_eq!(
            ansi_to_tui_color(90),
            TuiColor::Basic(ANSIBasicColor::DarkGray)
        );
        assert_eq!(ansi_to_tui_color(91), TuiColor::Basic(ANSIBasicColor::Red));
        assert_eq!(
            ansi_to_tui_color(92),
            TuiColor::Basic(ANSIBasicColor::Green)
        );
        assert_eq!(
            ansi_to_tui_color(93),
            TuiColor::Basic(ANSIBasicColor::Yellow)
        );
        assert_eq!(ansi_to_tui_color(94), TuiColor::Basic(ANSIBasicColor::Blue));
        assert_eq!(
            ansi_to_tui_color(95),
            TuiColor::Basic(ANSIBasicColor::Magenta)
        );
        assert_eq!(ansi_to_tui_color(96), TuiColor::Basic(ANSIBasicColor::Cyan));
        assert_eq!(
            ansi_to_tui_color(97),
            TuiColor::Basic(ANSIBasicColor::White)
        );

        // Bright background colors (100-107).
        assert_eq!(
            ansi_to_tui_color(100),
            TuiColor::Basic(ANSIBasicColor::DarkGray)
        );
        assert_eq!(ansi_to_tui_color(101), TuiColor::Basic(ANSIBasicColor::Red));
        assert_eq!(
            ansi_to_tui_color(102),
            TuiColor::Basic(ANSIBasicColor::Green)
        );
        assert_eq!(
            ansi_to_tui_color(103),
            TuiColor::Basic(ANSIBasicColor::Yellow)
        );
        assert_eq!(
            ansi_to_tui_color(104),
            TuiColor::Basic(ANSIBasicColor::Blue)
        );
        assert_eq!(
            ansi_to_tui_color(105),
            TuiColor::Basic(ANSIBasicColor::Magenta)
        );
        assert_eq!(
            ansi_to_tui_color(106),
            TuiColor::Basic(ANSIBasicColor::Cyan)
        );
        assert_eq!(
            ansi_to_tui_color(107),
            TuiColor::Basic(ANSIBasicColor::White)
        );

        // Edge cases and invalid codes.
        assert_eq!(ansi_to_tui_color(-1), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(8), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(29), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(38), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(39), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(48), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(49), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(89), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(98), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(99), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(108), TuiColor::Reset);
        assert_eq!(ansi_to_tui_color(999), TuiColor::Reset);
    }
}

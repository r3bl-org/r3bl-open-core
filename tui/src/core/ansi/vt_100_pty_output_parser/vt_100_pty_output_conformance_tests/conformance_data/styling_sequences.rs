// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`SGR`] (Select Graphic Rendition) sequence patterns for text styling and colors.
//!
//! This module provides sequences for all text formatting operations including
//! colors, attributes (bold, italic, underline), and [`reset`] operations.
//! Demonstrates proper [`SGR`] sequence construction and state management.
//!
//! ## [`VT-100` spec] References
//!
//! - [`SGR`] Codes: [`VT-100` spec] Section 3.3.5
//! - Color Support: [`ANSI`] X3.64 Standard
//! - Text Attributes: [`VT-100` spec] Appendix C
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`reset`]: https://man7.org/linux/man-pages/man1/reset.1.html
//! [`SGR`]: crate::SgrCode
//! [`VT-100` spec]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

use crate::{ANSIBasicColor, SgrCode};

/// Apply bold formatting to text.
///
/// **[`ANSI`] Spec**: `ESC [ 1 m` (Bold/Bright)
///
/// # Arguments
/// * `text` - Text to format with bold styling
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
#[must_use]
pub fn bold_text(text: &str) -> String {
    format!(
        "{}{}{}",
        SgrCode::Bold,
        text,
        SgrCode::ResetBoldDim // Reset only bold, preserve other attributes
    )
}

/// Apply inverse/reverse video formatting to text.
///
/// **[`ANSI`] Spec**: `ESC [ 7 m` (Reverse Video)
///
/// Swaps foreground and background colors, commonly used for highlighting.
///
/// # Arguments
/// * `text` - Text to format with reverse video
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
#[must_use]
pub fn reverse_text(text: &str) -> String {
    format!("{}{}{}", SgrCode::Invert, text, SgrCode::ResetInvert)
}

/// Set foreground color for text.
///
/// **[`ANSI`] Spec**: `ESC [ 3 {color} m` (Foreground Color)
///
/// # Arguments
/// * `color` - Basic [`ANSI`] color to apply
/// * `text` - Text to colorize
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
#[must_use]
pub fn colored_text(color: ANSIBasicColor, text: &str) -> String {
    format!(
        "{}{}{}",
        SgrCode::ForegroundBasic(color),
        text,
        SgrCode::Reset
    )
}

/// Apply multiple formatting attributes to text.
///
/// **[`ANSI`] Spec**: Multiple `ESC [ {code} m` sequences
///
/// Demonstrates combining multiple text attributes and proper [`reset`] handling.
///
/// # Arguments
/// * `text` - Text to format
/// * `bold` - Apply bold formatting
/// * `italic` - Apply italic formatting
/// * `fg_color` - Optional foreground color
/// * `bg_color` - Optional background color
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`reset`]: https://man7.org/linux/man-pages/man1/reset.1.html
#[must_use]
pub fn multi_style_text(
    text: &str,
    bold: bool,
    italic: bool,
    fg_color: Option<ANSIBasicColor>,
    bg_color: Option<ANSIBasicColor>,
) -> String {
    let mut sequence = String::new();

    // Apply styles.
    if bold {
        sequence.push_str(&SgrCode::Bold.to_string());
    }
    if italic {
        sequence.push_str(&SgrCode::Italic.to_string());
    }
    if let Some(color) = fg_color {
        sequence.push_str(&SgrCode::ForegroundBasic(color).to_string());
    }
    if let Some(color) = bg_color {
        sequence.push_str(&SgrCode::BackgroundBasic(color).to_string());
    }

    // Add text.
    sequence.push_str(text);

    // Reset all formatting.
    sequence.push_str(&SgrCode::Reset.to_string());

    sequence
}

/// Creates a rainbow-colored text sequence.
///
/// Demonstrates cycling through colors for visual testing of color support.
///
/// # Arguments
/// * `text` - Text to colorize (each character gets a different color)
#[must_use]
pub fn rainbow_text(text: &str) -> String {
    let colors = [
        ANSIBasicColor::Red,
        ANSIBasicColor::Yellow,
        ANSIBasicColor::Green,
        ANSIBasicColor::Cyan,
        ANSIBasicColor::Blue,
        ANSIBasicColor::Magenta,
    ];

    let mut sequence = String::new();

    for (i, ch) in text.chars().enumerate() {
        let color = colors[i % colors.len()];
        sequence.push_str(&colored_text(color, &ch.to_string()));
    }

    sequence
}

/// Test [`SGR`] partial [`reset`] functionality.
///
/// **[`ANSI`] Spec**: `ESC [ 22 m` (Reset Bold/Dim), `ESC [ 23 m` (Reset Italic), etc.
///
/// Demonstrates that partial resets preserve other attributes while
/// clearing specific ones.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`reset`]: https://man7.org/linux/man-pages/man1/reset.1.html
/// [`SGR`]: crate::SgrCode
#[must_use]
pub fn partial_reset_test() -> String {
    format!(
        "{}{}{}{}{}{}{}",
        SgrCode::Bold,
        SgrCode::Italic,
        SgrCode::ForegroundBasic(ANSIBasicColor::Red),
        "Bold+Italic+Red",
        SgrCode::ResetBoldDim, // Reset only bold, keep italic and red
        " Italic+Red",
        SgrCode::Reset // Reset everything
    )
}

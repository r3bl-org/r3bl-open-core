// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! SGR (Select Graphic Rendition) sequence patterns for text styling and colors.
//!
//! This module provides sequences for all text formatting operations including
//! colors, attributes (bold, italic, underline), and reset operations.
//! Demonstrates proper SGR sequence construction and state management.
//!
//! ## VT100 Specification References
//!
//! - SGR Codes: VT100 User Guide Section 3.3.5
//! - Color Support: ANSI X3.64 Standard
//! - Text Attributes: VT100 User Guide Appendix C

use crate::{
    ANSIBasicColor,
    SgrCode,
};

/// Apply bold formatting to text.
///
/// **ANSI Spec**: ESC[1m (Bold/Bright)
///
/// # Arguments
/// * `text` - Text to format with bold styling
pub fn bold_text(text: &str) -> String {
    format!("{}{}{}",
        SgrCode::Bold,
        text,
        SgrCode::ResetBoldDim // Reset only bold, preserve other attributes
    )
}

/// Apply italic formatting to text.
///
/// **ANSI Spec**: ESC[3m (Italic)
///
/// # Arguments
/// * `text` - Text to format with italic styling
pub fn italic_text(text: &str) -> String {
    format!("{}{}{}",
        SgrCode::Italic,
        text,
        SgrCode::ResetItalic
    )
}

/// Apply underline formatting to text.
///
/// **ANSI Spec**: ESC[4m (Underline)
///
/// # Arguments
/// * `text` - Text to format with underline styling
pub fn underline_text(text: &str) -> String {
    format!("{}{}{}",
        SgrCode::Underline,
        text,
        SgrCode::ResetUnderline
    )
}

/// Apply strikethrough formatting to text.
///
/// **ANSI Spec**: ESC[9m (Strikethrough)
///
/// # Arguments
/// * `text` - Text to format with strikethrough styling
pub fn strikethrough_text(text: &str) -> String {
    format!("{}{}{}",
        SgrCode::Strikethrough,
        text,
        SgrCode::ResetStrikethrough
    )
}

/// Apply inverse/reverse video formatting to text.
///
/// **ANSI Spec**: ESC[7m (Reverse Video)
///
/// Swaps foreground and background colors, commonly used for highlighting.
///
/// # Arguments
/// * `text` - Text to format with reverse video
pub fn reverse_text(text: &str) -> String {
    format!("{}{}{}",
        SgrCode::Invert,
        text,
        SgrCode::ResetInvert
    )
}

/// Set foreground color for text.
///
/// **ANSI Spec**: ESC[3{color}m (Foreground Color)
///
/// # Arguments
/// * `color` - Basic ANSI color to apply
/// * `text` - Text to colorize
pub fn colored_text(color: ANSIBasicColor, text: &str) -> String {
    format!("{}{}{}",
        SgrCode::ForegroundBasic(color),
        text,
        SgrCode::Reset
    )
}

/// Set background color for text.
///
/// **ANSI Spec**: ESC[4{color}m (Background Color)
///
/// # Arguments
/// * `color` - Basic ANSI color to apply as background
/// * `text` - Text with colored background
pub fn background_colored_text(color: ANSIBasicColor, text: &str) -> String {
    format!("{}{}{}",
        SgrCode::BackgroundBasic(color),
        text,
        SgrCode::Reset
    )
}

/// Apply multiple formatting attributes to text.
///
/// **ANSI Spec**: Multiple ESC[{code}m sequences
///
/// Demonstrates combining multiple text attributes and proper reset handling.
///
/// # Arguments
/// * `text` - Text to format
/// * `bold` - Apply bold formatting
/// * `italic` - Apply italic formatting
/// * `fg_color` - Optional foreground color
/// * `bg_color` - Optional background color
pub fn multi_style_text(
    text: &str,
    bold: bool,
    italic: bool,
    fg_color: Option<ANSIBasicColor>,
    bg_color: Option<ANSIBasicColor>,
) -> String {
    let mut sequence = String::new();

    // Apply styles
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

    // Add text
    sequence.push_str(text);

    // Reset all formatting
    sequence.push_str(&SgrCode::Reset.to_string());

    sequence
}

/// Create a rainbow-colored text sequence.
///
/// Demonstrates cycling through colors for visual testing of color support.
///
/// # Arguments
/// * `text` - Text to colorize (each character gets a different color)
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

/// Test SGR partial reset functionality.
///
/// **ANSI Spec**: ESC[22m (Reset Bold/Dim), ESC[23m (Reset Italic), etc.
///
/// Demonstrates that partial resets preserve other attributes while
/// clearing specific ones.
pub fn partial_reset_test() -> String {
    format!("{}{}{}{}{}{}{}",
        SgrCode::Bold,
        SgrCode::Italic,
        SgrCode::ForegroundBasic(ANSIBasicColor::Red),
        "Bold+Italic+Red",
        SgrCode::ResetBoldDim, // Reset only bold, keep italic and red
        " Italic+Red",
        SgrCode::Reset // Reset everything
    )
}

/// Test color palette coverage.
///
/// Creates a sequence that displays all basic ANSI colors for
/// comprehensive color support testing.
pub fn color_palette_test() -> String {
    let mut sequence = String::new();

    // Test all basic foreground colors
    sequence.push_str("Foreground: ");
    for color in [
        ANSIBasicColor::Black,
        ANSIBasicColor::DarkRed,
        ANSIBasicColor::DarkGreen,
        ANSIBasicColor::DarkYellow,
        ANSIBasicColor::DarkBlue,
        ANSIBasicColor::DarkMagenta,
        ANSIBasicColor::DarkCyan,
        ANSIBasicColor::Gray,
        ANSIBasicColor::DarkGray,
        ANSIBasicColor::Red,
        ANSIBasicColor::Green,
        ANSIBasicColor::Yellow,
        ANSIBasicColor::Blue,
        ANSIBasicColor::Magenta,
        ANSIBasicColor::Cyan,
        ANSIBasicColor::White,
    ] {
        sequence.push_str(&colored_text(color, "██"));
    }

    sequence.push_str(&SgrCode::Reset.to_string());
    sequence.push('\n');

    // Test all basic background colors
    sequence.push_str("Background: ");
    for color in [
        ANSIBasicColor::Black,
        ANSIBasicColor::DarkRed,
        ANSIBasicColor::DarkGreen,
        ANSIBasicColor::DarkYellow,
        ANSIBasicColor::DarkBlue,
        ANSIBasicColor::DarkMagenta,
        ANSIBasicColor::DarkCyan,
        ANSIBasicColor::Gray,
        ANSIBasicColor::DarkGray,
        ANSIBasicColor::Red,
        ANSIBasicColor::Green,
        ANSIBasicColor::Yellow,
        ANSIBasicColor::Blue,
        ANSIBasicColor::Magenta,
        ANSIBasicColor::Cyan,
        ANSIBasicColor::White,
    ] {
        sequence.push_str(&background_colored_text(color, "  "));
    }

    sequence.push_str(&SgrCode::Reset.to_string());
    sequence
}

/// Complete reset of all SGR attributes.
///
/// **ANSI Spec**: ESC[0m (Reset All Attributes)
///
/// Returns terminal text formatting to default state.
pub fn reset_all_styles() -> String {
    SgrCode::Reset.to_string()
}
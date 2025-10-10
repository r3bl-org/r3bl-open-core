// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extended color sequence parsing for SGR parameters.
//!
//! This module provides type-safe parsing of extended color sequences used in
//! VT100-compliant terminal emulators. These sequences enable 256-color palette support
//! and true RGB colors, going beyond the basic 16 ANSI colors.
//!
//! # Color Sequence Formats
//!
//! VT100 extended color sequences support two formats:
//!
//! ## Colon-Separated Format (Recommended)
//!
//! ```text
//! ESC[38:5:196m        → 256-color foreground (index 196)
//! ESC[48:5:196m        → 256-color background (index 196)
//! ESC[38:2:255:128:0m  → RGB foreground (orange)
//! ESC[48:2:255:128:0m  → RGB background (orange)
//! ```
//!
//! The colon format groups related sub-parameters together, making parsing simpler.
//! In VTE's parameter model, this arrives as a single parameter with multiple
//! sub-parameters: `[[38, 5, 196]]`
//!
//! ## Semicolon-Separated Format (Legacy)
//!
//! ```text
//! ESC[38;5;196m        → 256-color foreground (index 196)
//! ESC[48;5;196m        → 256-color background (index 196)
//! ESC[38;2;255;128;0m  → RGB foreground (orange)
//! ESC[48;2;255;128;0m  → RGB background (orange)
//! ```
//!
//! The semicolon format treats each value as a separate parameter:
//! `[[38], [5], [196]]`
//!
//! Both formats are valid and widely supported by modern terminals.
//!
//! # Architecture
//!
//! The [`ExtendedColorSequence`] enum provides type-safe parsing of color parameters,
//! ensuring that only valid color values are created. It works with the [`ParamsExt`]
//! trait's `extract_nth_all()` method to access the complete parameter slice.
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::ExtendedColorSequence;
//!
//! // Parse 256-color foreground: ESC[38:5:196m
//! let params = &[38, 5, 196];
//! if let Some((color, is_background)) = ExtendedColorSequence::parse_from_slice(params) {
//!     match color {
//!         ExtendedColorSequence::Ansi256 { index } => {
//!             assert_eq!(index, 196);
//!             assert_eq!(is_background, false);
//!         }
//!         _ => unreachable!(),
//!     }
//! }
//!
//! // Parse RGB background: ESC[48:2:255:128:0m
//! let params = &[48, 2, 255, 128, 0];
//! if let Some((color, is_background)) = ExtendedColorSequence::parse_from_slice(params) {
//!     match color {
//!         ExtendedColorSequence::Rgb { r, g, b } => {
//!             assert_eq!((r, g, b), (255, 128, 0));
//!             assert_eq!(is_background, true);
//!         }
//!         _ => unreachable!(),
//!     }
//! }
//! ```
//!
//! [`ParamsExt`]: crate::ParamsExt

use super::constants::{SGR_BG_EXTENDED, SGR_COLOR_MODE_256, SGR_COLOR_MODE_RGB,
                       SGR_FG_EXTENDED};

/// Extended color sequences for 256-color and RGB support.
///
/// This enum represents the two types of extended color sequences supported by modern
/// VT100-compliant terminals: 256-color palette indices and RGB true colors.
///
/// # Variants
///
/// - [`Ansi256`](ExtendedColorSequence::Ansi256): 256-color palette (indices 0-255)
///   - Colors 0-15: Standard ANSI colors (same as SGR 30-37, 90-97)
///   - Colors 16-231: 6×6×6 RGB cube
///   - Colors 232-255: 24-step grayscale ramp
///
/// - [`Rgb`](ExtendedColorSequence::Rgb): True RGB color (16.7 million colors)
///   - Each component (r, g, b) ranges from 0-255
///
/// # VT100 Specification
///
/// These sequences follow the ISO 8613-6 (ITU-T Rec. T.416) standard for color control:
/// - `38` sets foreground color
/// - `48` sets background color
/// - `5` indicates 256-color mode (next parameter is palette index)
/// - `2` indicates RGB mode (next 3 parameters are r, g, b values)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedColorSequence {
    /// 256-color palette index (0-255)
    ///
    /// # Color Palette Layout
    ///
    /// - **0-15**: Standard ANSI colors (matches basic 16-color palette)
    /// - **16-231**: 6×6×6 RGB cube (216 colors)
    ///   - Formula: `16 + 36r + 6g + b` where r,g,b ∈ `[0,5]`
    /// - **232-255**: Grayscale ramp (24 shades from dark to light)
    Ansi256 {
        /// Palette index (0-255)
        index: u8,
    },

    /// RGB color values (true color)
    ///
    /// Each component ranges from 0-255, providing 16.7 million possible colors.
    Rgb {
        /// Red component (0-255)
        r: u8,
        /// Green component (0-255)
        g: u8,
        /// Blue component (0-255)
        b: u8,
    },
}

impl ExtendedColorSequence {
    /// Parse extended color from a parameter slice.
    ///
    /// This method parses both colon-separated and semicolon-separated formats,
    /// automatically detecting whether the sequence is for foreground or background.
    ///
    /// # Supported Formats
    ///
    /// **256-color:**
    /// - `[38, 5, n]` → Foreground, palette index n (0-255)
    /// - `[48, 5, n]` → Background, palette index n (0-255)
    ///
    /// **RGB color:**
    /// - `[38, 2, r, g, b]` → Foreground, RGB values
    /// - `[48, 2, r, g, b]` → Background, RGB values
    ///
    /// # Parameters
    ///
    /// - `params`: The parameter slice from [`ParamsExt::extract_nth_all()`]
    ///
    /// # Returns
    ///
    /// - `Some((color, is_background))` - Successfully parsed color sequence
    ///   - `color`: The parsed color (256-color or RGB)
    ///   - `is_background`: `true` for background, `false` for foreground
    /// - `None` - Invalid or unrecognized sequence
    ///
    /// # Validation
    ///
    /// The method performs strict validation:
    /// - 256-color indices must be ≤ 255
    /// - RGB component values must be ≤ 255
    /// - Invalid sequences (wrong mode, out-of-range values) return `None`
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::ExtendedColorSequence;
    ///
    /// // Valid 256-color foreground
    /// let result = ExtendedColorSequence::parse_from_slice(&[38, 5, 196]);
    /// assert!(result.is_some());
    /// let (color, is_bg) = result.unwrap();
    /// assert!(!is_bg);  // foreground
    ///
    /// // Invalid: index out of range
    /// let result = ExtendedColorSequence::parse_from_slice(&[38, 5, 256]);
    /// assert!(result.is_none());
    ///
    /// // Valid RGB background
    /// let result = ExtendedColorSequence::parse_from_slice(&[48, 2, 255, 128, 0]);
    /// assert!(result.is_some());
    /// let (color, is_bg) = result.unwrap();
    /// assert!(is_bg);  // background
    /// ```
    ///
    /// [`ParamsExt::extract_nth_all()`]: crate::ParamsExt::extract_nth_all
    pub fn parse_from_slice(params: &[u16]) -> Option<(Self, bool)> {
        match params {
            [fg_or_bg, mode, rest @ ..]
                if *fg_or_bg == SGR_FG_EXTENDED || *fg_or_bg == SGR_BG_EXTENDED =>
            {
                let is_background = *fg_or_bg == SGR_BG_EXTENDED;

                match (*mode, rest) {
                    // 256-color mode: ESC[38:5:n or ESC[48:5:n
                    (SGR_COLOR_MODE_256, [index, ..]) if *index <= 255 => Some((
                        Self::Ansi256 {
                            index: *index as u8,
                        },
                        is_background,
                    )),

                    // RGB mode: ESC[38:2:r:g:b or ESC[48:2:r:g:b
                    (SGR_COLOR_MODE_RGB, [r, g, b, ..])
                        if *r <= 255 && *g <= 255 && *b <= 255 =>
                    {
                        Some((
                            Self::Rgb {
                                r: *r as u8,
                                g: *g as u8,
                                b: *b as u8,
                            },
                            is_background,
                        ))
                    }

                    // Invalid or unsupported mode
                    _ => None,
                }
            }
            // Not an extended color sequence
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_256_color_foreground() {
        let result = ExtendedColorSequence::parse_from_slice(&[38, 5, 196]);
        assert_eq!(
            result,
            Some((ExtendedColorSequence::Ansi256 { index: 196 }, false))
        );
    }

    #[test]
    fn test_parse_256_color_background() {
        let result = ExtendedColorSequence::parse_from_slice(&[48, 5, 196]);
        assert_eq!(
            result,
            Some((ExtendedColorSequence::Ansi256 { index: 196 }, true))
        );
    }

    #[test]
    fn test_parse_rgb_foreground() {
        let result = ExtendedColorSequence::parse_from_slice(&[38, 2, 255, 128, 0]);
        assert_eq!(
            result,
            Some((
                ExtendedColorSequence::Rgb {
                    r: 255,
                    g: 128,
                    b: 0
                },
                false
            ))
        );
    }

    #[test]
    fn test_parse_rgb_background() {
        let result = ExtendedColorSequence::parse_from_slice(&[48, 2, 255, 128, 0]);
        assert_eq!(
            result,
            Some((
                ExtendedColorSequence::Rgb {
                    r: 255,
                    g: 128,
                    b: 0
                },
                true
            ))
        );
    }

    #[test]
    fn test_parse_256_color_boundary_values() {
        // Valid: index 0
        let result = ExtendedColorSequence::parse_from_slice(&[38, 5, 0]);
        assert!(result.is_some());

        // Valid: index 255
        let result = ExtendedColorSequence::parse_from_slice(&[38, 5, 255]);
        assert!(result.is_some());

        // Invalid: index 256 (out of range)
        let result = ExtendedColorSequence::parse_from_slice(&[38, 5, 256]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_rgb_boundary_values() {
        // Valid: all zeros
        let result = ExtendedColorSequence::parse_from_slice(&[38, 2, 0, 0, 0]);
        assert!(result.is_some());

        // Valid: all 255
        let result = ExtendedColorSequence::parse_from_slice(&[38, 2, 255, 255, 255]);
        assert!(result.is_some());

        // Invalid: r out of range
        let result = ExtendedColorSequence::parse_from_slice(&[38, 2, 256, 0, 0]);
        assert!(result.is_none());

        // Invalid: g out of range
        let result = ExtendedColorSequence::parse_from_slice(&[38, 2, 0, 256, 0]);
        assert!(result.is_none());

        // Invalid: b out of range
        let result = ExtendedColorSequence::parse_from_slice(&[38, 2, 0, 0, 256]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_mode() {
        // Invalid mode: 3 (neither 2 nor 5)
        let result = ExtendedColorSequence::parse_from_slice(&[38, 3, 100]);
        assert!(result.is_none());

        // Invalid mode: 1
        let result = ExtendedColorSequence::parse_from_slice(&[48, 1, 100]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_sequence_format() {
        // Too short for 256-color
        let result = ExtendedColorSequence::parse_from_slice(&[38, 5]);
        assert!(result.is_none());

        // Too short for RGB
        let result = ExtendedColorSequence::parse_from_slice(&[38, 2, 255, 128]);
        assert!(result.is_none());

        // Empty slice
        let result = ExtendedColorSequence::parse_from_slice(&[]);
        assert!(result.is_none());

        // Just the color mode
        let result = ExtendedColorSequence::parse_from_slice(&[38]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_with_extra_parameters() {
        // Extra parameters after 256-color should still parse
        let result = ExtendedColorSequence::parse_from_slice(&[38, 5, 196, 99, 88]);
        assert_eq!(
            result,
            Some((ExtendedColorSequence::Ansi256 { index: 196 }, false))
        );

        // Extra parameters after RGB should still parse
        let result = ExtendedColorSequence::parse_from_slice(&[48, 2, 255, 128, 0, 99]);
        assert_eq!(
            result,
            Some((
                ExtendedColorSequence::Rgb {
                    r: 255,
                    g: 128,
                    b: 0
                },
                true
            ))
        );
    }
}

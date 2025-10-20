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
//! The [`SgrColorSequence`] enum provides type-safe parsing of color parameters,
//! ensuring that only valid color values are created. It works with the [`ParamsExt`]
//! trait's [`extract_nth_many_raw()`] method to access the complete parameter slice.
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::SgrColorSequence;
//!
//! // Parse 256-color foreground: ESC[38:5:196m
//! let params = &[38, 5, 196];
//! if let Some(color) = SgrColorSequence::parse_from_raw_slice(params) {
//!     match color {
//!         SgrColorSequence::SetForegroundAnsi256(index) => {
//!             assert_eq!(index, 196);
//!         }
//!         _ => unreachable!(),
//!     }
//! }
//!
//! // Parse RGB background: ESC[48:2:255:128:0m
//! let params = &[48, 2, 255, 128, 0];
//! if let Some(color) = SgrColorSequence::parse_from_raw_slice(params) {
//!     match color {
//!         SgrColorSequence::SetBackgroundRgb(r, g, b) => {
//!             assert_eq!((r, g, b), (255, 128, 0));
//!         }
//!         _ => unreachable!(),
//!     }
//! }
//! ```
//!
//! [`ParamsExt`]: crate::ParamsExt
//! [`extract_nth_many_raw()`]: crate::ParamsExt::extract_nth_many_raw

use super::constants::{CSI_START, CSI_SUB_PARAM_SEPARATOR, SGR_BG_EXTENDED,
                       SGR_COLOR_MODE_256, SGR_COLOR_MODE_RGB, SGR_FG_EXTENDED,
                       SGR_SET_GRAPHICS};
use crate::{AnsiValue, RgbValue, TuiColor,
            core::common::fast_stringify::{BufTextStorage, FastStringify},
            generate_impl_display_for_fast_stringify};
use std::fmt::Result;

/// Which layer (foreground or background) a color applies to.
///
/// This enum cleanly separates the **target layer** from the **color value**,
/// enabling better composition with [`TuiColor`] and other color types.
///
/// [`TuiColor`]: TuiColor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorTarget {
    /// Apply color to foreground (text color)
    Foreground,
    /// Apply color to background (background color)
    Background,
}

/// Extended color sequence operation parsed from VT100 SGR parameters.
///
/// This enum represents the four possible extended color operations that can be
/// parsed from VT100-compliant color sequences, directly encoding both the color
/// type (256-color or RGB) and the target layer (foreground or background).
///
/// # Variants
///
/// - [`SetForegroundAnsi256`](SgrColorSequence::SetForegroundAnsi256): 256-color
///   foreground
///   - Maps to color palette indices 0-255
///   - Sequence format: `ESC[38:5:n` or `ESC[38;5;n`
///
/// - [`SetBackgroundAnsi256`](SgrColorSequence::SetBackgroundAnsi256): 256-color
///   background
///   - Maps to color palette indices 0-255
///   - Sequence format: `ESC[48:5:n` or `ESC[48;5;n`
///
/// - [`SetForegroundRgb`](SgrColorSequence::SetForegroundRgb): True RGB foreground
///   - Each component (r, g, b) ranges from 0-255
///   - Sequence format: `ESC[38:2:r:g:b` or `ESC[38;2;r;g;b`
///
/// - [`SetBackgroundRgb`](SgrColorSequence::SetBackgroundRgb): True RGB background
///   - Each component (r, g, b) ranges from 0-255
///   - Sequence format: `ESC[48:2:r:g:b` or `ESC[48;2;r;g;b`
///
/// # VT100 Specification
///
/// These sequences follow the ISO 8613-6 (ITU-T Rec. T.416) standard:
/// - `38` = foreground color control
/// - `48` = background color control
/// - `5` = 256-color palette mode (next parameter is index)
/// - `2` = RGB mode (next 3 parameters are r, g, b values)
///
/// # Color Palette Layout (256-color mode)
///
/// - **0-15**: Standard ANSI colors (matches basic 16-color palette)
/// - **16-231**: 6×6×6 RGB cube (216 colors)
///   - Formula: `16 + 36r + 6g + b` where r,g,b ∈ `[0,5]`
/// - **232-255**: Grayscale ramp (24 shades from dark to light)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SgrColorSequence {
    /// Set foreground to 256-color palette index (0-255)
    ///
    /// # Example
    /// ```text
    /// ESC[38:5:196m  → Bright red foreground
    /// ```
    SetForegroundAnsi256(u8),

    /// Set background to 256-color palette index (0-255)
    ///
    /// # Example
    /// ```text
    /// ESC[48:5:196m  → Bright red background
    /// ```
    SetBackgroundAnsi256(u8),

    /// Set foreground to RGB true color
    ///
    /// # Example
    /// ```text
    /// ESC[38:2:255:128:0m  → Orange foreground (#FF8000)
    /// ```
    SetForegroundRgb(u8, u8, u8),

    /// Set background to RGB true color
    ///
    /// # Example
    /// ```text
    /// ESC[48:2:255:128:0m  → Orange background (#FF8000)
    /// ```
    SetBackgroundRgb(u8, u8, u8),
}

impl SgrColorSequence {
    /// Parse extended color sequence from a parameter slice.
    ///
    /// Parses both colon-separated and semicolon-separated formats, returning
    /// the appropriate color operation variant. See the module documentation for
    /// comprehensive format details and usage examples.
    ///
    /// # Parameters
    ///
    /// - `params`: The parameter slice from [`extract_nth_many_raw()`]
    ///
    /// # Returns
    ///
    /// - `Some(SgrColorSequence)` - Successfully parsed color operation
    /// - `None` - Invalid or unrecognized sequence
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::SgrColorSequence;
    ///
    /// // 256-color foreground: [38, 5, 196]
    /// let result = SgrColorSequence::parse_from_raw_slice(&[38, 5, 196]);
    /// assert_eq!(result, Some(SgrColorSequence::SetForegroundAnsi256(196)));
    ///
    /// // RGB background: [48, 2, r, g, b]
    /// let result = SgrColorSequence::parse_from_raw_slice(&[48, 2, 255, 128, 0]);
    /// assert_eq!(result, Some(SgrColorSequence::SetBackgroundRgb(255, 128, 0)));
    /// ```
    ///
    /// [`extract_nth_many_raw()`]: crate::ParamsExt::extract_nth_many_raw
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Values are validated <= 255 in guards
    pub fn parse_from_raw_slice(params: &[u16]) -> Option<Self> {
        match params {
            // 256-color foreground: ESC[38:5:n or ESC[38;5;n
            [fg_or_bg, SGR_COLOR_MODE_256, index, ..]
                if *fg_or_bg == SGR_FG_EXTENDED && *index <= 255 =>
            {
                Some(Self::SetForegroundAnsi256(*index as u8))
            }

            // 256-color background: ESC[48:5:n or ESC[48;5;n
            [fg_or_bg, SGR_COLOR_MODE_256, index, ..]
                if *fg_or_bg == SGR_BG_EXTENDED && *index <= 255 =>
            {
                Some(Self::SetBackgroundAnsi256(*index as u8))
            }

            // RGB foreground: ESC[38:2:r:g:b or ESC[38;2;r;g;b
            [fg_or_bg, SGR_COLOR_MODE_RGB, r, g, b, ..]
                if *fg_or_bg == SGR_FG_EXTENDED
                    && *r <= 255
                    && *g <= 255
                    && *b <= 255 =>
            {
                Some(Self::SetForegroundRgb(*r as u8, *g as u8, *b as u8))
            }

            // RGB background: ESC[48:2:r:g:b or ESC[48;2;r;g;b
            [fg_or_bg, SGR_COLOR_MODE_RGB, r, g, b, ..]
                if *fg_or_bg == SGR_BG_EXTENDED
                    && *r <= 255
                    && *g <= 255
                    && *b <= 255 =>
            {
                Some(Self::SetBackgroundRgb(*r as u8, *g as u8, *b as u8))
            }

            // Not an extended color sequence
            _ => None,
        }
    }

    /// Get which layer (foreground or background) this sequence targets.
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::{SgrColorSequence, ColorTarget};
    ///
    /// let seq = SgrColorSequence::SetForegroundAnsi256(42);
    /// assert_eq!(seq.target(), ColorTarget::Foreground);
    ///
    /// let seq = SgrColorSequence::SetBackgroundRgb(255, 0, 0);
    /// assert_eq!(seq.target(), ColorTarget::Background);
    /// ```
    #[must_use]
    pub fn target(&self) -> ColorTarget {
        match self {
            Self::SetForegroundAnsi256(_) | Self::SetForegroundRgb(_, _, _) => {
                ColorTarget::Foreground
            }
            Self::SetBackgroundAnsi256(_) | Self::SetBackgroundRgb(_, _, _) => {
                ColorTarget::Background
            }
        }
    }
}

impl From<SgrColorSequence> for TuiColor {
    /// Convert an extended color sequence to a normalized [`TuiColor`].
    ///
    /// This converts both 256-color palette and RGB color sequences to their
    /// corresponding `TuiColor` variants. The layer information (foreground/background)
    /// is preserved separately via [`SgrColorSequence::target()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::{SgrColorSequence, TuiColor};
    ///
    /// // 256-color → TuiColor::Ansi
    /// let seq = SgrColorSequence::SetForegroundAnsi256(196);
    /// let color = TuiColor::from(seq);
    /// assert!(matches!(color, TuiColor::Ansi(_)));
    ///
    /// // RGB → TuiColor::Rgb
    /// let seq = SgrColorSequence::SetBackgroundRgb(255, 128, 0);
    /// let color = TuiColor::from(seq);
    /// assert!(matches!(color, TuiColor::Rgb(_)));
    ///
    /// // Or using into()
    /// let color: TuiColor = SgrColorSequence::SetForegroundAnsi256(42).into();
    /// assert!(matches!(color, TuiColor::Ansi(_)));
    /// ```
    fn from(seq: SgrColorSequence) -> Self {
        match seq {
            SgrColorSequence::SetForegroundAnsi256(index)
            | SgrColorSequence::SetBackgroundAnsi256(index) => {
                TuiColor::Ansi(AnsiValue::new(index))
            }
            SgrColorSequence::SetForegroundRgb(r, g, b)
            | SgrColorSequence::SetBackgroundRgb(r, g, b) => {
                TuiColor::Rgb(RgbValue::from_u8(r, g, b))
            }
        }
    }
}

/// Sequence generation implementations (bidirectional pattern).
///
/// Like `DsrSequence` and `OscSequence`, `SgrColorSequence` implements both parsing
/// (`parse_from_slice`) and generation (`FastStringify` + `Display`) for bidirectional
/// use:
/// - Parsing: Convert incoming bytes → `SgrColorSequence` enum
/// - Generation: Convert `SgrColorSequence` enum → ANSI escape string
///
/// This enables type-safe, infallible test sequence generation without raw escape
/// strings.
impl FastStringify for SgrColorSequence {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
        acc.push_str(CSI_START);
        match self {
            SgrColorSequence::SetForegroundAnsi256(index) => {
                acc.push_str(&SGR_FG_EXTENDED.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&SGR_COLOR_MODE_256.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&index.to_string());
            }
            SgrColorSequence::SetBackgroundAnsi256(index) => {
                acc.push_str(&SGR_BG_EXTENDED.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&SGR_COLOR_MODE_256.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&index.to_string());
            }
            SgrColorSequence::SetForegroundRgb(r, g, b) => {
                acc.push_str(&SGR_FG_EXTENDED.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&SGR_COLOR_MODE_RGB.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&r.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&g.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&b.to_string());
            }
            SgrColorSequence::SetBackgroundRgb(r, g, b) => {
                acc.push_str(&SGR_BG_EXTENDED.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&SGR_COLOR_MODE_RGB.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&r.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&g.to_string());
                acc.push(CSI_SUB_PARAM_SEPARATOR);
                acc.push_str(&b.to_string());
            }
        }
        acc.push(SGR_SET_GRAPHICS);
        Ok(())
    }
}

generate_impl_display_for_fast_stringify!(SgrColorSequence);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_256_color_foreground() {
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 5, 196]);
        assert_eq!(result, Some(SgrColorSequence::SetForegroundAnsi256(196)));
    }

    #[test]
    fn test_parse_256_color_background() {
        let result = SgrColorSequence::parse_from_raw_slice(&[48, 5, 196]);
        assert_eq!(result, Some(SgrColorSequence::SetBackgroundAnsi256(196)));
    }

    #[test]
    fn test_parse_rgb_foreground() {
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 2, 255, 128, 0]);
        assert_eq!(
            result,
            Some(SgrColorSequence::SetForegroundRgb(255, 128, 0))
        );
    }

    #[test]
    fn test_parse_rgb_background() {
        let result = SgrColorSequence::parse_from_raw_slice(&[48, 2, 255, 128, 0]);
        assert_eq!(
            result,
            Some(SgrColorSequence::SetBackgroundRgb(255, 128, 0))
        );
    }

    #[test]
    fn test_parse_256_color_boundary_values() {
        // Valid: index 0
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 5, 0]);
        assert!(result.is_some());

        // Valid: index 255
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 5, 255]);
        assert!(result.is_some());

        // Invalid: index 256 (out of range)
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 5, 256]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_rgb_boundary_values() {
        // Valid: all zeros
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 2, 0, 0, 0]);
        assert!(result.is_some());

        // Valid: all 255
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 2, 255, 255, 255]);
        assert!(result.is_some());

        // Invalid: r out of range
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 2, 256, 0, 0]);
        assert!(result.is_none());

        // Invalid: g out of range
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 2, 0, 256, 0]);
        assert!(result.is_none());

        // Invalid: b out of range
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 2, 0, 0, 256]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_mode() {
        // Invalid mode: 3 (neither 2 nor 5)
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 3, 100]);
        assert!(result.is_none());

        // Invalid mode: 1
        let result = SgrColorSequence::parse_from_raw_slice(&[48, 1, 100]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_sequence_format() {
        // Too short for 256-color
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 5]);
        assert!(result.is_none());

        // Too short for RGB
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 2, 255, 128]);
        assert!(result.is_none());

        // Empty slice
        let result = SgrColorSequence::parse_from_raw_slice(&[]);
        assert!(result.is_none());

        // Just the color mode
        let result = SgrColorSequence::parse_from_raw_slice(&[38]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_with_extra_parameters() {
        // Extra parameters after 256-color should still parse
        let result = SgrColorSequence::parse_from_raw_slice(&[38, 5, 196, 99, 88]);
        assert_eq!(result, Some(SgrColorSequence::SetForegroundAnsi256(196)));

        // Extra parameters after RGB should still parse
        let result = SgrColorSequence::parse_from_raw_slice(&[48, 2, 255, 128, 0, 99]);
        assert_eq!(
            result,
            Some(SgrColorSequence::SetBackgroundRgb(255, 128, 0))
        );
    }

    // Tests for sequence generation (FastStringify + Display).

    #[test]
    fn test_display_foreground_ansi256() {
        let sequence = SgrColorSequence::SetForegroundAnsi256(196);
        assert_eq!(sequence.to_string(), "\x1b[38:5:196m");
    }

    #[test]
    fn test_display_background_ansi256() {
        let sequence = SgrColorSequence::SetBackgroundAnsi256(21);
        assert_eq!(sequence.to_string(), "\x1b[48:5:21m");
    }

    #[test]
    fn test_display_foreground_rgb() {
        let sequence = SgrColorSequence::SetForegroundRgb(255, 128, 0);
        assert_eq!(sequence.to_string(), "\x1b[38:2:255:128:0m");
    }

    #[test]
    fn test_display_background_rgb() {
        let sequence = SgrColorSequence::SetBackgroundRgb(0, 128, 255);
        assert_eq!(sequence.to_string(), "\x1b[48:2:0:128:255m");
    }

    #[test]
    fn test_display_boundary_values() {
        // 256-color boundaries
        let min_256 = SgrColorSequence::SetForegroundAnsi256(0);
        assert_eq!(min_256.to_string(), "\x1b[38:5:0m");

        let max_256 = SgrColorSequence::SetBackgroundAnsi256(255);
        assert_eq!(max_256.to_string(), "\x1b[48:5:255m");

        // RGB boundaries
        let black_rgb = SgrColorSequence::SetForegroundRgb(0, 0, 0);
        assert_eq!(black_rgb.to_string(), "\x1b[38:2:0:0:0m");

        let white_rgb = SgrColorSequence::SetBackgroundRgb(255, 255, 255);
        assert_eq!(white_rgb.to_string(), "\x1b[48:2:255:255:255m");
    }

    #[test]
    fn test_roundtrip_parsing_and_display() {
        // Parse a sequence, then generate it back - should produce valid (but not
        // necessarily identical) sequence. We use colon format for generation,
        // but parsing accepts both.

        // 256-color foreground
        let parsed = SgrColorSequence::parse_from_raw_slice(&[38, 5, 196]).unwrap();
        let generated = parsed.to_string();
        assert_eq!(generated, "\x1b[38:5:196m");

        // RGB background
        let parsed =
            SgrColorSequence::parse_from_raw_slice(&[48, 2, 255, 128, 0]).unwrap();
        let generated = parsed.to_string();
        assert_eq!(generated, "\x1b[48:2:255:128:0m");
    }
}

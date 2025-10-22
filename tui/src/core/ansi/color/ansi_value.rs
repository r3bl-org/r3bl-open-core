// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI 256-color palette representation.
//!
//! This provides a good balance between color precision and terminal compatibility.
//! Each index (0-255) maps to a specific color in the palette.

use super::{RgbValue,
            convert::{ansi_constants::ANSI_COLOR_PALETTE, convert_rgb_into_grayscale}};
use crate::TransformColor;

/// Represents a color in the ANSI 256-color palette format.
///
/// This provides a good balance between color precision and terminal compatibility.
/// Each index (0-255) maps to a specific color in the palette.
#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
pub struct AnsiValue {
    pub index: u8,
}

impl From<u8> for AnsiValue {
    fn from(index: u8) -> Self { Self { index } }
}

impl From<u16> for AnsiValue {
    fn from(value: u16) -> Self {
        debug_assert!(
            value <= 255,
            "AnsiValue must represent a valid 256-color palette index (0-255), got {value}"
        );
        #[allow(clippy::cast_possible_truncation)]
        let index = value as u8;
        Self { index }
    }
}

impl From<i32> for AnsiValue {
    fn from(value: i32) -> Self {
        debug_assert!(
            (0..=255).contains(&value),
            "AnsiValue must represent a valid 256-color palette index (0-255), got {value}"
        );
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = value as u8;
        Self { index }
    }
}

impl TransformColor for AnsiValue {
    fn as_grayscale(&self) -> AnsiValue {
        let index = self.index as usize;
        let rgb = ANSI_COLOR_PALETTE[index];
        let rgb = RgbValue::from(rgb);
        AnsiValue::from(convert_rgb_into_grayscale(rgb))
    }

    fn as_rgb(&self) -> RgbValue {
        let index = self.index as usize;
        ANSI_COLOR_PALETTE[index].into()
    }

    fn as_ansi(&self) -> AnsiValue { *self }
}

impl AnsiValue {
    /// Create a new ANSI color value.
    #[must_use]
    pub const fn new(color: u8) -> Self { Self { index: color } }

    /// Check if this is a basic ANSI color (indices 0-15).
    ///
    /// Basic ANSI colors (indices 0-15) have special handling:
    /// - They represent the standard 16 terminal colors
    /// - Color degradation treats them differently than extended colors (16-255)
    /// - When converting to grayscale, we convert via RGB first
    ///
    /// # Returns
    ///
    /// `true` if this is a basic color (0-15), `false` for extended colors (16-255)
    #[must_use]
    pub const fn is_basic(&self) -> bool { self.index < 16 }

    /// Check if this is an extended ANSI color (indices 16-255).
    ///
    /// Extended ANSI colors (indices 16-255) are from the 256-color palette:
    /// - Indices 16-231: 6×6×6 RGB color cube (216 colors)
    /// - Indices 232-255: Grayscale ramp (24 shades)
    /// - They don't have the special handling that basic colors (0-15) require
    ///
    /// # Returns
    ///
    /// `true` if this is an extended color (16-255), `false` for basic colors (0-15)
    #[must_use]
    pub const fn is_extended(&self) -> bool { !self.is_basic() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;
    use test_case::test_case;

    /// <https://www.ditig.com/256-colors-cheat-sheet>
    /// ANSI: 57 `BlueViolet`
    /// RGB: #5f00ff rgb(95,0,255)
    #[test]
    fn test_ansi_to_rgb() {
        let ansi = AnsiValue::new(57);
        let rgb = RgbValue::from(ansi);
        assert_eq2!(rgb, RgbValue::from_u8(95, 0, 255));
    }

    #[test_case(AnsiValue{index: 42}, RgbValue{red: 0, green: 215, blue: 135})]
    fn test_ansi256_color_as_rgb(ansi_color: AnsiValue, rgb_color: RgbValue) {
        assert_eq!(ansi_color.as_rgb(), rgb_color);
    }

    #[test_case(RgbValue{red: 0, green: 0, blue: 0}, 16)]
    #[test_case(RgbValue{red: 0, green: 128, blue: 255}, 33)]
    fn test_ansi256_color_as_ansi256(rgb_color: RgbValue, index: u8) {
        let expected_ansi = AnsiValue { index };
        assert_eq!(rgb_color.as_ansi(), expected_ansi);
    }

    #[test_case(RgbValue{red: 0, green: 128, blue: 255}, 245)]
    #[test_case(RgbValue{red: 255, green: 255, blue: 255}, 231)]
    fn test_ansi256_color_as_grayscale(rgb_color: RgbValue, index: u8) {
        let expected_gray = AnsiValue { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
    }

    #[test]
    fn test_ansi_value_from_u8() {
        let ansi = AnsiValue::from(42u8);
        assert_eq2!(ansi.index, 42);
    }

    #[test]
    fn test_ansi_value_from_u16() {
        let ansi = AnsiValue::from(100u16);
        assert_eq2!(ansi.index, 100);
    }

    #[test]
    fn test_ansi_value_from_i32() {
        let ansi = AnsiValue::from(150i32);
        assert_eq2!(ansi.index, 150);
    }

    #[test]
    fn test_is_basic_color() {
        // All basic colors (0-15) should return true
        for i in 0..16 {
            let ansi = AnsiValue::from(i);
            assert!(ansi.is_basic(), "AnsiValue({i}) should be basic");
        }

        // Extended colors (16-255) should return false
        let extended_colors = [16, 50, 100, 196, 255];
        for &i in &extended_colors {
            let ansi = AnsiValue::from(i);
            assert!(!ansi.is_basic(), "AnsiValue({i}) should not be basic");
        }
    }

    #[test]
    fn test_is_extended_color() {
        // All basic colors (0-15) should return false
        for i in 0..16 {
            let ansi = AnsiValue::from(i);
            assert!(!ansi.is_extended(), "AnsiValue({i}) should not be extended");
        }

        // Extended colors (16-255) should return true
        let extended_colors = [16, 50, 100, 196, 232, 255];
        for &i in &extended_colors {
            let ansi = AnsiValue::from(i);
            assert!(ansi.is_extended(), "AnsiValue({i}) should be extended");
        }
    }

    #[test]
    fn test_is_basic_and_extended_are_complementary() {
        // Every color is either basic or extended, not both
        for i in 0..=255 {
            let ansi = AnsiValue::from(i);
            let is_basic = ansi.is_basic();
            let is_extended = ansi.is_extended();

            // They should be mutually exclusive
            assert_ne!(
                is_basic, is_extended,
                "AnsiValue({i}) cannot be both basic and extended"
            );

            // One of them must be true
            assert!(
                is_basic || is_extended,
                "AnsiValue({i}) must be either basic or extended"
            );
        }
    }
}

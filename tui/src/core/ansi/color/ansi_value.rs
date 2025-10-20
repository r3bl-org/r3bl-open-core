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
    #[must_use]
    pub const fn new(color: u8) -> Self { Self { index: color } }
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
}

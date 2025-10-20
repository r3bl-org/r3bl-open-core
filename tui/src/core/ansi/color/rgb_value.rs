// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! RGB (24-bit truecolor) color representation.
//!
//! This is the most precise color representation supported by modern terminals.

use super::{AnsiValue, convert::convert_rgb_into_ansi256};
use crate::{TransformColor,
            common::{CommonError, CommonErrorType, CommonResult}};

/// Represents a color in RGB (24-bit truecolor) format.
///
/// This is the most precise color representation supported by modern terminals.
#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
pub struct RgbValue {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl From<(u8, u8, u8)> for RgbValue {
    fn from((red, green, blue): (u8, u8, u8)) -> Self { Self::from_u8(red, green, blue) }
}

impl From<u32> for RgbValue {
    fn from(value: u32) -> Self {
        use crate::common::LossyConvertToByte;
        let red = ((value >> 16) & 0xFF).to_u8_lossy();
        let green = ((value >> 8) & 0xFF).to_u8_lossy();
        let blue = (value & 0xFF).to_u8_lossy();
        Self { red, green, blue }
    }
}

impl Default for RgbValue {
    fn default() -> Self { Self::from_u8(255, 255, 255) }
}

impl RgbValue {
    #[must_use]
    pub fn from_u8(red: u8, green: u8, blue: u8) -> Self { Self { red, green, blue } }

    #[must_use]
    pub fn from_f32(red: f32, green: f32, blue: f32) -> Self {
        use crate::common::LossyConvertToByte;
        Self {
            red: (red * 255.0).to_u8_lossy(),
            green: (green * 255.0).to_u8_lossy(),
            blue: (blue * 255.0).to_u8_lossy(),
        }
    }

    /// # Errors
    ///
    /// Returns an error if the input string is not a valid hex color format.
    ///
    /// See [`CommonResult`] for error details.
    ///
    /// [`CommonResult`]: crate::common::CommonResult
    pub fn try_from_hex_color(input: &str) -> CommonResult<RgbValue> {
        use crate::tui_style::hex_color_parser::parse_hex_color;
        match parse_hex_color(input) {
            Ok((_, color)) => Ok(color),
            Err(_) => CommonError::new_error_result_with_only_type(
                CommonErrorType::InvalidHexColorFormat,
            ),
        }
    }

    /// # Panics
    ///
    /// This function will panic if the input string is not a valid hex color format.
    #[must_use]
    pub fn from_hex(input: &str) -> RgbValue {
        use crate::tui_style::hex_color_parser::parse_hex_color;
        #[allow(clippy::match_wild_err_arm)]
        match parse_hex_color(input) {
            Ok((_, color)) => color,
            Err(_) => {
                panic!("Invalid hex color format: {input}")
            }
        }
    }
}

impl TransformColor for RgbValue {
    fn as_rgb(&self) -> RgbValue { *self }

    fn as_ansi(&self) -> AnsiValue { convert_rgb_into_ansi256(*self) }

    fn as_grayscale(&self) -> AnsiValue { convert_rgb_into_ansi256(*self).as_grayscale() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;
    use test_case::test_case;

    #[test]
    fn test_new() {
        let value = RgbValue::from_u8(1, 2, 3);
        assert_eq2!((value.red, value.green, value.blue), (1, 2, 3));
    }

    #[test]
    fn test_try_from_hex_color() {
        // Valid.
        {
            let hex_color = "#ff0000";
            let value = RgbValue::try_from_hex_color(hex_color).unwrap();
            assert_eq2!((value.red, value.green, value.blue), (255, 0, 0));
        }

        // Invalid.
        {
            let hex_color = "#ff000";
            let value = RgbValue::try_from_hex_color(hex_color);
            assert!(value.is_err());
        }
    }

    #[test]
    fn test_rgb_value_default() {
        let default_rgb = RgbValue::default();
        assert_eq2!(default_rgb, RgbValue::from_u8(255, 255, 255));
    }

    #[test_case(RgbValue{red: 0, green: 0, blue: 0})]
    #[test_case(RgbValue{red: 0, green: 128, blue: 255})]
    #[test_case(RgbValue{red: 255, green: 255, blue: 255})]
    fn test_rgb_color_as_rgb(rgb_color: RgbValue) {
        assert_eq!(rgb_color.as_rgb(), rgb_color);
    }

    #[test_case(RgbValue{red: 0, green: 0, blue: 0}, 16)]
    #[test_case(RgbValue{red: 0, green: 128, blue: 255}, 33)]
    fn test_rgb_color_as_ansi256(rgb_color: RgbValue, index: u8) {
        let expected_ansi = AnsiValue { index };
        assert_eq!(rgb_color.as_ansi(), expected_ansi);
    }

    #[test_case(RgbValue{red: 0, green: 128, blue: 255}, 245)]
    #[test_case(RgbValue{red: 128, green: 128, blue: 128}, 244)]
    fn test_rgb_color_as_grayscale(rgb_color: RgbValue, index: u8) {
        let expected_gray = AnsiValue { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
    }

    /// <https://www.ditig.com/256-colors-cheat-sheet>
    /// ANSI: 57 `BlueViolet`
    /// RGB: #5f00ff rgb(95,0,255)
    #[test]
    fn test_rgb_to_ansi() {
        let rgb = RgbValue::from_u8(95, 0, 255);
        let ansi = AnsiValue::from(rgb);
        assert_eq2!(ansi, AnsiValue::new(57));
    }
}

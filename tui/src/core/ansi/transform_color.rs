// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AnsiValue, RgbValue};

pub trait TransformColor {
    /// Returns a [`RgbValue`] representation of the `self` color.
    fn as_rgb(&self) -> RgbValue;

    /// Returns the index of a color in 256-color ANSI palette approximating the `self`
    /// color.
    fn as_ansi(&self) -> AnsiValue;

    /// Returns the index of a color in 256-color ANSI palette approximating the `self`
    /// color as grayscale.
    fn as_grayscale(&self) -> AnsiValue;
}

#[cfg(test)]
mod tests {
    use super::TransformColor;
    use crate::{ASTColor, AnsiValue, RgbValue};
    use test_case::test_case;

    #[test_case(0, 0, 0)]
    #[test_case(255, 125, 0)]
    #[test_case(255, 255, 255)]
    fn test_color_as_rgb(red: u8, green: u8, blue: u8) {
        let rgb_color = ASTColor::Rgb((red, green, blue).into());
        assert_eq!(rgb_color.as_rgb(), RgbValue { red, green, blue });
    }

    #[test_case(ASTColor::Rgb((255, 255, 255).into()), 231)]
    #[test_case(ASTColor::Rgb((255, 128, 0).into()), 208)]
    fn test_color_as_ansi256(rgb_color: crate::ASTColor, index: u8) {
        let expected_ansi = AnsiValue { index };
        assert_eq!(rgb_color.as_ansi(), expected_ansi);
    }

    #[test_case(RgbValue{red: 0, green: 0, blue: 0})]
    #[test_case(RgbValue{red: 0, green: 128, blue: 255})]
    #[test_case(RgbValue{red: 255, green: 255, blue: 255})]
    fn test_rgb_color_as_rgb(rgb_color: RgbValue) {
        assert_eq!(rgb_color.as_rgb(), rgb_color);
    }

    #[test_case(AnsiValue{index: 42}, RgbValue{red: 0, green: 215, blue: 135})]
    fn test_ansi256_color_as_rgb(ansi_color: AnsiValue, rgb_color: RgbValue) {
        assert_eq!(ansi_color.as_rgb(), rgb_color);
    }

    #[test_case(RgbValue{red: 0, green: 0, blue: 0}, 16)]
    #[test_case(RgbValue{red: 0, green: 128, blue: 255}, 33)]
    fn test_rgb_color_as_ansi256(rgb_color: RgbValue, index: u8) {
        let expected_ansi = AnsiValue { index };
        assert_eq!(rgb_color.as_ansi(), expected_ansi);
    }

    #[test_case(ASTColor::Rgb((0, 0, 0).into()), 16)]
    #[test_case(ASTColor::Rgb((255, 128, 0).into()), 249)]
    fn test_color_as_grayscale(rgb_color: crate::ASTColor, index: u8) {
        let expected_gray = AnsiValue { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
    }

    #[test_case(RgbValue{red: 0, green: 128, blue: 255}, 245)]
    #[test_case(RgbValue{red: 128, green: 128, blue: 128}, 244)]
    fn test_rgb_color_as_grayscale(rgb_color: RgbValue, index: u8) {
        let expected_gray = AnsiValue { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
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
}

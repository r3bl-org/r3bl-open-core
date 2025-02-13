/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use crate::{Ansi256Color, RgbColor};

pub trait TransformColor {
    /// Returns a [RgbColor] representation of the `self` color.
    fn as_rgb(&self) -> RgbColor;

    /// Returns the index of a color in 256-color ANSI palette approximating the `self`
    /// color.
    fn as_ansi256(&self) -> Ansi256Color;

    /// Returns the index of a color in 256-color ANSI palette approximating the `self`
    /// color as grayscale.
    fn as_grayscale(&self) -> Ansi256Color;
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::TransformColor;
    use crate::{Ansi256Color, Color, RgbColor};

    #[test_case(0, 0, 0)]
    #[test_case(255, 125, 0)]
    #[test_case(255, 255, 255)]
    fn test_color_as_rgb(red: u8, green: u8, blue: u8) {
        let rgb_color = Color::Rgb(red, green, blue);
        assert_eq!(rgb_color.as_rgb(), RgbColor { red, green, blue });
    }

    #[test_case(Color::Rgb(255, 255, 255), 231)]
    #[test_case(Color::Rgb(255, 128, 0), 208)]
    fn test_color_as_ansi256(rgb_color: crate::Color, index: u8) {
        let expected_ansi = Ansi256Color { index };
        assert_eq!(rgb_color.as_ansi256(), expected_ansi);
    }

    #[test_case(RgbColor{red: 0, green: 0, blue: 0})]
    #[test_case(RgbColor{red: 0, green: 128, blue: 255})]
    #[test_case(RgbColor{red: 255, green: 255, blue: 255})]
    fn test_rgb_color_as_rgb(rgb_color: RgbColor) {
        assert_eq!(rgb_color.as_rgb(), rgb_color);
    }

    #[test_case(Ansi256Color{index: 42}, RgbColor{red: 0, green: 215, blue: 135})]
    fn test_ansi256_color_as_rgb(ansi_color: Ansi256Color, rgb_color: RgbColor) {
        assert_eq!(ansi_color.as_rgb(), rgb_color);
    }

    #[test_case(RgbColor{red: 0, green: 0, blue: 0}, 16)]
    #[test_case(RgbColor{red: 0, green: 128, blue: 255}, 33)]
    fn test_rgb_color_as_ansi256(rgb_color: RgbColor, index: u8) {
        let expected_ansi = Ansi256Color { index };
        assert_eq!(rgb_color.as_ansi256(), expected_ansi);
    }

    #[test_case(Color::Rgb(0, 0, 0), 16)]
    #[test_case(Color::Rgb(255, 128, 0), 249)]
    fn test_color_as_grayscale(rgb_color: crate::Color, index: u8) {
        let expected_gray = Ansi256Color { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
    }

    #[test_case(RgbColor{red: 0, green: 128, blue: 255}, 245)]
    #[test_case(RgbColor{red: 128, green: 128, blue: 128}, 244)]
    fn test_rgb_color_as_grayscale(rgb_color: RgbColor, index: u8) {
        let expected_gray = Ansi256Color { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
    }

    #[test_case(RgbColor{red: 0, green: 0, blue: 0}, 16)]
    #[test_case(RgbColor{red: 0, green: 128, blue: 255}, 33)]
    fn test_ansi256_color_as_ansi256(rgb_color: RgbColor, index: u8) {
        let expected_ansi = Ansi256Color { index };
        assert_eq!(rgb_color.as_ansi256(), expected_ansi);
    }

    #[test_case(RgbColor{red: 0, green: 128, blue: 255}, 245)]
    #[test_case(RgbColor{red: 255, green: 255, blue: 255}, 231)]
    fn test_ansi256_color_as_grayscale(rgb_color: RgbColor, index: u8) {
        let expected_gray = Ansi256Color { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
    }
}

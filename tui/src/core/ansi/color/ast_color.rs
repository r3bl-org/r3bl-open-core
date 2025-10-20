// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! More info:
//! - <https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg>
//! - <https://www.ditig.com/256-colors-cheat-sheet>
//! - <https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences>
//! - <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit>
//! - <https://en.wikipedia.org/wiki/ANSI_escape_code#24-bit>
//! - <https://en.wikipedia.org/wiki/ANSI_escape_code#Unix_environment_variables_relating_to_color_support>
//! - <https://en.wikipedia.org/wiki/8-bit_color>
//! - <https://github.com/Qix-/color-convert/>

use super::convert::{ansi_constants::ANSI_COLOR_PALETTE, convert_rgb_into_ansi256};
use crate::{AnsiValue, RgbValue, TransformColor};

/// This is the "top-level" color type that is used in this crate. For example this is
/// used in `ASTStyle` to represent the foreground and background colors.
/// - The other color types are "lower-level" and are used to convert between different
///   color types.
/// - The [`TransformColor`] trait is used to convert between a "top-level" color type and
///   a "lower-level" color type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ASTColor {
    Rgb(RgbValue),
    Ansi(AnsiValue),
}

impl Default for ASTColor {
    fn default() -> Self { ASTColor::Rgb((0, 0, 0).into()) }
}

impl From<AnsiValue> for ASTColor {
    fn from(ansi: AnsiValue) -> Self { ASTColor::Ansi(ansi) }
}

impl From<ASTColor> for RgbValue {
    fn from(ast_color: ASTColor) -> Self { ast_color.as_rgb() }
}

impl From<RgbValue> for ASTColor {
    fn from(rgb_value: RgbValue) -> Self { ASTColor::Rgb(rgb_value) }
}

impl TransformColor for ASTColor {
    fn as_rgb(&self) -> RgbValue {
        match self {
            ASTColor::Rgb(rgb_value) => *rgb_value,
            ASTColor::Ansi(ansi_value) => {
                let rgb_color: RgbValue =
                    ANSI_COLOR_PALETTE[ansi_value.index as usize].into();
                (rgb_color.red, rgb_color.green, rgb_color.blue).into()
            }
        }
    }

    fn as_ansi(&self) -> AnsiValue {
        match self {
            ASTColor::Rgb(rgb_value) => convert_rgb_into_ansi256(RgbValue {
                red: rgb_value.red,
                green: rgb_value.green,
                blue: rgb_value.blue,
            }),
            ASTColor::Ansi(ansi_value) => *ansi_value,
        }
    }

    fn as_grayscale(&self) -> AnsiValue {
        match self {
            ASTColor::Rgb(rgb_value) => convert_rgb_into_ansi256(RgbValue {
                red: rgb_value.red,
                green: rgb_value.green,
                blue: rgb_value.blue,
            })
            .as_grayscale(),
            ASTColor::Ansi(ansi) => {
                let ansi = *ansi;
                ansi.as_grayscale()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_color_as_ansi256(rgb_color: ASTColor, index: u8) {
        let expected_ansi = AnsiValue { index };
        assert_eq!(rgb_color.as_ansi(), expected_ansi);
    }

    #[test_case(ASTColor::Rgb((0, 0, 0).into()), 16)]
    #[test_case(ASTColor::Rgb((255, 128, 0).into()), 249)]
    fn test_color_as_grayscale(rgb_color: ASTColor, index: u8) {
        let expected_gray = AnsiValue { index };
        assert_eq!(rgb_color.as_grayscale(), expected_gray);
    }
}

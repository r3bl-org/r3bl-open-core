/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

//! More info:
//! - <https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg>
//! - <https://www.ditig.com/256-colors-cheat-sheet>
//! - <https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences>
//! - <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit>
//! - <https://en.wikipedia.org/wiki/ANSI_escape_code#24-bit>
//! - <https://en.wikipedia.org/wiki/ANSI_escape_code#Unix_environment_variables_relating_to_color_support>
//! - <https://en.wikipedia.org/wiki/8-bit_color>
//! - <https://github.com/Qix-/color-convert/>

use crate::{AnsiValue, RgbValue, TransformColor, convert_rgb_into_ansi256,
            core::ansi::ansi_constants::ANSI_COLOR_PALETTE};

/// This is the "top-level" color type that is used in this crate. For example this is
/// used in [`super::ASTStyle`] to represent the foreground and background colors.
/// - The other color types are "lower-level" and are used to convert between different
///   color types.
/// - The [`TransformColor`] trait is used to convert between a "top-level" color type and
///   a "lower-level" color type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ASTColor {
    Rgb(RgbValue),
    Ansi(AnsiValue),
}

mod ast_color_impl_block {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
}

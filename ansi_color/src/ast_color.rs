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

use crate::{TransformColor,
            color_utils,
            constants::ANSI_COLOR_PALETTE,
            convert_rgb_into_ansi256};

/// This is the "top-level" color type that is used in this crate. For example this is
/// used in [super::ASTStyle] to represent the foreground and background colors.
/// - The other color types are "lower-level" and are used to convert between different
///   color types.
/// - The [TransformColor] trait is used to convert between a "top-level" color type and a
///   "lower-level" color type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ASTColor {
    Rgb(u8, u8, u8),
    Ansi256(u8),
}

mod ast_color_impl_block {
    use super::*;

    impl Default for ASTColor {
        fn default() -> Self { ASTColor::Rgb(0, 0, 0) }
    }

    impl From<RgbColor> for ASTColor {
        fn from(rgb: RgbColor) -> Self { ASTColor::Rgb(rgb.red, rgb.green, rgb.blue) }
    }

    impl From<Ansi256Color> for ASTColor {
        fn from(ansi256: Ansi256Color) -> Self { ASTColor::Ansi256(ansi256.index) }
    }

    impl From<ASTColor> for RgbColor {
        fn from(ast_color: ASTColor) -> Self { ast_color.as_rgb() }
    }

    impl TransformColor for ASTColor {
        fn as_rgb(&self) -> RgbColor {
            match self {
                ASTColor::Rgb(r, g, b) => RgbColor {
                    red: *r,
                    green: *g,
                    blue: *b,
                },
                ASTColor::Ansi256(index) => Ansi256Color { index: *index }.as_rgb(),
            }
        }

        fn as_ansi256(&self) -> Ansi256Color {
            match self {
                ASTColor::Rgb(red, green, blue) => convert_rgb_into_ansi256(RgbColor {
                    red: *red,
                    green: *green,
                    blue: *blue,
                }),
                ASTColor::Ansi256(index) => Ansi256Color { index: *index },
            }
        }

        fn as_grayscale(&self) -> Ansi256Color {
            match self {
                ASTColor::Rgb(red, green, blue) => convert_rgb_into_ansi256(RgbColor {
                    red: *red,
                    green: *green,
                    blue: *blue,
                })
                .as_grayscale(),
                ASTColor::Ansi256(index) => Ansi256Color { index: *index }.as_grayscale(),
            }
        }
    }
}

/// This is a "lower-level" color type that is used to hold RGB values. And it can be converted
/// into other color types (both low and high level ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// Very similar to `tui_color!` in `r3bl_tui` crate.
#[macro_export]
macro_rules! rgb_color {
    (dark_pink) => {
        $crate::RgbColor::from((203, 85, 121))
    };

    (pink) => {
        $crate::RgbColor::from((195, 106, 138))
    };

    (lizard_green) => {
        $crate::RgbColor::from((20, 244, 0))
    };

    (dark_lizard_green) => {
        $crate::RgbColor::from((10, 122, 0))
    };

    (slate_grey) => {
        $crate::RgbColor::from((94, 103, 111))
    };

    (silver_metallic) => {
        $crate::RgbColor::from((213, 217, 220))
    };

    (frozen_blue) => {
        $crate::RgbColor::from((171, 204, 242))
    };

    (moonlight_blue) => {
        $crate::RgbColor::from((31, 36, 46))
    };

    (night_blue) => {
        $crate::RgbColor::from((14, 17, 23))
    };

    (guards_red) => {
        $crate::RgbColor::from((200, 1, 1))
    };

    (orange) => {
        $crate::RgbColor::from((255, 132, 18))
    };

    (black) => {
        $crate::RgbColor::from((0, 0, 0))
    };

    (dark_grey) => {
        $crate::RgbColor::from((64, 64, 64))
    };

    (red) => {
        $crate::RgbColor::from((255, 0, 0))
    };

    (dark_red) => {
        $crate::RgbColor::from((139, 0, 0))
    };

    (green) => {
        $crate::RgbColor::from((0, 255, 0))
    };

    (dark_green) => {
        $crate::RgbColor::from((0, 100, 0))
    };

    (yellow) => {
        $crate::RgbColor::from((255, 255, 0))
    };

    (dark_yellow) => {
        $crate::RgbColor::from((204, 204, 0))
    };

    (blue) => {
        $crate::RgbColor::from((0, 0, 255))
    };

    (dark_blue) => {
        $crate::RgbColor::from((0, 0, 139))
    };

    (magenta) => {
        $crate::RgbColor::from((255, 0, 255))
    };

    (dark_magenta) => {
        $crate::RgbColor::from((139, 0, 139))
    };

    (cyan) => {
        $crate::RgbColor::from((0, 255, 255))
    };

    (dark_cyan) => {
        $crate::RgbColor::from((0, 139, 139))
    };

    (white) => {
        $crate::RgbColor::from((255, 255, 255))
    };

    (grey) => {
        $crate::RgbColor::from((192, 192, 192))
    };

    (
        $arg_r : expr,
        $arg_g : expr,
        $arg_b : expr
        $(,)? /* optional trailing comma */
    ) => {
        $crate::RgbColor::from(($arg_r, $arg_g, $arg_b))
    };
}

mod rgb_color_impl_block {
    use super::*;

    impl From<(u8, u8, u8)> for RgbColor {
        fn from(rgb: (u8, u8, u8)) -> Self {
            RgbColor {
                red: rgb.0,
                green: rgb.1,
                blue: rgb.2,
            }
        }
    }

    impl TransformColor for RgbColor {
        fn as_rgb(&self) -> RgbColor { *self }

        fn as_ansi256(&self) -> Ansi256Color { convert_rgb_into_ansi256(*self) }

        fn as_grayscale(&self) -> Ansi256Color {
            convert_rgb_into_ansi256(*self).as_grayscale()
        }
    }
}

/// This is a "lower-level" color type that is used to hold an index into the 256-color palette.
/// And it can be converted into other color types (both low and high level ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ansi256Color {
    pub index: u8,
}

mod ansi_256_color_impl_block {
    use super::*;

    impl TransformColor for Ansi256Color {
        fn as_grayscale(&self) -> Ansi256Color {
            let index = self.index as usize;
            let rgb = ANSI_COLOR_PALETTE[index];
            let rgb = RgbColor::from(rgb);
            let gray = color_utils::convert_grayscale((rgb.red, rgb.green, rgb.blue));
            ASTColor::Rgb(gray.0, gray.1, gray.2).as_ansi256()
        }

        fn as_rgb(&self) -> RgbColor {
            let index = self.index as usize;
            ANSI_COLOR_PALETTE[index].into()
        }

        fn as_ansi256(&self) -> Ansi256Color { *self }
    }
}

/*
 *   Copyright (c) 2023 R3BL LLC
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

use crate::{convert_rgb_into_ansi256, Ansi256Color, RgbColor, TransformColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Rgb(u8, u8, u8),
    Ansi256(u8),
}

impl TransformColor for Color {
    fn as_rgb(&self) -> RgbColor {
        match self {
            Color::Rgb(r, g, b) => RgbColor {
                red: *r,
                green: *g,
                blue: *b,
            },
            Color::Ansi256(index) => Ansi256Color { index: *index }.as_rgb(),
        }
    }

    fn as_ansi256(&self) -> Ansi256Color {
        match self {
            Color::Rgb(red, green, blue) => convert_rgb_into_ansi256(RgbColor {
                red: *red,
                green: *green,
                blue: *blue,
            }),
            Color::Ansi256(index) => Ansi256Color { index: *index },
        }
    }

    fn as_grayscale(&self) -> Ansi256Color {
        match self {
            Color::Rgb(red, green, blue) => convert_rgb_into_ansi256(RgbColor {
                red: *red,
                green: *green,
                blue: *blue,
            })
            .as_grayscale(),
            Color::Ansi256(index) => Ansi256Color { index: *index }.as_grayscale(),
        }
    }
}

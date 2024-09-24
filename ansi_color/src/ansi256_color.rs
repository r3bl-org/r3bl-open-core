/*
 *   Copyright (c) 2024 R3BL LLC
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

 use crate::{color_utils,
    constants::ANSI_COLOR_PALETTE,
    Color,
    RgbColor,
    TransformColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ansi256Color {
    pub index: u8,
}

impl TransformColor for Ansi256Color {
    fn as_grayscale(&self) -> Ansi256Color {
        let index = self.index as usize;
        let rgb = ANSI_COLOR_PALETTE[index];
        let rgb = RgbColor::from(rgb);
        let gray = color_utils::convert_grayscale((rgb.red, rgb.green, rgb.blue));
        Color::Rgb(gray.0, gray.1, gray.2).as_ansi256()
    }

    fn as_rgb(&self) -> RgbColor {
        let index = self.index as usize;
        ANSI_COLOR_PALETTE[index].into()
    }

    fn as_ansi256(&self) -> Ansi256Color { *self }
}

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

use r3bl_ansi_color::Color;

#[derive(Copy, Clone, Debug)]
pub struct StyleSheet {
    pub normal_style: Style,
    pub selected_style: Style,
    pub header_style: Style,
}

impl Default for StyleSheet {
    fn default() -> Self {
        let normal_style = Style::default();
        let selected_style = Style {
            fg_color: Color::Rgb(250, 250, 250),
            bg_color: Color::Rgb(39, 45, 239),
            ..Style::default()
        };
        let header_style = Style {
            fg_color: Color::Rgb(50, 50, 50),
            bg_color: Color::Rgb(150, 150, 150),
            bold: true,
            ..Style::default()
        };
        StyleSheet {
            normal_style,
            selected_style,
            header_style,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    pub underline: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    pub fg_color: Color,
    pub bg_color: Color,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            bold: false,
            italic: false,
            dim: false,
            underline: false,
            reverse: false,
            hidden: false,
            strikethrough: false,
            fg_color: Color::Rgb(200, 200, 1),
            bg_color: Color::Rgb(100, 60, 150),
        }
    }
}

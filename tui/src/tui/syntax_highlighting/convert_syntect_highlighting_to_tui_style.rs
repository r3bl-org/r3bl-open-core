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

use r3bl_rs_utils_core::{RgbValue, TuiColor, TuiStyle};

type SyntectStyle = syntect::highlighting::Style;
type SyntectFontStyle = syntect::highlighting::FontStyle;
type SyntectColor = syntect::highlighting::Color;

pub fn convert_style_from_syntect_to_tui(st_style: SyntectStyle) -> TuiStyle {
    TuiStyle {
        color_fg: Some(convert_color_from_syntect_to_tui(st_style.foreground)),
        color_bg: Some(convert_color_from_syntect_to_tui(st_style.background)),
        bold: st_style.font_style.contains(SyntectFontStyle::BOLD),
        italic: st_style.font_style.contains(SyntectFontStyle::ITALIC),
        underline: st_style.font_style.contains(SyntectFontStyle::UNDERLINE),
        ..Default::default()
    }
}

pub fn convert_color_from_syntect_to_tui(st_color: SyntectColor) -> TuiColor {
    TuiColor::Rgb(RgbValue::from_u8(st_color.r, st_color.g, st_color.b))
}

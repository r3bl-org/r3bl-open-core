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

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::{assert_eq2, UnicodeString};

    use super::*;
    use crate::TuiStyledTexts;

    #[test]
    fn syntect_conversion() {
        let st_color_1 = syntect::highlighting::Color {
            r: 255,
            g: 255,
            b: 255,
            a: 0,
        };

        let st_color_2 = syntect::highlighting::Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };

        let st_vec: Vec<(syntect::highlighting::Style, &str)> = vec![
            // item 1.
            (
                syntect::highlighting::Style {
                    foreground: st_color_1,
                    background: st_color_1,
                    font_style: syntect::highlighting::FontStyle::empty(),
                },
                "st_color_1",
            ),
            // item 2.
            (
                syntect::highlighting::Style {
                    foreground: st_color_2,
                    background: st_color_2,
                    font_style: syntect::highlighting::FontStyle::BOLD,
                },
                "st_color_2",
            ),
            // item 3.
            (
                syntect::highlighting::Style {
                    foreground: st_color_1,
                    background: st_color_2,
                    font_style: syntect::highlighting::FontStyle::UNDERLINE
                        | syntect::highlighting::FontStyle::BOLD
                        | syntect::highlighting::FontStyle::ITALIC,
                },
                "st_color_1 and 2",
            ),
        ];

        let styled_texts = TuiStyledTexts::from(st_vec);

        // Should have 3 items.
        assert_eq2!(styled_texts.len(), 3);

        // item 1.
        {
            assert_eq2!(
                styled_texts[0].get_text(),
                &UnicodeString::from("st_color_1")
            );
            assert_eq2!(
                styled_texts[0].get_style().color_fg.unwrap(),
                TuiColor::Rgb(RgbValue {
                    red: 255,
                    green: 255,
                    blue: 255
                })
            );
            assert_eq2!(
                styled_texts[0].get_style().color_bg.unwrap(),
                TuiColor::Rgb(RgbValue {
                    red: 255,
                    green: 255,
                    blue: 255
                })
            );
        }

        // item 2.
        {
            assert_eq2!(
                styled_texts[1].get_text(),
                &UnicodeString::from("st_color_2")
            );
            assert_eq2!(
                styled_texts[1].get_style().color_fg.unwrap(),
                TuiColor::Rgb(RgbValue {
                    red: 0,
                    green: 0,
                    blue: 0
                })
            );
            assert_eq2!(
                styled_texts[1].get_style().color_bg.unwrap(),
                TuiColor::Rgb(RgbValue {
                    red: 0,
                    green: 0,
                    blue: 0
                })
            );
            assert_eq2!(styled_texts[1].get_style().bold, true);
        }

        // item 3.
        {
            assert_eq2!(
                styled_texts[2].get_text(),
                &UnicodeString::from("st_color_1 and 2")
            );
            assert_eq2!(
                styled_texts[2].get_style().color_fg.unwrap(),
                TuiColor::Rgb(RgbValue {
                    red: 255,
                    green: 255,
                    blue: 255
                })
            );
            assert_eq2!(
                styled_texts[2].get_style().color_bg.unwrap(),
                TuiColor::Rgb(RgbValue {
                    red: 0,
                    green: 0,
                    blue: 0
                })
            );
            assert_eq2!(styled_texts[2].get_style().bold, true);
            assert_eq2!(styled_texts[2].get_style().underline, true);
        }
    }
}

/*
 *   Copyright (c) 2022-2024 R3BL LLC
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

//! This module contains code for converting between syntect styled texts and tui styled
//! texts.
//!
//! A [Vec] or [crate::List] of styled text represents a single line of text in an editor
//! component, which is the output of a syntax highlighter (that takes plain text and
//! returns the styled text).
//!
//! There is a major difference in doing this conversion which is:
//! - tui styled texts are styled unicode strings,
//! - while syntect styled texts are styled plain text strings.
//!
//! This requires the conversion code to perform the following steps:
//! 1. Convert the syntect [SyntectStyleStrSpanLine] into a [StyleUSSpanLine].
//! 2. Then convert [StyleUSSpanLine] into a [TuiStyledTexts].

use r3bl_core::{tui_styled_text,
                RgbValue,
                TuiColor,
                TuiStyle,
                TuiStyledText,
                TuiStyledTexts};
use syntect::parsing::SyntaxSet;

use super::{StyleUSSpan, StyleUSSpanLine};
use crate::aliases::US;

// Type aliases for syntect types.

type SyntectStyle = syntect::highlighting::Style;
type SyntectFontStyle = syntect::highlighting::FontStyle;
type SyntectColor = syntect::highlighting::Color;

/// Span are chunks of a text that have an associated style. There are usually multiple spans in a
/// line of text.
pub type SyntectStyleStrSpan<'a> = (SyntectStyle, &'a str);

/// A line of text is made up of multiple [SyntectStyleStrSpan]s.
pub type SyntectStyleStrSpanLine<'a> = Vec<SyntectStyleStrSpan<'a>>;

pub fn try_get_syntax_ref<'a>(
    syntax_set: &'a SyntaxSet,
    file_extension: &'a str,
) -> Option<&'a syntect::parsing::SyntaxReference> {
    syntax_set.find_syntax_by_extension(file_extension)
}

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

pub fn convert_highlighted_line_from_syntect_to_tui(
    syntect_highlighted_line: SyntectStyleStrSpanLine<'_>,
) -> StyleUSSpanLine {
    let mut it = convert(&syntect_highlighted_line);

    // Remove the background color from each style in the theme.
    it.iter_mut()
        .for_each(|StyleUSSpan { style, text: _ }| style.remove_bg_color());

    return it;

    fn convert(vec_styled_str: &SyntectStyleStrSpanLine<'_>) -> StyleUSSpanLine {
        let mut it: StyleUSSpanLine = Default::default();

        for (style, text) in vec_styled_str {
            let my_style = convert_style_from_syntect_to_tui(*style);
            let unicode_string = US::from(*text);
            it.push(StyleUSSpan::new(my_style, unicode_string));
        }

        it
    }
}

pub fn convert_span_line_from_syntect_to_tui_styled_texts(
    syntect_styles: &SyntectStyleStrSpanLine<'_>,
) -> TuiStyledTexts {
    let mut acc = TuiStyledTexts::default();
    for (syntect_style, text) in syntect_styles {
        let my_style = convert_style_from_syntect_to_tui(*syntect_style);
        acc += tui_styled_text!(@style: my_style, @text: text.to_string());
    }
    acc
}

#[cfg(test)]
mod tests_simple_md_highlight {
    use r3bl_core::{assert_eq2, color, ConvertToPlainText, TuiStyledTexts};
    use syntect::{easy::HighlightLines,
                  highlighting::Style,
                  parsing::SyntaxSet,
                  util::LinesWithEndings};

    use crate::{convert_span_line_from_syntect_to_tui_styled_texts,
                try_load_r3bl_theme};

    #[test]
    fn simple_md_highlight() {
        // Generate MD content.
        let md_content = {
            #[cfg(target_os = "windows")]
            {
                let mut it = include_str!("test_assets/valid-content.md").to_string();
                it = it.replace("\r\n", "\n");
                it
            }
            #[cfg(not(target_os = "windows"))]
            {
                include_str!("test_assets/valid-content.md").to_string()
            }
        };

        // Load these once at the start of your program.
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme = try_load_r3bl_theme().unwrap();

        // Prepare Markdown syntax highlighting.q
        let md_syntax = syntax_set.find_syntax_by_extension("md").unwrap();
        let mut highlight_lines = HighlightLines::new(md_syntax, &theme);

        let mut line_idx = 0;
        let mut vec_styled_texts = vec![];

        for line in /* LinesWithEndings enables use of newlines mode. */
            LinesWithEndings::from(md_content.as_str())
        {
            let vec_styled_str: Vec<(Style, &str)> =
                highlight_lines.highlight_line(line, &syntax_set).unwrap();

            // // To pretty print the output, use the following:
            // let escaped = as_24_bit_terminal_escaped(&vec_styled_str[..], false);
            // print!("{}", escaped);

            let styled_texts: TuiStyledTexts =
                convert_span_line_from_syntect_to_tui_styled_texts(&vec_styled_str);

            line_idx += 1;
            for (col_idx, styled_text) in styled_texts.inner.iter().enumerate() {
                println!("[L#:{line_idx} => C#:{col_idx}] {styled_text:#?}");
            }
            vec_styled_texts.push(styled_texts);
        }

        // 42 lines.
        assert_eq2!(vec_styled_texts.len(), 42);

        // Interrogate first line.
        {
            let line = &vec_styled_texts[0];
            assert_eq2!(line.len(), 4);
            assert_eq2!(line.to_plain_text_us(), "# My Heading\n".into());
            let col1 = &line[0];
            assert_eq2!(col1.get_style().bold, true);
            let col3 = &line[2];
            assert_eq2!(col3.get_style().color_fg.unwrap(), color!(46, 206, 43));
        }

        // Interrogate last line.
        {
            let line = &vec_styled_texts[41];
            assert_eq2!(line.len(), 1);
            assert_eq2!(line.to_plain_text_us(), "--- END ---\n".into());
            let col1 = &line[0];
            assert_eq2!(col1.get_style().color_fg.unwrap(), color!(193, 179, 208));
        }
    }
}

#[cfg(test)]
mod tests_convert_span_line_and_highlighted_line {
    use r3bl_core::{assert_eq2, RgbValue, TuiColor, TuiStyledTexts, UnicodeString};

    use crate::convert_span_line_from_syntect_to_tui_styled_texts;

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

        let vec_styled_str: Vec<(syntect::highlighting::Style, &str)> = vec![
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

        let styled_texts: TuiStyledTexts =
            convert_span_line_from_syntect_to_tui_styled_texts(&vec_styled_str);

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

#[cfg(test)]
mod tests_convert_style_and_color {
    use r3bl_core::{assert_eq2,
                    ch,
                    color,
                    console_log,
                    get_tui_style,
                    get_tui_styles,
                    throws,
                    tui_stylesheet,
                    ANSIBasicColor,
                    CommonResult,
                    RgbValue,
                    TuiColor,
                    TuiStyle,
                    TuiStylesheet};
    use r3bl_macro::tui_style;

    use crate::convert_style_from_syntect_to_tui;

    #[test]
    fn syntect_style_conversion() {
        let st_style: syntect::highlighting::Style = syntect::highlighting::Style {
            foreground: syntect::highlighting::Color::WHITE,
            background: syntect::highlighting::Color::BLACK,
            font_style: syntect::highlighting::FontStyle::BOLD
                | syntect::highlighting::FontStyle::ITALIC
                | syntect::highlighting::FontStyle::UNDERLINE,
        };
        let style = convert_style_from_syntect_to_tui(st_style);
        assert_eq2!(
            style.color_fg.unwrap(),
            TuiColor::Rgb(RgbValue {
                red: 255,
                green: 255,
                blue: 255
            })
        );
        assert_eq2!(
            style.color_bg.unwrap(),
            TuiColor::Rgb(RgbValue {
                red: 0,
                green: 0,
                blue: 0
            })
        );
        assert_eq2!(style.bold, true);
        assert_eq2!(style.underline, true);
    }

    #[test]
    fn test_cascade_style() {
        let style_bold_green_fg = tui_style! {
          id: 1 // "bold_green_fg"
          attrib: [bold]
          color_fg: TuiColor::Basic(ANSIBasicColor::Green)
        };

        let style_dim = tui_style! {
          id: 2 // "dim"
          attrib: [dim]
        };

        let style_yellow_bg = tui_style! {
          id: 3 // "yellow_bg"
          color_bg: TuiColor::Basic(ANSIBasicColor::Yellow)
        };

        let style_padding = tui_style! {
          id: 4 // "padding"
          padding: 2
        };

        let style_red_fg = tui_style! {
          id: 5 // "red_fg"
          color_fg: TuiColor::Basic(ANSIBasicColor::Red)
        };

        let style_padding_another = tui_style! {
          id: 6 // "padding"
          padding: 1
        };

        let my_style = style_bold_green_fg
            + style_dim
            + style_yellow_bg
            + style_padding
            + style_red_fg
            + style_padding_another;

        console_log!(my_style);

        assert_eq2!(my_style.padding.unwrap(), ch!(3));
        assert_eq2!(
            my_style.color_bg.unwrap(),
            TuiColor::Basic(ANSIBasicColor::Yellow)
        );
        assert_eq2!(
            my_style.color_fg.unwrap(),
            TuiColor::Basic(ANSIBasicColor::Red)
        );
        assert!(my_style.bold);
        assert!(my_style.dim);
        assert!(my_style.computed);
        assert!(!my_style.underline);
    }

    #[test]
    fn test_stylesheet() {
        let mut stylesheet = TuiStylesheet::new();

        let style1 = make_a_style(1);
        let result = stylesheet.add_style(style1);
        result.unwrap();
        assert_eq2!(stylesheet.styles.len(), 1);

        let style2 = make_a_style(2);
        let result = stylesheet.add_style(style2);
        result.unwrap();
        assert_eq2!(stylesheet.styles.len(), 2);

        // Test find_style_by_id.
        {
            // No macro.
            assert_eq2!(stylesheet.find_style_by_id(1).unwrap().id, 1);
            assert_eq2!(stylesheet.find_style_by_id(2).unwrap().id, 2);
            assert!(stylesheet.find_style_by_id(3).is_none());
            // Macro.
            assert_eq2!(get_tui_style!(@from: stylesheet, 1).unwrap().id, 1);
            assert_eq2!(get_tui_style!(@from: stylesheet, 2).unwrap().id, 2);
            assert!(get_tui_style!(@from: stylesheet, 3).is_none());
        }

        // Test find_styles_by_ids.
        {
            // Contains.
            assertions_for_find_styles_by_ids(&stylesheet.find_styles_by_ids(vec![1, 2]));
            assertions_for_find_styles_by_ids(&get_tui_styles!(
                @from: &stylesheet,
                [1, 2]
            ));
            fn assertions_for_find_styles_by_ids(result: &Option<Vec<TuiStyle>>) {
                assert_eq2!(result.as_ref().unwrap().len(), 2);
                assert_eq2!(result.as_ref().unwrap()[0].id, 1);
                assert_eq2!(result.as_ref().unwrap()[1].id, 2);
            }
            // Does not contain.
            assert_eq2!(stylesheet.find_styles_by_ids(vec![3, 4]), None);
            assert_eq2!(get_tui_styles!(@from: stylesheet, [3, 4]), None);
        }
    }

    #[test]
    fn test_stylesheet_builder() -> CommonResult<()> {
        throws!({
            let id_2 = 2;
            let style1 = make_a_style(1);
            let mut stylesheet = tui_stylesheet! {
              style1,
              tui_style! {
                    id: id_2 /* using a variable instead of string literal */
                    padding: 1
                    color_bg: TuiColor::Rgb (RgbValue{ red: 55, green: 55, blue: 248 })
              },
              make_a_style(3),
              vec![
                tui_style! {
                  id: 4
                  padding: 1
                  color_bg: TuiColor::Rgb (RgbValue{ red: 55, green: 55, blue: 248 })
                },
                tui_style! {
                  id: 5
                  padding: 1
                  color_bg: TuiColor::Rgb (RgbValue{ red: 85, green: 85, blue: 255 })
                },
              ],
              make_a_style(6)
            };

            assert_eq2!(stylesheet.styles.len(), 6);
            assert_eq2!(stylesheet.find_style_by_id(1).unwrap().id, 1);
            assert_eq2!(stylesheet.find_style_by_id(2).unwrap().id, 2);
            assert_eq2!(stylesheet.find_style_by_id(3).unwrap().id, 3);
            assert_eq2!(stylesheet.find_style_by_id(4).unwrap().id, 4);
            assert_eq2!(stylesheet.find_style_by_id(5).unwrap().id, 5);
            assert_eq2!(stylesheet.find_style_by_id(6).unwrap().id, 6);
            assert!(stylesheet.find_style_by_id(7).is_none());

            let result = stylesheet.find_styles_by_ids(vec![1, 2]);
            assert_eq2!(result.as_ref().unwrap().len(), 2);
            assert_eq2!(result.as_ref().unwrap()[0].id, 1);
            assert_eq2!(result.as_ref().unwrap()[1].id, 2);
            assert_eq2!(stylesheet.find_styles_by_ids(vec![13, 41]), None);
            let style7 = make_a_style(7);
            let result = stylesheet.add_style(style7);
            result.unwrap();
            assert_eq2!(stylesheet.styles.len(), 7);
            assert_eq2!(stylesheet.find_style_by_id(7).unwrap().id, 7);
        });
    }

    /// Helper function.
    fn make_a_style(id: u8) -> TuiStyle {
        TuiStyle {
            id,
            dim: true,
            bold: true,
            color_fg: color!(0, 0, 0).into(),
            color_bg: color!(0, 0, 0).into(),
            ..TuiStyle::default()
        }
    }
}

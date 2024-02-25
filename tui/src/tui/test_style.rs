/*
 *   Copyright (c) 2022 R3BL LLC
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

//! The reason the tests are in this separate file and not inside of `style.rs` is because the
//! [r3bl_rs_utils_macro::style!] macro is in a different crate than the [r3bl_rs_utils_core::Style]
//! struct.
//!
//! In order to use this macro, the test has to be in a different crate (aka `r3bl_tui` craete) than
//! both:
//!
//! 1. the [r3bl_rs_utils_core::Style] struct (`r3bl_rs_utils_core` crate)
//! 2. the [r3bl_rs_utils_macro::style!] macro (`r3bl_rs_utils_macro` crate).

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::*;
    use r3bl_rs_utils_macro::tui_style;

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

        debug!(my_style);

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

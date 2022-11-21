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

#[cfg(test)]
mod tests {
  use r3bl_rs_utils_core::*;
  use r3bl_rs_utils_macro::style;

  use crate::*;

  /// Make sure that the code to clip styled text to a range [ start_col .. end_col ] works. The
  /// list of styled unicode string represents a single line of text in an editor component.
  #[cfg(test)]
  mod clip_styled_texts {
    use super::*;

    mod helpers {
      use super::*;

      pub fn get_s1() -> Style {
        style! {
          id: "s1"
          color_bg: TuiColor::Rgb { r: 1, g: 1, b: 1 }
        }
      }

      pub fn get_s2() -> Style {
        style! {
          id: "s2"
          color_bg: TuiColor::Rgb { r: 2, g: 2, b: 2 }
        }
      }

      /// ```ignore
      /// <span style="s1">first</span>
      /// <span style="s1"> </span>
      /// <span style="s2">second</span>
      /// ```
      pub fn get_list() -> List<(Style, UnicodeString)> {
        let mut it = List::default();
        it.push((get_s1(), UnicodeString::from("first")));
        it.push((get_s1(), UnicodeString::from(" ")));
        it.push((get_s2(), UnicodeString::from("second")));
        it
      }
    }

    #[test]
    fn list_1_range_2_5() {
      use helpers::*;

      assert_eq!(get_list().len(), 3);

      let scroll_offset_col_index = ch!(2);
      let max_display_col_count = ch!(5);
      let expected_clipped_string = "rst s";

      // BEFORE:
      //    ┌→s1
      //    │    ┌→s2
      //    │    │┌→s3
      //    ▒▒▒▒▒█▒▒▒▒▒▒
      // R ┌────────────┐
      // 0 │first second│
      //   └────────────┘
      //   C012345678901
      //
      // AFTER: Cut [ 2 .. 5 ].
      //      ┌→s1
      //      │  ┌→s2
      //      │  │┌→s3
      // R   ┌─────┐
      // 0 fi│rst s│econd
      //     └─────┘
      //     C01234 5678901

      // Equivalent no highlight version.
      {
        let line = StyledTexts::from(get_list()).get_plain_text().string;
        let line = UnicodeString::from(line);
        let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
        let truncated_line = UnicodeString::from(truncated_line);
        let truncated_line = truncated_line.truncate_end_to_fit_display_cols(max_display_col_count);
        assert_eq2!(truncated_line, expected_clipped_string);
      }

      // clip2 version.
      {
        let clipped = get_list().clip(scroll_offset_col_index, max_display_col_count);
        // println!("{}", clipped.pretty_print());
        assert_eq2!(clipped.len(), 3);
        let lhs = clipped.get_plain_text().string;
        assert_eq2!(lhs, expected_clipped_string);
      }
    }

    #[test]
    fn list_1_range_0_3() {
      use helpers::*;

      assert_eq!(get_list().len(), 3);

      let scroll_offset_col_index = ch!(0);
      let max_display_col_count = ch!(3);
      let expected_clipped_string = "fir";

      // BEFORE:
      //    ┌→s1
      //    │    ┌→s2
      //    │    │┌→s3
      //    ▒▒▒▒▒█▒▒▒▒▒▒
      // R ┌────────────┐
      // 0 │first second│
      //   └────────────┘
      //   C012345678901
      //
      // AFTER: Cut [ 0 .. 3 ].
      //    ┌→s1
      //    │     ┌→s2
      //    │     │┌→s3
      // R ┌───┐
      // 0 │fir│st second
      //   └───┘
      //   C012 345678901

      // Equivalent no highlight version.
      {
        let line = StyledTexts::from(helpers::get_list()).get_plain_text().string;
        let line = UnicodeString::from(line);
        let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
        let truncated_line = UnicodeString::from(truncated_line);
        let truncated_line = truncated_line.truncate_end_to_fit_display_cols(max_display_col_count);
        assert_eq2!(truncated_line, expected_clipped_string);
      }

      // clip2 version.
      {
        let clipped = helpers::get_list().clip(scroll_offset_col_index, max_display_col_count);
        // println!("{}", clipped.pretty_print());
        assert_eq2!(clipped.len(), 1);
        let left = clipped.get_plain_text().string;
        let right = expected_clipped_string;
        assert_eq2!(left, right);
      }
    }

    #[test]
    fn list_1_range_0_5() {
      use helpers::*;

      assert_eq!(get_list().len(), 3);

      let scroll_offset_col_index = ch!(0);
      let max_display_col_count = ch!(5);
      let expected_clipped_string = "first";

      // BEFORE:
      //    ┌→s1
      //    │    ┌→s2
      //    │    │┌→s3
      //    ▒▒▒▒▒█▒▒▒▒▒▒
      // R ┌────────────┐
      // 0 │first second│
      //   └────────────┘
      //   C012345678901
      //
      // AFTER: Cut [ 0 .. 5 ].
      //    ┌→s1
      //    │     ┌→s2
      //    │     │┌→s3
      // R ┌─────┐
      // 0 │first│ second
      //   └─────┘
      //   C01234 5678901

      // Equivalent no highlight version.
      {
        let line = StyledTexts::from(helpers::get_list()).get_plain_text().string;
        let line = UnicodeString::from(line);
        let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
        let truncated_line = UnicodeString::from(truncated_line);
        let truncated_line = truncated_line.truncate_end_to_fit_display_cols(max_display_col_count);
        assert_eq2!(truncated_line, expected_clipped_string);
      }

      // clip2 version.
      {
        let clipped = helpers::get_list().clip(scroll_offset_col_index, max_display_col_count);
        // println!("{}", clipped.pretty_print());
        assert_eq2!(clipped.len(), 1);
        let lhs = clipped.get_plain_text().string;
        let rhs = expected_clipped_string;
        assert_eq2!(lhs, rhs);
      }
    }

    #[test]
    fn list_1_range_2_8() {
      use helpers::*;

      assert_eq!(get_list().len(), 3);

      let scroll_offset_col_index = ch!(2);
      let max_display_col_count = ch!(8);
      let expected_clipped_string = "rst seco";

      // BEFORE:
      //    ┌→s1
      //    │    ┌→s2
      //    │    │┌→s3
      //    ▒▒▒▒▒█▒▒▒▒▒▒
      // R ┌────────────┐
      // 0 │first second│
      //   └────────────┘
      //   C012345678901
      //
      // AFTER: Cut [ 2 .. 8 ].
      //      ┌→s1
      //      │  ┌→s2
      //      │  │┌→s3
      // R   ┌────────┐
      // 0 fi│rst seco│nd
      //     └────────┘
      //     C01234567 8901

      // Expected no highlight version.
      {
        let line = StyledTexts::from(helpers::get_list()).get_plain_text().string;
        let line = UnicodeString::from(line);
        let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
        let truncated_line = UnicodeString::from(truncated_line);
        let truncated_line = truncated_line.truncate_end_to_fit_display_cols(max_display_col_count);
        assert_eq2!(truncated_line, expected_clipped_string);
      }

      // clip2 version.
      {
        let clipped = helpers::get_list().clip(scroll_offset_col_index, max_display_col_count);
        // println!("{}", clipped.pretty_print());
        assert_eq2!(clipped.len(), 3);
        let left = clipped.get_plain_text().string;
        let right = expected_clipped_string;
        assert_eq2!(left, right);
      }
    }

    #[test]
    fn list_2() {
      use helpers::*;

      fn get_list() -> List<(Style, UnicodeString)> {
        let mut list = List::default();
        list.push((
          get_s1(),
          UnicodeString::from("01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234"),
        ));
        list
      }

      let scroll_offset_col_index = ch!(1);
      let max_display_col_count = ch!(77);
      let expected_clipped_string = "1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234";

      // BEFORE:
      // ┌→0                                                                              │
      // │                                                                           ┌→77 │
      // .............................................................................    │ viewport
      // 01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234
      //
      // AFTER:
      // ┌→0                                                                              │
      // │                                                                           ┌→77 │
      // .............................................................................    │ viewport
      // 1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234

      // Expected no highlight version.
      {
        let line = StyledTexts::from(get_list()).get_plain_text().string;
        let line = UnicodeString::from(line);
        let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
        let truncated_line = UnicodeString::from(truncated_line);
        let truncated_line = truncated_line.truncate_end_to_fit_display_cols(max_display_col_count);
        assert_eq2!(truncated_line, expected_clipped_string);
      }

      // clip2 version.
      {
        let clipped = get_list().clip(scroll_offset_col_index, max_display_col_count);
        // println!("{}", clipped.pretty_print());
        assert_eq2!(clipped.len(), 1);
        let lhs = clipped.get_plain_text().string;
        let rhs = expected_clipped_string;
        assert_eq2!(lhs, rhs);
      }
    }

    #[test]
    fn list_3() {
      use helpers::*;

      fn get_list() -> List<(Style, UnicodeString)> {
        let mut list = List::default();
        list.push((
          get_s1(),
          UnicodeString::from("01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 0123456"),
        ));
        list
      }

      let scroll_offset_col_index = ch!(1);
      let max_display_col_count = ch!(77);
      let expected_clipped_string = "1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 012345";

      // BEFORE:
      // ┌→0                                                                              │
      // │                                                                           ┌→77 │
      // .............................................................................    │ viewport
      // 01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 0123456
      //
      // AFTER:
      // ┌→0                                                                              │
      // │                                                                           ┌→77 │
      // .............................................................................    │ viewport
      // 1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 012345

      // Expected no highlight version.
      {
        let line = StyledTexts::from(get_list()).get_plain_text().string;
        let line = UnicodeString::from(line);
        let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
        let truncated_line = UnicodeString::from(truncated_line);
        let truncated_line = truncated_line.truncate_end_to_fit_display_cols(max_display_col_count);
        assert_eq2!(truncated_line, expected_clipped_string);
      }

      // clip2 version.
      {
        let clipped = get_list().clip(scroll_offset_col_index, max_display_col_count);
        // println!("{}", clipped.pretty_print());
        assert_eq2!(clipped.len(), 1);
        let left = clipped.get_plain_text().string;
        let right = expected_clipped_string;
        assert_eq2!(left, right);
      }
    }
  }

  #[test]
  fn syntect_conversion() {
    let st_color_1 = syntect::highlighting::Color {
      r: 255,
      g: 255,
      b: 255,
      a: 0,
    };

    let st_color_2 = syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 0 };

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

    let styled_texts = StyledTexts::from(st_vec);

    // Should have 3 items.
    assert_eq2!(styled_texts.len(), 3);

    // item 1.
    {
      assert_eq2!(styled_texts[0].get_plain_text(), &UnicodeString::from("st_color_1"));
      assert_eq2!(
        styled_texts[0].get_style().color_fg.unwrap(),
        TuiColor::Rgb { r: 255, g: 255, b: 255 }
      );
      assert_eq2!(
        styled_texts[0].get_style().color_bg.unwrap(),
        TuiColor::Rgb { r: 255, g: 255, b: 255 }
      );
    }

    // item 2.
    {
      assert_eq2!(styled_texts[1].get_plain_text(), &UnicodeString::from("st_color_2"));
      assert_eq2!(
        styled_texts[1].get_style().color_fg.unwrap(),
        TuiColor::Rgb { r: 0, g: 0, b: 0 }
      );
      assert_eq2!(
        styled_texts[1].get_style().color_bg.unwrap(),
        TuiColor::Rgb { r: 0, g: 0, b: 0 }
      );
      assert_eq2!(styled_texts[1].get_style().bold, true);
    }

    // item 3.
    {
      assert_eq2!(
        styled_texts[2].get_plain_text(),
        &UnicodeString::from("st_color_1 and 2")
      );
      assert_eq2!(
        styled_texts[2].get_style().color_fg.unwrap(),
        TuiColor::Rgb { r: 255, g: 255, b: 255 }
      );
      assert_eq2!(
        styled_texts[2].get_style().color_bg.unwrap(),
        TuiColor::Rgb { r: 0, g: 0, b: 0 }
      );
      assert_eq2!(styled_texts[2].get_style().bold, true);
      assert_eq2!(styled_texts[2].get_style().underline, true);
    }
  }

  #[test]
  fn test_create_styled_text_with_dsl() -> CommonResult<()> {
    throws!({
      let st_vec = helpers::create_styled_text()?;
      assert_eq2!(st_vec.is_empty(), false);
      assert_eq2!(st_vec.len(), 2);
    })
  }

  #[test]
  fn test_styled_text_renders_correctly() -> CommonResult<()> {
    throws!({
      let st_vec = helpers::create_styled_text()?;
      let mut render_ops = render_ops!();
      st_vec.render_into(&mut render_ops);

      let mut pipeline = render_pipeline!();
      pipeline.push(ZOrder::Normal, render_ops);

      debug!(pipeline);
      assert_eq2!(pipeline.len(), 1);

      let set: &Vec<RenderOps> = pipeline.get(&ZOrder::Normal).unwrap();

      // "Hello" and "World" together.
      assert_eq2!(set.len(), 1);

      // 3 RenderOp each for "Hello" & "World".
      assert_eq2!(pipeline.get_all_render_op_in(ZOrder::Normal).unwrap().len(), 6);
    })
  }

  mod helpers {
    use super::*;

    pub fn create_styled_text() -> CommonResult<StyledTexts> {
      throws_with_return!({
        let stylesheet = create_stylesheet()?;
        let maybe_style1 = stylesheet.find_style_by_id("style1");
        let maybe_style2 = stylesheet.find_style_by_id("style2");

        styled_texts! {
          styled_text! {
            "Hello".to_string(),
            maybe_style1.unwrap()
          },
          styled_text! {
            "World".to_string(),
            maybe_style2.unwrap()
          }
        }
      })
    }

    pub fn create_stylesheet() -> CommonResult<Stylesheet> {
      throws_with_return!({
        stylesheet! {
          style! {
            id: "style1"
            padding: 1
            color_bg: TuiColor::Rgb { r: 55, g: 55, b: 100 }
          },
          style! {
            id: "style2"
            padding: 1
            color_bg: TuiColor::Rgb { r: 55, g: 55, b: 248 }
          }
        }
      })
    }
  }
}

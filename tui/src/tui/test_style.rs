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

//! <https://docs.rs/bitmask/latest/bitmask/macro.bitmask.html>

#[cfg(test)]
mod tests {
  use r3bl_rs_utils_core::*;
  use r3bl_rs_utils_macro::style;

  #[test]
  fn syntect_style_conversion() {
    let st_style: syntect::highlighting::Style = syntect::highlighting::Style {
      foreground: syntect::highlighting::Color::WHITE,
      background: syntect::highlighting::Color::BLACK,
      font_style: syntect::highlighting::FontStyle::BOLD
        | syntect::highlighting::FontStyle::ITALIC
        | syntect::highlighting::FontStyle::UNDERLINE,
    };
    let style = Style::from(st_style);
    assert_eq2!(style.color_fg.unwrap(), TuiColor::Rgb { r: 255, g: 255, b: 255 });
    assert_eq2!(style.color_bg.unwrap(), TuiColor::Rgb { r: 0, g: 0, b: 0 });
    assert_eq2!(style.bold, true);
    assert_eq2!(style.underline, true);
  }

  #[test]
  fn test_bitflags() {
    with_mut! {
      StyleFlag::empty(),
      as mask1,
      run {
        mask1.insert(StyleFlag::UNDERLINE_SET);
        mask1.insert(StyleFlag::DIM_SET);
        assert!(mask1.contains(StyleFlag::UNDERLINE_SET));
        assert!(mask1.contains(StyleFlag::DIM_SET));
        assert!(!mask1.contains(StyleFlag::COLOR_FG_SET));
        assert!(!mask1.contains(StyleFlag::COLOR_BG_SET));
        assert!(!mask1.contains(StyleFlag::BOLD_SET));
        assert!(!mask1.contains(StyleFlag::PADDING_SET));
      }
    };

    with_mut! {
      StyleFlag::BOLD_SET | StyleFlag::DIM_SET,
      as mask2,
      run {
        assert!(mask2.contains(StyleFlag::BOLD_SET));
        assert!(mask2.contains(StyleFlag::DIM_SET));
        assert!(!mask2.contains(StyleFlag::UNDERLINE_SET));
        assert!(!mask2.contains(StyleFlag::COLOR_FG_SET));
        assert!(!mask2.contains(StyleFlag::COLOR_BG_SET));
        assert!(!mask2.contains(StyleFlag::PADDING_SET));
      }
    }

    assert!(!mask1.contains(mask2));
  }

  #[test]
  fn test_all_fields_in_style() {
    let mut style = Style {
      id: "foo".to_string(),
      bold: true,
      dim: true,
      underline: true,
      reverse: true,
      hidden: true,
      strikethrough: true,
      color_fg: color!(@red).into(),
      color_bg: color!(0, 0, 0).into(),
      padding: Some(ch!(10)),
      ..Style::default()
    };

    assert!(!style.computed);
    assert_eq2!(style.id, "foo");
    assert!(style.bold);
    assert!(style.dim);
    assert!(style.underline);
    assert!(style.reverse);
    assert!(style.hidden);
    assert!(style.strikethrough);
    assert_eq2!(style.color_fg, color!(@red).into());
    assert_eq2!(style.color_bg, color!(0, 0, 0).into());
    assert_eq2!(style.padding, Some(ch!(10)));

    let mask = style.get_bitflags();
    assert!(!mask.contains(StyleFlag::COMPUTED_SET));
    assert!(mask.contains(StyleFlag::BOLD_SET));
    assert!(mask.contains(StyleFlag::DIM_SET));
    assert!(mask.contains(StyleFlag::UNDERLINE_SET));
    assert!(mask.contains(StyleFlag::REVERSE_SET));
    assert!(mask.contains(StyleFlag::HIDDEN_SET));
    assert!(mask.contains(StyleFlag::STRIKETHROUGH_SET));
    assert!(mask.contains(StyleFlag::COLOR_FG_SET));
    assert!(mask.contains(StyleFlag::COLOR_BG_SET));
    assert!(mask.contains(StyleFlag::PADDING_SET));
  }

  #[test]
  fn test_style() {
    let mut style = make_a_style("test_style");
    let bitflags = style.get_bitflags();
    assert!(bitflags.contains(StyleFlag::BOLD_SET));
    assert!(bitflags.contains(StyleFlag::DIM_SET));
    assert!(!bitflags.contains(StyleFlag::UNDERLINE_SET));
  }

  #[test]
  fn test_cascade_style() {
    let style_bold_green_fg = style! {
      id: "bold_green_fg"
      attrib: [bold]
      color_fg: TuiColor::Green
    };

    let style_dim = style! {
      id: "dim"
      attrib: [dim]
    };

    let style_yellow_bg = style! {
      id: "yellow_bg"
      color_bg: TuiColor::Yellow
    };

    let style_padding = style! {
      id: "padding"
      padding: 2
    };

    let style_red_fg = style! {
      id: "red_fg"
      color_fg: TuiColor::Red
    };

    let style_padding_another = style! {
      id: "padding"
      padding: 1
    };

    let mut my_style =
      style_bold_green_fg + style_dim + style_yellow_bg + style_padding + style_red_fg + style_padding_another;

    debug!(my_style);

    assert_eq2!(
      my_style.get_bitflags().contains(
        StyleFlag::COLOR_FG_SET
          | StyleFlag::COLOR_BG_SET
          | StyleFlag::BOLD_SET
          | StyleFlag::DIM_SET
          | StyleFlag::PADDING_SET
          | StyleFlag::COMPUTED_SET
      ),
      true
    );

    assert_eq2!(my_style.padding.unwrap(), ch!(3));
    assert_eq2!(my_style.color_bg.unwrap(), TuiColor::Yellow);
    assert_eq2!(my_style.color_fg.unwrap(), TuiColor::Red);
    assert!(my_style.bold);
    assert!(my_style.dim);
    assert!(my_style.computed);
    assert!(!my_style.underline);
  }

  #[test]
  fn test_stylesheet() {
    let mut stylesheet = Stylesheet::new();

    let style1 = make_a_style("style1");
    let result = stylesheet.add_style(style1);
    result.unwrap();
    assert_eq2!(stylesheet.styles.len(), 1);

    let style2 = make_a_style("style2");
    let result = stylesheet.add_style(style2);
    result.unwrap();
    assert_eq2!(stylesheet.styles.len(), 2);

    // Test find_style_by_id.
    {
      // No macro.
      assert_eq2!(stylesheet.find_style_by_id("style1").unwrap().id, "style1");
      assert_eq2!(stylesheet.find_style_by_id("style2").unwrap().id, "style2");
      assert!(stylesheet.find_style_by_id("style3").is_none());
      // Macro.
      assert_eq2!(get_style!(from: stylesheet, "style1").unwrap().id, "style1");
      assert_eq2!(get_style!(from: stylesheet, "style2").unwrap().id, "style2");
      assert!(get_style!(from: stylesheet, "style3").is_none());
    }

    // Test find_styles_by_ids.
    {
      // Contains.
      assertions_for_find_styles_by_ids(&stylesheet.find_styles_by_ids(vec!["style1", "style2"]));
      assertions_for_find_styles_by_ids(&get_styles!(from: &stylesheet, ["style1", "style2"]));
      fn assertions_for_find_styles_by_ids(result: &Option<Vec<Style>>) {
        assert_eq2!(result.as_ref().unwrap().len(), 2);
        assert_eq2!(result.as_ref().unwrap()[0].id, "style1");
        assert_eq2!(result.as_ref().unwrap()[1].id, "style2");
      }
      // Does not contain.
      assert_eq2!(stylesheet.find_styles_by_ids(vec!["style3", "style4"]), None);
      assert_eq2!(get_styles!(from: stylesheet, ["style3", "style4"]), None);
    }
  }

  #[test]
  fn test_stylesheet_builder() -> CommonResult<()> {
    let id_2 = "style2";
    let style1 = make_a_style("style1");
    let mut stylesheet = stylesheet! {
      style1,
      style! {
            id: id_2 /* using a variable instead of string literal */
            padding: 1
            color_bg: TuiColor::Rgb { r: 55, g: 55, b: 248 }
      },
      make_a_style("style3"),
      vec![
        style! {
          id: "style4"
          padding: 1
          color_bg: TuiColor::Rgb { r: 55, g: 55, b: 248 }
        },
        style! {
          id: "style5"
          padding: 1
          color_bg: TuiColor::Rgb { r: 85, g: 85, b: 255 }
        },
      ],
      make_a_style("style6")
    };

    assert_eq2!(stylesheet.styles.len(), 6);
    assert_eq2!(stylesheet.find_style_by_id("style1").unwrap().id, "style1");
    assert_eq2!(stylesheet.find_style_by_id("style2").unwrap().id, "style2");
    assert_eq2!(stylesheet.find_style_by_id("style3").unwrap().id, "style3");
    assert_eq2!(stylesheet.find_style_by_id("style4").unwrap().id, "style4");
    assert_eq2!(stylesheet.find_style_by_id("style5").unwrap().id, "style5");
    assert_eq2!(stylesheet.find_style_by_id("style6").unwrap().id, "style6");
    assert!(stylesheet.find_style_by_id("style7").is_none());

    let result = stylesheet.find_styles_by_ids(vec!["style1", "style2"]);
    assert_eq2!(result.as_ref().unwrap().len(), 2);
    assert_eq2!(result.as_ref().unwrap()[0].id, "style1");
    assert_eq2!(result.as_ref().unwrap()[1].id, "style2");
    assert_eq2!(stylesheet.find_styles_by_ids(vec!["style13", "style41"]), None);
    let style7 = make_a_style("style7");
    let result = stylesheet.add_style(style7);
    result.unwrap();
    assert_eq2!(stylesheet.styles.len(), 7);
    assert_eq2!(stylesheet.find_style_by_id("style7").unwrap().id, "style7");
    Ok(())
  }

  /// Helper function.
  fn make_a_style(id: &str) -> Style {
    Style {
      id: id.to_string(),
      dim: true,
      bold: true,
      color_fg: color!(0, 0, 0).into(),
      color_bg: color!(0, 0, 0).into(),
      ..Style::default()
    }
  }
}

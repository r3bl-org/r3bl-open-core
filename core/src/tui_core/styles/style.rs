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

use core::fmt::Debug;
use std::{fmt::{Display, Formatter},
          ops::{Add, AddAssign}};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::*;

// ┏━━━━━━━┓
// ┃ Style ┃
// ┛       ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Use [crate::style] proc macro to generate code for this struct. Here's an example.
///
/// ```ignore
/// use r3bl_rs_utils_core::Stylesheet;
/// use r3bl_rs_utils_macro::style;
///
/// // Turquoise:  TuiColor::Rgb { r: 51, g: 255, b: 255 }
/// // Pink:       TuiColor::Rgb { r: 252, g: 157, b: 248 }
/// // Blue:       TuiColor::Rgb { r: 55, g: 55, b: 248 }
/// // Faded blue: TuiColor::Rgb { r: 85, g: 85, b: 255 }
/// let mut stylesheet = Stylesheet::new();
///
/// stylesheet.add_styles(vec![
///   style! {
///     id: style1
///     attrib: [dim, bold]
///     padding: 1
///     color_bg: TuiColor::Rgb { r: 55, g: 55, b: 248 }
///   },
///   style! {
///     id: style2
///     padding: 1
///     color_bg: TuiColor::Rgb { r: 85, g: 85, b: 255 }
///   }
/// ]);
/// ```
///
/// Here are the [crossterm docs on
/// attributes](https://docs.rs/crossterm/0.25.0/crossterm/style/enum.Attribute.html)
#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Style {
  pub id: String,
  pub bold: bool,
  pub dim: bool,
  pub underline: bool,
  pub reverse: bool,
  pub hidden: bool,
  pub strikethrough: bool,
  pub computed: bool,
  pub color_fg: Option<TuiColor>,
  pub color_bg: Option<TuiColor>,
  /// The semantics of this are the same as CSS. The padding is space that is taken up inside a
  /// `FlexBox`. This does not affect the size or position of a `FlexBox`, it only applies to the
  /// contents inside of that `FlexBox`.
  ///
  /// [`FlexBox` docs](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/tui/layout/flex_box/struct.FlexBox.html).
  pub padding: Option<ChUnit>,
  pub cached_bitflags: Option<StyleFlag>,
  pub lolcat: bool,
}

// ┏━━━━━━━━━━┓
// ┃ Addition ┃
// ┛          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod addition {
  use super::*;

  impl Add for Style {
    type Output = Self;
    fn add(self, other: Self) -> Self { add_styles(self, other) }
  }

  pub fn add_styles(lhs: Style, rhs: Style) -> Style {
    // Computed style has no id.
    let mut new_style: Style = Style {
      id: "".to_string(),
      computed: true,
      ..Style::default()
    };

    apply_style_flag(&mut new_style, &lhs);
    apply_style_flag(&mut new_style, &rhs);

    // other (if set) overrides new_style.
    fn apply_style_flag(new_style: &mut Style, other: &Style) {
      let other_mask = other.clone().get_bitflags();
      if other_mask.contains(StyleFlag::COLOR_FG_SET) {
        new_style.color_fg = other.color_fg;
      }
      if other_mask.contains(StyleFlag::COLOR_BG_SET) {
        new_style.color_bg = other.color_bg;
      }
      if other_mask.contains(StyleFlag::BOLD_SET) {
        new_style.bold = other.bold;
      }
      if other_mask.contains(StyleFlag::DIM_SET) {
        new_style.dim = other.dim;
      }
      if other_mask.contains(StyleFlag::UNDERLINE_SET) {
        new_style.underline = other.underline;
      }
      if other_mask.contains(StyleFlag::PADDING_SET) {
        new_style.padding = other.padding;
      }
      if other_mask.contains(StyleFlag::REVERSE_SET) {
        new_style.reverse = other.reverse;
      }
      if other_mask.contains(StyleFlag::HIDDEN_SET) {
        new_style.hidden = other.hidden;
      }
      if other_mask.contains(StyleFlag::STRIKETHROUGH_SET) {
        new_style.strikethrough = other.strikethrough;
      }
    }

    // Aggregate paddings.
    let aggregate_padding: ChUnit = lhs.padding.unwrap_or_else(|| ch!(0)) + rhs.padding.unwrap_or_else(|| ch!(0));
    if *aggregate_padding > 0 {
      new_style.padding = aggregate_padding.into();
    } else {
      new_style.padding = None;
    }

    // Recalculate the bitflags.
    new_style.reset_bitflags();
    new_style.get_bitflags();

    new_style
  }

  impl AddAssign<&Style> for Style {
    fn add_assign(&mut self, rhs: &Style) {
      let sum = add_styles(self.clone(), rhs.clone());
      *self = sum;
    }
  }
}

// ┏━━━━━━━━━━━━━━━┓
// ┃ Style helpers ┃
// ┛               ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod helpers {
  use super::*;

  impl Display for Style {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      let msg = format!("{self:?}");
      f.write_str(&msg)
    }
  }

  impl Debug for Style {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
      let mut msg_vec: Vec<String> = vec![];

      if self.computed {
        msg_vec.push("computed".to_string())
      } else if self.id.is_empty() {
        msg_vec.push("id: N/A".to_string())
      } else {
        msg_vec.push(self.id.to_string());
      }

      if self.bold {
        msg_vec.push("bold".to_string())
      }

      if self.dim {
        msg_vec.push("dim".to_string())
      }

      if self.underline {
        msg_vec.push("underline".to_string())
      }

      if self.reverse {
        msg_vec.push("reverse".to_string())
      }

      if self.hidden {
        msg_vec.push("hidden".to_string())
      }

      if self.strikethrough {
        msg_vec.push("strikethrough".to_string())
      }

      write!(
        f,
        "Style {{ {} | fg: {:?} | bg: {:?} | padding: {:?} }}",
        msg_vec.join(" + "),
        self.color_fg,
        self.color_bg,
        self.padding.unwrap_or_else(|| ch!(0))
      )
    }
  }
}

// ┏━━━━━━━━━━━━━━━━┓
// ┃ Style bitflags ┃
// ┛                ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
bitflags! {
  /// Bitflags for [Style].
  /// https://docs.rs/bitflags/0.8.2/bitflags/macro.bitflags.html
  #[derive(Serialize, Deserialize)]
  pub struct StyleFlag: u8 {
    const COLOR_FG_SET        = 0b0000_0001;
    const COLOR_BG_SET        = 0b0000_0010;
    const BOLD_SET            = 0b0000_0100;
    const DIM_SET             = 0b0000_1000;
    const UNDERLINE_SET       = 0b0001_0000;
    const PADDING_SET          = 0b0010_0000;
    const COMPUTED_SET        = 0b0100_0000;
    const REVERSE_SET         = 0b1000_0000;
    const HIDDEN_SET          = 0b1000_0001;
    const STRIKETHROUGH_SET   = 0b1000_0010;
  }
}

// ┏━━━━━━━━━━━━┓
// ┃ Style impl ┃
// ┛            ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
impl Style {
  pub fn remove_bg_color(&mut self) {
    self.color_bg = None;
    self.reset_bitflags();
    self.get_bitflags();
  }

  /// The `StyleFlag` is lazily computed and cached after the first time it is evaluated. A `Style`
  /// can be built simply or by using the [crate::style] proc macro and the expectation is that once
  /// built, the style won't be modified.
  pub fn get_bitflags(&mut self) -> StyleFlag {
    unwrap_option_or_compute_if_none! {
      self.cached_bitflags,
      || self.compute_bitflags()
    }
  }

  pub fn reset_bitflags(&mut self) { self.cached_bitflags = None; }

  fn compute_bitflags(&self) -> StyleFlag {
    let mut it = StyleFlag::empty();

    if self.color_fg.is_some() {
      it.insert(StyleFlag::COLOR_FG_SET);
    }
    if self.color_bg.is_some() {
      it.insert(StyleFlag::COLOR_BG_SET);
    }
    if self.bold {
      it.insert(StyleFlag::BOLD_SET);
    }
    if self.dim {
      it.insert(StyleFlag::DIM_SET);
    }
    if self.underline {
      it.insert(StyleFlag::UNDERLINE_SET);
    }
    if self.padding.is_some() {
      it.insert(StyleFlag::PADDING_SET);
    }
    if self.computed {
      it.insert(StyleFlag::COMPUTED_SET);
    }
    if self.reverse {
      it.insert(StyleFlag::REVERSE_SET);
    }
    if self.hidden {
      it.insert(StyleFlag::HIDDEN_SET);
    }
    if self.strikethrough {
      it.insert(StyleFlag::STRIKETHROUGH_SET);
    }

    it
  }
}

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ syntect::highlighting::Style -> Style ┃
// ┛                                       ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod conversion {
  use super::*;

  type SyntectStyle = syntect::highlighting::Style;
  type SyntectFontStyle = syntect::highlighting::FontStyle;
  type SyntectColor = syntect::highlighting::Color;

  impl From<SyntectStyle> for Style {
    fn from(st_style: SyntectStyle) -> Self {
      Style {
        color_fg: Some(st_style.foreground.into()),
        color_bg: Some(st_style.background.into()),
        bold: st_style.font_style.contains(SyntectFontStyle::BOLD),
        underline: st_style.font_style.contains(SyntectFontStyle::UNDERLINE),
        ..Default::default()
      }
    }
  }

  impl From<SyntectColor> for TuiColor {
    fn from(st_color: SyntectColor) -> Self {
      TuiColor::Rgb {
        r: st_color.r,
        g: st_color.g,
        b: st_color.b,
      }
    }
  }
}

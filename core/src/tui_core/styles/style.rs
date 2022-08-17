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

// ╭┄┄┄┄┄┄┄╮
// │ Style │
// ╯       ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Use [crate::style] proc macro to generate code for this struct. Here's an example.
///
/// ```ignore
/// // Turquoise:  Color::Rgb { r: 51, g: 255, b: 255 }
/// // Pink:       Color::Rgb { r: 252, g: 157, b: 248 }
/// // Blue:       Color::Rgb { r: 55, g: 55, b: 248 }
/// // Faded blue: Color::Rgb { r: 85, g: 85, b: 255 }
/// let mut stylesheet = Stylesheet::new();
///
/// stylesheet.add_styles(vec![
///   style! {
///     id: style1
///     margin: 1
///     color_bg: Color::Rgb { r: 55, g: 55, b: 248 }
///   },
///   style! {
///     id: style2
///     margin: 1
///     color_bg: Color::Rgb { r: 85, g: 85, b: 255 }
///   }
/// ])?;
/// ```
#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Style {
  pub id: String,
  pub bold: bool,
  pub dim: bool,
  pub underline: bool,
  pub reverse: bool,
  pub hidden: bool,
  pub strikethrough: bool,
  pub computed: bool,
  pub color_fg: Option<TWColor>,
  pub color_bg: Option<TWColor>,
  pub margin: Option<UnitType>,
  pub cached_bitflags: Option<StyleFlag>,
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Style helpers │
// ╯               ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
mod helpers {
  use super::*;

  /// Implement specificity behavior for [Style] by implementing [Add] trait. Here's the rule: `Style
  /// + Style (overrides) = Style`.
  ///
  /// Docs:
  /// - <https://doc.rust-lang.org/book/ch19-03-advanced-traits.html>
  impl Add<Self> for Style {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
      // Computed style has no id.
      self.computed = true;
      self.id = "".to_string();

      // other (if set) overrides self.
      let other_mask = other.clone().get_bitflags();
      if other_mask.contains(StyleFlag::COLOR_FG_SET) {
        self.color_fg = other.color_fg;
      }
      if other_mask.contains(StyleFlag::COLOR_BG_SET) {
        self.color_bg = other.color_bg;
      }
      if other_mask.contains(StyleFlag::BOLD_SET) {
        self.bold = other.bold;
      }
      if other_mask.contains(StyleFlag::DIM_SET) {
        self.dim = other.dim;
      }
      if other_mask.contains(StyleFlag::UNDERLINE_SET) {
        self.underline = other.underline;
      }
      if other_mask.contains(StyleFlag::MARGIN_SET) {
        self.margin = other.margin;
      }
      if other_mask.contains(StyleFlag::REVERSE_SET) {
        self.reverse = other.reverse;
      }
      if other_mask.contains(StyleFlag::HIDDEN_SET) {
        self.hidden = other.hidden;
      }
      if other_mask.contains(StyleFlag::STRIKETHROUGH_SET) {
        self.strikethrough = other.strikethrough;
      }

      // Recalculate the bitflags.
      self.reset_bitflags();
      self.get_bitflags();

      self
    }
  }

  impl AddAssign<&Style> for Style {
    fn add_assign(&mut self, other: &Style) { *self = self.clone() + other.clone(); }
  }

  impl Display for Style {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      let msg = format!("{:?}", self);
      f.write_str(&msg)
    }
  }

  impl Debug for Style {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
      let mut msg_vec: Vec<String> = vec![];

      if self.computed {
        msg_vec.push("computed".to_string())
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
        "Style {{ {} | fg: {:?} | bg: {:?} | margin: {:?} }}",
        msg_vec.join("+"),
        self.color_fg,
        self.color_bg,
        if self.margin.is_some() {
          self.margin.unwrap()
        } else {
          0
        }
      )
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Style bitflags │
// ╯                ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
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
    const MARGIN_SET          = 0b0010_0000;
    const COMPUTED_SET        = 0b0100_0000;
    const REVERSE_SET         = 0b1000_0000;
    const HIDDEN_SET          = 0b1000_0001;
    const STRIKETHROUGH_SET   = 0b1000_0010;
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Style impl │
// ╯            ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
impl Style {
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
    if self.margin.is_some() {
      it.insert(StyleFlag::MARGIN_SET);
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

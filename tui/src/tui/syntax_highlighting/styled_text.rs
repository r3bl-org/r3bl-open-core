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

use std::ops::{Add, AddAssign};

use r3bl_rs_utils_core::*;

use crate::*;

// ┏━━━━━━━━━━━━┓
// ┃ StyledText ┃
// ┛            ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Use [styled_text!] macro for easier construction.
#[derive(Debug, Clone)]
pub struct StyledText {
  plain_text: String,
  style: Style,
}

impl StyledText {
  /// Just as a precaution, the `text` argument is passed through
  /// [try_strip_ansi](ANSIText::try_strip_ansi) method to remove any ANSI escape sequences.
  pub fn new(text: String, style: Style) -> Self {
    let plain_text = match ANSIText::try_strip_ansi(&text) {
      Some(plain_text) => plain_text,
      None => text,
    };
    StyledText { plain_text, style }
  }

  pub fn get_plain_text(&self) -> &str { &self.plain_text }

  pub fn get_style(&self) -> &Style { &self.style }
}

/// Macro to make building [StyledText] easy.
///
/// Here's an example.
/// ```ignore
/// let st = styled_text! {
///   "Hello".to_string(),
///   maybe_style1.unwrap()
/// };
/// ```
#[macro_export]
macro_rules! styled_text {
  () => {
    StyledText::new(String::new(), Style::default())
  };
  ($text:expr) => {
    StyledText::new($text.to_string(), Style::default())
  };
  ($text:expr, $style:expr) => {
    StyledText::new($text.to_string(), $style)
  };
}

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Vec<StyledText>, StyledTexts ┃
// ┛                              ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
impl Add<StyledText> for Vec<StyledText> {
  type Output = Vec<StyledText>;
  fn add(mut self, other: StyledText) -> Self::Output {
    self.push(other);
    self
  }
}

impl AddAssign<StyledText> for Vec<StyledText> {
  fn add_assign(&mut self, other: StyledText) { self.push(other); }
}

pub trait StyledTexts {
  fn len(&self) -> usize;
  fn is_empty(&self) -> bool;
  fn get_plain_text(&self) -> String;
  fn render(&self, z_order: ZOrder) -> RenderPipeline;
  fn display_width(&self) -> ChUnit;
}

impl StyledTexts for Vec<StyledText> {
  fn len(&self) -> usize { self.len() }

  fn is_empty(&self) -> bool { self.is_empty() }

  fn get_plain_text(&self) -> String {
    let mut plain_text = String::new();
    for styled_text in self {
      plain_text.push_str(&styled_text.plain_text);
    }
    plain_text
  }

  fn display_width(&self) -> ChUnit {
    let unicode_string: UnicodeString = self.get_plain_text().into();
    unicode_string.display_width
  }

  fn render(&self, z_order: ZOrder) -> RenderPipeline {
    let mut pipeline = render_pipeline!(@new_empty);

    for styled_text in self {
      let style = styled_text.style.clone();
      let text = styled_text.plain_text.clone();
      render_pipeline! {
        @push_into pipeline at z_order =>
          RenderOp::ApplyColors(style.clone().into()),
          RenderOp::PrintTextWithAttributes(text, style.into()),
          RenderOp::ResetColor
      }
    }

    pipeline
  }
}

/// Macro to make building [`Vec<StyledText>`] easy.
///
/// Here's an example.
/// ```ignore
/// let mut st_vec = styled_texts! {
///   styled_text! {
///     "Hello".to_string(),
///     maybe_style1.unwrap()
///   },
///   styled_text! {
///     "World".to_string(),
///     maybe_style2.unwrap()
///   }
/// };
/// ```
#[macro_export]
macro_rules! styled_texts {
  ($($style:expr),*) => {
    {
      let mut styled_text_vec: Vec<StyledText> = Default::default();
      $(
        styled_text_vec += $style;
      )*
      styled_text_vec
    }
  };
}

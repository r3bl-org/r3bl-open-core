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

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ StyledText │
// ╯            ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Use [styled_text!] macro for easier construction.
#[derive(Debug, Clone)]
pub struct StyledText {
  plain_text: String,
  style: Style,
}

impl StyledText {
  /// Just as a precaution, the `text` argument is passed through [try_strip_ansi] method to remove
  /// any ANSI escape sequences.
  pub fn new(text: String, style: Style) -> Self {
    let plain_text = match try_strip_ansi(&text) {
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

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ StyledTextVec │
// ╯               ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Use [styled_text_vec!] macro for easier construction.
#[derive(Debug, Clone, Default)]
pub struct StyledTextVec {
  pub vec_spans: Vec<StyledText>,
}

impl Add<StyledText> for StyledTextVec {
  type Output = StyledTextVec;
  fn add(self, other: StyledText) -> StyledTextVec {
    let mut vec_spans = self.vec_spans;
    vec_spans.push(other);
    StyledTextVec { vec_spans }
  }
}

impl AddAssign<StyledText> for StyledTextVec {
  fn add_assign(&mut self, other: StyledText) { self.vec_spans.push(other); }
}

impl StyledTextVec {
  pub fn len(&self) -> usize { self.vec_spans.len() }

  pub fn is_empty(&self) -> bool { self.vec_spans.is_empty() }

  pub fn get_plain_text(&self) -> String {
    let mut plain_text = String::new();
    for styled_text in &self.vec_spans {
      plain_text.push_str(&styled_text.plain_text);
    }
    plain_text
  }

  pub fn render(&self) -> TWCommandQueue {
    let mut tw_command_queue = TWCommandQueue::default();

    for styled_text in &self.vec_spans {
      let style = styled_text.style.clone();
      let text = styled_text.plain_text.clone();
      tw_command_queue.push(TWCommand::ApplyColors(style.clone().into()));
      tw_command_queue.push(TWCommand::PrintWithAttributes(text, style.into()));
      tw_command_queue.push(TWCommand::ResetColor);
    }

    tw_command_queue
  }
}

/// Macro to make building [StyledTextVec] easy.
///
/// Here's an example.
/// ```ignore
/// let mut st_vec = styled_text_vec! {
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
macro_rules! styled_text_vec {
  ($($style:expr),*) => {
    {
      let mut styled_text_vec = StyledTextVec::default();
      $(
        styled_text_vec += $style;
      )*
      styled_text_vec
    }
  };
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ UnicodeStringExt │
// ╯                  ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
impl UnicodeStringExt for StyledTextVec {
  fn unicode_string(&self) -> UnicodeString { self.get_plain_text().unicode_string() }
}

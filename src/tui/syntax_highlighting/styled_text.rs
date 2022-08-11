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

use crate::*;

pub struct StyledText {
  pub plain_text: String,
  pub style: Style,
}

pub struct StyledTextVec {
  pub vec: Vec<StyledText>,
}

impl StyledTextVec {
  pub fn get_plain_text(&self) -> String {
    let mut plain_text = String::new();
    for styled_text in &self.vec {
      plain_text.push_str(&styled_text.plain_text);
    }
    plain_text
  }

  pub fn render(&self) -> TWCommandQueue {
    // TODO:
    todo!();
  }
}

impl UnicodeStringExt for StyledTextVec {
  fn unicode_string(&self) -> UnicodeString {
    // TODO: use the plain_text field (which is a String) and re-use its trait impl
    todo!()
  }
}

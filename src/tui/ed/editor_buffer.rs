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

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::*;

#[derive(Clone, Default, PartialEq, Serialize, Deserialize, GetSize)]
pub struct EditorBuffer {
  /// A list of lines representing the document being edited.
  pub vec_lines: Vec<String>,
  /// The current caret position.
  pub caret: Position,
  /// The col and row offset for scrolling if active.
  pub scroll_offset: Position,
  /// Lolcat struct for generating rainbow colors.
  pub lolcat: Lolcat,
}

pub fn char_to_string(character: char) -> String {
  let my_string: String = String::from(character);
  my_string
}

impl EditorBuffer {
  pub fn get_char_at_caret(&self) -> Option<char> { self.get_char_at_position(self.caret) }
  pub fn get_char_at_position(&self, position: Position) -> Option<char> {
    let line = self.vec_lines.get(convert_from_base_unit!(position.row))?;
    let character = line.chars().nth(convert_from_base_unit!(position.col))?;
    Some(character)
  }

  pub fn insert_char_into_current_line(&mut self, character: char) {
    self.insert_into_current_line(&char_to_string(character))
  }

  pub fn insert_str_into_current_line(&mut self, chunk: &str) {
    self.insert_into_current_line(chunk)
  }

  fn insert_into_current_line(&mut self, chunk: &str) {
    let caret_row = convert_from_base_unit!(self.caret.row);
    let caret_col = convert_from_base_unit!(self.caret.col);
    if self.vec_lines.get(caret_row).is_some() {
      insert_into_existing_line(self, caret_row, caret_col, chunk);
    } else {
      insert_into_new_line(self, caret_row, chunk);
    }

    /// Helper function.
    fn insert_into_existing_line(
      this: &mut EditorBuffer, caret_row: usize, caret_col: usize, chunk: &str,
    ) {
      // Update existing line at caret_row.
      if let Some(line) = this.vec_lines.get_mut(caret_row) {
        // Get the new line.
        if let Ok((new_line, char_display_width)) = line
          .unicode_string()
          .insert_char_at_display_col(convert_to_base_unit!(caret_col), chunk)
        {
          // Replace existing line w/ new line.
          let _ = std::mem::replace(line, new_line);

          // Update caret position.
          let char_display_width = convert_from_base_unit!(char_display_width);
          this.caret.add_cols(char_display_width);
        }
      }
    }

    /// Helper function.
    fn insert_into_new_line(this: &mut EditorBuffer, caret_row: usize, chunk: &str) {
      // Fill in any missing lines.
      if this.vec_lines.get(caret_row).is_none() {
        for row_idx in 0..caret_row + 1 {
          if this.vec_lines.get(row_idx).is_none() {
            this.vec_lines.push(String::new());
          }
        }
      }
      // Actually add the character to the correct line.
      if let Some(line) = this.vec_lines.get_mut(caret_row) {
        line.push_str(chunk);
        this.caret.add_cols(UnicodeString::str_display_width(chunk));
      }
    }
  }
}

mod editor_buffer_helpers {
  use super::*;

  impl std::fmt::Debug for EditorBuffer {
    fn fmt(&self, f: &mut __private::Formatter<'_>) -> std::fmt::Result {
      write! { f,
        "\nEditorBuffer [ \n ├ lines: {}, size: {}, \n ├ cursor: {:?}, scroll_offset: {:?}, \n └ lolcat: [{}, {}, {}, {}] \n]",
        self.vec_lines.len(),
        self.vec_lines.get_heap_size(),
        self.caret,
        self.scroll_offset,
        pretty_print_f64(self.lolcat.color_wheel_control.seed),
        pretty_print_f64(self.lolcat.color_wheel_control.spread),
        pretty_print_f64(self.lolcat.color_wheel_control.frequency),
        self.lolcat.color_wheel_control.color_change_speed
      }
    }
  }

  /// More info: <https://stackoverflow.com/questions/63214346/how-to-truncate-f64-to-2-decimal-places>
  fn pretty_print_f64(before: f64) -> f64 { f64::trunc(before * 100.0) / 100.0 }
}

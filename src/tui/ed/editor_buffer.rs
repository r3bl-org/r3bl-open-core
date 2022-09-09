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
  /// The current caret position. This is the "display" and not "logical" position as defined in
  /// [UnicodeString]. This works w/ [crate::RenderOp] as well, so you can directly move this
  /// position.
  pub caret: Position,
  /// The col and row offset for scrolling if active.
  pub scroll_offset: Position,
  /// Lolcat struct for generating rainbow colors.
  pub lolcat: Lolcat,
}

macro_rules! empty_check_early_return {
  ($arg_this: expr, @None) => {
    if $arg_this.vec_lines.is_empty() {
      return None;
    }
  };
  ($arg_this: expr, @Nothing) => {
    if $arg_this.vec_lines.is_empty() {
      return;
    }
  };
}

mod helpers {
  pub(super) fn char_to_string(character: char) -> String {
    let my_string: String = String::from(character);
    my_string
  }

  pub(super) enum CaretLocationInLine {
    AtStart,
    AtEnd,
    InMiddle,
  }
}
use helpers::*;

impl EditorBuffer {
  pub fn is_empty(&self) -> bool { self.vec_lines.is_empty() }

  pub fn get_string_to_left_of_caret(&self) -> Option<(String, UnitType)> {
    self.get_string_to_left_of_position(self.caret)
  }
  pub fn get_string_to_left_of_position(&self, position: Position) -> Option<(String, UnitType)> {
    empty_check_early_return!(self, @None);
    let line = self.vec_lines.get(convert_from_base_unit!(position.row))?;
    line
      .unicode_string()
      .get_string_at_left_of_display_col(position.col)
  }

  pub fn get_string_at_caret(&self) -> Option<(String, UnitType)> {
    self.get_string_at_position(self.caret)
  }
  pub fn get_string_at_position(&self, position: Position) -> Option<(String, UnitType)> {
    empty_check_early_return!(self, @None);
    let line = self.vec_lines.get(convert_from_base_unit!(position.row))?;
    let (str_seg, unicode_width) = line
      .unicode_string()
      .get_string_at_display_col(position.col)?;
    Some((str_seg, unicode_width))
  }

  pub fn get_line_at_caret(&self) -> Option<&String> { self.get_line_at_position(self.caret) }
  pub fn get_line_at_position(&self, position: Position) -> Option<&String> {
    empty_check_early_return!(self, @None);
    let line = self.vec_lines.get(convert_from_base_unit!(position.row))?;
    Some(line)
  }

  pub fn get_display_width_of_line_at_caret(&self) -> UnitType {
    let line = self.get_line_at_caret();
    if let Some(line) = line {
      line.unicode_string().display_width
    } else {
      0
    }
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

  pub fn caret_is_at_end_of_current_line(&self) -> bool {
    if let Some(line) = self.get_line_at_caret() {
      let line_display_width = line.unicode_string().display_width;
      self.caret.col == convert_to_base_unit!(line_display_width)
    } else {
      false
    }
  }

  pub fn caret_is_at_start_of_current_line(&self) -> bool {
    if self.get_line_at_caret().is_some() {
      self.caret.col == 0
    } else {
      false
    }
  }

  pub fn get_string_at_end_of_current_line(&self) -> Option<(String, UnitType)> {
    let line = self.get_line_at_caret()?;
    if self.caret_is_at_end_of_current_line() {
      let maybe_last_str_seg = line.unicode_string().get_string_at_end();
      return maybe_last_str_seg;
    }
    None
  }

  fn where_is_caret_in_current_line(&self) -> CaretLocationInLine {
    if self.caret_is_at_start_of_current_line() {
      CaretLocationInLine::AtStart
    } else if self.caret_is_at_end_of_current_line() {
      CaretLocationInLine::AtEnd
    } else {
      CaretLocationInLine::InMiddle
    }
  }

  /// Move one character to the left. Figure out how wide the current character is (unicode width)
  /// and then move the "display" caret position back that many columns.
  pub fn move_caret_left(&mut self) {
    empty_check_early_return!(self, @Nothing);
    match self.where_is_caret_in_current_line() {
      CaretLocationInLine::AtStart => {
        // Do nothing.
      }
      CaretLocationInLine::AtEnd => {
        if let Some((_, unicode_width)) = self.get_string_at_end_of_current_line() {
          dec_unsigned!(self.caret.col, by: unicode_width);
        }
      }
      CaretLocationInLine::InMiddle => {
        if let Some((_, unicode_width)) = self.get_string_to_left_of_caret() {
          dec_unsigned!(self.caret.col, by: unicode_width);
        }
      }
    }
  }

  /// Move one character to the right. Figure out how wide the current character is (unicode width)
  /// and then move the "display" caret position forward that many columns.
  pub fn move_caret_right(&mut self) {
    empty_check_early_return!(self, @Nothing);
    match self.where_is_caret_in_current_line() {
      CaretLocationInLine::AtEnd => {
        // Do nothing.
      }
      CaretLocationInLine::AtStart | CaretLocationInLine::InMiddle => {
        if let Some((_, unicode_width)) = self.get_string_at_caret() {
          let max_display_width = self.get_display_width_of_line_at_caret();
          inc_unsigned!(self.caret.col, by: unicode_width, max: max_display_width);
        }
      }
    }
  }
}

mod debug_format_helpers {
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

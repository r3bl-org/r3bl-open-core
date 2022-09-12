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

use r3bl_rs_utils_core::*;

use crate::*;

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

pub enum CaretLocation {
  AtStartOfLine,
  AtEndOfLine,
  InMiddleOfLine,
}

pub mod line_buffer_move_caret {
  use super::*;

  pub fn right(this: &mut EditorBuffer) {
    empty_check_early_return!(this, @Nothing);
    match line_buffer_locate_caret::find(this) {
      CaretLocation::AtEndOfLine => {
        // Do nothing.
      }
      CaretLocation::AtStartOfLine | CaretLocation::InMiddleOfLine => {
        if let Some((_, unicode_width)) = line_buffer_get_content::string_at_caret(this) {
          let max_display_width = line_buffer_get_content::display_width_of_line(this);
          this
            .caret
            .add_cols_with_bounds(unicode_width, max_display_width);
        }
      }
    }
  }

  pub fn left(this: &mut EditorBuffer) {
    empty_check_early_return!(this, @Nothing);
    match line_buffer_locate_caret::find(this) {
      CaretLocation::AtStartOfLine => {
        // Do nothing.
      }
      CaretLocation::AtEndOfLine => {
        if let Some((_, unicode_width)) = line_buffer_get_content::string_at_end_of_line(this) {
          this.caret.col -= unicode_width;
        }
      }
      CaretLocation::InMiddleOfLine => {
        if let Some((_, unicode_width)) = line_buffer_get_content::string_to_left_of_caret(this) {
          this.caret.col -= unicode_width;
        }
      }
    }
  }
}

pub mod line_buffer_get_content {
  use super::*;

  pub fn display_width_of_line(this: &EditorBuffer) -> ChUnit {
    let line = line_buffer_get_content::line_as_string(this);
    if let Some(line) = line {
      line.unicode_string().display_width
    } else {
      ch!(0)
    }
  }

  pub fn line_as_string(this: &EditorBuffer) -> Option<&String> {
    let position = this.caret;
    empty_check_early_return!(this, @None);
    let line = this.vec_lines.get(ch!(@to_usize position.row))?;
    Some(line)
  }

  pub fn string_at_caret(this: &EditorBuffer) -> Option<(String, ChUnit)> {
    let position = this.caret;
    empty_check_early_return!(this, @None);
    let line = this.vec_lines.get(ch!(@to_usize position.row))?;
    let (str_seg, unicode_width) = line
      .unicode_string()
      .get_string_at_display_col(position.col)?;
    Some((str_seg, unicode_width))
  }

  pub fn string_to_left_of_caret(this: &EditorBuffer) -> Option<(String, ChUnit)> {
    let position = this.caret;
    empty_check_early_return!(this, @None);
    let line = this.vec_lines.get(ch!(@to_usize position.row))?;
    line
      .unicode_string()
      .get_string_at_left_of_display_col(position.col)
  }

  pub fn string_at_end_of_line(this: &EditorBuffer) -> Option<(String, ChUnit)> {
    let line = line_buffer_get_content::line_as_string(this)?;
    if let CaretLocation::AtEndOfLine = line_buffer_locate_caret::find(this) {
      let maybe_last_str_seg = line.unicode_string().get_string_at_end();
      return maybe_last_str_seg;
    }
    None
  }
}

pub mod line_buffer_locate_caret {
  use super::*;

  pub fn find(this: &EditorBuffer) -> CaretLocation {
    if line_buffer_locate_caret::is_at_start_of_line(this) {
      CaretLocation::AtStartOfLine
    } else if line_buffer_locate_caret::is_at_end_of_line(this) {
      CaretLocation::AtEndOfLine
    } else {
      CaretLocation::InMiddleOfLine
    }
  }

  fn is_at_end_of_line(this: &EditorBuffer) -> bool {
    if let Some(line) = line_buffer_get_content::line_as_string(this) {
      let line_display_width = line.unicode_string().display_width;
      this.caret.col == line_display_width
    } else {
      false
    }
  }

  fn is_at_start_of_line(this: &EditorBuffer) -> bool {
    if line_buffer_get_content::line_as_string(this).is_some() {
      *this.caret.col == 0
    } else {
      false
    }
  }
}

pub mod line_buffer_insert {
  use super::*;

  pub fn at_caret(this: &mut EditorBuffer, chunk: &str) {
    let caret_row: usize = ch!(@to_usize this.caret.row);
    let caret_col: usize = ch!(@to_usize this.caret.col);

    if this.vec_lines.get(caret_row).is_some() {
      insert_into_existing_line(this, caret_row, caret_col, chunk);
    } else {
      fill_in_missing_lines_up_to_caret(this, caret_row);
      insert_into_new_line(this, caret_row, chunk);
    }
  }

  fn insert_into_existing_line(
    this: &mut EditorBuffer, caret_row: usize, caret_col: usize, chunk: &str,
  ) {
    // Update existing line at caret_row.
    if let Some(line) = this.vec_lines.get_mut(caret_row) {
      // Get the new line.
      if let Ok((new_line, char_display_width)) = line
        .unicode_string()
        .insert_char_at_display_col(ch!(caret_col), chunk)
      {
        // Replace existing line w/ new line.
        let _ = std::mem::replace(line, new_line);

        // Update caret position.
        let char_display_width = ch!(@to_usize char_display_width);
        this.caret.add_cols(char_display_width);
      }
    }
  }

  fn fill_in_missing_lines_up_to_caret(this: &mut EditorBuffer, caret_row: usize) {
    // Fill in any missing lines.
    if this.vec_lines.get(caret_row).is_none() {
      for row_idx in 0..caret_row + 1 {
        if this.vec_lines.get(row_idx).is_none() {
          this.vec_lines.push(String::new());
        }
      }
    }
  }

  fn insert_into_new_line(this: &mut EditorBuffer, caret_row: usize, chunk: &str) {
    // Actually add the character to the correct line.
    if let Some(line) = this.vec_lines.get_mut(caret_row) {
      line.push_str(chunk);
      this.caret.add_cols(UnicodeString::str_display_width(chunk));
    }
  }
}

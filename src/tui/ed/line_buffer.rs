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

use std::mem::replace;

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::*;

macro_rules! empty_check_early_return {
  ($arg_this: expr, @None) => {
    if $arg_this.is_empty() {
      return None;
    }
  };
  ($arg_this: expr, @Nothing) => {
    if $arg_this.is_empty() {
      return;
    }
  };
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, GetSize)]
pub enum CaretColLocation {
  /// Also covers state where there is no col, or only 1 col.
  AtStartOfLine,
  AtEndOfLine,
  InMiddleOfLine,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, GetSize)]
pub enum CaretRowLocation {
  /// Also covers state where there is no row, or only 1 row.
  AtTopOfBuffer,
  AtBottomOfBuffer,
  InMiddleOfBuffer,
}

pub mod validator {
  use super::*;

  /// It is possible when moving the caret for it to end up in the middle of a grapheme cluster. In
  /// this case, move the caret to the end of the cluster, since the intention is to move the caret
  /// to the next "character".
  pub fn validate_caret_col_position(this: &mut EditorBuffer) -> Option<()> {
    let line = line_buffer_get_content::line_as_string(this)?;
    let line_us = line.unicode_string();

    if let Some(segment) = line_us.is_display_col_in_middle_of_grapheme_cluster(this.caret.col) {
      // Is in middle.
      this
        .caret
        .set_cols(segment.unicode_width + segment.display_col_offset);
    }

    None
  }
}

pub mod line_buffer_move_caret {
  use super::*;

  pub fn up(this: &mut EditorBuffer) -> Option<()> {
    empty_check_early_return!(this, @None);
    match line_buffer_locate_caret::find_row(this) {
      CaretRowLocation::AtTopOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtBottomOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        if *this.caret.row > 0 {
          this.caret.row -= 1;
          this
            .caret
            .clip_cols_to_bounds(line_buffer_get_content::line_display_width(this));
          validator::validate_caret_col_position(this);
        }
      }
    }
    None
  }

  pub fn down(this: &mut EditorBuffer) -> Option<()> {
    empty_check_early_return!(this, @None);
    match line_buffer_locate_caret::find_row(this) {
      CaretRowLocation::AtBottomOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtTopOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        let max_row = ch!(this.len(), @dec);
        if this.caret.row < max_row {
          this.caret.row += 1;
          this
            .caret
            .clip_cols_to_bounds(line_buffer_get_content::line_display_width(this));
          validator::validate_caret_col_position(this);
        }
      }
    }
    None
  }

  pub fn right(this: &mut EditorBuffer) -> Option<()> {
    empty_check_early_return!(this, @None);
    match line_buffer_locate_caret::find_col(this) {
      CaretColLocation::AtEndOfLine => {
        // Do nothing.
      }
      CaretColLocation::AtStartOfLine | CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          line_buffer_get_content::string_at_caret(this)?;
        let max_display_width = line_buffer_get_content::line_display_width(this);
        this
          .caret
          .add_cols_with_bounds(unicode_width, max_display_width);
      }
    }
    None
  }

  pub fn left(this: &mut EditorBuffer) -> Option<()> {
    empty_check_early_return!(this, @None);
    match line_buffer_locate_caret::find_col(this) {
      CaretColLocation::AtStartOfLine => {
        // Do nothing.
      }
      CaretColLocation::AtEndOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          line_buffer_get_content::string_at_end_of_line(this)?;
        this.caret.col -= unicode_width;
        validator::validate_caret_col_position(this);
      }
      CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          line_buffer_get_content::string_to_left_of_caret(this)?;
        this.caret.col -= unicode_width;
        validator::validate_caret_col_position(this);
      }
    }
    None
  }
}

pub mod line_buffer_get_content {
  use super::*;

  pub fn line_display_width(this: &EditorBuffer) -> ChUnit {
    let line = line_buffer_get_content::line_as_string(this);
    if let Some(line) = line {
      line.unicode_string().display_width
    } else {
      ch!(0)
    }
  }

  pub fn line_as_string(this: &EditorBuffer) -> Option<String> {
    empty_check_early_return!(this, @None);
    let position = this.caret;
    let line = this.get(ch!(@to_usize position.row))?;
    Some(line.clone())
  }

  pub fn next_line_as_string(this: &EditorBuffer) -> Option<String> {
    empty_check_early_return!(this, @None);
    let position = this.caret;
    let line = this.get(ch!(@to_usize position.row, @inc))?;
    Some(line.clone())
  }

  pub fn prev_line_as_string(this: &EditorBuffer) -> Option<String> {
    empty_check_early_return!(this, @None);
    let position = this.caret;
    if position.row == ch!(0) {
      return None;
    }
    let line = this.get(ch!(@to_usize position.row, @dec))?;
    Some(line.clone())
  }

  pub fn string_at_caret(this: &EditorBuffer) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(this, @None);
    let position = this.caret;
    let line = this.get(ch!(@to_usize position.row))?;
    let unicode_string = line.unicode_string();
    let result = unicode_string.get_string_at_display_col(position.col)?;
    Some(result)
  }

  pub fn string_to_left_of_caret(this: &EditorBuffer) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(this, @None);
    match line_buffer_locate_caret::find_col(this) {
      // Caret is at end of line, past the last character.
      CaretColLocation::AtEndOfLine => {
        let mut caret_copy = this.caret;
        caret_copy.sub_cols(1);
        let line = this.get(ch!(@to_usize caret_copy.row))?;
        let unicode_string = line.unicode_string();
        unicode_string.get_string_at_display_col(caret_copy.col)
      }
      // Caret is not at end of line.
      _ => {
        let position = this.caret;
        let line = this.get(ch!(@to_usize position.row))?;
        let unicode_string = line.unicode_string();
        unicode_string.get_string_at_left_of_display_col(position.col)
      }
    }
  }

  pub fn string_at_end_of_line(this: &EditorBuffer) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(this, @None);
    let line = line_buffer_get_content::line_as_string(this)?;
    if let CaretColLocation::AtEndOfLine = line_buffer_locate_caret::find_col(this) {
      let maybe_last_str_seg = line.unicode_string().get_string_at_end();
      return maybe_last_str_seg;
    }
    None
  }
}

pub mod line_buffer_locate_caret {
  use super::*;

  /// Locate the col.
  pub fn find_col(this: &EditorBuffer) -> CaretColLocation {
    if line_buffer_locate_caret::col_is_at_start_of_line(this) {
      CaretColLocation::AtStartOfLine
    } else if line_buffer_locate_caret::col_is_at_end_of_line(this) {
      CaretColLocation::AtEndOfLine
    } else {
      CaretColLocation::InMiddleOfLine
    }
  }

  fn col_is_at_start_of_line(this: &EditorBuffer) -> bool {
    if line_buffer_get_content::line_as_string(this).is_some() {
      *this.caret.col == 0
    } else {
      false
    }
  }

  fn col_is_at_end_of_line(this: &EditorBuffer) -> bool {
    if let Some(line) = line_buffer_get_content::line_as_string(this) {
      let line_display_width = line.unicode_string().display_width;
      this.caret.col == line_display_width
    } else {
      false
    }
  }

  /// Locate the row.
  pub fn find_row(this: &EditorBuffer) -> CaretRowLocation {
    if row_is_at_top_of_buffer(this) {
      CaretRowLocation::AtTopOfBuffer
    } else if row_is_at_bottom_of_buffer(this) {
      CaretRowLocation::AtBottomOfBuffer
    } else {
      CaretRowLocation::InMiddleOfBuffer
    }
  }

  // R ┌──────────┐
  // 0 ▸          │
  //   └▴─────────┘
  //   C0123456789
  fn row_is_at_top_of_buffer(this: &EditorBuffer) -> bool { *this.caret.row == 0 }

  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸a         │
  //   └▴─────────┘
  //   C0123456789
  fn row_is_at_bottom_of_buffer(this: &EditorBuffer) -> bool {
    if this.is_empty() || this.len() == 1 {
      false // If there is only one line, then the caret is not at the bottom, its at the top.
    } else {
      let max_row_count = ch!(this.len(), @dec);
      this.caret.row == max_row_count
    }
  }
}

pub mod line_buffer_mut {
  use super::*;

  pub fn insert_str_at_caret(this: &mut EditorBuffer, chunk: &str) {
    let caret_row: usize = ch!(@to_usize this.caret.row);
    let caret_col: usize = ch!(@to_usize this.caret.col);

    if this.get(caret_row).is_some() {
      insert_into_existing_line(this, caret_row, caret_col, chunk);
    } else {
      fill_in_missing_lines_up_to_row(this, caret_row);
      insert_into_new_line(this, caret_row, chunk);
    }
  }

  pub fn insert_new_line_at_caret(this: &mut EditorBuffer) {
    if this.is_empty() {
      this.mutate_lines(|lines, _| {
        lines.push(String::new());
      });
      return;
    }

    match line_buffer_locate_caret::find_col(this) {
      CaretColLocation::AtEndOfLine => insert_new_line_at_end_of_current_line(this),
      CaretColLocation::AtStartOfLine => insert_new_line_at_start_of_current_line(this),
      CaretColLocation::InMiddleOfLine => insert_new_line_at_middle_of_current_line(this),
    }

    // Handle inserting a new line at the end of the current line.
    fn insert_new_line_at_end_of_current_line(this: &mut EditorBuffer) {
      this.mutate_caret(|caret| {
        caret.add_rows(1);
        caret.reset_cols();
      });
      // insert empty line at caret.
      this.mutate_lines(|lines, caret| {
        lines.insert(ch!(@to_usize caret.row), String::new());
      });
    }

    // Handle inserting a new line at the start of the current line.
    fn insert_new_line_at_start_of_current_line(this: &mut EditorBuffer) {
      let row_index = ch!(@to_usize this.caret.row);
      if row_index == 0 {
        this.mutate_lines(|lines, _| {
          lines.insert(0, String::new());
        });
      } else {
        this.mutate_lines(|lines, _| {
          lines.insert(row_index, String::new());
        });
      }
      this.mutate_caret(|caret| {
        caret.add_rows(1);
      });
    }

    // Handle inserting a new line at the middle of the current line.
    fn insert_new_line_at_middle_of_current_line(this: &mut EditorBuffer) {
      if let Some(line_content) = line_buffer_get_content::line_as_string(this) {
        let unicode_string = line_content.unicode_string();
        let split_result = unicode_string.split_at_display_col(this.caret.col);

        log_no_err_debug!(split_result);

        if let Some((
          NewUnicodeStringResult {
            new_string: left, ..
          },
          NewUnicodeStringResult {
            new_string: right, ..
          },
        )) = split_result
        {
          let row_index = ch!(@to_usize this.caret.row);
          this.mutate_lines(|lines, _| {
            let _ = replace(&mut lines[row_index], left);
            lines.insert(row_index + 1, right);
          });
          this.mutate_caret(|caret| {
            caret.add_rows(1);
            caret.reset_cols();
          });
        }
      }
    }
  }

  pub fn delete_at_caret(this: &mut EditorBuffer) -> Option<()> {
    empty_check_early_return!(this, @None);
    if line_buffer_get_content::string_at_caret(this).is_some() {
      delete_in_middle_of_line(this)?;
    } else {
      delete_at_end_of_line(this)?;
    }
    return None;

    // R ┌──────────┐
    // 0 ▸abc       │
    // 1 │ab        │
    // 2 │a         │
    //   └─▴────────┘
    //   C0123456789
    fn delete_in_middle_of_line(this: &mut EditorBuffer) -> Option<()> {
      let cur_line = line_buffer_get_content::line_as_string(this)?;
      let unicode_string = cur_line.unicode_string();
      let NewUnicodeStringResult {
        new_string: new_line,
        ..
      } = unicode_string.delete_char_at_display_col(this.caret.col)?;
      let row_index = ch!(@to_usize this.caret.row);
      this.mutate_lines(|lines, _| {
        let _ = replace(&mut lines[row_index], new_line);
      });
      None
    }

    // R ┌──────────┐
    // 0 ▸abc       │
    // 1 │ab        │
    // 2 │a         │
    //   └───▴──────┘
    //   C0123456789
    fn delete_at_end_of_line(this: &mut EditorBuffer) -> Option<()> {
      let this_line = line_buffer_get_content::line_as_string(this)?;
      let next_line = line_buffer_get_content::next_line_as_string(this)?;

      this.mutate_lines(|lines, caret| {
        let _ = replace(&mut lines[ch!(@to_usize caret.row)], this_line + &next_line);
        lines.remove(ch!(@to_usize caret.row, @inc));
      });
      None
    }
  }

  pub fn backspace_at_caret(this: &mut EditorBuffer) -> Option<()> {
    empty_check_early_return!(this, @None);
    if let Some(UnicodeStringSegmentSliceResult {
      display_col_at_which_seg_starts,
      ..
    }) = line_buffer_get_content::string_to_left_of_caret(this)
    {
      backspace_in_middle_of_line(this, display_col_at_which_seg_starts)?;
    } else {
      backspace_at_start_of_line(this)?;
    }
    return None;

    // R ┌──────────┐
    // 0 ▸abc       │
    // 1 │ab        │
    // 2 │a         │
    //   └─▴────────┘
    //   C0123456789
    fn backspace_in_middle_of_line(
      this: &mut EditorBuffer, delete_at_this_display_idx: ChUnit,
    ) -> Option<()> {
      let cur_line = line_buffer_get_content::line_as_string(this)?;
      let unicode_string = cur_line.unicode_string();
      let NewUnicodeStringResult {
        new_string: new_line,
        ..
      } = unicode_string.delete_char_at_display_col(delete_at_this_display_idx)?;
      this.mutate_lines(|lines, caret| {
        let _ = replace(&mut lines[ch!(@to_usize caret.row)], new_line);
      });
      this.mutate_caret(|caret| {
        caret.set_cols(delete_at_this_display_idx);
      });

      None
    }

    // R ┌──────────┐
    // 0 │abc       │
    // 1 ▸ab        │
    // 2 │a         │
    //   └▴─────────┘
    //   C0123456789
    fn backspace_at_start_of_line(this: &mut EditorBuffer) -> Option<()> {
      let this_line = line_buffer_get_content::line_as_string(this)?;
      let prev_line = line_buffer_get_content::prev_line_as_string(this)?;
      let prev_line_cols = prev_line.unicode_string().display_width;
      this.mutate_lines(|lines, caret| {
        let _ = replace(
          &mut lines[ch!(@to_usize caret.row, @dec)],
          prev_line + &this_line,
        );
        lines.remove(ch!(@to_usize caret.row));
      });
      this.mutate_caret(|caret| {
        caret.sub_rows(1);
        caret.set_cols(prev_line_cols);
      });

      None
    }
  }

  fn insert_into_existing_line(
    this: &mut EditorBuffer, caret_row: usize, caret_col: usize, chunk: &str,
  ) -> Option<()> {
    let line = this.get(caret_row)?;

    let NewUnicodeStringResult {
      new_string: new_line,
      unicode_width: char_display_width,
    } = line
      .unicode_string()
      .insert_char_at_display_col(ch!(caret_col), chunk)?;

    this.mutate_lines(|lines, caret| {
      // Replace existing line w/ new line.
      let _ = replace(&mut lines[caret_row], new_line);

      // Update caret position.
      caret.add_cols(ch!(@to_usize char_display_width));
    });

    None
  }

  fn fill_in_missing_lines_up_to_row(this: &mut EditorBuffer, caret_row: usize) {
    // Fill in any missing lines.
    if this.get(caret_row).is_none() {
      for row_idx in 0..caret_row + 1 {
        if this.get(row_idx).is_none() {
          this.mutate_lines(|lines, _| {
            lines.push(String::new());
          });
        }
      }
    }
  }

  fn insert_into_new_line(this: &mut EditorBuffer, caret_row: usize, chunk: &str) -> Option<()> {
    // Actually add the character to the correct line.
    let _ = this.get(caret_row)?;

    this.mutate_lines(|lines, caret| {
      let _ = replace(&mut lines[ch!(@to_usize caret_row)], chunk.to_string());
      caret.add_cols(UnicodeString::str_display_width(chunk));
    });

    None
  }
}

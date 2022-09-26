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
  ($arg_buffer: expr, @None) => {
    if $arg_buffer.is_empty() {
      return None;
    }
  };
  ($arg_buffer: expr, @Nothing) => {
    if $arg_buffer.is_empty() {
      return;
    }
  };
}

// ╭┄┄┄┄┄┄┄┄┄┄┄╮
// │ Caret get │
// ╯           ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod editor_ops_caret {
  use super::*;

  /// Locate the col.
  pub fn find_col(buffer: &EditorBuffer) -> CaretColLocation {
    if editor_ops_caret::col_is_at_start_of_line(buffer) {
      CaretColLocation::AtStartOfLine
    } else if editor_ops_caret::col_is_at_end_of_line(buffer) {
      CaretColLocation::AtEndOfLine
    } else {
      CaretColLocation::InMiddleOfLine
    }
  }

  fn col_is_at_start_of_line(buffer: &EditorBuffer) -> bool {
    if editor_ops_content::line_at_caret_to_string(buffer).is_some() {
      *buffer.get_caret().col == 0
    } else {
      false
    }
  }

  fn col_is_at_end_of_line(buffer: &EditorBuffer) -> bool {
    if let Some(line) = editor_ops_content::line_at_caret_to_string(buffer) {
      buffer.get_caret().col == line.display_width
    } else {
      false
    }
  }

  /// Locate the row.
  pub fn find_row(buffer: &EditorBuffer) -> CaretRowLocation {
    if row_is_at_top_of_buffer(buffer) {
      CaretRowLocation::AtTopOfBuffer
    } else if row_is_at_bottom_of_buffer(buffer) {
      CaretRowLocation::AtBottomOfBuffer
    } else {
      CaretRowLocation::InMiddleOfBuffer
    }
  }

  // R ┌──────────┐
  // 0 ▸          │
  //   └▴─────────┘
  //   C0123456789
  fn row_is_at_top_of_buffer(buffer: &EditorBuffer) -> bool { *buffer.get_caret().row == 0 }

  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸a         │
  //   └▴─────────┘
  //   C0123456789
  fn row_is_at_bottom_of_buffer(buffer: &EditorBuffer) -> bool {
    if buffer.is_empty() || buffer.get_lines().len() == 1 {
      false // If there is only one line, then the caret is not at the bottom, its at the top.
    } else {
      let max_row_count = ch!(buffer.get_lines().len(), @dec);
      buffer.get_caret().row == max_row_count
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄╮
// │ Caret mut │
// ╯           ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
// TK: 📜 set engine { scroll_offset } to adjust the caret movement
pub mod editor_ops_caret_mut {
  use super::*;

  pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    // let EditorArgs { buffer, engine } = args;
    empty_check_early_return!(buffer, @None);
    match editor_ops_caret::find_row(buffer) {
      CaretRowLocation::AtTopOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtBottomOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        let min_row = ch!(0);
        if buffer.get_caret().row > min_row {
          let new_row = buffer.get_caret().row - 1;
          let line_display_width = editor_ops_content::line_display_width_at_row(buffer, new_row);
          mutate::change_editor_buffer(buffer, engine, |_, caret| {
            caret.row = new_row;
            caret.clip_col_to_bounds(line_display_width);
          });
        }
      }
    }

    None
  }

  pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_caret::find_row(buffer) {
      CaretRowLocation::AtBottomOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtTopOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        let max_row = ch!(buffer.get_lines().len(), @dec);
        if buffer.get_caret().row < max_row {
          let new_row = buffer.get_caret().row + 1;
          let line_display_width = editor_ops_content::line_display_width_at_row(buffer, new_row);
          mutate::change_editor_buffer(buffer, engine, |_, caret| {
            caret.row = new_row;
            caret.clip_col_to_bounds(line_display_width);
          });
        }
      }
    }

    None
  }

  pub fn right(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_caret::find_col(buffer) {
      CaretColLocation::AtEndOfLine => {
        // Do nothing.
      }
      CaretColLocation::AtStartOfLine | CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_content::string_at_caret(buffer, engine)?;
        let max_display_width = editor_ops_content::line_display_width_at_caret(buffer);
        mutate::change_editor_buffer(buffer, engine, |_, caret| {
          caret.add_col_with_bounds(unicode_width, max_display_width);
        });
      }
    }

    None
  }

  pub fn left(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_caret::find_col(buffer) {
      CaretColLocation::AtStartOfLine => {
        // Do nothing.
      }
      CaretColLocation::AtEndOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_content::string_at_end_of_line_at_caret(buffer)?;
        mutate::change_editor_buffer(buffer, engine, |_, caret| {
          caret.col -= unicode_width;
        });
      }
      CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_content::string_to_left_of_caret(buffer)?;
        mutate::change_editor_buffer(buffer, engine, |_, caret| {
          caret.col -= unicode_width;
        });
      }
    }
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Content get │
// ╯             ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod editor_ops_content {
  use super::*;

  pub fn line_display_width_at_caret(buffer: &EditorBuffer) -> ChUnit {
    let line = editor_ops_content::line_at_caret_to_string(buffer);
    if let Some(line) = line {
      line.display_width
    } else {
      ch!(0)
    }
  }

  pub fn line_display_width_at_row(buffer: &EditorBuffer, row_idx: ChUnit) -> ChUnit {
    let line = buffer.get_lines().get(ch!(@to_usize row_idx));
    if let Some(line) = line {
      line.display_width
    } else {
      ch!(0)
    }
  }

  pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let position = buffer.get_caret();
    let line = buffer.get_lines().get(ch!(@to_usize position.row))?;
    Some(line.clone())
  }

  pub fn next_line_below_caret_to_string(buffer: &EditorBuffer) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let position = buffer.get_caret();
    let line = buffer.get_lines().get(ch!(@to_usize position.row, @inc))?;
    Some(line.clone())
  }

  pub fn prev_line_above_caret_to_string(buffer: &EditorBuffer) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let position = buffer.get_caret();
    if position.row == ch!(0) {
      return None;
    }
    let line = buffer.get_lines().get(ch!(@to_usize position.row, @dec))?;
    Some(line.clone())
  }

  // TK: 🚨✅ Adjust caret for scroll_offset
  pub fn string_at_caret(
    buffer: &EditorBuffer,
    engine: &EditorRenderEngine,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let position = buffer.get_caret();

    // TK: 🚨🌈🏳️‍🌈 Add enum {Raw, Scroll} to get_caret() and move logic below to Scroll variant
    let position_row_adjusted_for_scroll_offset = position.row + engine.scroll_offset.row;
    let position_col_adjusted_for_scroll_offset = position.col + engine.scroll_offset.col;

    let line = buffer
      .get_lines()
      .get(ch!(@to_usize position_row_adjusted_for_scroll_offset))?;

    let result = line.get_string_at_display_col(position_col_adjusted_for_scroll_offset)?;

    Some(result)
  }

  pub fn string_to_left_of_caret(buffer: &EditorBuffer) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    match editor_ops_caret::find_col(buffer) {
      // Caret is at end of line, past the last character.
      CaretColLocation::AtEndOfLine => {
        let line = buffer
          .get_lines()
          .get(ch!(@to_usize buffer.get_caret().row))?;
        line.get_string_at_end()
      }
      // Caret is not at end of line.
      _ => {
        let line = buffer
          .get_lines()
          .get(ch!(@to_usize buffer.get_caret().row))?;
        line.get_string_at_left_of_display_col(buffer.get_caret().col)
      }
    }
  }

  pub fn string_at_end_of_line_at_caret(
    buffer: &EditorBuffer,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let line = editor_ops_content::line_at_caret_to_string(buffer)?;
    if let CaretColLocation::AtEndOfLine = editor_ops_caret::find_col(buffer) {
      let maybe_last_str_seg = line.get_string_at_end();
      return maybe_last_str_seg;
    }
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Content mut │
// ╯             ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
// TK: 📜 set engine { scroll_offset } to adjust the caret movement
pub mod editor_ops_content_mut {
  use super::*;

  pub fn insert_str_at_caret(
    buffer: &mut EditorBuffer,
    engine: &mut EditorRenderEngine,
    chunk: &str,
  ) {
    let row: usize = ch!(@to_usize buffer.get_caret().row);
    let col: usize = ch!(@to_usize buffer.get_caret().col);

    if buffer.get_lines().get(row).is_some() {
      insert_into_existing_line(buffer, engine, position!(col: col, row: row), chunk);
    } else {
      fill_in_missing_lines_up_to_row(buffer, engine, row);
      insert_into_new_line(buffer, engine, row, chunk);
    }
  }

  // TK: 🚨✅ handle user entering newlines & scrolling the doc
  pub fn insert_new_line_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) {
    if buffer.is_empty() {
      mutate::change_editor_buffer(buffer, engine, |lines, _| {
        lines.push(String::new().into());
      });
      return;
    }

    match editor_ops_caret::find_col(buffer) {
      CaretColLocation::AtEndOfLine => insert_new_line_at_end_of_current_line(buffer, engine),
      CaretColLocation::AtStartOfLine => insert_new_line_at_start_of_current_line(buffer, engine),
      CaretColLocation::InMiddleOfLine => insert_new_line_at_middle_of_current_line(buffer, engine),
    }

    // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ Inner functions │
    // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
    // Handle inserting a new line at the end of the current line.
    fn insert_new_line_at_end_of_current_line(
      buffer: &mut EditorBuffer,
      engine: &mut EditorRenderEngine,
    ) {
      // insert empty line at caret.
      mutate::change_editor_buffer(buffer, engine, |lines, caret| {
        caret.add_row(1);
        caret.reset_col();
        lines.insert(ch!(@to_usize caret.row), String::new().into());
      });
    }

    // Handle inserting a new line at the start of the current line.
    fn insert_new_line_at_start_of_current_line(
      buffer: &mut EditorBuffer,
      engine: &mut EditorRenderEngine,
    ) {
      let row_index = ch!(@to_usize buffer.get_caret().row);
      if row_index == 0 {
        mutate::change_editor_buffer(buffer, engine, |lines, _| {
          lines.insert(0, String::new().into());
        });
      } else {
        mutate::change_editor_buffer(buffer, engine, |lines, _| {
          lines.insert(row_index, String::new().into());
        });
      }
      mutate::change_editor_buffer(buffer, engine, |_, caret| {
        caret.add_row(1);
      });
    }

    // Handle inserting a new line at the middle of the current line.
    fn insert_new_line_at_middle_of_current_line(
      buffer: &mut EditorBuffer,
      engine: &mut EditorRenderEngine,
    ) {
      if let Some(line_content) = editor_ops_content::line_at_caret_to_string(buffer) {
        let split_result = line_content.split_at_display_col(buffer.get_caret().col);

        if let Some((
          NewUnicodeStringResult {
            new_unicode_string: left,
            ..
          },
          NewUnicodeStringResult {
            new_unicode_string: right,
            ..
          },
        )) = split_result
        {
          let row_index = ch!(@to_usize buffer.get_caret().row);
          mutate::change_editor_buffer(buffer, engine, |lines, caret| {
            let _ = replace(&mut lines[row_index], left);
            lines.insert(row_index + 1, right);
            caret.add_row(1);
            caret.reset_col();
          });
        }
      }
    }
  }

  pub fn delete_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    if editor_ops_content::string_at_caret(buffer, engine).is_some() {
      delete_in_middle_of_line(buffer, engine)?;
    } else {
      delete_at_end_of_line(buffer, engine)?;
    }
    return None;

    // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ Inner functions │
    // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

    // R ┌──────────┐
    // 0 ▸abc       │
    // 1 │ab        │
    // 2 │a         │
    //   └─▴────────┘
    //   C0123456789
    fn delete_in_middle_of_line(
      buffer: &mut EditorBuffer,
      engine: &mut EditorRenderEngine,
    ) -> Nope {
      let cur_line = editor_ops_content::line_at_caret_to_string(buffer)?;
      let NewUnicodeStringResult {
        new_unicode_string: new_line,
        ..
      } = cur_line.delete_char_at_display_col(buffer.get_caret().col)?;
      let row_index = ch!(@to_usize buffer.get_caret().row);
      mutate::change_editor_buffer(buffer, engine, |lines, _| {
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
    fn delete_at_end_of_line(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
      let this_line = editor_ops_content::line_at_caret_to_string(buffer)?;
      let next_line = editor_ops_content::next_line_below_caret_to_string(buffer)?;

      mutate::change_editor_buffer(buffer, engine, |lines, caret| {
        let _ = replace(&mut lines[ch!(@to_usize caret.row)], this_line + &next_line);
        lines.remove(ch!(@to_usize caret.row, @inc));
      });
      None
    }
  }

  pub fn backspace_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    if let Some(UnicodeStringSegmentSliceResult {
      display_col_at_which_seg_starts,
      ..
    }) = editor_ops_content::string_to_left_of_caret(buffer)
    {
      backspace_in_middle_of_line(buffer, engine, display_col_at_which_seg_starts)?;
    } else {
      backspace_at_start_of_line(buffer, engine)?;
    }

    return None;

    // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ Inner functions │
    // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

    // R ┌──────────┐
    // 0 ▸abc       │
    // 1 │ab        │
    // 2 │a         │
    //   └─▴────────┘
    //   C0123456789
    fn backspace_in_middle_of_line(
      buffer: &mut EditorBuffer,
      engine: &mut EditorRenderEngine,
      delete_at_this_display_idx: ChUnit,
    ) -> Nope {
      let cur_line = editor_ops_content::line_at_caret_to_string(buffer)?;
      let NewUnicodeStringResult {
        new_unicode_string: new_line,
        ..
      } = cur_line.delete_char_at_display_col(delete_at_this_display_idx)?;
      mutate::change_editor_buffer(buffer, engine, |lines, caret| {
        let _ = replace(&mut lines[ch!(@to_usize caret.row)], new_line);
        caret.set_col(delete_at_this_display_idx);
      });

      None
    }

    // R ┌──────────┐
    // 0 │abc       │
    // 1 ▸ab        │
    // 2 │a         │
    //   └▴─────────┘
    //   C0123456789
    fn backspace_at_start_of_line(
      buffer: &mut EditorBuffer,
      engine: &mut EditorRenderEngine,
    ) -> Nope {
      let this_line = editor_ops_content::line_at_caret_to_string(buffer)?;
      let prev_line = editor_ops_content::prev_line_above_caret_to_string(buffer)?;
      let prev_line_cols = prev_line.display_width;
      mutate::change_editor_buffer(buffer, engine, |lines, caret| {
        let _ = replace(
          &mut lines[ch!(@to_usize caret.row, @dec)],
          prev_line + &this_line,
        );
        lines.remove(ch!(@to_usize caret.row));
        caret.sub_row(1);
        caret.set_col(prev_line_cols);
      });

      None
    }
  }

  fn insert_into_existing_line(
    buffer: &mut EditorBuffer,
    engine: &mut EditorRenderEngine,
    pos: Position,
    chunk: &str,
  ) -> Nope {
    let row_index = ch!(@to_usize pos.row);
    let line = buffer.get_lines().get(row_index)?;

    let NewUnicodeStringResult {
      new_unicode_string: new_line,
      unicode_width: char_display_width,
    } = line.insert_char_at_display_col(ch!(pos.col), chunk)?;

    mutate::change_editor_buffer(buffer, engine, |lines, caret| {
      // Replace existing line w/ new line.
      let _ = replace(&mut lines[row_index], new_line);

      // Update caret position.
      caret.add_col(ch!(@to_usize char_display_width));
    });

    None
  }

  // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
  // │ Inner functions │
  // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

  fn fill_in_missing_lines_up_to_row(
    buffer: &mut EditorBuffer,
    engine: &mut EditorRenderEngine,
    caret_row: usize,
  ) {
    // Fill in any missing lines.
    if buffer.get_lines().get(caret_row).is_none() {
      for row_idx in 0..caret_row + 1 {
        if buffer.get_lines().get(row_idx).is_none() {
          mutate::change_editor_buffer(buffer, engine, |lines, _| {
            lines.push(String::new().into());
          });
        }
      }
    }
  }

  fn insert_into_new_line(
    buffer: &mut EditorBuffer,
    engine: &mut EditorRenderEngine,
    caret_row: usize,
    chunk: &str,
  ) -> Nope {
    // Actually add the character to the correct line.
    let _ = buffer.get_lines().get(caret_row)?;

    mutate::change_editor_buffer(buffer, engine, |lines, caret| {
      let _ = replace(
        &mut lines[ch!(@to_usize caret_row)],
        UnicodeString::from(chunk),
      );
      caret.add_col(UnicodeString::str_display_width(chunk));
    });

    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Internal module: mut EditorBuffer + EditorRenderEngine │
// ╯                                                        ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub(super) mod mutate {
  use super::*;

  /// Internal function.
  pub(super) fn change_editor_buffer(
    buffer: &mut EditorBuffer,
    engine: &mut EditorRenderEngine,
    mutator: impl FnOnce(
      /* EditorBuffer::lines */ &mut Vec<UnicodeString>,
      /* EditorBuffer::caret */ &mut Position,
    ),
  ) -> Nope {
    // Run the mutator first.
    buffer.apply_mut(mutator);

    // Check to see whether the caret is in the correct display column.
    validate_caret_col_position_not_in_middle_of_grapheme_cluster(buffer);

    // Check to see whether scrolling needs to be activated.
    validate_vertical_scroll(buffer, engine);

    // TK: 🚨🔮 validate horizontal scroll

    None
  }

  /// Validate that the caret is vertically within the viewport.
  ///
  /// If vertical scrolling should be activated, then the following are modified:
  /// 1. [caret](EditorBuffer::caret)'s row
  /// 2. [scroll_offset](EditorEngine::scroll_offset)).
  ///
  /// Otherwise, no changes are made.
  fn validate_vertical_scroll(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) {
    match editor_ops_caret::find_row(buffer) {
      // TK: 🚨✅ impl this
      CaretRowLocation::AtBottomOfBuffer => {
        let Size {
          row: max_display_row_count,
          ..
        } = engine.current_box.style_adjusted_bounds_size;

        let caret_row = buffer.get_caret().row;

        if caret_row > max_display_row_count {
          engine.scroll_offset.row += 1;
          buffer.apply_mut(|_, caret| {
            caret.sub_row(1);
          });
        }
      }
      CaretRowLocation::AtTopOfBuffer => {
        // TK: 🚨🌈 impl this
      }
      CaretRowLocation::InMiddleOfBuffer => {
        // TK: 🚨🌈 impl this
      }
    }
  }

  fn validate_caret_col_position_not_in_middle_of_grapheme_cluster(
    buffer: &mut EditorBuffer,
  ) -> Nope {
    let (lines, caret) = buffer.get_mut();
    let line = lines.get(ch!(@to_usize caret.row))?;
    let segment = line.is_display_col_in_middle_of_grapheme_cluster(caret.col)?;
    // Is in middle.
    caret.set_col(segment.unicode_width + segment.display_col_offset);
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Caret location enums │
// ╯                      ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod caret_enums {
  use super::*;

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
}
pub use caret_enums::*;

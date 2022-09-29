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
pub mod editor_ops_get_caret {
  use super::*;

  /// Locate the col.
  pub fn find_col(args: EditorArgs<'_>) -> CaretColLocation {
    let EditorArgs { buffer, engine } = args;

    if editor_ops_get_caret::col_is_at_start_of_line(buffer, engine) {
      CaretColLocation::AtStartOfLine
    } else if editor_ops_get_caret::col_is_at_end_of_line(buffer, engine) {
      CaretColLocation::AtEndOfLine
    } else {
      CaretColLocation::InMiddleOfLine
    }
  }

  fn col_is_at_start_of_line(buffer: &EditorBuffer, engine: &EditorRenderEngine) -> bool {
    if editor_ops_get_content::line_at_caret_to_string(buffer, engine).is_some() {
      *buffer.get_caret(CaretKind::ScrollAdjusted).col == 0
    } else {
      false
    }
  }

  fn col_is_at_end_of_line(buffer: &EditorBuffer, engine: &EditorRenderEngine) -> bool {
    if let Some(line) = editor_ops_get_content::line_at_caret_to_string(buffer, engine) {
      buffer.get_caret(CaretKind::ScrollAdjusted).col == line.display_width
    } else {
      false
    }
  }

  /// Locate the row.
  pub fn find_row(args: EditorArgs<'_>) -> CaretRowLocation {
    let EditorArgs { buffer, engine } = args;

    if row_is_at_top_of_buffer(buffer, engine) {
      CaretRowLocation::AtTopOfBuffer
    } else if row_is_at_bottom_of_buffer(buffer, engine) {
      CaretRowLocation::AtBottomOfBuffer
    } else {
      CaretRowLocation::InMiddleOfBuffer
    }
  }

  // R ┌──────────┐
  // 0 ▸          │
  //   └▴─────────┘
  //   C0123456789
  fn row_is_at_top_of_buffer(buffer: &EditorBuffer, engine: &EditorRenderEngine) -> bool {
    *buffer.get_caret(CaretKind::ScrollAdjusted).row == 0
  }

  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸a         │
  //   └▴─────────┘
  //   C0123456789
  fn row_is_at_bottom_of_buffer(buffer: &EditorBuffer, engine: &EditorRenderEngine) -> bool {
    if buffer.is_empty() || buffer.get_lines().len() == 1 {
      false // If there is only one line, then the caret is not at the bottom, its at the top.
    } else {
      let max_row_count = ch!(buffer.get_lines().len(), @dec);
      buffer.get_caret(CaretKind::ScrollAdjusted).row == max_row_count
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄╮
// │ Caret mut │
// ╯           ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod editor_ops_mut_caret {
  use super::*;
  use crate::scroll_buffer::{dec_col, inc_row};

  pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    // let EditorArgs { buffer, engine } = args;
    empty_check_early_return!(buffer, @None);
    match editor_ops_get_caret::find_row(EditorArgs { buffer, engine }) {
      CaretRowLocation::AtTopOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtBottomOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        mutate_buffer::apply_change_with_validations(buffer, engine, |_, caret, scroll_offset| {
          scroll_buffer::dec_row(caret, scroll_offset);
        });

        let line_display_width = editor_ops_get_content::line_display_width_at_row_index(
          buffer,
          buffer.get_caret(CaretKind::ScrollAdjusted).row,
        );

        mutate_buffer::apply_change_with_validations(buffer, engine, |_, caret, _| {
          caret.clip_col_to_bounds(line_display_width);
        });
      }
    }

    None
  }

  pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_get_caret::find_row(EditorArgs { buffer, engine }) {
      CaretRowLocation::AtBottomOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtTopOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        let max_row_idx = ch!(buffer.get_lines().len(), @dec);
        let cur_row_idx = buffer.get_caret(CaretKind::ScrollAdjusted).row;

        let is_there_a_line_below = cur_row_idx < max_row_idx;
        if is_there_a_line_below {
          let line_display_width =
            editor_ops_get_content::line_display_width_at_row_index(buffer, cur_row_idx + 1);
          mutate_buffer::apply_change_with_validations(buffer, engine, |_, caret, _| {
            inc_row(caret);
            caret.clip_col_to_bounds(line_display_width);
          });
        }
      }
    }

    None
  }

  // TK: 🚨🌈 handle horiz scrolling
  pub fn right(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_get_caret::find_col(EditorArgs { buffer, engine }) {
      CaretColLocation::AtEndOfLine => {
        // Do nothing.
      }
      CaretColLocation::AtStartOfLine | CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_get_content::string_at_caret(buffer, engine)?;
        let max_display_width = editor_ops_get_content::line_display_width_at_caret(buffer, engine);
        mutate_buffer::apply_change_with_validations(buffer, engine, |_, caret, _| {
          caret.add_col_with_bounds(unicode_width, max_display_width);
        });
      }
    }

    None
  }

  // TK: 🚨🌈 handle horiz scrolling
  pub fn left(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_get_caret::find_col(EditorArgs { buffer, engine }) {
      CaretColLocation::AtStartOfLine => {
        // Do nothing.
      }
      CaretColLocation::AtEndOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_get_content::string_at_end_of_line_at_caret(buffer, engine)?;
        mutate_buffer::apply_change_with_validations(buffer, engine, |_, caret, scroll_offset| {
          dec_col(caret, scroll_offset, unicode_width)
        });
      }
      CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_get_content::string_to_left_of_caret(buffer, engine)?;
        mutate_buffer::apply_change_with_validations(buffer, engine, |_, caret, scroll_offset| {
          dec_col(caret, scroll_offset, unicode_width)
        });
      }
    }
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Content get │
// ╯             ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod editor_ops_get_content {
  use super::*;

  pub fn line_display_width_at_caret(buffer: &EditorBuffer, engine: &EditorRenderEngine) -> ChUnit {
    let line = editor_ops_get_content::line_at_caret_to_string(buffer, engine);
    if let Some(line) = line {
      line.display_width
    } else {
      ch!(0)
    }
  }

  pub fn line_display_width_at_row_index(buffer: &EditorBuffer, row_idx: ChUnit) -> ChUnit {
    let line = buffer.get_lines().get(ch!(@to_usize row_idx));
    if let Some(line) = line {
      line.display_width
    } else {
      ch!(0)
    }
  }

  pub fn line_at_caret_to_string(
    buffer: &EditorBuffer,
    engine: &EditorRenderEngine,
  ) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row;
    let line = buffer.get_lines().get(ch!(@to_usize row_index))?;
    Some(line.clone())
  }

  pub fn next_line_below_caret_to_string(
    buffer: &EditorBuffer,
    engine: &EditorRenderEngine,
  ) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row;
    let line = buffer.get_lines().get(ch!(@to_usize row_index, @inc))?;
    Some(line.clone())
  }

  pub fn prev_line_above_caret_to_string(
    buffer: &EditorBuffer,
    engine: &EditorRenderEngine,
  ) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row;
    if row_index == ch!(0) {
      return None;
    }
    let line = buffer.get_lines().get(ch!(@to_usize row_index, @dec))?;
    Some(line.clone())
  }

  pub fn string_at_caret(
    buffer: &EditorBuffer,
    engine: &EditorRenderEngine,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let position = buffer.get_caret(CaretKind::ScrollAdjusted);
    let line = buffer.get_lines().get(ch!(@to_usize position.row))?;
    let result = line.get_string_at_display_col(position.col)?;
    Some(result)
  }

  pub fn string_to_left_of_caret(
    buffer: &EditorBuffer,
    engine: &EditorRenderEngine,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let position = buffer.get_caret(CaretKind::ScrollAdjusted);
    let line = buffer.get_lines().get(ch!(@to_usize position.row))?;
    match editor_ops_get_caret::find_col(EditorArgs { buffer, engine }) {
      // Caret is at end of line, past the last character.
      CaretColLocation::AtEndOfLine => line.get_string_at_end(),
      // Caret is not at end of line.
      _ => line.get_string_at_left_of_display_col(position.col),
    }
  }

  pub fn string_at_end_of_line_at_caret(
    buffer: &EditorBuffer,
    engine: &EditorRenderEngine,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let line = editor_ops_get_content::line_at_caret_to_string(buffer, engine)?;
    if let CaretColLocation::AtEndOfLine =
      editor_ops_get_caret::find_col(EditorArgs { buffer, engine })
    {
      let maybe_last_str_seg = line.get_string_at_end();
      return maybe_last_str_seg;
    }
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Content mut │
// ╯             ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod editor_ops_mut_content {
  use super::*;
  use crate::scroll_buffer::{dec_row, reset_col};

  pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
    let EditorArgsMut { buffer, engine } = args;

    let position = buffer.get_caret(CaretKind::ScrollAdjusted);

    let row: usize = ch!(@to_usize position.row);
    let col: usize = ch!(@to_usize position.col);

    if buffer.get_lines().get(row).is_some() {
      insert_into_existing_line(
        EditorArgsMut { buffer, engine },
        position!(col: col, row: row),
        chunk,
      );
    } else {
      fill_in_missing_lines_up_to_row(EditorArgsMut { buffer, engine }, row);
      insert_into_new_line(EditorArgsMut { buffer, engine }, row, chunk);
    }
  }

  pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    if buffer.is_empty() {
      mutate_buffer::apply_change_with_validations(buffer, engine, |lines, _, _| {
        lines.push(String::new().into());
      });
      return;
    }

    match editor_ops_get_caret::find_col(EditorArgs { buffer, engine }) {
      CaretColLocation::AtEndOfLine => {
        insert_new_line_at_end_of_current_line(EditorArgsMut { buffer, engine });
      }
      CaretColLocation::AtStartOfLine => {
        insert_new_line_at_start_of_current_line(EditorArgsMut { buffer, engine });
      }
      CaretColLocation::InMiddleOfLine => {
        insert_new_line_at_middle_of_current_line(EditorArgsMut { buffer, engine });
      }
    }

    // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ Inner functions │
    // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
    // Handle inserting a new line at the end of the current line.
    fn insert_new_line_at_end_of_current_line(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      // Insert empty line at caret.
      mutate_buffer::apply_change_with_validations(
        buffer,
        engine,
        |lines, caret, scroll_offset| {
          scroll_buffer::inc_row(caret);
          reset_col(caret, scroll_offset);
          lines.insert(ch!(@to_usize caret.row), String::new().into());
        },
      );
    }

    // Handle inserting a new line at the start of the current line.
    fn insert_new_line_at_start_of_current_line(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      let row_index = ch!(@to_usize buffer.get_caret(CaretKind::ScrollAdjusted).row);
      if row_index == 0 {
        mutate_buffer::apply_change_with_validations(buffer, engine, |lines, _, _| {
          lines.insert(0, String::new().into());
        });
      } else {
        mutate_buffer::apply_change_with_validations(buffer, engine, |lines, _, _| {
          lines.insert(row_index, String::new().into());
        });
      }

      mutate_buffer::apply_change_with_validations(buffer, engine, |_, caret, _| {
        scroll_buffer::inc_row(caret);
      });
    }

    // Handle inserting a new line at the middle of the current line.
    fn insert_new_line_at_middle_of_current_line(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      if let Some(line_content) = editor_ops_get_content::line_at_caret_to_string(buffer, engine) {
        let position = buffer.get_caret(CaretKind::ScrollAdjusted);

        let col_index = position.col;
        let split_result = line_content.split_at_display_col(col_index);
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
          let row_index = ch!(@to_usize position.row);
          mutate_buffer::apply_change_with_validations(
            buffer,
            engine,
            |lines, caret, scroll_offset| {
              let _ = replace(&mut lines[row_index], left);
              lines.insert(row_index + 1, right);
              scroll_buffer::inc_row(caret);
              reset_col(caret, scroll_offset);
            },
          );
        }
      }
    }
  }

  pub fn delete_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    if editor_ops_get_content::string_at_caret(buffer, engine).is_some() {
      delete_in_middle_of_line(buffer, engine)?;
    } else {
      delete_at_end_of_line(buffer, engine)?;
    }
    return None;

    // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ Inner functions │
    // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

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
      let cur_line = editor_ops_get_content::line_at_caret_to_string(buffer, engine)?;
      let NewUnicodeStringResult {
        new_unicode_string: new_line,
        ..
      } = cur_line.delete_char_at_display_col(buffer.get_caret(CaretKind::ScrollAdjusted).col)?;
      let row_index = ch!(@to_usize buffer.get_caret(CaretKind::ScrollAdjusted).row);
      mutate_buffer::apply_change_with_validations(buffer, engine, |lines, _, _| {
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
      let this_line = editor_ops_get_content::line_at_caret_to_string(buffer, engine)?;
      let next_line = editor_ops_get_content::next_line_below_caret_to_string(buffer, engine)?;

      mutate_buffer::apply_change_with_validations(buffer, engine, |lines, caret, _| {
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
    }) = editor_ops_get_content::string_to_left_of_caret(buffer, engine)
    {
      backspace_in_middle_of_line(buffer, engine, display_col_at_which_seg_starts)?;
    } else {
      backspace_at_start_of_line(buffer, engine)?;
    }

    return None;

    // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ Inner functions │
    // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

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
      let cur_line = editor_ops_get_content::line_at_caret_to_string(buffer, engine)?;
      let NewUnicodeStringResult {
        new_unicode_string: new_line,
        ..
      } = cur_line.delete_char_at_display_col(delete_at_this_display_idx)?;
      mutate_buffer::apply_change_with_validations(buffer, engine, |lines, caret, _| {
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
      let this_line = editor_ops_get_content::line_at_caret_to_string(buffer, engine)?;
      let prev_line = editor_ops_get_content::prev_line_above_caret_to_string(buffer, engine)?;
      let prev_line_cols = prev_line.display_width;
      mutate_buffer::apply_change_with_validations(
        buffer,
        engine,
        |lines, caret, scroll_offset| {
          let _ = replace(
            &mut lines[ch!(@to_usize caret.row, @dec)],
            prev_line + &this_line,
          );
          lines.remove(ch!(@to_usize caret.row));
          dec_row(caret, scroll_offset);
          caret.set_col(prev_line_cols);
        },
      );

      None
    }
  }

  fn insert_into_existing_line(args: EditorArgsMut<'_>, pos: Position, chunk: &str) -> Nope {
    let EditorArgsMut { buffer, engine } = args;

    let row_index = ch!(@to_usize pos.row);
    let line = buffer.get_lines().get(row_index)?;

    let NewUnicodeStringResult {
      new_unicode_string: new_line,
      unicode_width: char_display_width,
    } = line.insert_char_at_display_col(ch!(pos.col), chunk)?;

    mutate_buffer::apply_change_with_validations(buffer, engine, |lines, caret, _| {
      // Replace existing line w/ new line.
      let _ = replace(&mut lines[row_index], new_line);

      // Update caret position.
      caret.add_col(ch!(@to_usize char_display_width));
    });

    None
  }

  // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
  // │ Inner functions │
  // ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

  fn fill_in_missing_lines_up_to_row(args: EditorArgsMut<'_>, caret_row: usize) {
    let EditorArgsMut { buffer, engine } = args;

    // Fill in any missing lines.
    if buffer.get_lines().get(caret_row).is_none() {
      for row_idx in 0..caret_row + 1 {
        if buffer.get_lines().get(row_idx).is_none() {
          mutate_buffer::apply_change_with_validations(buffer, engine, |lines, _, _| {
            lines.push(String::new().into());
          });
        }
      }
    }
  }

  fn insert_into_new_line(args: EditorArgsMut<'_>, caret_row: usize, chunk: &str) -> Nope {
    let EditorArgsMut { buffer, engine } = args;

    // Actually add the character to the correct line.
    let _ = buffer.get_lines().get(caret_row)?;

    mutate_buffer::apply_change_with_validations(buffer, engine, |lines, caret, _| {
      let _ = replace(
        &mut lines[ch!(@to_usize caret_row)],
        UnicodeString::from(chunk),
      );
      caret.add_col(UnicodeString::str_display_width(chunk));
    });

    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Change EditorBuffer │
// ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod mutate_buffer {
  use super::*;

  /// In addition to mutating the buffer, this function also runs validation functions to ensure
  /// that:
  /// 1. the caret is in not in the middle of a unicode segment character.
  /// 2. the caret is not out of bounds vertically or horizontally and activates scrolling if needed.
  pub fn apply_change_with_validations(
    buffer: &mut EditorBuffer,
    engine: &mut EditorRenderEngine,
    mutator: impl FnOnce(
      /* EditorBuffer::lines */ &mut Vec<UnicodeString>,
      /* EditorBuffer::caret */ &mut Position,
      /* EditorRenderEngine::scroll_offset */ &mut ScrollOffset,
    ),
  ) -> Nope {
    // Run the mutator first.
    let (lines, caret, scroll_offset) = buffer.get_mut();
    mutator(lines, caret, scroll_offset);

    // Check to see whether scroll is valid.
    scroll_buffer::validate_caret_in_viewport_activate_scroll_if_needed(EditorArgsMut {
      buffer,
      engine,
    });

    // Check to see whether the caret is in the correct display column.
    validate_caret_col_position_not_in_middle_of_grapheme_cluster(buffer);

    None
  }

  /// This function is visible inside the editor_ops.rs module only. It is not meant to be called
  /// directly, but instead is called by [mutate_buffer::apply_change_with_validations].
  fn validate_caret_col_position_not_in_middle_of_grapheme_cluster(
    buffer: &mut EditorBuffer,
  ) -> Nope {
    let (lines, caret, _) = buffer.get_mut();
    let line = lines.get(ch!(@to_usize caret.row))?;
    let segment = line.is_display_col_in_middle_of_grapheme_cluster(caret.col)?;
    // Is in middle.
    caret.set_col(segment.unicode_width + segment.display_col_offset);
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Scroll EditorBuffer │
// ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod scroll_buffer {
  use super::*;

  /// This is meant to be called inside [mutate_buffer::apply_change_with_validations].
  pub fn inc_col() {
    // TK: 🚨🌈 impl this
  }

  /// This does not simply decrement the caret.col but mutates scroll_offset if scrolling is active.
  ///
  /// This is meant to be called inside [mutate_buffer::apply_change_with_validations].
  pub fn dec_col(caret: &mut Position, scroll_offset: &mut ScrollOffset, col_amount: ChUnit) {
    let horiz_scroll_is_active = scroll_offset.col > ch!(0);
    let not_at_start_of_line = caret.col > ch!(0);

    match horiz_scroll_is_active {
      false => {
        caret.col -= col_amount; // Scroll is inactive.
      }
      true => {
        if not_at_start_of_line {
          caret.col -= col_amount; // Scroll active & Not at start of line.
        } else {
          scroll_offset.col -= col_amount; // Scroll active & At start of line.
        }
      }
    }
  }

  /// This is meant to be called inside [mutate_buffer::apply_change_with_validations].
  pub fn reset_col(caret: &mut Position, scroll_offset: &mut ScrollOffset) {
    scroll_offset.col = ch!(0);
    caret.col = ch!(0);
  }

  /// This does not simply decrement the caret.row but mutates scroll_offset if scrolling is active.
  ///
  /// This is meant to be called inside [mutate_buffer::apply_change_with_validations].
  pub fn dec_row(caret: &mut Position, scroll_offset: &mut ScrollOffset) {
    let vert_scroll_is_active = scroll_offset.row > ch!(0);
    let not_at_top_of_buffer = caret.row > ch!(0);

    match vert_scroll_is_active {
      false => {
        caret.row -= 1; // Scroll is inactive.
      }
      true => {
        if not_at_top_of_buffer {
          caret.row -= 1; // Scroll active & Not at top of buffer.
        } else {
          scroll_offset.row -= 1; // Scroll active & At top of buffer.
        }
      }
    }
  }

  /// This is just a marker function that only increments the caret.row.
  ///
  /// This is meant to be called inside [mutate_buffer::apply_change_with_validations], which will
  /// then call [validate_caret_in_viewport_activate_scroll_if_needed] after this function is
  /// called.
  pub fn inc_row(caret: &mut Position) { caret.row += 1; }

  /// Check whether caret is vertically within the viewport.
  ///
  /// - If it isn't then scroll by mutating:
  ///    1. [caret](EditorBuffer::get_caret())'s row , so it is within the viewport.
  ///    2. [scroll_offset](EditorBuffer::get_scroll_offset())'s row, to actually apply scrolling.
  /// - Otherwise, no changes are made.
  ///
  /// This function is not meant to be called directly, but instead is called by
  /// [mutate_buffer::apply_change_with_validations].
  pub fn validate_caret_in_viewport_activate_scroll_if_needed(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    // Handle vertical scrolling (past bottom of buffer).
    if let CaretRowLocation::AtBottomOfBuffer =
      editor_ops_get_caret::find_row(EditorArgs { buffer, engine })
    {
      let max_display_row_count = engine.current_box.style_adjusted_bounds_size.rows;
      let caret_row = buffer.get_caret(CaretKind::Raw).row;
      if caret_row > max_display_row_count {
        let row_diff = caret_row - max_display_row_count;
        let (_, caret, scroll_offset) = buffer.get_mut();
        scroll_offset.row += row_diff;
        caret.row -= row_diff;
      }
    }

    // Handle horizontal scrolling (past end of line).
    if let CaretColLocation::AtEndOfLine =
      editor_ops_get_caret::find_col(EditorArgs { buffer, engine })
    {
      let max_display_col_count = engine.current_box.style_adjusted_bounds_size.cols;
      let caret_col = buffer.get_caret(CaretKind::Raw).col;
      if caret_col > max_display_col_count {
        let col_diff = caret_col - max_display_col_count;
        let (_, caret, scroll_offset) = buffer.get_mut();
        scroll_offset.col += col_diff;
        caret.col -= col_diff;
        // There is a chance that the caret.col is in the middle of a grapheme cluster at the end of
        // this block. However, this function is always called before
        // validate_caret_col_position_not_in_middle_of_grapheme_cluster() which will fix this.
      }
    }
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

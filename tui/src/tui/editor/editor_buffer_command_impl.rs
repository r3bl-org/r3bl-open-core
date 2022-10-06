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

use std::{cmp::Ordering, mem::replace};

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::{scroll::*, *};

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
pub mod locate_caret {
  use super::*;

  /// Locate the col.
  pub fn find_col(args: EditorArgs<'_>) -> CaretColLocation {
    let EditorArgs { buffer, engine } = args;

    if locate_caret::col_is_at_start_of_line(buffer, engine) {
      CaretColLocation::AtStartOfLine
    } else if locate_caret::col_is_at_end_of_line(buffer, engine) {
      CaretColLocation::AtEndOfLine
    } else {
      CaretColLocation::InMiddleOfLine
    }
  }

  fn col_is_at_start_of_line(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
    if get_content::line_at_caret_to_string(buffer, engine).is_some() {
      *buffer.get_caret(CaretKind::ScrollAdjusted).col == 0
    } else {
      false
    }
  }

  fn col_is_at_end_of_line(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
    if let Some(line) = get_content::line_at_caret_to_string(buffer, engine) {
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
  fn row_is_at_top_of_buffer(buffer: &EditorBuffer, _engine: &EditorEngine) -> bool {
    *buffer.get_caret(CaretKind::ScrollAdjusted).row == 0
  }

  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸a         │
  //   └▴─────────┘
  //   C0123456789
  fn row_is_at_bottom_of_buffer(buffer: &EditorBuffer, _engine: &EditorEngine) -> bool {
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
pub mod move_caret {
  use super::*;

  pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    match locate_caret::find_row(EditorArgs { buffer, engine }) {
      CaretRowLocation::AtTopOfBuffer => {
        // Do nothing.
        if buffer.get_caret(CaretKind::ScrollAdjusted).col != ch!(0) {
          validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
            reset_caret_col(caret, scroll_offset);
          });
        }
      }
      CaretRowLocation::AtBottomOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        // There is a line above the caret.
        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
          scroll::dec_caret_row(caret, scroll_offset);
        });
        clip_caret_to_content_width(EditorArgsMut { buffer, engine });
      }
    }

    None
  }

  pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    if get_content::next_line_below_caret_exists(buffer, engine) {
      // There is a line below the caret.
      let viewport_height = engine.viewport_height();
      validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
        inc_caret_row(caret, scroll_offset, viewport_height);
      });
      clip_caret_to_content_width(EditorArgsMut { buffer, engine });
    } else {
      // Move to the end of the line.
      move_caret::to_end_of_line(buffer, engine);
    }

    None
  }

  /// Convenience function for simply calling [reset_caret_col].
  pub fn to_start_of_line(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
      reset_caret_col(caret, scroll_offset);
    });
    None
  }

  pub fn to_end_of_line(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    let line_content_display_width = get_content::line_display_width_at_row_index(
      buffer,
      buffer.get_caret(CaretKind::ScrollAdjusted).row,
    );
    let viewport_width = engine.viewport_width();
    validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
      set_caret_col(
        caret,
        scroll_offset,
        viewport_width,
        line_content_display_width,
        line_content_display_width,
      );
    });

    None
  }

  pub fn right(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    match locate_caret::find_col(EditorArgs { buffer, engine }) {
      CaretColLocation::AtEndOfLine => {
        if get_content::next_line_below_caret_exists(buffer, engine) {
          // If there is a line below the caret, move the caret to the start of the next line.
          let viewport_height = engine.viewport_height();
          validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
            inc_caret_row(caret, scroll_offset, viewport_height);
            reset_caret_col(caret, scroll_offset);
          });
        }
      }
      CaretColLocation::AtStartOfLine | CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          get_content::string_at_caret(buffer, engine)?;
        let max_display_width = get_content::line_display_width_at_caret(buffer, engine);
        let viewport_width = engine.viewport_width();
        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
          inc_caret_col(
            caret,
            scroll_offset,
            unicode_width,
            max_display_width,
            viewport_width,
          );
        });
      }
    }

    None
  }

  pub fn left(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    match locate_caret::find_col(EditorArgs { buffer, engine }) {
      CaretColLocation::AtStartOfLine => {
        if get_content::prev_line_above_caret_to_string(buffer, engine).is_some() {
          // If there is a line above the caret, move the caret to the end of the previous line.
          validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
            dec_caret_row(caret, scroll_offset);
          });
          move_caret::to_end_of_line(buffer, engine);
        }
      }
      CaretColLocation::AtEndOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          get_content::string_at_end_of_line_at_caret(buffer, engine)?;
        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
          dec_caret_col(caret, scroll_offset, unicode_width)
        });
      }
      CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          get_content::string_to_left_of_caret(buffer, engine)?;
        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
          dec_caret_col(caret, scroll_offset, unicode_width)
        });
      }
    }
    None
  }

  pub fn page_up(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    let viewport_height = engine.viewport_height();
    change_caret_row_by(
      EditorArgsMut { engine, buffer },
      viewport_height,
      CaretDirection::Up,
    );
    None
  }

  pub fn page_down(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    let viewport_height = engine.viewport_height();
    change_caret_row_by(
      EditorArgsMut { engine, buffer },
      viewport_height,
      CaretDirection::Down,
    );
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Content get │
// ╯             ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod get_content {
  use super::*;

  pub fn line_display_width_at_caret(buffer: &EditorBuffer, engine: &EditorEngine) -> ChUnit {
    let line = get_content::line_at_caret_to_string(buffer, engine);
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

  pub fn next_line_below_caret_exists(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
    let next_line = get_content::next_line_below_caret_to_string(buffer, engine);
    next_line.is_some()
  }

  pub fn line_at_caret_to_string(
    buffer: &EditorBuffer,
    _engine: &EditorEngine,
  ) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row;
    let line = buffer.get_lines().get(ch!(@to_usize row_index))?;
    Some(line.clone())
  }

  pub fn next_line_below_caret_to_string(
    buffer: &EditorBuffer,
    _engine: &EditorEngine,
  ) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row;
    let line = buffer.get_lines().get(ch!(@to_usize row_index + 1))?;
    Some(line.clone())
  }

  pub fn prev_line_above_caret_exists(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
    let prev_line = get_content::prev_line_above_caret_to_string(buffer, engine);
    prev_line.is_some()
  }

  pub fn prev_line_above_caret_to_string(
    buffer: &EditorBuffer,
    _engine: &EditorEngine,
  ) -> Option<UnicodeString> {
    empty_check_early_return!(buffer, @None);
    let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row;
    if row_index == ch!(0) {
      return None;
    }
    let line = buffer.get_lines().get(ch!(@to_usize row_index - 1))?;
    Some(line.clone())
  }

  pub fn string_at_caret(
    buffer: &EditorBuffer,
    _engine: &EditorEngine,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);
    let line = buffer.get_lines().get(ch!(@to_usize caret_adj.row))?;
    let result = line.get_string_at_display_col(caret_adj.col)?;
    Some(result)
  }

  pub fn string_to_left_of_caret(
    buffer: &EditorBuffer,
    engine: &EditorEngine,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);
    let line = buffer.get_lines().get(ch!(@to_usize caret_adj.row))?;
    match locate_caret::find_col(EditorArgs { buffer, engine }) {
      // Caret is at end of line, past the last character.
      CaretColLocation::AtEndOfLine => line.get_string_at_end(),
      // Caret is not at end of line.
      _ => line.get_string_at_left_of_display_col(caret_adj.col),
    }
  }

  pub fn string_at_end_of_line_at_caret(
    buffer: &EditorBuffer,
    engine: &EditorEngine,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    empty_check_early_return!(buffer, @None);
    let line = get_content::line_at_caret_to_string(buffer, engine)?;
    if let CaretColLocation::AtEndOfLine = locate_caret::find_col(EditorArgs { buffer, engine }) {
      let maybe_last_str_seg = line.get_string_at_end();
      return maybe_last_str_seg;
    }
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Content mut │
// ╯             ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod mut_content {
  use super::*;

  pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
    let EditorArgsMut { buffer, engine } = args;

    let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);

    let row: usize = ch!(@to_usize caret_adj.row);
    let col: usize = ch!(@to_usize caret_adj.col);

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
      validate::apply_change(buffer, engine, |lines, _, _| {
        lines.push(String::new().into());
      });
      return;
    }

    match locate_caret::find_col(EditorArgs { buffer, engine }) {
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

      let viewport_height = engine.viewport_height();

      validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
        let new_row_idx = scroll::inc_caret_row(caret, scroll_offset, viewport_height);
        reset_caret_col(caret, scroll_offset);
        lines.insert(new_row_idx, String::new().into());
      });
    }

    // Handle inserting a new line at the start of the current line.
    fn insert_new_line_at_start_of_current_line(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      let viewport_height = engine.viewport_height();

      validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
        let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
        lines.insert(cur_row_idx, String::new().into());
      });

      validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
        scroll::inc_caret_row(caret, scroll_offset, viewport_height);
      });
    }

    // Handle inserting a new line at the middle of the current line.
    fn insert_new_line_at_middle_of_current_line(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      if let Some(line_content) = get_content::line_at_caret_to_string(buffer, engine) {
        let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);

        let col_index = caret_adj.col;
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
          let row_index = ch!(@to_usize caret_adj.row);
          let viewport_height = engine.viewport_height();

          validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
            let _ = replace(&mut lines[row_index], left);
            lines.insert(row_index + 1, right);
            scroll::inc_caret_row(caret, scroll_offset, viewport_height);
            reset_caret_col(caret, scroll_offset);
          });
        }
      }
    }
  }

  pub fn delete_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    if get_content::string_at_caret(buffer, engine).is_some() {
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
    fn delete_in_middle_of_line(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
      let cur_line = get_content::line_at_caret_to_string(buffer, engine)?;
      let NewUnicodeStringResult {
        new_unicode_string: new_line,
        ..
      } = cur_line.delete_char_at_display_col(buffer.get_caret(CaretKind::ScrollAdjusted).col)?;

      validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
        let row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
        let _ = replace(&mut lines[row_idx], new_line);
      });
      None
    }

    // R ┌──────────┐
    // 0 ▸abc       │
    // 1 │ab        │
    // 2 │a         │
    //   └───▴──────┘
    //   C0123456789
    fn delete_at_end_of_line(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
      let this_line = get_content::line_at_caret_to_string(buffer, engine)?;
      let next_line = get_content::next_line_below_caret_to_string(buffer, engine)?;

      validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
        let row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
        let _ = replace(&mut lines[row_idx], this_line + &next_line);
        lines.remove(row_idx + 1);
      });
      None
    }
  }

  pub fn backspace_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
    empty_check_early_return!(buffer, @None);

    if let Some(UnicodeStringSegmentSliceResult {
      display_col_at_which_seg_starts,
      ..
    }) = get_content::string_to_left_of_caret(buffer, engine)
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
      engine: &mut EditorEngine,
      delete_at_this_display_col: ChUnit,
    ) -> Nope {
      let cur_line = get_content::line_at_caret_to_string(buffer, engine)?;
      let NewUnicodeStringResult {
        new_unicode_string: new_line,
        ..
      } = cur_line.delete_char_at_display_col(delete_at_this_display_col)?;

      let viewport_width = engine.viewport_width();
      validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
        let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
        let _ = replace(&mut lines[cur_row_idx], new_line);
        set_caret_col(
          caret,
          scroll_offset,
          viewport_width,
          lines[cur_row_idx].display_width,
          delete_at_this_display_col,
        );
      });

      None
    }

    // R ┌──────────┐
    // 0 │abc       │
    // 1 ▸ab        │
    // 2 │a         │
    //   └▴─────────┘
    //   C0123456789
    fn backspace_at_start_of_line(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Nope {
      let viewport_width = engine.viewport_width();

      let this_line = get_content::line_at_caret_to_string(buffer, engine)?;
      let prev_line = get_content::prev_line_above_caret_to_string(buffer, engine)?;

      let prev_line_eol_col = prev_line.display_width;
      validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
        let prev_row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset) - 1;
        let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
        let _ = replace(&mut lines[prev_row_idx], prev_line + &this_line);
        let new_merged_line_content_display_width = lines[prev_row_idx].display_width;
        lines.remove(cur_row_idx);
        dec_caret_row(caret, scroll_offset);
        set_caret_col(
          caret,
          scroll_offset,
          viewport_width,
          new_merged_line_content_display_width,
          prev_line_eol_col,
        );
      });

      None
    }
  }

  fn insert_into_existing_line(args: EditorArgsMut<'_>, caret_adj: Position, chunk: &str) -> Nope {
    let EditorArgsMut { buffer, engine } = args;

    let row_index = ch!(@to_usize caret_adj.row);
    let line = buffer.get_lines().get(row_index)?;

    let NewUnicodeStringResult {
      new_unicode_string: new_line,
      unicode_width: char_display_width,
    } = line.insert_char_at_display_col(ch!(caret_adj.col), chunk)?;

    let viewport_width = engine.viewport_width();

    validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
      // Replace existing line w/ new line.
      let _ = replace(&mut lines[row_index], new_line);
      let line_content_display_width = lines[row_index].display_width;

      // Update caret position.
      inc_caret_col(
        caret,
        scroll_offset,
        char_display_width,
        line_content_display_width,
        viewport_width,
      );
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
          validate::apply_change(buffer, engine, |lines, _, _| {
            lines.push(String::new().into());
          });
        }
      }
    }
  }

  fn insert_into_new_line(args: EditorArgsMut<'_>, caret_adj_row: usize, chunk: &str) -> Nope {
    let EditorArgsMut { buffer, engine } = args;

    // Make sure there's a line at caret_adj_row.
    let _ = buffer.get_lines().get(caret_adj_row)?;

    let viewport_width = engine.viewport_width();

    validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
      // Actually add the character to the correct line.
      let _ = replace(
        &mut lines[ch!(@to_usize caret_adj_row)],
        UnicodeString::from(chunk),
      );

      let line_content_display_width = lines[caret_adj_row].display_width;
      let col_amt = ch!(UnicodeString::str_display_width(chunk));

      // Update caret position.
      inc_caret_col(
        caret,
        scroll_offset,
        col_amt,
        line_content_display_width,
        viewport_width,
      );
    });

    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Change EditorBuffer │
// ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod validate {
  use super::*;

  /// In addition to mutating the buffer, this function also runs validation functions to ensure
  /// that:
  /// 1. the caret is in not in the middle of a unicode segment character.
  /// 2. the caret is not out of bounds vertically or horizontally and activates scrolling if needed.
  pub fn apply_change(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    mutator: impl FnOnce(
      /* EditorBuffer::lines */ &mut Vec<UnicodeString>,
      /* EditorBuffer::caret */ &mut Position,
      /* EditorEngine::scroll_offset */ &mut ScrollOffset,
    ),
  ) -> Nope {
    // Run the mutator first.
    let (lines, caret, scroll_offset) = buffer.get_mut();
    mutator(lines, caret, scroll_offset);

    // Check to see whether the caret is in the correct display column.
    validate_caret_col_position_not_in_middle_of_grapheme_cluster(EditorArgsMut { engine, buffer });

    None
  }

  /// This function is visible inside the editor_ops.rs module only. It is not meant to be called
  /// directly, but instead is called by [validate::apply_change].
  pub fn validate_caret_col_position_not_in_middle_of_grapheme_cluster(
    args: EditorArgsMut<'_>,
  ) -> Nope {
    let EditorArgsMut { buffer, engine } = args;

    let viewport_width = engine.viewport_width();

    let (lines, caret, scroll_offset) = buffer.get_mut();
    let row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
    let col_idx = ch!(EditorBuffer::calc_scroll_adj_caret_col(
      caret,
      scroll_offset
    ));
    let line = lines.get(row_idx)?;
    let segment = line.is_display_col_in_middle_of_grapheme_cluster(col_idx)?;

    // Is in middle.
    scroll::set_caret_col(
      caret,
      scroll_offset,
      viewport_width,
      line.display_width,
      segment.unicode_width + segment.display_col_offset,
    );
    None
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Scroll EditorBuffer │
// ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod scroll {
  use super::*;

  /// Try and leave the caret where it is, however, if the caret is out of the viewport, then
  /// scroll. This is meant to be called inside [validate::apply_change].
  pub fn clip_caret_to_content_width(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    let caret = buffer.get_caret(CaretKind::ScrollAdjusted);
    let scroll_offset = buffer.get_scroll_offset();
    let line_content_display_width = get_content::line_display_width_at_caret(buffer, engine);

    let caret_adj_col = ch!(EditorBuffer::calc_scroll_adj_caret_col(
      &caret,
      &scroll_offset
    ));

    let is_caret_col_overflow_content_width = caret_adj_col >= line_content_display_width;

    if is_caret_col_overflow_content_width {
      move_caret::to_end_of_line(buffer, engine);
    }
  }

  /// This is meant to be called inside [validate::apply_change] or
  /// [validate::validate_caret_col_position_not_in_middle_of_grapheme_cluster].
  pub fn set_caret_col(
    caret: &mut Position,
    scroll_offset: &mut ScrollOffset,
    viewport_width: ChUnit,
    line_content_display_width: ChUnit,
    desired_col: ChUnit,
  ) {
    let caret_adj_col = ch!(EditorBuffer::calc_scroll_adj_caret_col(
      caret,
      scroll_offset
    ));

    match caret_adj_col.cmp(&desired_col) {
      Ordering::Less => {
        // Move caret right.
        let diff = desired_col - caret_adj_col;
        inc_caret_col(
          caret,
          scroll_offset,
          diff,
          line_content_display_width,
          viewport_width,
        );
      }
      Ordering::Greater => {
        // Move caret left.
        let diff = caret_adj_col - desired_col;
        dec_caret_col(caret, scroll_offset, diff);
      }
      Ordering::Equal => {
        // Do nothing.
      }
    }
  }

  /// This is meant to be called inside [validate::apply_change].
  pub fn inc_caret_col(
    caret: &mut Position,
    scroll_offset: &mut ScrollOffset,
    col_amt: ChUnit,
    line_content_display_width: ChUnit,
    viewport_width: ChUnit,
  ) {
    // Just move the caret right.
    caret.add_col_with_bounds(col_amt, line_content_display_width);

    // Check to see if viewport needs to be scrolled.
    let is_caret_col_overflow_viewport_width = caret.col >= viewport_width;

    if is_caret_col_overflow_viewport_width {
      let diff_overflow = ch!(1) + caret.col - viewport_width;
      scroll_offset.col += diff_overflow; // Activate horiz scroll.
      caret.col -= diff_overflow; // Shift caret.
    }
  }

  /// This does not simply decrement the caret.col but mutates scroll_offset if scrolling is active.
  ///
  /// This is meant to be called inside [validate::apply_change].
  pub fn dec_caret_col(caret: &mut Position, scroll_offset: &mut ScrollOffset, col_amt: ChUnit) {
    let horiz_scroll_is_active = scroll_offset.col > ch!(0);
    let not_at_start_of_viewport = caret.col > ch!(0);

    match horiz_scroll_is_active {
      // HORIZONTAL SCROLL INACTIVE
      false => {
        caret.col -= col_amt; // Scroll inactive.
      }
      true => {
        // HORIZONTAL SCROLL ACTIVE
        if not_at_start_of_viewport {
          let need_to_scroll_left = col_amt > caret.col;
          match need_to_scroll_left {
            false => {
              caret.col -= col_amt; // Just move caret left by col_amt.
            }
            true => {
              let diff = col_amt - caret.col;
              caret.col -= col_amt; // Move caret left by col_amt.
              scroll_offset.col -= diff; // Move scroll left by diff.
            }
          }
        } else {
          scroll_offset.col -= col_amt; // Scroll active & At start of viewport.
                                        // Safe to sub, since scroll_offset.col can never be negative.
        }
      }
    }
  }

  /// This is meant to be called inside [validate::apply_change].
  pub fn reset_caret_col(caret: &mut Position, scroll_offset: &mut ScrollOffset) {
    scroll_offset.col = ch!(0);
    caret.col = ch!(0);
  }

  /// Decrement caret.row by 1, and adjust scrolling if active. This won't check whether it is
  /// inside or outside the buffer content boundary. You should check that before calling this
  /// function.
  ///
  /// This does not simply decrement the caret.row but mutates scroll_offset if scrolling is active.
  /// This can end up deactivating vertical scrolling as well.
  ///
  /// > Since caret.row can never be negative, this function must handle changes to scroll_offset
  /// > itself, and can't rely on [validate::apply_change] scroll validations
  /// > [scroll::validate_scroll].
  ///
  /// This is meant to be called inside [validate::apply_change].
  pub fn dec_caret_row(caret: &mut Position, scroll_offset: &mut ScrollOffset) -> usize {
    let vert_scroll_is_active = scroll_offset.row > ch!(0);
    let not_at_top_of_viewport = caret.row > ch!(0);

    match vert_scroll_is_active {
      // VERTICAL SCROLL INACTIVE
      false => {
        caret.row -= 1; // Scroll inactive.
                        // Safe to minus 1, since caret.row can never be negative.
      }
      // VERTICAL SCROLL ACTIVE
      true => {
        if not_at_top_of_viewport {
          caret.row -= 1; // Scroll active & Not at top of viewport.
        } else {
          scroll_offset.row -= 1; // Scroll active & At top of viewport.
                                  // Safe to minus 1, since scroll_offset.row can never be negative.
        }
      }
    }

    EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset)
  }

  /// Try to increment caret.row by row_amt. This will not scroll past the bottom of the buffer. It
  /// will also activate scrolling if needed.
  ///
  /// ```text
  /// +---------------------+
  /// 0                     |
  /// |        above        | <- caret_row_adj
  /// |                     |
  /// +--- scroll_offset ---+
  /// |         ↑           |
  /// |                     |
  /// |      within vp      |
  /// |                     |
  /// |         ↓           |
  /// +--- scroll_offset ---+
  /// |    + vp height      |
  /// |                     |
  /// |        below        | <- caret_row_adj
  /// |                     |
  /// +---------------------+
  /// ```
  pub fn change_caret_row_by(args: EditorArgsMut<'_>, row_amt: ChUnit, direction: CaretDirection) {
    let EditorArgsMut { buffer, engine } = args;

    match direction {
      CaretDirection::Down => {
        let viewport_height = engine.viewport_height();
        let current_caret_adj_row = buffer.get_caret(CaretKind::ScrollAdjusted).row;
        let mut desired_caret_adj_row = current_caret_adj_row + row_amt;
        scroll::clip_caret_row_to_content_height(buffer, &mut desired_caret_adj_row);

        // Calculate how many rows we need to increment caret row by.
        let mut diff = desired_caret_adj_row - current_caret_adj_row;

        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
          while diff > ch!(0) {
            scroll::inc_caret_row(caret, scroll_offset, viewport_height);
            diff -= 1;
          }
        });
      }
      CaretDirection::Up => {
        let mut diff = row_amt;
        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
          while diff > ch!(0) {
            scroll::dec_caret_row(caret, scroll_offset);
            diff -= 1;
            if EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset) == 0 {
              break;
            }
          }
        });
      }
      _ => {}
    }
  }

  /// Clip desired_caret_adj_row (to the max buffer length) if it overflows past the bottom of the
  /// buffer.
  pub fn clip_caret_row_to_content_height(
    buffer: &mut EditorBuffer,
    desired_caret_adj_row: &mut ChUnit,
  ) {
    // Clip desired_caret_adj_row if it overflows past the bottom of the buffer.
    let max_row_count = ch!(buffer.get_lines().len(), @dec);
    let is_past_end_of_buffer = *desired_caret_adj_row > max_row_count;
    if is_past_end_of_buffer {
      *desired_caret_adj_row = max_row_count;
    }
  }

  /// Increment caret.row by 1, and adjust scrolling if active. This won't check whether it is
  /// inside or outside the buffer content boundary. You should check that before calling this
  /// function.
  ///
  /// Returns the new scroll adjusted caret row.
  ///
  /// This increments the caret.row and can activate vertical scrolling if the caret.row goes past
  /// the viewport height.
  ///
  /// This is meant to be called inside [validate::apply_change].
  pub fn inc_caret_row(
    caret: &mut Position,
    scroll_offset: &mut ScrollOffset,
    viewport_height: ChUnit,
  ) -> usize {
    let at_bottom_of_viewport = caret.row >= viewport_height;

    // Fun fact: The following logic is the same whether scroll is active or not.
    if at_bottom_of_viewport {
      scroll_offset.row += 1; // Activate scroll since at bottom of viewport.
    } else {
      caret.row += 1; // Scroll inactive & Not at bottom of viewport.
    }

    EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset)
  }

  /// Check whether caret is vertically within the viewport. This is meant to be used after resize
  /// events and for [inc_caret_col], [inc_caret_row] operations. Note that [dec_caret_col] and
  /// [dec_caret_row] are handled differently (and not by this function) since they can never be
  /// negative.
  ///
  /// - If it isn't then scroll by mutating:
  ///    1. [caret](EditorBuffer::get_caret())'s row , so it is within the viewport.
  ///    2. [scroll_offset](EditorBuffer::get_scroll_offset())'s row, to actually apply scrolling.
  /// - Otherwise, no changes are made.
  ///
  /// This function is not meant to be called directly, but instead is called by
  /// [validate::apply_change].
  pub fn validate_scroll(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    validate_vertical_scroll(EditorArgsMut { buffer, engine });
    validate_horizontal_scroll(EditorArgsMut { buffer, engine });

    /// Handle vertical scrolling (make sure caret is within viewport).
    ///
    /// Check whether caret is in the viewport.
    /// - If to top of viewport, then adjust scroll_offset & set it.
    /// - If to bottom of viewport, then adjust scroll_offset & set it.
    /// - If in viewport, then do nothing.
    ///
    /// ```text
    ///                    +0--------------------+
    ///                    0                     |
    ///                    |        above        | <- caret_row_adj
    ///                    |                     |
    ///                    +--- scroll_offset ---+
    ///              ->    |         ↑           |      ↑
    ///              |     |                     |      |
    ///   caret.row  |     |      within vp      |  vp height
    ///              |     |                     |      |
    ///              ->    |         ↓           |      ↓
    ///                    +--- scroll_offset ---+
    ///                    |    + vp height      |
    ///                    |                     |
    ///                    |        below        | <- caret_row_adj
    ///                    |                     |
    ///                    +---------------------+
    /// ```
    fn validate_vertical_scroll(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      let viewport_height = engine.viewport_height();

      // Make sure that caret can't go past the bottom of the buffer.
      {
        let caret_row_adj = buffer.get_caret(CaretKind::ScrollAdjusted).row;
        let is_caret_row_adj_overflows_buffer = caret_row_adj > buffer.len();
        if is_caret_row_adj_overflows_buffer {
          let diff = buffer.len() - caret_row_adj;
          let (_, caret, _) = buffer.get_mut();
          caret.row -= diff;
        }
      }

      // Make sure that scroll_offset can't go past the bottom of the buffer.
      {
        let scroll_offset_row = buffer.get_scroll_offset().row;
        let is_scroll_offset_row_overflows_buffer = scroll_offset_row > buffer.len();
        if is_scroll_offset_row_overflows_buffer {
          let diff = buffer.len() - scroll_offset_row;
          let (_, _, scroll_offset) = buffer.get_mut();
          scroll_offset.row -= diff;
        }
      }

      let caret_row_adj = buffer.get_caret(CaretKind::ScrollAdjusted).row;
      let scroll_offset_row = buffer.get_scroll_offset().row;

      let is_caret_row_adj_within_viewport = caret_row_adj >= scroll_offset_row
        && caret_row_adj <= (scroll_offset_row + viewport_height);

      match is_caret_row_adj_within_viewport {
        true => {
          // Caret is within viewport, do nothing.
        }
        false => {
          // Caret is outside viewport.
          let is_caret_row_adj_above_viewport = caret_row_adj < scroll_offset_row;
          match is_caret_row_adj_above_viewport {
            false => {
              // Caret is below viewport.
              let row_diff = caret_row_adj - (scroll_offset_row + viewport_height);
              let (_, caret, scroll_offset) = buffer.get_mut();
              scroll_offset.row += row_diff;
              caret.row -= row_diff;
            }
            true => {
              // Caret is above viewport.
              let row_diff = scroll_offset_row - caret_row_adj;
              let (_, caret, scroll_offset) = buffer.get_mut();
              scroll_offset.row -= row_diff;
              caret.row += row_diff;
            }
          }
        }
      }
    }

    /// Handle horizontal scrolling (make sure caret is within viewport).
    ///
    /// Check whether caret is in the viewport.
    /// - If to left of viewport, then adjust scroll_offset & set it.
    /// - If to right of viewport, then adjust scroll_offset & set it.
    /// - If in viewport, then do nothing.
    ///
    /// ```text
    ///           <-   vp width   ->
    /// +0--------+----------------+---------->
    /// 0         |                |
    /// | left of |<-  within vp ->| right of
    /// |         |                |
    /// +---------+----------------+---------->
    ///       scroll_offset    scroll_offset
    ///                        + vp width
    /// ```
    fn validate_horizontal_scroll(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      let viewport_width = engine.viewport_width();

      let caret_col_adj = buffer.get_caret(CaretKind::ScrollAdjusted).col;
      let scroll_offset_col = buffer.get_scroll_offset().col;

      let is_caret_col_abs_within_viewport =
        caret_col_adj >= scroll_offset_col && caret_col_adj < scroll_offset_col + viewport_width;

      match is_caret_col_abs_within_viewport {
        true => {
          // Caret is within viewport, nothing to do.
        }
        false => {
          // Caret is outside viewport.
          let (_, caret, scroll_offset) = buffer.get_mut();

          if caret_col_adj < scroll_offset_col {
            // Caret is to the left of viewport.
            scroll_offset.col = caret_col_adj;
            caret.col = ch!(0);
          } else {
            // Caret is to the right of viewport.
            scroll_offset.col = caret_col_adj - viewport_width + ch!(1);
            caret.col = viewport_width - ch!(1);
          }
        }
      }
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Caret location enums │
// ╯                      ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod enums {
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
pub use enums::*;

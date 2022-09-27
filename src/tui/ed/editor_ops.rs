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
  pub fn find_row(buffer: &EditorBuffer, engine: &EditorRenderEngine) -> CaretRowLocation {
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

  // TK: 🚨🌈 handle scrolling
  pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    // let EditorArgs { buffer, engine } = args;
    empty_check_early_return!(buffer, @None);
    match editor_ops_get_caret::find_row(buffer, engine) {
      CaretRowLocation::AtTopOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtBottomOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        let old_row_idx_raw = buffer.get_caret(CaretKind::Raw).row;
        let old_row_idx = buffer.get_caret(CaretKind::ScrollAdjusted).row;

        let bounds_size = engine.current_box.style_adjusted_bounds_size;

        mutate::change_editor_buffer(buffer, engine, |_, caret, scroll_offset| {
          scroll::dec_row(caret, scroll_offset);
        });

        let line_display_width = editor_ops_get_content::line_display_width_at_row_index(
          buffer,
          buffer.get_caret(CaretKind::ScrollAdjusted).row,
        );

        mutate::change_editor_buffer(buffer, engine, |_, caret, _| {
          caret.clip_col_to_bounds(line_display_width);
        });

        let new_row_idx_raw = buffer.get_caret(CaretKind::Raw).row;
        let new_row_idx = buffer.get_caret(CaretKind::ScrollAdjusted).row;

        // TK: ‼️ remove debug
        log_no_err!(
          DEBUG,
          "🔼🔼🔼 \n\told_row [raw: {:?}, adj: {:?}], \n\tnew_row [raw: {:?}, adj: {:?}]\n\tscroll_offset: {:?}",
          *old_row_idx_raw,
          *old_row_idx,
          *new_row_idx_raw,
          *new_row_idx,
          *buffer.get_scroll_offset().row
        );
      }
    }

    None
  }

  // TK: 🚨🌈 handle scrolling
  pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_get_caret::find_row(buffer, engine) {
      CaretRowLocation::AtBottomOfBuffer => {
        // Do nothing.
      }
      CaretRowLocation::AtTopOfBuffer | CaretRowLocation::InMiddleOfBuffer => {
        let max_row = ch!(buffer.get_lines().len(), @dec);
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row;
        if row_index < max_row {
          let new_row = row_index + 1;
          let line_display_width =
            editor_ops_get_content::line_display_width_at_row_index(buffer, new_row);
          mutate::change_editor_buffer(buffer, engine, |_, caret, _| {
            caret.row = new_row;
            caret.clip_col_to_bounds(line_display_width);
          });
        }
      }
    }

    None
  }

  // TK: 🚨🔮 handle scrolling
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
        mutate::change_editor_buffer(buffer, engine, |_, caret, _| {
          caret.add_col_with_bounds(unicode_width, max_display_width);
        });
      }
    }

    None
  }

  // TK: 🚨🔮 handle scrolling
  pub fn left(buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) -> Nope {
    empty_check_early_return!(buffer, @None);
    match editor_ops_get_caret::find_col(EditorArgs { buffer, engine }) {
      CaretColLocation::AtStartOfLine => {
        // Do nothing.
      }
      CaretColLocation::AtEndOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_get_content::string_at_end_of_line_at_caret(buffer, engine)?;
        mutate::change_editor_buffer(buffer, engine, |_, caret, _| {
          caret.col -= unicode_width;
        });
      }
      CaretColLocation::InMiddleOfLine => {
        let UnicodeStringSegmentSliceResult { unicode_width, .. } =
          editor_ops_get_content::string_to_left_of_caret(buffer, engine)?;
        mutate::change_editor_buffer(buffer, engine, |_, caret, _| {
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
      mutate::change_editor_buffer(buffer, engine, |lines, _, _| {
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
      mutate::change_editor_buffer(buffer, engine, |lines, caret, _| {
        // TK: 🚨🌈 call scroll::inc_row()
        caret.add_row(1);
        caret.reset_col();
        lines.insert(ch!(@to_usize caret.row), String::new().into());
      });
    }

    // Handle inserting a new line at the start of the current line.
    fn insert_new_line_at_start_of_current_line(args: EditorArgsMut<'_>) {
      let EditorArgsMut { buffer, engine } = args;

      let row_index = ch!(@to_usize buffer.get_caret(CaretKind::ScrollAdjusted).row);
      if row_index == 0 {
        mutate::change_editor_buffer(buffer, engine, |lines, _, _| {
          lines.insert(0, String::new().into());
        });
      } else {
        mutate::change_editor_buffer(buffer, engine, |lines, _, _| {
          lines.insert(row_index, String::new().into());
        });
      }
      mutate::change_editor_buffer(buffer, engine, |_, caret, _| {
        // TK: 🚨🌈 call scroll::inc_row()
        caret.add_row(1);
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
          mutate::change_editor_buffer(buffer, engine, |lines, caret, _| {
            let _ = replace(&mut lines[row_index], left);
            lines.insert(row_index + 1, right);
            // TK: 🚨🌈 call scroll::inc_row()
            caret.add_row(1);
            caret.reset_col();
          });
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
      mutate::change_editor_buffer(buffer, engine, |lines, _, _| {
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

      mutate::change_editor_buffer(buffer, engine, |lines, caret, _| {
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
      mutate::change_editor_buffer(buffer, engine, |lines, caret, _| {
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
      mutate::change_editor_buffer(buffer, engine, |lines, caret, _| {
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

  fn insert_into_existing_line(args: EditorArgsMut<'_>, pos: Position, chunk: &str) -> Nope {
    let EditorArgsMut { buffer, engine } = args;

    let row_index = ch!(@to_usize pos.row);
    let line = buffer.get_lines().get(row_index)?;

    let NewUnicodeStringResult {
      new_unicode_string: new_line,
      unicode_width: char_display_width,
    } = line.insert_char_at_display_col(ch!(pos.col), chunk)?;

    mutate::change_editor_buffer(buffer, engine, |lines, caret, _| {
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
          mutate::change_editor_buffer(buffer, engine, |lines, _, _| {
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

    mutate::change_editor_buffer(buffer, engine, |lines, caret, _| {
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
  use std::cmp::Ordering;

  use super::*;

  /// Internal function.
  pub(super) fn change_editor_buffer(
    buffer: &mut EditorBuffer,
    engine: &mut EditorRenderEngine,
    mutator: impl FnOnce(
      /* EditorBuffer::lines */ &mut Vec<UnicodeString>,
      /* EditorBuffer::caret */ &mut Position,
      /* EditorRenderEngine::scroll_offset */ &mut ScrollOffset,
    ),
  ) -> Nope {
    // TK: ‼️ remove debug
    let dbg_pre_mut_caret = buffer.get_caret(CaretKind::ScrollAdjusted);
    let dbg_pre_mut_scroll_offset = buffer.get_scroll_offset();

    // Run the mutator first.
    let (lines, caret, scroll_offset) = buffer.get_mut();
    mutator(lines, caret, scroll_offset);

    // TK: ‼️ remove debug
    let dbg_post_mut_caret = buffer.get_caret(CaretKind::ScrollAdjusted);
    let dbg_post_mut_scroll_offset = buffer.get_scroll_offset();

    // Check to see whether the caret is in the correct display column.
    validate_caret_col_position_not_in_middle_of_grapheme_cluster(buffer);

    // TK: ‼️ remove debug
    log_no_err!(
      DEBUG,
      "📜📜📜 pre_mut_caret -> {:?}, post_mut_caret -> {:?}",
      dbg_pre_mut_caret,
      dbg_post_mut_caret,
    );

    // TK: ‼️ remove debug
    match dbg_post_mut_caret.row.cmp(&dbg_pre_mut_caret.row) {
      Ordering::Greater => {}
      Ordering::Less => {}
      Ordering::Equal => {}
    }

    // TK: ‼️ remove debug
    match dbg_post_mut_scroll_offset
      .row
      .cmp(&dbg_pre_mut_scroll_offset.row)
    {
      Ordering::Greater => {}
      Ordering::Less => {}
      Ordering::Equal => {}
    }

    // TK: 🚨🔮 Check to see if horizontal scroll is needed on caret inc
    // TK: 🚨🔮 Check to see if horizontal scroll is needed on caret dec

    None
  }

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

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Internal module: scroll │
// ╯                         ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub(super) mod scroll {
  use super::*;

  pub fn dec_row(caret: &mut Position, scroll_offset: &mut ScrollOffset) {
    if scroll_offset.row > ch!(0) {
      // Scrolling is active.
      if caret.row > ch!(0) {
        caret.row -= 1;
      } else {
        scroll_offset.row -= 1;
      }
    } else {
      // Scrolling is inactive.
      caret.row -= 1;
    }
  }

  pub fn inc_row(caret: &mut Position, buffer: &mut EditorBuffer, engine: &mut EditorRenderEngine) {
    caret.row += 1;
    validate_caret_in_viewport_activate_scroll_if_needed(EditorArgsMut { buffer, engine });
  }

  /// Check whether caret is vertically within the viewport.
  /// - If it isn't then scroll by mutating:
  ///    1. [caret](EditorBuffer::caret)'s row , so it is within the viewport.
  ///    2. [scroll_offset](EditorEngine::scroll_offset)'s row, to actually apply scrolling.
  /// - Otherwise, no changes are made.
  pub fn validate_caret_in_viewport_activate_scroll_if_needed(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    if let CaretRowLocation::AtBottomOfBuffer = editor_ops_get_caret::find_row(buffer, engine) {
      let max_display_row_count = engine.current_box.style_adjusted_bounds_size.rows;
      let caret_row = buffer.get_caret(CaretKind::Raw).row;
      if caret_row > max_display_row_count {
        let (_, caret, scroll_offset) = buffer.get_mut();
        scroll_offset.row += 1;
        caret.sub_row(1);
      }
    }
  }

  // TK: 🚨🌈 impl this
  pub fn inc_col() {}
  // TK: 🚨🌈 impl this
  pub fn dec_col() {}
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

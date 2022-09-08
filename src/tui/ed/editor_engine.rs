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

use std::fmt::Debug;

use serde::*;

use crate::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorEngine;

/// Private struct to help keep function signatures smaller.
struct Context<'a> {
  editor_buffer: &'a EditorBuffer,
  style_adj_box_origin_pos: Position,
  style_adj_box_bounds_size: Size,
  has_focus: &'a HasFocus,
  current_box: &'a FlexBox,
}

const DEFAULT_CURSOR_CHAR: char = '▒';

#[allow(dead_code)]
enum CaretPaintStyle {
  /// Using cursor show / hide.
  GlobalCursor,
  /// Painting the editor_buffer.caret position w/ reverse style.
  LocalPaintedEffect,
}

impl EditorEngine {
  // FIXME: impl apply #23
  pub async fn apply(
    &mut self, editor_buffer: &EditorBuffer, input_event: &InputEvent,
  ) -> CommonResult<Option<EditorBuffer>> {
    match input_event {
      // Process each character.
      InputEvent::Keyboard(Keypress::Plain {
        key: Key::Character(character),
      }) => {
        let mut new_editor_buffer = editor_buffer.clone();
        new_editor_buffer.insert_char_into_current_line(*character);
        Ok(Some(new_editor_buffer))
      }
      // Other keypresses.
      _ => Ok(None),
    }
  }

  // FIXME: impl render #23
  pub async fn render(
    &mut self, editor_buffer: &EditorBuffer, has_focus: &HasFocus, current_box: &FlexBox,
  ) -> CommonResult<TWCommandQueue> {
    throws_with_return!({
      // Create this struct to pass around fewer variables.
      let context = Context {
        editor_buffer,
        style_adj_box_origin_pos: current_box.style_adjusted_origin_pos, // Adjusted for padding (if set).
        style_adj_box_bounds_size: current_box.style_adjusted_bounds_size, // Adjusted for padding (if set).
        has_focus,
        current_box,
      };

      if editor_buffer.buffer.is_empty() {
        render_empty_state(&context)
      } else {
        let q_content = render_content(&context);
        let q_caret = render_caret(CaretPaintStyle::LocalPaintedEffect, &context);
        command_queue!(@join_and_drop q_content, q_caret)
      }
    })
  }
}

// This simply clips the content to the `style_adj_box_bounds_size`.
fn render_content(context_ref: &Context<'_>) -> TWCommandQueue {
  let Context {
    style_adj_box_origin_pos,
    style_adj_box_bounds_size,
    current_box,
    editor_buffer,
    ..
  } = context_ref;
  let mut queue = command_queue!(@new_empty);

  let Size {
    col: max_content_display_cols,
    row: mut max_display_row_count,
  } = style_adj_box_bounds_size;

  // Paint each line in the buffer.
  for (index, line) in editor_buffer.buffer.iter().enumerate() {
    // Clip the content to max rows.
    if max_display_row_count == 0 {
      break;
    }
    // Clip the content to max cols.
    let line_unicode_string = line.unicode_string();
    let truncated_line =
      line_unicode_string.truncate_to_fit_display_cols(*max_content_display_cols);
    command_queue! {
      @push_into queue at ZOrder::Normal =>
        TWCommand::MoveCursorPositionRelTo(
        *style_adj_box_origin_pos,
        position! { col: 0 , row: convert_to_base_unit!(index) }
        ),
        TWCommand::ApplyColors(current_box.get_computed_style()),
        TWCommand::PrintPlainTextWithAttributes(truncated_line.into(), current_box.get_computed_style()),
        TWCommand::ResetColor
    };
    if max_display_row_count >= 1 {
      max_display_row_count -= 1;
    }
  }

  queue
}

/// Implement caret painting using two different strategies represented by [CaretPaintStyle].
fn render_caret(style: CaretPaintStyle, context_ref: &Context<'_>) -> TWCommandQueue {
  let Context {
    style_adj_box_origin_pos,
    has_focus,
    current_box,
    editor_buffer,
    ..
  } = context_ref;
  let mut queue: TWCommandQueue = TWCommandQueue::default();

  if has_focus.does_current_box_have_focus(current_box) {
    match style {
      CaretPaintStyle::GlobalCursor => {
        command_queue! {
          @push_into queue at ZOrder::Caret =>
            TWCommand::RequestShowCaretAtPositionRelTo(*style_adj_box_origin_pos, editor_buffer.caret)
        };
      }
      CaretPaintStyle::LocalPaintedEffect => {
        command_queue! {
          @push_into queue at ZOrder::Caret =>
            TWCommand::MoveCursorPositionRelTo(*style_adj_box_origin_pos, editor_buffer.caret),
            TWCommand::PrintPlainTextWithAttributes(
              editor_buffer.get_char_at_caret().unwrap_or(DEFAULT_CURSOR_CHAR).into(),
              style! { attrib: [reverse] }.into()),
            TWCommand::MoveCursorPositionRelTo(*style_adj_box_origin_pos, editor_buffer.caret)
        };
      }
    }
  }

  queue
}

fn render_empty_state(context_ref: &Context<'_>) -> TWCommandQueue {
  let Context {
    style_adj_box_origin_pos,
    style_adj_box_bounds_size,
    has_focus,
    current_box,
    ..
  } = context_ref;
  let mut queue: TWCommandQueue = TWCommandQueue::default();
  let mut content_cursor_pos = position! { col: 0 , row: 0 };

  // Paint the text.
  command_queue! {
    @push_into queue at ZOrder::Normal =>
      TWCommand::MoveCursorPositionRelTo(*style_adj_box_origin_pos, position! { col: 0 , row: 0 }),
      TWCommand::ApplyColors(style! {
        color_fg: TWColor::Red
      }.into()),
      TWCommand::PrintPlainTextWithAttributes("No content added".into(), None),
      TWCommand::ResetColor
  };

  // Paint the emoji.
  if has_focus.does_current_box_have_focus(current_box) {
    command_queue! {
      @push_into queue at ZOrder::Normal =>
        TWCommand::MoveCursorPositionRelTo(
          *style_adj_box_origin_pos,
          content_cursor_pos.add_rows_with_bounds(1, *style_adj_box_bounds_size)),
        TWCommand::PrintPlainTextWithAttributes("👀".into(), None)
    };
  }

  queue
}

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

impl EditorEngine {
  // TODO: impl apply
  pub async fn apply(
    &mut self, editor_buffer: &EditorBuffer, input_event: &InputEvent,
  ) -> CommonResult<Option<EditorBuffer>> {
    if let InputEvent::Keyboard(Keypress::Plain { .. }) = input_event {
      Ok(Some(editor_buffer.clone()))
    } else {
      Ok(None)
    }
  }

  // TODO: impl render
  pub async fn render(
    &mut self, editor_buffer: &EditorBuffer, has_focus: &HasFocus, current_box: &FlexBox,
  ) -> CommonResult<TWCommandQueue> {
    throws_with_return!({
      // Setup intermediate vars.
      let box_origin_pos = current_box.style_adjusted_origin_pos; // Adjusted for style margin (if any).
      let box_bounds_size = current_box.style_adjusted_bounds_size; // Adjusted for style margin (if any).
      let mut content_cursor_pos = position! { col: 0 , row: 0 };
      let mut queue: TWCommandQueue = TWCommandQueue::default();

      if editor_buffer.buffer.is_empty() {
        // Paint no content.
        tw_command_queue! {
          queue push
          TWCommand::MoveCursorPositionRelTo(box_origin_pos, content_cursor_pos),
          TWCommand::ApplyColors(current_box.get_computed_style()),
          TWCommand::PrintWithAttributes("No content added".into(), current_box.get_computed_style()),
          TWCommand::ResetColor
        };
      } else {
        // Paint the buffer.
        for line in &editor_buffer.buffer {
          tw_command_queue! {
            queue push
            TWCommand::MoveCursorPositionRelTo(box_origin_pos, content_cursor_pos),
            TWCommand::ApplyColors(current_box.get_computed_style()),
            TWCommand::PrintWithAttributes(line.into(), current_box.get_computed_style()),
            TWCommand::ResetColor
          };
          content_cursor_pos.add_row_with_bounds(1, box_bounds_size);
        }
      }

      // Paint is_focused.
      if has_focus.does_current_box_have_focus(current_box) {
        tw_command_queue! {
          queue push
          TWCommand::MoveCursorPositionRelTo(
            box_origin_pos,
            content_cursor_pos.add_row_with_bounds(1, box_bounds_size)
          ),
          TWCommand::PrintWithAttributes("👀".into(), None)
        };
      }

      // Return the command queue.
      queue
    })
  }
}

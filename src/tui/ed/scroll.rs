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

use crate::*;

pub mod manage_scroll {
  use super::*;

  pub fn mutate() -> Nope { None }

  pub fn detect(
    origin_pos: &Position, size: &Size, editor_buffer: &EditorBuffer,
  ) -> Option<ScrollOffset> {
    let content_line_count = ch!(editor_buffer.get_lines().len()); /* 1 index */

    let viewport_max_row_count = size.row /* 0 index */ + 1;
    let viewport_max_row_index = origin_pos.row + size.row;

    let caret_row_index = editor_buffer.get_caret().row;

    if viewport_max_row_count > content_line_count {
      return None;
    }

    // TK: handle scrolling: up, or down (based on caret location)
    let mut scroll_offset: ScrollOffset = position!(col:0, row:0);

    if caret_row_index > viewport_max_row_index {
      scroll_offset.row = caret_row_index - viewport_max_row_index;
    };

    Some(scroll_offset)
  }
}

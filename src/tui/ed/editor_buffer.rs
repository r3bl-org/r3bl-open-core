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

use crate::*;

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

pub enum CaretDirection {
  Up,
  Down,
  Left,
  Right,
}

impl EditorBuffer {
  pub fn is_empty(&self) -> bool { self.vec_lines.is_empty() }

  pub fn insert_new_line(&mut self) { line_buffer_insert::new_line_at_caret(self); }

  /// Insert [char] at the current [caret position](EditorBuffer::caret) into the current line.
  pub fn insert_char(&mut self, character: char) {
    line_buffer_insert::str_at_caret(self, &char_to_string(character))
  }

  /// Insert [str] at the current [caret position](EditorBuffer::caret) into the current line.
  pub fn insert_str(&mut self, chunk: &str) { line_buffer_insert::str_at_caret(self, chunk) }

  /// Move one character to the left, or right. Calculate how wide the current character is (unicode
  /// width) and then move the "display" caret position back that many columns.
  pub fn move_caret(&mut self, direction: CaretDirection) {
    match direction {
      CaretDirection::Left => line_buffer_move_caret::left(self),
      CaretDirection::Right => line_buffer_move_caret::right(self),
      CaretDirection::Up => line_buffer_move_caret::up(self),
      CaretDirection::Down => line_buffer_move_caret::down(self),
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

pub(crate) fn char_to_string(character: char) -> String {
  let my_string: String = String::from(character);
  my_string
}

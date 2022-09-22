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

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBuffer │
// ╯              ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Stores the data for a single editor buffer.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize, GetSize)]
pub struct EditorBuffer {
  /// A list of lines representing the document being edited.
  lines: Vec<UnicodeString>,
  /// The current caret position. This is the "display" and not "logical" position as defined in
  /// [UnicodeString]. This works w/ [crate::RenderOp] as well, so you can directly move this
  /// position.
  caret: Position,
  /// Lolcat struct for generating rainbow colors.
  pub lolcat: Lolcat,
  /// Layout data, set by [apply_editor_event](EditorBuffer::apply_editor_event)
  pub bounds_size: Size,
  /// Layout data, set by [apply_editor_event](EditorBuffer::apply_editor_event)
  pub origin_pos: Position,
}

pub mod access_and_mutate {
  use super::*;

  impl EditorBuffer {
    pub fn get_mut(&mut self) -> (&mut Vec<UnicodeString>, &mut Position) {
      (&mut self.lines, &mut self.caret)
    }

    pub fn get_lines(&self) -> &Vec<UnicodeString> { &self.lines }

    pub fn get_caret(&self) -> Position { self.caret }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBuffer -> Event based interface │
// ╯                                       ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Example.
/// ```rust
/// use r3bl_rs_utils::*;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// let mut editor_buffer = EditorBuffer::default();
/// editor_buffer.apply_editor_event(
///   EditorEvent::new(
///     EditorBufferCommand::InsertChar('a'),
///     Position::default(),
///     Size::default(),
///   ), &Arc::new(RwLock::new(TWData::default()))
/// );
/// ```
impl EditorBuffer {
  pub fn apply_editor_event(&mut self, editor_event: EditorEvent, shared_tw_data: &SharedTWData) {
    let EditorEvent {
      editor_buffer_command,
      bounds_size,
      origin_pos,
    } = editor_event;

    // TK: 💉✅ save the bounds_size & origin of the box in EditorBuffer on apply
    // Save the extra layout and position data for later.
    self.bounds_size = bounds_size;
    self.origin_pos = origin_pos;

    // TK: 🚨 use shared_tw_data::user_data_store in order to save/load user data
    match editor_buffer_command {
      EditorBufferCommand::InsertChar(character) => self.insert_char(character),
      EditorBufferCommand::InsertNewLine => self.insert_new_line(),
      EditorBufferCommand::Delete => self.delete(),
      EditorBufferCommand::Backspace => self.backspace(),
      EditorBufferCommand::MoveCaret(direction) => self.move_caret(direction),
      EditorBufferCommand::InsertString(string) => self.insert_str(&string),
    };
  }

  /// Example.
  /// ```rust
  /// use r3bl_rs_utils::*;
  /// use std::sync::Arc;
  /// use tokio::sync::RwLock;
  ///
  /// let mut editor_buffer = EditorBuffer::default();
  /// editor_buffer.apply_editor_events(vec![
  ///  EditorEvent::new(EditorBufferCommand::InsertChar('a'),
  ///     Position::default(),
  ///     Size::default(),
  ///   ),
  ///  EditorEvent::new(EditorBufferCommand::MoveCaret(CaretDirection::Left),
  ///    Position::default(),
  ///    Size::default(),
  ///  ),
  /// ], &Arc::new(RwLock::new(TWData::default())));
  /// ```
  pub fn apply_editor_events(
    &mut self, editor_event_vec: Vec<EditorEvent>, shared_tw_data: &SharedTWData,
  ) {
    for editor_event in editor_event_vec {
      self.apply_editor_event(editor_event, shared_tw_data);
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBuffer -> Function based interface │
// ╯                                          ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
impl EditorBuffer {
  pub fn is_empty(&self) -> bool { self.lines.is_empty() }

  pub fn insert_new_line(&mut self) { line_buffer_content_mut::insert_new_line_at_caret(self); }

  /// Insert [char] at the current [caret position](EditorBuffer::get_caret) into the current line.
  pub fn insert_char(&mut self, character: char) {
    line_buffer_content_mut::insert_str_at_caret(self, &String::from(character))
  }

  /// Insert [str] at the current [caret position](EditorBuffer::get_caret) into the current line.
  pub fn insert_str(&mut self, chunk: &str) {
    line_buffer_content_mut::insert_str_at_caret(self, chunk)
  }

  /// Move one character to the left, or right. Calculate how wide the current character is (unicode
  /// width) and then move the "display" caret position back that many columns.
  pub fn move_caret(&mut self, direction: CaretDirection) {
    match direction {
      CaretDirection::Left => line_buffer_caret_mut::left(self),
      CaretDirection::Right => line_buffer_caret_mut::right(self),
      CaretDirection::Up => line_buffer_caret_mut::up(self),
      CaretDirection::Down => line_buffer_caret_mut::down(self),
    };
  }

  pub fn delete(&mut self) { line_buffer_content_mut::delete_at_caret(self); }

  pub fn backspace(&mut self) { line_buffer_content_mut::backspace_at_caret(self); }
}

mod debug_format_helpers {
  use super::*;

  impl std::fmt::Debug for EditorBuffer {
    fn fmt(&self, f: &mut __private::Formatter<'_>) -> std::fmt::Result {
      write! { f,
        "\nEditorBuffer [ \n \
        ├ lines: {}, size: {}, \n \
        ├ caret: {:?}, \n \
        ├ origin_pos: {:?}, bounds_size: {:?}\n \
        └ lolcat: [{}, {}, {}, {}] \n]",
        self.lines.len(), self.lines.get_heap_size(),
        self.caret,
        self.origin_pos, self.bounds_size,
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

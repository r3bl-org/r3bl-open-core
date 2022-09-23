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

use std::{fmt::{Debug, Display, Result},
          ops::Add};

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::*;

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBuffer │
// ╯              ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Stores the data for a single editor buffer. This struct is stored in the [Store]. And it is
/// paired w/ [EditorEngine] at runtime; which provides all the operations that can be performed on
/// this.
#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct EditorBuffer {
  /// A list of lines representing the document being edited.
  lines: Vec<UnicodeString>,
  /// The current caret position. This is the "display" and not "logical" position as defined in
  /// [UnicodeString]. This works w/ [crate::RenderOp] as well, so you can directly move this
  /// position.
  caret: Position,
  /// Lolcat struct for generating rainbow colors.
  pub lolcat: Lolcat,
}

mod constructor {
  use super::*;

  impl Default for EditorBuffer {
    fn default() -> Self {
      // Potentially do any other initialization here.
      call_if_true!(DEBUG_TUI_MOD, {
        log_no_err!(
          DEBUG,
          "🪙 {}",
          "construct EditorBuffer { lines, caret, lolcat }"
        );
      });

      Self {
        lines: vec![UnicodeString::default()],
        caret: Position::default(),
        lolcat: Lolcat::default(),
      }
    }
  }
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
impl EditorBuffer {
  pub fn apply_editor_event<S, A>(
    engine: &mut EditorEngine, this: &mut EditorBuffer, editor_buffer_command: EditorBufferCommand,
    shared_tw_data: &SharedTWData, component_registry: &mut ComponentRegistry<S, A>, self_id: &str,
  ) where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    let EditorEngine {
      bounds_size,
      origin_pos,
      ..
    } = engine;

    // TK: 🚨 pass engine to all the functions below
    match editor_buffer_command {
      EditorBufferCommand::InsertChar(character) => this.insert_char(character),
      EditorBufferCommand::InsertNewLine => this.insert_new_line(engine),
      EditorBufferCommand::Delete => this.delete(),
      EditorBufferCommand::Backspace => this.backspace(),
      EditorBufferCommand::MoveCaret(direction) => this.move_caret(direction),
      EditorBufferCommand::InsertString(string) => this.insert_str(&string),
    };
  }

  pub fn apply_editor_events<S, A>(
    engine: &mut EditorEngine, this: &mut EditorBuffer, editor_event_vec: Vec<EditorBufferCommand>,
    shared_tw_data: &SharedTWData, component_registry: &mut ComponentRegistry<S, A>, self_id: &str,
  ) where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    for editor_event in editor_event_vec {
      EditorBuffer::apply_editor_event(
        engine,
        this,
        editor_event,
        shared_tw_data,
        component_registry,
        self_id,
      );
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBuffer -> Function based interface │
// ╯                                          ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
impl EditorBuffer {
  pub fn is_empty(&self) -> bool { self.lines.is_empty() }

  pub fn insert_new_line(&mut self, engine: &mut EditorEngine) {
    line_buffer_content_mut::insert_new_line_at_caret(self, engine);
  }

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

  impl Debug for EditorBuffer {
    fn fmt(&self, f: &mut __private::Formatter<'_>) -> Result {
      write! { f,
        "\nEditorBuffer [ \n \
        ├ lines: {}, size: {}, \n \
        ├ caret: {:?}, \n \
        └ lolcat: [{}, {}, {}, {}] \n]",
        self.lines.len(), self.lines.get_heap_size(),
        self.caret,
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

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
  /// The col and row offset for scrolling if active.
  scroll_offset: Position,
  /// Lolcat struct for generating rainbow colors.
  pub lolcat: Lolcat,
}

pub type ScrollOffset = Position;

pub mod access_and_mutate {
  use super::*;

  impl EditorBuffer {
    pub fn get_mut(&mut self) -> (&mut Vec<UnicodeString>, &mut Position, &mut ScrollOffset) {
      (&mut self.lines, &mut self.caret, &mut self.scroll_offset)
    }

    pub fn get_lines(&self) -> &Vec<UnicodeString> { &self.lines }

    pub fn get_caret(&self) -> Position { self.caret }

    pub fn get_scroll_offset(&self) -> Position { self.scroll_offset }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBuffer -> Event based interface │
// ╯                                       ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Example.
/// ```rust
/// use r3bl_rs_utils::*;
///
/// let mut editor_buffer = EditorBuffer::default();
/// editor_buffer.apply_command(EditorBufferCommand::InsertChar('a'));
/// ```
impl EditorBuffer {
  pub fn apply_command(&mut self, command: EditorBufferCommand) {
    match command {
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
  ///
  /// let mut editor_buffer = EditorBuffer::default();
  /// editor_buffer.apply_commands(vec![
  ///  EditorBufferCommand::InsertChar('a'),
  ///  EditorBufferCommand::MoveCaret(CaretDirection::Left),
  /// ]);
  /// ```
  pub fn apply_commands(&mut self, commands: Vec<EditorBufferCommand>) {
    for command in commands {
      self.apply_command(command);
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

pub mod editor_buffer_command {
  use super::*;

  // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
  // │ EditorBufferCommand │
  // ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  /// Commands that can be executed on an [EditorBuffer]. By providing a conversion from [InputEvent]
  /// to [EditorBufferCommand] it becomes easier to write event handlers that consume [InputEvent] and
  /// then execute [EditorBufferCommand] on an [EditorBuffer].
  #[derive(Clone, PartialEq, Eq, Serialize, Deserialize, GetSize)]
  pub enum EditorBufferCommand {
    InsertChar(char),
    InsertString(String),
    InsertNewLine,
    Delete,
    Backspace,
    MoveCaret(CaretDirection),
  }

  #[derive(Clone, PartialEq, Eq, Serialize, Deserialize, GetSize)]
  pub enum CaretDirection {
    Up,
    Down,
    Left,
    Right,
  }

  impl EditorBufferCommand {
    pub fn try_convert_input_event(input_event: &InputEvent) -> Option<EditorBufferCommand> {
      let maybe_editor_buffer_command: Result<EditorBufferCommand, _> = input_event.try_into();
      match maybe_editor_buffer_command {
        Ok(editor_buffer_command) => Some(editor_buffer_command),
        Err(_) => None,
      }
    }
  }

  impl TryFrom<&InputEvent> for EditorBufferCommand {
    type Error = String;

    fn try_from(input_event: &InputEvent) -> Result<Self, Self::Error> {
      match input_event {
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::Character(character),
        }) => Ok(Self::InsertChar(*character)),
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Enter),
        }) => Ok(Self::InsertNewLine),
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Delete),
        }) => Ok(Self::Delete),
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Backspace),
        }) => Ok(Self::Backspace),
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Up),
        }) => Ok(Self::MoveCaret(CaretDirection::Up)),
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Down),
        }) => Ok(Self::MoveCaret(CaretDirection::Down)),
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Left),
        }) => Ok(Self::MoveCaret(CaretDirection::Left)),
        InputEvent::Keyboard(Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Right),
        }) => Ok(Self::MoveCaret(CaretDirection::Right)),
        _ => Err(format!("Invalid input event: {:?}", input_event)),
      }
    }
  }
}
pub use editor_buffer_command::*;

mod debug_format_helpers {
  use super::*;

  impl std::fmt::Debug for EditorBuffer {
    fn fmt(&self, f: &mut __private::Formatter<'_>) -> std::fmt::Result {
      write! { f,
        "\nEditorBuffer [ \n ├ lines: {}, size: {}, \n ├ caret: {:?}, scroll_offset: {:?}, \n └ lolcat: [{}, {}, {}, {}] \n]",
        self.lines.len(),
        self.lines.get_heap_size(),
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

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

use serde::{Deserialize, Serialize};

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBufferCommand │
// ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Commands that can be executed on an [EditorBuffer]. By providing a conversion from [InputEvent]
/// to [EditorBufferCommand] it becomes easier to write event handlers that consume [InputEvent] and
/// then execute [EditorBufferCommand] on an [EditorBuffer].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorBufferCommand {
  InsertChar(char),
  InsertString(String),
  InsertNewLine,
  Delete,
  Backspace,
  Home,
  End,
  PageDown,
  PageUp,
  MoveCaret(CaretDirection),
  Resize(Size),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaretDirection {
  Up,
  Down,
  Left,
  Right,
}

impl TryFrom<&InputEvent> for EditorBufferCommand {
  type Error = String;

  fn try_from(input_event: &InputEvent) -> Result<Self, Self::Error> {
    match input_event {
      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::PageDown),
      }) => Ok(EditorBufferCommand::PageDown),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::PageUp),
      }) => Ok(EditorBufferCommand::PageUp),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::Home),
      }) => Ok(EditorBufferCommand::Home),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::End),
      }) => Ok(EditorBufferCommand::End),

      InputEvent::Resize(size) => Ok(EditorBufferCommand::Resize(*size)),

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

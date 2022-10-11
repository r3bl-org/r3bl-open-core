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

use std::fmt::{Debug, Display};

use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄╮
// │ EditorOp │
// ╯          ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Commands that can be executed on an [EditorBuffer]. By providing a conversion from [InputEvent]
/// to [EditorOp] it becomes easier to write event handlers that consume [InputEvent] and
/// then execute [EditorOp] on an [EditorBuffer].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorOp {
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

impl TryFrom<&InputEvent> for EditorOp {
  type Error = String;

  fn try_from(input_event: &InputEvent) -> Result<Self, Self::Error> {
    match input_event {
      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::PageDown),
      }) => Ok(EditorOp::PageDown),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::PageUp),
      }) => Ok(EditorOp::PageUp),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::Home),
      }) => Ok(EditorOp::Home),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::End),
      }) => Ok(EditorOp::End),

      InputEvent::Resize(size) => Ok(EditorOp::Resize(*size)),

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

impl EditorOp {
  pub fn apply_editor_event<S, A>(
    engine: &mut EditorEngine,
    buffer: &mut EditorBuffer,
    editor_buffer_command: EditorOp,
    _shared_tw_data: &SharedTWData,
    _component_registry: &mut ComponentRegistry<S, A>,
    _self_id: &str,
  ) where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    match editor_buffer_command {
      EditorOp::InsertChar(character) => {
        mut_content::insert_str_at_caret(EditorArgsMut { buffer, engine }, &String::from(character))
      }
      EditorOp::InsertNewLine => {
        mut_content::insert_new_line_at_caret(EditorArgsMut { buffer, engine });
      }
      EditorOp::Delete => {
        mut_content::delete_at_caret(buffer, engine);
      }
      EditorOp::Backspace => {
        mut_content::backspace_at_caret(buffer, engine);
      }
      EditorOp::MoveCaret(direction) => {
        match direction {
          CaretDirection::Left => move_caret::left(buffer, engine),
          CaretDirection::Right => move_caret::right(buffer, engine),
          CaretDirection::Up => move_caret::up(buffer, engine),
          CaretDirection::Down => move_caret::down(buffer, engine),
        };
      }
      EditorOp::InsertString(chunk) => {
        mut_content::insert_str_at_caret(EditorArgsMut { buffer, engine }, &chunk)
      }
      EditorOp::Resize(_) => {
        // Check to see whether scroll is valid.
        scroll::validate_scroll(EditorArgsMut { buffer, engine });
      }
      EditorOp::Home => {
        move_caret::to_start_of_line(buffer, engine);
      }
      EditorOp::End => {
        move_caret::to_end_of_line(buffer, engine);
      }
      EditorOp::PageDown => {
        move_caret::page_down(buffer, engine);
      }
      EditorOp::PageUp => {
        move_caret::page_up(buffer, engine);
      }
    };
  }

  pub fn apply_editor_events<S, A>(
    engine: &mut EditorEngine,
    buffer: &mut EditorBuffer,
    editor_event_vec: Vec<EditorOp>,
    shared_tw_data: &SharedTWData,
    component_registry: &mut ComponentRegistry<S, A>,
    self_id: &str,
  ) where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    for editor_event in editor_event_vec {
      EditorOp::apply_editor_event(
        engine,
        buffer,
        editor_event,
        shared_tw_data,
        component_registry,
        self_id,
      );
    }
  }
}

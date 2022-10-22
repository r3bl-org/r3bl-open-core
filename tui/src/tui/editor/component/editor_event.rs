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

use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::*;

// ┏━━━━━━━━━━━━━┓
// ┃ EditorEvent ┃
// ┛             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Events that can be applied to the [EditorEngine] to modify an [EditorBuffer].
///
/// By providing a conversion from [InputEvent] to [EditorEvent] it becomes easier to write event
/// handlers that consume [InputEvent] and then execute [EditorEvent] on an [EditorBuffer].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorEvent {
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

impl TryFrom<&InputEvent> for EditorEvent {
  type Error = String;

  fn try_from(input_event: &InputEvent) -> Result<Self, Self::Error> {
    match input_event {
      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::PageDown),
      }) => Ok(EditorEvent::PageDown),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::PageUp),
      }) => Ok(EditorEvent::PageUp),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::Home),
      }) => Ok(EditorEvent::Home),

      InputEvent::Keyboard(Keypress::Plain {
        key: Key::SpecialKey(SpecialKey::End),
      }) => Ok(EditorEvent::End),

      InputEvent::Resize(size) => Ok(EditorEvent::Resize(*size)),

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

impl EditorEvent {
  pub fn apply_editor_event<S, A>(
    editor_engine: &mut EditorEngine,
    editor_buffer: &mut EditorBuffer,
    editor_event: EditorEvent,
    _shared_tw_data: &SharedTWData,
    _component_registry: &mut ComponentRegistry<S, A>,
    _self_id: FlexBoxIdType,
  ) where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    match editor_event {
      EditorEvent::InsertChar(character) => EditorEngineDataApi::insert_str_at_caret(
        EditorArgsMut {
          editor_buffer,
          editor_engine,
        },
        &String::from(character),
      ),
      EditorEvent::InsertNewLine => {
        EditorEngineDataApi::insert_new_line_at_caret(EditorArgsMut {
          editor_buffer,
          editor_engine,
        });
      }
      EditorEvent::Delete => {
        EditorEngineDataApi::delete_at_caret(editor_buffer, editor_engine);
      }
      EditorEvent::Backspace => {
        EditorEngineDataApi::backspace_at_caret(editor_buffer, editor_engine);
      }
      EditorEvent::MoveCaret(direction) => {
        match direction {
          CaretDirection::Left => EditorEngineDataApi::left(editor_buffer, editor_engine),
          CaretDirection::Right => EditorEngineDataApi::right(editor_buffer, editor_engine),
          CaretDirection::Up => EditorEngineDataApi::up(editor_buffer, editor_engine),
          CaretDirection::Down => EditorEngineDataApi::down(editor_buffer, editor_engine),
        };
      }
      EditorEvent::InsertString(chunk) => EditorEngineDataApi::insert_str_at_caret(
        EditorArgsMut {
          editor_buffer,
          editor_engine,
        },
        &chunk,
      ),
      EditorEvent::Resize(_) => {
        // Check to see whether scroll is valid.
        EditorEngineDataApi::validate_scroll(EditorArgsMut {
          editor_buffer,
          editor_engine,
        });
      }
      EditorEvent::Home => {
        EditorEngineDataApi::home(editor_buffer, editor_engine);
      }
      EditorEvent::End => {
        EditorEngineDataApi::end(editor_buffer, editor_engine);
      }
      EditorEvent::PageDown => {
        EditorEngineDataApi::page_down(editor_buffer, editor_engine);
      }
      EditorEvent::PageUp => {
        EditorEngineDataApi::page_up(editor_buffer, editor_engine);
      }
    };
  }

  pub fn apply_editor_events<S, A>(
    editor_engine: &mut EditorEngine,
    editor_buffer: &mut EditorBuffer,
    editor_event_vec: Vec<EditorEvent>,
    shared_tw_data: &SharedTWData,
    component_registry: &mut ComponentRegistry<S, A>,
    self_id: FlexBoxIdType,
  ) where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    for editor_event in editor_event_vec {
      EditorEvent::apply_editor_event(
        editor_engine,
        editor_buffer,
        editor_event,
        shared_tw_data,
        component_registry,
        self_id,
      );
    }
  }
}

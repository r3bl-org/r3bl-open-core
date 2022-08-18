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

use crossterm::event::*;
use serde::{Deserialize, Serialize};

// FIXME: convert crossterm::MouseEvent -> MouseInput
use crate::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub struct MouseInput {
  pub pos: Position,
  pub kind: MouseInputKind,
  pub maybe_modifier_keys: Option<ModifierKeys>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum MouseInputKind {
  MouseDown(MouseButton),
  MouseUp(MouseButton),
  MouseMove,
  MouseDrag(MouseButton),
  ScrollUp,
  ScrollDown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum MouseButton {
  Left,
  Right,
  Middle,
}

impl From<MouseEvent> for MouseButton {
  fn from(mouse_event: MouseEvent) -> Self {
    let pos: Position = (mouse_event.column, mouse_event.row).into();
    let maybe_modifier_keys: Option<ModifierKeys> = convert_key_modifiers(&mouse_event.modifiers);

    // FIXME: handle MouseEventKind enum

    todo!()
  }
}

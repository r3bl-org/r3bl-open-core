/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::{try_convert_key_modifiers, ModifierKeysMask};
use crate::{col, row, Pos};

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub struct MouseInput {
    pub pos: Pos,
    pub kind: MouseInputKind,
    pub maybe_modifier_keys: Option<ModifierKeysMask>,
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum MouseInputKind {
    MouseDown(Button),
    MouseUp(Button),
    MouseMove,
    MouseDrag(Button),
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum Button {
    Left,
    Right,
    Middle,
}

impl From<MouseEvent> for MouseInput {
    fn from(mouse_event: MouseEvent) -> Self {
        let pos = col(mouse_event.column) + row(mouse_event.row);
        let maybe_modifier_keys = try_convert_key_modifiers(&mouse_event.modifiers);
        let kind: MouseInputKind = mouse_event.kind.into();
        MouseInput {
            pos,
            kind,
            maybe_modifier_keys,
        }
    }
}

impl From<MouseEventKind> for MouseInputKind {
    fn from(mouse_event_kind: MouseEventKind) -> Self {
        match mouse_event_kind {
            MouseEventKind::Down(button) => MouseInputKind::MouseDown(button.into()),
            MouseEventKind::Up(button) => MouseInputKind::MouseUp(button.into()),
            MouseEventKind::Moved => MouseInputKind::MouseMove,
            MouseEventKind::Drag(button) => MouseInputKind::MouseDrag(button.into()),
            MouseEventKind::ScrollUp => MouseInputKind::ScrollUp,
            MouseEventKind::ScrollDown => MouseInputKind::ScrollDown,
            MouseEventKind::ScrollLeft => MouseInputKind::ScrollDown,
            MouseEventKind::ScrollRight => MouseInputKind::ScrollRight,
        }
    }
}

impl From<MouseButton> for Button {
    fn from(mouse_button: MouseButton) -> Self {
        match mouse_button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        }
    }
}

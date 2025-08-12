// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::{ModifierKeysMask, try_convert_key_modifiers};
use crate::{Pos, col, row};

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
            MouseEventKind::ScrollLeft => MouseInputKind::ScrollLeft,
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

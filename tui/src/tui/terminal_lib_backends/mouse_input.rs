// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::{ModifierKeysMask, try_convert_key_modifiers};
use crate::{Pos, col, row};

/// Represents a mouse input event in the terminal.
///
/// This struct captures all the essential information about a mouse interaction,
/// including the position where it occurred, the type of mouse event, and any
/// modifier keys that were held during the event.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{MouseInput, MouseInputKind, Button, Pos, col, row};
///
/// let mouse_click = MouseInput {
///     pos: col(10) + row(5),
///     kind: MouseInputKind::MouseDown(Button::Left),
///     maybe_modifier_keys: None,
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub struct MouseInput {
    /// The position in the terminal where the mouse event occurred.
    pub pos: Pos,
    /// The specific type of mouse event (click, move, scroll, etc.).
    pub kind: MouseInputKind,
    /// Optional modifier keys (Ctrl, Alt, Shift) held during the event.
    pub maybe_modifier_keys: Option<ModifierKeysMask>,
}

/// Represents different types of mouse input events.
///
/// This enum covers all the mouse interactions that can occur in a terminal,
/// from basic clicks and movements to scroll wheel actions.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{MouseInputKind, Button};
///
/// // Left mouse button press
/// let click = MouseInputKind::MouseDown(Button::Left);
///
/// // Mouse movement without buttons pressed
/// let movement = MouseInputKind::MouseMove;
///
/// // Vertical scroll up
/// let scroll = MouseInputKind::ScrollUp;
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum MouseInputKind {
    /// Mouse button pressed down at a position.
    MouseDown(Button),
    /// Mouse button released at a position.
    MouseUp(Button),
    /// Mouse cursor moved without any buttons pressed.
    MouseMove,
    /// Mouse moved while a button is held down (dragging).
    MouseDrag(Button),
    /// Scroll wheel moved up (away from user).
    ScrollUp,
    /// Scroll wheel moved down (toward user).
    ScrollDown,
    /// Horizontal scroll to the left.
    ScrollLeft,
    /// Horizontal scroll to the right.
    ScrollRight,
}

/// Represents mouse buttons that can be pressed.
///
/// This enum covers the standard mouse buttons supported by most terminals.
/// All buttons support press, release, and drag operations.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::Button;
///
/// let primary = Button::Left;    // Primary button (usually left)
/// let secondary = Button::Right; // Secondary button (context menu)
/// let tertiary = Button::Middle; // Middle button (often scroll wheel click)
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum Button {
    /// Left mouse button (primary button for most users).
    Left,
    /// Right mouse button (typically opens context menus).
    Right,
    /// Middle mouse button (often the scroll wheel when pressed).
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

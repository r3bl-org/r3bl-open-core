// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Internal protocol types for VT-100 terminal input parsing.
//!
//! These types are used internally by the parser to represent protocol-level
//! events. The parser converts these to canonical [`InputEvent`] from [`terminal_io`].
//!
//! [`InputEvent`]: crate::terminal_io::InputEvent
//! [`terminal_io`]: mod@crate::terminal_io

use crate::{KeyState, TermPos};

/// Keyboard modifiers for input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VT100KeyModifiers {
    pub shift: KeyState,
    pub ctrl: KeyState,
    pub alt: KeyState,
}

impl VT100KeyModifiers {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shift: KeyState::NotPressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        }
    }
}

impl Default for VT100KeyModifiers {
    fn default() -> Self { Self::new() }
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100MouseButton {
    Left,
    Middle,
    Right,
    Unknown,
}

/// Scroll direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Paste mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100PasteMode {
    Start,
    End,
}

/// Internal protocol focus state (maps to canonical `FocusEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100FocusState {
    Gained,
    Lost,
}

/// Keyboard key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100KeyCode {
    /// Regular printable character.
    Char(char),
    /// Function keys F1-F12.
    Function(u8), // 1-12
    /// Arrow keys.
    Up,
    Down,
    Left,
    Right,
    /// Special navigation keys.
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    /// Whitespace keys.
    Tab,
    BackTab,
    Enter,
    /// Escape key.
    Escape,
    /// Backspace key.
    Backspace,
}

/// Mouse event actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100MouseAction {
    /// Mouse button pressed down.
    Press,
    /// Mouse button released.
    Release,
    /// Mouse moved while button held (drag).
    Drag,
    /// Mouse moved without buttons.
    Motion,
    /// Scroll wheel rotated.
    Scroll(VT100ScrollDirection),
}

/// Internal protocol event from VT-100 parsing.
///
/// This is an intermediate representation used during parsing.
/// It gets converted to the canonical `InputEvent` from `terminal_io`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VT100InputEvent {
    /// Keyboard event with character, modifiers, and key code.
    Keyboard {
        code: VT100KeyCode,
        modifiers: VT100KeyModifiers,
    },
    /// Mouse event with button, position, and action.
    Mouse {
        button: VT100MouseButton,
        pos: TermPos,
        action: VT100MouseAction,
        modifiers: VT100KeyModifiers,
    },
    /// Terminal resize event with new dimensions.
    Resize { rows: u16, cols: u16 },
    /// Terminal focus event (gained or lost).
    Focus(VT100FocusState),
    /// Paste mode notification (start or end).
    Paste(VT100PasteMode),
}

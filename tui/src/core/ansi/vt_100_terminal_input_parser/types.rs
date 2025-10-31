// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input event types for VT-100 terminal input parsing.
//!
//! These types are protocol-agnostic and represent the high-level events
//! that result from parsing ANSI sequences and UTF-8 text.

use crate::TermPos;

/// Keyboard modifiers for input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyModifiers {
    pub fn new() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
        }
    }
}

impl Default for KeyModifiers {
    fn default() -> Self { Self::new() }
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Unknown,
}

/// Scroll direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Paste mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    Start,
    End,
}

/// Focus event state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusState {
    Gained,
    Lost,
}

/// Keyboard key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
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
pub enum MouseAction {
    /// Mouse button pressed down.
    Press,
    /// Mouse button released.
    Release,
    /// Mouse moved while button held (drag).
    Drag,
    /// Mouse moved without buttons.
    Motion,
    /// Scroll wheel rotated.
    Scroll(ScrollDirection),
}

/// High-level input events from terminal input.
///
/// These are the result of parsing ANSI sequences and UTF-8 text.
/// They are backend-agnostic and can be used by any terminal application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// Keyboard event with character, modifiers, and key code.
    Keyboard {
        code: KeyCode,
        modifiers: KeyModifiers,
    },
    /// Mouse event with button, position, and action.
    Mouse {
        button: MouseButton,
        pos: TermPos,
        action: MouseAction,
        modifiers: KeyModifiers,
    },
    /// Terminal resize event with new dimensions.
    Resize { rows: u16, cols: u16 },
    /// Terminal focus event (gained or lost).
    Focus(FocusState),
    /// Paste mode notification (start or end).
    Paste(PasteMode),
}

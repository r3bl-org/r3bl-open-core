// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared Protocol Types for VT-100 Terminal Input Parsing
//!
//! This module defines the data structures that flow through the parsing pipeline.
//! All specialized parsers (keyboard, mouse, terminal_events, utf8) produce
//! [`VT100InputEvent`] instances built from these types.
//!
//! ## Where You Are in the Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  types.rs - Foundation Layer                            │  ← **YOU ARE HERE**
//! │  • VT100InputEvent (output of all parsers)              │
//! │  • VT100KeyCode, VT100KeyModifiers (keyboard)           │
//! │  • VT100MouseButton, VT100MouseAction (mouse)           │
//! │  • VT100FocusState, VT100PasteMode (terminal events)    │
//! │  • VT100ScrollDirection (scroll wheel)                  │
//! └─────────────────────────────────────────────────────────┘
//!                             ▲
//!                             │ (types used by all modules)
//!         ┌───────────────────┼───────────────────┐
//!         │                   │                   │
//!     parser.rs          keyboard.rs          mouse.rs
//!   (routing)          (CSI/SS3)        (SGR/X10/RXVT)
//!                  terminal_events.rs     utf8.rs
//!                   (resize/focus)       (text)
//! ```
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`parser`], [`keyboard`], [`mouse`], [`terminal_events`], [`utf8`] -
//!   Modules using these types
//! - 🔧 **Backend**: [`DirectToAnsiInputDevice`] - Converts VT100InputEvent to InputEvent
//! - 📚 **Canonical Types**: [`InputEvent`], [`Key`], [`MouseInput`] - Final user-facing
//!   types
//!
//! ## Type Conversion Flow
//!
//! ```text
//! Raw bytes → Parser → VT100InputEvent → DirectToAnsiInputDevice → InputEvent
//!                      (protocol layer)   (conversion layer)        (canonical)
//! ```
//!
//! These types are **internal protocol representations**. The backend I/O layer
//! converts them to canonical types from [`terminal_io`] before exposing to users.
//!
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`InputEvent`]: crate::terminal_io::InputEvent
//! [`Key`]: crate::Key
//! [`MouseInput`]: crate::MouseInput
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`parser`]: mod@super::parser
//! [`terminal_events`]: mod@super::terminal_events
//! [`terminal_io`]: mod@crate::terminal_io
//! [`utf8`]: mod@super::utf8

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

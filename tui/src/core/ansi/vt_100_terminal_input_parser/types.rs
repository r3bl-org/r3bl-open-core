// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input event types for VT-100 terminal input parsing.
//!
//! These types are protocol-agnostic and represent the high-level events
//! that result from parsing ANSI sequences and UTF-8 text.

use crate::{TermCol, TermRow};
use std::num::NonZeroU16;

/// Represents a position (column, row) on the terminal using [1-based coordinates].
///
/// [1-based coordinates]: mod@super#one-based-mouse-input-events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: TermCol,
    pub row: TermRow,
}

impl Pos {
    /// Construct a terminal position from raw 1-based coordinate values.
    ///
    /// This is the primary constructor for ANSI sequence parsing where coordinates
    /// are received as raw `u16` values that are known to be 1-based and non-zero.
    ///
    /// This is similar to [`parse_cursor_position`].
    ///
    /// # Panics
    ///
    /// Panics if either coordinate is zero (invalid VT-100 coordinate).
    ///
    /// [`parse_cursor_position`]: crate::core::ansi::vt_100_ansi_parser::parse_cursor_position
    #[must_use]
    pub fn from_one_based(col: u16, row: u16) -> Self {
        let col_nz = NonZeroU16::new(col).expect("Column must be non-zero (1-based)");
        let row_nz = NonZeroU16::new(row).expect("Row must be non-zero (1-based)");

        Self {
            col: TermCol::from_raw_non_zero_value(col_nz),
            row: TermRow::from_raw_non_zero_value(row_nz),
        }
    }
}

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
        pos: Pos,
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

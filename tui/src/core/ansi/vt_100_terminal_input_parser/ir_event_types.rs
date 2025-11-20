// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # VT-100 Input Event IR (Intermediate Representation)
//!
//! This module defines the **intermediate representation (IR) types** for VT-100
//! terminal input parsing. These types represent the protocol layer between raw ANSI
//! bytes and application-facing canonical types.
//!
//! ## Where You Are in the Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  ir_event_types - Foundation Layer                       │  ← **YOU ARE HERE**
//! │  • VT100InputEventIR (output of all parsers)             │
//! │  • VT100KeyCodeIR, VT100KeyModifiersIR (keyboard)        │
//! │  • VT100MouseButtonIR, VT100MouseActionIR (mouse)        │
//! │  • VT100FocusStateIR, VT100PasteModeIR (terminal events) │
//! │  • VT100ScrollDirectionIR (scroll wheel)                 │
//! └─────────────────────────▲────────────────────────────────┘
//!                           │ (types used by all modules)
//!       ┌───────────────────┼───────────────────┐
//!       │                   │                   │
//!   parser.rs           keyboard.rs           mouse.rs
//!   (routing)           (`CSI`/`SS3`)         (`SGR`/`X10`/`RXVT`)
//!                       terminal_events.rs    utf8.rs
//!                       (resize/focus)        (text)
//! ```
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`parser`], [`keyboard`], [`mouse`], [`terminal_events`], [`utf8`] -
//!   Modules using these types
//! - 🔧 **Backend**: [`DirectToAnsiInputDevice`] - Converts [`VT100InputEventIR`] to
//!   [`InputEvent`]
//! - 📚 **Canonical Types**: [`InputEvent`], [`Key`], [`MouseInput`] - Final user-facing
//!   types from [`terminal_io`]
//!
//! ## Why an IR Layer?
//!
//! The IR layer exists for four critical architectural reasons:
//!
//! ### 1. Backend Independence
//! Your public API ([`InputEvent`]) remains stable while backend protocols change.
//! If you add Windows Console API or another backend later, they convert *their*
//! IR to the same [`InputEvent`] without touching application code.
//!
//! ### 2. Protocol Quirk Absorption
//! VT-100 has quirks that shouldn't leak to applications:
//! - 1-based coordinates (humans count from 1, arrays use 0)
//! - Inconsistent mouse protocols (`SGR`, `X10`, `RXVT`)
//! - Modifier key encoding variations
//! - Escape sequence ambiguities (`ESC` vs arrow keys)
//!
//! The IR layer normalizes these quirks during conversion to canonical types.
//!
//! ### 3. Type Safety
//! Protocol types use VT-100 nomenclature ([`VT100KeyCodeIR`], [`VT100MouseButtonIR`]),
//! while canonical types use domain-appropriate names ([`Key`], [`Button`]).
//! Different types prevent accidental mixing of protocol details with domain logic.
//!
//! ### 4. Testability
//! You can test protocol parsing in isolation (bytes → IR) without terminal I/O,
//! and test application logic with mock canonical events.
//!
//! ## IR Types (Protocol Layer)
//!
//! All types prefixed with `VT100` are protocol-specific IR:
//! - [`VT100InputEventIR`] - Top-level IR event enum
//! - [`VT100KeyCodeIR`] - Keyboard key codes from VT-100 sequences
//! - [`VT100KeyModifiersIR`] - Modifier key states (shift, ctrl, alt)
//! - [`VT100MouseButtonIR`] - Mouse button identifiers
//! - [`VT100MouseActionIR`] - Mouse event types (press, drag, scroll, etc.)
//! - [`VT100ScrollDirectionIR`] - Scroll wheel directions
//! - [`VT100FocusStateIR`] - Focus gained/lost states
//! - [`VT100PasteModeIR`] - Bracketed paste markers
//!
//! ## Canonical Types (Public API)
//!
//! Applications should use these instead:
//! - [`InputEvent`] - Backend-agnostic input events
//! - [`Key`] - Keyboard keys with clean domain names
//! - [`KeyPress`] - Key with modifiers
//! - [`MouseInput`] - Mouse events with 0-based coordinates
//! - [`FocusEvent`] - Focus events
//!
//! ## Type Conversion Flow
//!
//! ```text
//! Raw ANSI bytes
//!      ↓ (parser.rs, keyboard.rs, mouse.rs, etc.)
//! VT100InputEventIR (IR)  ← YOU ARE HERE
//!      ↓ (protocol_conversion.rs)
//! InputEvent (canonical)
//!      ↓
//! Application code
//! ```
//!
//! [`Button`]: crate::Button
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`FocusEvent`]: crate::FocusEvent
//! [`InputEvent`]: crate::InputEvent
//! [`KeyPress`]: crate::KeyPress
//! [`Key`]: crate::Key
//! [`MouseInput`]: crate::MouseInput
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`parser`]: mod@super::parser
//! [`terminal_events`]: mod@super::terminal_events
//! [`terminal_io`]: enum@crate::terminal_io::InputEvent
//! [`utf8`]: mod@super::utf8

use crate::{ColWidth, RowHeight, TermPos, terminal_io::KeyState};

/// Internal protocol event from VT-100 parsing.
///
/// This is an intermediate representation used during parsing.
/// It gets converted to the canonical [`InputEvent`] from [`terminal_io`].
///
/// [`InputEvent`]: crate::terminal_io::InputEvent
/// [`terminal_io`]: enum@crate::terminal_io::InputEvent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VT100InputEventIR {
    /// Keyboard event with character, modifiers, and key code.
    Keyboard {
        code: VT100KeyCodeIR,
        modifiers: VT100KeyModifiersIR,
    },
    /// Mouse event with button, position, and action.
    Mouse {
        button: VT100MouseButtonIR,
        pos: TermPos,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    },
    /// Terminal resize event with new dimensions.
    ///
    /// The [`col_width`] and [`row_height`] represent terminal dimensions as
    /// counts (1-based), not indices. A terminal with 80 columns has 80 total
    /// columns to display text.
    ///
    /// [`col_width`]: crate::ColWidth
    /// [`row_height`]: crate::RowHeight
    Resize {
        col_width: ColWidth,
        row_height: RowHeight,
    },
    /// Terminal focus event (gained or lost).
    Focus(VT100FocusStateIR),
    /// Paste mode notification (start or end).
    Paste(VT100PasteModeIR),
}

/// Keyboard modifiers for input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VT100KeyModifiersIR {
    pub shift: KeyState,
    pub ctrl: KeyState,
    pub alt: KeyState,
}

impl VT100KeyModifiersIR {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shift: KeyState::NotPressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        }
    }
}

impl Default for VT100KeyModifiersIR {
    fn default() -> Self { Self::new() }
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100MouseButtonIR {
    Left,
    Middle,
    Right,
    Unknown,
}

/// Scroll direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100ScrollDirectionIR {
    Up,
    Down,
    Left,
    Right,
}

/// Paste mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100PasteModeIR {
    Start,
    End,
}

/// Internal protocol focus state (maps to canonical `FocusEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100FocusStateIR {
    Gained,
    Lost,
}

/// Keyboard key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100KeyCodeIR {
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
pub enum VT100MouseActionIR {
    /// Mouse button pressed down.
    Press,
    /// Mouse button released.
    Release,
    /// Mouse moved while button held (drag).
    Drag,
    /// Mouse moved without buttons.
    Motion,
    /// Scroll wheel rotated.
    Scroll(VT100ScrollDirectionIR),
}

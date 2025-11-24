// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module defines the **intermediate representation (IR) types** for VT-100
//! terminal input parsing. See [`VT100InputEventIR`] for architecture details and
//! documentation.

use crate::{ColWidth, RowHeight, TermPos, terminal_io::KeyState};

/// Internal protocol event from VT-100 parsing.
///
/// This is the **intermediate representation (IR)** - the output of all parsers in this
/// module. These types represent the protocol layer between raw ANSI bytes and
/// application-facing canonical types.
///
/// ## Where This Type Fits in the Architecture
///
/// For the full data flow, see the [parent module documentation]. This diagram shows
/// how this module [`ir_event_types`] serves as the foundation layer:
///
/// ```text
/// ┌─────────────────────────────────────────────────────────┐  ┌──────────────────┐
/// │ Foundation Layer                                        ◀──┤ **YOU ARE HERE** │
/// │ • VT100InputEventIR (output of all parsers)             │  └──────────────────┘
/// │ • VT100KeyCodeIR, VT100KeyModifiersIR (keyboard)        │
/// │ • VT100MouseButtonIR, VT100MouseActionIR (mouse)        │
/// │ • VT100FocusStateIR, VT100PasteModeIR (terminal events) │
/// │ • VT100ScrollDirectionIR (scroll wheel)                 │
/// └────────────────────────▲────────────────────────────────┘
///                          │ (types used by all modules)
///      ┌───────────────────┼───────────────────┐
///      │                   │                   │
///  router.rs           keyboard.rs           mouse.rs
///  (routing)           (`CSI`/`SS3`)         (`SGR`/`X10`/`RXVT`)
///                      terminal_events.rs    utf8.rs
///                      (resize/focus)        (text)
/// ```
///
/// **Navigate**:
/// - ⬆️ **Used by**: [`router`], [`keyboard`], [`mouse`], [`terminal_events`], [`utf8`]
/// - ⬇️ **Converted by**: [`convert_input_event()`] in `protocol_conversion.rs` (not this
///   module)
///
/// ## Why an IR Layer?
///
/// The IR layer exists for four critical architectural reasons:
///
/// - Backend Independence - The public API ([`InputEvent`]) remains stable while backend
///   protocols change. If we add Windows Console API or another backend later, we can
///   convert *that* IR to the same [`InputEvent`] without touching application code.
///
/// - Protocol Quirk Absorption - VT-100 has quirks that shouldn't leak to applications.
///   The IR layer normalizes these quirks during conversion to canonical types:
///   - VT-100 uses 1-based coordinates, canonical types use 0-based.
///   - Multiple mouse protocols (`SGR`, `X10`, `RXVT`) with different encodings.
///   - Tab/Enter/Backspace send same bytes as Ctrl+I/Ctrl+M/Ctrl+H.
///   - `ESC` key and escape sequences (like arrow keys) both start with `0x1B`.
///
/// - Type Safety - Protocol types use VT-100 nomenclature ([`VT100KeyCodeIR`],
///   [`VT100MouseButtonIR`]), while canonical types use domain-appropriate names
///   ([`Key`], [`Button`]). Different types prevent accidental mixing of protocol details
///   with domain logic.
///
/// - Testability - We can test protocol parsing in isolation (bytes → IR) without
///   terminal I/O, and test application logic with mock canonical events.
///
/// ## IR to Canonical Conversion
///
/// This module only defines IR types. The actual conversion to canonical types happens in
/// [`convert_input_event()`] within `protocol_conversion.rs` in the `direct_to_ansi`
/// terminal backend. It is the responsibility of each terminal backend to convert its IR
/// types to canonical types.
///
/// [`Button`]: crate::Button
/// [`ir_event_types`]: mod@super::ir_event_types
/// [`convert_input_event()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::protocol_conversion::convert_input_event
/// [`InputEvent`]: crate::InputEvent
/// [`Key`]: crate::Key
/// [`keyboard`]: mod@super::keyboard
/// [`mouse`]: mod@super::mouse
/// [`router`]: mod@super::router
/// [`terminal_events`]: mod@super::terminal_events
/// [`utf8`]: mod@super::utf8
/// [parent module documentation]: mod@super#primary-consumer
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

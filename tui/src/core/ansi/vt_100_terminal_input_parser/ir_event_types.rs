// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module defines the **intermediate representation (IR) types** for [`VT-100`]
//! terminal input parsing. See [`VT100InputEventIR`] for architecture details and
//! documentation.
//!
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

use crate::{ByteOffset, OSC_CODE_COLOR_REPORT_BACKGROUND, OSC_CODE_COLOR_REPORT_CURSOR,
            OSC_CODE_COLOR_REPORT_FOREGROUND, OSC_CODE_COLOR_REPORT_HIGHLIGHT,
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND,
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND,
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND, RgbValue, TermPos, VPHeight,
            VPWidth, terminal_io::KeyState};
use std::{fmt::{Display, Formatter},
          str::FromStr};

/// Internal protocol event from [`VT-100`] parsing.
///
/// This is the **intermediate representation (IR)** - the output of all parsers in this
/// module. These types represent the protocol layer between raw [`ANSI`] bytes and
/// application-facing canonical types.
///
/// ## Where This Type Fits in the Architecture
///
/// For the full data flow, see the [parent module documentation]. This diagram shows how
/// this module [`ir_event_types`] serves as the foundation layer:
///
/// ```text
/// ┌──────────────────────────────────────────────────────────┐  ┌──────────────────┐
/// │ Foundation Layer                                         ◄──┤ **YOU ARE HERE** │
/// │ • VT100InputEventIR (output of all parsers)              │  └──────────────────┘
/// │ • VT100KeyCodeIR, VT100KeyModifiersIR (keyboard)         │
/// │ • VT100MouseButtonIR, VT100MouseActionIR (mouse)         │
/// │ • VT100FocusStateIR, VT100PasteModeIR (terminal events)  │
/// │ • TerminalColorReport, TerminalColorRole (color reports) │
/// │ • VT100ScrollDirectionIR (scroll wheel)                  │
/// └────────────────────────▲─────────────────────────────────┘
///                          │ (types used by all modules)
///      ┌───────────────────┼───────────────────┐
///      │                   │                   │
///  chunk_decoder       keyboard/             mouse.rs
///  (decoder)           (`CSI`/`SS3`)         (`SGR`/`X10`/`RXVT`)
///                      terminal_events.rs    utf8.rs
///                      (resize/focus/paste/  (text)
///                       color)
/// ```
///
/// **Navigate**:
/// - ⬆️ **Used by**: [`chunk_decoder`], [`keyboard`], [`mouse`], [`terminal_events`],
///   [`utf8`]
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
/// - Protocol Quirk Absorption - [`VT-100`] has quirks that shouldn't leak to
///   applications. The IR layer normalizes these quirks during conversion to canonical
///   types:
///   - [`VT-100`] uses 1-based coordinates, canonical types use 0-based.
///   - Multiple mouse protocols ([`SGR`], [`X10`], [`RXVT`]) with different encodings.
///   - Tab/Enter/Backspace send same bytes as `Ctrl+I`/`Ctrl+M`/`Ctrl+H`.
///   - [`ESC`] key and escape sequences (like arrow keys) both start with `0x1B`.
///
/// - Type Safety - Protocol types use [`VT-100`] nomenclature ([`VT100KeyCodeIR`],
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
/// [`convert_input_event()`] within [`protocol_conversion`] in the [`direct_to_ansi`]
/// terminal backend. It is the responsibility of each terminal backend to convert its IR
/// types to canonical types.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`Button`]: crate::Button
/// [`chunk_decoder`]: mod@super::chunk_decoder
/// [`convert_input_event()`]: crate::direct_to_ansi::input::protocol_conversion::convert_input_event
/// [`direct_to_ansi`]: mod@crate::direct_to_ansi
/// [`ESC`]: crate::EscSequence
/// [`InputEvent`]: crate::InputEvent
/// [`ir_event_types`]: mod@super::ir_event_types
/// [`Key`]: crate::Key
/// [`keyboard`]: mod@super::chunk_decoder::keyboard
/// [`mouse`]: mod@super::chunk_decoder::mouse
/// [`protocol_conversion`]: mod@crate::direct_to_ansi::input::protocol_conversion
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`terminal_events`]: mod@super::chunk_decoder::terminal_events
/// [`utf8`]: mod@super::chunk_decoder::utf8
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
/// [parent module documentation]: mod@crate::vt_100_terminal_input_parser
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
    /// [`col_width`]: crate::VPWidth
    /// [`row_height`]: crate::VPHeight
    Resize {
        col_width: VPWidth,
        row_height: VPHeight,
    },

    /// Terminal focus event (gained or lost).
    Focus(VT100FocusStateIR),

    /// Paste mode notification (start or end).
    Paste(VT100PasteModeIR),

    /// Terminal foreground or background color query report ([`OSC`] 10 or 11).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    ColorReport(TerminalColorReport),

    /// Protocol control sequence that is recognized and consumed, but does not generate
    /// an application-level input event (such as framed terminal query responses like
    /// [`OSC`] 52 or unhandled commands).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Ignored,
}

/// Result of parsing an input byte sequence into an intermediate representation event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInputEventIR {
    /// The parsed intermediate representation event.
    pub event: VT100InputEventIR,
    /// Total bytes consumed from the buffer.
    pub bytes_consumed: ByteOffset,
}

impl ParsedInputEventIR {
    #[must_use]
    pub const fn new(event: VT100InputEventIR, bytes_consumed: ByteOffset) -> Self {
        Self {
            event,
            bytes_consumed,
        }
    }

    #[must_use]
    pub fn consumed_usize(&self) -> usize { self.bytes_consumed.as_usize() }
}

/// Keyboard modifiers for input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VT100KeyModifiersIR {
    pub shift: KeyState,
    pub ctrl: KeyState,
    pub alt: KeyState,
}

impl VT100KeyModifiersIR {
    /// No modifier keys pressed.
    pub const NONE: Self = Self {
        shift: KeyState::NotPressed,
        ctrl: KeyState::NotPressed,
        alt: KeyState::NotPressed,
    };

    /// Only Shift pressed.
    pub const SHIFT: Self = Self {
        shift: KeyState::Pressed,
        ctrl: KeyState::NotPressed,
        alt: KeyState::NotPressed,
    };

    /// Only Alt pressed.
    pub const ALT: Self = Self {
        shift: KeyState::NotPressed,
        ctrl: KeyState::NotPressed,
        alt: KeyState::Pressed,
    };

    /// Only Ctrl pressed.
    pub const CTRL: Self = Self {
        shift: KeyState::NotPressed,
        ctrl: KeyState::Pressed,
        alt: KeyState::NotPressed,
    };

    /// Create with no modifiers pressed (same as [`Self::NONE`]).
    #[must_use]
    pub const fn new() -> Self { Self::NONE }

    /// Returns a copy with Shift marked as pressed.
    #[must_use]
    pub const fn with_shift(mut self) -> Self {
        self.shift = KeyState::Pressed;
        self
    }

    /// Returns a copy with Ctrl marked as pressed.
    #[must_use]
    pub const fn with_ctrl(mut self) -> Self {
        self.ctrl = KeyState::Pressed;
        self
    }

    /// Returns a copy with Alt marked as pressed.
    #[must_use]
    pub const fn with_alt(mut self) -> Self {
        self.alt = KeyState::Pressed;
        self
    }
}

impl Default for VT100KeyModifiersIR {
    fn default() -> Self { Self::NONE }
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

/// Internal protocol focus state (maps to canonical [`FocusEvent`]).
///
/// [`FocusEvent`]: crate::FocusEvent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VT100FocusStateIR {
    Gained,
    Lost,
}

/// Structured terminal color response decoded from an [`OSC`] 10, 11, 12, 13, 14, 17, or
/// 19 sequence.
///
/// Contains the reported [`RgbValue`] and the functional [`TerminalColorRole`] it applies
/// to.
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalColorReport {
    pub role: TerminalColorRole,
    pub color: RgbValue,
}

/// Functional role for a terminal color report sequence ([`OSC`] 10, 11, 12, 13, 14, 17,
/// or 19).
///
/// # Omitted Sequences ([`OSC`] 15, 16, and 18)
///
/// In the [XTerm Control Sequences] specification for dynamic window colors ([`OSC`]
/// 10-19), parameter codes 15, 16, and 18 are historically allocated to Tektronix 4014
/// mode:
/// - [`OSC`] 15: Tektronix foreground color.
/// - [`OSC`] 16: Tektronix background color.
/// - [`OSC`] 18: Tektronix cursor color.
///
/// Because modern terminal emulators do not implement Tektronix vector graphics
/// emulation, these sequences are obsolete and omitted here. Only the [`VT-100`] text,
/// cursor, pointer, and highlight color roles are supported.
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [XTerm Control Sequences]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalColorRole {
    /// Foreground color report ([`OSC`] 10).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Foreground,

    /// Background color report ([`OSC`] 11).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Background,

    /// Text cursor color report ([`OSC`] 12).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Cursor,

    /// Mouse pointer foreground color report ([`OSC`] 13).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    MouseForeground,

    /// Mouse pointer background color report ([`OSC`] 14).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    MouseBackground,

    /// Highlight / selection background color report ([`OSC`] 17).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Highlight,

    /// Highlight / selection foreground color report ([`OSC`] 19).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    HighlightForeground,
}

impl TerminalColorRole {
    /// Returns the [`OSC`] parameter code for this color role.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Foreground => OSC_CODE_COLOR_REPORT_FOREGROUND,
            Self::Background => OSC_CODE_COLOR_REPORT_BACKGROUND,
            Self::Cursor => OSC_CODE_COLOR_REPORT_CURSOR,
            Self::MouseForeground => OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND,
            Self::MouseBackground => OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND,
            Self::Highlight => OSC_CODE_COLOR_REPORT_HIGHLIGHT,
            Self::HighlightForeground => OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND,
        }
    }
}

impl FromStr for TerminalColorRole {
    type Err = ();

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        match code {
            OSC_CODE_COLOR_REPORT_FOREGROUND => Ok(Self::Foreground),
            OSC_CODE_COLOR_REPORT_BACKGROUND => Ok(Self::Background),
            OSC_CODE_COLOR_REPORT_CURSOR => Ok(Self::Cursor),
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND => Ok(Self::MouseForeground),
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND => Ok(Self::MouseBackground),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT => Ok(Self::Highlight),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND => Ok(Self::HighlightForeground),
            _ => Err(()),
        }
    }
}

impl Display for TerminalColorRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_modifiers_constants_and_builders() {
        assert_eq!(VT100KeyModifiersIR::default(), VT100KeyModifiersIR::NONE);
        assert_eq!(VT100KeyModifiersIR::new(), VT100KeyModifiersIR::NONE);

        assert_eq!(
            VT100KeyModifiersIR::SHIFT,
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::NotPressed,
            }
        );

        assert_eq!(
            VT100KeyModifiersIR::ALT,
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::Pressed,
            }
        );

        assert_eq!(
            VT100KeyModifiersIR::CTRL,
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
                alt: KeyState::NotPressed,
            }
        );

        assert_eq!(
            VT100KeyModifiersIR::NONE
                .with_ctrl()
                .with_shift()
                .with_alt(),
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                ctrl: KeyState::Pressed,
                alt: KeyState::Pressed,
            }
        );

        assert_eq!(
            VT100KeyModifiersIR::CTRL.with_shift(),
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                ctrl: KeyState::Pressed,
                alt: KeyState::NotPressed,
            }
        );
    }

    #[test]
    fn test_terminal_color_role_display() {
        assert_eq!(
            TerminalColorRole::Foreground.as_str(),
            OSC_CODE_COLOR_REPORT_FOREGROUND
        );
        assert_eq!(
            TerminalColorRole::Background.as_str(),
            OSC_CODE_COLOR_REPORT_BACKGROUND
        );
        assert_eq!(
            TerminalColorRole::Cursor.as_str(),
            OSC_CODE_COLOR_REPORT_CURSOR
        );
        assert_eq!(
            TerminalColorRole::MouseForeground.as_str(),
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND
        );
        assert_eq!(
            TerminalColorRole::MouseBackground.as_str(),
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND
        );
        assert_eq!(
            TerminalColorRole::Highlight.as_str(),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT
        );
        assert_eq!(
            TerminalColorRole::HighlightForeground.as_str(),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND
        );

        assert_eq!(
            format!("{}", TerminalColorRole::Foreground),
            OSC_CODE_COLOR_REPORT_FOREGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::Background),
            OSC_CODE_COLOR_REPORT_BACKGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::Cursor),
            OSC_CODE_COLOR_REPORT_CURSOR
        );
        assert_eq!(
            format!("{}", TerminalColorRole::MouseForeground),
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::MouseBackground),
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND
        );
        assert_eq!(
            format!("{}", TerminalColorRole::Highlight),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT
        );
        assert_eq!(
            format!("{}", TerminalColorRole::HighlightForeground),
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND
        );
    }

    #[test]
    fn test_terminal_color_role_from_str() {
        assert_eq!(
            OSC_CODE_COLOR_REPORT_FOREGROUND.parse(),
            Ok(TerminalColorRole::Foreground)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_BACKGROUND.parse(),
            Ok(TerminalColorRole::Background)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_CURSOR.parse(),
            Ok(TerminalColorRole::Cursor)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND.parse(),
            Ok(TerminalColorRole::MouseForeground)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND.parse(),
            Ok(TerminalColorRole::MouseBackground)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_HIGHLIGHT.parse(),
            Ok(TerminalColorRole::Highlight)
        );
        assert_eq!(
            OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND.parse(),
            Ok(TerminalColorRole::HighlightForeground)
        );

        assert!("".parse::<TerminalColorRole>().is_err());
        assert!("0".parse::<TerminalColorRole>().is_err());
        assert!("20".parse::<TerminalColorRole>().is_err());
        assert!("invalid".parse::<TerminalColorRole>().is_err());
    }
}

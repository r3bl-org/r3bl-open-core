// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module defines the **intermediate representation (IR) types** for [`VT-100`]
//! terminal input parsing. See [`VT100InputEventIR`] for architecture details and
//! documentation.
//!
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

use crate::{ByteOffset, TermPos, TerminalColorReport, VPHeight, VPWidth,
            core::ansi::constants::{MODIFIER_ALT, MODIFIER_CTRL,
                                    MODIFIER_PARAMETER_OFFSET, MODIFIER_SHIFT},
            terminal_io::KeyState};

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
/// [parent module documentation]: mod@crate::core::ansi::vt_100_terminal_input_parser
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
    /// [`OSC`]: crate::OscSequence
    TerminalColorReport(TerminalColorReport),

    /// Protocol control sequence that is recognized and consumed, but does not generate
    /// an application-level input event (such as framed terminal query responses like
    /// [`OSC`] 52 or unhandled commands).
    ///
    /// [`OSC`]: crate::OscSequence
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

    #[cfg(test)]
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

    /// Checks if any modifier keys ([`shift`], [`ctrl`], or [`alt`]) are pressed.
    ///
    /// [`alt`]: Self::alt
    /// [`ctrl`]: Self::ctrl
    /// [`shift`]: Self::shift
    #[must_use]
    pub const fn has_modifiers(self) -> bool {
        matches!(self.shift, KeyState::Pressed)
            || matches!(self.ctrl, KeyState::Pressed)
            || matches!(self.alt, KeyState::Pressed)
    }

    /// Extracts the raw modifier bitmask following the [`VT-100`] [`ANSI`] convention:
    /// `Shift(1) | Alt(2) | Ctrl(4)`.
    ///
    /// Where bitfield flags are:
    /// - bit 0 (value 1): [`MODIFIER_SHIFT`]
    /// - bit 1 (value 2): [`MODIFIER_ALT`]
    /// - bit 2 (value 4): [`MODIFIER_CTRL`]
    ///
    /// This provides exact symmetry with the inbound sequence decoder, which subtracts
    /// [`MODIFIER_PARAMETER_OFFSET`] to recover the original modifier bitmask.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`MODIFIER_ALT`]: crate::core::ansi::constants::MODIFIER_ALT
    /// [`MODIFIER_CTRL`]: crate::core::ansi::constants::MODIFIER_CTRL
    /// [`MODIFIER_PARAMETER_OFFSET`]: crate::core::ansi::constants::MODIFIER_PARAMETER_OFFSET
    /// [`MODIFIER_SHIFT`]: crate::core::ansi::constants::MODIFIER_SHIFT
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[must_use]
    pub const fn to_modifier_bits(self) -> u8 {
        let mut mask: u8 = 0;
        if let KeyState::Pressed = self.shift {
            mask |= MODIFIER_SHIFT;
        }
        if let KeyState::Pressed = self.alt {
            mask |= MODIFIER_ALT;
        }
        if let KeyState::Pressed = self.ctrl {
            mask |= MODIFIER_CTRL;
        }
        mask
    }

    /// Converts modifier keys into the [`CSI`] wire parameter: `1 + bitfield`.
    ///
    /// ## Parameter Values
    ///
    /// - `1`: no modifiers (1 + 0)
    /// - `2`: Shift (1 + 1)
    /// - `3`: Alt (1 + 2)
    /// - `4`: Alt+Shift (1 + 3)
    /// - `5`: Ctrl (1 + 4)
    /// - `6`: Ctrl+Shift (1 + 5)
    /// - `7`: Ctrl+Alt (1 + 6)
    /// - `8`: Ctrl+Alt+Shift (1 + 7)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub const fn to_csi_parameter(self) -> u8 {
        self.to_modifier_bits() + MODIFIER_PARAMETER_OFFSET
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
    fn test_vt100_key_modifiers_ir_methods() {
        let none = VT100KeyModifiersIR::NONE;
        assert!(!none.has_modifiers());
        assert_eq!(none.to_modifier_bits(), 0);
        assert_eq!(none.to_csi_parameter(), 1);

        let shift = VT100KeyModifiersIR::SHIFT;
        assert!(shift.has_modifiers());
        assert_eq!(shift.to_modifier_bits(), MODIFIER_SHIFT);
        assert_eq!(shift.to_csi_parameter(), 2);

        let alt = VT100KeyModifiersIR::ALT;
        assert!(alt.has_modifiers());
        assert_eq!(alt.to_modifier_bits(), MODIFIER_ALT);
        assert_eq!(alt.to_csi_parameter(), 3);

        let ctrl = VT100KeyModifiersIR::CTRL;
        assert!(ctrl.has_modifiers());
        assert_eq!(ctrl.to_modifier_bits(), MODIFIER_CTRL);
        assert_eq!(ctrl.to_csi_parameter(), 5);

        let all = VT100KeyModifiersIR::CTRL.with_alt().with_shift();
        assert!(all.has_modifiers());
        assert_eq!(
            all.to_modifier_bits(),
            MODIFIER_SHIFT | MODIFIER_ALT | MODIFIER_CTRL
        );
        assert_eq!(all.to_csi_parameter(), 8);
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`] escape sequence generator for terminal INPUT (test fixtures).
//!
//! Provides input sequence generation for testing. Creates symmetry with [`ansi_output`]
//! for output sequences.
//!
//! ## Purpose
//!
//! **This module is for testing only.** It is not used in production code.
//!
//! The generator enables:
//! 1. **Round-trip validation**: Parse [`ANSI`] → [`InputEvent`] → Generate [`ANSI`] →
//!    Verify match
//! 2. **Test helpers**: Build test sequences without hardcoding raw bytes
//! 3. **Parser verification**: Confirm parsers handle all modifier combinations correctly
//!
//! ## Available Items
//!
//! - Pre-computed constants:
//!   - [`SEQ_ARROW_UP`], [`SEQ_ARROW_DOWN`], [`SEQ_ARROW_RIGHT`], [`SEQ_ARROW_LEFT`]
//!   - [`SEQ_HOME`], [`SEQ_END`], [`SEQ_BACKTAB`]
//!   - [`SEQ_F1`], [`SEQ_F2`], [`SEQ_F3`], [`SEQ_F4`]
//! - Low-level builder functions:
//!   - [`csi`], [`ss3`], [`csi_tilde`], [`csi_modified`]
//! - High-level generators:
//!   - [`generate_keyboard_sequence`]
//!   - [`generate_mouse_sequence_bytes`]
//!   - [`generate_resize_sequence`]
//!   - [`generate_focus_sequence`]
//!   - [`generate_paste_sequence`]
//!
//! [`ansi_output`]: crate::core::ansi::generator::ansi_output
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`InputEvent`]: crate::InputEvent

use crate::{KeyState, LossyConvertToByte, NarrowingCastToU8, OscSequence,
            core::ansi::{constants::{ANSI_CSI_U_CHAR,
                                     ANSI_FUNCTION_KEY_TERMINATOR_CHAR,
                                     ANSI_PARAM_SEPARATOR_CHAR,
                                     ANSI_SUBPARAM_SEPARATOR_CHAR, ASCII_DEL,
                                     CONTROL_CHAR_MASK, CONTROL_ENTER, CONTROL_NUL,
                                     CONTROL_TAB, CSI_START, ESC_STR,
                                     FOCUS_GAINED_FINAL_CHAR, FOCUS_LOST_FINAL_CHAR,
                                     FUNCTION_F1_CODE, FUNCTION_F2_CODE,
                                     FUNCTION_F3_CODE, FUNCTION_F4_CODE,
                                     FUNCTION_F5_CODE, FUNCTION_F6_CODE,
                                     FUNCTION_F7_CODE, FUNCTION_F8_CODE,
                                     FUNCTION_F9_CODE, FUNCTION_F10_CODE,
                                     FUNCTION_F11_CODE, FUNCTION_F12_CODE,
                                     MODIFIER_ALT, MODIFIER_PARAMETER_BASE_CHAR_VAL,
                                     MODIFIER_PARAMETER_OFFSET,
                                     MOUSE_LEFT_BUTTON_CODE, MOUSE_MIDDLE_BUTTON_CODE,
                                     MOUSE_MODIFIER_ALT, MOUSE_MODIFIER_CTRL,
                                     MOUSE_MODIFIER_SHIFT, MOUSE_MOTION_FLAG,
                                     MOUSE_RELEASE_BUTTON_CODE,
                                     MOUSE_RIGHT_BUTTON_CODE,
                                     MOUSE_SCROLL_DOWN_BUTTON,
                                     MOUSE_SCROLL_LEFT_BUTTON,
                                     MOUSE_SCROLL_RIGHT_BUTTON,
                                     MOUSE_SCROLL_UP_BUTTON, MOUSE_SGR_MARKER_CHAR,
                                     MOUSE_SGR_PRESS_CHAR, MOUSE_SGR_RELEASE_CHAR,
                                     MOUSE_X10_COORD_OFFSET, MOUSE_X10_MARKER_CHAR,
                                     MOUSE_X10_PREFIX, OSC_DELIMITER, OSC_START,
                                     OSC_TERMINATOR_ST, PASTE_END_GENERATE_CODE,
                                     PASTE_START_GENERATE_CODE,
                                     RESIZE_EVENT_CODE_CHAR, RESIZE_TERMINATOR_CHAR,
                                     SPECIAL_DELETE_CODE, SPECIAL_INSERT_CODE,
                                     SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE,
                                     UTF8_CHAR_MAX_BYTES},
                         vt_100_terminal_input_parser::{TerminalColorReport,
                                                        VT100FocusStateIR,
                                                        VT100InputEventIR,
                                                        VT100KeyCodeIR,
                                                        VT100KeyModifiersIR,
                                                        VT100MouseActionIR,
                                                        VT100MouseButtonIR,
                                                        VT100PasteModeIR}},
            input_sequences::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_SS3_O, ARROW_DOWN_FINAL,
                              ARROW_LEFT_FINAL, ARROW_RIGHT_FINAL, ARROW_UP_FINAL,
                              BACKTAB_FINAL, SPECIAL_END_FINAL, SPECIAL_HOME_FINAL,
                              SS3_F1_FINAL, SS3_F2_FINAL, SS3_F3_FINAL, SS3_F4_FINAL}};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PRE-COMPUTED SEQUENCE CONSTANTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Arrow Up: `ESC [ A`.
pub const SEQ_ARROW_UP: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_UP_FINAL];

/// Arrow Down: `ESC [ B`.
pub const SEQ_ARROW_DOWN: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_DOWN_FINAL];

/// Arrow Right: `ESC [ C`.
pub const SEQ_ARROW_RIGHT: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_RIGHT_FINAL];

/// Arrow Left: `ESC [ D`.
pub const SEQ_ARROW_LEFT: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_LEFT_FINAL];

/// Home: `ESC [ H`.
pub const SEQ_HOME: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, SPECIAL_HOME_FINAL];

/// End: `ESC [ F`.
pub const SEQ_END: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, SPECIAL_END_FINAL];

/// [`BackTab`] (Shift+Tab): `ESC [ Z`.
///
/// [`BackTab`]: VT100KeyCodeIR::BackTab
pub const SEQ_BACKTAB: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, BACKTAB_FINAL];

/// F1 (SS3): `ESC O P`.
pub const SEQ_F1: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F1_FINAL];

/// F2 (SS3): `ESC O Q`.
pub const SEQ_F2: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F2_FINAL];

/// F3 (SS3): `ESC O R`.
pub const SEQ_F3: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F3_FINAL];

/// F4 (SS3): `ESC O S`.
pub const SEQ_F4: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F4_FINAL];

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// LOW-LEVEL BUILDER FUNCTIONS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Builds a 3-byte [`CSI`] sequence: `ESC [ <final>`.
///
/// Encodes an unparameterized [`CSI`] sequence consisting of [`ANSI_ESC`],
/// [`ANSI_CSI_BRACKET`], and the trailing `final_byte` (e.g. `ESC [ A` for cursor up).
///
/// [`ANSI_CSI_BRACKET`]: crate::core::ansi::constants::ANSI_CSI_BRACKET
/// [`ANSI_ESC`]: crate::core::ansi::constants::ANSI_ESC
/// [`CSI`]: crate::CsiSequence
#[must_use]
pub const fn csi(final_byte: u8) -> [u8; 3] { [ANSI_ESC, ANSI_CSI_BRACKET, final_byte] }

/// Builds an [`SS3`] (Single Shift Select 3) sequence: `ESC O <final>`.
///
/// In [`ECMA-48`] and [`VT-100`] terminals, [`SS3`] sequences are generated by unshifted
/// function keys `F1` through `F4` (e.g. [`SEQ_F1`]), as well as numeric keypad keys in
/// application mode.
///
/// [`ECMA-48`]: https://ecma-international.org/publications-and-standards/standards/ecma-48/
/// [`SEQ_F1`]: SEQ_F1
/// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#FS,_GS,_RS,_US
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[must_use]
pub const fn ss3(final_byte: u8) -> [u8; 3] { [ANSI_ESC, ANSI_SS3_O, final_byte] }

/// Builds [`CSI`] tilde sequence: `ESC [ <code> ~`.
///
/// Used for function keys (F5+), Insert, Delete, [`PageUp`], [`PageDown`].
///
/// [`CSI`]: crate::CsiSequence
/// [`PageDown`]: VT100KeyCodeIR::PageDown
/// [`PageUp`]: VT100KeyCodeIR::PageUp
#[must_use]
pub fn csi_tilde(code: u16) -> Vec<u8> {
    format!("{CSI_START}{code}{ANSI_FUNCTION_KEY_TERMINATOR_CHAR}").into_bytes()
}

/// Builds [`CSI`] modified sequence: `ESC [ 1 ; <modifier parameter> <final>`.
///
/// Encodes an [`XTerm`]-style modified navigation key (arrows, `Home`, `End`) or function
/// key into the standard wire format.
///
/// ## Format Structure
///
/// - Leading `'1'` ([`MODIFIER_PARAMETER_BASE_CHAR_VAL`]): Base parameter representing
///   the default count/prefix for modified navigation keys in the [`XTerm`]
///   specification.
/// - Semicolon ([`ANSI_PARAM_SEPARATOR_CHAR`]): Delimits the base parameter from the
///   modifier.
/// - Modifier parameter: Calculated as `modifier_bits + MODIFIER_PARAMETER_OFFSET`.
/// - Final byte: Terminating [`ASCII`] character byte (e.g. `b'H'` for `Home`, `b'A'` for
///   `Up`).
///
/// ## Modifier Encoding and Parameter Offset
///
/// In the [`XTerm`] specification, modifier parameters are encoded with a base offset of
/// 1:
///
/// ```text
/// wire_parameter = modifier_bits + MODIFIER_PARAMETER_OFFSET
/// ```
///
/// The raw modifier bits are additive bitwise flags:
/// - [`MODIFIER_SHIFT`] = `1` (`0b001`)
/// - [`MODIFIER_ALT`] = `2` (`0b010`)
/// - [`MODIFIER_CTRL`] = `4` (`0b100`)
///
/// Adding [`MODIFIER_PARAMETER_OFFSET`] (value `1`) maps the bitfield into the wire
/// parameter format:
/// - `1 + 1 = 2`: Shift (e.g. `Shift+Home` generates `ESC [ 1 ; 2 H`)
/// - `2 + 1 = 3`: Alt (e.g. `Alt+Up` generates `ESC [ 1 ; 3 A`)
/// - `4 + 1 = 5`: Ctrl (e.g. `Ctrl+Left` generates `ESC [ 1 ; 5 D`)
///
/// This provides exact symmetry with the inbound sequence decoder, which subtracts
/// [`MODIFIER_PARAMETER_OFFSET`] to recover the original modifier bitmask.
///
/// [`ANSI_PARAM_SEPARATOR_CHAR`]: crate::core::ansi::constants::ANSI_PARAM_SEPARATOR_CHAR
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CSI`]: crate::CsiSequence
/// [`MODIFIER_ALT`]: crate::core::ansi::constants::MODIFIER_ALT
/// [`MODIFIER_CTRL`]: crate::core::ansi::constants::MODIFIER_CTRL
/// [`MODIFIER_PARAMETER_BASE_CHAR_VAL`]: crate::core::ansi::constants::MODIFIER_PARAMETER_BASE_CHAR_VAL
/// [`MODIFIER_PARAMETER_OFFSET`]: crate::core::ansi::constants::MODIFIER_PARAMETER_OFFSET
/// [`MODIFIER_SHIFT`]: crate::core::ansi::constants::MODIFIER_SHIFT
/// [`XTerm`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
#[must_use]
pub fn csi_modified(modifier: u8, final_byte: u8) -> Vec<u8> {
    let param = modifier + MODIFIER_PARAMETER_OFFSET;
    let final_byte = char::from(final_byte); // eg: 72u8 as 'H' - final_byte is always a single ASCII byte
    format!(
        "{CSI_START}{MODIFIER_PARAMETER_BASE_CHAR_VAL}{ANSI_PARAM_SEPARATOR_CHAR}{param}{final_byte}",
    )
    .into_bytes()
}

/// Generate a [`Kitty`] `CSI u` sequence: `ESC [ <codepoint> ; <modifier parameter> u`.
///
/// Encodes a character and modifier combination into the enhanced keyboard sequence
/// format specified by the [`Kitty Keyboard Protocol`].
///
/// ## Modifier Encoding and Parameter Offset
///
/// In the [`Kitty`] and [`XTerm`] specifications, modifier parameters are encoded with a
/// base offset of 1:
///
/// ```text
/// wire_parameter = modifier_bits + MODIFIER_PARAMETER_OFFSET
/// ```
///
/// The raw modifier bits are additive bitwise flags:
/// - [`MODIFIER_SHIFT`] = `1` (`0b001`)
/// - [`MODIFIER_ALT`] = `2` (`0b010`)
/// - [`MODIFIER_CTRL`] = `4` (`0b100`)
///
/// Adding [`MODIFIER_PARAMETER_OFFSET`] (value `1`) maps the bitfield into the wire
/// parameter format:
/// - `0 + 1 = 1`: No modifiers
/// - `1 + 1 = 2`: Shift
/// - `2 + 1 = 3`: Alt (e.g. `Alt+[` generates `ESC [ 91 ; 3 u`)
/// - `4 + 1 = 5`: Ctrl
///
/// This provides exact symmetry with the inbound sequence decoder, which subtracts
/// [`MODIFIER_PARAMETER_OFFSET`] to recover the original modifier bitmask.
///
/// [`Kitty Keyboard Protocol`]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`MODIFIER_ALT`]: crate::core::ansi::constants::MODIFIER_ALT
/// [`MODIFIER_CTRL`]: crate::core::ansi::constants::MODIFIER_CTRL
/// [`MODIFIER_PARAMETER_OFFSET`]: crate::core::ansi::constants::MODIFIER_PARAMETER_OFFSET
/// [`MODIFIER_SHIFT`]: crate::core::ansi::constants::MODIFIER_SHIFT
/// [`XTerm`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
#[must_use]
pub fn csi_u(ch: char, modifier: u8) -> Vec<u8> {
    csi_u_codepoint(u32::from(ch), modifier)
}

/// Generate a [`Kitty`] Keyboard Protocol sequence for a [`Unicode`] codepoint:
/// `CSI <codepoint> ; <modifier> u`.
///
/// Encodes the key according to the [Kitty Keyboard Protocol] specification.
/// The `modifier` byte is offset by [`MODIFIER_PARAMETER_OFFSET`] to produce the
/// parameter.
///
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`MODIFIER_PARAMETER_OFFSET`]: crate::core::ansi::constants::MODIFIER_PARAMETER_OFFSET
/// [`Unicode`]: https://en.wikipedia.org/wiki/Unicode
/// [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
#[must_use]
pub fn csi_u_codepoint(codepoint: u32, modifier: u8) -> Vec<u8> {
    let param = modifier + MODIFIER_PARAMETER_OFFSET;
    format!("{CSI_START}{codepoint}{ANSI_PARAM_SEPARATOR_CHAR}{param}{ANSI_CSI_U_CHAR}")
        .into_bytes()
}

/// Generate a [`Kitty`] Keyboard Protocol sequence with an event type:
/// `CSI <codepoint> ; <modifier> : <event_type> u`.
///
/// Event types:
/// - 1: Press
/// - 2: Repeat
/// - 3: Release
///
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`MODIFIER_PARAMETER_OFFSET`]: crate::core::ansi::constants::MODIFIER_PARAMETER_OFFSET
#[must_use]
pub fn csi_u_with_event_type(codepoint: u32, modifier: u8, event_type: u8) -> Vec<u8> {
    let param = modifier + MODIFIER_PARAMETER_OFFSET;
    format!("{CSI_START}{codepoint}{ANSI_PARAM_SEPARATOR_CHAR}{param}{ANSI_SUBPARAM_SEPARATOR_CHAR}{event_type}{ANSI_CSI_U_CHAR}")
        .into_bytes()
}

/// Generate a [`Kitty`] Keyboard Protocol sequence with an alternate key subparameter:
/// `CSI <codepoint> : <alternate_key> ; <modifier> u`.
///
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`MODIFIER_PARAMETER_OFFSET`]: crate::core::ansi::constants::MODIFIER_PARAMETER_OFFSET
#[must_use]
pub fn csi_u_with_alternate_key(
    codepoint: u32,
    alternate_key: u32,
    modifier: u8,
) -> Vec<u8> {
    let param = modifier + MODIFIER_PARAMETER_OFFSET;
    format!("{CSI_START}{codepoint}{ANSI_SUBPARAM_SEPARATOR_CHAR}{alternate_key}{ANSI_PARAM_SEPARATOR_CHAR}{param}{ANSI_CSI_U_CHAR}")
        .into_bytes()
}

/// Generate an Alt-modified character sequence: `ESC <character>`.
///
/// Encodes the character as [`UTF-8`] following the [`ANSI_ESC`] byte.
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn alt_key(ch: char) -> Vec<u8> { format!("{ESC_STR}{ch}").into_bytes() }

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HIGH-LEVEL GENERATORS: Main entry points for generating ANSI sequences
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Generates [`ANSI`] bytes for an input event.
///
/// Converts any input event back into the [`ANSI`] [`CSI`] sequence format that terminals
/// send. This enables round-trip validation: [`InputEvent`] → bytes → parse →
/// [`InputEvent`].
///
/// ## Supported Events
///
/// - **Keyboard**: All key codes with modifiers (arrows, function keys, special keys)
/// - **Resize**: Window resize notifications (`CSI 8 ; rows ; cols t`)
/// - **Focus**: Focus gained/lost events (`CSI I` / `CSI O`)
/// - **Paste**: [`Bracketed paste`] mode (`CSI 200~` / `CSI 201~`)
/// - **Mouse**: [`SGR`] mouse format (`CSI < button ; col ; row M/m`)
///
/// ## Returns
///
/// - [`Some(Vec<u8>)`] for recognized events
/// - [`None`] for unsupported or invalid combinations
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`Bracketed paste`]: https://en.wikipedia.org/wiki/Bracketed-paste
/// [`CSI`]: crate::CsiSequence
/// [`InputEvent`]: crate::InputEvent
/// [`SGR`]: crate::SgrCode
/// [`Some(Vec<u8>)`]: std::option::Option::Some
#[must_use]
pub fn generate_keyboard_sequence(event: &VT100InputEventIR) -> Option<Vec<u8>> {
    match event {
        VT100InputEventIR::Keyboard { code, modifiers } => {
            keyboard::generate_key_sequence(*code, *modifiers)
        }
        VT100InputEventIR::Resize {
            col_width,
            row_height,
        } => {
            let rows = row_height.as_u16();
            let cols = col_width.as_u16();
            Some(terminal_events::generate_resize_sequence(rows, cols))
        }
        VT100InputEventIR::Focus(state) => {
            Some(terminal_events::generate_focus_sequence(*state))
        }
        VT100InputEventIR::Paste(mode) => {
            Some(terminal_events::generate_paste_sequence(*mode))
        }
        VT100InputEventIR::Mouse {
            button,
            pos,
            action,
            modifiers,
        } => Some(mouse::generate_sgr_sequence(
            *button,
            pos.col.as_u16(),
            pos.row.as_u16(),
            *action,
            *modifiers,
        )),
        VT100InputEventIR::TerminalColorReport(report) => {
            Some(terminal_events::generate_color_report_sequence(*report))
        }
        VT100InputEventIR::Ignored => None,
    }
}

/// Generate [`ANSI`] bytes for a mouse event in [`X10`] format.
///
/// Generates sequences like: `ESC [ M Cb Cx Cy` (6 bytes)
///
/// ## [`X10`] Format Details
///
/// - `Cb` = button byte: button code (0-2) + modifier flags + motion flag
/// - `Cx` = column byte: `actual_column` + 32 ([`ASCII`] offset encoding)
/// - `Cy` = row byte: `actual_row` + 32 ([`ASCII`] offset encoding)
///
/// Button encoding:
/// - Bits 0-1: Button (0=left, 1=middle, 2=right, 3=release)
/// - Bit 2: Shift modifier (4)
/// - Bit 3: Alt modifier (8)
/// - Bit 4: Ctrl modifier (16)
/// - Bit 5: Motion flag (32)
///
/// ## Arguments
///
/// - `button`: Mouse button
/// - `col`: Column coordinate ([1-based])
/// - `row`: Row coordinate ([1-based])
/// - `action`: Press, Release, Motion, or Drag
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
///
/// [1-based]: crate::vt_100_terminal_input_parser::chunk_decoder::mouse#verifying-coordinate-systems
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
#[must_use]
pub fn generate_x10_mouse_sequence(
    button: VT100MouseButtonIR,
    col: u16,
    row: u16,
    action: VT100MouseActionIR,
    modifiers: VT100KeyModifiersIR,
) -> Vec<u8> {
    mouse::generate_x10_sequence(button, col, row, action, modifiers)
}

/// Generate [`ANSI`] bytes for a mouse event in [`RXVT`] format.
///
/// Generates sequences like: `ESC [ Cb ; Cx ; Cy M` (variable length)
///
/// ## [`RXVT`] Format Details
///
/// Uses decimal numbers separated by semicolons (human-readable format):
/// - `Cb` = button code (decimal): button (0-2) + modifier bits
/// - `Cx` = column coordinate (decimal, [1-based])
/// - `Cy` = row coordinate (decimal, [1-based])
/// - Final: `M` for press
///
/// ## Arguments
///
/// - `button`: Mouse button
/// - `col`: Column coordinate ([1-based])
/// - `row`: Row coordinate ([1-based])
/// - `action`: Press or Release ([`RXVT`] primarily uses Press)
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
///
/// [1-based]: crate::vt_100_terminal_input_parser::chunk_decoder::mouse#verifying-coordinate-systems
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
#[must_use]
pub fn generate_rxvt_mouse_sequence(
    button: VT100MouseButtonIR,
    col: u16,
    row: u16,
    action: VT100MouseActionIR,
    modifiers: VT100KeyModifiersIR,
) -> Vec<u8> {
    mouse::generate_rxvt_sequence(button, col, row, action, modifiers)
}

/// Generate [`ANSI`] bytes for a mouse event in [`SGR`] format.
///
/// Generates sequences like: `ESC [<button;col;rowM` or `ESC [<button;col;rowm`
///
/// ## Arguments
///
/// - `button`: Mouse button (0=left, 1=middle, 2=right, 64-67=scroll)
/// - `col`: Column coordinate ([1-based])
/// - `row`: Row coordinate ([1-based])
/// - `action`: Press, Release, or Drag
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
///
/// [1-based]: crate::vt_100_terminal_input_parser::chunk_decoder::mouse#verifying-coordinate-systems
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`SGR`]: crate::SgrCode
#[must_use]
pub fn generate_mouse_sequence_bytes(
    button: VT100MouseButtonIR,
    col: u16,
    row: u16,
    action: VT100MouseActionIR,
    modifiers: VT100KeyModifiersIR,
) -> Vec<u8> {
    mouse::generate_sgr_sequence(button, col, row, action, modifiers)
}

/// Generate a window resize sequence: `CSI 8 ; rows ; cols t`
///
/// This is the [`ANSI`] sequence sent by terminals when they are resized.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
#[must_use]
pub fn generate_resize_sequence(rows: u16, cols: u16) -> Vec<u8> {
    terminal_events::generate_resize_sequence(rows, cols)
}

/// Generate a focus event sequence.
///
/// - Focus gained: `CSI I`
/// - Focus lost: `CSI O`
#[must_use]
pub fn generate_focus_sequence(state: VT100FocusStateIR) -> Vec<u8> {
    terminal_events::generate_focus_sequence(state)
}

/// Generate a bracketed paste mode sequence.
///
/// - Paste start: `CSI 200 ~`
/// - Paste end: `CSI 201 ~`
#[must_use]
pub fn generate_paste_sequence(mode: VT100PasteModeIR) -> Vec<u8> {
    terminal_events::generate_paste_sequence(mode)
}

/// Generate a terminal color report sequence ([`OSC`] 10, 11, 12, 13, 14, 17, or 19).
///
/// Format: `ESC ] <role_code> ; rgb:rrrr/gggg/bbbb ESC \`
///
/// Supported role codes:
/// - `10`: Text foreground
/// - `11`: Text background
/// - `12`: Text cursor
/// - `13`: Mouse pointer foreground
/// - `14`: Mouse pointer background
/// - `17`: Highlight / selection background
/// - `19`: Highlight / selection foreground
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[must_use]
pub fn generate_color_report_sequence(report: TerminalColorReport) -> Vec<u8> {
    terminal_events::generate_color_report_sequence(report)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PRIVATE MODULES: Implementation details organized by functionality
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Keyboard sequence generation (arrows, function keys, special keys, characters).
mod keyboard {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Generate [`ANSI`] bytes for a specific key code and modifiers.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    pub fn generate_key_sequence(
        code: VT100KeyCodeIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Option<Vec<u8>> {
        match code {
            // Arrow keys: CSI [1;mod] A/B/C/D
            VT100KeyCodeIR::Up => Some(generate_arrow_key(ARROW_UP_FINAL, modifiers)),
            VT100KeyCodeIR::Down => Some(generate_arrow_key(ARROW_DOWN_FINAL, modifiers)),
            VT100KeyCodeIR::Right => {
                Some(generate_arrow_key(ARROW_RIGHT_FINAL, modifiers))
            }
            VT100KeyCodeIR::Left => Some(generate_arrow_key(ARROW_LEFT_FINAL, modifiers)),

            // Navigation keys: CSI [1;mod] H/F
            VT100KeyCodeIR::Home => {
                Some(generate_arrow_key(SPECIAL_HOME_FINAL, modifiers))
            }
            VT100KeyCodeIR::End => Some(generate_arrow_key(SPECIAL_END_FINAL, modifiers)),

            // Special keys: CSI n [;mod] ~
            VT100KeyCodeIR::Insert => {
                Some(generate_tilde_key(SPECIAL_INSERT_CODE, modifiers))
            }
            VT100KeyCodeIR::Delete => {
                Some(generate_tilde_key(SPECIAL_DELETE_CODE, modifiers))
            }
            VT100KeyCodeIR::PageUp => {
                Some(generate_tilde_key(SPECIAL_PAGE_UP_CODE, modifiers))
            }
            VT100KeyCodeIR::PageDown => {
                Some(generate_tilde_key(SPECIAL_PAGE_DOWN_CODE, modifiers))
            }

            // Function keys: CSI n [;mod] ~
            VT100KeyCodeIR::Function(n) => generate_function_key(n, modifiers),

            // Raw byte keys (not CSI sequences)
            VT100KeyCodeIR::Tab => Some(vec![CONTROL_TAB]),
            VT100KeyCodeIR::BackTab => Some(generate_simple_csi(BACKTAB_FINAL)),
            VT100KeyCodeIR::Enter => Some(vec![CONTROL_ENTER]),
            VT100KeyCodeIR::Escape => Some(vec![ANSI_ESC]),
            VT100KeyCodeIR::Backspace => Some(vec![ASCII_DEL]),
            VT100KeyCodeIR::Char(c) => generate_char(c, modifiers),
        }
    }

    /// Generate arrow or navigation key sequence: `CSI [1;mod] final`.
    fn generate_arrow_key(final_byte: u8, modifiers: VT100KeyModifiersIR) -> Vec<u8> {
        if modifiers.has_modifiers() {
            csi_modified(modifiers.to_modifier_bits(), final_byte)
        } else {
            csi(final_byte).to_vec()
        }
    }

    /// Generate simple [`CSI`] sequence: `CSI final`.
    ///
    /// [`CSI`]: crate::CsiSequence
    fn generate_simple_csi(final_byte: u8) -> Vec<u8> { csi(final_byte).to_vec() }

    /// Generate tilde-terminated sequence: `CSI code [;mod] ~`.
    fn generate_tilde_key(code: u16, modifiers: VT100KeyModifiersIR) -> Vec<u8> {
        if modifiers.has_modifiers() {
            let param = modifiers.to_csi_parameter();
            format!(
                "{CSI_START}{code}\
                {ANSI_PARAM_SEPARATOR_CHAR}{param}\
                {ANSI_FUNCTION_KEY_TERMINATOR_CHAR}"
            )
            .into_bytes()
        } else {
            csi_tilde(code)
        }
    }

    /// Generate function key sequence (F1-F12).
    fn generate_function_key(n: u8, modifiers: VT100KeyModifiersIR) -> Option<Vec<u8>> {
        let code = match n {
            1 => FUNCTION_F1_CODE,
            2 => FUNCTION_F2_CODE,
            3 => FUNCTION_F3_CODE,
            4 => FUNCTION_F4_CODE,
            5 => FUNCTION_F5_CODE,
            6 => FUNCTION_F6_CODE,
            7 => FUNCTION_F7_CODE,
            8 => FUNCTION_F8_CODE,
            9 => FUNCTION_F9_CODE,
            10 => FUNCTION_F10_CODE,
            11 => FUNCTION_F11_CODE,
            12 => FUNCTION_F12_CODE,
            _ => return None,
        };
        Some(generate_tilde_key(code, modifiers))
    }

    /// Generate character sequence with modifier support.
    fn generate_char(c: char, modifiers: VT100KeyModifiersIR) -> Option<Vec<u8>> {
        // Alt+[: Kitty protocol `CSI u` sequence (`ESC [ 91 ; 3 u`) to avoid CSI
        // collision.
        if c == '['
            && modifiers.alt == KeyState::Pressed
            && modifiers.ctrl == KeyState::NotPressed
            && modifiers.shift == KeyState::NotPressed
        {
            return Some(csi_u('[', MODIFIER_ALT));
        }

        // Alt+letter: ESC + character
        if modifiers.alt == KeyState::Pressed
            && modifiers.ctrl == KeyState::NotPressed
            && modifiers.shift == KeyState::NotPressed
        {
            return Some(alt_key(c));
        }

        // Ctrl+letter: control byte (letter & CONTROL_CHAR_MASK)
        if modifiers.ctrl == KeyState::Pressed
            && modifiers.alt == KeyState::NotPressed
            && modifiers.shift == KeyState::NotPressed
        {
            if c.is_ascii_alphabetic() {
                let control_byte =
                    c.to_ascii_lowercase().to_u8_lossy() & CONTROL_CHAR_MASK;
                return Some(vec![control_byte]);
            }
            return None; // Ctrl+non-letter not supported
        }

        // Plain character: UTF-8 encoded
        let mut buf = [0u8; UTF8_CHAR_MAX_BYTES];
        let encoded = c.encode_utf8(&mut buf);
        Some(encoded.as_bytes().to_vec())
    }
}

/// Mouse sequence generation ([`X10`], [`RXVT`], [`SGR`] formats).
///
/// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
/// [`SGR`]: crate::SgrCode
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
mod mouse {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Generate mouse sequence in [`SGR`] format: `ESC [<button;col;rowM/m`.
    ///
    /// [`SGR`]: crate::SgrCode
    pub fn generate_sgr_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        let button_code = action_to_base_button_code(action, button);

        // Apply modifiers and action flags to button code
        let mut code = button_code;

        // Handle action/drag flag
        let action_char = match action {
            VT100MouseActionIR::Release => MOUSE_SGR_RELEASE_CHAR,
            VT100MouseActionIR::Drag | VT100MouseActionIR::Motion => {
                code |= MOUSE_MOTION_FLAG; // Motion/Drag flag (bit 5)
                MOUSE_SGR_PRESS_CHAR
            }
            VT100MouseActionIR::Press | VT100MouseActionIR::Scroll(_) => {
                MOUSE_SGR_PRESS_CHAR
            }
        };

        code = apply_modifiers(code, modifiers);

        // Build sequence: ESC[<button;col;rowM/m
        format!(
            "{CSI_START}{MOUSE_SGR_MARKER_CHAR}{code}{ANSI_PARAM_SEPARATOR_CHAR}{col}{ANSI_PARAM_SEPARATOR_CHAR}{row}{action_char}"
        )
        .into_bytes()
    }

    /// Generate mouse sequence in [`X10`] format: `ESC [ M Cb Cx Cy`.
    ///
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    pub fn generate_x10_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        let mut cb = action_to_base_button_code(action, button);

        // Handle action
        match action {
            VT100MouseActionIR::Release => cb = MOUSE_RELEASE_BUTTON_CODE,
            VT100MouseActionIR::Motion | VT100MouseActionIR::Drag => {
                cb |= MOUSE_MOTION_FLAG;
            }
            VT100MouseActionIR::Press | VT100MouseActionIR::Scroll(_) => {}
        }

        cb = apply_modifiers(cb, modifiers);

        // X10 coordinate encoding: add offset to make printable ASCII
        let cx = (col + MOUSE_X10_COORD_OFFSET).as_u8_narrowing();
        let cy = (row + MOUSE_X10_COORD_OFFSET).as_u8_narrowing();

        // Build sequence: ESC [ M Cb Cx Cy
        let mut bytes = MOUSE_X10_PREFIX.to_vec();
        bytes.push(cb.as_u8_narrowing());
        bytes.push(cx);
        bytes.push(cy);
        bytes.push(CONTROL_NUL);
        bytes
    }

    /// Generate mouse sequence in [`RXVT`] format: `ESC [ Cb ; Cx ; Cy M`.
    ///
    /// [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
    pub fn generate_rxvt_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        let mut cb = action_to_base_button_code(action, button);

        // Handle action
        match action {
            VT100MouseActionIR::Release => cb = MOUSE_RELEASE_BUTTON_CODE,
            VT100MouseActionIR::Motion | VT100MouseActionIR::Drag => {
                cb |= MOUSE_MOTION_FLAG;
            }
            VT100MouseActionIR::Press | VT100MouseActionIR::Scroll(_) => {}
        }

        cb = apply_modifiers(cb, modifiers);

        // Build sequence: ESC [ Cb ; Cx ; Cy M
        format!(
            "{CSI_START}{cb}\
            {ANSI_PARAM_SEPARATOR_CHAR}{col}\
            {ANSI_PARAM_SEPARATOR_CHAR}{row}\
            {MOUSE_X10_MARKER_CHAR}"
        )
        .into_bytes()
    }

    /// Converts button enum to protocol code.
    fn button_to_code(button: VT100MouseButtonIR) -> u16 {
        match button {
            VT100MouseButtonIR::Left => MOUSE_LEFT_BUTTON_CODE,
            VT100MouseButtonIR::Unknown => MOUSE_RELEASE_BUTTON_CODE,
            VT100MouseButtonIR::Middle => MOUSE_MIDDLE_BUTTON_CODE,
            VT100MouseButtonIR::Right => MOUSE_RIGHT_BUTTON_CODE,
        }
    }

    /// Converts a mouse action and button into its base protocol button code.
    ///
    /// This handles overriding the button code for scroll events (which map to 64+).
    fn action_to_base_button_code(
        action: VT100MouseActionIR,
        button: VT100MouseButtonIR,
    ) -> u16 {
        match action {
            VT100MouseActionIR::Scroll(scroll_dir) => {
                use crate::core::ansi::vt_100_terminal_input_parser::VT100ScrollDirectionIR;
                match scroll_dir {
                    VT100ScrollDirectionIR::Up => MOUSE_SCROLL_UP_BUTTON,
                    VT100ScrollDirectionIR::Down => MOUSE_SCROLL_DOWN_BUTTON,
                    VT100ScrollDirectionIR::Left => MOUSE_SCROLL_LEFT_BUTTON,
                    VT100ScrollDirectionIR::Right => MOUSE_SCROLL_RIGHT_BUTTON,
                }
            }
            _ => button_to_code(button),
        }
    }

    /// Applies modifier flags to button code.
    fn apply_modifiers(mut code: u16, modifiers: VT100KeyModifiersIR) -> u16 {
        if modifiers.shift == KeyState::Pressed {
            code |= MOUSE_MODIFIER_SHIFT;
        }
        if modifiers.alt == KeyState::Pressed {
            code |= MOUSE_MODIFIER_ALT;
        }
        if modifiers.ctrl == KeyState::Pressed {
            code |= MOUSE_MODIFIER_CTRL;
        }
        code
    }
}

/// Terminal event sequence generation (resize, focus, paste).
mod terminal_events {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Generate a window resize sequence: `CSI 8 ; rows ; cols t`.
    pub fn generate_resize_sequence(rows: u16, cols: u16) -> Vec<u8> {
        format!(
            "{CSI_START}{RESIZE_EVENT_CODE_CHAR}\
            {ANSI_PARAM_SEPARATOR_CHAR}{rows}\
            {ANSI_PARAM_SEPARATOR_CHAR}{cols}\
            {RESIZE_TERMINATOR_CHAR}"
        )
        .into_bytes()
    }

    /// Generate a focus event sequence: `CSI I` (gained) or `CSI O` (lost).
    pub fn generate_focus_sequence(state: VT100FocusStateIR) -> Vec<u8> {
        let focus_char = match state {
            VT100FocusStateIR::Gained => FOCUS_GAINED_FINAL_CHAR,
            VT100FocusStateIR::Lost => FOCUS_LOST_FINAL_CHAR,
        };
        format!("{CSI_START}{focus_char}").into_bytes()
    }

    /// Generate a bracketed paste sequence: `CSI 200 ~` (start) or `CSI 201 ~` (end).
    pub fn generate_paste_sequence(mode: VT100PasteModeIR) -> Vec<u8> {
        let code = match mode {
            VT100PasteModeIR::Start => PASTE_START_GENERATE_CODE,
            VT100PasteModeIR::End => PASTE_END_GENERATE_CODE,
        };
        format!("{CSI_START}{code}{ANSI_FUNCTION_KEY_TERMINATOR_CHAR}").into_bytes()
    }

    /// Generate an [`OSC`] color report sequence ([`OSC`] 10, 11, 12, 13, 14, 17, or 19):
    /// - Format: `ESC ] <role_code> ; rgb:rrrr/gggg/bbbb ESC \`
    ///
    /// Supported role codes:
    /// - `10`: Text foreground
    /// - `11`: Text background
    /// - `12`: Text cursor
    /// - `13`: Mouse pointer foreground
    /// - `14`: Mouse pointer background
    /// - `17`: Highlight / selection background
    /// - `19`: Highlight / selection foreground
    ///
    /// The X11 `rgb:rrrr/gggg/bbbb` color spec uses 16-bit hex values (4 digits each).
    /// Repeating each 8-bit channel byte (`{channel:02x}{channel:02x}`) scales `0xRR`
    /// to `0xRRRR`.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub fn generate_color_report_sequence(report: TerminalColorReport) -> Vec<u8> {
        OscSequence::ColorReport(report).to_string().into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RgbValue, TermPos, VPHeight, VPWidth,
                core::ansi::vt_100_terminal_input_parser::{TerminalColorRole,
                                                           VT100ScrollDirectionIR}};

    // ==================== Terminal Events ====================

    #[test]
    fn test_generate_resize_event() {
        let event = VT100InputEventIR::Resize {
            row_height: VPHeight::from(24),
            col_width: VPWidth::from(80),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[8;24;80t");
        assert_eq!(generate_resize_sequence(24, 80), b"\x1b[8;24;80t");
    }

    #[test]
    fn test_generate_focus_gained() {
        let event = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[I");
        assert_eq!(
            generate_focus_sequence(VT100FocusStateIR::Gained),
            b"\x1b[I"
        );
    }

    #[test]
    fn test_generate_focus_lost() {
        let event = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[O");
        assert_eq!(generate_focus_sequence(VT100FocusStateIR::Lost), b"\x1b[O");
    }

    #[test]
    fn test_generate_paste_start() {
        let event = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[200~");
        assert_eq!(
            generate_paste_sequence(VT100PasteModeIR::Start),
            b"\x1b[200~"
        );
    }

    #[test]
    fn test_generate_paste_end() {
        let event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[201~");
        assert_eq!(generate_paste_sequence(VT100PasteModeIR::End), b"\x1b[201~");
    }

    #[test]
    fn test_generate_color_report_foreground() {
        let report = TerminalColorReport {
            role: TerminalColorRole::Foreground,
            color: RgbValue::from_u8(0x1e, 0x2a, 0x3b),
        };
        let event = VT100InputEventIR::TerminalColorReport(report);
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b]10;rgb:1e1e/2a2a/3b3b\x1b\\");
        assert_eq!(
            generate_color_report_sequence(report),
            b"\x1b]10;rgb:1e1e/2a2a/3b3b\x1b\\"
        );
    }

    #[test]
    fn test_generate_color_report_background() {
        let report = TerminalColorReport {
            role: TerminalColorRole::Background,
            color: RgbValue::from_u8(0x00, 0xff, 0x7f),
        };
        let event = VT100InputEventIR::TerminalColorReport(report);
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b]11;rgb:0000/ffff/7f7f\x1b\\");
        assert_eq!(
            generate_color_report_sequence(report),
            b"\x1b]11;rgb:0000/ffff/7f7f\x1b\\"
        );
    }

    #[test]
    fn test_generate_color_report_other_roles() {
        let roles = [
            (TerminalColorRole::Cursor, "12"),
            (TerminalColorRole::MouseForeground, "13"),
            (TerminalColorRole::MouseBackground, "14"),
            (TerminalColorRole::Highlight, "17"),
            (TerminalColorRole::HighlightForeground, "19"),
        ];

        for (role, code) in roles {
            let report = TerminalColorReport {
                role,
                color: RgbValue::from_u8(0xaa, 0xbb, 0xcc),
            };
            let event = VT100InputEventIR::TerminalColorReport(report);
            let bytes = generate_keyboard_sequence(&event).expect("conversion error");
            let expected = format!(
                "{OSC_START}{code}{OSC_DELIMITER}rgb:aaaa/bbbb/cccc{OSC_TERMINATOR_ST}"
            )
            .into_bytes();
            assert_eq!(bytes, expected);
            assert_eq!(generate_color_report_sequence(report), expected);
        }
    }

    // ==================== Arrow Keys ====================

    #[test]
    fn test_generate_arrow_up() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Up,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[A");
    }

    #[test]
    fn test_generate_arrow_down() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Down,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[B");
    }

    #[test]
    fn test_generate_arrow_right() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Right,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[C");
    }

    #[test]
    fn test_generate_arrow_left() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Left,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[D");
    }

    // ==================== Arrow Keys with Modifiers ====================

    #[test]
    fn test_generate_shift_up() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Up,
            modifiers: VT100KeyModifiersIR::SHIFT,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        // Shift modifier: parameter = 1 + 1 = 2.
        assert_eq!(bytes, b"\x1b[1;2A");
    }

    #[test]
    fn test_generate_alt_right() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Right,
            modifiers: VT100KeyModifiersIR::ALT,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        // Alt modifier: parameter = 1 + 2 = 3.
        assert_eq!(bytes, b"\x1b[1;3C");
    }

    #[test]
    fn test_generate_ctrl_down() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Down,
            modifiers: VT100KeyModifiersIR::CTRL,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        // Ctrl modifier: parameter = 1 + 4 = 5.
        assert_eq!(bytes, b"\x1b[1;5B");
    }

    #[test]
    fn test_generate_ctrl_alt_shift_left() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Left,
            modifiers: VT100KeyModifiersIR::CTRL.with_alt().with_shift(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        // Shift+Alt+Ctrl modifiers: parameter = 1 + 7 = 8.
        assert_eq!(bytes, b"\x1b[1;8D");
    }

    // ==================== Special Navigation Keys ====================

    #[test]
    fn test_generate_home_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Home,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[H");
    }

    #[test]
    fn test_generate_end_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::End,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[F");
    }

    #[test]
    fn test_generate_shift_home() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Home,
            modifiers: VT100KeyModifiersIR::SHIFT,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[1;2H");
    }

    #[test]
    fn test_generate_ctrl_home() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Home,
            modifiers: VT100KeyModifiersIR::CTRL,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[1;5H");
    }

    #[test]
    fn test_generate_shift_end() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::End,
            modifiers: VT100KeyModifiersIR::SHIFT,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[1;2F");
    }

    #[test]
    fn test_generate_ctrl_end() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::End,
            modifiers: VT100KeyModifiersIR::CTRL,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[1;5F");
    }

    #[test]
    fn test_generate_insert_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Insert,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[2~");
    }

    #[test]
    fn test_generate_delete_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Delete,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[3~");
    }

    #[test]
    fn test_generate_page_up() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::PageUp,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[5~");
    }

    #[test]
    fn test_generate_page_down() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::PageDown,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[6~");
    }

    // ==================== Function Keys ====================

    #[test]
    fn test_generate_f1_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(1),
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[11~");
    }

    #[test]
    fn test_generate_f6_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(6),
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[17~");
    }

    #[test]
    fn test_generate_f12_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(12),
            modifiers: VT100KeyModifiersIR::default(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        assert_eq!(bytes, b"\x1b[24~");
    }

    #[test]
    fn test_generate_all_function_keys() {
        let expected_codes = [
            (1, b"\x1b[11~".as_slice()),
            (2, b"\x1b[12~"),
            (3, b"\x1b[13~"),
            (4, b"\x1b[14~"),
            (5, b"\x1b[15~"),
            (6, b"\x1b[17~"),
            (7, b"\x1b[18~"),
            (8, b"\x1b[19~"),
            (9, b"\x1b[20~"),
            (10, b"\x1b[21~"),
            (11, b"\x1b[23~"),
            (12, b"\x1b[24~"),
        ];
        for (n, expected) in expected_codes {
            let event = VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(n),
                modifiers: VT100KeyModifiersIR::default(),
            };
            let bytes = generate_keyboard_sequence(&event).expect("conversion error");
            assert_eq!(bytes, expected, "Failed for F{n}");
        }
    }

    #[test]
    fn test_generate_shift_f5() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(5),
            modifiers: VT100KeyModifiersIR::SHIFT,
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        // Shift modifier: parameter = 1 + 1 = 2.
        assert_eq!(bytes, b"\x1b[15;2~");
    }

    #[test]
    fn test_generate_ctrl_alt_f10() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(10),
            modifiers: VT100KeyModifiersIR::CTRL.with_alt(),
        };
        let bytes = generate_keyboard_sequence(&event).expect("conversion error");
        // Ctrl+Alt modifiers: parameter = 1 + 6 = 7.
        assert_eq!(bytes, b"\x1b[21;7~");
    }

    #[test]
    fn test_generate_invalid_function_key() {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(13),
            modifiers: VT100KeyModifiersIR::default(),
        };
        assert_eq!(generate_keyboard_sequence(&event), None);
    }

    // ==================== Raw Byte Key Generation ====================

    #[test]
    fn test_generate_raw_byte_keys() {
        // Tab: generates raw byte 0x09.
        let tab_event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Tab,
            modifiers: VT100KeyModifiersIR::default(),
        };
        assert_eq!(generate_keyboard_sequence(&tab_event), Some(vec![0x09]));

        // Enter: generates raw byte 0x0D (CR).
        let enter_event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Enter,
            modifiers: VT100KeyModifiersIR::default(),
        };
        assert_eq!(generate_keyboard_sequence(&enter_event), Some(vec![0x0D]));

        // Escape: generates raw byte 0x1B.
        let escape_event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Escape,
            modifiers: VT100KeyModifiersIR::default(),
        };
        assert_eq!(generate_keyboard_sequence(&escape_event), Some(vec![0x1B]));

        // Backspace: generates raw byte 0x7F (DEL).
        let backspace_event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Backspace,
            modifiers: VT100KeyModifiersIR::default(),
        };
        assert_eq!(
            generate_keyboard_sequence(&backspace_event),
            Some(vec![0x7F])
        );

        // BackTab: generates CSI Z sequence.
        let backtab_event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::BackTab,
            modifiers: VT100KeyModifiersIR::default(),
        };
        assert_eq!(
            generate_keyboard_sequence(&backtab_event),
            Some(vec![0x1B, b'[', b'Z'])
        );

        // Char: generates UTF-8 encoded bytes.
        let char_event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('H'),
            modifiers: VT100KeyModifiersIR::default(),
        };
        assert_eq!(generate_keyboard_sequence(&char_event), Some(vec![b'H']));
    }

    #[test]
    fn test_generate_ctrl_and_alt_chars() {
        // Ctrl+A (0x01).
        let ctrl_a = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('a'),
            modifiers: VT100KeyModifiersIR::CTRL,
        };
        assert_eq!(generate_keyboard_sequence(&ctrl_a), Some(vec![0x01]));

        // Ctrl+Z (0x1A).
        let ctrl_z = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('Z'),
            modifiers: VT100KeyModifiersIR::CTRL,
        };
        assert_eq!(generate_keyboard_sequence(&ctrl_z), Some(vec![0x1A]));

        // Ctrl+non-letter returns None.
        let ctrl_num = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('1'),
            modifiers: VT100KeyModifiersIR::CTRL,
        };
        assert_eq!(generate_keyboard_sequence(&ctrl_num), None);

        // Alt+a (ESC a).
        let alt_a = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('a'),
            modifiers: VT100KeyModifiersIR::ALT,
        };
        assert_eq!(generate_keyboard_sequence(&alt_a), Some(alt_key('a')));
        assert_eq!(alt_key('a'), vec![0x1B, b'a']);
        assert_eq!(alt_key(']'), vec![0x1B, b']']);

        // Alt+[ (Kitty protocol ESC [ 91 ; 3 u).
        let alt_bracket = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('['),
            modifiers: VT100KeyModifiersIR::ALT,
        };
        assert_eq!(
            generate_keyboard_sequence(&alt_bracket),
            Some(csi_u('[', MODIFIER_ALT))
        );
        assert_eq!(csi_u('[', MODIFIER_ALT), b"\x1b[91;3u");
    }

    // ==================== Mouse Generation ====================

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_generate_mouse_sgr() {
        // Left click press at (10, 20).
        let left_press = generate_mouse_sequence_bytes(
            VT100MouseButtonIR::Left,
            10,
            20,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(left_press, b"\x1b[<0;10;20M");

        // Left click release at (10, 20).
        let left_release = generate_mouse_sequence_bytes(
            VT100MouseButtonIR::Left,
            10,
            20,
            VT100MouseActionIR::Release,
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(left_release, b"\x1b[<0;10;20m");

        // Middle button press (button 1).
        let middle_press = generate_mouse_sequence_bytes(
            VT100MouseButtonIR::Middle,
            5,
            10,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(middle_press, b"\x1b[<1;5;10M");

        // Unknown button press (button 3).
        let unknown_press = generate_mouse_sequence_bytes(
            VT100MouseButtonIR::Unknown,
            5,
            10,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(
            unknown_press,
            format!(
                "{CSI_START}{MOUSE_SGR_MARKER_CHAR}3{ANSI_PARAM_SEPARATOR_CHAR}5{ANSI_PARAM_SEPARATOR_CHAR}10{MOUSE_SGR_PRESS_CHAR}"
            )
            .into_bytes()
        );

        // Drag with Shift modifier (button 0 + motion 32 + shift 4 = 36).
        let drag_shift = generate_mouse_sequence_bytes(
            VT100MouseButtonIR::Left,
            15,
            25,
            VT100MouseActionIR::Drag,
            VT100KeyModifiersIR::SHIFT,
        );
        assert_eq!(
            drag_shift,
            format!(
                "{CSI_START}{MOUSE_SGR_MARKER_CHAR}36{ANSI_PARAM_SEPARATOR_CHAR}15{ANSI_PARAM_SEPARATOR_CHAR}25{MOUSE_SGR_PRESS_CHAR}"
            )
            .into_bytes()
        );

        // Scroll Up (button 64).
        let scroll_up = generate_mouse_sequence_bytes(
            VT100MouseButtonIR::Unknown,
            5,
            8,
            VT100MouseActionIR::Scroll(VT100ScrollDirectionIR::Up),
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(
            scroll_up,
            format!(
                "{CSI_START}{MOUSE_SGR_MARKER_CHAR}64{ANSI_PARAM_SEPARATOR_CHAR}5{ANSI_PARAM_SEPARATOR_CHAR}8{MOUSE_SGR_PRESS_CHAR}"
            )
            .into_bytes()
        );

        // Scroll Down (65), Left (66), Right (67).
        for (dir, expected_btn) in [
            (VT100ScrollDirectionIR::Down, 65),
            (VT100ScrollDirectionIR::Left, 66),
            (VT100ScrollDirectionIR::Right, 67),
        ] {
            let scroll = generate_mouse_sequence_bytes(
                VT100MouseButtonIR::Unknown,
                5,
                8,
                VT100MouseActionIR::Scroll(dir),
                VT100KeyModifiersIR::default(),
            );
            assert_eq!(
                scroll,
                format!(
                    "{CSI_START}{MOUSE_SGR_MARKER_CHAR}{expected_btn}{ANSI_PARAM_SEPARATOR_CHAR}5{ANSI_PARAM_SEPARATOR_CHAR}8{MOUSE_SGR_PRESS_CHAR}"
                )
                .into_bytes()
            );
        }

        // Mouse with Alt modifier via generate_keyboard_sequence (Right button 2 + Alt 8
        // = 10).
        let mouse_alt = VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Right,
            pos: TermPos::from_one_based(12, 34),
            action: VT100MouseActionIR::Press,
            modifiers: VT100KeyModifiersIR::ALT,
        };
        assert_eq!(
            generate_keyboard_sequence(&mouse_alt),
            Some(
                format!(
                    "{CSI_START}{MOUSE_SGR_MARKER_CHAR}10{ANSI_PARAM_SEPARATOR_CHAR}12{ANSI_PARAM_SEPARATOR_CHAR}34{MOUSE_SGR_PRESS_CHAR}"
                )
                .into_bytes()
            )
        );

        // Mouse via generate_keyboard_sequence (Right button 2 + Ctrl 16 = 18).
        let mouse_event = VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Right,
            pos: TermPos::from_one_based(12, 34),
            action: VT100MouseActionIR::Press,
            modifiers: VT100KeyModifiersIR::CTRL,
        };
        assert_eq!(
            generate_keyboard_sequence(&mouse_event),
            Some(
                format!(
                    "{CSI_START}{MOUSE_SGR_MARKER_CHAR}18{ANSI_PARAM_SEPARATOR_CHAR}12{ANSI_PARAM_SEPARATOR_CHAR}34{MOUSE_SGR_PRESS_CHAR}"
                )
                .into_bytes()
            )
        );
    }

    #[test]
    fn test_generate_mouse_x10() {
        let x10 = generate_x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        // ESC [ M (0 + 32) (1 + 32) (1 + 32) \0
        assert_eq!(x10, vec![0x1B, b'[', b'M', 0, 33, 33, 0]);

        // Release action (maps to release button code 3).
        let x10_rel = generate_x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Release,
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(x10_rel, vec![0x1B, b'[', b'M', 3, 33, 33, 0]);

        // Drag action with Alt modifier (button 0 + motion 32 + alt 8 = 40).
        let x10_drag = generate_x10_mouse_sequence(
            VT100MouseButtonIR::Left,
            1,
            1,
            VT100MouseActionIR::Drag,
            VT100KeyModifiersIR::ALT,
        );
        assert_eq!(x10_drag, vec![0x1B, b'[', b'M', 40, 33, 33, 0]);
    }

    #[test]
    fn test_generate_mouse_rxvt() {
        let rxvt = generate_rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            10,
            20,
            VT100MouseActionIR::Press,
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(rxvt, b"\x1b[0;10;20M");

        // Release action (maps to release button code 3).
        let rxvt_rel = generate_rxvt_mouse_sequence(
            VT100MouseButtonIR::Middle,
            10,
            20,
            VT100MouseActionIR::Release,
            VT100KeyModifiersIR::default(),
        );
        assert_eq!(rxvt_rel, b"\x1b[3;10;20M");

        // Motion action with Alt modifier (motion 32 + alt 8 = 40).
        let rxvt_motion = generate_rxvt_mouse_sequence(
            VT100MouseButtonIR::Left,
            10,
            20,
            VT100MouseActionIR::Motion,
            VT100KeyModifiersIR::ALT,
        );
        assert_eq!(rxvt_motion, b"\x1b[40;10;20M");
    }

    // ==================== Builders & Constants ====================

    #[test]
    fn test_low_level_builders() {
        assert_eq!(csi(b'A'), [0x1B, b'[', b'A']);
        assert_eq!(ss3(b'P'), [0x1B, b'O', b'P']);
        assert_eq!(csi_tilde(15), b"\x1b[15~");
        assert_eq!(csi_modified(1, b'A'), b"\x1b[1;2A");
    }

    #[test]
    fn test_precomputed_constants() {
        assert_eq!(SEQ_ARROW_UP, b"\x1b[A");
        assert_eq!(SEQ_ARROW_DOWN, b"\x1b[B");
        assert_eq!(SEQ_ARROW_RIGHT, b"\x1b[C");
        assert_eq!(SEQ_ARROW_LEFT, b"\x1b[D");
        assert_eq!(SEQ_HOME, b"\x1b[H");
        assert_eq!(SEQ_END, b"\x1b[F");
        assert_eq!(SEQ_BACKTAB, b"\x1b[Z");
        assert_eq!(SEQ_F1, b"\x1bOP");
        assert_eq!(SEQ_F2, b"\x1bOQ");
        assert_eq!(SEQ_F3, b"\x1bOR");
        assert_eq!(SEQ_F4, b"\x1bOS");
    }

    #[test]
    fn test_ignored_event() {
        assert_eq!(
            generate_keyboard_sequence(&VT100InputEventIR::Ignored),
            None
        );
    }
}

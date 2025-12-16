// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words rowm

//! ANSI escape sequence generator for terminal INPUT (test fixtures).
//!
//! Provides input sequence generation for testing. Creates symmetry with
//! [`ansi_sequence_generator_output`] for output sequences.
//!
//! ## Purpose
//!
//! **This module is for testing only.** It is not used in production code.
//!
//! The generator enables:
//! 1. **Round-trip validation**: Parse ANSI → `InputEvent` → Generate ANSI → Verify match
//! 2. **Test helpers**: Build test sequences without hardcoding raw bytes
//! 3. **Parser verification**: Confirm parsers handle all modifier combinations correctly
//!
//! ## Available Items
//!
//! | Category                      | Items                                                                                                                                                     |
//! | :---------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------- |
//! | Pre-computed constants        | [`SEQ_ARROW_UP`], [`SEQ_ARROW_DOWN`], [`SEQ_ARROW_RIGHT`], [`SEQ_ARROW_LEFT`], [`SEQ_HOME`], [`SEQ_END`], [`SEQ_BACKTAB`], [`SEQ_F1`]–[`SEQ_F4`]          |
//! | Low-level builder functions   | [`csi`], [`ss3`], [`csi_tilde`], [`csi_modified`]                                                                                                         |
//! | High-level generators         | [`generate_keyboard_sequence`], [`generate_mouse_sequence_bytes`], [`generate_resize_sequence`], [`generate_focus_sequence`], [`generate_paste_sequence`] |

use crate::{KeyState,
            core::ansi::{constants::{ASCII_DEL, ASCII_DIGIT_0, CONTROL_ENTER,
                                     CONTROL_NUL, CONTROL_TAB, CSI_PREFIX,
                                     FOCUS_GAINED_FINAL, FOCUS_LOST_FINAL,
                                     FUNCTION_F1_CODE, FUNCTION_F2_CODE,
                                     FUNCTION_F3_CODE, FUNCTION_F4_CODE,
                                     FUNCTION_F5_CODE, FUNCTION_F6_CODE,
                                     FUNCTION_F7_CODE, FUNCTION_F8_CODE,
                                     FUNCTION_F9_CODE, FUNCTION_F10_CODE,
                                     FUNCTION_F11_CODE, FUNCTION_F12_CODE,
                                     MODIFIER_ALT, MODIFIER_CTRL,
                                     MODIFIER_PARAMETER_BASE_CHAR, MODIFIER_SHIFT,
                                     MOUSE_LEFT_BUTTON_CODE, MOUSE_MIDDLE_BUTTON_CODE,
                                     MOUSE_MODIFIER_ALT, MOUSE_MODIFIER_CTRL,
                                     MOUSE_MODIFIER_SHIFT, MOUSE_MOTION_FLAG,
                                     MOUSE_RELEASE_BUTTON_CODE,
                                     MOUSE_RIGHT_BUTTON_CODE,
                                     MOUSE_SCROLL_DOWN_BUTTON,
                                     MOUSE_SCROLL_LEFT_BUTTON,
                                     MOUSE_SCROLL_RIGHT_BUTTON,
                                     MOUSE_SCROLL_UP_BUTTON, MOUSE_SGR_PREFIX,
                                     MOUSE_SGR_PRESS, MOUSE_SGR_RELEASE,
                                     MOUSE_X10_MARKER, MOUSE_X10_PREFIX,
                                     PASTE_END_GENERATE_CODE,
                                     PASTE_START_GENERATE_CODE,
                                     RESIZE_EVENT_GENERATE_CODE, RESIZE_TERMINATOR,
                                     SPECIAL_DELETE_CODE, SPECIAL_INSERT_CODE,
                                     SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE},
                         vt_100_terminal_input_parser::{VT100FocusStateIR,
                                                        VT100InputEventIR,
                                                        VT100KeyCodeIR,
                                                        VT100KeyModifiersIR,
                                                        VT100MouseActionIR,
                                                        VT100MouseButtonIR,
                                                        VT100PasteModeIR}},
            input_sequences::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_FUNCTION_KEY_TERMINATOR,
                              ANSI_PARAM_SEPARATOR, ANSI_SS3_O, ARROW_DOWN_FINAL,
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

/// `BackTab` (Shift+Tab): `ESC [ Z`.
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

/// Builds CSI sequence: `ESC [ <final>`.
#[must_use]
pub const fn csi(final_byte: u8) -> [u8; 3] { [ANSI_ESC, ANSI_CSI_BRACKET, final_byte] }

/// Builds SS3 sequence: `ESC O <final>`.
#[must_use]
pub const fn ss3(final_byte: u8) -> [u8; 3] { [ANSI_ESC, ANSI_SS3_O, final_byte] }

/// Builds CSI tilde sequence: `ESC [ <code> ~`.
///
/// Used for function keys (F5+), Insert, Delete, `PageUp`, `PageDown`.
#[must_use]
pub fn csi_tilde(code: u16) -> Vec<u8> {
    let mut seq = vec![ANSI_ESC, ANSI_CSI_BRACKET];
    seq.extend(code.to_string().as_bytes());
    seq.push(ANSI_FUNCTION_KEY_TERMINATOR);
    seq
}

/// Builds CSI with modifier: `ESC [ 1 ; <mod+1> <final>`.
///
/// Modifier encoding: Shift=1, Alt=2, Ctrl=4 (additive).
/// The parameter sent is `1 + modifier_bits`.
#[must_use]
pub fn csi_modified(modifier: u8, final_byte: u8) -> Vec<u8> {
    let param = 1 + modifier;
    vec![
        ANSI_ESC,
        ANSI_CSI_BRACKET,
        b'1',
        ANSI_PARAM_SEPARATOR,
        b'0' + param,
        final_byte,
    ]
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HIGH-LEVEL GENERATORS - Main entry points for generating ANSI sequences
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Generate ANSI bytes for an input event.
///
/// Converts any input event back into the ANSI `CSI` sequence format that terminals
/// send. This enables round-trip validation: `InputEvent` → bytes → parse → `InputEvent`.
///
/// ## Supported Events
///
/// - **Keyboard**: All key codes with modifiers (arrows, function keys, special keys)
/// - **Resize**: Window resize notifications (`CSI 8 ; rows ; cols t`)
/// - **Focus**: Focus gained/lost events (`CSI I` / `CSI O`)
/// - **Paste**: Bracketed paste mode (`CSI 200~` / `CSI 201~`)
/// - **Mouse**: `SGR` mouse format (`CSI < button ; col ; row M/m`)
///
/// ## Returns
///
/// - `Some(Vec<u8>)` for recognized events
/// - `None` for unsupported or invalid combinations
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
            let rows = u16::try_from(row_height.as_usize()).unwrap_or(u16::MAX);
            let cols = u16::try_from(col_width.as_usize()).unwrap_or(u16::MAX);
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
    }
}

/// Generate ANSI bytes for a mouse event in `X10`/Normal format.
///
/// Generates sequences like: `ESC [ M Cb Cx Cy` (6 bytes)
///
/// ## `X10` Format Details
///
/// - `Cb` = button byte: button code (0-2) + modifier flags + motion flag
/// - `Cx` = column byte: `actual_column` + 32 (ASCII offset encoding)
/// - `Cy` = row byte: `actual_row` + 32 (ASCII offset encoding)
///
/// Button encoding:
/// - Bits 0-1: Button (0=left, 1=middle, 2=right, 3=release)
/// - Bit 2: Shift modifier (4)
/// - Bit 3: Alt modifier (8)
/// - Bit 4: Ctrl modifier (16)
/// - Bit 5: Motion flag (32)
///
/// ## Parameters
///
/// - `button`: Mouse button
/// - `col`: Column coordinate ([1-based])
/// - `row`: Row coordinate ([1-based])
/// - `action`: Press, Release, Motion, or Drag
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
///
/// [1-based]: crate::core::ansi::vt_100_terminal_input_parser#one-based-mouse-input-events
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

/// Generate ANSI bytes for a mouse event in `RXVT` format.
///
/// Generates sequences like: `ESC [ Cb ; Cx ; Cy M` (variable length)
///
/// ## `RXVT` Format Details
///
/// Uses decimal numbers separated by semicolons (human-readable format):
/// - `Cb` = button code (decimal): button (0-2) + modifier bits
/// - `Cx` = column coordinate (decimal, [1-based])
/// - `Cy` = row coordinate (decimal, [1-based])
/// - Final: `M` for press
///
/// ## Parameters
///
/// - `button`: Mouse button
/// - `col`: Column coordinate ([1-based])
/// - `row`: Row coordinate ([1-based])
/// - `action`: Press or Release (`RXVT` primarily uses Press)
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
///
/// [1-based]: crate::core::ansi::vt_100_terminal_input_parser#one-based-mouse-input-events
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

/// Generate ANSI bytes for a mouse event in `SGR` format.
///
/// Generates sequences like: `ESC [<button;col;rowM` or `ESC [<button;col;rowm`
///
/// ## Parameters
///
/// - `button`: Mouse button (0=left, 1=middle, 2=right, 64-67=scroll)
/// - `col`: Column coordinate ([1-based])
/// - `row`: Row coordinate ([1-based])
/// - `action`: Press, Release, or Drag
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
///
/// [1-based]: crate::core::ansi::vt_100_terminal_input_parser#one-based-mouse-input-events
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
/// This is the ANSI sequence sent by terminals when they are resized.
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PRIVATE MODULES - Implementation details organized by functionality
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Keyboard sequence generation (arrows, function keys, special keys, characters).
mod keyboard {
    use super::*;

    /// Generate ANSI bytes for a specific key code and modifiers.
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

            // Navigation keys: CSI H/F (no modifier support in this format)
            VT100KeyCodeIR::Home => Some(generate_simple_csi(SPECIAL_HOME_FINAL)),
            VT100KeyCodeIR::End => Some(generate_simple_csi(SPECIAL_END_FINAL)),

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

    /// Generate arrow key sequence: `CSI [1;mod] final`.
    fn generate_arrow_key(final_byte: u8, modifiers: VT100KeyModifiersIR) -> Vec<u8> {
        let mut bytes = CSI_PREFIX.to_vec();
        if encoding::has_modifiers(modifiers) {
            bytes.push(encoding::push_ascii_number(1));
            bytes.push(ANSI_PARAM_SEPARATOR);
            bytes.push(encoding::encode_modifiers(modifiers));
        }
        bytes.push(final_byte);
        bytes
    }

    /// Generate simple CSI sequence: `CSI final`.
    fn generate_simple_csi(final_byte: u8) -> Vec<u8> {
        let mut bytes = CSI_PREFIX.to_vec();
        bytes.push(final_byte);
        bytes
    }

    /// Generate tilde-terminated sequence: `CSI code [;mod] ~`.
    fn generate_tilde_key(code: u16, modifiers: VT100KeyModifiersIR) -> Vec<u8> {
        let mut bytes = CSI_PREFIX.to_vec();
        let code_str = code.to_string();
        bytes.extend_from_slice(code_str.as_bytes());

        if encoding::has_modifiers(modifiers) {
            bytes.push(ANSI_PARAM_SEPARATOR);
            bytes.push(encoding::encode_modifiers(modifiers));
        }

        bytes.push(ANSI_FUNCTION_KEY_TERMINATOR);
        bytes
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
        // Alt+letter: ESC + character
        if modifiers.alt == KeyState::Pressed
            && modifiers.ctrl == KeyState::NotPressed
            && modifiers.shift == KeyState::NotPressed
        {
            let mut bytes = vec![ANSI_ESC];
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
            return Some(bytes);
        }

        // Ctrl+letter: control byte (letter & 0x1F)
        if modifiers.ctrl == KeyState::Pressed
            && modifiers.alt == KeyState::NotPressed
            && modifiers.shift == KeyState::NotPressed
        {
            if c.is_ascii_alphabetic() {
                let control_byte = (c.to_ascii_lowercase() as u8) & 0x1F;
                return Some(vec![control_byte]);
            }
            return None; // Ctrl+non-letter not supported
        }

        // Plain character: UTF-8 encoded
        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        Some(encoded.as_bytes().to_vec())
    }
}

/// Mouse sequence generation (X10, RXVT, SGR formats).
mod mouse {
    use super::*;

    /// Generate mouse sequence in SGR format: `ESC [<button;col;rowM/m`.
    pub fn generate_sgr_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        // Handle scroll events: buttons 64-67 (up/down/left/right)
        let button_code = match action {
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
        };

        // Apply modifiers and action flags to button code
        let mut code = button_code;

        // Handle action/drag flag
        let action_char = match action {
            VT100MouseActionIR::Release => MOUSE_SGR_RELEASE as char,
            VT100MouseActionIR::Drag => {
                code |= MOUSE_MOTION_FLAG; // Drag flag (bit 5)
                MOUSE_SGR_PRESS as char
            }
            VT100MouseActionIR::Press
            | VT100MouseActionIR::Motion
            | VT100MouseActionIR::Scroll(_) => MOUSE_SGR_PRESS as char,
        };

        code = apply_modifiers(code, modifiers);

        // Build sequence: ESC[<button;col;rowM/m
        let mut bytes = MOUSE_SGR_PREFIX.to_vec();
        bytes.extend_from_slice(&encoding::push_ascii_u16(code));
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.extend_from_slice(&encoding::push_ascii_u16(col));
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.extend_from_slice(&encoding::push_ascii_u16(row));
        bytes.push(action_char as u8);
        bytes
    }

    /// Generate mouse sequence in X10 format: `ESC [ M Cb Cx Cy`.
    pub fn generate_x10_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        let mut cb = button_to_code(button);

        // Handle action
        match action {
            VT100MouseActionIR::Release => cb = MOUSE_RELEASE_BUTTON_CODE,
            VT100MouseActionIR::Motion | VT100MouseActionIR::Drag => {
                cb |= MOUSE_MOTION_FLAG;
            }
            VT100MouseActionIR::Press | VT100MouseActionIR::Scroll(_) => {}
        }

        cb = apply_modifiers(cb, modifiers);

        // X10 coordinate encoding: add 32 to make printable ASCII
        #[allow(clippy::cast_possible_truncation)]
        let cx = (col + 32) as u8;
        #[allow(clippy::cast_possible_truncation)]
        let cy = (row + 32) as u8;

        // Build sequence: ESC [ M Cb Cx Cy
        let mut bytes = MOUSE_X10_PREFIX.to_vec();
        #[allow(clippy::cast_possible_truncation)]
        bytes.push(cb as u8);
        bytes.push(cx);
        bytes.push(cy);
        bytes.push(CONTROL_NUL);
        bytes
    }

    /// Generate mouse sequence in RXVT format: `ESC [ Cb ; Cx ; Cy M`.
    pub fn generate_rxvt_sequence(
        button: VT100MouseButtonIR,
        col: u16,
        row: u16,
        action: VT100MouseActionIR,
        modifiers: VT100KeyModifiersIR,
    ) -> Vec<u8> {
        let mut cb = button_to_code(button);

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
        let mut bytes = CSI_PREFIX.to_vec();
        bytes.extend_from_slice(&encoding::push_ascii_u16(cb));
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.extend_from_slice(&encoding::push_ascii_u16(col));
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.extend_from_slice(&encoding::push_ascii_u16(row));
        bytes.push(MOUSE_X10_MARKER);
        bytes
    }

    /// Convert button enum to protocol code.
    fn button_to_code(button: VT100MouseButtonIR) -> u16 {
        match button {
            VT100MouseButtonIR::Left | VT100MouseButtonIR::Unknown => {
                MOUSE_LEFT_BUTTON_CODE
            }
            VT100MouseButtonIR::Middle => MOUSE_MIDDLE_BUTTON_CODE,
            VT100MouseButtonIR::Right => MOUSE_RIGHT_BUTTON_CODE,
        }
    }

    /// Apply modifier flags to button code.
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
    use super::*;

    /// Generate a window resize sequence: `CSI 8 ; rows ; cols t`.
    pub fn generate_resize_sequence(rows: u16, cols: u16) -> Vec<u8> {
        let mut bytes = CSI_PREFIX.to_vec();
        bytes.push(RESIZE_EVENT_GENERATE_CODE);
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.extend_from_slice(&encoding::push_ascii_u16(rows));
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.extend_from_slice(&encoding::push_ascii_u16(cols));
        bytes.push(RESIZE_TERMINATOR);
        bytes
    }

    /// Generate a focus event sequence: `CSI I` (gained) or `CSI O` (lost).
    pub fn generate_focus_sequence(state: VT100FocusStateIR) -> Vec<u8> {
        let mut bytes = CSI_PREFIX.to_vec();
        match state {
            VT100FocusStateIR::Gained => bytes.push(FOCUS_GAINED_FINAL),
            VT100FocusStateIR::Lost => bytes.push(FOCUS_LOST_FINAL),
        }
        bytes
    }

    /// Generate a bracketed paste sequence: `CSI 200 ~` (start) or `CSI 201 ~` (end).
    pub fn generate_paste_sequence(mode: VT100PasteModeIR) -> Vec<u8> {
        let mut bytes = CSI_PREFIX.to_vec();
        match mode {
            VT100PasteModeIR::Start => {
                bytes.extend_from_slice(PASTE_START_GENERATE_CODE.as_bytes());
            }
            VT100PasteModeIR::End => {
                bytes.extend_from_slice(PASTE_END_GENERATE_CODE.as_bytes());
            }
        }
        bytes.push(ANSI_FUNCTION_KEY_TERMINATOR);
        bytes
    }
}

/// ANSI encoding utilities (modifiers, ASCII conversion).
mod encoding {
    use super::*;

    /// Check if any modifiers are pressed.
    pub fn has_modifiers(modifiers: VT100KeyModifiersIR) -> bool {
        modifiers.shift == KeyState::Pressed
            || modifiers.ctrl == KeyState::Pressed
            || modifiers.alt == KeyState::Pressed
    }

    /// Encode modifier flags into a single byte following VT-100 ANSI convention.
    ///
    /// **VT-100 Modifier Encoding**: `parameter = 1 + bitfield`
    ///
    /// Where bitfield is:
    /// - bit 0 (value 1): Shift
    /// - bit 1 (value 2): Alt
    /// - bit 2 (value 4): Ctrl
    ///
    /// ## Parameter Values
    ///
    /// - `1` → no modifiers (1 + 0)
    /// - `2` → Shift (1 + 1)
    /// - `3` → Alt (1 + 2)
    /// - `4` → Alt+Shift (1 + 3)
    /// - `5` → Ctrl (1 + 4)
    /// - `6` → Ctrl+Shift (1 + 5)
    /// - `7` → Ctrl+Alt (1 + 6)
    /// - `8` → Ctrl+Alt+Shift (1 + 7)
    pub fn encode_modifiers(modifiers: VT100KeyModifiersIR) -> u8 {
        let mut mask: u8 = 0;
        if modifiers.shift == KeyState::Pressed {
            mask |= MODIFIER_SHIFT;
        }
        if modifiers.alt == KeyState::Pressed {
            mask |= MODIFIER_ALT;
        }
        if modifiers.ctrl == KeyState::Pressed {
            mask |= MODIFIER_CTRL;
        }
        MODIFIER_PARAMETER_BASE_CHAR + mask
    }

    /// Convert a numeric value (0-9) to its ASCII character representation.
    pub fn push_ascii_number(value: u8) -> u8 {
        debug_assert!(
            value <= 9,
            "Value must be a single digit (0-9), got {value}"
        );
        ASCII_DIGIT_0 + value
    }

    /// Convert a multi-digit numeric value to its ASCII byte representation.
    pub fn push_ascii_u16(value: u16) -> Vec<u8> { value.to_string().into_bytes() }
}

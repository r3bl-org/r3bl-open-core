// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input event generator - converts high-level input events to ANSI sequences.
//!
//! This module provides the inverse operation to the input parsers in
//! [`vt_100_terminal_input_parser`].
//!
//! ## Purpose
//!
//! **This module is for testing only.** It is not used in production code.
//!
//! The generator enables:
//! 1. **Round-trip validation**: Parse ANSI → InputEvent → Generate ANSI → Verify match
//! 2. **Test helpers**: Build test sequences without hardcoding raw bytes
//! 3. **Parser verification**: Confirm parsers handle all modifier combinations correctly
//!
//! [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser

use crate::core::ansi::{constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ARROW_DOWN_FINAL, ARROW_LEFT_FINAL,
                                    ARROW_RIGHT_FINAL, ARROW_UP_FINAL,
                                    FUNCTION_F1_CODE, FUNCTION_F2_CODE,
                                    FUNCTION_F3_CODE, FUNCTION_F4_CODE,
                                    FUNCTION_F5_CODE, FUNCTION_F6_CODE,
                                    FUNCTION_F7_CODE, FUNCTION_F8_CODE,
                                    FUNCTION_F9_CODE, FUNCTION_F10_CODE,
                                    FUNCTION_F11_CODE, FUNCTION_F12_CODE, MODIFIER_ALT,
                                    MODIFIER_CTRL, MODIFIER_SHIFT, SPECIAL_DELETE_CODE,
                                    SPECIAL_END_FINAL, SPECIAL_HOME_FINAL,
                                    SPECIAL_INSERT_CODE, SPECIAL_PAGE_DOWN_CODE,
                                    SPECIAL_PAGE_UP_CODE},
                        vt_100_terminal_input_parser::{InputEvent, KeyCode,
                                                       KeyModifiers, FocusState, PasteMode, MouseButton, MouseAction}};

/// Generate ANSI bytes for an input event.
///
/// Converts any input event back into the ANSI CSI sequence format that terminals
/// send. This enables round-trip validation: InputEvent → bytes → parse → InputEvent.
///
/// ## Supported Events
///
/// - **Keyboard**: All key codes with modifiers (arrows, function keys, special keys)
/// - **Resize**: Window resize notifications (CSI 8 ; rows ; cols t)
/// - **Focus**: Focus gained/lost events (CSI I / CSI O)
/// - **Paste**: Bracketed paste mode (CSI 200~ / CSI 201~)
/// - **Mouse**: SGR mouse format (CSI < button ; col ; row M/m)
///
/// ## Returns
///
/// - `Some(Vec<u8>)` for recognized events
/// - `None` for unsupported or invalid combinations
///
/// ## Usage
///
/// This function is used internally by tests to generate sequences for round-trip
/// validation. See the test suite for examples of all supported event types.
pub fn generate_keyboard_sequence(event: &InputEvent) -> Option<Vec<u8>> {
    match event {
        InputEvent::Keyboard { code, modifiers } => {
            generate_key_sequence(*code, *modifiers)
        }
        InputEvent::Resize { rows, cols } => {
            Some(generate_resize_sequence(*rows, *cols))
        }
        InputEvent::Focus(state) => {
            Some(generate_focus_sequence(*state))
        }
        InputEvent::Paste(mode) => {
            Some(generate_paste_sequence(*mode))
        }
        InputEvent::Mouse { button, pos, action, modifiers } => {
            Some(generate_mouse_sequence_bytes(*button, pos.col.as_u16(), pos.row.as_u16(), *action, *modifiers))
        }
    }
}

/// Generate ANSI bytes for a mouse event in SGR format.
///
/// Generates sequences like: `ESC[<button;col;rowM` or `ESC[<button;col;rowm`
///
/// ## Parameters
///
/// - `button`: Mouse button (0=left, 1=middle, 2=right, 64-67=scroll)
/// - `col`: Column coordinate (1-based)
/// - `row`: Row coordinate (1-based)
/// - `action`: Press, Release, or Drag
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
pub fn generate_mouse_sequence_bytes(
    button: MouseButton,
    col: u16,
    row: u16,
    action: MouseAction,
    modifiers: KeyModifiers,
) -> Vec<u8> {
    let button_code = match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::Unknown => 0, // Default to left for unknown buttons
    };

    // Apply modifiers and action flags to button code
    let mut code = button_code;

    // Handle action/drag flag
    let action_char = match action {
        MouseAction::Press => 'M',
        MouseAction::Release => 'm',
        MouseAction::Drag => {
            code |= 32; // Drag flag (bit 5)
            'M'
        }
        MouseAction::Motion => 'M', // Motion events use M like press
        MouseAction::Scroll(_) => 'M', // Scroll uses button codes 64-67
    };

    // Apply modifiers: shift=1, alt=2, ctrl=4
    let mut modifier_bits: u8 = 0;
    if modifiers.shift {
        modifier_bits |= 1;
    }
    if modifiers.alt {
        modifier_bits |= 2;
    }
    if modifiers.ctrl {
        modifier_bits |= 4;
    }
    code |= modifier_bits;

    // Build sequence: ESC[<button;col;rowM/m
    let mut bytes = vec![ANSI_ESC, ANSI_CSI_BRACKET, b'<'];
    bytes.extend_from_slice(code.to_string().as_bytes());
    bytes.push(b';');
    bytes.extend_from_slice(col.to_string().as_bytes());
    bytes.push(b';');
    bytes.extend_from_slice(row.to_string().as_bytes());
    bytes.push(action_char as u8);
    bytes
}

/// Generate ANSI bytes for a specific key code and modifiers.
fn generate_key_sequence(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    // Build the base sequence
    let mut bytes = vec![ANSI_ESC, ANSI_CSI_BRACKET];

    let has_modifiers = modifiers.shift || modifiers.ctrl || modifiers.alt;

    match code {
        // ==================== Arrow Keys ====================
        KeyCode::Up => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_UP_FINAL);
            Some(bytes)
        }
        KeyCode::Down => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_DOWN_FINAL);
            Some(bytes)
        }
        KeyCode::Right => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_RIGHT_FINAL);
            Some(bytes)
        }
        KeyCode::Left => {
            if has_modifiers {
                bytes.push(b'1');
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_LEFT_FINAL);
            Some(bytes)
        }

        // ==================== Special Keys (CSI H/F) ====================
        KeyCode::Home => {
            bytes.push(SPECIAL_HOME_FINAL);
            Some(bytes)
        }
        KeyCode::End => {
            bytes.push(SPECIAL_END_FINAL);
            Some(bytes)
        }

        // ==================== Special Keys (CSI n~) ====================
        KeyCode::Insert => {
            generate_special_key_sequence(&mut bytes, SPECIAL_INSERT_CODE, modifiers)
        }
        KeyCode::Delete => {
            generate_special_key_sequence(&mut bytes, SPECIAL_DELETE_CODE, modifiers)
        }
        KeyCode::PageUp => {
            generate_special_key_sequence(&mut bytes, SPECIAL_PAGE_UP_CODE, modifiers)
        }
        KeyCode::PageDown => {
            generate_special_key_sequence(&mut bytes, SPECIAL_PAGE_DOWN_CODE, modifiers)
        }

        // ==================== Function Keys (CSI n~) ====================
        KeyCode::Function(n) => {
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
                _ => return None, // Invalid function key number
            };
            generate_special_key_sequence(&mut bytes, code, modifiers)
        }

        // ==================== Other Keys ====================
        // Tab, Enter, Escape, Backspace are typically raw control characters,
        // not CSI sequences. Not implemented in generator as they're handled
        // differently in the input parsing layer.
        KeyCode::Tab
        | KeyCode::BackTab
        | KeyCode::Enter
        | KeyCode::Escape
        | KeyCode::Backspace => None,

        // Char events are also handled differently (UTF-8 text)
        KeyCode::Char(_) => None,
    }
}

/// Generate a special key or function key sequence (CSI n~).
fn generate_special_key_sequence(
    bytes: &mut Vec<u8>,
    code: u16,
    modifiers: KeyModifiers,
) -> Option<Vec<u8>> {
    // Format: CSI code~ or CSI code; modifier~
    let code_str = code.to_string();
    bytes.extend_from_slice(code_str.as_bytes());

    if modifiers.shift || modifiers.ctrl || modifiers.alt {
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.push(encode_modifiers(modifiers));
    }

    bytes.push(ANSI_FUNCTION_KEY_TERMINATOR);
    Some(bytes.clone())
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
///
/// **Confirmed by terminal observation**: `ESC[1;5A` = Ctrl+Up (parameter 5 = 1+4)
fn encode_modifiers(modifiers: KeyModifiers) -> u8 {
    let mut mask: u8 = 0;
    if modifiers.shift {
        mask |= MODIFIER_SHIFT;
    }
    if modifiers.alt {
        mask |= MODIFIER_ALT;
    }
    if modifiers.ctrl {
        mask |= MODIFIER_CTRL;
    }
    // VT-100 formula: parameter = 1 + bitfield
    // Produce ASCII digit character for the parameter (1-8 as '1'-'8')
    b'1' + mask
}

/// Generate a window resize sequence: `CSI 8 ; rows ; cols t`
///
/// This is the ANSI sequence sent by terminals when they are resized.
pub fn generate_resize_sequence(rows: u16, cols: u16) -> Vec<u8> {
    let mut bytes = vec![ANSI_ESC, ANSI_CSI_BRACKET];
    bytes.push(b'8');
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(rows.to_string().as_bytes());
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(cols.to_string().as_bytes());
    bytes.push(b't');
    bytes
}

/// Generate a focus event sequence.
///
/// - Focus gained: `CSI I`
/// - Focus lost: `CSI O`
pub fn generate_focus_sequence(state: FocusState) -> Vec<u8> {
    match state {
        FocusState::Gained => vec![ANSI_ESC, ANSI_CSI_BRACKET, b'I'],
        FocusState::Lost => vec![ANSI_ESC, ANSI_CSI_BRACKET, b'O'],
    }
}

/// Generate a bracketed paste mode sequence.
///
/// - Paste start: `CSI 200 ~`
/// - Paste end: `CSI 201 ~`
pub fn generate_paste_sequence(mode: PasteMode) -> Vec<u8> {
    let mut bytes = vec![ANSI_ESC, ANSI_CSI_BRACKET];
    match mode {
        PasteMode::Start => {
            bytes.extend_from_slice(b"200");
        }
        PasteMode::End => {
            bytes.extend_from_slice(b"201");
        }
    }
    bytes.push(b'~');
    bytes
}

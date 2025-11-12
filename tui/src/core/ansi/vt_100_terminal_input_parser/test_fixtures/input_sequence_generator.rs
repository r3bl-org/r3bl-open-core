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
//! 1. **Round-trip validation**: Parse ANSI → `InputEvent` → Generate ANSI → Verify match
//! 2. **Test helpers**: Build test sequences without hardcoding raw bytes
//! 3. **Parser verification**: Confirm parsers handle all modifier combinations correctly
//!
//! [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser

use crate::{KeyState,
            core::ansi::{constants::{ANSI_FUNCTION_KEY_TERMINATOR,
                                     ANSI_PARAM_SEPARATOR, ARROW_DOWN_FINAL,
                                     ARROW_LEFT_FINAL, ARROW_RIGHT_FINAL,
                                     ARROW_UP_FINAL, ASCII_DIGIT_0, CONTROL_NUL,
                                     CSI_PREFIX, FOCUS_GAINED_FINAL, FOCUS_LOST_FINAL,
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
                                     SPECIAL_DELETE_CODE, SPECIAL_END_FINAL,
                                     SPECIAL_HOME_FINAL, SPECIAL_INSERT_CODE,
                                     SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE},
                         vt_100_terminal_input_parser::{VT100FocusState,
                                                        VT100InputEvent, VT100KeyCode,
                                                        VT100KeyModifiers,
                                                        VT100MouseAction,
                                                        VT100MouseButton,
                                                        VT100PasteMode}}};

/// Generate ANSI bytes for an input event.
///
/// Converts any input event back into the ANSI CSI sequence format that terminals
/// send. This enables round-trip validation: `InputEvent` → bytes → parse → `InputEvent`.
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
#[must_use]
pub fn generate_keyboard_sequence(event: &VT100InputEvent) -> Option<Vec<u8>> {
    match event {
        VT100InputEvent::Keyboard { code, modifiers } => {
            generate_key_sequence(*code, *modifiers)
        }
        VT100InputEvent::Resize {
            col_width,
            row_height,
        } => {
            let rows = u16::try_from(row_height.as_usize()).unwrap_or(u16::MAX);
            let cols = u16::try_from(col_width.as_usize()).unwrap_or(u16::MAX);
            Some(generate_resize_sequence(rows, cols))
        }
        VT100InputEvent::Focus(state) => Some(generate_focus_sequence(*state)),
        VT100InputEvent::Paste(mode) => Some(generate_paste_sequence(*mode)),
        VT100InputEvent::Mouse {
            button,
            pos,
            action,
            modifiers,
        } => Some(generate_mouse_sequence_bytes(
            *button,
            pos.col.as_u16(),
            pos.row.as_u16(),
            *action,
            *modifiers,
        )),
    }
}

/// Generate ANSI bytes for a mouse event in X10/Normal format.
///
/// Generates sequences like: `ESC [ M Cb Cx Cy` (6 bytes)
///
/// ## X10 Format Details
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
/// - `col`: Column coordinate (1-based)
/// - `row`: Row coordinate (1-based)
/// - `action`: Press, Release, Motion, or Drag
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
#[must_use]
pub fn generate_x10_mouse_sequence(
    button: VT100MouseButton,
    col: u16,
    row: u16,
    action: VT100MouseAction,
    modifiers: VT100KeyModifiers,
) -> Vec<u8> {
    // Base button code (Unknown defaults to Left)
    let button_code = match button {
        VT100MouseButton::Left | VT100MouseButton::Unknown => MOUSE_LEFT_BUTTON_CODE,
        VT100MouseButton::Middle => MOUSE_MIDDLE_BUTTON_CODE,
        VT100MouseButton::Right => MOUSE_RIGHT_BUTTON_CODE,
    };

    let mut cb = button_code;

    // Handle action
    match action {
        /* Release always sends button=3 */
        VT100MouseAction::Release => cb = MOUSE_RELEASE_BUTTON_CODE,
        /* Motion and drag set motion flag (bit 5) */
        VT100MouseAction::Motion | VT100MouseAction::Drag => cb |= MOUSE_MOTION_FLAG,
        /* Press uses base button code; Scroll not typically used in X10 */
        VT100MouseAction::Press | VT100MouseAction::Scroll(_) => {}
    }

    // Apply modifiers
    if modifiers.shift == KeyState::Pressed {
        cb |= MOUSE_MODIFIER_SHIFT;
    }
    if modifiers.alt == KeyState::Pressed {
        cb |= MOUSE_MODIFIER_ALT;
    }
    if modifiers.ctrl == KeyState::Pressed {
        cb |= MOUSE_MODIFIER_CTRL;
    }

    // X10 coordinate encoding: add 32 to make printable ASCII
    #[allow(clippy::cast_possible_truncation)]
    let cx = (col + 32) as u8;
    #[allow(clippy::cast_possible_truncation)]
    let cy = (row + 32) as u8;

    // Build sequence: ESC [ M Cb Cx Cy
    let mut bytes = MOUSE_X10_PREFIX.to_vec();
    // Safe to cast cb from u16 to u8: button byte encoding uses only 8 bits.
    // - Regular buttons (0-3) + motion flag (32) + modifiers (4,8,16) = max 63
    // - Scroll buttons not used in X10 format
    // All values fit safely in u8 range (0-255). We use u16 during bitwise operations
    // for consistency with the parser, then narrow to u8 for serialization.
    #[allow(clippy::cast_possible_truncation)]
    bytes.push(cb as u8);
    bytes.push(cx);
    bytes.push(cy);
    bytes.push(CONTROL_NUL); // Null terminator (some implementations include this)
    bytes
}

/// Generate ANSI bytes for a mouse event in RXVT format.
///
/// Generates sequences like: `ESC [ Cb ; Cx ; Cy M` (variable length)
///
/// ## RXVT Format Details
///
/// Uses decimal numbers separated by semicolons (human-readable format):
/// - `Cb` = button code (decimal): button (0-2) + modifier bits
/// - `Cx` = column coordinate (decimal, 1-based)
/// - `Cy` = row coordinate (decimal, 1-based)
/// - Final: `M` for press
///
/// ## Parameters
///
/// - `button`: Mouse button
/// - `col`: Column coordinate (1-based)
/// - `row`: Row coordinate (1-based)
/// - `action`: Press or Release (RXVT primarily uses Press)
/// - `modifiers`: Key modifiers (Shift, Ctrl, Alt)
#[must_use]
pub fn generate_rxvt_mouse_sequence(
    button: VT100MouseButton,
    col: u16,
    row: u16,
    action: VT100MouseAction,
    modifiers: VT100KeyModifiers,
) -> Vec<u8> {
    // Base button code (Unknown defaults to Left)
    let button_code = match button {
        VT100MouseButton::Left | VT100MouseButton::Unknown => MOUSE_LEFT_BUTTON_CODE,
        VT100MouseButton::Middle => MOUSE_MIDDLE_BUTTON_CODE,
        VT100MouseButton::Right => MOUSE_RIGHT_BUTTON_CODE,
    };

    let mut cb = button_code;

    // Handle action
    match action {
        VT100MouseAction::Release => cb = MOUSE_RELEASE_BUTTON_CODE,
        VT100MouseAction::Motion | VT100MouseAction::Drag => cb |= MOUSE_MOTION_FLAG,
        VT100MouseAction::Press | VT100MouseAction::Scroll(_) => {}
    }

    // Apply modifiers (same encoding as X10)
    if modifiers.shift == KeyState::Pressed {
        cb |= MOUSE_MODIFIER_SHIFT;
    }
    if modifiers.alt == KeyState::Pressed {
        cb |= MOUSE_MODIFIER_ALT;
    }
    if modifiers.ctrl == KeyState::Pressed {
        cb |= MOUSE_MODIFIER_CTRL;
    }

    // Build sequence: ESC [ Cb ; Cx ; Cy M
    let mut bytes = CSI_PREFIX.to_vec();
    bytes.extend_from_slice(&push_ascii_u16(cb));
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(&push_ascii_u16(col));
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(&push_ascii_u16(row));
    bytes.push(MOUSE_X10_MARKER);
    bytes
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
#[must_use]
pub fn generate_mouse_sequence_bytes(
    button: VT100MouseButton,
    col: u16,
    row: u16,
    action: VT100MouseAction,
    modifiers: VT100KeyModifiers,
) -> Vec<u8> {
    // Handle scroll events: buttons 64-67 (up/down/left/right)
    let button_code = match action {
        VT100MouseAction::Scroll(scroll_dir) => {
            use crate::core::ansi::vt_100_terminal_input_parser::VT100ScrollDirection;
            match scroll_dir {
                VT100ScrollDirection::Up => MOUSE_SCROLL_UP_BUTTON,
                VT100ScrollDirection::Down => MOUSE_SCROLL_DOWN_BUTTON,
                VT100ScrollDirection::Left => MOUSE_SCROLL_LEFT_BUTTON,
                VT100ScrollDirection::Right => MOUSE_SCROLL_RIGHT_BUTTON,
            }
        }
        _ => match button {
            VT100MouseButton::Middle => MOUSE_MIDDLE_BUTTON_CODE,
            VT100MouseButton::Right => MOUSE_RIGHT_BUTTON_CODE,
            VT100MouseButton::Left | VT100MouseButton::Unknown => MOUSE_LEFT_BUTTON_CODE,
        },
    };

    // Apply modifiers and action flags to button code
    let mut code = button_code;

    // Handle action/drag flag
    let action_char = match action {
        VT100MouseAction::Release => MOUSE_SGR_RELEASE as char,
        VT100MouseAction::Drag => {
            code |= MOUSE_MOTION_FLAG; // Drag flag (bit 5)
            MOUSE_SGR_PRESS as char
        }
        VT100MouseAction::Press
        | VT100MouseAction::Motion
        | VT100MouseAction::Scroll(_) => {
            // Motion and scroll events use M like press
            MOUSE_SGR_PRESS as char
        }
    };

    // Apply modifiers: shift=4, alt=8, ctrl=16 (bits 2, 3, 4)
    // These match the encoding used by the parser in extract_modifiers()
    if modifiers.shift == KeyState::Pressed {
        code |= MOUSE_MODIFIER_SHIFT;
    }
    if modifiers.alt == KeyState::Pressed {
        code |= MOUSE_MODIFIER_ALT;
    }
    if modifiers.ctrl == KeyState::Pressed {
        code |= MOUSE_MODIFIER_CTRL;
    }

    // Build sequence: ESC[<button;col;rowM/m
    let mut bytes = MOUSE_SGR_PREFIX.to_vec();
    bytes.extend_from_slice(&push_ascii_u16(code));
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(&push_ascii_u16(col));
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(&push_ascii_u16(row));
    bytes.push(action_char as u8);
    bytes
}

/// Generate ANSI bytes for a specific key code and modifiers.
fn generate_key_sequence(
    code: VT100KeyCode,
    modifiers: VT100KeyModifiers,
) -> Option<Vec<u8>> {
    // Build the base sequence
    let mut bytes = CSI_PREFIX.to_vec();

    let has_modifiers = modifiers.shift == KeyState::Pressed
        || modifiers.ctrl == KeyState::Pressed
        || modifiers.alt == KeyState::Pressed;

    match code {
        // ==================== Arrow Keys ====================
        VT100KeyCode::Up => {
            if has_modifiers {
                // Use helper to push ASCII '1' (0x31), not numeric value 1 (0x01)
                bytes.push(push_ascii_number(1));
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_UP_FINAL);
            Some(bytes)
        }
        VT100KeyCode::Down => {
            if has_modifiers {
                // Use helper to push ASCII '1' (0x31), not numeric value 1 (0x01)
                bytes.push(push_ascii_number(1));
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_DOWN_FINAL);
            Some(bytes)
        }
        VT100KeyCode::Right => {
            if has_modifiers {
                // Use helper to push ASCII '1' (0x31), not numeric value 1 (0x01)
                bytes.push(push_ascii_number(1));
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_RIGHT_FINAL);
            Some(bytes)
        }
        VT100KeyCode::Left => {
            if has_modifiers {
                // Use helper to push ASCII '1' (0x31), not numeric value 1 (0x01)
                bytes.push(push_ascii_number(1));
                bytes.push(ANSI_PARAM_SEPARATOR);
                bytes.push(encode_modifiers(modifiers));
            }
            bytes.push(ARROW_LEFT_FINAL);
            Some(bytes)
        }

        // ==================== Special Keys (CSI H/F) ====================
        VT100KeyCode::Home => {
            bytes.push(SPECIAL_HOME_FINAL);
            Some(bytes)
        }
        VT100KeyCode::End => {
            bytes.push(SPECIAL_END_FINAL);
            Some(bytes)
        }

        // ==================== Special Keys (CSI n~) ====================
        VT100KeyCode::Insert => Some(generate_special_key_sequence(
            &mut bytes,
            SPECIAL_INSERT_CODE,
            modifiers,
        )),
        VT100KeyCode::Delete => Some(generate_special_key_sequence(
            &mut bytes,
            SPECIAL_DELETE_CODE,
            modifiers,
        )),
        VT100KeyCode::PageUp => Some(generate_special_key_sequence(
            &mut bytes,
            SPECIAL_PAGE_UP_CODE,
            modifiers,
        )),
        VT100KeyCode::PageDown => Some(generate_special_key_sequence(
            &mut bytes,
            SPECIAL_PAGE_DOWN_CODE,
            modifiers,
        )),

        // ==================== Function Keys (CSI n~) ====================
        VT100KeyCode::Function(n) => {
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
            Some(generate_special_key_sequence(&mut bytes, code, modifiers))
        }

        // ==================== Other Keys ====================
        // Tab, Enter, Escape, Backspace, and Char events are typically raw control
        // characters or UTF-8 text, not CSI sequences. Not implemented in
        // generator as they're handled differently in the input parsing layer.
        VT100KeyCode::Tab
        | VT100KeyCode::BackTab
        | VT100KeyCode::Enter
        | VT100KeyCode::Escape
        | VT100KeyCode::Backspace
        | VT100KeyCode::Char(_) => None,
    }
}

/// Generate a special key or function key sequence (CSI n~).
fn generate_special_key_sequence(
    bytes: &mut Vec<u8>,
    code: u16,
    modifiers: VT100KeyModifiers,
) -> Vec<u8> {
    // Format: CSI code~ or CSI code; modifier~
    let code_str = code.to_string();
    bytes.extend_from_slice(code_str.as_bytes());

    if modifiers.shift == KeyState::Pressed
        || modifiers.ctrl == KeyState::Pressed
        || modifiers.alt == KeyState::Pressed
    {
        bytes.push(ANSI_PARAM_SEPARATOR);
        bytes.push(encode_modifiers(modifiers));
    }

    bytes.push(ANSI_FUNCTION_KEY_TERMINATOR);
    bytes.clone()
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
fn encode_modifiers(modifiers: VT100KeyModifiers) -> u8 {
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
    // VT-100 formula: parameter = 1 + bitfield
    // Produce ASCII digit character for the parameter (1-8 as '1'-'8')
    MODIFIER_PARAMETER_BASE_CHAR + mask
}

/// Convert a numeric value (0-9) to its ASCII character representation.
///
/// ## ANSI Protocol Requirement
///
/// ANSI escape sequences use ASCII characters to represent numeric parameters.
/// This function performs the critical conversion from numeric values to their
/// ASCII byte representations.
///
/// ## Examples
///
/// ```text
/// Number  → ASCII Char → Byte Value
/// ------    ----------   ----------
/// 0       → '0'        → 0x30 (48)
/// 1       → '1'        → 0x31 (49)
/// 5       → '5'        → 0x35 (53)
/// 9       → '9'        → 0x39 (57)
/// ```
///
/// ## Common Error Pattern This Prevents
///
/// ```rust,ignore
/// // WRONG - pushes numeric value directly
/// bytes.push(1);  // Pushes byte 0x01, not ASCII '1'
///
/// // CORRECT - converts to ASCII first
/// bytes.push(push_ascii_number(1));  // Pushes byte 0x31 = '1'
/// ```
///
/// ## Parameters
///
/// - `value`: Numeric value (0-9)
///
/// ## Returns
///
/// ASCII byte representation of the digit
///
/// ## Panics
///
/// Panics if `value` is not in range 0-9 (debug builds only)
fn push_ascii_number(value: u8) -> u8 {
    debug_assert!(
        value <= 9,
        "Value must be a single digit (0-9), got {value}"
    );
    ASCII_DIGIT_0 + value
}

/// Convert a multi-digit numeric value to its ASCII byte representation.
///
/// ## Purpose
///
/// ANSI escape sequences represent multi-digit numbers as sequences of ASCII digits.
/// This helper eliminates repetitive `to_string().as_bytes()` calls throughout the
/// codebase.
///
/// ## Examples
///
/// ```rust,ignore
/// // Instead of:
/// bytes.extend_from_slice(col.to_string().as_bytes());
///
/// // Use:
/// bytes.extend_from_slice(&push_ascii_u16(col));
/// ```
///
/// ## Common Use Cases
///
/// - Mouse coordinates (e.g., column 120, row 50)
/// - Window resize dimensions (e.g., 80x24)
/// - Multi-digit parameter codes (e.g., 200 for paste start)
///
/// ## Parameters
///
/// - `value`: Numeric value to convert (any u16 value)
///
/// ## Returns
///
/// A `Vec<u8>` containing the ASCII byte representation of the number.
/// For example, 123 returns vec![b'1', b'2', b'3'].
fn push_ascii_u16(value: u16) -> Vec<u8> { value.to_string().into_bytes() }

/// Generate a window resize sequence: `CSI 8 ; rows ; cols t`
///
/// This is the ANSI sequence sent by terminals when they are resized.
#[must_use]
pub fn generate_resize_sequence(rows: u16, cols: u16) -> Vec<u8> {
    let mut bytes = CSI_PREFIX.to_vec();
    bytes.push(RESIZE_EVENT_GENERATE_CODE);
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(&push_ascii_u16(rows));
    bytes.push(ANSI_PARAM_SEPARATOR);
    bytes.extend_from_slice(&push_ascii_u16(cols));
    bytes.push(RESIZE_TERMINATOR);
    bytes
}

/// Generate a focus event sequence.
///
/// - Focus gained: `CSI I`
/// - Focus lost: `CSI O`
#[must_use]
pub fn generate_focus_sequence(state: VT100FocusState) -> Vec<u8> {
    let mut bytes = CSI_PREFIX.to_vec();
    match state {
        VT100FocusState::Gained => bytes.push(FOCUS_GAINED_FINAL),
        VT100FocusState::Lost => bytes.push(FOCUS_LOST_FINAL),
    }
    bytes
}

/// Generate a bracketed paste mode sequence.
///
/// - Paste start: `CSI 200 ~`
/// - Paste end: `CSI 201 ~`
#[must_use]
pub fn generate_paste_sequence(mode: VT100PasteMode) -> Vec<u8> {
    let mut bytes = CSI_PREFIX.to_vec();
    match mode {
        VT100PasteMode::Start => {
            bytes.extend_from_slice(PASTE_START_GENERATE_CODE.as_bytes());
        }
        VT100PasteMode::End => {
            bytes.extend_from_slice(PASTE_END_GENERATE_CODE.as_bytes());
        }
    }
    bytes.push(ANSI_FUNCTION_KEY_TERMINATOR);
    bytes
}

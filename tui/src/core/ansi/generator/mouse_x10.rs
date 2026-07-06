// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![allow(clippy::cast_possible_truncation)]

use crate::{Button, KeyState, MouseInput, MouseInputKind, TermCol, TermRow,
            core::ansi::constants::{MOUSE_LEFT_BUTTON_CODE, MOUSE_MIDDLE_BUTTON_CODE,
                                    MOUSE_MODIFIER_ALT, MOUSE_MODIFIER_CTRL,
                                    MOUSE_MODIFIER_SHIFT, MOUSE_MOTION_FLAG,
                                    MOUSE_RELEASE_BUTTON_CODE, MOUSE_RIGHT_BUTTON_CODE,
                                    MOUSE_SCROLL_DOWN_BUTTON, MOUSE_SCROLL_LEFT_BUTTON,
                                    MOUSE_SCROLL_RIGHT_BUTTON, MOUSE_SCROLL_UP_BUTTON,
                                    MOUSE_X10_COORD_OFFSET, MOUSE_X10_PREFIX}};

/// Generates an legacy [`X10`] mouse sequence for the given [`MouseInput`].
///
/// The [`X10`] mouse sequence format is:
/// - `CSI M Cb Cx Cy`
/// - where Cb, Cx, and Cy are byte values offset by `32`.
///
/// Note: This format cannot encode coordinates larger than `223`.
///
/// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
#[must_use]
pub fn generate(mouse_input: &MouseInput, x: TermCol, y: TermRow) -> Option<Vec<u8>> {
    let x_val: u16 = x.as_u16();
    let y_val: u16 = y.as_u16();

    // X10 protocol can only support coordinates up to 255 - 32 = 223
    if x_val > 223 || y_val > 223 {
        return None;
    }

    let mut button_code = match mouse_input.kind {
        MouseInputKind::MouseDown(button) => get_button_code(button),
        MouseInputKind::MouseUp(_) => MOUSE_RELEASE_BUTTON_CODE, // X10 just sends 3
        MouseInputKind::MouseDrag(button) => get_button_code(button) | MOUSE_MOTION_FLAG,
        MouseInputKind::MouseMove => MOUSE_RELEASE_BUTTON_CODE | MOUSE_MOTION_FLAG,
        MouseInputKind::ScrollUp => MOUSE_SCROLL_UP_BUTTON,
        MouseInputKind::ScrollDown => MOUSE_SCROLL_DOWN_BUTTON,
        MouseInputKind::ScrollLeft => MOUSE_SCROLL_LEFT_BUTTON,
        MouseInputKind::ScrollRight => MOUSE_SCROLL_RIGHT_BUTTON,
    };

    if let Some(modifiers) = &mouse_input.maybe_modifier_keys {
        if modifiers.shift_key_state == KeyState::Pressed {
            button_code |= MOUSE_MODIFIER_SHIFT;
        }
        if modifiers.alt_key_state == KeyState::Pressed {
            button_code |= MOUSE_MODIFIER_ALT;
        }
        if modifiers.ctrl_key_state == KeyState::Pressed {
            button_code |= MOUSE_MODIFIER_CTRL;
        }
    }

    let cb = (button_code + MOUSE_X10_COORD_OFFSET) as u8;
    let cx = (x_val + MOUSE_X10_COORD_OFFSET) as u8;
    let cy = (y_val + MOUSE_X10_COORD_OFFSET) as u8;

    let mut bytes = Vec::with_capacity(6);
    bytes.extend_from_slice(MOUSE_X10_PREFIX);
    bytes.push(cb);
    bytes.push(cx);
    bytes.push(cy);

    Some(bytes)
}

fn get_button_code(button: Button) -> u16 {
    match button {
        Button::Left => MOUSE_LEFT_BUTTON_CODE,
        Button::Middle => MOUSE_MIDDLE_BUTTON_CODE,
        Button::Right => MOUSE_RIGHT_BUTTON_CODE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, row, Button, KeyState, ModifierKeysMask, MouseInput, MouseInputKind};
    use core::num::NonZeroU16;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_legacy_mouse_sequence() {
        let nz_10 = NonZeroU16::new(10).unwrap();
        let term_col = crate::term_col(nz_10);
        let term_row = crate::term_row(nz_10);

        let cb_offset = MOUSE_X10_COORD_OFFSET as u8;
        let cx = (10 + MOUSE_X10_COORD_OFFSET) as u8;
        let cy = (10 + MOUSE_X10_COORD_OFFSET) as u8;

        // 1. Left click at x=10, y=10
        let input1 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseDown(Button::Left),
            maybe_modifier_keys: None,
        };
        let bytes1 = generate(&input1, term_col, term_row).unwrap();

        let mut expected1 = Vec::new();
        expected1.extend_from_slice(MOUSE_X10_PREFIX);
        expected1.push(MOUSE_LEFT_BUTTON_CODE as u8 + cb_offset);
        expected1.push(cx);
        expected1.push(cy);
        assert_eq!(bytes1, expected1);

        // 2. Left release at x=10, y=10
        let input2 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseUp(Button::Left),
            maybe_modifier_keys: None,
        };
        let bytes2 = generate(&input2, term_col, term_row).unwrap();

        let mut expected2 = Vec::new();
        expected2.extend_from_slice(MOUSE_X10_PREFIX);
        expected2.push(MOUSE_RELEASE_BUTTON_CODE as u8 + cb_offset);
        expected2.push(cx);
        expected2.push(cy);
        assert_eq!(bytes2, expected2);

        // 3. Right click with Shift modifier
        let mut modifiers = ModifierKeysMask::new();
        modifiers.shift_key_state = KeyState::Pressed;
        let input3 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseDown(Button::Right),
            maybe_modifier_keys: Some(modifiers),
        };
        let bytes3 = generate(&input3, term_col, term_row).unwrap();

        let mut expected_bytes3 = Vec::new();
        expected_bytes3.extend_from_slice(MOUSE_X10_PREFIX);
        expected_bytes3.push((MOUSE_RIGHT_BUTTON_CODE | MOUSE_MODIFIER_SHIFT) as u8 + cb_offset);
        expected_bytes3.push(cx);
        expected_bytes3.push(cy);
        assert_eq!(bytes3, expected_bytes3);

        // 4. Out of bounds coordinates (should return None)
        let nz_224 = NonZeroU16::new(224).unwrap();
        let term_col_oob = crate::term_col(nz_224);
        let bytes4 = generate(&input1, term_col_oob, term_row);
        assert!(bytes4.is_none());
    }
}

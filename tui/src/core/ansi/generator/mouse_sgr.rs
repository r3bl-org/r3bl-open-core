// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Button, KeyState, MouseInput, MouseInputKind, TermCol, TermRow,
            core::ansi::constants::{MOUSE_LEFT_BUTTON_CODE, MOUSE_MIDDLE_BUTTON_CODE,
                                    MOUSE_MODIFIER_ALT, MOUSE_MODIFIER_CTRL,
                                    MOUSE_MODIFIER_SHIFT, MOUSE_MOTION_FLAG,
                                    MOUSE_RELEASE_BUTTON_CODE, MOUSE_RIGHT_BUTTON_CODE,
                                    MOUSE_SCROLL_DOWN_BUTTON, MOUSE_SCROLL_LEFT_BUTTON,
                                    MOUSE_SCROLL_RIGHT_BUTTON, MOUSE_SCROLL_UP_BUTTON,
                                    MOUSE_SGR_PREFIX, MOUSE_SGR_PRESS,
                                    MOUSE_SGR_RELEASE}};


    /// Generates an [`ANSI`] [`SGR`] mouse sequence for the given `MouseInput`.
    ///
    /// The [`SGR`] mouse sequence format is:
    /// - `CSI < button_code ; x ; y M` (for press/scroll/drag/hover)
    /// - `CSI < button_code ; x ; y m` (for release)
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`SGR`]: crate::SgrCode
    #[must_use]
    pub fn generate(mouse_input: &MouseInput, x: TermCol, y: TermRow) -> Option<Vec<u8>> {
        let mut is_release = false;

        let mut button_code = match mouse_input.kind {
            MouseInputKind::MouseDown(button) => get_button_code(button),
            MouseInputKind::MouseUp(button) => {
                is_release = true;
                get_button_code(button)
            }
            MouseInputKind::MouseDrag(button) => {
                get_button_code(button) | MOUSE_MOTION_FLAG
            }
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

        let suffix = if is_release {
            MOUSE_SGR_RELEASE
        } else {
            MOUSE_SGR_PRESS
        };

        let prefix_str = std::str::from_utf8(MOUSE_SGR_PREFIX).unwrap_or("\x1b[<");

        Some(
            format!(
                "{}{};{};{}{}",
                prefix_str, button_code, x, y, suffix as char
            )
            .into_bytes(),
        )
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
    use crate::{Button, KeyState, ModifierKeysMask, MouseInput, MouseInputKind, col, row};
    use core::num::NonZeroU16;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_sgr_mouse_sequence() {
        let prefix_str = std::str::from_utf8(MOUSE_SGR_PREFIX).unwrap_or("\x1b[<");

        let nz_10 = NonZeroU16::new(10).unwrap();
        let term_col = crate::term_col(nz_10);
        let term_row = crate::term_row(nz_10);

        // 1. Left click at x=10, y=10
        let input1 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseDown(Button::Left),
            maybe_modifier_keys: None,
        };
        let bytes1 = generate(&input1, term_col, term_row).unwrap();
        assert_eq!(
            bytes1,
            format!(
                "{}{};10;10{}",
                prefix_str, MOUSE_LEFT_BUTTON_CODE, MOUSE_SGR_PRESS as char
            )
            .into_bytes()
        );

        // 2. Left release at x=10, y=10
        let input2 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseUp(Button::Left),
            maybe_modifier_keys: None,
        };
        let bytes2 = generate(&input2, term_col, term_row).unwrap();
        assert_eq!(
            bytes2,
            format!(
                "{}{};10;10{}",
                prefix_str, MOUSE_LEFT_BUTTON_CODE, MOUSE_SGR_RELEASE as char
            )
            .into_bytes()
        );

        // 3. Right click with Shift modifier
        let mut modifiers = ModifierKeysMask::new();
        modifiers.shift_key_state = KeyState::Pressed;
        let input3 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseDown(Button::Right),
            maybe_modifier_keys: Some(modifiers),
        };
        let bytes3 = generate(&input3, term_col, term_row).unwrap();
        assert_eq!(
            bytes3,
            format!(
                "{}{};10;10{}",
                prefix_str,
                MOUSE_RIGHT_BUTTON_CODE | MOUSE_MODIFIER_SHIFT,
                MOUSE_SGR_PRESS as char
            )
            .into_bytes()
        );

        // 4. Scroll Up
        let input4 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::ScrollUp,
            maybe_modifier_keys: None,
        };
        let bytes4 = generate(&input4, term_col, term_row).unwrap();
        assert_eq!(
            bytes4,
            format!(
                "{}{};10;10{}",
                prefix_str, MOUSE_SCROLL_UP_BUTTON, MOUSE_SGR_PRESS as char
            )
            .into_bytes()
        );

        // 5. Scroll Down
        let input5 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::ScrollDown,
            maybe_modifier_keys: None,
        };
        let bytes5 = generate(&input5, term_col, term_row).unwrap();
        assert_eq!(
            bytes5,
            format!(
                "{}{};10;10{}",
                prefix_str, MOUSE_SCROLL_DOWN_BUTTON, MOUSE_SGR_PRESS as char
            )
            .into_bytes()
        );

        // 6. Hover (Move without buttons)
        let input6 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseMove,
            maybe_modifier_keys: None,
        };
        let bytes6 = generate(&input6, term_col, term_row).unwrap();
        assert_eq!(
            bytes6,
            format!(
                "{}{};10;10{}",
                prefix_str,
                MOUSE_RELEASE_BUTTON_CODE | MOUSE_MOTION_FLAG,
                MOUSE_SGR_PRESS as char
            )
            .into_bytes()
        );

        // 7. Drag (Middle button)
        let input7 = MouseInput {
            pos: col(9) + row(9),
            kind: MouseInputKind::MouseDrag(Button::Middle),
            maybe_modifier_keys: None,
        };
        let bytes7 = generate(&input7, term_col, term_row).unwrap();
        assert_eq!(
            bytes7,
            format!(
                "{}{};10;10{}",
                prefix_str,
                MOUSE_MIDDLE_BUTTON_CODE | MOUSE_MOTION_FLAG,
                MOUSE_SGR_PRESS as char
            )
            .into_bytes()
        );
    }
}

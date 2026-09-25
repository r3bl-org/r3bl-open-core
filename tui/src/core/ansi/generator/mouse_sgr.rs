// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Button, KeyState, MouseInput, MouseInputKind, TermCol, TermRow,
            core::ansi::constants::{ANSI_PARAM_SEPARATOR_CHAR, CSI_START,
                                    MOUSE_LEFT_BUTTON_CODE, MOUSE_MIDDLE_BUTTON_CODE,
                                    MOUSE_MODIFIER_ALT, MOUSE_MODIFIER_CTRL,
                                    MOUSE_MODIFIER_SHIFT, MOUSE_MOTION_FLAG,
                                    MOUSE_RELEASE_BUTTON_CODE, MOUSE_RIGHT_BUTTON_CODE,
                                    MOUSE_SCROLL_DOWN_BUTTON, MOUSE_SCROLL_LEFT_BUTTON,
                                    MOUSE_SCROLL_RIGHT_BUTTON, MOUSE_SCROLL_UP_BUTTON,
                                    MOUSE_SGR_MARKER_CHAR, MOUSE_SGR_PRESS_CHAR,
                                    MOUSE_SGR_RELEASE_CHAR}};

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
    let x_val = x.as_u16();
    let y_val = y.as_u16();
    let mut is_release = false;

    let mut button_code = match mouse_input.kind {
        MouseInputKind::MouseDown(button) => get_button_code(button),
        MouseInputKind::MouseUp(button) => {
            is_release = true;
            get_button_code(button)
        }
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

    let suffix = if is_release {
        MOUSE_SGR_RELEASE_CHAR
    } else {
        MOUSE_SGR_PRESS_CHAR
    };

    Some(
        format!(
            "{CSI_START}{MOUSE_SGR_MARKER_CHAR}{button_code}\
            {ANSI_PARAM_SEPARATOR_CHAR}{x_val}\
            {ANSI_PARAM_SEPARATOR_CHAR}{y_val}\
            {suffix}"
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
    use crate::{Button, KeyState, MOUSE_SGR_PREFIX_STR, ModifierKeysMask, MouseInput,
                MouseInputKind, term_col, term_row, vp_col, vp_row};
    use core::num::NonZeroU16;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_sgr_mouse_sequence() {
        let prefix_str = MOUSE_SGR_PREFIX_STR.to_string();

        let term_col_val = term_col(NonZeroU16::new(10).unwrap());
        let term_row_val = term_row(NonZeroU16::new(10).unwrap());

        // 1. Left click at x=10, y=10
        let input1 = MouseInput {
            pos: vp_col(9) + vp_row(9),
            kind: MouseInputKind::MouseDown(Button::Left),
            maybe_modifier_keys: None,
        };
        let bytes1 =
            generate(&input1, term_col_val, term_row_val).expect("conversion error");
        assert_eq!(
            bytes1,
            format!(
                "{prefix_str}{MOUSE_LEFT_BUTTON_CODE}\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {MOUSE_SGR_PRESS_CHAR}"
            )
            .into_bytes()
        );

        // 2. Left release at x=10, y=10
        let input2 = MouseInput {
            pos: vp_col(9) + vp_row(9),
            kind: MouseInputKind::MouseUp(Button::Left),
            maybe_modifier_keys: None,
        };
        let bytes2 =
            generate(&input2, term_col_val, term_row_val).expect("conversion error");
        assert_eq!(
            bytes2,
            format!(
                "{prefix_str}{MOUSE_LEFT_BUTTON_CODE}\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {MOUSE_SGR_RELEASE_CHAR}"
            )
            .into_bytes()
        );

        // 3. Right click with Shift modifier
        let mut modifiers = ModifierKeysMask::new();
        modifiers.shift_key_state = KeyState::Pressed;
        let input3 = MouseInput {
            pos: vp_col(9) + vp_row(9),
            kind: MouseInputKind::MouseDown(Button::Right),
            maybe_modifier_keys: Some(modifiers),
        };
        let bytes3 =
            generate(&input3, term_col_val, term_row_val).expect("conversion error");
        assert_eq!(
            bytes3,
            format!(
                "{prefix_str}{a}\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {MOUSE_SGR_PRESS_CHAR}",
                a = MOUSE_RIGHT_BUTTON_CODE | MOUSE_MODIFIER_SHIFT
            )
            .into_bytes()
        );

        // 4. Scroll Up
        let input4 = MouseInput {
            pos: vp_col(9) + vp_row(9),
            kind: MouseInputKind::ScrollUp,
            maybe_modifier_keys: None,
        };
        let bytes4 =
            generate(&input4, term_col_val, term_row_val).expect("conversion error");
        assert_eq!(
            bytes4,
            format!(
                "{prefix_str}{MOUSE_SCROLL_UP_BUTTON}\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {MOUSE_SGR_PRESS_CHAR}"
            )
            .into_bytes()
        );

        // 5. Scroll Down
        let input5 = MouseInput {
            pos: vp_col(9) + vp_row(9),
            kind: MouseInputKind::ScrollDown,
            maybe_modifier_keys: None,
        };
        let bytes5 =
            generate(&input5, term_col_val, term_row_val).expect("conversion error");
        assert_eq!(
            bytes5,
            format!(
                "{prefix_str}{MOUSE_SCROLL_DOWN_BUTTON}\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {MOUSE_SGR_PRESS_CHAR}"
            )
            .into_bytes()
        );

        // 6. Hover (Move without buttons)
        let input6 = MouseInput {
            pos: vp_col(9) + vp_row(9),
            kind: MouseInputKind::MouseMove,
            maybe_modifier_keys: None,
        };
        let bytes6 =
            generate(&input6, term_col_val, term_row_val).expect("conversion error");
        assert_eq!(
            bytes6,
            format!(
                "{prefix_str}{a}\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {MOUSE_SGR_PRESS_CHAR}",
                a = MOUSE_RELEASE_BUTTON_CODE | MOUSE_MOTION_FLAG
            )
            .into_bytes()
        );

        // 7. Drag (Middle button)
        let input7 = MouseInput {
            pos: vp_col(9) + vp_row(9),
            kind: MouseInputKind::MouseDrag(Button::Middle),
            maybe_modifier_keys: None,
        };
        let bytes7 =
            generate(&input7, term_col_val, term_row_val).expect("conversion error");
        assert_eq!(
            bytes7,
            format!(
                "{prefix_str}{a}{ANSI_PARAM_SEPARATOR_CHAR}10\
                {ANSI_PARAM_SEPARATOR_CHAR}10\
                {MOUSE_SGR_PRESS_CHAR}",
                a = MOUSE_MIDDLE_BUTTON_CODE | MOUSE_MOTION_FLAG
            )
            .into_bytes()
        );
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Protocol conversion layer: VT-100 IR → Public API types.
//!
//! This module converts protocol-level intermediate representation (IR) from the
//! VT-100 parser into the public API types that applications consume. This layer
//! decouples protocol-specific details from the stable public API.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Raw ANSI bytes: "\x1B[A"                                        │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//! ┌────────────────────────────▼────────────────────────────────────┐
//! │ vt_100_terminal_input_parser/ (Protocol Layer - IR)             │
//! │   parse_keyboard_sequence() → VT100InputEvent::Keyboard         │
//! │   parse_mouse_sequence()    → VT100InputEvent::Mouse            │
//! │   VT100KeyCode, VT100KeyModifiers, VT100MouseButton, etc.       │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//! ┌────────────────────────────▼────────────────────────────────────┐
//! │ protocol_conversion.rs (THIS MODULE - IR → Public API)          │
//! │   convert_input_event()       VT100InputEvent → InputEvent      │
//! │   convert_key_code_to_keypress()  VT100KeyCode → KeyPress       │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//! ┌────────────────────────────▼────────────────────────────────────┐
//! │ Public API (Application Layer)                                  │
//! │   InputEvent::Keyboard(KeyPress)                                │
//! │   Key, KeyPress, MouseInput, FocusEvent, etc.                   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why This Layer Exists
//!
//! - **Protocol independence**: Applications depend on stable `InputEvent` types, not
//!   VT-100 specifics
//! - **Multi-backend support**: Future backends (e.g., Windows Console API) can convert
//!   their IR to the same public API
//! - **Type safety**: Protocol types use VT-100 nomenclature; public API uses
//!   domain-appropriate names
//! - **Evolution**: Protocol can change without breaking application code

use crate::{Button, ColWidth, FocusEvent, InputEvent, Key, KeyPress, KeyState,
            ModifierKeysMask, MouseInput, MouseInputKind, Pos, RowHeight, SpecialKey,
            core::ansi::vt_100_terminal_input_parser::{VT100FocusState,
                                                       VT100InputEvent, VT100KeyCode,
                                                       VT100KeyModifiers,
                                                       VT100MouseAction,
                                                       VT100MouseButton,
                                                       VT100ScrollDirection}};

/// Convert protocol-level `VT100KeyCode` and `VT100KeyModifiers` to canonical `KeyPress`.
///
/// Maps VT-100 IR key codes to the public API `Key` enum, handling:
/// - Character keys: `VT100KeyCode::Char` → `Key::Character`
/// - Function keys: `VT100KeyCode::Function(n)` → `Key::FunctionKey(Fn)`
/// - Special keys: `VT100KeyCode::Up` → `Key::SpecialKey(SpecialKey::Up)`
/// - Modifiers: Shift, Ctrl, Alt masks
///
/// Returns:
/// - `KeyPress::Plain` if no modifiers are active
/// - `KeyPress::WithModifiers` if any modifiers (Shift/Ctrl/Alt) are pressed
pub(super) fn convert_key_code_to_keypress(
    code: VT100KeyCode,
    modifiers: VT100KeyModifiers,
) -> KeyPress {
    let key = match code {
        VT100KeyCode::Char(ch) => Key::Character(ch),
        VT100KeyCode::Function(n) => {
            use crate::FunctionKey;
            match n {
                1 => Key::FunctionKey(FunctionKey::F1),
                2 => Key::FunctionKey(FunctionKey::F2),
                3 => Key::FunctionKey(FunctionKey::F3),
                4 => Key::FunctionKey(FunctionKey::F4),
                5 => Key::FunctionKey(FunctionKey::F5),
                6 => Key::FunctionKey(FunctionKey::F6),
                7 => Key::FunctionKey(FunctionKey::F7),
                8 => Key::FunctionKey(FunctionKey::F8),
                9 => Key::FunctionKey(FunctionKey::F9),
                10 => Key::FunctionKey(FunctionKey::F10),
                11 => Key::FunctionKey(FunctionKey::F11),
                12 => Key::FunctionKey(FunctionKey::F12),
                _ => Key::Character('?'), // Fallback
            }
        }
        VT100KeyCode::Up => Key::SpecialKey(SpecialKey::Up),
        VT100KeyCode::Down => Key::SpecialKey(SpecialKey::Down),
        VT100KeyCode::Left => Key::SpecialKey(SpecialKey::Left),
        VT100KeyCode::Right => Key::SpecialKey(SpecialKey::Right),
        VT100KeyCode::Home => Key::SpecialKey(SpecialKey::Home),
        VT100KeyCode::End => Key::SpecialKey(SpecialKey::End),
        VT100KeyCode::PageUp => Key::SpecialKey(SpecialKey::PageUp),
        VT100KeyCode::PageDown => Key::SpecialKey(SpecialKey::PageDown),
        VT100KeyCode::Tab => Key::SpecialKey(SpecialKey::Tab),
        VT100KeyCode::BackTab => Key::SpecialKey(SpecialKey::BackTab),
        VT100KeyCode::Delete => Key::SpecialKey(SpecialKey::Delete),
        VT100KeyCode::Insert => Key::SpecialKey(SpecialKey::Insert),
        VT100KeyCode::Enter => Key::SpecialKey(SpecialKey::Enter),
        VT100KeyCode::Backspace => Key::SpecialKey(SpecialKey::Backspace),
        VT100KeyCode::Escape => Key::SpecialKey(SpecialKey::Esc),
    };

    // Convert modifiers (now using canonical KeyState directly)
    let mask = ModifierKeysMask {
        shift_key_state: modifiers.shift,
        ctrl_key_state: modifiers.ctrl,
        alt_key_state: modifiers.alt,
    };

    if mask.shift_key_state == KeyState::NotPressed
        && mask.ctrl_key_state == KeyState::NotPressed
        && mask.alt_key_state == KeyState::NotPressed
    {
        KeyPress::Plain { key }
    } else {
        KeyPress::WithModifiers { key, mask }
    }
}

/// Convert protocol-level `VT100InputEvent` to canonical `InputEvent`.
///
/// Converts all VT-100 IR event types to public API types:
/// - **Keyboard**: `VT100InputEvent::Keyboard` → `InputEvent::Keyboard(KeyPress)`
/// - **Mouse**: `VT100InputEvent::Mouse` → `InputEvent::Mouse(MouseInput)`
///   - Converts button types: `VT100MouseButton::Left` → `Button::Left`
///   - Converts actions: `VT100MouseAction::Press` → `MouseInputKind::MouseDown`
///   - Converts coordinates: 1-based `TermPos` → 0-based `Pos`
/// - **Resize**: `VT100InputEvent::Resize` → `InputEvent::Resize(Size)`
/// - **Focus**: `VT100InputEvent::Focus` → `InputEvent::Focus(FocusEvent)`
/// - **Paste**: Should never be called (handled by state machine in `try_read_event()`)
///
/// Returns `None` if the event cannot be converted (e.g., unknown mouse button).
pub(super) fn convert_input_event(vt100_event: VT100InputEvent) -> Option<InputEvent> {
    match vt100_event {
        VT100InputEvent::Keyboard { code, modifiers } => {
            let keypress = convert_key_code_to_keypress(code, modifiers);
            Some(InputEvent::Keyboard(keypress))
        }
        VT100InputEvent::Mouse {
            button,
            pos,
            action,
            modifiers,
        } => {
            let button_kind = match button {
                VT100MouseButton::Left => Button::Left,
                VT100MouseButton::Right => Button::Right,
                VT100MouseButton::Middle => Button::Middle,
                VT100MouseButton::Unknown => return None,
            };

            let kind = match action {
                VT100MouseAction::Press => MouseInputKind::MouseDown(button_kind),
                VT100MouseAction::Release => MouseInputKind::MouseUp(button_kind),
                VT100MouseAction::Drag => MouseInputKind::MouseDrag(button_kind),
                VT100MouseAction::Motion => MouseInputKind::MouseMove,
                VT100MouseAction::Scroll(direction) => match direction {
                    VT100ScrollDirection::Up => MouseInputKind::ScrollUp,
                    VT100ScrollDirection::Down => MouseInputKind::ScrollDown,
                    VT100ScrollDirection::Left => MouseInputKind::ScrollLeft,
                    VT100ScrollDirection::Right => MouseInputKind::ScrollRight,
                },
            };

            let maybe_modifier_keys = if modifiers.shift == KeyState::Pressed
                || modifiers.ctrl == KeyState::Pressed
                || modifiers.alt == KeyState::Pressed
            {
                Some(ModifierKeysMask {
                    shift_key_state: modifiers.shift,
                    ctrl_key_state: modifiers.ctrl,
                    alt_key_state: modifiers.alt,
                })
            } else {
                None
            };

            // Convert TermPos to Pos (convert from 1-based to 0-based)
            // TermCol and TermRow have built-in conversion to 0-based indices
            let canonical_pos = Pos {
                col_index: pos.col.to_zero_based(),
                row_index: pos.row.to_zero_based(),
            };

            let mouse_input = MouseInput {
                pos: canonical_pos,
                kind,
                maybe_modifier_keys,
            };
            Some(InputEvent::Mouse(mouse_input))
        }
        VT100InputEvent::Resize { rows, cols } => Some(InputEvent::Resize(crate::Size {
            col_width: ColWidth::from(cols),
            row_height: RowHeight::from(rows),
        })),
        VT100InputEvent::Focus(focus_state) => {
            let event = match focus_state {
                VT100FocusState::Gained => FocusEvent::Gained,
                VT100FocusState::Lost => FocusEvent::Lost,
            };
            Some(InputEvent::Focus(event))
        }
        VT100InputEvent::Paste(_paste_mode) => {
            unreachable!(
                "Paste events are handled by state machine in try_read_event() \
                 and should never reach convert_input_event()"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionKey, TermPos, col, row};

    // MARK: Keyboard conversion tests

    #[test]
    fn test_convert_character_keys() {
        // Test regular character keys without modifiers
        let test_cases = vec![
            ('a', Key::Character('a')),
            ('Z', Key::Character('Z')),
            ('5', Key::Character('5')),
            (' ', Key::Character(' ')),
            ('!', Key::Character('!')),
        ];

        for (ch, expected_key) in test_cases {
            let result = convert_key_code_to_keypress(
                VT100KeyCode::Char(ch),
                VT100KeyModifiers::default(),
            );

            match result {
                KeyPress::Plain { key } => {
                    assert_eq!(
                        key, expected_key,
                        "Character '{}' should convert correctly",
                        ch
                    );
                }
                KeyPress::WithModifiers { .. } => {
                    panic!("Expected Plain keypress for character '{}'", ch);
                }
            }
        }
    }

    #[test]
    fn test_convert_function_keys() {
        // Test all function keys F1-F12
        let test_cases = vec![
            (1, FunctionKey::F1),
            (2, FunctionKey::F2),
            (3, FunctionKey::F3),
            (4, FunctionKey::F4),
            (5, FunctionKey::F5),
            (6, FunctionKey::F6),
            (7, FunctionKey::F7),
            (8, FunctionKey::F8),
            (9, FunctionKey::F9),
            (10, FunctionKey::F10),
            (11, FunctionKey::F11),
            (12, FunctionKey::F12),
        ];

        for (n, expected_fn_key) in test_cases {
            let result = convert_key_code_to_keypress(
                VT100KeyCode::Function(n),
                VT100KeyModifiers::default(),
            );

            match result {
                KeyPress::Plain {
                    key: Key::FunctionKey(fn_key),
                } => {
                    assert_eq!(
                        fn_key, expected_fn_key,
                        "Function key F{} should convert correctly",
                        n
                    );
                }
                _ => panic!("Expected Plain FunctionKey for F{}", n),
            }
        }
    }

    #[test]
    fn test_convert_function_key_out_of_range() {
        // Test fallback for out-of-range function key numbers
        let result = convert_key_code_to_keypress(
            VT100KeyCode::Function(99),
            VT100KeyModifiers::default(),
        );

        match result {
            KeyPress::Plain {
                key: Key::Character('?'),
            } => {
                // Correct fallback
            }
            _ => panic!("Expected fallback to '?' for out-of-range function key"),
        }
    }

    #[test]
    fn test_convert_arrow_keys() {
        // Test all arrow keys
        let test_cases = vec![
            (VT100KeyCode::Up, SpecialKey::Up),
            (VT100KeyCode::Down, SpecialKey::Down),
            (VT100KeyCode::Left, SpecialKey::Left),
            (VT100KeyCode::Right, SpecialKey::Right),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiers::default());

            match result {
                KeyPress::Plain {
                    key: Key::SpecialKey(special_key),
                } => {
                    assert_eq!(special_key, expected_special_key);
                }
                _ => panic!("Expected Plain SpecialKey for arrow key"),
            }
        }
    }

    #[test]
    fn test_convert_navigation_keys() {
        // Test navigation keys (Home, End, PageUp, PageDown)
        let test_cases = vec![
            (VT100KeyCode::Home, SpecialKey::Home),
            (VT100KeyCode::End, SpecialKey::End),
            (VT100KeyCode::PageUp, SpecialKey::PageUp),
            (VT100KeyCode::PageDown, SpecialKey::PageDown),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiers::default());

            match result {
                KeyPress::Plain {
                    key: Key::SpecialKey(special_key),
                } => {
                    assert_eq!(special_key, expected_special_key);
                }
                _ => panic!("Expected Plain SpecialKey for navigation key"),
            }
        }
    }

    #[test]
    fn test_convert_editing_keys() {
        // Test editing keys (Insert, Delete, Backspace, Enter, Esc)
        let test_cases = vec![
            (VT100KeyCode::Insert, SpecialKey::Insert),
            (VT100KeyCode::Delete, SpecialKey::Delete),
            (VT100KeyCode::Backspace, SpecialKey::Backspace),
            (VT100KeyCode::Enter, SpecialKey::Enter),
            (VT100KeyCode::Escape, SpecialKey::Esc),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiers::default());

            match result {
                KeyPress::Plain {
                    key: Key::SpecialKey(special_key),
                } => {
                    assert_eq!(special_key, expected_special_key);
                }
                _ => panic!("Expected Plain SpecialKey for editing key"),
            }
        }
    }

    #[test]
    fn test_convert_tab_keys() {
        // Test Tab and BackTab (Shift+Tab)
        let test_cases = vec![
            (VT100KeyCode::Tab, SpecialKey::Tab),
            (VT100KeyCode::BackTab, SpecialKey::BackTab),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiers::default());

            match result {
                KeyPress::Plain {
                    key: Key::SpecialKey(special_key),
                } => {
                    assert_eq!(special_key, expected_special_key);
                }
                _ => panic!("Expected Plain SpecialKey for tab key"),
            }
        }
    }

    #[test]
    fn test_convert_with_shift_modifier() {
        // Test key with Shift modifier
        let modifiers = VT100KeyModifiers {
            shift: KeyState::Pressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCode::Char('a'), modifiers);

        match result {
            KeyPress::WithModifiers { key, mask } => {
                assert_eq!(key, Key::Character('a'));
                assert_eq!(mask.shift_key_state, KeyState::Pressed);
                assert_eq!(mask.ctrl_key_state, KeyState::NotPressed);
                assert_eq!(mask.alt_key_state, KeyState::NotPressed);
            }
            KeyPress::Plain { .. } => panic!("Expected WithModifiers for Shift+key"),
        }
    }

    #[test]
    fn test_convert_with_ctrl_modifier() {
        // Test key with Ctrl modifier
        let modifiers = VT100KeyModifiers {
            shift: KeyState::NotPressed,
            ctrl: KeyState::Pressed,
            alt: KeyState::NotPressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCode::Char('c'), modifiers);

        match result {
            KeyPress::WithModifiers { key, mask } => {
                assert_eq!(key, Key::Character('c'));
                assert_eq!(mask.shift_key_state, KeyState::NotPressed);
                assert_eq!(mask.ctrl_key_state, KeyState::Pressed);
                assert_eq!(mask.alt_key_state, KeyState::NotPressed);
            }
            KeyPress::Plain { .. } => panic!("Expected WithModifiers for Ctrl+key"),
        }
    }

    #[test]
    fn test_convert_with_alt_modifier() {
        // Test key with Alt modifier
        let modifiers = VT100KeyModifiers {
            shift: KeyState::NotPressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::Pressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCode::Left, modifiers);

        match result {
            KeyPress::WithModifiers { key, mask } => {
                assert_eq!(key, Key::SpecialKey(SpecialKey::Left));
                assert_eq!(mask.shift_key_state, KeyState::NotPressed);
                assert_eq!(mask.ctrl_key_state, KeyState::NotPressed);
                assert_eq!(mask.alt_key_state, KeyState::Pressed);
            }
            KeyPress::Plain { .. } => panic!("Expected WithModifiers for Alt+key"),
        }
    }

    #[test]
    fn test_convert_with_multiple_modifiers() {
        // Test key with Ctrl+Shift+Alt
        let modifiers = VT100KeyModifiers {
            shift: KeyState::Pressed,
            ctrl: KeyState::Pressed,
            alt: KeyState::Pressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCode::Function(5), modifiers);

        match result {
            KeyPress::WithModifiers { key, mask } => {
                assert_eq!(key, Key::FunctionKey(FunctionKey::F5));
                assert_eq!(mask.shift_key_state, KeyState::Pressed);
                assert_eq!(mask.ctrl_key_state, KeyState::Pressed);
                assert_eq!(mask.alt_key_state, KeyState::Pressed);
            }
            KeyPress::Plain { .. } => {
                panic!("Expected WithModifiers for Ctrl+Shift+Alt+F5")
            }
        }
    }

    // MARK: Mouse conversion tests

    #[test]
    fn test_convert_mouse_buttons() {
        // Test all mouse button types
        let test_cases = vec![
            (VT100MouseButton::Left, Button::Left),
            (VT100MouseButton::Right, Button::Right),
            (VT100MouseButton::Middle, Button::Middle),
        ];

        for (vt100_button, expected_button) in test_cases {
            let vt100_event = VT100InputEvent::Mouse {
                button: vt100_button,
                pos: TermPos::from_one_based(1, 1),
                action: VT100MouseAction::Press,
                modifiers: VT100KeyModifiers::default(),
            };

            let result = convert_input_event(vt100_event);
            match result {
                Some(InputEvent::Mouse(mouse_input)) => match mouse_input.kind {
                    MouseInputKind::MouseDown(button) => {
                        assert_eq!(button, expected_button);
                    }
                    _ => panic!("Expected MouseDown action"),
                },
                _ => panic!("Expected Mouse event"),
            }
        }
    }

    #[test]
    fn test_convert_mouse_unknown_button() {
        // Test that Unknown button returns None
        let vt100_event = VT100InputEvent::Mouse {
            button: VT100MouseButton::Unknown,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseAction::Press,
            modifiers: VT100KeyModifiers::default(),
        };

        let result = convert_input_event(vt100_event);
        assert!(result.is_none(), "Unknown mouse button should return None");
    }

    #[test]
    fn test_convert_mouse_actions() {
        // Test all mouse action types
        let button = VT100MouseButton::Left;

        // Press
        let vt100_event = VT100InputEvent::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseAction::Press,
            modifiers: VT100KeyModifiers::default(),
        };
        match convert_input_event(vt100_event) {
            Some(InputEvent::Mouse(mouse_input)) => {
                assert!(matches!(
                    mouse_input.kind,
                    MouseInputKind::MouseDown(Button::Left)
                ));
            }
            _ => panic!("Expected MouseDown event"),
        }

        // Release
        let vt100_event = VT100InputEvent::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseAction::Release,
            modifiers: VT100KeyModifiers::default(),
        };
        match convert_input_event(vt100_event) {
            Some(InputEvent::Mouse(mouse_input)) => {
                assert!(matches!(
                    mouse_input.kind,
                    MouseInputKind::MouseUp(Button::Left)
                ));
            }
            _ => panic!("Expected MouseUp event"),
        }

        // Drag
        let vt100_event = VT100InputEvent::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseAction::Drag,
            modifiers: VT100KeyModifiers::default(),
        };
        match convert_input_event(vt100_event) {
            Some(InputEvent::Mouse(mouse_input)) => {
                assert!(matches!(
                    mouse_input.kind,
                    MouseInputKind::MouseDrag(Button::Left)
                ));
            }
            _ => panic!("Expected MouseDrag event"),
        }

        // Motion
        let vt100_event = VT100InputEvent::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseAction::Motion,
            modifiers: VT100KeyModifiers::default(),
        };
        match convert_input_event(vt100_event) {
            Some(InputEvent::Mouse(mouse_input)) => {
                assert!(matches!(mouse_input.kind, MouseInputKind::MouseMove));
            }
            _ => panic!("Expected MouseMove event"),
        }
    }

    #[test]
    fn test_convert_mouse_scroll_directions() {
        // Test all scroll directions
        let test_cases = vec![
            (VT100ScrollDirection::Up, MouseInputKind::ScrollUp),
            (VT100ScrollDirection::Down, MouseInputKind::ScrollDown),
            (VT100ScrollDirection::Left, MouseInputKind::ScrollLeft),
            (VT100ScrollDirection::Right, MouseInputKind::ScrollRight),
        ];

        for (vt100_dir, expected_kind) in test_cases {
            let vt100_event = VT100InputEvent::Mouse {
                button: VT100MouseButton::Left,
                pos: TermPos::from_one_based(1, 1),
                action: VT100MouseAction::Scroll(vt100_dir),
                modifiers: VT100KeyModifiers::default(),
            };

            match convert_input_event(vt100_event) {
                Some(InputEvent::Mouse(mouse_input)) => {
                    assert_eq!(mouse_input.kind, expected_kind);
                }
                _ => panic!("Expected Mouse scroll event"),
            }
        }
    }

    #[test]
    fn test_convert_mouse_position_conversion() {
        // Test 1-based TermPos → 0-based Pos conversion
        let test_cases = vec![
            (TermPos::from_one_based(1, 1), col(0) + row(0)),
            (TermPos::from_one_based(5, 10), col(4) + row(9)),
            (TermPos::from_one_based(80, 24), col(79) + row(23)),
        ];

        for (term_pos, expected_pos) in test_cases {
            let vt100_event = VT100InputEvent::Mouse {
                button: VT100MouseButton::Left,
                pos: term_pos,
                action: VT100MouseAction::Press,
                modifiers: VT100KeyModifiers::default(),
            };

            match convert_input_event(vt100_event) {
                Some(InputEvent::Mouse(mouse_input)) => {
                    assert_eq!(
                        mouse_input.pos, expected_pos,
                        "Position conversion failed for {:?}",
                        term_pos
                    );
                }
                _ => panic!("Expected Mouse event"),
            }
        }
    }

    #[test]
    fn test_convert_mouse_with_modifiers() {
        // Test mouse event with Shift modifier
        let modifiers = VT100KeyModifiers {
            shift: KeyState::Pressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        };

        let vt100_event = VT100InputEvent::Mouse {
            button: VT100MouseButton::Left,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseAction::Press,
            modifiers,
        };

        match convert_input_event(vt100_event) {
            Some(InputEvent::Mouse(mouse_input)) => {
                assert!(mouse_input.maybe_modifier_keys.is_some());
                let mask = mouse_input.maybe_modifier_keys.unwrap();
                assert_eq!(mask.shift_key_state, KeyState::Pressed);
                assert_eq!(mask.ctrl_key_state, KeyState::NotPressed);
                assert_eq!(mask.alt_key_state, KeyState::NotPressed);
            }
            _ => panic!("Expected Mouse event with modifiers"),
        }
    }

    #[test]
    fn test_convert_mouse_without_modifiers() {
        // Test mouse event with no modifiers (should have None for maybe_modifier_keys)
        let vt100_event = VT100InputEvent::Mouse {
            button: VT100MouseButton::Left,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseAction::Press,
            modifiers: VT100KeyModifiers::default(),
        };

        match convert_input_event(vt100_event) {
            Some(InputEvent::Mouse(mouse_input)) => {
                assert!(
                    mouse_input.maybe_modifier_keys.is_none(),
                    "Mouse event without modifiers should have None for maybe_modifier_keys"
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    // MARK: Resize and Focus conversion tests

    #[test]
    fn test_convert_resize_event() {
        // Test resize event conversion
        let test_cases = vec![(80, 24), (120, 40), (1, 1), (200, 60)];

        for (cols, rows) in test_cases {
            let vt100_event = VT100InputEvent::Resize { rows, cols };

            match convert_input_event(vt100_event) {
                Some(InputEvent::Resize(size)) => {
                    assert_eq!(
                        size.col_width,
                        ColWidth::from(cols),
                        "Column width mismatch for cols={}",
                        cols
                    );
                    assert_eq!(
                        size.row_height,
                        RowHeight::from(rows),
                        "Row height mismatch for rows={}",
                        rows
                    );
                }
                _ => panic!("Expected Resize event for {}x{}", cols, rows),
            }
        }
    }

    #[test]
    fn test_convert_focus_gained() {
        // Test focus gained event
        let vt100_event = VT100InputEvent::Focus(VT100FocusState::Gained);

        match convert_input_event(vt100_event) {
            Some(InputEvent::Focus(FocusEvent::Gained)) => {
                // Correct conversion
            }
            _ => panic!("Expected Focus(Gained) event"),
        }
    }

    #[test]
    fn test_convert_focus_lost() {
        // Test focus lost event
        let vt100_event = VT100InputEvent::Focus(VT100FocusState::Lost);

        match convert_input_event(vt100_event) {
            Some(InputEvent::Focus(FocusEvent::Lost)) => {
                // Correct conversion
            }
            _ => panic!("Expected Focus(Lost) event"),
        }
    }

    // MARK: Input event integration tests

    #[test]
    fn test_convert_keyboard_event() {
        // Test full keyboard event conversion path
        let vt100_event = VT100InputEvent::Keyboard {
            code: VT100KeyCode::Char('x'),
            modifiers: VT100KeyModifiers {
                shift: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
                alt: KeyState::NotPressed,
            },
        };

        match convert_input_event(vt100_event) {
            Some(InputEvent::Keyboard(keypress)) => match keypress {
                KeyPress::WithModifiers { key, mask } => {
                    assert_eq!(key, Key::Character('x'));
                    assert_eq!(mask.ctrl_key_state, KeyState::Pressed);
                }
                _ => panic!("Expected WithModifiers keypress"),
            },
            _ => panic!("Expected Keyboard event"),
        }
    }
}

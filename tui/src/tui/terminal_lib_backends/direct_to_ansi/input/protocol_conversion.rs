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
//! │   parse_keyboard_sequence() → VT100InputEventIR::Keyboard       │
//! │   parse_mouse_sequence()    → VT100InputEventIR::Mouse          │
//! │   VT100KeyCodeIR, VT100KeyModifiersIR, VT100MouseButtonIR, etc. │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//! ┌────────────────────────────▼────────────────────────────────────┐
//! │ protocol_conversion.rs (THIS MODULE - IR → Public API)          │
//! │   convert_input_event()       VT100InputEventIR → InputEvent    │
//! │   convert_key_code_to_keypress()  VT100KeyCodeIR → KeyPress     │
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

use crate::{Button, FocusEvent, InputEvent, Key, KeyPress, KeyState, ModifierKeysMask,
            MouseInput, MouseInputKind, Pos, SpecialKey,
            core::ansi::vt_100_terminal_input_parser::{VT100FocusStateIR,
                                                       VT100InputEventIR,
                                                       VT100KeyCodeIR,
                                                       VT100KeyModifiersIR,
                                                       VT100MouseActionIR,
                                                       VT100MouseButtonIR,
                                                       VT100ScrollDirectionIR}};

/// Convert protocol-level [`VT100InputEventIR`] to canonical [`InputEvent`].
///
/// Converts all VT-100 IR event types to public API types:
/// - **Keyboard**: [`VT100InputEventIR::Keyboard`] → [`InputEvent::Keyboard`]
/// - **Mouse**: [`VT100InputEventIR::Mouse`] → [`InputEvent::Mouse`]
///   - Converts button types: [`VT100MouseButtonIR::Left`] → [`Button::Left`]
///   - Converts actions: [`VT100MouseActionIR::Press`] → [`MouseInputKind::MouseDown`]
///   - Converts coordinates: 1-based [`TermPos`] → 0-based [`Pos`]
/// - **Resize**: [`VT100InputEventIR::Resize`] → [`InputEvent::Resize`]
/// - **Focus**: [`VT100InputEventIR::Focus`] → [`InputEvent::Focus`]
/// - **Paste**: Should never be called (handled by state machine in `next()`)
///
/// Returns `None` if the event cannot be converted (e.g., unknown mouse button).
///
/// [`TermPos`]: crate::TermPos
#[must_use]
pub fn convert_input_event(vt100_event: VT100InputEventIR) -> Option<InputEvent> {
    match vt100_event {
        VT100InputEventIR::Keyboard { code, modifiers } => {
            let keypress = convert_key_code_to_keypress(code, modifiers);
            Some(InputEvent::Keyboard(keypress))
        }
        VT100InputEventIR::Mouse {
            button,
            pos,
            action,
            modifiers,
        } => {
            let button_kind = match button {
                VT100MouseButtonIR::Left => Button::Left,
                VT100MouseButtonIR::Right => Button::Right,
                VT100MouseButtonIR::Middle => Button::Middle,
                VT100MouseButtonIR::Unknown => return None,
            };

            let kind = match action {
                VT100MouseActionIR::Press => MouseInputKind::MouseDown(button_kind),
                VT100MouseActionIR::Release => MouseInputKind::MouseUp(button_kind),
                VT100MouseActionIR::Drag => MouseInputKind::MouseDrag(button_kind),
                VT100MouseActionIR::Motion => MouseInputKind::MouseMove,
                VT100MouseActionIR::Scroll(direction) => match direction {
                    VT100ScrollDirectionIR::Up => MouseInputKind::ScrollUp,
                    VT100ScrollDirectionIR::Down => MouseInputKind::ScrollDown,
                    VT100ScrollDirectionIR::Left => MouseInputKind::ScrollLeft,
                    VT100ScrollDirectionIR::Right => MouseInputKind::ScrollRight,
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
        VT100InputEventIR::Resize {
            col_width,
            row_height,
        } => Some(InputEvent::Resize(crate::Size {
            col_width,
            row_height,
        })),
        VT100InputEventIR::Focus(focus_state) => {
            let event = match focus_state {
                VT100FocusStateIR::Gained => FocusEvent::Gained,
                VT100FocusStateIR::Lost => FocusEvent::Lost,
            };
            Some(InputEvent::Focus(event))
        }
        VT100InputEventIR::Paste(_paste_mode) => {
            unreachable!(
                "Paste events are handled by state machine in next() \
                 and should never reach convert_input_event()"
            )
        }
    }
}

/// Convert protocol-level [`VT100KeyCodeIR`] and [`VT100KeyModifiersIR`] to canonical
/// [`KeyPress`].
///
/// Maps VT-100 IR key codes to the public API [`Key`] enum, handling:
/// - Character keys: [`VT100KeyCodeIR::Char`] → [`Key::Character`]
/// - Function keys: [`VT100KeyCodeIR::Function`] → [`Key::FunctionKey`]
/// - Special keys: [`VT100KeyCodeIR::Up`] → [`Key::SpecialKey`]
/// - Modifiers: Shift, Ctrl, Alt masks
///
/// Returns:
/// - [`KeyPress::Plain`] if no modifiers are active
/// - [`KeyPress::WithModifiers`] if any modifiers (Shift/Ctrl/Alt) are pressed
fn convert_key_code_to_keypress(
    code: VT100KeyCodeIR,
    modifiers: VT100KeyModifiersIR,
) -> KeyPress {
    let key = match code {
        VT100KeyCodeIR::Char(ch) => Key::Character(ch),
        VT100KeyCodeIR::Function(n) => {
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
        VT100KeyCodeIR::Up => Key::SpecialKey(SpecialKey::Up),
        VT100KeyCodeIR::Down => Key::SpecialKey(SpecialKey::Down),
        VT100KeyCodeIR::Left => Key::SpecialKey(SpecialKey::Left),
        VT100KeyCodeIR::Right => Key::SpecialKey(SpecialKey::Right),
        VT100KeyCodeIR::Home => Key::SpecialKey(SpecialKey::Home),
        VT100KeyCodeIR::End => Key::SpecialKey(SpecialKey::End),
        VT100KeyCodeIR::PageUp => Key::SpecialKey(SpecialKey::PageUp),
        VT100KeyCodeIR::PageDown => Key::SpecialKey(SpecialKey::PageDown),
        VT100KeyCodeIR::Tab => Key::SpecialKey(SpecialKey::Tab),
        VT100KeyCodeIR::BackTab => Key::SpecialKey(SpecialKey::BackTab),
        VT100KeyCodeIR::Delete => Key::SpecialKey(SpecialKey::Delete),
        VT100KeyCodeIR::Insert => Key::SpecialKey(SpecialKey::Insert),
        VT100KeyCodeIR::Enter => Key::SpecialKey(SpecialKey::Enter),
        VT100KeyCodeIR::Backspace => Key::SpecialKey(SpecialKey::Backspace),
        VT100KeyCodeIR::Escape => Key::SpecialKey(SpecialKey::Esc),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColWidth, FunctionKey, RowHeight, TermPos, col, row};

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
                VT100KeyCodeIR::Char(ch),
                VT100KeyModifiersIR::default(),
            );

            match result {
                KeyPress::Plain { key } => {
                    assert_eq!(
                        key, expected_key,
                        "Character '{ch}' should convert correctly"
                    );
                }
                KeyPress::WithModifiers { .. } => {
                    panic!("Expected Plain keypress for character '{ch}'");
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
                VT100KeyCodeIR::Function(n),
                VT100KeyModifiersIR::default(),
            );

            match result {
                KeyPress::Plain {
                    key: Key::FunctionKey(fn_key),
                } => {
                    assert_eq!(
                        fn_key, expected_fn_key,
                        "Function key F{n} should convert correctly"
                    );
                }
                _ => panic!("Expected Plain FunctionKey for F{n}"),
            }
        }
    }

    #[test]
    fn test_convert_function_key_out_of_range() {
        // Test fallback for out-of-range function key numbers
        let result = convert_key_code_to_keypress(
            VT100KeyCodeIR::Function(99),
            VT100KeyModifiersIR::default(),
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
            (VT100KeyCodeIR::Up, SpecialKey::Up),
            (VT100KeyCodeIR::Down, SpecialKey::Down),
            (VT100KeyCodeIR::Left, SpecialKey::Left),
            (VT100KeyCodeIR::Right, SpecialKey::Right),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiersIR::default());

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
            (VT100KeyCodeIR::Home, SpecialKey::Home),
            (VT100KeyCodeIR::End, SpecialKey::End),
            (VT100KeyCodeIR::PageUp, SpecialKey::PageUp),
            (VT100KeyCodeIR::PageDown, SpecialKey::PageDown),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiersIR::default());

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
            (VT100KeyCodeIR::Insert, SpecialKey::Insert),
            (VT100KeyCodeIR::Delete, SpecialKey::Delete),
            (VT100KeyCodeIR::Backspace, SpecialKey::Backspace),
            (VT100KeyCodeIR::Enter, SpecialKey::Enter),
            (VT100KeyCodeIR::Escape, SpecialKey::Esc),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiersIR::default());

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
            (VT100KeyCodeIR::Tab, SpecialKey::Tab),
            (VT100KeyCodeIR::BackTab, SpecialKey::BackTab),
        ];

        for (vt100_code, expected_special_key) in test_cases {
            let result =
                convert_key_code_to_keypress(vt100_code, VT100KeyModifiersIR::default());

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
        let modifiers = VT100KeyModifiersIR {
            shift: KeyState::Pressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCodeIR::Char('a'), modifiers);

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
        let modifiers = VT100KeyModifiersIR {
            shift: KeyState::NotPressed,
            ctrl: KeyState::Pressed,
            alt: KeyState::NotPressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCodeIR::Char('c'), modifiers);

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
        let modifiers = VT100KeyModifiersIR {
            shift: KeyState::NotPressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::Pressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCodeIR::Left, modifiers);

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
        let modifiers = VT100KeyModifiersIR {
            shift: KeyState::Pressed,
            ctrl: KeyState::Pressed,
            alt: KeyState::Pressed,
        };

        let result = convert_key_code_to_keypress(VT100KeyCodeIR::Function(5), modifiers);

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
            (VT100MouseButtonIR::Left, Button::Left),
            (VT100MouseButtonIR::Right, Button::Right),
            (VT100MouseButtonIR::Middle, Button::Middle),
        ];

        for (vt100_button, expected_button) in test_cases {
            let vt100_event = VT100InputEventIR::Mouse {
                button: vt100_button,
                pos: TermPos::from_one_based(1, 1),
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::default(),
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
        let vt100_event = VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Unknown,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseActionIR::Press,
            modifiers: VT100KeyModifiersIR::default(),
        };

        let result = convert_input_event(vt100_event);
        assert!(result.is_none(), "Unknown mouse button should return None");
    }

    #[test]
    fn test_convert_mouse_actions() {
        // Test all mouse action types
        let button = VT100MouseButtonIR::Left;

        // Press
        let vt100_event = VT100InputEventIR::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseActionIR::Press,
            modifiers: VT100KeyModifiersIR::default(),
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
        let vt100_event = VT100InputEventIR::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseActionIR::Release,
            modifiers: VT100KeyModifiersIR::default(),
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
        let vt100_event = VT100InputEventIR::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseActionIR::Drag,
            modifiers: VT100KeyModifiersIR::default(),
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
        let vt100_event = VT100InputEventIR::Mouse {
            button,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseActionIR::Motion,
            modifiers: VT100KeyModifiersIR::default(),
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
            (VT100ScrollDirectionIR::Up, MouseInputKind::ScrollUp),
            (VT100ScrollDirectionIR::Down, MouseInputKind::ScrollDown),
            (VT100ScrollDirectionIR::Left, MouseInputKind::ScrollLeft),
            (VT100ScrollDirectionIR::Right, MouseInputKind::ScrollRight),
        ];

        for (vt100_dir, expected_kind) in test_cases {
            let vt100_event = VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: TermPos::from_one_based(1, 1),
                action: VT100MouseActionIR::Scroll(vt100_dir),
                modifiers: VT100KeyModifiersIR::default(),
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
            let vt100_event = VT100InputEventIR::Mouse {
                button: VT100MouseButtonIR::Left,
                pos: term_pos,
                action: VT100MouseActionIR::Press,
                modifiers: VT100KeyModifiersIR::default(),
            };

            match convert_input_event(vt100_event) {
                Some(InputEvent::Mouse(mouse_input)) => {
                    assert_eq!(
                        mouse_input.pos, expected_pos,
                        "Position conversion failed for {term_pos:?}"
                    );
                }
                _ => panic!("Expected Mouse event"),
            }
        }
    }

    #[test]
    fn test_convert_mouse_with_modifiers() {
        // Test mouse event with Shift modifier
        let modifiers = VT100KeyModifiersIR {
            shift: KeyState::Pressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        };

        let vt100_event = VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Left,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseActionIR::Press,
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
        let vt100_event = VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Left,
            pos: TermPos::from_one_based(1, 1),
            action: VT100MouseActionIR::Press,
            modifiers: VT100KeyModifiersIR::default(),
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
            let vt100_event = VT100InputEventIR::Resize {
                col_width: ColWidth::from(cols),
                row_height: RowHeight::from(rows),
            };

            match convert_input_event(vt100_event) {
                Some(InputEvent::Resize(size)) => {
                    assert_eq!(
                        size.col_width,
                        ColWidth::from(cols),
                        "Column width mismatch for cols={cols}"
                    );
                    assert_eq!(
                        size.row_height,
                        RowHeight::from(rows),
                        "Row height mismatch for rows={rows}"
                    );
                }
                _ => panic!("Expected Resize event for {cols}x{rows}"),
            }
        }
    }

    #[test]
    fn test_convert_focus_gained() {
        // Test focus gained event
        let vt100_event = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);

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
        let vt100_event = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);

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
        let vt100_event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('x'),
            modifiers: VT100KeyModifiersIR {
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
                KeyPress::Plain { .. } => panic!("Expected WithModifiers keypress"),
            },
            _ => panic!("Expected Keyboard event"),
        }
    }
}

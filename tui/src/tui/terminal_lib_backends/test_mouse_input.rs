// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    use crate::{Button, ModifierKeysMask, MouseInput, MouseInputKind, assert_eq2, col,
                row};

    #[test]
    fn test_convert_mouse_event_mouse_moved() {
        // Mouse moved w/ modifier.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::Moved,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::SHIFT,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(converted_mouse_input.kind, MouseInputKind::MouseMove);
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
            assert_eq2!(
                converted_mouse_input.maybe_modifier_keys,
                Some(ModifierKeysMask::new().with_shift())
            );
        }
        // Mouse moved.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::Moved,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(converted_mouse_input.kind, MouseInputKind::MouseMove);
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
        }
    }
    #[test]

    fn test_convert_mouse_event_mouse_scroll() {
        // Mouse scroll down.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(converted_mouse_input.kind, MouseInputKind::ScrollDown);
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
        }
        // Mouse scroll up.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(converted_mouse_input.kind, MouseInputKind::ScrollUp);
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
        }
    }

    #[test]
    fn test_convert_mouse_event_mouse_click() {
        // Mouse down.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(
                converted_mouse_input.kind,
                MouseInputKind::MouseDown(Button::Left)
            );
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
        }
        // Mouse down w/ modifier.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::SHIFT,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(
                converted_mouse_input.kind,
                MouseInputKind::MouseDown(Button::Left)
            );
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
            assert_eq2!(
                converted_mouse_input.maybe_modifier_keys,
                Some(ModifierKeysMask::new().with_shift())
            );
        }
        // Mouse up.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(
                converted_mouse_input.kind,
                MouseInputKind::MouseUp(Button::Left)
            );
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
        }
    }

    #[test]
    fn test_convert_mouse_event_mouse_drag() {
        // Mouse drag.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(
                converted_mouse_input.kind,
                MouseInputKind::MouseDrag(Button::Left)
            );
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
        }
        // Mouse drag w/ modifiers.
        {
            let mouse_event: MouseEvent = MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::SHIFT | KeyModifiers::ALT,
            };
            let converted_mouse_input: MouseInput = mouse_event.into();
            assert_eq2!(
                converted_mouse_input.kind,
                MouseInputKind::MouseDrag(Button::Left)
            );
            assert_eq2!(converted_mouse_input.pos, col(0) + row(0));
            assert_eq2!(
                converted_mouse_input.maybe_modifier_keys,
                Some(ModifierKeysMask::new().with_alt().with_shift())
            );
        }
    }
}

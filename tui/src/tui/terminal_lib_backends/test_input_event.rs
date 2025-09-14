// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use crate::{InputEvent, Key, ModifierKeysMask, assert_eq2, convert_key_event,
                crossterm_keyevent, key_press, throws, try_convert_key_modifiers};

    #[test]
    #[allow(clippy::missing_errors_doc)]
    fn test_convert_key_event_into_input_event() -> Result<(), ()> {
        throws!({
            // Crossterm KeyEvents.
            let x = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::NONE
            };
            let caps_x = crossterm_keyevent! {
              code: KeyCode::Char('X'),
              modifiers: KeyModifiers::SHIFT
            };
            let ctrl_x = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::CONTROL
            };

            // InputEvents.
            let x_tw = InputEvent::try_from(x)?;
            let caps_x_tw = InputEvent::try_from(caps_x)?;
            let ctrl_x_tw = InputEvent::try_from(ctrl_x);

            // Check that the conversion is correct.
            assert_eq2!(x_tw, InputEvent::Keyboard(key_press! {@char 'x'}));
            assert_eq2!(caps_x_tw, InputEvent::Keyboard(key_press! {@char 'X'}));
            assert_eq2!(ctrl_x_tw, Ok(InputEvent::Keyboard(ctrl_x.try_into()?)));
        });
    }

    #[test]
    #[allow(clippy::missing_errors_doc)]
    fn test_input_event_matches_correctly() -> Result<(), ()> {
        throws!({
            let x = InputEvent::Keyboard(key_press! { @char 'x' });
            let caps_x = InputEvent::Keyboard(key_press! {@char 'X'});
            let ctrl_x = InputEvent::Keyboard(
                crossterm_keyevent! {
                  code: KeyCode::Char('x'),
                  modifiers: KeyModifiers::CONTROL
                }
                .try_into()?,
            );
            let events_to_match_against = [x, caps_x, ctrl_x];

            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::SHIFT
            }; // "Shift + x"
            let converted_event: InputEvent = key_event.try_into()?; // "X"

            let result = converted_event.matches(&events_to_match_against);

            assert!(result);
        });
    }

    #[test]
    fn test_copy_modifiers_from_key_event() {
        // "x"
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::NONE
            };
            let maybe_modifier_keys = try_convert_key_modifiers(&key_event.modifiers);
            assert!(maybe_modifier_keys.is_none());
        }
        // "Ctrl + x"
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::CONTROL
            };
            let maybe_modifier_keys = try_convert_key_modifiers(&key_event.modifiers);
            assert!(maybe_modifier_keys.is_some());
            assert!(maybe_modifier_keys.unwrap() == ModifierKeysMask::new().with_ctrl());
        }
        // "Ctrl + Shift + X".
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('X'),
              modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT
            };
            let maybe_modifier_keys = try_convert_key_modifiers(&key_event.modifiers);
            assert!(maybe_modifier_keys.is_some());
            assert!(
                maybe_modifier_keys.unwrap()
                    == (ModifierKeysMask::new().with_ctrl().with_shift())
            );
        }
        // "Ctrl + Shift + Alt + X".
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('X'),
              modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT
            };
            let maybe_modifier_keys = try_convert_key_modifiers(&key_event.modifiers);
            assert!(maybe_modifier_keys.is_some());
            assert!(
                maybe_modifier_keys.unwrap()
                    == ModifierKeysMask::new().with_alt().with_ctrl().with_shift()
            );
        }
    }

    #[test]
    fn test_copy_code_from_key_event() {
        // "x"
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::NONE
            };
            let maybe_non_modifier_keys =
                convert_key_event::copy_code_from_key_event(&key_event);
            assert_eq2!(maybe_non_modifier_keys.unwrap(), Key::Character('x'));
        }
        // "Ctrl + x"
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::CONTROL
            };
            let maybe_non_modifier_keys =
                convert_key_event::copy_code_from_key_event(&key_event);
            assert_eq2!(maybe_non_modifier_keys.unwrap(), Key::Character('x'));
        }
        // "Ctrl + Shift + x".
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT
            };
            let maybe_non_modifier_keys =
                convert_key_event::copy_code_from_key_event(&key_event);
            assert_eq2!(maybe_non_modifier_keys.unwrap(), Key::Character('x'));
        }
        // "Ctrl + Shift + Alt + x".
        {
            let key_event = crossterm_keyevent! {
              code: KeyCode::Char('x'),
              modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT
            };
            let maybe_non_modifier_keys =
                convert_key_event::copy_code_from_key_event(&key_event);
            assert_eq2!(maybe_non_modifier_keys.unwrap(), Key::Character('x'));
        }
    }
}

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{InputEvent, Key, KeyPress, SpecialKey};

/// Provide a conversion from [`crate::InputEvent`] to [`DialogEvent`].
///
/// This makes it easier to write event handlers that consume [`crate::InputEvent`] and
/// then process events in [`crate::DialogComponent`] and [`crate::DialogEngine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogEvent {
    EnterPressed,
    EscPressed,
    None,
}

/// Converts an [`InputEvent`] reference into a [`DialogEvent`].
/// - Matches Enter to [`DialogEvent::EnterPressed`].
/// - Matches Esc to [`DialogEvent::EscPressed`].
/// - Matches all other input events to [`DialogEvent::None`].
impl From<&InputEvent> for DialogEvent {
    fn from(input_event: &InputEvent) -> DialogEvent {
        if let InputEvent::Keyboard(keypress) = input_event {
            match keypress {
                // Compare to `Enter`.
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Enter),
                } => {
                    return DialogEvent::EnterPressed;
                }

                // Compare to `Esc`.
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Esc),
                } => {
                    return DialogEvent::EscPressed;
                }

                _ => {}
            }
        }

        DialogEvent::None
    }
}

#[cfg(test)]
mod test_dialog_event {
    use super::*;
    use crate::{assert_eq2, key_press};

    #[test]
    fn dialog_event_handles_enter() {
        let input_event = InputEvent::Keyboard(key_press!(@special SpecialKey::Enter));
        let dialog_event = DialogEvent::from(&input_event);
        assert_eq2!(dialog_event, DialogEvent::EnterPressed);
    }

    #[test]
    fn dialog_event_handles_esc() {
        let input_event = InputEvent::Keyboard(key_press!(@special SpecialKey::Esc));
        let dialog_event = DialogEvent::from(&input_event);
        assert_eq2!(dialog_event, DialogEvent::EscPressed);
    }

    #[test]
    fn dialog_event_handles_other_events() {
        let input_event = InputEvent::Keyboard(key_press!(@char 'a'));
        let dialog_event = DialogEvent::from(&input_event);
        assert_eq2!(dialog_event, DialogEvent::None);
    }
}

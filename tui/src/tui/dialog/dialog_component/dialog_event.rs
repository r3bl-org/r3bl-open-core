// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::terminal_lib_backends::{InputEvent, Key, KeyPress, SpecialKey};

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

mod dialog_event_impl {
    use super::{DialogEvent, InputEvent, Key, KeyPress, SpecialKey};

    impl DialogEvent {
        /// Tries to convert the given [`InputEvent`] into a
        /// [`DialogEvent`].
        /// - Enter and Esc are also matched against to return
        ///   [`DialogEvent::EnterPressed`] and [`DialogEvent::EscPressed`]
        /// - Otherwise, [Err] is returned.
        #[must_use]
        pub fn from(input_event: &InputEvent) -> Self {
            if let InputEvent::Keyboard(keypress) = input_event {
                match keypress {
                    // Compare to `Enter`.
                    KeyPress::Plain {
                        key: Key::SpecialKey(SpecialKey::Enter),
                    } => {
                        return Self::EnterPressed;
                    }

                    // Compare to `Esc`.
                    KeyPress::Plain {
                        key: Key::SpecialKey(SpecialKey::Esc),
                    } => {
                        return Self::EscPressed;
                    }

                    _ => {}
                }
            }

            Self::None
        }
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
}

/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use crate::terminal_lib_backends::{InputEvent, Key, KeyPress, SpecialKey};

/// Provide a conversion from [crate::InputEvent] to [DialogEvent].
///
/// This makes it easier to write event handlers that consume [crate::InputEvent] and then
/// process events in [crate::DialogComponent] and [crate::DialogEngine].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogEvent {
    EnterPressed,
    EscPressed,
    None,
}

mod dialog_event_impl {
    use super::*;

    impl DialogEvent {
        /// Tries to convert the given [InputEvent] into a [DialogEvent].
        /// - Enter and Esc are also matched against to return [DialogEvent::EnterPressed]
        ///   and [DialogEvent::EscPressed]
        /// - Otherwise, [Err] is returned.
        pub fn from(input_event: InputEvent) -> Self {
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
        let dialog_event = DialogEvent::from(input_event);
        assert_eq2!(dialog_event, DialogEvent::EnterPressed);
    }

    #[test]
    fn dialog_event_handles_esc() {
        let input_event = InputEvent::Keyboard(key_press!(@special SpecialKey::Esc));
        let dialog_event = DialogEvent::from(input_event);
        assert_eq2!(dialog_event, DialogEvent::EscPressed);
    }
}

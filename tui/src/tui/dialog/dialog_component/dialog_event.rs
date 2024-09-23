/*
 *   Copyright (c) 2022 R3BL LLC
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

use serde::{Deserialize, Serialize};

/// Provide a conversion from [crate::InputEvent] to [DialogEvent].
///
/// This makes it easier to write event handlers that consume [crate::InputEvent] and then
/// process events in [crate::DialogComponent] and [crate::DialogEngine].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogEvent {
    EnterPressed,
    EscPressed,
    None,
}

mod dialog_event_impl {
    use super::*;
    use crate::{InputEvent, Key, KeyPress, SpecialKey};

    impl DialogEvent {
        /// Tries to convert the given [InputEvent] into a [DialogEvent].
        /// - Enter and Esc are also matched against to return [DialogEvent::EnterPressed] and
        ///   [DialogEvent::EscPressed]
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
    use r3bl_rs_utils_core::*;

    use crate::{keypress, DialogEvent, InputEvent, SpecialKey};

    #[test]
    fn dialog_event_handles_enter() {
        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Enter));
        let dialog_event = DialogEvent::from(input_event);
        assert_eq2!(dialog_event, DialogEvent::EnterPressed);
    }

    #[test]
    fn dialog_event_handles_esc() {
        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Esc));
        let dialog_event = DialogEvent::from(input_event);
        assert_eq2!(dialog_event, DialogEvent::EscPressed);
    }
}

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

use crate::*;

// ┏━━━━━━━━━━━━━┓
// ┃ DialogEvent ┃
// ┛             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// By providing a conversion from [InputEvent] to [DialogEvent] it becomes easier to write event
/// handlers that consume [InputEvent] and then process events in [DialogComponent] and
/// [DialogEngineApi].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogEvent {
    ActivateModal,
    EnterPressed,
    EscPressed,
}

impl DialogEvent {
    /// Tries to convert the given [InputEvent] into a [DialogEvent].
    /// - The optional `modal_keypress` is used to determine whether the [InputEvent] should be
    ///   converted to [DialogEvent::ActivateModal].
    /// - Enter and Esc are also matched against to return [DialogEvent::EnterPressed] and
    ///   [DialogEvent::EscPressed]
    /// - Otherwise, [Err] is returned.
    pub fn try_from(
        input_event: &InputEvent,
        maybe_modal_keypress: Option<KeyPress>,
    ) -> Option<Self> {
        if let InputEvent::Keyboard(keypress) = input_event {
            // Compare to `modal_keypress` (if any).
            if let Some(modal_keypress) = maybe_modal_keypress {
                if keypress == &modal_keypress {
                    return Some(Self::ActivateModal);
                }
            }

            match keypress {
                // Compare to `Enter`.
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Enter),
                } => {
                    return Some(Self::EnterPressed);
                }

                // Compare to `Esc`.
                KeyPress::Plain {
                    key: Key::SpecialKey(SpecialKey::Esc),
                } => {
                    return Some(Self::EscPressed);
                }

                _ => {}
            }
        }

        None
    }
}

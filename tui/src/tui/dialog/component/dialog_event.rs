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
  /// Tries to convert the given [InputEvent] into a [DialogEvent]. The `modal_keypress` is used to
  /// determine whether the [InputEvent] should be converted to [DialogEvent::ActivateModal].
  pub fn try_from(input_event: &InputEvent, modal_keypress: Keypress) -> Result<Self, String> {
    let mut maybe_dialog_event: Option<Self> = None;

    if let InputEvent::Keyboard(keypress) = input_event {
      if *keypress == modal_keypress {
        maybe_dialog_event = Some(Self::ActivateModal);
      }

      match keypress {
        Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Enter),
        } => {
          maybe_dialog_event = Some(Self::EnterPressed);
        }

        Keypress::Plain {
          key: Key::SpecialKey(SpecialKey::Esc),
        } => {
          maybe_dialog_event = Some(Self::EscPressed);
        }

        _ => {}
      }
    }

    return if let Some(dialog_event) = maybe_dialog_event {
      Ok(dialog_event)
    } else {
      Err(format!(
        "Unable to convert this InputEvent to DialogEvent: {:?}",
        input_event
      ))
    };
  }
}

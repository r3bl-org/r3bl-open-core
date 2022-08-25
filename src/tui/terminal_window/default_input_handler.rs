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

use crate::*;

pub struct DefaultInputEventHandler;

impl DefaultInputEventHandler {
  /// This function does **not** consume the `input_event` argument. [InputEvent] implements [Copy]
  /// (no need to pass references into this function).
  pub async fn no_consume(
    input_event: InputEvent, exit_keys: &[InputEvent],
  ) -> Continuation<String> {
    // Early return if any exit key sequence is pressed.
    if input_event.matches(exit_keys) {
      return Continuation::Exit;
    }

    // Default input event handling.
    match input_event {
      InputEvent::Keyboard(keypress) => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Keypress: {:?}", keypress)
        );
      }
      InputEvent::Resize(size) => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Resize: {:?}", size)
        );
        return Continuation::ResizeAndContinue(size);
      }
      InputEvent::Mouse(mouse_event) => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Mouse: {:?}", mouse_event)
        );
      }
      InputEvent::Focus(focus_event) => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Focus: {:?}", focus_event)
        );
      }
      InputEvent::Paste(text) => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Paste: {:?}", text)
        );
      }
    }

    Continuation::Continue
  }
}

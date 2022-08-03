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

use crossterm::event::*;

use crate::*;

pub struct TWDefaultInputEventHandler;

impl TWDefaultInputEventHandler {
  /// This function does **not** consume the `input_event` argument. [TWInputEvent] implements [Copy]
  /// (no need to pass references into this function).
  pub async fn no_consume(input_event: TWInputEvent, exit_keys: &[KeyEvent]) -> Continuation {
    // Early return if any exit key sequence is pressed.
    if let Continuation::Exit = TWDefaultInputEventHandler::from(input_event, exit_keys) {
      return Continuation::Exit;
    }

    // Default input event handling.
    match input_event {
      TWInputEvent::NonDisplayableKeypress(key_event) => {
        call_if_true!(
          DEBUG,
          log_no_err!(
            INFO,
            "default_event_handler -> NonDisplayableKeypress: {:?}",
            key_event
          )
        );
      }
      TWInputEvent::DisplayableKeypress(character) => {
        call_if_true!(
          DEBUG,
          log_no_err!(
            INFO,
            "default_event_handler -> DisplayableKeypress: {:?}",
            character
          )
        );
      }
      TWInputEvent::Resize(size) => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Resize: {:?}", size)
        );
        return Continuation::ResizeAndContinue(size);
      }
      TWInputEvent::Mouse(mouse_event) => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Mouse: {:?}", mouse_event)
        );
      }
      _ => {
        call_if_true!(
          DEBUG,
          log_no_err!(INFO, "default_event_handler -> Other: {:?}", input_event)
        );
      }
    }

    Continuation::Continue
  }

  fn from(input_event: TWInputEvent, exit_keys: &[KeyEvent]) -> Continuation {
    if let TWInputEvent::NonDisplayableKeypress(key_event) = input_event {
      if exit_keys.contains(&key_event) {
        return Continuation::Exit;
      }
    }
    Continuation::Continue
  }
}

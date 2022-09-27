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

use std::fmt::{Display, Formatter};

use crossterm::event::{Event::*, *};
use serde::*;

use crate::*;

/// Please see [Keypress] for more information about handling keyboard input.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputEvent {
  Keyboard(Keypress),
  Resize(Size),
  Mouse(MouseInput),
  Focus(FocusEvent),
  /// A string that was pasted into the terminal. Only emitted if `bracketed-paste` feature has been
  /// enabled for crossterm in Cargo.toml.
  Paste(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusEvent {
  Gained,
  Lost,
}

mod helpers {
  use super::*;

  impl InputEvent {
    /// Checks to see whether the `input_event` matches any of the `exit_keys`. Returns `true` if it
    /// does and `false` otherwise.
    pub fn matches(&self, exit_keys: &[InputEvent]) -> bool {
      for exit_key in exit_keys {
        if self == exit_key {
          return true;
        }
      }
      false
    }
  }

  impl Display for InputEvent {
    /// For [ToString].
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
  }
}

pub(crate) mod converters {
  use super::*;

  impl TryFrom<Event> for InputEvent {
    type Error = ();
    /// Typecast / convert [Event] to [InputEvent].
    fn try_from(event: Event) -> Result<Self, Self::Error> {
      match event {
        Key(key_event) => Ok(key_event.try_into()?),
        Mouse(mouse_event) => Ok(mouse_event.into()),
        Resize(cols, rows) => Ok((rows, cols).into()),
        FocusGained => Ok(InputEvent::Focus(FocusEvent::Gained)),
        FocusLost => Ok(InputEvent::Focus(FocusEvent::Lost)),
        Paste(text) => Ok(InputEvent::Paste(text)),
      }
    }
  }

  impl From<(/* rows: */ u16, /* cols: */ u16)> for InputEvent {
    /// Typecast / convert [(u16, u16)] to [InputEvent::Resize].
    fn from(size: (u16, u16)) -> Self {
      let (rows, cols) = size;
      InputEvent::Resize(size! { cols: cols, rows: rows })
    }
  }

  impl From<MouseEvent> for InputEvent {
    /// Typecast / convert [MouseEvent] to [InputEvent::Mouse].
    fn from(mouse_event: MouseEvent) -> Self { InputEvent::Mouse(mouse_event.into()) }
  }

  impl TryFrom<KeyEvent> for InputEvent {
    type Error = ();
    fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
      Ok(InputEvent::Keyboard(key_event.try_into()?))
    }
  }
}

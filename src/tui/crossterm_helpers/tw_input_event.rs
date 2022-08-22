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

use crate::*;

/// Please see [Keypress] for more information about handling keyboard input.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TWInputEvent {
  Keyboard(Keypress),
  Resize(Size),
  Mouse(MouseInput),
}

mod helpers {
  use super::*;

  impl TWInputEvent {
    /// Checks to see whether the `input_event` matches any of the `exit_keys`. Returns `true` if it
    /// does and `false` otherwise.
    pub fn matches(&self, exit_keys: &[TWInputEvent]) -> bool {
      for exit_key in exit_keys {
        let lhs = *self;
        let rhs = *exit_key;
        if lhs == rhs {
          return true;
        }
      }
      false
    }
  }

  impl Display for TWInputEvent {
    /// For [ToString].
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
  }
}

pub(crate) mod converters {
  use super::*;

  impl TryFrom<Event> for TWInputEvent {
    type Error = ();
    /// Typecast / convert [Event] to [TWInputEvent].
    fn try_from(event: Event) -> Result<Self, Self::Error> {
      match event {
        Key(key_event) => Ok(key_event.try_into()?),
        Mouse(mouse_event) => Ok(mouse_event.into()),
        Resize(cols, rows) => Ok((rows, cols).into()),
      }
    }
  }

  impl From<(/* rows: */ u16, /* cols: */ u16)> for TWInputEvent {
    /// Typecast / convert [(u16, u16)] to [TWInputEvent::Resize].
    fn from(size: (u16, u16)) -> Self {
      let (rows, cols) = size;
      TWInputEvent::Resize(size! { col: cols, row: rows })
    }
  }

  impl From<MouseEvent> for TWInputEvent {
    /// Typecast / convert [MouseEvent] to [TWInputEvent::Mouse].
    fn from(mouse_event: MouseEvent) -> Self { TWInputEvent::Mouse(mouse_event.into()) }
  }

  impl TryFrom<KeyEvent> for TWInputEvent {
    type Error = ();
    fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
      Ok(TWInputEvent::Keyboard(key_event.try_into()?))
    }
  }
}

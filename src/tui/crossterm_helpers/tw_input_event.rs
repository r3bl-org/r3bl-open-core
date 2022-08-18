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

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TWInputEvent {
  /// [char] that can be printed to the console. Displayable characters are:
  /// - `a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z`
  /// - `A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z`
  /// - `1, 2, 3, 4, 5, 6, 7, 8, 9, 0`
  /// - `!, @, #, $, %, ^, &, *, (, ), _, +, -, =, [, ], {, }, |, \, ,, ., /, <, >, ?, `, ~`
  DisplayableKeypress(char),
  NonDisplayableKeypress(Keypress),
  Resize(Size),
  // FIXME: remove MouseEvent dep; replace w/ MouseInput
  Mouse(MouseEvent),
  None,
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

  impl Default for TWInputEvent {
    fn default() -> Self { TWInputEvent::None }
  }
}

mod converters {
  use super::*;

  impl From<Event> for TWInputEvent {
    /// Typecast / convert [Event] to [TWInputEvent].
    fn from(event: Event) -> Self {
      match event {
        Key(key_event) => key_event.into(),
        Mouse(mouse_event) => mouse_event.into(),
        Resize(cols, rows) => (rows, cols).into(),
      }
    }
  }

  impl From<(/* rows: */ u16, /* cols: */ u16)> for TWInputEvent {
    /// Typecast / convert [(u16, u16)] to [TWInputEvent::Resize].
    fn from(size: (u16, u16)) -> Self {
      let (rows, cols) = size;
      TWInputEvent::Resize(Size { cols, rows })
    }
  }

  impl From<MouseEvent> for TWInputEvent {
    /// Typecast / convert [MouseEvent] to [TWInputEvent::Mouse].
    fn from(mouse_event: MouseEvent) -> Self { TWInputEvent::Mouse(mouse_event) }
  }

  impl From<KeyEvent> for TWInputEvent {
    /// Typecast / convert [KeyEvent] to [TWInputEvent::DisplayableKeypress], or
    /// [TWInputEvent::NonDisplayableKeypress].
    ///
    /// Docs:
    ///  - [Crossterm
    ///    KeyCode::Char](https://docs.rs/crossterm/latest/crossterm/event/enum.KeyCode.html#variant.Char)
    fn from(key_event: KeyEvent) -> Self {
      match key_event {
        KeyEvent {
          code: KeyCode::Char(character),
          modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
        } => TWInputEvent::DisplayableKeypress(character),
        // All other key presses.
        _ => TWInputEvent::NonDisplayableKeypress(key_event.into()),
      }
    }
  }
}

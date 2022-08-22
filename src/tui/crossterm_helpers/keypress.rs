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
use serde::{Deserialize, Serialize};

use crate::*;

/// Please see [convert_key_event::special_handling_of_character_key_event] for more information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub struct Keypress {
  pub maybe_modifier_keys: Option<ModifierKeys>,
  pub non_modifier_key: NonModifierKey,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum NonModifierKey {
  /// [char] that can be printed to the console. Displayable characters are:
  /// - `a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z`
  /// - `A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z`
  /// - `1, 2, 3, 4, 5, 6, 7, 8, 9, 0`
  /// - `!, @, #, $, %, ^, &, *, (, ), _, +, -, =, [, ], {, }, |, \, ,, ., /, <, >, ?, `, ~`
  Character(char),
  Special(SpecialKey),
  Function(FunctionKey),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]

pub enum FunctionKey {
  F1,
  F2,
  F3,
  F4,
  F5,
  F6,
  F7,
  F8,
  F9,
  F10,
  F11,
  F12,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]

pub enum SpecialKey {
  Backspace,
  Enter,
  Left,
  Right,
  Up,
  Down,
  Home,
  End,
  PageUp,
  PageDown,
  Tab,
  BackTab, /* Shift + Tab */
  Delete,
  Insert,
  Esc,
}

#[macro_export]
macro_rules! keypress {
  // @char
  (@char $arg_char : expr) => {
    Keypress {
      maybe_modifier_keys: None,
      non_modifier_key: NonModifierKey::Character($arg_char),
    }
  };
  (@char $arg_modifiers : expr, $arg_char : expr) => {
    Keypress {
      maybe_modifier_keys: Some($arg_modifiers),
      non_modifier_key: NonModifierKey::Character($arg_char),
    }
  };

  // @special
  (@special $arg_special : expr) => {
    Keypress {
      maybe_modifier_keys: None,
      non_modifier_key: NonModifierKey::Special($arg_special),
    }
  };
  (@special $arg_modifiers : expr, $arg_special : expr) => {
    Keypress {
      maybe_modifier_keys: Some($arg_modifiers),
      non_modifier_key: NonModifierKey::Special($arg_special),
    }
  };

  // @fn
  (@fn $arg_function : expr) => {
    Keypress {
      maybe_modifier_keys: None,
      non_modifier_key: NonModifierKey::Function($arg_function),
    }
  };
  (@fn $arg_modifiers : expr, $arg_function : expr) => {
    Keypress {
      maybe_modifier_keys: Some($arg_modifiers),
      non_modifier_key: NonModifierKey::Function($arg_function),
    }
  };
}

pub mod convert_key_event {
  use super::*;

  /// Difference in meaning between `intersects` and `contains`:
  /// - `intersects` -> means that the given bit shows up in your variable, but it might contain other
  ///   bits.
  /// - `contains` -> means that your variable ONLY contains these bits.
  pub fn copy_modifiers_from_key_event(key_event: &KeyEvent) -> Option<ModifierKeys> {
    convert_key_modifiers(&key_event.modifiers)
  }

  pub fn copy_code_from_key_event(key_event: &KeyEvent) -> Option<NonModifierKey> {
    match key_event.code {
      KeyCode::Null => None,
      KeyCode::Backspace => NonModifierKey::Special(SpecialKey::Backspace).into(),
      KeyCode::Enter => NonModifierKey::Special(SpecialKey::Enter).into(),
      KeyCode::Left => NonModifierKey::Special(SpecialKey::Left).into(),
      KeyCode::Right => NonModifierKey::Special(SpecialKey::Right).into(),
      KeyCode::Up => NonModifierKey::Special(SpecialKey::Up).into(),
      KeyCode::Down => NonModifierKey::Special(SpecialKey::Down).into(),
      KeyCode::Home => NonModifierKey::Special(SpecialKey::Home).into(),
      KeyCode::End => NonModifierKey::Special(SpecialKey::End).into(),
      KeyCode::PageUp => NonModifierKey::Special(SpecialKey::PageUp).into(),
      KeyCode::PageDown => NonModifierKey::Special(SpecialKey::PageDown).into(),
      KeyCode::Tab => NonModifierKey::Special(SpecialKey::Tab).into(),
      KeyCode::BackTab => NonModifierKey::Special(SpecialKey::BackTab).into(),
      KeyCode::Delete => NonModifierKey::Special(SpecialKey::Delete).into(),
      KeyCode::Insert => NonModifierKey::Special(SpecialKey::Insert).into(),
      KeyCode::Esc => NonModifierKey::Special(SpecialKey::Esc).into(),
      KeyCode::F(fn_key) => match fn_key {
        1 => NonModifierKey::Function(FunctionKey::F1).into(),
        2 => NonModifierKey::Function(FunctionKey::F2).into(),
        3 => NonModifierKey::Function(FunctionKey::F3).into(),
        4 => NonModifierKey::Function(FunctionKey::F4).into(),
        5 => NonModifierKey::Function(FunctionKey::F5).into(),
        6 => NonModifierKey::Function(FunctionKey::F6).into(),
        7 => NonModifierKey::Function(FunctionKey::F7).into(),
        8 => NonModifierKey::Function(FunctionKey::F8).into(),
        9 => NonModifierKey::Function(FunctionKey::F9).into(),
        10 => NonModifierKey::Function(FunctionKey::F10).into(),
        11 => NonModifierKey::Function(FunctionKey::F11).into(),
        12 => NonModifierKey::Function(FunctionKey::F12).into(),
        _ => None,
      },
      KeyCode::Char(character) => NonModifierKey::Character(character).into(),
    }
  }

  impl TryFrom<KeyEvent> for Keypress {
    type Error = ();
    /// Convert [KeyEvent] to [Keypress].
    fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
      special_handling_of_character_key_event(key_event)
    }
  }

  /// Typecast / convert [KeyEvent] to [TWInputEvent::Key]. There is special handling of displayable
  /// characters in this conversion. This occurs if the [KeyEvent] is a [KeyCode::Char].
  ///
  /// An example is typing "X" which shows up in crossterm as "Shift + X". In this case, the
  /// [KeyModifiers] `SHIFT` and `NONE` are ignored when converted into a [TWInputEvent]! This means
  /// the following:
  ///
  /// 1. Type "x"         -> you get TWInputEVent::Key(keypress! {@char 'x'})
  /// 2. Type "X"         -> you get TWInputEVent::Key(keypress! {@char 'X'}) and not
  ///                        TWInputEVent::Key(keypress! {@char ModifierKeys::SHIFT, 'X'}) ie, the
  ///                        "SHIFT" is ignored
  /// 3. Type "Shift + x" -> same as "X"
  ///
  /// The test `test_tw_input_event_matches_correctly` in `test_tw_input_event.rs` demonstrates
  /// this.
  ///
  /// Docs:
  ///  - [Crossterm
  ///    KeyCode::Char](https://docs.rs/crossterm/latest/crossterm/event/enum.KeyCode.html#variant.Char)
  pub(crate) fn special_handling_of_character_key_event(
    key_event: KeyEvent,
  ) -> Result<Keypress, ()> {
    match key_event {
      KeyEvent {
        code: KeyCode::Char(character),
        modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
      } => {
        // Character.
        Ok(keypress! { @char character })
      }
      _ => {
        // All other key presses.
        let modifiers: Option<ModifierKeys> = copy_modifiers_from_key_event(&key_event);
        let maybe_keypress: Option<NonModifierKey> = copy_code_from_key_event(&key_event);

        if let Some(keypress) = maybe_keypress {
          Ok(Keypress {
            maybe_modifier_keys: modifiers,
            non_modifier_key: keypress,
          })
        } else {
          Err(())
        }
      }
    }
  }
}

// Re-export for testing.
#[allow(unused_imports)]
pub(crate) use convert_key_event::*;

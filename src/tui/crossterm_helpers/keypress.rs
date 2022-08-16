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

use bitflags::bitflags;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct Keypress {
  pub modifier_keys: Option<ModifierKeys>,
  pub non_modifier_key: Option<NonModifierKey>,
}

bitflags! {
  #[derive(Serialize, Deserialize)]
  pub struct ModifierKeys: u8 {
    const SHIFT   = 0b0000_0001;
    const CONTROL = 0b0000_0010;
    const ALT     = 0b0000_0100;
  }
}

#[derive(Clone, Debug)]
pub enum NonModifierKey {
  Character(char),
  Special(SpecialKey),
  Function(FunctionKey),
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

// FIXME: test this!
// FIXME: replace all usages of KeyEvent w/ TWKeypressEvent

impl From<KeyEvent> for Keypress {
  /// Convert [KeyEvent] to [TWKeypressEvent].
  fn from(key_event: KeyEvent) -> Self {
    // Copy `modifiers` from `KeyEvent`.
    let modifiers: Option<ModifierKeys> = if key_event.modifiers.contains(KeyModifiers::NONE) {
      None
    } else {
      let mut my_modifiers = ModifierKeys::empty(); // 0b0000_0000
      if key_event.modifiers.contains(KeyModifiers::SHIFT) {
        my_modifiers.insert(ModifierKeys::SHIFT) // my_modifiers = 0b0000_0001;
      }
      if key_event.modifiers.contains(KeyModifiers::CONTROL) {
        my_modifiers.insert(ModifierKeys::CONTROL) // my_modifiers = 0b0000_0010;
      }
      if key_event.modifiers.contains(KeyModifiers::ALT) {
        my_modifiers.insert(ModifierKeys::ALT) // my_modifiers = 0b0000_0100;
      }
      my_modifiers.into()
    };

    // Copy `code` from `KeyEvent`.
    let keypress: Option<NonModifierKey> = match key_event.code {
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
    };

    Keypress {
      modifier_keys: modifiers,
      non_modifier_key: keypress,
    }
  }
}

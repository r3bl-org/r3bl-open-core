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
use r3bl_rs_utils::*;

#[test]
fn test_keypress_character_key() {
  // No modifier.
  {
    let macro_syntax = keypress! { @char 'a' };
    let struct_syntax = Keypress {
      non_modifier_key: NonModifierKey::Character('a'),
      maybe_modifier_keys: None,
    };
    assert_eq2!(macro_syntax, struct_syntax);
  }

  // With modifier.
  {
    let macro_syntax = keypress! { @char ModifierKeys::SHIFT | ModifierKeys::CTRL, 'a' };
    let struct_syntax = Keypress {
      non_modifier_key: NonModifierKey::Character('a'),
      maybe_modifier_keys: Some(ModifierKeys::SHIFT | ModifierKeys::CTRL),
    };
    assert_eq2!(macro_syntax, struct_syntax);
  }
}

#[test]
fn test_keypress_special_key() {
  // No modifier.
  {
    let macro_syntax = keypress! { @special SpecialKey::Left };
    let struct_syntax = Keypress {
      non_modifier_key: NonModifierKey::Special(SpecialKey::Left),
      maybe_modifier_keys: None,
    };
    assert_eq2!(macro_syntax, struct_syntax);
  }
  // With modifier.
  {
    let macro_syntax =
      keypress! { @special ModifierKeys::CTRL | ModifierKeys::ALT, SpecialKey::Left };
    let struct_syntax = Keypress {
      non_modifier_key: NonModifierKey::Special(SpecialKey::Left),
      maybe_modifier_keys: Some(ModifierKeys::CTRL | ModifierKeys::ALT),
    };
    assert_eq2!(macro_syntax, struct_syntax);
  }
}

#[test]
fn test_keypress_function_key() {
  // No modifier.
  {
    let macro_syntax = keypress! { @fn FunctionKey::F1 };
    let struct_syntax = Keypress {
      non_modifier_key: NonModifierKey::Function(FunctionKey::F1),
      maybe_modifier_keys: None,
    };
    assert_eq2!(macro_syntax, struct_syntax);
  }
  // With modifier.
  {
    let macro_syntax = keypress! { @fn ModifierKeys::SHIFT, FunctionKey::F1 };
    let struct_syntax = Keypress {
      non_modifier_key: NonModifierKey::Function(FunctionKey::F1),
      maybe_modifier_keys: Some(ModifierKeys::SHIFT),
    };
    assert_eq2!(macro_syntax, struct_syntax);
  }
}

#[test]
fn test_keypress() -> Result<(), ()> {
  // "x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::NONE,
    };
    let keypress: Keypress = key_event.try_into()?;
    assert_eq2!(
      keypress,
      Keypress {
        non_modifier_key: NonModifierKey::Character('x'),
        maybe_modifier_keys: None,
      }
    );
  }

  // "Ctrl + x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::CONTROL,
    };
    let converted_keypress: Keypress = key_event.try_into()?;
    assert_eq2!(
      converted_keypress,
      Keypress {
        maybe_modifier_keys: ModifierKeys::CTRL.into(),
        non_modifier_key: NonModifierKey::Character('x'),
      }
    );
  }

  // "Ctrl + Alt + x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT,
    };
    let converted_keypress: Keypress = key_event.try_into()?;
    assert_eq2!(
      converted_keypress,
      Keypress {
        maybe_modifier_keys: Some(ModifierKeys::CTRL | ModifierKeys::ALT),
        non_modifier_key: NonModifierKey::Character('x'),
      }
    );
  }

  Ok(())
}

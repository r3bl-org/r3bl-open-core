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

#[cfg(test)]
mod tests {
  use crossterm::event::*;
  use r3bl_rs_utils_core::*;

  use crate::*;

  #[test]
  fn test_keypress_character_key() {
    // No modifier.
    {
      let macro_syntax = keypress! { @char 'a' };
      let struct_syntax = KeyPress::Plain {
        key: Key::Character('a'),
      };
      assert_eq2!(macro_syntax, struct_syntax);
    }

    // With modifier.
    {
      let macro_syntax = keypress! { @char ModifierKeysMask::SHIFT | ModifierKeysMask::CTRL, 'a' };
      let struct_syntax = KeyPress::WithModifiers {
        key: Key::Character('a'),
        mask: ModifierKeysMask::SHIFT | ModifierKeysMask::CTRL,
      };
      assert_eq2!(macro_syntax, struct_syntax);
    }
  }

  #[test]
  fn test_keypress_special_key() {
    // No modifier.
    {
      let macro_syntax = keypress! { @special SpecialKey::Left };
      let struct_syntax = KeyPress::Plain {
        key: Key::SpecialKey(SpecialKey::Left),
      };
      assert_eq2!(macro_syntax, struct_syntax);
    }
    // With modifier.
    {
      let macro_syntax = keypress! { @special ModifierKeysMask::CTRL | ModifierKeysMask::ALT, SpecialKey::Left };
      let struct_syntax = KeyPress::WithModifiers {
        key: Key::SpecialKey(SpecialKey::Left),
        mask: ModifierKeysMask::CTRL | ModifierKeysMask::ALT,
      };
      assert_eq2!(macro_syntax, struct_syntax);
    }
  }

  #[test]
  fn test_keypress_function_key() {
    // No modifier.
    {
      let macro_syntax = keypress! { @fn FunctionKey::F1 };
      let struct_syntax = KeyPress::Plain {
        key: Key::FunctionKey(FunctionKey::F1),
      };
      assert_eq2!(macro_syntax, struct_syntax);
    }
    // With modifier.
    {
      let macro_syntax = keypress! { @fn ModifierKeysMask::SHIFT, FunctionKey::F1 };
      let struct_syntax = KeyPress::WithModifiers {
        key: Key::FunctionKey(FunctionKey::F1),
        mask: ModifierKeysMask::SHIFT,
      };
      assert_eq2!(macro_syntax, struct_syntax);
    }
  }

  #[test]
  fn test_keypress() -> Result<(), ()> {
    // "x"
    {
      let key_event = keyevent!(code: KeyCode::Char('x'), modifiers: KeyModifiers::NONE);
      let keypress: KeyPress = key_event.try_into()?;
      assert_eq2!(
        keypress,
        KeyPress::Plain {
          key: Key::Character('x'),
        }
      );
    }

    // "Ctrl + x"
    {
      let key_event = keyevent!(code: KeyCode::Char('x'), modifiers: KeyModifiers::CONTROL);
      let converted_keypress: KeyPress = key_event.try_into()?;
      assert_eq2!(
        converted_keypress,
        KeyPress::WithModifiers {
          mask: ModifierKeysMask::CTRL,
          key: Key::Character('x'),
        }
      );
    }

    // "Ctrl + Alt + x"
    {
      let key_event = keyevent!(
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT
      );
      let converted_keypress: KeyPress = key_event.try_into()?;
      assert_eq2!(
        converted_keypress,
        KeyPress::WithModifiers {
          mask: ModifierKeysMask::CTRL | ModifierKeysMask::ALT,
          key: Key::Character('x'),
        }
      );
    }

    Ok(())
  }
}

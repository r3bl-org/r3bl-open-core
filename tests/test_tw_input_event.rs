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
fn test_convert_keyevent_into_twinputevent() -> Result<(), ()> {
  // Crossterm KeyEvents.
  let x = KeyEvent {
    code: KeyCode::Char('x'),
    modifiers: KeyModifiers::NONE,
  };
  let caps_x = KeyEvent {
    code: KeyCode::Char('X'),
    modifiers: KeyModifiers::SHIFT,
  };
  let ctrl_x = KeyEvent {
    code: KeyCode::Char('x'),
    modifiers: KeyModifiers::CONTROL,
  };

  // TWInputEvents.
  let x_tw = TWInputEvent::try_from(x)?;
  let caps_x_tw = TWInputEvent::try_from(caps_x)?;
  let ctrl_x_tw = TWInputEvent::try_from(ctrl_x);

  // Check that the conversion is correct.
  assert_eq2!(x_tw, TWInputEvent::Keyboard(keypress! {@char 'x'}));
  assert_eq2!(caps_x_tw, TWInputEvent::Keyboard(keypress! {@char 'X'}));
  assert_eq2!(ctrl_x_tw, Ok(TWInputEvent::Keyboard(ctrl_x.try_into()?)));

  Ok(())
}

#[test]
fn test_tw_input_event_matches_correctly() -> Result<(), ()> {
  let x = TWInputEvent::Keyboard(keypress! { @char 'x' });
  let caps_x = TWInputEvent::Keyboard(keypress! {@char 'X'});
  let ctrl_x = TWInputEvent::Keyboard(
    KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::CONTROL,
    }
    .try_into()?,
  );
  let events_to_match_against = [x, caps_x, ctrl_x];

  let key_event = KeyEvent {
    code: KeyCode::Char('x'),
    modifiers: KeyModifiers::SHIFT,
  }; // "Shift + x"
  let converted_event: TWInputEvent = key_event.try_into()?; // "X"

  let result = converted_event.matches(&events_to_match_against);

  assert!(result);

  Ok(())
}

#[test]
fn test_copy_modifiers_from_key_event() {
  // "x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::NONE,
    };
    let maybe_modifier_keys = convert_key_event::copy_modifiers_from_key_event(&key_event);
    assert!(maybe_modifier_keys.is_none());
  }
  // "Ctrl + x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::CONTROL,
    };
    let maybe_modifier_keys = convert_key_event::copy_modifiers_from_key_event(&key_event);
    assert!(maybe_modifier_keys.is_some());
    assert!(maybe_modifier_keys.unwrap().contains(ModifierKeys::CTRL));
  }
  // "Ctrl + Shift + X"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('X'),
      modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    };
    let maybe_modifier_keys = convert_key_event::copy_modifiers_from_key_event(&key_event);
    assert!(maybe_modifier_keys.is_some());
    assert!(maybe_modifier_keys
      .unwrap()
      .contains(ModifierKeys::CTRL | ModifierKeys::SHIFT));
  }
  // "Ctrl + Shift + Alt + X"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('X'),
      modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT,
    };
    let maybe_modifier_keys = convert_key_event::copy_modifiers_from_key_event(&key_event);
    assert!(maybe_modifier_keys.is_some());
    assert!(maybe_modifier_keys
      .unwrap()
      .contains(ModifierKeys::CTRL | ModifierKeys::SHIFT | ModifierKeys::ALT));
  }
}

#[test]
fn test_copy_code_from_key_event() {
  // "x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::NONE,
    };
    let maybe_non_modifier_keys = convert_key_event::copy_code_from_key_event(&key_event);
    assert_eq2!(
      maybe_non_modifier_keys.unwrap(),
      NonModifierKey::Character('x')
    );
  }
  // "Ctrl + x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::CONTROL,
    };
    let maybe_non_modifier_keys = convert_key_event::copy_code_from_key_event(&key_event);
    assert_eq2!(
      maybe_non_modifier_keys.unwrap(),
      NonModifierKey::Character('x')
    );
  }
  // "Ctrl + Shift + x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    };
    let maybe_non_modifier_keys = convert_key_event::copy_code_from_key_event(&key_event);
    assert_eq2!(
      maybe_non_modifier_keys.unwrap(),
      NonModifierKey::Character('x')
    );
  }
  // "Ctrl + Shift + Alt + x"
  {
    let key_event = KeyEvent {
      code: KeyCode::Char('x'),
      modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT,
    };
    let maybe_non_modifier_keys = convert_key_event::copy_code_from_key_event(&key_event);
    assert_eq2!(
      maybe_non_modifier_keys.unwrap(),
      NonModifierKey::Character('x')
    );
  }
}

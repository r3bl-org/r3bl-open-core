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

/// See [convert_key_event::special_handling_of_character_key_event] for more information. Use the
/// macro [keypress!] instead of directly constructing this struct.`
///
/// ```ignore
/// fn make_keypress() {
///   let _ = Keypress::WithModifiers {
///     modifier_keys: ModifierKeysMask::ALT,
///     non_modifier_key: Key::Character('a'),
///   };
///   let _ = Keypress::Plain {
///     key: Key::Character('a'),
///   };
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum Keypress {
  Plain { key: Key },
  WithModifiers { mask: ModifierKeysMask, key: Key },
}

#[macro_export]
macro_rules! keypress {
  // @char
  (@char $arg_char : expr) => {
    Keypress::Plain {
      key: Key::Character($arg_char),
    }
  };
  (@char $arg_modifiers : expr, $arg_char : expr) => {
    Keypress::WithModifiers {
      mask: $arg_modifiers,
      key: Key::Character($arg_char),
    }
  };

  // @special
  (@special $arg_special : expr) => {
    Keypress::Plain {
      key: Key::SpecialKey($arg_special),
    }
  };
  (@special $arg_modifiers : expr, $arg_special : expr) => {
    Keypress::WithModifiers {
      mask: $arg_modifiers,
      key: Key::SpecialKey($arg_special),
    }
  };

  // @fn
  (@fn $arg_function : expr) => {
    Keypress::Plain {
      key: Key::FunctionKey($arg_function),
    }
  };
  (@fn $arg_modifiers : expr, $arg_function : expr) => {
    Keypress::WithModifiers {
      mask: $arg_modifiers,
      key: Key::FunctionKey($arg_function),
    }
  };
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum Key {
  /// [char] that can be printed to the console. Displayable characters are:
  /// - `a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z`
  /// - `A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z`
  /// - `1, 2, 3, 4, 5, 6, 7, 8, 9, 0`
  /// - `!, @, #, $, %, ^, &, *, (, ), _, +, -, =, [, ], {, }, |, \, ,, ., /, <, >, ?, `, ~`
  Character(char),
  SpecialKey(SpecialKey),
  FunctionKey(FunctionKey),
  Enhanced(Enhanced),
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

pub mod convert_key_event {
  use super::*;
  impl TryFrom<KeyEvent> for Keypress {
    type Error = ();
    /// Convert [KeyEvent] to [Keypress].
    fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
      special_handling_of_character_key_event(key_event)
    }
  }

  /// Typecast / convert [KeyEvent] to [Keypress].
  ///
  /// There is special handling of displayable characters in this conversion. This occurs if the
  /// [KeyEvent] is a [KeyCode::Char].
  ///
  /// An example is typing "X" by pressing "Shift + X" on the keyboard, which shows up in crossterm
  /// as "Shift + X". In this case, the [KeyModifiers] `SHIFT` and `NONE` are ignored when converted
  /// into a [Keypress]. This means the following:
  ///
  /// ```text
  /// ╔════════════════════╦═══════════════════════════════════════════════════════════════╗
  /// ║ User action        ║ Result                                                        ║
  /// ╠════════════════════╬═══════════════════════════════════════════════════════════════╣
  /// ║ Type "x"           ║ InputEvent::Key(keypress! {@char 'x'})                        ║
  /// ╠════════════════════╬═══════════════════════════════════════════════════════════════╣
  /// ║ Type "X"           ║ InputEvent::Key(keypress! {@char 'X'}) and not                ║
  /// ║ (On keyboard press ║ InputEvent::Key(keypress! {@char ModifierKeys::SHIFT, 'X'})   ║
  /// ║ Shift+X)           ║ ie, the "SHIFT" is ignored                                    ║
  /// ╠════════════════════╬═══════════════════════════════════════════════════════════════╣
  /// ║ Type "Shift + x"   ║ same as above                                                 ║
  /// ╚════════════════════╩═══════════════════════════════════════════════════════════════╝
  /// ```
  ///
  /// The test `test_input_event_matches_correctly` in `test_input_event.rs` demonstrates
  /// this.
  ///
  /// Docs:
  ///  - [Crossterm
  ///    KeyCode::Char](https://docs.rs/crossterm/latest/crossterm/event/enum.KeyCode.html#variant.Char)
  pub(crate) fn special_handling_of_character_key_event(
    key_event: KeyEvent,
  ) -> Result<Keypress, ()> {
    return match key_event {
      KeyEvent {
        kind: KeyEventKind::Press,
        .. /* ignore everything else: code, modifiers, etc */
      } => {
        match_event_kind_press(key_event)
      }
      _=> {
        Err(())
      }
    };

    fn match_event_kind_press(key_event: KeyEvent) -> Result<Keypress, ()> {
      match key_event {
        // Character keys (ignore SHIFT).
        KeyEvent {
          code: KeyCode::Char(character),
          modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT, // Ignore SHIFT.
          .. // Ignore other fields.
        } => {
          generate_character_key(character)
        },
        // Non character keys.
        _ => {
          let maybe_modifiers_keys_mask: Option<ModifierKeysMask> = copy_modifiers_from_key_event(&key_event);
          let maybe_key: Option<Key> = copy_code_from_key_event(&key_event);
          if let Some(key) = maybe_key {
            if let Some(mask) = maybe_modifiers_keys_mask {
              generate_non_character_key_with_modifiers(key, mask)
            } else {
              generate_non_character_key_without_modifiers(key)
            }
          } else {
            Err(())
          }
        }
      }
    }

    fn generate_character_key(character: char) -> Result<Keypress, ()> {
      Ok(keypress! { @char character })
    }

    fn generate_non_character_key_without_modifiers(key: Key) -> Result<Keypress, ()> {
      Ok(Keypress::Plain { key })
    }

    fn generate_non_character_key_with_modifiers(
      key: Key, mask: ModifierKeysMask,
    ) -> Result<Keypress, ()> {
      Ok(Keypress::WithModifiers { mask, key })
    }
  }

  /// Macro to insulate this library from changes in crossterm [KeyEvent] constructor & fields.
  #[macro_export]
  macro_rules! keyevent {
    (
      code: $arg_key_code: expr,
      modifiers: $arg_key_modifiers: expr
    ) => {
      KeyEvent::new($arg_key_code, $arg_key_modifiers)
    };
  }

  /// Difference in meaning between `intersects` and `contains`:
  /// - `intersects` -> means that the given bit shows up in your variable, but it might contain other
  ///   bits.
  /// - `contains` -> means that your variable ONLY contains these bits.
  pub fn copy_modifiers_from_key_event(key_event: &KeyEvent) -> Option<ModifierKeysMask> {
    convert_key_modifiers(&key_event.modifiers)
  }

  fn match_fn_key(fn_key: u8) -> Option<Key> {
    match fn_key {
      1 => Key::FunctionKey(FunctionKey::F1).into(),
      2 => Key::FunctionKey(FunctionKey::F2).into(),
      3 => Key::FunctionKey(FunctionKey::F3).into(),
      4 => Key::FunctionKey(FunctionKey::F4).into(),
      5 => Key::FunctionKey(FunctionKey::F5).into(),
      6 => Key::FunctionKey(FunctionKey::F6).into(),
      7 => Key::FunctionKey(FunctionKey::F7).into(),
      8 => Key::FunctionKey(FunctionKey::F8).into(),
      9 => Key::FunctionKey(FunctionKey::F9).into(),
      10 => Key::FunctionKey(FunctionKey::F10).into(),
      11 => Key::FunctionKey(FunctionKey::F11).into(),
      12 => Key::FunctionKey(FunctionKey::F12).into(),
      _ => None,
    }
  }

  fn match_media_key(media_key: MediaKeyCode) -> Option<Key> {
    // Make the code easier to read below using this alias.
    type KC = MediaKeyCode;
    Some(match media_key {
      KC::Play => Key::Enhanced(Enhanced::MediaKey(MediaKey::Play)),
      KC::Pause => Key::Enhanced(Enhanced::MediaKey(MediaKey::Pause)),
      KC::Stop => Key::Enhanced(Enhanced::MediaKey(MediaKey::Stop)),
      KC::PlayPause => Key::Enhanced(Enhanced::MediaKey(MediaKey::PlayPause)),
      KC::Reverse => Key::Enhanced(Enhanced::MediaKey(MediaKey::Reverse)),
      KC::FastForward => Key::Enhanced(Enhanced::MediaKey(MediaKey::FastForward)),
      KC::Rewind => Key::Enhanced(Enhanced::MediaKey(MediaKey::Rewind)),
      KC::TrackNext => Key::Enhanced(Enhanced::MediaKey(MediaKey::TrackNext)),
      KC::TrackPrevious => Key::Enhanced(Enhanced::MediaKey(MediaKey::TrackPrevious)),
      KC::Record => Key::Enhanced(Enhanced::MediaKey(MediaKey::Record)),
      KC::LowerVolume => Key::Enhanced(Enhanced::MediaKey(MediaKey::LowerVolume)),
      KC::RaiseVolume => Key::Enhanced(Enhanced::MediaKey(MediaKey::RaiseVolume)),
      KC::MuteVolume => Key::Enhanced(Enhanced::MediaKey(MediaKey::MuteVolume)),
    })
  }

  fn match_modifier_key_code(modifier_key_code: ModifierKeyCode) -> Option<Key> {
    // Make the code easier to read below using this alias.
    type KC = ModifierKeyCode;
    Some(match modifier_key_code {
      KC::LeftShift => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::LeftShift)),
      KC::LeftControl => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::LeftControl)),
      KC::LeftAlt => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::LeftAlt)),
      KC::LeftSuper => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::LeftSuper)),
      KC::LeftHyper => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::LeftHyper)),
      KC::LeftMeta => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::LeftMeta)),
      KC::RightShift => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::RightShift)),
      KC::RightControl => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::RightControl)),
      KC::RightAlt => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::RightAlt)),
      KC::RightSuper => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::RightSuper)),
      KC::RightHyper => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::RightHyper)),
      KC::RightMeta => Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::RightMeta)),
      KC::IsoLevel3Shift => {
        Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::IsoLevel3Shift))
      }
      KC::IsoLevel5Shift => {
        Key::Enhanced(Enhanced::ModifierKeyEnum(ModifierKeyEnum::IsoLevel5Shift))
      }
    })
  }

  pub fn copy_code_from_key_event(key_event: &KeyEvent) -> Option<Key> {
    // Make the code easier to read below using this alias.
    type KC = KeyCode;
    match key_event.code {
      KC::Null => None,
      KC::Backspace => Key::SpecialKey(SpecialKey::Backspace).into(),
      KC::Enter => Key::SpecialKey(SpecialKey::Enter).into(),
      KC::Left => Key::SpecialKey(SpecialKey::Left).into(),
      KC::Right => Key::SpecialKey(SpecialKey::Right).into(),
      KC::Up => Key::SpecialKey(SpecialKey::Up).into(),
      KC::Down => Key::SpecialKey(SpecialKey::Down).into(),
      KC::Home => Key::SpecialKey(SpecialKey::Home).into(),
      KC::End => Key::SpecialKey(SpecialKey::End).into(),
      KC::PageUp => Key::SpecialKey(SpecialKey::PageUp).into(),
      KC::PageDown => Key::SpecialKey(SpecialKey::PageDown).into(),
      KC::Tab => Key::SpecialKey(SpecialKey::Tab).into(),
      KC::BackTab => Key::SpecialKey(SpecialKey::BackTab).into(),
      KC::Delete => Key::SpecialKey(SpecialKey::Delete).into(),
      KC::Insert => Key::SpecialKey(SpecialKey::Insert).into(),
      KC::Esc => Key::SpecialKey(SpecialKey::Esc).into(),
      KC::F(fn_key) => match_fn_key(fn_key),
      KC::Char(character) => Key::Character(character).into(),
      // New "enhanced" keys since crossterm 0.25.0
      KC::CapsLock => Key::Enhanced(Enhanced::SpecialKeyExt(SpecialKeyExt::CapsLock)).into(),
      KC::ScrollLock => Key::Enhanced(Enhanced::SpecialKeyExt(SpecialKeyExt::ScrollLock)).into(),
      KC::NumLock => Key::Enhanced(Enhanced::SpecialKeyExt(SpecialKeyExt::NumLock)).into(),
      KC::PrintScreen => Key::Enhanced(Enhanced::SpecialKeyExt(SpecialKeyExt::PrintScreen)).into(),
      KC::Pause => Key::Enhanced(Enhanced::SpecialKeyExt(SpecialKeyExt::Pause)).into(),
      KC::Menu => Key::Enhanced(Enhanced::SpecialKeyExt(SpecialKeyExt::Menu)).into(),
      KC::KeypadBegin => Key::Enhanced(Enhanced::SpecialKeyExt(SpecialKeyExt::KeypadBegin)).into(),
      KC::Media(media_key) => match_media_key(media_key),
      KC::Modifier(modifier_key_code) => match_modifier_key_code(modifier_key_code),
    }
  }
}

// Re-export so this is visible for testing.
#[allow(unused_imports)]
pub(crate) use convert_key_event::*;

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

use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyEventKind,
                       KeyModifiers,
                       MediaKeyCode,
                       ModifierKeyCode};
use serde::{Deserialize, Serialize};

use super::{Enhanced, ModifierKeysMask};
use crate::{convert_key_modifiers, MediaKey, ModifierKeyEnum, SpecialKeyExt};

/// Examples.
///
/// ```rust
/// use r3bl_tui::*;
///
/// fn make_keypress() {
///   let a = keypress!(@char 'a');
///   let a = KeyPress::Plain {
///     key: Key::Character('a'),
///   };
///
///   let alt_a = keypress!(@char ModifierKeysMask::new().with_alt(), 'a');
///   let alt_a = KeyPress::WithModifiers {
///     key: Key::Character('a'),
///     mask: ModifierKeysMask {
///         alt_key_state: KeyState::Pressed,
///         ..Default::default()
///     },
///   };
///
///   let enter = keypress!(@special SpecialKey::Enter);
///   let enter = KeyPress::Plain {
///     key: Key::SpecialKey(SpecialKey::Enter),
///   };
///
///   let alt_enter = keypress!(@special ModifierKeysMask::new().with_alt(), SpecialKey::Enter);
///   let alt_enter = KeyPress::WithModifiers {
///     key: Key::SpecialKey(SpecialKey::Enter),
///     mask: ModifierKeysMask {
///         alt_key_state: KeyState::Pressed,
///         ..Default::default()
///     }
///   };
/// }
/// ```
#[macro_export]
macro_rules! keypress {
    // @char
    (@char $arg_char : expr) => {
        $crate::KeyPress::Plain {
            key: $crate::Key::Character($arg_char),
        }
    };

    (@char $arg_modifiers : expr, $arg_char : expr) => {
        $crate::KeyPress::WithModifiers {
            mask: $arg_modifiers,
            key: $crate::Key::Character($arg_char),
        }
    };

    // @special
    (@special $arg_special : expr) => {
        $crate::KeyPress::Plain {
            key: $crate::Key::SpecialKey($arg_special),
        }
    };

    (@special $arg_modifiers : expr, $arg_special : expr) => {
        $crate::KeyPress::WithModifiers {
            mask: $arg_modifiers,
            key: $crate::Key::SpecialKey($arg_special),
        }
    };

    // @fn
    (@fn $arg_function : expr) => {
        $crate::KeyPress::Plain {
            key: $crate::Key::FunctionKey($arg_function),
        }
    };

    (@fn $arg_modifiers : expr, $arg_function : expr) => {
        $crate::KeyPress::WithModifiers {
            mask: $arg_modifiers,
            key: $crate::Key::FunctionKey($arg_function),
        }
    };
}

/// This is equivalent to [crossterm::event::KeyEvent] except that it is cleaned up
/// semantically and impossible states are removed.
///
/// It enables the TUI framework to use a different backend other than `crossterm` in the
/// future. Apps written using this framework use [KeyPress] and not
/// [crossterm::event::KeyEvent]. See [convert_key_event] for more information on the
/// conversion.
///
/// Please use the [keypress!] macro instead of directly constructing this struct.
///
/// # Kitty keyboard protocol support limitations
///
/// 1. `Keypress` explicitly matches on `KeyEventKind::Press` as of crossterm 0.25.0. It
///    added a new field in KeyEvent, called
///    [`kind`](https://github.com/crossterm-rs/crossterm/blob/10d1dc246dcd708b4902d53a542f732cba32ce99/src/event.rs#L645).
///    Currently in terminals that do NOT support [kitty keyboard
///    protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/), in other words most
///    terminals, the `kind` is always `Press`. This is made explicit in the code.
///
/// 2. Also, the [KeyEvent]'s `state` is totally ignored in the conversion to [KeyPress].
///    The [crossterm::event::KeyEventState] isn't even considered in the conversion code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum KeyPress {
    Plain { key: Key },
    WithModifiers { key: Key, mask: ModifierKeysMask },
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
    /// See [`crossterm::event::PushKeyboardEnhancementFlags`] for more details on [kitty
    /// keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) and the
    /// terminals on which this is currently supported:
    /// * [kitty terminal](https://sw.kovidgoyal.net/kitty/)
    /// * [foot terminal](https://codeberg.org/dnkl/foot/issues/319)
    /// * [WezTerm
    ///   terminal](https://wezfurlong.org/wezterm/config/lua/config/enable_kitty_keyboard.html)
    /// * [notcurses library](https://github.com/dankamongmen/notcurses/issues/2131)
    /// * [neovim text editor](https://github.com/neovim/neovim/pull/18181)
    /// * [kakoune text editor](https://github.com/mawww/kakoune/issues/4103)
    /// * [dte text editor](https://gitlab.com/craigbarnes/dte/-/issues/138)
    ///
    /// Crossterm docs:
    /// - [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
    /// - [`PushKeyboardEnhancementFlags`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
    ///
    /// **Note:** [MediaKey] and [SpecialKey] can be read if:
    /// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` has been enabled with
    /// `PushKeyboardEnhancementFlags`.
    ///
    /// **Note:** [ModifierKeyEnum] can only be read if **both**
    /// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` and
    /// `KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES` have been enabled with
    /// `PushKeyboardEnhancementFlags`.
    ///
    /// Here's how you can enable crossterm enhanced mode.
    ///
    /// ```ignore
    /// use std::io::{Write, stdout};
    /// use crossterm::execute;
    /// use crossterm::event::{
    ///     KeyboardEnhancementFlags,
    ///     PushKeyboardEnhancementFlags,
    ///     PopKeyboardEnhancementFlags
    /// };
    ///
    /// let mut stdout = stdout();
    ///
    /// execute!(
    ///     stdout,
    ///     PushKeyboardEnhancementFlags(
    ///         KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
    ///     )
    /// );
    ///
    /// // Your code here.
    ///
    /// execute!(stdout, PopKeyboardEnhancementFlags);
    /// ```
    KittyKeyboardProtocol(Enhanced),
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

/// Typecast / convert [KeyEvent] to [KeyPress].
///
/// There is special handling of displayable characters in this conversion. This occurs if the
/// [KeyEvent] is a [KeyCode::Char].
///
/// An example is typing "X" by pressing "Shift + X" on the keyboard, which shows up in crossterm
/// as "Shift + X". In this case, the [KeyModifiers] `SHIFT` and `NONE` are ignored when converted
/// into a [KeyPress]. This means the following:
///
/// ```text
/// ╔════════════════════╦════════════════════════════════════════════════════════════════╗
/// ║ User action        ║ Result                                                         ║
/// ╠════════════════════╬════════════════════════════════════════════════════════════════╣
/// ║ Type "x"           ║ InputEvent::Key(keypress! {@char 'x'})                         ║
/// ╠════════════════════╬════════════════════════════════════════════════════════════════╣
/// ║ Type "X"           ║ InputEvent::Key(keypress! {@char 'X'}) and not                 ║
/// ║ (On keyboard press ║ InputEvent::Key(keypress! {@char ModifierKeysMask::SHIFT, 'X'})║
/// ║ Shift+X)           ║ ie, the "SHIFT" is ignored                                     ║
/// ╠════════════════════╬════════════════════════════════════════════════════════════════╣
/// ║ Type "Shift + x"   ║ same as above                                                  ║
/// ╚════════════════════╩════════════════════════════════════════════════════════════════╝
/// ```
///
/// The test `test_input_event_matches_correctly` in `test_input_event.rs` demonstrates
/// this.
///
/// Docs:
///  - [Crossterm
///    KeyCode::Char](https://docs.rs/crossterm/latest/crossterm/event/enum.KeyCode.html#variant.Char)
pub mod convert_key_event {
    use super::*;

    impl TryFrom<KeyEvent> for KeyPress {
        type Error = ();
        /// Convert [KeyEvent] to [KeyPress].
        fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
            special_handling_of_character_key_event(key_event)
        }
    }

    pub(crate) fn special_handling_of_character_key_event(
        key_event: KeyEvent,
    ) -> Result<KeyPress, ()> {
        return match key_event {
      KeyEvent {
        kind: KeyEventKind::Press,
        .. /* ignore everything else: code, modifiers, etc */
      } => {
        process_only_key_event_kind_press(key_event)
      }
      _=> {
        Err(())
      }
    };

        fn process_only_key_event_kind_press(
            key_event: KeyEvent,
        ) -> Result<KeyPress, ()> {
            match key_event {
        // If character keys, then ignore SHIFT or NONE modifiers.
        KeyEvent {
          code: KeyCode::Char(character),
          modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT, // Ignore SHIFT.
          .. // Ignore `state`. We know `kind`=`KeyEventKind::Press`.
        } => {
          generate_character_key(character)
        },
        // Non character keys.
        _ => {
          let maybe_modifiers_keys_mask: Option<ModifierKeysMask> = convert_key_modifiers(&key_event.modifiers);
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

        fn generate_character_key(character: char) -> Result<KeyPress, ()> {
            Ok(keypress! { @char character })
        }

        fn generate_non_character_key_without_modifiers(
            key: Key,
        ) -> Result<KeyPress, ()> {
            Ok(KeyPress::Plain { key })
        }

        fn generate_non_character_key_with_modifiers(
            key: Key,
            mask: ModifierKeysMask,
        ) -> Result<KeyPress, ()> {
            Ok(KeyPress::WithModifiers { mask, key })
        }
    }

    /// Macro to insulate this library from changes in crossterm
    /// [crossterm::event::KeyEvent] constructor & fields.
    #[macro_export]
    macro_rules! crossterm_keyevent {
        (
            code: $arg_key_code: expr,
            modifiers: $arg_key_modifiers: expr
        ) => {
            crossterm::event::KeyEvent::new($arg_key_code, $arg_key_modifiers)
        };
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
            KC::CapsLock => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::CapsLock,
            ))
            .into(),
            KC::ScrollLock => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::ScrollLock,
            ))
            .into(),
            KC::NumLock => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::NumLock,
            ))
            .into(),
            KC::PrintScreen => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::PrintScreen,
            ))
            .into(),
            KC::Pause => {
                Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(SpecialKeyExt::Pause))
                    .into()
            }
            KC::Menu => {
                Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(SpecialKeyExt::Menu))
                    .into()
            }
            KC::KeypadBegin => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::KeypadBegin,
            ))
            .into(),
            KC::Media(media_key) => match_enhanced_media_key(media_key),
            KC::Modifier(modifier_key_code) => {
                match_enhanced_modifier_key_code(modifier_key_code)
            }
        }
    }

    fn match_enhanced_media_key(media_key: MediaKeyCode) -> Option<Key> {
        // Make the code easier to read below using this alias.
        type KC = MediaKeyCode;
        Some(match media_key {
            KC::Play => Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Play)),
            KC::Pause => Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Pause)),
            KC::Stop => Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Stop)),
            KC::PlayPause => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::PlayPause))
            }
            KC::Reverse => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Reverse))
            }
            KC::FastForward => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::FastForward))
            }
            KC::Rewind => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Rewind))
            }
            KC::TrackNext => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::TrackNext))
            }
            KC::TrackPrevious => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::TrackPrevious))
            }
            KC::Record => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Record))
            }
            KC::LowerVolume => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::LowerVolume))
            }
            KC::RaiseVolume => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::RaiseVolume))
            }
            KC::MuteVolume => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::MuteVolume))
            }
        })
    }

    fn match_enhanced_modifier_key_code(
        modifier_key_code: ModifierKeyCode,
    ) -> Option<Key> {
        // Make the code easier to read below using this alias.
        type KC = ModifierKeyCode;
        Some(match modifier_key_code {
            KC::LeftShift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftShift,
            )),
            KC::LeftControl => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftControl,
            )),
            KC::LeftAlt => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftAlt,
            )),
            KC::LeftSuper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftSuper,
            )),
            KC::LeftHyper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftHyper,
            )),
            KC::LeftMeta => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftMeta,
            )),
            KC::RightShift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightShift,
            )),
            KC::RightControl => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightControl,
            )),
            KC::RightAlt => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightAlt,
            )),
            KC::RightSuper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightSuper,
            )),
            KC::RightHyper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightHyper,
            )),
            KC::RightMeta => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightMeta,
            )),
            KC::IsoLevel3Shift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::IsoLevel3Shift,
            )),
            KC::IsoLevel5Shift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::IsoLevel5Shift,
            )),
        })
    }
}

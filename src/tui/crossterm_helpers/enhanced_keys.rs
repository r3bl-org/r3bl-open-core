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

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum Enhanced {
  /// **Note:** this key can only be read if
  /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
  /// [`PushKeyboardEnhancementFlags`].
  MediaKey(MediaKey),
  /// **Note:** this key can only be read if
  /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
  /// [`PushKeyboardEnhancementFlags`].
  SpecialKeyExt(SpecialKeyExt),
  /// **Note:** these keys can only be read if **both**
  /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] and
  /// [`KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES`] have been enabled with
  /// [`PushKeyboardEnhancementFlags`].
  ModifierKeyEnum(ModifierKeyEnum),
}

/// **Note:** these keys can only be read if **both**
/// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] and
/// [`KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES`] have been enabled with
/// [`PushKeyboardEnhancementFlags`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum ModifierKeyEnum {
  /// Left Shift key.
  LeftShift,
  /// Left Control key.
  LeftControl,
  /// Left Alt key.
  LeftAlt,
  /// Left Super key.
  LeftSuper,
  /// Left Hyper key.
  LeftHyper,
  /// Left Meta key.
  LeftMeta,
  /// Right Shift key.
  RightShift,
  /// Right Control key.
  RightControl,
  /// Right Alt key.
  RightAlt,
  /// Right Super key.
  RightSuper,
  /// Right Hyper key.
  RightHyper,
  /// Right Meta key.
  RightMeta,
  /// Iso Level3 Shift key.
  IsoLevel3Shift,
  /// Iso Level5 Shift key.
  IsoLevel5Shift,
}

/// **Note:** this key can only be read if
/// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
/// [`PushKeyboardEnhancementFlags`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum SpecialKeyExt {
  CapsLock,
  ScrollLock,
  NumLock,
  PrintScreen,
  Pause,
  Menu,
  KeypadBegin,
}

/// **Note:** this key can only be read if
/// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
/// [`PushKeyboardEnhancementFlags`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum MediaKey {
  Play,
  Pause,
  PlayPause,
  Reverse,
  Stop,
  FastForward,
  Rewind,
  TrackNext,
  TrackPrevious,
  Record,
  LowerVolume,
  RaiseVolume,
  MuteVolume,
}

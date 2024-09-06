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

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, size_of::SizeOf,
)]
pub struct ModifierKeysMask {
    pub shift_key_state: KeyState,
    pub ctrl_key_state: KeyState,
    pub alt_key_state: KeyState,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, size_of::SizeOf,
)]
pub enum KeyState {
    Pressed,
    #[default]
    NotPressed,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, size_of::SizeOf)]
pub enum MatchResult {
    Matches,
    DoesNotMatch,
}

impl From<bool> for MatchResult {
    fn from(other: bool) -> Self {
        if other {
            MatchResult::Matches
        } else {
            MatchResult::DoesNotMatch
        }
    }
}

impl ModifierKeysMask {
    pub fn with_shift(mut self) -> Self {
        self.shift_key_state = KeyState::Pressed;
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl_key_state = KeyState::Pressed;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt_key_state = KeyState::Pressed;
        self
    }

    pub fn new() -> Self {
        ModifierKeysMask {
            shift_key_state: KeyState::NotPressed,
            ctrl_key_state: KeyState::NotPressed,
            alt_key_state: KeyState::NotPressed,
        }
    }

    /// Check `other` for
    /// [crossterm::event::KeyModifiers](crossterm::event::KeyModifiers::SHIFT) bit.
    /// Check `other` for `CONTROL` bit. Check `other` for `ALT` bit. If all bits
    /// match `self` then return `true`, otherwise return `false`.
    ///
    /// Difference in meaning between `intersects` and `contains`:
    /// - `intersects` -> means that the given bit shows up in your variable, but it
    ///   might contain other bits.
    /// - `contains` -> means that your variable ONLY contains these bits.
    /// - Docs: <https://docs.rs/bitflags/latest/bitflags/index.html>
    pub fn matches(&self, other: KeyModifiers) -> MatchResult {
        match (
            other.intersects(KeyModifiers::SHIFT),
            other.intersects(KeyModifiers::CONTROL),
            other.intersects(KeyModifiers::ALT),
        ) {
            (true, true, true) => (self.shift_key_state == KeyState::Pressed
                && self.ctrl_key_state == KeyState::Pressed
                && self.alt_key_state == KeyState::Pressed)
                .into(),
            (true, true, false) => (self.shift_key_state == KeyState::Pressed
                && self.ctrl_key_state == KeyState::Pressed
                && self.alt_key_state == KeyState::NotPressed)
                .into(),
            (true, false, true) => (self.shift_key_state == KeyState::Pressed
                && self.ctrl_key_state == KeyState::NotPressed
                && self.alt_key_state == KeyState::Pressed)
                .into(),
            (true, false, false) => (self.shift_key_state == KeyState::Pressed
                && self.ctrl_key_state == KeyState::NotPressed
                && self.alt_key_state == KeyState::NotPressed)
                .into(),
            (false, true, true) => (self.shift_key_state == KeyState::NotPressed
                && self.ctrl_key_state == KeyState::Pressed
                && self.alt_key_state == KeyState::Pressed)
                .into(),
            (false, true, false) => (self.shift_key_state == KeyState::NotPressed
                && self.ctrl_key_state == KeyState::Pressed
                && self.alt_key_state == KeyState::NotPressed)
                .into(),
            (false, false, true) => (self.shift_key_state == KeyState::NotPressed
                && self.ctrl_key_state == KeyState::NotPressed
                && self.alt_key_state == KeyState::Pressed)
                .into(),
            (false, false, false) => (self.shift_key_state == KeyState::NotPressed
                && self.ctrl_key_state == KeyState::NotPressed
                && self.alt_key_state == KeyState::NotPressed)
                .into(),
        }
    }
}

pub fn convert_key_modifiers(modifiers: &KeyModifiers) -> Option<ModifierKeysMask> {
    // Start w/ empty my_modifiers.
    let my_modifiers = ModifierKeysMask::from(*modifiers);
    if modifiers.is_empty() {
        None
    } else {
        Some(my_modifiers)
    }
}

impl From<KeyModifiers> for ModifierKeysMask {
    /// Difference in meaning between `intersects` and `contains`:
    /// - `intersects` -> means that the given bit shows up in your variable, but it might contain
    ///   other bits.
    /// - `contains` -> means that your variable ONLY contains these bits.
    /// - Docs: <https://docs.rs/bitflags/latest/bitflags/index.html>
    fn from(other: KeyModifiers) -> ModifierKeysMask {
        // Start w/ empty my_modifiers.
        let mut it: ModifierKeysMask = ModifierKeysMask {
            shift_key_state: KeyState::NotPressed,
            ctrl_key_state: KeyState::NotPressed,
            alt_key_state: KeyState::NotPressed,
        };

        // Try and set any bitflags from key_event.
        if other.intersects(KeyModifiers::SHIFT) {
            it.shift_key_state = KeyState::Pressed;
        }
        if other.intersects(KeyModifiers::CONTROL) {
            it.ctrl_key_state = KeyState::Pressed;
        }
        if other.intersects(KeyModifiers::ALT) {
            it.alt_key_state = KeyState::Pressed;
        }

        it
    }
}

#[cfg(test)]
mod rs_modifier_keys_mask_tests {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_empty_mask() {
        let mask = ModifierKeysMask::new();

        assert_eq2!(mask.shift_key_state, KeyState::NotPressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::NotPressed);
        assert_eq2!(mask.alt_key_state, KeyState::NotPressed);

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::DoesNotMatch);
        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL),
            MatchResult::DoesNotMatch
        );
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::DoesNotMatch);
    }

    #[test]
    fn test_shift_mask() {
        let mask = ModifierKeysMask::new().with_shift();

        assert_eq2!(mask.shift_key_state, KeyState::Pressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::NotPressed);
        assert_eq2!(mask.alt_key_state, KeyState::NotPressed);

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::Matches);
        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL),
            MatchResult::DoesNotMatch
        );
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::DoesNotMatch);
    }

    #[test]
    fn test_ctrl_mask() {
        let mask = ModifierKeysMask::new().with_ctrl();

        assert_eq2!(mask.shift_key_state, KeyState::NotPressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::Pressed);
        assert_eq2!(mask.alt_key_state, KeyState::NotPressed);

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::DoesNotMatch);
        assert_eq2!(mask.matches(KeyModifiers::CONTROL), MatchResult::Matches);
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::DoesNotMatch);
    }

    #[test]
    fn test_alt_mask() {
        let mask = ModifierKeysMask::new().with_alt();

        assert_eq2!(mask.shift_key_state, KeyState::NotPressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::NotPressed);
        assert_eq2!(mask.alt_key_state, KeyState::Pressed);

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::DoesNotMatch);
        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL),
            MatchResult::DoesNotMatch
        );
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::Matches);
    }

    #[test]
    fn test_shift_ctrl_mask() {
        let mask = ModifierKeysMask::new().with_shift().with_ctrl();

        assert_eq2!(mask.shift_key_state, KeyState::Pressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::Pressed);
        assert_eq2!(mask.alt_key_state, KeyState::NotPressed);

        assert_eq2!(
            mask.matches(KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            MatchResult::Matches
        );

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::DoesNotMatch);
        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL),
            MatchResult::DoesNotMatch
        );
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::DoesNotMatch);
    }

    #[test]
    fn test_shift_alt_mask() {
        let mask = ModifierKeysMask::new().with_shift().with_alt();

        assert_eq2!(mask.shift_key_state, KeyState::Pressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::NotPressed);
        assert_eq2!(mask.alt_key_state, KeyState::Pressed);

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::DoesNotMatch);
        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL),
            MatchResult::DoesNotMatch
        );
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::DoesNotMatch);

        assert_eq2!(
            mask.matches(KeyModifiers::SHIFT | KeyModifiers::ALT),
            MatchResult::Matches
        );
    }

    #[test]
    fn test_ctrl_alt_mask() {
        let mask = ModifierKeysMask::new().with_ctrl().with_alt();

        assert_eq2!(mask.shift_key_state, KeyState::NotPressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::Pressed);
        assert_eq2!(mask.alt_key_state, KeyState::Pressed);

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::DoesNotMatch);
        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL),
            MatchResult::DoesNotMatch
        );
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::DoesNotMatch);

        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL | KeyModifiers::ALT),
            MatchResult::Matches
        );
    }

    #[test]
    fn test_shift_ctrl_alt_mask() {
        let mask = ModifierKeysMask::new().with_shift().with_ctrl().with_alt();

        assert_eq2!(mask.shift_key_state, KeyState::Pressed);
        assert_eq2!(mask.ctrl_key_state, KeyState::Pressed);
        assert_eq2!(mask.alt_key_state, KeyState::Pressed);

        assert_eq2!(mask.matches(KeyModifiers::SHIFT), MatchResult::DoesNotMatch);
        assert_eq2!(
            mask.matches(KeyModifiers::CONTROL),
            MatchResult::DoesNotMatch
        );
        assert_eq2!(mask.matches(KeyModifiers::ALT), MatchResult::DoesNotMatch);

        assert_eq2!(
            mask.matches(KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT),
            MatchResult::Matches
        );
    }
}

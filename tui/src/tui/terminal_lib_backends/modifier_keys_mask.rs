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
use crossterm::event::*;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct ModifierKeysMask: u8 {
        const SHIFT = 0b0000_0001;
        const CTRL  = 0b0000_0010;
        const ALT   = 0b0000_0100;
    }
}

pub fn convert_key_modifiers(modifiers: &KeyModifiers) -> Option<ModifierKeysMask> {
    // Start w/ empty my_modifiers.
    let my_modifiers: ModifierKeysMask = (*modifiers).into();
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
    fn from(other: KeyModifiers) -> Self {
        // Start w/ empty my_modifiers.
        let mut my_modifiers: ModifierKeysMask = ModifierKeysMask::empty(); // 0b0000_0000

        // Try and set any bitflags from key_event.
        if other.intersects(KeyModifiers::SHIFT) {
            my_modifiers.insert(ModifierKeysMask::SHIFT) // my_modifiers = 0b0000_0001;
        }
        if other.intersects(KeyModifiers::CONTROL) {
            my_modifiers.insert(ModifierKeysMask::CTRL) // my_modifiers = 0b0000_0010;
        }
        if other.intersects(KeyModifiers::ALT) {
            my_modifiers.insert(ModifierKeysMask::ALT) // my_modifiers = 0b0000_0100;
        }

        my_modifiers
    }
}

#[cfg(test)]
mod test_bitflags {
    use bitflags::bitflags;

    #[test]
    fn bitflags_test() {
        bitflags! {
            struct TestFlags: u8 {
                const SHIFT = 0b0000_0001;
                const CTRL  = 0b0000_0010;
                const ALT   = 0b0000_0100;
            }
        }

        let mut my_flags: TestFlags = TestFlags::empty(); // 0b0000_0000
        my_flags.insert(TestFlags::SHIFT); // my_flags = 0b0000_0001;
        my_flags.insert(TestFlags::CTRL); // my_flags = 0b0000_0011;

        assert!(my_flags.contains(TestFlags::SHIFT));
        assert!(my_flags.contains(TestFlags::CTRL));
        assert!(!my_flags.contains(TestFlags::ALT));

        assert!(my_flags.intersects(TestFlags::SHIFT));
        assert!(my_flags.intersects(TestFlags::CTRL));
        assert!(!my_flags.intersects(TestFlags::ALT));
    }
}

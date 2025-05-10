/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use rand::{rngs::ThreadRng, Rng};

use crate::InlineString;

const PET_NAMES: [&str; 20] = [
    "buddy", "max", "bella", "charlie", "lucy", "daisy", "molly", "lola", "sadie",
    "maggie", "bailey", "sophie", "chloe", "duke", "lily", "rocky", "jack", "cooper",
    "riley", "zoey",
];

const FRUIT_NAMES: [&str; 20] = [
    "apple",
    "banana",
    "orange",
    "pear",
    "peach",
    "strawberry",
    "grape",
    "kiwi",
    "mango",
    "pineapple",
    "watermelon",
    "cherry",
    "blueberry",
    "raspberry",
    "lemon",
    "lime",
    "grapefruit",
    "plum",
    "apricot",
    "pomegranate",
];

pub fn generate_friendly_strongly_random_id() -> InlineString {
    let mut rng: ThreadRng = rand::rng();

    let pet = PET_NAMES[rng.random_range(0..PET_NAMES.len())];
    let fruit = FRUIT_NAMES[rng.random_range(0..FRUIT_NAMES.len())];
    let number: u16 = rng.random_range(0..1000);

    let mut acc = InlineString::with_capacity(
        pet.len() + fruit.len() + 3 + 2, // 3 for the number, 2 for the dashes
    );

    let uuid = uuid::Uuid::new_v4();

    use std::fmt::Write as _;
    _ = write!(acc, "{pet}-{fruit}-{number:03}-{uuid}");

    acc
}

pub fn generate_friendly_random_id() -> InlineString {
    let mut rng: ThreadRng = rand::rng();

    let pet = PET_NAMES[rng.random_range(0..PET_NAMES.len())];
    let fruit = FRUIT_NAMES[rng.random_range(0..FRUIT_NAMES.len())];
    let number: u16 = rng.random_range(0..1000);

    let mut acc = InlineString::with_capacity(
        pet.len() + fruit.len() + 3 + 2, // 3 for the number, 2 for the dashes
    );
    use std::fmt::Write as _;
    _ = write!(acc, "{pet}-{fruit}-{number:03}");

    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_constants_are_lowercase() {
        for name in PET_NAMES.iter() {
            assert_eq!(name, &name.to_lowercase());
        }
        for name in FRUIT_NAMES.iter() {
            assert_eq!(name, &name.to_lowercase());
        }
    }

    #[test]
    fn test_generate_friendly_random_id() {
        let id = generate_friendly_random_id();
        println!("Generated ID: {id}");
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert!(PET_NAMES.contains(&parts[0].to_lowercase().as_str()));
        assert!(FRUIT_NAMES.contains(&parts[1].to_lowercase().as_str()));
        assert!(parts[2].parse::<u16>().is_ok());
    }
}

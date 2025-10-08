// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::InlineString;
use rand::{Rng, rngs::ThreadRng};

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

#[must_use]
pub fn generate_friendly_strongly_random_id() -> InlineString {
    use std::fmt::Write;

    let mut rng: ThreadRng = rand::rng();

    let pet = PET_NAMES[rng.random_range(0..PET_NAMES.len())];
    let fruit = FRUIT_NAMES[rng.random_range(0..FRUIT_NAMES.len())];
    let number: u16 = rng.random_range(0..1000);

    let mut acc = InlineString::with_capacity(
        pet.len() + fruit.len() + 3 + 2, // 3 for the number, 2 for the dashes
    );

    let uuid = uuid::Uuid::new_v4();

    // We don't care about the result of this operation.
    write!(acc, "{pet}-{fruit}-{number:03}-{uuid}").ok();

    acc
}

#[must_use]
pub fn generate_friendly_random_id() -> InlineString {
    use std::fmt::Write;

    let mut rng: ThreadRng = rand::rng();

    let pet = PET_NAMES[rng.random_range(0..PET_NAMES.len())];
    let fruit = FRUIT_NAMES[rng.random_range(0..FRUIT_NAMES.len())];
    let number: u16 = rng.random_range(0..1000);

    let mut acc = InlineString::with_capacity(
        pet.len() + fruit.len() + 3 + 2, // 3 for the number, 2 for the dashes
    );

    // We don't care about the result of this operation.
    write!(acc, "{pet}-{fruit}-{number:03}").ok();

    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_constants_are_lowercase() {
        for name in &PET_NAMES {
            assert_eq!(name, &name.to_lowercase());
        }
        for name in &FRUIT_NAMES {
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

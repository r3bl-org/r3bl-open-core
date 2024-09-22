/*
 *   Copyright (c) 2024 R3BL LLC
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

use rand::Rng;

const PET_NAMES: [&str; 20] = [
    "Buddy", "Max", "Bella", "Charlie", "Lucy", "Daisy", "Molly", "Lola", "Sadie",
    "Maggie", "Bailey", "Sophie", "Chloe", "Duke", "Lily", "Rocky", "Jack", "Cooper",
    "Riley", "Zoey",
];

const FRUIT_NAMES: [&str; 20] = [
    "Apple",
    "Banana",
    "Orange",
    "Pear",
    "Peach",
    "Strawberry",
    "Grape",
    "Kiwi",
    "Mango",
    "Pineapple",
    "Watermelon",
    "Cherry",
    "Blueberry",
    "Raspberry",
    "Lemon",
    "Lime",
    "Grapefruit",
    "Plum",
    "Apricot",
    "Pomegranate",
];

pub fn generate_friendly_random_id() -> String {
    // Generate friendly pet and fruit name combination.
    let pet = {
        let mut rng = rand::thread_rng();
        let pet = PET_NAMES[rng.gen_range(0..PET_NAMES.len())];
        pet.to_lowercase()
    };

    let fruit = {
        let mut rng = rand::thread_rng();
        let fruit = FRUIT_NAMES[rng.gen_range(0..FRUIT_NAMES.len())];
        fruit.to_lowercase()
    };

    let random_number = {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..1000)
    };

    format!("{pet}-{fruit}-{random_number}")
}

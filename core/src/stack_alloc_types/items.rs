/*
 *   Copyright (c) 2025 R3BL LLC
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

use super::{InlineString, InlineVec};

// XMARK: Clever Rust, use of newtype pattern to convert various types to InlineVec<InlineString>.

/// Use the ["newtype" pattern](https://youtu.be/3-Ika3mAOGQ?si=EgcSROsbgcM5hTIY) to
/// handle conversions from various types to `InlineVec<InlineString>` aka [ItemsOwned].
/// The motivation for building this struct is to allow the seamless conversion of
/// various types into `InlineVec<InlineString>`.
///
/// `ItemsBorrowed` is a "newtype" that wraps a slice of items, which is now owned but
/// borrowed. This allows us to implement lots of `From` traits on it, and gets around the
/// orphan rule.
///
/// Here are the types that can be converted (each one is an `ItemsBorrowed`) into
/// [ItemsOwned]:
/// 1. `Vec<&str>`: `vec!["one", "two", "three"]`
/// 2. `&str`: &["one", "two", "three"]
/// 3. `Vec<String>`: `vec!["one".to_string(), "two".to_string(), "three".to_string()]`
/// 4. `InlineVec<&str>`: `smallvec::smallvec!["one", "two", "three"]`
#[derive(Debug)]
pub struct ItemsBorrowed<'a, T: AsRef<str>>(pub &'a [T]);

/// Convert [ItemsBorrowed] to [ItemsOwned].
pub fn items_owned<'a, T: AsRef<str>>(items: ItemsBorrowed<'a, T>) -> ItemsOwned {
    let mut inline_vec = InlineVec::with_capacity(items.0.len());
    for item in items.0 {
        inline_vec.push(item.as_ref().into());
    }
    inline_vec
}

/// Shorthand for the "output" type that [ItemsBorrowed] is converted into. As the name
/// implies, this is owned, while the other is borrowed.
pub type ItemsOwned = InlineVec<InlineString>;

/// Convert `ItemsBorrowed` to `InlineVec<InlineString>` aka `ItemsOwned`.
/// - Case 1: Convert `vec!["one", "two", "three"]` to `ItemsOwned`.
/// - Case 2: Convert `&["one", "two", "three"]` to `ItemsOwned`.
/// - Case 3: Convert `Vec<String>` to `ItemsOwned`.
/// - Case 4: Convert `smallvec::smallvec!["one", "two", "three"]` to `ItemsOwned`.
impl<'a, T: AsRef<str>> From<ItemsBorrowed<'a, T>> for ItemsOwned {
    fn from(items: ItemsBorrowed<'a, T>) -> Self {
        let mut inline_vec = InlineVec::new();
        for item in items.0 {
            inline_vec.push(item.as_ref().into());
        }
        inline_vec
    }
}

/// Convert `ItemsBorrowed` to `Vec<String>`.
impl<'a, T: AsRef<str>> From<ItemsBorrowed<'a, T>> for Vec<String> {
    fn from(items: ItemsBorrowed<'a, T>) -> Self {
        items.0.iter().map(|s| s.as_ref().to_string()).collect()
    }
}

/// Convert `ItemsBorrowed` to `Vec<&str>`.
impl<'a> From<ItemsBorrowed<'a, &'a str>> for Vec<&'a str> {
    fn from(items: ItemsBorrowed<'a, &'a str>) -> Self { items.0.to_vec() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_items_output() {
        // Case 1: vec!["one", "two", "three"]
        {
            let items_vec = vec!["one", "two", "three"];
            let inline_vec: ItemsOwned = items_owned(ItemsBorrowed(&items_vec));
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }

        // Case 2: &["one", "two", "three"]
        {
            let items_array = &["one", "two", "three"];
            let items = ItemsBorrowed(items_array);
            let inline_vec: ItemsOwned = items_owned(items);
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }
        // Case 3: Vec<String>
        {
            let items_string_vec: Vec<String> =
                vec!["one".to_string(), "two".to_string(), "three".to_string()];
            let items = ItemsBorrowed(&items_string_vec);
            let inline_vec: ItemsOwned = items_owned(items);
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }
        // Case 4: smallvec::smallvec!["one", "two", "three"]
        {
            let items_smallvec: InlineVec<&str> =
                smallvec::smallvec!["one", "two", "three"];
            let items = ItemsBorrowed(&items_smallvec);
            let inline_vec: ItemsOwned = items_owned(items);
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }
        // Case 5: empty vec
        {
            let items_empty: Vec<&str> = vec![];
            let items = ItemsBorrowed(&items_empty);
            let inline_vec: ItemsOwned = items_owned(items);
            assert_eq!(inline_vec.len(), 0);
        }
    }

    #[test]
    fn test_convert_items_to_vec_str() {
        // Case 1: vec!["one", "two", "three"]
        {
            let items_vec = vec!["one", "two", "three"];
            let items = ItemsBorrowed(&items_vec);
            let vec_str: Vec<&str> = items.into();
            assert_eq!(vec_str.len(), 3);
            assert_eq!(vec_str[0], "one");
            assert_eq!(vec_str[1], "two");
            assert_eq!(vec_str[2], "three");
        }

        // Case 2: &["one", "two", "three"]
        {
            let items_array = &["one", "two", "three"];
            let items = ItemsBorrowed(items_array);
            let vec_str: Vec<&str> = items.into();
            assert_eq!(vec_str.len(), 3);
            assert_eq!(vec_str[0], "one");
            assert_eq!(vec_str[1], "two");
            assert_eq!(vec_str[2], "three");
        }
    }

    #[test]
    fn test_convert_items_to_vec_string() {
        // Case 1: vec!["one", "two", "three"]
        {
            let items_vec = vec!["one", "two", "three"];
            let items = ItemsBorrowed(&items_vec);
            let vec_string: Vec<String> = items.into();
            assert_eq!(vec_string.len(), 3);
            assert_eq!(vec_string[0], "one");
            assert_eq!(vec_string[1], "two");
            assert_eq!(vec_string[2], "three");
        }

        // Case 2: &["one", "two", "three"]
        {
            let items_array = &["one", "two", "three"];
            let items = ItemsBorrowed(items_array);
            let vec_string: Vec<String> = items.into();
            assert_eq!(vec_string.len(), 3);
            assert_eq!(vec_string[0], "one");
            assert_eq!(vec_string[1], "two");
            assert_eq!(vec_string[2], "three");
        }
    }

    #[test]
    fn test_convert_items_to_inline_vec() {
        // Case 1: vec!["one", "two", "three"]
        {
            let items_vec = vec!["one", "two", "three"];
            let items = ItemsBorrowed(&items_vec);
            let inline_vec: ItemsOwned = items.into();
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }

        // Case 2: &["one", "two", "three"]
        {
            let items_array = &["one", "two", "three"];
            let items = ItemsBorrowed(items_array);
            let inline_vec: ItemsOwned = items.into();
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }

        // Case 3: Vec<String>
        {
            let items_string_vec: Vec<String> =
                vec!["one".to_string(), "two".to_string(), "three".to_string()];
            let items = ItemsBorrowed(&items_string_vec);
            let inline_vec: ItemsOwned = items.into();
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }

        // Case 4: smallvec::smallvec!["one", "two", "three"]
        {
            let items_smallvec: InlineVec<&str> =
                smallvec::smallvec!["one", "two", "three"];
            let items = ItemsBorrowed(&items_smallvec);
            let inline_vec: ItemsOwned = items.into();
            assert_eq!(inline_vec.len(), 3);
            assert_eq!(inline_vec[0], "one");
            assert_eq!(inline_vec[1], "two");
            assert_eq!(inline_vec[2], "three");
        }
    }
}

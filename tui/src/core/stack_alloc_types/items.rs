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

use std::{iter::FromIterator,
          ops::{Deref, DerefMut}};

use super::{InlineString, InlineVec};

// XMARK: Clever Rust, use of newtype pattern to convert various types to `ItemsOwned`.

/// The primary reason this module exists is to be able to easily convert from a borrowed
/// type to an owned type. This module is built to make it easy to use
/// `[r3bl_tui::readline_async::choose()]`. The `choose()` needs a list of items to allow
/// the user to choose from.
///
/// This list of items can be easily constructed from:
/// - Case 1: `vec!["one", "two", "three"]`
/// - Case 2: `&["one", "two", "three"]`
/// - Case 3: `vec!["one".to_string(), "two".to_string(), "three".to_string()]`
///
/// The ["newtype" pattern](https://youtu.be/3-Ika3mAOGQ?si=EgcSROsbgcM5hTIY) is used here
/// to facilitate the conversion of various types to `ItemsOwned`. The `ItemsOwned` type
/// is a wrapper around `InlineVec<InlineString>`, which is a stack-allocated vector of
/// strings. The `InlineVec` type is used to avoid heap allocations for small vectors,
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ItemsOwned(pub InlineVec<InlineString>);

pub fn items_owned(arg_owned_items: impl Into<ItemsOwned>) -> ItemsOwned {
    let items: ItemsOwned = arg_owned_items.into();
    items
}

mod convert_to_vec_string {
    use super::ItemsOwned;

    impl ItemsOwned {
        /// Convert `ItemsOwned` to `Vec<String>`.
        #[must_use]
        pub fn to_vec(&self) -> Vec<String> { self.into() }
    }

    /// Convert `ItemsOwned` to `Vec<String>`. For compatibility with other Rust std lib
    /// types.
    impl From<ItemsOwned> for Vec<String> {
        fn from(items: ItemsOwned) -> Self {
            items.0.iter().map(ToString::to_string).collect()
        }
    }

    /// Convert `&ItemsOwned` to `Vec<String>`. For compatibility with other Rust std lib
    /// types.
    impl From<&ItemsOwned> for Vec<String> {
        fn from(items: &ItemsOwned) -> Self {
            items.0.iter().map(ToString::to_string).collect()
        }
    }
}

mod constructors {
    use super::{ItemsOwned, InlineVec};

    impl ItemsOwned {
        #[must_use]
        pub fn new() -> Self { ItemsOwned(InlineVec::new()) }

        #[must_use]
        pub fn with_capacity(capacity: usize) -> Self {
            ItemsOwned(InlineVec::with_capacity(capacity))
        }
    }
}

mod iter_impl {
    use super::{FromIterator, InlineString, ItemsOwned, InlineVec};

    /// `FromIterator` for [`ItemsOwned`] for `collect()`.
    impl FromIterator<InlineString> for ItemsOwned {
        fn from_iter<I: IntoIterator<Item = InlineString>>(iter: I) -> Self {
            let inline_vec = iter.into_iter().collect::<InlineVec<InlineString>>();
            ItemsOwned(inline_vec)
        }
    }

    /// Iterate over a reference to [`ItemsOwned`].
    impl<'a> IntoIterator for &'a ItemsOwned {
        type Item = &'a InlineString;
        type IntoIter = std::slice::Iter<'a, InlineString>;

        fn into_iter(self) -> Self::IntoIter { self.0.iter() }
    }

    /// Iterate over [`ItemsOwned`].
    impl IntoIterator for ItemsOwned {
        type Item = InlineString;

        /// Use the `IntoIter` type that matches what `InlineVec` returns.
        type IntoIter = <InlineVec<InlineString> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
    }
}

mod convert_into_items_owned {
    use smallvec::SmallVec;

    use super::{ItemsOwned, InlineVec, InlineString};

    // XMARK: Clever Rust, to make it easy to work with arrays of any size, eg: `&["1",
    // "2"]`, `vec!["1", "2"]`, `vec!["1".to_string(), "2".to_string()]`

    /// Convert `SmallVec<[&str; N]>` to `ItemsOwned` for any size N.
    impl<const N: usize> From<SmallVec<[&str; N]>> for ItemsOwned {
        fn from(items: SmallVec<[&str; N]>) -> Self {
            let mut inline_vec = InlineVec::with_capacity(items.len());
            for item in items {
                inline_vec.push(item.into());
            }
            ItemsOwned(inline_vec)
        }
    }

    impl<const N: usize> From<&[&str; N]> for ItemsOwned {
        /// Handle arrays of any fixed size.
        fn from(items: &[&str; N]) -> Self {
            // Delegate to the slice implementation.
            ItemsOwned::from(&items[..])
        }
    }

    impl From<&[&str]> for ItemsOwned {
        /// The slice implementation is used to convert from a slice of `&str` to
        /// `ItemsOwned`.
        fn from(items: &[&str]) -> Self {
            let mut inline_vec = InlineVec::with_capacity(items.len());
            for item in items {
                inline_vec.push((*item).into());
            }
            ItemsOwned(inline_vec)
        }
    }

    impl From<Vec<&str>> for ItemsOwned {
        fn from(items: Vec<&str>) -> Self {
            let mut inline_vec = InlineVec::with_capacity(items.len());
            for item in items {
                inline_vec.push(item.into());
            }
            ItemsOwned(inline_vec)
        }
    }

    impl From<InlineVec<InlineString>> for ItemsOwned {
        fn from(items: InlineVec<InlineString>) -> Self { ItemsOwned(items) }
    }

    impl From<&InlineString> for ItemsOwned {
        fn from(item: &InlineString) -> Self {
            let mut inline_vec = InlineVec::with_capacity(1);
            inline_vec.push(item.clone());
            ItemsOwned(inline_vec)
        }
    }

    impl From<InlineString> for ItemsOwned {
        fn from(item: InlineString) -> Self {
            let mut inline_vec = InlineVec::with_capacity(1);
            inline_vec.push(item);
            ItemsOwned(inline_vec)
        }
    }

    impl From<&str> for ItemsOwned {
        fn from(item: &str) -> Self {
            let mut inline_vec = InlineVec::with_capacity(1);
            inline_vec.push(item.into());
            ItemsOwned(inline_vec)
        }
    }

    impl From<String> for ItemsOwned {
        fn from(item: String) -> Self {
            let mut inline_vec = InlineVec::with_capacity(1);
            inline_vec.push(item.into());
            ItemsOwned(inline_vec)
        }
    }

    /// Convert `Vec<String>` to `ItemsOwned`.
    impl From<Vec<String>> for ItemsOwned {
        fn from(items: Vec<String>) -> Self {
            let mut inline_vec = InlineVec::with_capacity(items.len());
            for item in items {
                inline_vec.push(item.into());
            }
            ItemsOwned(inline_vec)
        }
    }
}

mod deref_deref_mut_impl {
    use super::{Deref, ItemsOwned, InlineVec, InlineString, DerefMut};

    impl Deref for ItemsOwned {
        type Target = InlineVec<InlineString>;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for ItemsOwned {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline_vec;

    #[test]
    fn test_use_with_choose() {
        fn choose(arg: impl Into<ItemsOwned>) { let _items: ItemsOwned = arg.into(); }

        // Case 1: vec!["one", "two", "three"]
        {
            let items_vec = vec!["one", "two", "three"];
            choose(items_vec);
        }

        // Case 2: &["one", "two", "three"]
        {
            let items_array = &["one", "two", "three"];
            choose(items_array);
        }

        // Case 3: vec!["one".to_string(), "two".to_string(), "three".to_string()]
        {
            let items_vec: Vec<String> =
                vec!["one".to_string(), "two".to_string(), "three".to_string()];
            choose(items_vec);
        }
    }

    #[test]
    fn test_iterate_items_owned() {
        // Iterate over a reference to ItemsOwned.
        {
            let items_vec = vec!["one", "two", "three"];
            let inline_vec = items_owned(items_vec);
            let mut acc = InlineVec::new();
            for item in &inline_vec {
                acc.push(item.clone());
            }
            assert_eq!(acc.len(), 3);
            assert_eq!(acc[0], "one");
            assert_eq!(acc[1], "two");
            assert_eq!(acc[2], "three");
        }

        // Iterate over ItemsOwned.
        {
            let items_vec = vec!["one", "two", "three"];
            let inline_vec = items_owned(items_vec);
            let mut acc = InlineVec::new();
            for item in inline_vec {
                acc.push(item);
            }
            assert_eq!(acc.len(), 3);
            assert_eq!(acc[0], "one");
            assert_eq!(acc[1], "two");
            assert_eq!(acc[2], "three");
        }
    }

    #[test]
    fn test_convert_to_vec() {
        // Convert ItemsOwned to Vec<String> using .into().
        {
            let items_vec = vec!["one", "two", "three"];
            let inline_vec = items_owned(items_vec);
            let vec: Vec<String> = inline_vec.into();
            assert_eq!(vec.len(), 3);
            assert_eq!(vec[0], "one");
            assert_eq!(vec[1], "two");
            assert_eq!(vec[2], "three");
        }

        // Convert ItemsOwned to Vec<String> using .to_vec().
        {
            let items_vec = vec!["one", "two", "three"];
            let inline_vec = items_owned(items_vec);
            let vec: Vec<String> = inline_vec.to_vec();
            assert_eq!(vec.len(), 3);
            assert_eq!(vec[0], "one");
            assert_eq!(vec[1], "two");
            assert_eq!(vec[2], "three");
        }
    }

    #[test]
    fn test_from_smallvec_str() {
        use smallvec::{smallvec, SmallVec};

        let small: SmallVec<[&str; 3]> = smallvec!["a", "b", "c"];
        let items_owned = ItemsOwned::from(small);

        assert_eq!(items_owned.len(), 3);
        assert_eq!(items_owned[0], "a");
        assert_eq!(items_owned[1], "b");
        assert_eq!(items_owned[2], "c");
    }

    #[test]
    fn test_from_inline_vec_str() {
        let small = inline_vec!["a", "b", "c"];
        let items_owned = ItemsOwned::from(small);

        assert_eq!(items_owned.len(), 3);
        assert_eq!(items_owned[0], "a");
        assert_eq!(items_owned[1], "b");
        assert_eq!(items_owned[2], "c");
    }
}

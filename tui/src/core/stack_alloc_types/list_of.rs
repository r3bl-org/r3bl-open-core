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

//! This module provides a custom [List] struct that wraps around a [`SmallVec`] to
//! provide additional functionality and traits implementations for ease of use within the
//! `tui` crate. Specifically so that the  [From] trait can be implemented for [List] of
//! `T`. Where `T` is any number of types in the tui crate.
//!
//! The [List] struct is designed to be a more flexible and efficient alternative to a
//! standard [Vec], with the ability to implement custom traits and macros.
//! 1. This gets around the orphan rule for implementing standard library traits.
//! 2. Using a [`SmallVec`] allows for stack allocation rather than heap allocation. Its
//!    internal backing store is essentially an array-vec. Starts out as a stack allocated
//!    array which can spill over into the heap if needed.
//!
//! # Features
//!
//! - Implements [`AddAssign`] trait for adding items, other lists, and vectors to the
//!   list.
//! - Implements [From] trait for converting from [`ListStorage`] and [Vec] to [List].
//! - Implements [Deref] and [`DerefMut`] traits for easy access to the inner
//!   [`SmallVec`].
//! - Provides a [`crate::list`!] macro for convenient list creation.
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::{list, List};
//!
//! let mut list = List::new();
//! list += 1;
//! list += vec![2, 3, 4];
//!
//! let another_list = list![5, 6, 7];
//! list += another_list;
//! ```

use std::ops::{AddAssign, Deref, DerefMut};

use sizing_list_of::ListStorage;
use smallvec::SmallVec;

use crate::InlineVecStr;

/// This needs to be accessible by the rest of the crate, and anyone using the [List]
/// struct.
pub mod sizing_list_of {
    use super::SmallVec;
    pub type ListStorage<T> = SmallVec<[T; DEFAULT_LIST_STORAGE_SIZE]>;
    const DEFAULT_LIST_STORAGE_SIZE: usize = 8;
}

/// Redundant struct to [Vec]. Added so that [From] trait can be implemented for for
/// [List] of `T`. Where `T` is any number of types in the tui crate.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct List<T> {
    pub inner: ListStorage<T>,
}

impl<T> List<T> {
    #[must_use]
    pub fn with_capacity(size: usize) -> Self {
        Self {
            inner: ListStorage::with_capacity(size),
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: ListStorage::new(),
        }
    }
}

/// Add (other) item to list (self).
impl<T> AddAssign<T> for List<T> {
    fn add_assign(&mut self, other_item: T) { self.push(other_item); }
}

/// Add (other) list to list (self).
impl<T> AddAssign<List<T>> for List<T> {
    fn add_assign(&mut self, other_list: List<T>) { self.extend(other_list.inner); }
}

/// Add (other) vec to list (self).
impl<T> AddAssign<Vec<T>> for List<T> {
    fn add_assign(&mut self, other_vec: Vec<T>) { self.extend(other_vec); }
}

impl<'a> From<InlineVecStr<'a>> for List<&'a str> {
    fn from(other: InlineVecStr<'a>) -> Self {
        let mut it = List::with_capacity(other.len());
        it.extend(other);
        it
    }
}

impl<T> From<Vec<T>> for List<T> {
    fn from(other: Vec<T>) -> Self {
        let mut it = List::with_capacity(other.len());
        it.extend(other);
        it
    }
}

impl<T> Deref for List<T> {
    type Target = ListStorage<T>;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> DerefMut for List<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

#[macro_export]
macro_rules! list {
    (
        $($item: expr),*
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
        {
            #[allow(unused_mut)]
            let mut it = $crate::List::new();
            $(
                it.inner.push($item);
            )*
            it
        }
    };
}

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! General-purpose **stack-allocated** list type with balanced performance.
//!
//! # [List] - Balanced Stack Allocation
//!
//! - Uses `SmallVec<[T; 8]>` as a compromise between performance and stack safety
//! - ~328 bytes on stack
//! - Good for most use cases outside specialized parsing/rendering
//!
//! # Why This Size?
//!
//! Originally sized at 8 elements to balance:
//! - **Performance**: Avoids heap allocations for common small lists
//! - **Stack safety**: Won't cause overflow in moderately deep recursion
//!
//! # See Also
//!
//! For specialized use cases, see:
//! - [`crate::RenderList`] - Performance-optimized (`SmallVec[16]`) for hot rendering
//!   paths
//! - [`crate::ParseList`] - Heap-allocated (`Vec`) for stack safety with deep recursion
//!
//! # Features
//!
//! Implements:
//! - [`AddAssign`] trait for adding items, other lists, and vectors
//! - [From] trait for converting from [Vec]
//! - [Deref] and [`DerefMut`] traits for easy access to inner storage
//! - Companion macro: [`crate::list`!]
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::{list, List};
//!
//! // General purpose
//! let mut list = List::new();
//! list += 1;
//! list += vec![2, 3, 4];
//! ```

use crate::InlineVecStr;
use sizing_list_of::ListStorage;
use smallvec::SmallVec;
use std::ops::{AddAssign, Deref, DerefMut};

/// Storage type alias for [List].
pub mod sizing_list_of {
    use super::SmallVec;

    /// General storage: `SmallVec[8]` balanced default (~328 bytes on stack).
    /// Reverted from 16→8 to avoid stack overflow in parser.
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

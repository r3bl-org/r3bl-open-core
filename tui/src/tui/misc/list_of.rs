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

use std::ops::{AddAssign, Deref, DerefMut};

use r3bl_core::MicroVecBackingStore;
use serde::{Deserialize, Serialize};

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

/// Redundant struct to [Vec]. Added so that [From] trait can be implemented for for [List] of
/// `T`. Where `T` is any number of types in the tui crate.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct List<T> {
    pub inner: MicroVecBackingStore<T>,
}

impl<T> List<T> {
    pub fn with_capacity(size: usize) -> Self {
        Self {
            inner: MicroVecBackingStore::with_capacity(size),
        }
    }

    pub fn new() -> Self {
        Self {
            inner: MicroVecBackingStore::new(),
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

impl<T> From<MicroVecBackingStore<T>> for List<T> {
    fn from(other: MicroVecBackingStore<T>) -> Self { Self { inner: other } }
}

impl<T> From<Vec<T>> for List<T> {
    fn from(other: Vec<T>) -> Self {
        let mut it = List::with_capacity(other.len());
        it.extend(other);
        it
    }
}

impl<T> Deref for List<T> {
    type Target = MicroVecBackingStore<T>;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> DerefMut for List<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

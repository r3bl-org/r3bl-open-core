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

use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! list {
     (
         $($item: expr),*
         $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
     ) => {
         {
             #[allow(unused_mut)]
             let mut it = List::new();
             $(
                 it.inner.push($item);
             )*
             it
         }
     };
 }

/// Redundant struct to [Vec]. Added so that [From] trait can be implemented for for [List] of
/// `T`. Where `T` is any number of types in the tui crate.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, size_of::SizeOf)]
pub struct List<T>
where
    T: size_of::SizeOf,
{
    pub inner: Vec<T>,
}

impl<T> List<T>
where
    T: size_of::SizeOf,
{
    pub fn with_capacity(size: usize) -> Self {
        Self {
            inner: Vec::with_capacity(size),
        }
    }

    pub fn new() -> Self { Self { inner: Vec::new() } }
}

/// Add (other) item to list (self).
impl<T> AddAssign<T> for List<T>
where
    T: size_of::SizeOf,
{
    fn add_assign(&mut self, other_item: T) { self.push(other_item); }
}

/// Add (other) list to list (self).
impl<T> AddAssign<List<T>> for List<T>
where
    T: size_of::SizeOf,
{
    fn add_assign(&mut self, other_list: List<T>) { self.extend(other_list.inner); }
}

/// Add (other) vec to list (self).
impl<T> AddAssign<Vec<T>> for List<T>
where
    T: size_of::SizeOf,
{
    fn add_assign(&mut self, other_vec: Vec<T>) { self.extend(other_vec); }
}

impl<T> From<List<T>> for Vec<T>
where
    T: size_of::SizeOf,
{
    fn from(list: List<T>) -> Self { list.inner }
}

impl<T> From<Vec<T>> for List<T>
where
    T: size_of::SizeOf,
{
    fn from(other: Vec<T>) -> Self { Self { inner: other } }
}

impl<T> Deref for List<T>
where
    T: size_of::SizeOf,
{
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> DerefMut for List<T>
where
    T: size_of::SizeOf,
{
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

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

//! Data structures to make it easier to work w/ lazily computed values and
//! caching them.

use std::{collections::HashMap, hash::Hash};

/// This struct allows users to create a lazy hash map. A function must be
/// provided that computes the values when they are first requested. These
/// values are cached for the lifetime this struct.
///
/// # Examples
///
/// ```rust
/// use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
///
/// use r3bl_rs_utils::utils::LazyMemoValues;
///
/// // These are copied in the closure below.
/// let arc_atomic_count = AtomicUsize::new(0);
/// let mut a_variable = 123;
/// let mut a_flag = false;
///
/// let mut generate_value_fn = LazyMemoValues::new(|it| {
///   arc_atomic_count.fetch_add(1, SeqCst);
///   a_variable = 12;
///   a_flag = true;
///   a_variable + it
/// });
///
/// assert_eq!(arc_atomic_count.load(SeqCst), 0);
/// assert_eq!(generate_value_fn.get_ref(&1), &13);
/// assert_eq!(arc_atomic_count.load(SeqCst), 1);
/// assert_eq!(generate_value_fn.get_ref(&1), &13); // Won't regenerate the value.
/// assert_eq!(arc_atomic_count.load(SeqCst), 1); // Doesn't change.
/// assert_eq!(generate_value_fn.get_ref(&2), &14);
/// assert_eq!(arc_atomic_count.load(SeqCst), 2);
/// assert_eq!(generate_value_fn.get_ref(&2), &14);
/// assert_eq!(generate_value_fn.get_copy(&2), 14);
/// assert_eq!(a_variable, 12);
/// assert_eq!(a_flag, true);
/// ```
#[derive(Debug)]
pub struct LazyMemoValues<F, T, V>
where
    F: FnMut(&T) -> V,
    T: Clone + Eq + Hash,
    V: Clone,
{
    pub create_value_fn: F,
    pub value_map: HashMap<T, V>,
}

impl<F, T, V> LazyMemoValues<F, T, V>
where
    F: FnMut(&T) -> V,
    T: Clone + Eq + Hash,
    V: Clone,
{
    pub fn new(create_value_fn: F) -> Self {
        LazyMemoValues {
            create_value_fn,
            value_map: HashMap::new(),
        }
    }

    pub fn get_ref(&mut self, arg: &T) -> &V {
        if !self.value_map.contains_key(arg) {
            let arg = arg.clone();
            let value = (self.create_value_fn)(&arg);
            self.value_map.insert(arg, value);
        }
        self.value_map.get(arg).as_ref().unwrap()
    }

    pub fn get_copy(&mut self, arg: &T) -> V { self.get_ref(arg).clone() }
}

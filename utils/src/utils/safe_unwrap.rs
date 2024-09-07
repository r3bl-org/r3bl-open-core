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

//! Functions that make it easy to unwrap a value safely.
//!
//! These functions are provided to improve the ergonomics of using wrapped values in
//! Rust. Examples of wrapped values are `<Arc<RwLock<T>>`, and `<Option>`.
//!
//! These functions are inspired by Kotlin scope functions & TypeScript expression based
//! language library which can be found [here on
//! `r3bl-ts-utils`](https://github.com/r3bl-org/r3bl-ts-utils).

use std::{fmt::Debug,
          sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

pub type ReadGuarded<'a, T> = RwLockReadGuard<'a, T>;
pub type WriteGuarded<'a, T> = RwLockWriteGuard<'a, T>;

/// Macros to unwrap various locks.
#[macro_export]
macro_rules! unwrap {
    (mutex from $arc_mutex: expr) => {
        $arc_mutex.lock().unwrap()
    };

    (r_lock from $arc_rw_lock: expr) => {
        $arc_rw_lock.read().unwrap()
    };

    (w_lock from $arc_rw_lock: expr) => {
        $arc_rw_lock.write().unwrap()
    };
}

// Functions to unwrap various locks.

pub fn unwrap_arc_read_lock_and_call<T, F, R>(
    arc_lock_wrapped_value: &Arc<RwLock<T>>,
    receiver_fn: &mut F,
) -> R
where
    F: FnMut(&T) -> R,
{
    let arc_clone = arc_lock_wrapped_value.clone();
    let read_guarded: ReadGuarded<'_, T> = unwrap!(r_lock from arc_clone);
    receiver_fn(&read_guarded)
}

pub fn unwrap_arc_write_lock_and_call<T, F, R>(
    arc_lock_wrapped_value: &Arc<RwLock<T>>,
    receiver_fn: &mut F,
) -> R
where
    F: FnMut(&mut T) -> R,
{
    let arc_clone = arc_lock_wrapped_value.clone();
    let mut write_guarded: WriteGuarded<'_, T> = unwrap!(w_lock from arc_clone);
    receiver_fn(&mut write_guarded)
}

// Helper lambdas.

pub fn with_mut<T, F, R>(arg: &mut T, receiver_fn: &mut F) -> R
where
    F: FnMut(&mut T) -> R,
{
    receiver_fn(arg)
}

pub fn with<T, F, R>(arg: T, receiver_fn: F) -> R
where
    F: Fn(T) -> R,
{
    receiver_fn(arg)
}

pub fn call_if_some<T, F>(option_wrapped_value: &Option<T>, receiver_fn: &F)
where
    F: Fn(&T),
{
    if let Some(value) = option_wrapped_value {
        receiver_fn(value);
    }
}

pub fn call_if_none<T, F>(option_wrapped_value: &Option<T>, receiver_fn: &F)
where
    F: Fn(),
{
    if (option_wrapped_value).is_none() {
        receiver_fn();
    }
}

pub fn call_if_ok<T, F, E>(option_wrapped_value: &Result<T, E>, receiver_fn: &F)
where
    F: Fn(&T),
{
    if let Ok(value) = option_wrapped_value {
        receiver_fn(value);
    }
}

pub fn call_if_err<T, F, E>(option_wrapped_value: &Result<T, E>, receiver_fn: &F)
where
    F: Fn(&E),
    T: Debug,
{
    if option_wrapped_value.is_err() {
        receiver_fn(option_wrapped_value.as_ref().unwrap_err());
    }
}

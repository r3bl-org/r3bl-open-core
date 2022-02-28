/*
 Copyright 2022 R3BL LLC

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

      https://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
*/

//! Functions that make it easy to unwrap a value safely. These functions are provided to improve
//! the ergonomics of using wrapped values in Rust. Examples of wrapped values are
//! `<Arc<RwLock<T>>`, and `<Option>`. These functions are inspired by Kotlin scope functions &
//! TypeScript expression based language library which can be found [here on
//! `r3bl-ts-utils`](https://github.com/r3bl-org/r3bl-ts-utils).

use std::{
  fmt::Debug,
  sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub type ReadGuarded<'a, T> = RwLockReadGuard<'a, T>;
pub type WriteGuarded<'a, T> = RwLockWriteGuard<'a, T>;

pub fn unwrap_arc_read_lock_and_call<T, F, R>(
  arc_lock_wrapped_value: &Arc<RwLock<T>>,
  receiver_fn: &mut F,
) -> R
where
  F: FnMut(&T) -> R,
  T: 'static + Send + Sync + Clone + Debug,
{
  let arc_copy = arc_lock_wrapped_value.clone();
  let read_guard: ReadGuarded<T> = arc_copy.read().unwrap();
  receiver_fn(&*read_guard)
}

pub fn unwrap_arc_write_lock_and_call<T, F, R>(
  arc_lock_wrapped_value: &Arc<RwLock<T>>,
  receiver_fn: &mut F,
) -> R
where
  F: FnMut(&mut T) -> R,
  T: 'static + Send + Sync + Clone + Debug,
{
  let arc_copy = arc_lock_wrapped_value.clone();
  let mut write_guard: WriteGuarded<T> = arc_copy.write().unwrap();
  receiver_fn(&mut write_guard)
}

pub fn call_if_some<T, F>(
  option_wrapped_value: &Option<T>,
  receiver_fn: &F,
) where
  F: Fn(&T),
  T: 'static + Send + Sync + Clone + Debug,
{
  if let Some(value) = option_wrapped_value {
    receiver_fn(value);
  }
}

pub fn with_mut<T, F, R>(
  arg: &mut T,
  receiver_fn: &mut F,
) -> R
where
  F: FnMut(&mut T) -> R,
{
  receiver_fn(arg)
}

pub fn with<T, F, R>(
  arg: T,
  receiver_fn: F,
) -> R
where
  F: Fn(T) -> R,
{
  receiver_fn(arg)
}

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

#![allow(non_camel_case_types)]

type _Arc<T> = std::sync::Arc<T>;
type _RwLock<T> = tokio::sync::RwLock<T>;
type _WriteG<'a, T> = tokio::sync::RwLockWriteGuard<'a, T>;
type _ReadG<'a, T> = tokio::sync::RwLockReadGuard<'a, T>;

/// This trait marks a type as being safe to share across threads (parallel
/// safe) and tasks (async safe).
///
/// [Async trait docs](https://github.com/dtolnay/async-trait).
#[async_trait::async_trait]
pub trait SafeToShare<T> {
  async fn set_value(&self, value: T);
  async fn get_value<'a>(&'a self) -> _ReadG<'a, T>;
  fn get_ref(&self) -> _Arc<_RwLock<T>>;
}

/// This trait marks a type as being safe to mutate (interior mutability) across
/// threads (parallel safe) and tasks (async safe). These are just convenience
/// static methods. You can simply use the `read()` and `write()` methods
/// directly on the `Arc` reference.
///
/// [Async trait docs](https://github.com/dtolnay/async-trait).
#[async_trait::async_trait]
pub trait SafeToMutate<T> {
  async fn with_ref_get_value_w_lock<'a>(my_arc: &'a _Arc<_RwLock<T>>) -> _WriteG<'a, T>;
  async fn with_ref_get_value_r_lock<'a>(my_arc: &'a _Arc<_RwLock<T>>) -> _ReadG<'a, T>;
  async fn with_ref_set_value(my_arc: &_Arc<_RwLock<T>>, value: T);
}

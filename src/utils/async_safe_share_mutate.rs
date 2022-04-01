/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
*/

#![allow(non_camel_case_types)]

type ARC<T> = std::sync::Arc<T>;
type RWLOCK<T> = tokio::sync::RwLock<T>;
type RWLOCK_WG<'a, T> = tokio::sync::RwLockWriteGuard<'a, T>;
type RWLOCK_RG<'a, T> = tokio::sync::RwLockReadGuard<'a, T>;

/// [Async trait docs](https://github.com/dtolnay/async-trait)
#[async_trait::async_trait]
pub trait SafeToShare<T> {
  async fn set_value(
    &self,
    value: T,
  );
  async fn get_value<'a>(&'a self) -> RWLOCK_RG<'a, T>;
  fn get_ref(&self) -> ARC<RWLOCK<T>>;
}

/// [Async trait docs](https://github.com/dtolnay/async-trait)
#[async_trait::async_trait]
pub trait SafeToMutate<T> {
  async fn with_ref_get_value_w_lock<'a>(my_arc: &'a ARC<RWLOCK<T>>) -> RWLOCK_WG<'a, T>;
  async fn with_ref_get_value_r_lock<'a>(my_arc: &'a ARC<RWLOCK<T>>) -> RWLOCK_RG<'a, T>;
  async fn with_ref_set_value(
    my_arc: &ARC<RWLOCK<T>>,
    value: T,
  );
}

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

use async_trait::async_trait;

#[async_trait]
pub trait AsyncReducer<S, A>
where
  S: Sync + Send,
  A: Sync + Send,
{
  async fn run(&self, action: &A, state: &S) -> S;

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  #[allow(clippy::all)]
  fn new() -> AsyncReducerItem<S, A>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Box::new(Self::default())
  }
}

pub type AsyncReducerTraitObject<S, A> = dyn AsyncReducer<S, A> + Send + Sync;
pub type AsyncReducerItem<S, A> = Box<dyn AsyncReducer<S, A> + Send + Sync>;
pub type AsyncReducerVec<S, A> = Vec<AsyncReducerItem<S, A>>;

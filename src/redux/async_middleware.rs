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

use super::StoreStateMachine;
use async_trait::async_trait;
use r3bl_rs_utils_macro::make_safe_async_fn_wrapper;

make_safe_async_fn_wrapper! {
  named SafeMiddlewareFnWrapper<A, S>
  containing fn_mut
  of_type FnMut(A, S) -> Option<A>
}

// FIXME: add new async trait here
// FIXME: add vec of this async trait here
// FIXME: use this new vec in async_store_state_machine.rs

#[async_trait]
pub trait AsyncMiddleware<S, A>
where
  A: Sync + Send,
  S: Sync + Send,
{
  async fn run(
    &self,
    action: A,
    store_ref: ARC<RWLOCK<StoreStateMachine<S, A>>>,
  ) -> Option<A>;

  fn new() -> Self
  where
    Self: Sized;
}

#[derive(Default)]
pub struct AsyncMiddlewareVec<S, A> {
  pub vec: Vec<ARC<RWLOCK<dyn AsyncMiddleware<A, S> + Send + Sync>>>,
}

impl<S, A> AsyncMiddlewareVec<S, A> {
  pub fn push(
    &mut self,
    middleware: ARC<RWLOCK<dyn AsyncMiddleware<A, S> + Send + Sync>>,
  ) {
    self.vec.push(middleware);
  }

  pub fn clear(&mut self) {
    self.vec.clear();
  }
}

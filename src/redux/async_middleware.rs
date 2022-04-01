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

// Imports.
use std::{
  marker::{Send, Sync},
  sync::Arc,
};
use tokio::{sync::RwLock, task::JoinHandle};

/// Excellent resources on lifetimes and returning references:
/// 1. https://stackoverflow.com/questions/59442080/rust-pass-a-function-reference-to-threads
/// 2. https://stackoverflow.com/questions/68547268/cannot-borrow-data-in-an-arc-as-mutable
/// 3. https://willmurphyscode.net/2018/04/25/fixing-a-simple-lifetime-error-in-rust/
pub type SafeMiddlewareFn<A> = Arc<RwLock<dyn FnMut(A) -> Option<A> + Sync + Send>>;
//                             ^^^^^^^^^^                             ^^^^^^^^^^^
//                             Safe to pass      Declare`FnMut` has thread safety
//                             around.           requirement to rust compiler.

#[derive(Clone)]
pub struct SafeMiddlewareFnWrapper<A> {
  fn_mut: SafeMiddlewareFn<A>,
}

impl<A> SafeMiddlewareFnWrapper<A>
where
  A: Sync + Send + 'static,
{
  pub fn from(
    fn_mut: impl FnMut(A) -> Option<A> + Send + Sync + 'static
  ) -> SafeMiddlewareFnWrapper<A> {
    SafeMiddlewareFnWrapper::set(Arc::new(RwLock::new(fn_mut)))
  }

  fn set(fn_mut: SafeMiddlewareFn<A>) -> Self {
    Self { fn_mut }
  }

  /// Get a clone of the `fn_mut` field (which holds a thread safe `FnMut`).
  pub fn get(&self) -> SafeMiddlewareFn<A> {
    self.fn_mut.clone()
  }

  /// This is an async function. Make sure to use `await` on the return value.
  pub fn spawn(
    &self,
    action: A,
  ) -> JoinHandle<Option<A>> {
    let arc_lock_fn_mut = self.get();
    tokio::spawn(async move {
      // Actually run function.
      let mut fn_mut = arc_lock_fn_mut.write().await;
      fn_mut(action)
    })
  }
}

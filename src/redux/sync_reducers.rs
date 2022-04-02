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

use std::sync::Arc;

/// Reducer function signature. This is not [`Sized`].
pub type ReducerFn<S, A> = dyn Fn(&S, &A) -> S + Sync + Send + 'static;

/// [`ReducerFn`] has to be wrapped in an [`Arc`] because it is [`Sized`] and safe to
/// share between threads.
/// 1. It does not allow interior mutability.
/// 2. It is not thread safe, since it performs no locking.
pub struct ShareableReducerFn<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  fn_mut: Arc<ReducerFn<S, A>>,
}

impl<S, A> ShareableReducerFn<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  /// Constructing a [`ReducerFnWrapper`] using a sized argument `fn_mut`, which can be a
  /// normal function or a lambda.
  pub fn new(
    fn_mut: impl Fn(&S, &A) -> S + Send + Sync + 'static
  ) -> ShareableReducerFn<S, A> {
    Self {
      fn_mut: Arc::new(fn_mut),
    }
  }

  pub fn invoke(
    &self,
    state: &S,
    action: &A,
  ) -> S {
    let fn_mut_ref = self.fn_mut.clone();
    fn_mut_ref(state, action)
  }
}

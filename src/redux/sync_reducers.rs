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

use std::sync::{Arc, Mutex};

/// Reducer function.
pub type ReducerFn<S, A> = dyn Fn(&S, &A) -> S;
pub type SafeReducerFn<S, A> = Arc<Mutex<dyn Fn(&S, &A) -> S + Send + Sync + 'static>>;

#[derive(Clone)]
pub struct ReducerFnWrapper<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  fn_mut: SafeReducerFn<S, A>,
}

impl<S, A> ReducerFnWrapper<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  pub fn from(
    fn_mut: impl Fn(&S, &A) -> S + Send + Sync + 'static
  ) -> ReducerFnWrapper<S, A> {
    ReducerFnWrapper::set(Arc::new(Mutex::new(fn_mut)))
  }

  fn set(fn_mut: SafeReducerFn<S, A>) -> Self {
    Self { fn_mut }
  }

  pub fn get(&self) -> SafeReducerFn<S, A> {
    self.fn_mut.clone()
  }

  pub fn invoke(
    &self,
    state: &S,
    action: &A,
  ) -> S {
    let arc_locked_fn_mut = self.get();
    let fn_mut = arc_locked_fn_mut.lock().unwrap();
    fn_mut(state, action)
  }
}

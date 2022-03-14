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
use tokio::{task::JoinHandle, sync::RwLock};

/// Subscriber function.
pub type SafeSubscriberFn<S> = Arc<RwLock<dyn FnMut(S) + Sync + Send>>;

pub struct SafeSubscriberFnWrapper<S> {
  fn_mut: SafeSubscriberFn<S>,
}

impl<S: Sync + Send + 'static> SafeSubscriberFnWrapper<S> {
  pub fn from(
    fn_mut: impl FnMut(S) -> () + Send + Sync + 'static
  ) -> SafeSubscriberFnWrapper<S> {
    SafeSubscriberFnWrapper::set(Arc::new(RwLock::new(fn_mut)))
  }

  fn set(fn_mut: SafeSubscriberFn<S>) -> Self {
    Self { fn_mut }
  }

  pub fn get(&self) -> SafeSubscriberFn<S> {
    self.fn_mut.clone()
  }

  pub fn spawn(
    &self,
    state: S,
  ) -> JoinHandle<()> {
    let arc_lock_fn_mut = self.get();
    tokio::spawn(async move {
      // Actually run function.
      let mut fn_mut = arc_lock_fn_mut.write().await;
      fn_mut(state)
    })
  }
}

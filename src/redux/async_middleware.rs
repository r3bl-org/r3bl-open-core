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

use super::StoreStateMachine;
use async_trait::async_trait;
use tokio::sync::RwLock;

#[async_trait]
pub trait AsyncMiddleware<S, A>
where
  S: Sync + Send,
  A: Sync + Send,
{
  async fn run(
    &self,
    action: A,
    store_ref: Arc<RwLock<StoreStateMachine<S, A>>>,
  ) -> Option<A>;

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  fn new() -> Arc<RwLock<dyn AsyncMiddleware<S, A> + Send + Sync + 'static>>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Arc::new(RwLock::new(Self::default()))
  }
}

#[derive(Default)]
pub struct AsyncMiddlewareVec<S, A> {
  pub vec: Vec<Arc<RwLock<dyn AsyncMiddleware<S, A> + Send + Sync>>>,
}

impl<S, A> AsyncMiddlewareVec<S, A> {
  pub fn push(
    &mut self,
    middleware: Arc<RwLock<dyn AsyncMiddleware<S, A> + Send + Sync>>,
  ) {
    self.vec.push(middleware);
  }

  pub fn clear(&mut self) {
    self.vec.clear();
  }
}

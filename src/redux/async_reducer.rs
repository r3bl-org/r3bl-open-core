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

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait AsyncReducer<S, A>
where
  S: Sync + Send,
  A: Sync + Send,
{
  async fn run(
    &self,
    action: &A,
    state: &S,
  ) -> S;

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  fn new() -> Arc<RwLock<dyn AsyncReducer<S, A> + Send + Sync + 'static>>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Arc::new(RwLock::new(Self::default()))
  }
}

#[derive(Default)]
pub struct AsyncReducerVec<S, A> {
  pub vec: Vec<Arc<RwLock<dyn AsyncReducer<S, A> + Send + Sync>>>,
}

impl<S, A> AsyncReducerVec<S, A> {
  pub fn push(
    &mut self,
    reducer: Arc<RwLock<dyn AsyncReducer<S, A> + Send + Sync>>,
  ) {
    self.vec.push(reducer);
  }

  pub fn clear(&mut self) {
    self.vec.clear();
  }

  pub fn clone(&mut self) -> Vec<Arc<RwLock<dyn AsyncReducer<S, A> + Send + Sync>>> {
    self.vec.clone()
  }
}

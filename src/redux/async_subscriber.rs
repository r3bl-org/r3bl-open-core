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

#[async_trait]
pub trait AsyncSubscriber<S>
where
  S: Sync + Send,
{
  async fn run(
    &self,
    state: S,
  );

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  fn new() -> Box<dyn AsyncSubscriber<S> + Send + Sync>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Box::new(Self::default())
  }
}

#[derive(Default)]
pub struct AsyncSubscriberVec<S> {
  pub vec: Vec<Box<dyn AsyncSubscriber<S> + Send + Sync>>,
}

impl<S> AsyncSubscriberVec<S> {
  pub fn push(
    &mut self,
    middleware: Box<dyn AsyncSubscriber<S> + Send + Sync>,
  ) {
    self.vec.push(middleware);
  }

  pub fn clear(&mut self) {
    self.vec.clear();
  }
}

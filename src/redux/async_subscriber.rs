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
  async fn run(&self, state: S);

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  #[allow(clippy::all)]
  fn new() -> AsyncSubscriberItem<S>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Box::new(Self::default())
  }
}

pub type AsyncSubscriberTraitObject<S> = dyn AsyncSubscriber<S> + Send + Sync;
pub type AsyncSubscriberItem<S> = Box<dyn AsyncSubscriber<S> + Send + Sync>;
pub type AsyncSubscriberVec<S> = Vec<AsyncSubscriberItem<S>>;

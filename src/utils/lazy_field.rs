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

pub trait LazyExecutor<T>
where
  T: Send + Sync,
{
  fn compute(&mut self) -> T;

  /// Book: <https://doc.rust-lang.org/book/ch10-02-traits.html>
  #[allow(clippy::all)]
  fn new() -> Box<dyn LazyExecutor<T> + Send + Sync>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Box::new(Self::default())
  }
}

pub struct LazyField<T>
where
  T: Send + Sync,
{
  pub lazy_executor: Box<dyn LazyExecutor<T> + Send + Sync>,
  pub field: T,
  pub has_computed: bool,
}

impl<T> LazyField<T>
where
  T: Send + Sync,
  T: Default + Clone,
{
  pub fn new(lazy_executor: Box<dyn LazyExecutor<T> + Send + Sync>) -> Self {
    Self {
      lazy_executor,
      field: T::default(),
      has_computed: false,
    }
  }

  pub fn compute(&mut self) -> T {
    if self.has_computed {
      self.field.clone()
    } else {
      self.field = self.lazy_executor.compute();
      self.has_computed = true;
      self.field.clone()
    }
  }
}

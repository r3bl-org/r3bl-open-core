/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
*/

use core::fmt::Debug;

const DEBUG: bool = false;

pub type LazyResult<T> = Result<T, Box<LazyExecutionError>>;
pub type LazyComputeFunction<T> = Box<dyn FnMut() -> LazyResult<T>>;

pub trait LazyExecutor<T>
where
  T: Send + Sync,
{
  fn compute(&mut self) -> T;

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  fn new() -> Box<dyn LazyExecutor<T> + Send + Sync>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Box::new(Self::default())
  }
}

pub struct LazyField2<T>
where
  T: Send + Sync,
{
  pub lazy_executor: Box<dyn LazyExecutor<T> + Send + Sync>,
  pub field: T,
  pub has_computed: bool,
}

impl<T> LazyField2<T>
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
      return self.field.clone();
    } else {
      self.field = self.lazy_executor.compute();
      self.has_computed = true;
      return self.field.clone();
    }
  }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct LazyExecutionError {
  err_type: LazyExecutionErrorType,
}

#[derive(Debug, Clone, Copy)]
pub enum LazyExecutionErrorType {
  ComputeFieldFnError,
  LazyComputeFunctionNotProvided,
}

#[derive(Debug, Clone)]
pub enum LazyExecutionState<T>
where
  T: Clone + Copy + Debug,
{
  NotComputedYet,
  ComputedResultingInError(LazyExecutionError),
  ComputedResultingInValue(T),
}

pub struct LazyField<T>
where
  T: Clone + Copy + Debug,
{
  pub field: LazyExecutionState<T>,
  pub compute_field_value_fn: Option<LazyComputeFunction<T>>,
}

impl<T> std::fmt::Debug for LazyField<T>
where
  T: Clone + Copy + Debug,
{
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    let msg = match &self.compute_field_value_fn {
      Some(_) => Some("compute_field_value_fn is set"),
      None => Some("compute_field_value_fn is *not* set"),
    };
    f.debug_struct("LazyField")
      .field("field", &self.field)
      .field("compute_field_value_fn", &msg)
      .finish()
  }
}

impl<T> Default for LazyField<T>
where
  T: Clone + Copy + Debug,
{
  fn default() -> Self {
    Self {
      field: LazyExecutionState::NotComputedYet,
      compute_field_value_fn: None,
    }
  }
}

impl<T> LazyField<T>
where
  T: Clone + Copy + Debug,
{
  pub fn new(boxed_compute_field_value_fn: LazyComputeFunction<T>) -> Self {
    Self {
      field: LazyExecutionState::NotComputedYet,
      compute_field_value_fn: Some(boxed_compute_field_value_fn),
    }
  }

  pub fn access_field(&mut self) -> LazyResult<T> {
    if self
      .compute_field_value_fn
      .is_none()
    {
      let err = LazyExecutionError {
        err_type: LazyExecutionErrorType::LazyComputeFunctionNotProvided,
      };
      return Err(Box::new(err));
    }

    let compute_field_value_fn = self
      .compute_field_value_fn
      .as_mut()
      .unwrap();

    if let LazyExecutionState::NotComputedYet = self.field {
      let computed_field_value_result = &(compute_field_value_fn)();
      match computed_field_value_result {
        Ok(computed_field_value) => {
          if DEBUG {
            println!("once - computing value");
          }
          self.field =
            LazyExecutionState::ComputedResultingInValue(computed_field_value.clone());
          return Ok(computed_field_value.clone());
        }
        Err(e) => {
          if DEBUG {
            println!("once - problem computing value");
          }
          let e_clone = *e.clone();
          self.field = LazyExecutionState::ComputedResultingInError(e_clone);
          return Err(e.clone());
        }
      }
    }

    if let LazyExecutionState::ComputedResultingInValue(value) = self.field {
      if DEBUG {
        println!("returning cached value");
      }
      return Ok(value.clone());
    }

    if let LazyExecutionState::ComputedResultingInError(e) = self.field {
      if DEBUG {
        println!("returning cached error");
      }
      return Err(Box::new(e));
    }

    panic!("unreachable");
  }
}

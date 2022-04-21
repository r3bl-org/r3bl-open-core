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

use crate::redux::{
  AsyncMiddlewareSpawnsVec, AsyncMiddlewareVec, AsyncReducerVec, AsyncSubscriberVec,
};
use core::{fmt::Debug, hash::Hash};

pub struct StoreStateMachine<S, A>
where
  S: Sync + Send,
  A: Sync + Send,
{
  pub state: S,
  pub history: Vec<S>,
  pub middleware_vec: AsyncMiddlewareVec<S, A>,
  pub middleware_spawns_vec: AsyncMiddlewareSpawnsVec<S, A>,
  pub subscriber_vec: AsyncSubscriberVec<S>,
  pub reducer_vec: AsyncReducerVec<S, A>,
}

impl<S, A> Default for StoreStateMachine<S, A>
where
  S: Default + Sync + Send,
  A: Default + Sync + Send,
{
  fn default() -> StoreStateMachine<S, A> {
    StoreStateMachine {
      state: Default::default(),
      history: vec![],
      middleware_vec: Default::default(),
      middleware_spawns_vec: Default::default(),
      reducer_vec: Default::default(),
      subscriber_vec: Default::default(),
    }
  }
}

// TODO: make history implementation more comprehensive (eg: max history size) & add tests.

// Handle dispatch & history.
impl<S, A> StoreStateMachine<S, A>
where
  S: Clone + Default + PartialEq + Debug + Hash + Sync + Send,
  A: Clone + Send + Sync,
{
  pub fn get_state_clone(&self) -> S {
    self.state.clone()
  }

  pub async fn dispatch_action(
    &mut self,
    action: A,
  ) {
    // Run middlewares.
    self
      .middleware_runner(action.clone())
      .await;

    // Dispatch the action.
    self
      .actually_dispatch_action(&action.clone())
      .await;
  }

  async fn actually_dispatch_action(
    &mut self,
    action: &A,
  ) {
    self.run_reducers(action).await;
    self.run_subscribers().await;
  }

  /// Run these in parallel.
  async fn run_subscribers(&self) {
    let mut vec_fut = vec![];
    let state_clone = self.get_state_clone();
    for fun in &self.subscriber_vec.vec {
      vec_fut.push(fun.run(state_clone.clone()));
    }
    futures::future::join_all(vec_fut).await;
  }

  /// Run these in sequence.
  async fn run_reducers(
    &mut self,
    action: &A,
  ) {
    if self.reducer_vec.vec.is_empty() {
      return;
    }
    for reducer in &self.reducer_vec.vec {
      let new_state = reducer
        .run(&action, &self.state)
        .await;
      self.state = new_state;
    }
    self.update_history();
  }

  // Update history.
  fn update_history(&mut self)
  where
    S: PartialEq + Clone,
  {
    let new_state = self.get_state_clone();

    // Update history.
    let mut update_history = false;
    if self.history.is_empty() {
      update_history = true;
    } else if let Some(last_known_state) = self.history.last() {
      if *last_known_state != new_state {
        update_history = true;
      }
    }
    if update_history {
      self
        .history
        .push(new_state.clone())
    };
  }

  /// Run these in parallel.
  pub async fn middleware_runner(
    &mut self,
    action: A,
  ) {
    self
      .run_middleware_vec(action.clone())
      .await;

    self
      .run_middleware_spawns_vec(action.clone())
      .await;
  }

  /// Run concurrently (cooperatively on a single thread).
  async fn run_middleware_vec(
    &mut self,
    my_action: A,
  ) {
    let mut vec_fut = vec![];

    for item in &self.middleware_vec.vec {
      let value = item.run(
        my_action.clone(),
        self.get_state_clone(),
      );
      vec_fut.push(value);
    }

    let vec_opt_action = futures::future::join_all(vec_fut).await;

    for opt_action in vec_opt_action {
      if let Some(action) = opt_action {
        self
          .actually_dispatch_action(&action)
          .await;
      }
    }
  }

  /// Run in parallel (on multiple threads, if using Tokio's multithreaded executor).
  async fn run_middleware_spawns_vec(
    &mut self,
    my_action: A,
  ) {
    let mut vec_join_handle = vec![];

    for item in &self.middleware_spawns_vec.vec {
      let fut = item
        .run(
          my_action.clone(),
          self.get_state_clone(),
        )
        .await;
      vec_join_handle.push(fut);
    }

    let vec_results = futures::future::join_all(vec_join_handle).await;

    for join_handle in vec_results {
      let result = join_handle;
      if let Ok(result) = result {
        if let Some(action) = result {
          self
            .actually_dispatch_action(&action)
            .await;
        }
      }
    }
  }
}

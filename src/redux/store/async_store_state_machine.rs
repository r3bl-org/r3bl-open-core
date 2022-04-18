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

use crate::redux::{AsyncMiddlewareVec, AsyncReducerVec, AsyncSubscriberVec};
use core::{fmt::Debug, hash::Hash};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct StoreStateMachine<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  pub state: S,
  pub history: Vec<S>,
  pub middleware_vec: AsyncMiddlewareVec<S, A>,
  pub subscriber_vec: AsyncSubscriberVec<S>,
  pub reducer_vec: AsyncReducerVec<S, A>,
}

impl<StateT, ActionT> Default for StoreStateMachine<StateT, ActionT>
where
  StateT: Default + Sync + Send + 'static,
  ActionT: Default + Sync + Send + 'static,
{
  fn default() -> StoreStateMachine<StateT, ActionT> {
    StoreStateMachine {
      state: Default::default(),
      history: vec![],
      middleware_vec: Default::default(),
      reducer_vec: Default::default(),
      subscriber_vec: Default::default(),
    }
  }
}

// TODO: make history implementation more comprehensive (eg: max history size) & add tests.

// Handle dispatch & history.
impl<S, A> StoreStateMachine<S, A>
where
  S: Clone + Default + PartialEq + Debug + Hash + Sync + Send + 'static,
  A: Clone + Send + Sync + 'static,
{
  pub fn get_state_clone(&self) -> S {
    self.state.clone()
  }

  pub async fn dispatch_action(
    &mut self,
    action: A,
    my_ref: Arc<RwLock<StoreStateMachine<S, A>>>,
  ) {
    // Run middleware & collect resulting actions.
    let mut resulting_actions = self
      .middleware_runner(action.clone(), my_ref)
      .await;

    // Add the original action to the resulting actions.
    resulting_actions.push(action.clone());

    // Dispatch the resulting actions.
    for action in resulting_actions.iter() {
      self
        .actually_dispatch_action(action)
        .await;
    }
  }

  async fn actually_dispatch_action(
    &mut self,
    action: &A,
  ) {
    self.run_reducers(action).await;
    self.run_subscribers().await;
  }

  async fn run_subscribers(&self) {
    let state_clone = self.get_state_clone();
    for item in &self.subscriber_vec.vec {
      let fun = item.write().await;
      fun.run(state_clone.clone()).await;
    }
  }

  async fn run_reducers(
    &mut self,
    action: &A,
  ) {
    if self.reducer_vec.vec.is_empty() {
      return;
    }
    let vec_clone = &self.reducer_vec.clone();
    for item in vec_clone {
      let reducer = item.read().await;
      let new_state = reducer
        .run(&action, &self.state)
        .await;
      self.update_history(&new_state);
      self.state = new_state;
    }
  }

  // Update history.
  fn update_history(
    &mut self,
    new_state: &S,
  ) where
    S: PartialEq + Clone,
  {
    // Update history.
    let mut update_history = false;
    if self.history.is_empty() {
      update_history = true;
    } else if let Some(last_known_state) = self.history.last() {
      if *last_known_state != *new_state {
        update_history = true;
      }
    }
    if update_history {
      self
        .history
        .push(new_state.clone())
    };
  }

  /// Run middleware and return a list of resulting actions. If a middleware produces `None` that
  /// isn't added to the list that's returned.
  pub async fn middleware_runner(
    &self,
    action: A,
    my_ref: Arc<RwLock<StoreStateMachine<S, A>>>,
  ) -> Vec<A> {
    let mut return_vec = vec![];

    self
      .run_middleware_vec(
        action.clone(),
        my_ref.clone(),
        &mut return_vec,
      )
      .await;

    return return_vec;
  }

  async fn run_middleware_vec(
    &self,
    my_action: A,
    my_ref: Arc<RwLock<StoreStateMachine<S, A>>>,
    return_vec: &mut Vec<A>,
  ) {
    for item in &self.middleware_vec.vec {
      let fun = item.write().await;
      let result = fun
        .run(my_action.clone(), my_ref.clone())
        .await;
      if let Some(result) = result {
        return_vec.push(result);
      }
    }
  }
}

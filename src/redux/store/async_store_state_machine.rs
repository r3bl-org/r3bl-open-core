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
  AsyncMiddlewareVec, SafeList, SafeMiddlewareFnWrapper, SafeSubscriberFnWrapper,
  ShareableReducerFn,
};
use core::{fmt::Debug, hash::Hash};
use r3bl_rs_utils_core::SafeToShare;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct StoreStateMachine<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  pub state: S,
  pub history: Vec<S>,
  pub subscriber_fn_list: SafeList<SafeSubscriberFnWrapper<S>>,
  pub reducer_fn_list: SafeList<ShareableReducerFn<S, A>>,
  pub middleware_vec: AsyncMiddlewareVec<S, A>,
  // FIXME: deprecate middleware_fn_list
  pub middleware_fn_list: SafeList<SafeMiddlewareFnWrapper<A, Arc<RwLock<Self>>>>,
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
      subscriber_fn_list: Default::default(),
      reducer_fn_list: Default::default(),
      middleware_vec: Default::default(),
      // FIXME: deprecate middleware_fn_list
      middleware_fn_list: Default::default(),
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

  async fn run_subscribers(&mut self) {
    let state_clone = &self.get_state_clone();
    let locked_list = self.subscriber_fn_list.get_ref();
    let list_r = locked_list.read().await;
    for subscriber_fn in list_r.iter() {
      subscriber_fn
        .spawn(state_clone.clone())
        .await
        .unwrap();
    }
  }

  async fn run_reducers(
    &mut self,
    action: &A,
  ) {
    let locked_list = self.reducer_fn_list.get_ref();
    let list_r = locked_list.read().await;
    for reducer_fn in list_r.iter() {
      let new_state = reducer_fn.invoke(&self.state, &action);
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

  // FIXME: run middleware_vec
  // FIXME: deprecate middleware_fn_list
  /// Run middleware and return a list of resulting actions. If a middleware produces `None` that
  /// isn't added to the list that's returned.
  pub async fn middleware_runner(
    &mut self,
    action: A,
    my_ref: Arc<RwLock<StoreStateMachine<S, A>>>,
  ) -> Vec<A> {
    let mut return_vec = vec![];

    self
      .run_middleware_fn_list(
        action.clone(),
        my_ref.clone(),
        &mut return_vec,
      )
      .await;

    return return_vec;
  }

  // FIXME: add run_middleware_vec()

  async fn run_middleware_fn_list(
    &mut self,
    action_clone: A,
    my_ref_clone: Arc<RwLock<StoreStateMachine<S, A>>>,
    return_vec: &mut Vec<A>,
  ) {
    let locked_list = self.middleware_fn_list.get_ref();
    let list_r = locked_list.read().await;
    for item_fn in list_r.iter() {
      let result = item_fn
        .spawn(
          action_clone.clone(),
          my_ref_clone.clone(),
        )
        .await;
      match result {
        Ok(Some(action)) => {
          return_vec.push(action);
        }
        _ => (),
      };
    }
  }
}

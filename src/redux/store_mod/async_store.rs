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

use my_proc_macros_lib::make_struct_safe_to_share_and_mutate;
use std::{fmt::Debug, hash::Hash};
use tokio::task::JoinHandle;

use crate::redux::{
  async_middleware::SafeMiddlewareFnWrapper, async_subscriber::SafeSubscriberFnWrapper,
  sync_reducers::ReducerFnWrapper, StoreStateMachine,
};

make_struct_safe_to_share_and_mutate! {
  named Store<S, A>
  where S: Sync + Send + 'static + Default, A: Sync + Send + 'static
  containing my_store_state_machine
  of_type StoreStateMachine<S, A>
}

/// Thread safe and async Redux store (using [`tokio`]). This is built atop [`StoreData`] (which
/// should not be used directly).
impl<'a, S, A> Store<S, A>
where
  S: Default + Clone + PartialEq + Debug + Hash + Sync + Send + 'static,
  A: Clone + Sync + Send + 'static,
{
  pub async fn get_state(&self) -> S {
    self
      .get_value()
      .await
      .state
      .clone()
  }

  pub async fn get_history(&self) -> Vec<S> {
    self
      .get_value()
      .await
      .history
      .clone()
  }

  pub async fn dispatch_spawn(
    &self,
    action: A,
  ) -> JoinHandle<()> {
    let my_ref = self.get_ref();
    tokio::spawn(async move {
      my_ref
        .write()
        .await
        .dispatch_action(&action)
        .await;
    })
  }

  pub async fn dispatch(
    &self,
    action: &A,
  ) {
    self
      .get_ref()
      .write()
      .await
      .dispatch_action(action)
      .await;
  }

  pub async fn add_subscriber(
    &mut self,
    subscriber_fn: SafeSubscriberFnWrapper<S>,
  ) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .subscriber_manager
      .push(subscriber_fn)
      .await;
    self
  }

  pub async fn clear_subscribers(&mut self) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .subscriber_manager
      .clear()
      .await;
    self
  }

  pub async fn add_middleware(
    &mut self,
    middleware_fn: SafeMiddlewareFnWrapper<A>,
  ) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .middleware_manager
      .push(middleware_fn)
      .await;
    self
  }

  pub async fn clear_middlewares(&mut self) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .middleware_manager
      .clear()
      .await;
    self
  }

  pub async fn add_reducer(
    &mut self,
    reducer_fn: ReducerFnWrapper<S, A>,
  ) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .reducer_manager
      .push(reducer_fn)
      .await;
    self
  }
}

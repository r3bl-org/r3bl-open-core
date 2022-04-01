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

//! Thread safe and async Redux store (using [`tokio`]). This is built atop [`StoreData`] (which
//! should not be used directly).

use my_proc_macros_lib::make_struct_safe_to_share_and_mutate;
use std::{fmt::Debug, hash::Hash};
use tokio::task::JoinHandle;

use crate::redux::{
  async_middleware::SafeMiddlewareFnWrapper, async_subscriber::SafeSubscriberFnWrapper,
  sync_reducers::ReducerFnWrapper, MiddlewareManager, ReducerManager, StoreStateMachine,
  SubscriberManager,
};

make_struct_safe_to_share_and_mutate! {
  named Store<S, A>
  where S: Sync + Send + 'static + Default, A: Sync + Send + 'static
  containing my_store_state_machine
  of_type StoreStateMachine<S, A>
}

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
      Store::with_ref_get_value_w_lock(&my_ref)
        .await
        .dispatch_action(&action)
        .await;
    })
  }

  pub async fn dispatch(
    &self,
    action: &A,
  ) {
    Store::with_ref_get_value_w_lock(&self.get_ref())
      .await
      .dispatch_action(action)
      .await;
  }

  pub async fn add_subscriber(
    &mut self,
    subscriber_fn: SafeSubscriberFnWrapper<S>,
  ) -> &mut Store<S, A> {
    Store::with_ref_get_value_w_lock(&self.get_ref())
      .await
      .subscriber_manager
      .push(subscriber_fn)
      .await;
    self
  }

  // TODO: 🎗️ modify below ⬇
  pub async fn clear_subscribers(&mut self) -> &mut Store<S, A> {
    with_subscriber_manager_w!(
      self,
      |it: &'a mut SubscriberManager<S>| async {
        it.clear().await;
      }
    );
    self
  }

  pub async fn add_middleware(
    &mut self,
    middleware_fn: SafeMiddlewareFnWrapper<A>,
  ) -> &mut Store<S, A> {
    with_middleware_manager_w!(
      self,
      |it: &'a mut MiddlewareManager<A>| async {
        it.push(middleware_fn).await;
      }
    );
    self
  }

  pub async fn clear_middlewares(&mut self) -> &mut Store<S, A> {
    with_middleware_manager_w!(
      self,
      |it: &'a mut MiddlewareManager<A>| async {
        it.clear().await;
      }
    );
    self
  }

  pub async fn add_reducer(
    &mut self,
    reducer_fn: ReducerFnWrapper<S, A>,
  ) -> &mut Store<S, A> {
    with_reducer_manager_w!(
      self,
      |it: &'a mut ReducerManager<S, A>| async {
        it.push(reducer_fn).await;
      }
    );
    self
  }
}

// Macros.

macro_rules! with_subscriber_manager_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get_ref();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w.subscriber_manager).await;
  };
}

macro_rules! with_middleware_manager_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get_ref();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w.middleware_manager).await;
  };
}

macro_rules! with_reducer_manager_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get_ref();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w.reducer_manager).await;
  };
}

macro_rules! with_self_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get_ref();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w).await;
  };
}

pub(crate) use with_middleware_manager_w;
pub(crate) use with_reducer_manager_w;
pub(crate) use with_self_w;
pub(crate) use with_subscriber_manager_w;

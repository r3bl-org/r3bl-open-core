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

use std::{fmt::Debug, hash::Hash, sync::Arc};
use tokio::{task::JoinHandle, sync::RwLock};

use crate::redux::{
  StoreStateMachine, async_subscriber::SafeSubscriberFnWrapper,
  async_middleware::SafeMiddlewareFnWrapper, sync_reducers::ReducerFnWrapper,
  ReducerManager, MiddlewareManager, SubscriberManager,
};

/// Thread safe and async Redux store (using [`tokio`]). This is built atop [`StoreData`] (which
/// should not be used directly).
pub struct Store<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  store_state_machine_arc: SafeStoreStateMachineWrapper<S, A>,
}

pub type SafeStoreStateMachineWrapper<S, A> = Arc<RwLock<StoreStateMachine<S, A>>>;

impl<S, A> Default for Store<S, A>
where
  S: Sync + Send + 'static + Default,
  A: Sync + Send + 'static,
{
  fn default() -> Self {
    Self {
      store_state_machine_arc: Arc::new(RwLock::new(Default::default())),
    }
  }
}

impl<'a, S, A> Store<S, A>
where
  S: Default + Clone + PartialEq + Debug + Hash + Sync + Send + 'static,
  A: Clone + Sync + Send + 'static,
{
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self) -> SafeStoreStateMachineWrapper<S, A> {
    self.store_state_machine_arc.clone()
  }

  pub async fn get_state(&self) -> S {
    self.get().read().await.state.clone()
  }

  pub async fn get_history(&self) -> Vec<S> {
    self.get().read().await.history.clone()
  }

  pub async fn dispatch_spawn(
    &self,
    action: A,
  ) -> JoinHandle<()> {
    let action_clone = action.clone();
    let self_arc_clone = self.get();
    tokio::spawn(async move {
      let mut state_manager = self_arc_clone.write().await;
      state_manager.dispatch_action(&action_clone).await;
    })
  }

  pub async fn dispatch(
    &self,
    action: &A,
  ) {
    with_self_w!(
      self,
      |state_manager: &'a mut StoreStateMachine<S, A>| async {
        state_manager.dispatch_action(action).await;
      }
    );
  }

  pub async fn add_subscriber(
    &mut self,
    subscriber_fn: SafeSubscriberFnWrapper<S>,
  ) -> &mut Store<S, A> {
    with_subscriber_manager_w!(self, |it: &'a mut SubscriberManager<S>| async {
      it.push(subscriber_fn).await;
    });
    self
  }

  pub async fn clear_subscribers(&mut self) -> &mut Store<S, A> {
    with_subscriber_manager_w!(self, |it: &'a mut SubscriberManager<S>| async {
      it.clear().await;
    });
    self
  }

  pub async fn add_middleware(
    &mut self,
    middleware_fn: SafeMiddlewareFnWrapper<A>,
  ) -> &mut Store<S, A> {
    with_middleware_manager_w!(self, |it: &'a mut MiddlewareManager<A>| async {
      it.push(middleware_fn).await;
    });
    self
  }

  pub async fn clear_middleware(&mut self) -> &mut Store<S, A> {
    with_middleware_manager_w!(self, |it: &'a mut MiddlewareManager<A>| async {
      it.clear().await;
    });
    self
  }

  pub async fn add_reducer(
    &mut self,
    reducer_fn: ReducerFnWrapper<S, A>,
  ) -> &mut Store<S, A> {
    with_reducer_manager_w!(self, |it: &'a mut ReducerManager<S, A>| async {
      it.push(reducer_fn).await;
    });
    self
  }
}

// Macros.

macro_rules! with_subscriber_manager_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w.subscriber_manager).await;
  };
}

macro_rules! with_middleware_manager_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w.middleware_manager).await;
  };
}

macro_rules! with_reducer_manager_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w.reducer_manager).await;
  };
}

macro_rules! with_self_w {
  ($this:ident, $lambda:expr) => {
    let arc = $this.get();
    let mut my_state_machine_w = arc.write().await;
    $lambda(&mut my_state_machine_w).await;
  };
}

pub(crate) use with_self_w;
pub(crate) use with_reducer_manager_w;
pub(crate) use with_middleware_manager_w;
pub(crate) use with_subscriber_manager_w;

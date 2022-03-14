use std::{fmt::Debug, hash::Hash, sync::Arc};
use tokio::{task::JoinHandle, sync::RwLock};

use crate::redux::{
  StoreStateMachine, async_subscriber::SafeSubscriberFnWrapper,
  async_middleware::SafeMiddlewareFnWrapper, sync_reducers::ReducerFnWrapper,
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

impl<S, A> Store<S, A>
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
    let arc_clone = self.get();
    tokio::spawn(async move {
      let mut state_manager = arc_clone.write().await;
      state_manager.dispatch_action(&action_clone).await;
    })
  }

  pub async fn dispatch(
    &self,
    action: &A,
  ) {
    self
      .get()
      .write()
      .await
      .dispatch_action(&action.clone())
      .await;
  }

  pub async fn add_subscriber(
    &mut self,
    subscriber_fn: SafeSubscriberFnWrapper<S>,
  ) -> &mut Store<S, A> {
    let my_state_manager_arc = self.get();
    let mut my_state_manager = my_state_manager_arc.write().await;
    my_state_manager
      .subscriber_manager
      .push(subscriber_fn)
      .await;
    self
  }

  pub async fn clear_subscribers(&mut self) -> &mut Store<S, A> {
    let my_state_manager_arc = self.get();
    let mut my_state_manager = my_state_manager_arc.write().await;
    my_state_manager.subscriber_manager.clear().await;
    self
  }

  pub async fn add_middleware(
    &mut self,
    middleware_fn: SafeMiddlewareFnWrapper<A>,
  ) -> &mut Store<S, A> {
    let my_state_manager_arc = self.get();
    let mut my_state_manager = my_state_manager_arc.write().await;
    my_state_manager
      .middleware_manager
      .push(middleware_fn)
      .await;
    self
  }

  pub async fn clear_middleware(&mut self) -> &mut Store<S, A> {
    let my_state_manager_arc = self.get();
    let mut my_state_manager = my_state_manager_arc.write().await;
    my_state_manager.middleware_manager.clear().await;
    self
  }

  pub async fn add_reducer(
    &mut self,
    reducer_fn: ReducerFnWrapper<S, A>,
  ) -> &mut Store<S, A> {
    let my_state_manager_arc = self.get();
    let mut my_state_manager = my_state_manager_arc.write().await;
    my_state_manager.reducer_manager.push(reducer_fn).await;
    self
  }
}

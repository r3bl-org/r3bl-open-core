use std::{
  fmt::Debug,
  hash::Hash,
  sync::{Arc, RwLock},
};
use super::{
  StateManager, async_subscribers::SafeSubscriberFnWrapper,
  async_middleware::SafeMiddlewareFnWrapper, sync_reducers::ReducerFnWrapper,
  ListManager,
};

/// Thread safe and async Redux store (using [`tokio`]). This is built atop [`StoreData`] (which
/// should not be used directly).
pub struct Store<S, A> {
  store_arc: SafeStateManager<S>,
  subscriber_manager: ListManager<SafeSubscriberFnWrapper<S>>,
  middleware_manager: ListManager<SafeMiddlewareFnWrapper<A>>,
  reducer_manager: ListManager<ReducerFnWrapper<S, A>>,
}

pub type SafeStateManager<S> = Arc<RwLock<StateManager<S>>>;

impl<S, A> Store<S, A>
where
  S: Default + Clone + PartialEq + Debug + Hash + Sync + Send + 'static,
  A: Clone + Sync + Send + 'static,
{
  pub fn new() -> Store<S, A> {
    Store {
      store_arc: Arc::new(RwLock::new(Default::default())),
      reducer_manager: ListManager::new(),
      subscriber_manager: ListManager::new(),
      middleware_manager: ListManager::new(),
    }
  }

  pub fn get(&self) -> SafeStateManager<S> {
    self.store_arc.clone()
  }

  pub fn get_state(&self) -> S {
    self.get().read().unwrap().state.clone()
  }

  pub async fn dispatch(
    &self,
    action: &A,
  ) {
    self
      .get()
      .write()
      .unwrap()
      .dispatch_action(
        &action.clone(),
        &self.reducer_manager,
        &self.subscriber_manager,
        &self.middleware_manager,
      )
      .await;
  }

  pub fn add_subscriber(
    &mut self,
    subscriber_fn: SafeSubscriberFnWrapper<S>,
  ) -> &mut Store<S, A> {
    self.subscriber_manager.push(subscriber_fn);
    self
  }

  pub fn clear_subscribers(&mut self) -> &mut Store<S, A> {
    self.subscriber_manager.clear();
    self
  }

  pub fn add_middleware(
    &mut self,
    middleware_fn: SafeMiddlewareFnWrapper<A>,
  ) -> &mut Store<S, A> {
    self.middleware_manager.push(middleware_fn);
    self
  }

  pub fn clear_middleware(&mut self) -> &mut Store<S, A> {
    self.middleware_manager.clear();
    self
  }

  pub fn add_reducer(
    &mut self,
    reducer_fn: ReducerFnWrapper<S, A>,
  ) -> &mut Store<S, A> {
    self.reducer_manager.push(reducer_fn);
    self
  }
}

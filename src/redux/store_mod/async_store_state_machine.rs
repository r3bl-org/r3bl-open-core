use core::{hash::Hash, fmt::Debug};

use crate::redux::{
  SafeListManager, SafeSubscriberFnWrapper, SafeMiddlewareFnWrapper, ReducerFnWrapper,
};

pub struct StoreStateMachine<S, A>
where
  S: Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  pub state: S,
  pub history: Vec<S>,
  pub subscriber_manager: SafeListManager<SafeSubscriberFnWrapper<S>>,
  pub middleware_manager: SafeListManager<SafeMiddlewareFnWrapper<A>>,
  pub reducer_manager: SafeListManager<ReducerFnWrapper<S, A>>,
}

impl<S, A> Default for StoreStateMachine<S, A>
where
  S: Default + Sync + Send + 'static,
  A: Sync + Send + 'static,
{
  fn default() -> StoreStateMachine<S, A> {
    StoreStateMachine {
      state: Default::default(),
      history: vec![],
      subscriber_manager: Default::default(),
      middleware_manager: Default::default(),
      reducer_manager: Default::default(),
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
  pub async fn dispatch_action(
    &mut self,
    action: &A,
  ) {
    // Run middleware & collect resulting actions.
    let mut resulting_actions = self.middleware_runner(action).await;

    // Add the original action to the resulting actions.
    resulting_actions.push(action.clone());

    // Dispatch the resulting actions.
    for action in resulting_actions.iter() {
      self.actually_dispatch_action(action).await;
    }
  }

  pub async fn actually_dispatch_action(
    &mut self,
    action: &A,
  ) {
    // Run reducers.
    {
      let locked_list = self.reducer_manager.get();
      let list = locked_list.write().await;
      list.iter().for_each(|reducer_fn| {
        let new_state = reducer_fn.invoke(&self.state, &action);
        update_history(&mut self.history, &new_state);
        self.state = new_state;
      });
    }

    // Run subscribers.
    {
      let locked_list = self.subscriber_manager.get();
      let list = locked_list.write().await;
      for subscriber_fn in list.iter() {
        subscriber_fn.spawn(self.state.clone()).await.unwrap();
      }
    }

    // Update history.
    fn update_history<S>(
      history: &mut Vec<S>,
      new_state: &S,
    ) where
      S: PartialEq + Clone,
    {
      // Update history.
      let mut update_history = false;
      if history.is_empty() {
        update_history = true;
      } else if let Some(last_known_state) = history.last() {
        if *last_known_state != *new_state {
          update_history = true;
        }
      }
      if update_history {
        history.push(new_state.clone())
      };
    }
  }

  /// Run middleware and return a list of resulting actions. If a middleware produces `None` that
  /// isn't added to the list that's returned.
  pub async fn middleware_runner(
    &mut self,
    action: &A,
  ) -> Vec<A> {
    let mut results: Vec<A> = vec![];
    let locked_list = self.middleware_manager.get();
    let list = locked_list.write().await;
    for middleware_fn in list.iter() {
      let result = middleware_fn.spawn(action.clone()).await;
      if let Ok(option) = result {
        if let Some(action) = option {
          results.push(action);
        }
      }
    }
    results
  }
}

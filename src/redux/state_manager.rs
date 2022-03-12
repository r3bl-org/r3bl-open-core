use core::{hash::Hash, fmt::Debug};
use super::{ListManager, ReducerFnWrapper, SafeSubscriberFnWrapper, SafeMiddlewareFnWrapper};

/// Redux store. Do not use this directly, please use [`Store`] instead.
pub struct StateManager<S> {
  pub state: S,
  pub history: Vec<S>,
}

/// Default impl of Redux store.
impl<S> Default for StateManager<S>
where
  S: Default,
{
  fn default() -> StateManager<S> {
    StateManager {
      state: Default::default(),
      history: vec![],
    }
  }
}

// TODO: make history implementation more comprehensive (eg: max history size) & add tests.

// Handle dispatch & history.
impl<S> StateManager<S>
where
  S: Clone + Default + PartialEq + Debug + Hash + Sync + Send + 'static,
{
  pub async fn dispatch_action<A>(
    &mut self,
    action: &A,
    reducer_manager: &ListManager<ReducerFnWrapper<S, A>>,
    subscriber_manager: &ListManager<SafeSubscriberFnWrapper<S>>,
    middleware_manager: &ListManager<SafeMiddlewareFnWrapper<A>>,
  ) where
    A: Clone + Send + Sync + 'static,
  {
    // Run middleware & collect resulting actions.
    let mut resulting_actions = self.middleware_runner(action, middleware_manager).await;

    // Add the original action to the resulting actions.
    resulting_actions.push(action.clone());

    // Dispatch the resulting actions.
    for action in resulting_actions.iter() {
      self
        .actually_dispatch_action(action, reducer_manager, subscriber_manager)
        .await;
    }
  }

  async fn actually_dispatch_action<A>(
    &mut self,
    action: &A,
    reducer_manager: &ListManager<ReducerFnWrapper<S, A>>,
    subscriber_manager: &ListManager<SafeSubscriberFnWrapper<S>>,
  ) where
    A: Clone + Send + Sync + 'static,
  {
    // Run reducers.
    reducer_manager.iter().for_each(|reducer_fn| {
      let reducer_fn = reducer_fn.get();
      let new_state = reducer_fn(&self.state, &action);
      update_history(&mut self.history, &new_state);
      self.state = new_state;
    });

    // Run subscribers.
    for subscriber_fn in subscriber_manager.iter() {
      subscriber_fn.spawn(self.state.clone()).await.unwrap();
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
  async fn middleware_runner<A>(
    &mut self,
    action: &A,
    middleware_manager: &ListManager<SafeMiddlewareFnWrapper<A>>,
  ) -> Vec<A>
  where
    A: Clone + Send + Sync + 'static,
  {
    let mut results: Vec<A> = vec![];
    for middleware_fn in middleware_manager.iter() {
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

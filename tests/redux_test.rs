// Integration tests
// Functions that are in scope: <https://stackoverflow.com/a/45151641/2085356>
// About integration tests: <https://doc.rust-lang.org/book/ch11-03-test-organization.html#the-tests-directory>
// Tokio test macro: <https://docs.rs/tokio/latest/tokio/attr.test.html>

// Imports.
use r3bl_rs_utils::redux::{
  ReducerFnWrapper, SafeMiddlewareFnWrapper, SafeSubscriberFnWrapper, Store,
};
use r3bl_rs_utils::utils::with;
use std::sync::{Arc, Mutex};

/// Action enum.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Action {
  Add(i32, i32),
  AddPop(i32),
  Clear,
  MiddlewareCreateClearAction,
}

/// State.
#[derive(Clone, Default, PartialEq, Debug, Hash)]
pub struct State {
  pub stack: Vec<i32>,
}

// TODO: Write integration tests for history.

#[tokio::test]
async fn test_redux_store_works_for_main_use_cases() {
  // Reducer function (pure).
  let reducer_fn = |state: &State, action: &Action| match action {
    Action::Add(a, b) => {
      let sum = a + b;
      State { stack: vec![sum] }
    }
    Action::AddPop(a) => {
      let sum = a + state.stack[0];
      State { stack: vec![sum] }
    }
    Action::Clear => State { stack: vec![] },
    _ => state.clone(),
  };

  // This shared object is used to collect results from the subscriber function & test it later.
  let shared_object = Arc::new(Mutex::new(Vec::<i32>::new()));
  // This subscriber function is curried to capture a reference to the shared object.
  let subscriber_fn = with(shared_object.clone(), |it| {
    let curried_fn = move |state: State| {
      let mut stack = it.lock().unwrap();
      stack.push(state.stack[0]);
    };
    curried_fn
  });

  // This middleware function is curried to capture a reference to the shared object.
  let mw_returns_none = with(shared_object.clone(), |it| {
    let curried_fn = move |action: Action| {
      let mut stack = it.lock().unwrap();
      match action {
        Action::Add(_, _) => stack.push(-1),
        Action::AddPop(_) => stack.push(-2),
        Action::Clear => stack.push(-3),
        _ => {}
      }
      None
    };
    curried_fn
  });

  // This middleware function is curried to capture a reference to the shared object.
  let mw_returns_action = with(shared_object.clone(), |it| {
    let curried_fn = move |action: Action| {
      let mut stack = it.lock().unwrap();
      match action {
        Action::MiddlewareCreateClearAction => stack.push(-4),
        _ => {}
      }
      Some(Action::Clear)
    };
    curried_fn
  });

  // Setup store.
  let mut store = Store::<State, Action>::new();
  store
    .add_reducer(ReducerFnWrapper::new(reducer_fn))
    .add_subscriber(SafeSubscriberFnWrapper::new(subscriber_fn))
    .add_middleware(SafeMiddlewareFnWrapper::new(mw_returns_none));

  // Test reducer and subscriber by dispatching Add and AddPop actions asynchronously.
  store.dispatch(&Action::Add(1, 2)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(3));
  store.dispatch(&Action::AddPop(1)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(4));
  store.clear_subscribers();

  // Test async middleware: mw_returns_none.
  store.dispatch(&Action::Add(1, 2)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-1));
  store.dispatch(&Action::AddPop(1)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-2));
  store.dispatch(&Action::Clear).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-3));
  store.clear_middleware();

  // Test async middleware: mw_returns_action.
  shared_object.lock().unwrap().clear();
  store
    .add_middleware(SafeMiddlewareFnWrapper::new(mw_returns_action))
    .dispatch(&Action::MiddlewareCreateClearAction)
    .await;
  assert_eq!(store.get_state().stack.len(), 0);
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-4));
}

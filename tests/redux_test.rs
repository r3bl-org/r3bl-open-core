/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

// Imports.
use r3bl_rs_utils::utils::with;
use std::sync::{Arc, Mutex};
use r3bl_rs_utils::redux::{
  Store, ReducerFnWrapper, SafeSubscriberFnWrapper, SafeMiddlewareFnWrapper,
};

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
  let mut store = Store::<State, Action>::default();
  store
    .add_reducer(ReducerFnWrapper::from(reducer_fn))
    .await
    .add_subscriber(SafeSubscriberFnWrapper::from(subscriber_fn))
    .await
    .add_middleware(SafeMiddlewareFnWrapper::from(mw_returns_none))
    .await;

  // Test reducer and subscriber by dispatching Add and AddPop actions sync & async.
  store.dispatch_spawn(Action::Add(10, 10)).await;
  store.dispatch(&Action::Add(1, 2)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(3));
  store.dispatch(&Action::AddPop(1)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(21));
  store.clear_subscribers().await;

  // Test async middleware: mw_returns_none.
  store.dispatch(&Action::Add(1, 2)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-1));
  store.dispatch(&Action::AddPop(1)).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-2));
  store.dispatch(&Action::Clear).await;
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-3));
  store.clear_middlewares().await;

  // Test async middleware: mw_returns_action.
  shared_object.lock().unwrap().clear();
  store
    .add_middleware(SafeMiddlewareFnWrapper::from(mw_returns_action))
    .await
    .dispatch(&Action::MiddlewareCreateClearAction)
    .await;
  assert_eq!(store.get_state().await.stack.len(), 0);
  assert_eq!(shared_object.lock().unwrap().pop(), Some(-4));
}

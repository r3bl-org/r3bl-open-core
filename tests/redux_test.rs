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
use async_trait::async_trait;
use r3bl_rs_utils::redux::{
  AsyncMiddleware, AsyncSubscriber, ShareableReducerFn, Store, StoreStateMachine,
};

use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

/// Action enum.
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Action {
  // Reducer actions.
  Add(i32, i32),
  AddPop(i32),
  Clear,
  // Middleware actions for AsyncMwReturnsNone.
  AsyncMwReturnsNone_Add(i32, i32),
  AsyncMwReturnsNone_AddPop(i32),
  AsyncMwReturnsNone_Clear,
  // Middleware actions for AsyncMwReturnsAction.
  AsyncMwReturnsAction_SetState,
  // For Default impl.
  Noop,
}

impl Default for Action {
  fn default() -> Self {
    Action::Noop
  }
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

  // This shared object is used to collect results from the subscriber & middleware &
  // reducer functions & test it later.
  let shared_object_ref = Arc::new(Mutex::new(Vec::<i32>::new()));
  let subscriber_fn2 = MyAsyncSubscriber {
    shared_object_ref: shared_object_ref.clone(),
  };
  let my_async_mw_returns_none = AsyncMwReturnsNone {
    shared_object_ref: shared_object_ref.clone(),
  };
  let my_async_mw_returns_action = AsyncMwReturnsAction {
    shared_object_ref: shared_object_ref.clone(),
  };

  // Setup store w/ only reducer & subscriber (no middlewares).
  let mut store = Store::<State, Action>::default();
  store
    .add_reducer(ShareableReducerFn::from(
      reducer_fn,
    ))
    .await
    .add_subscriber(Arc::new(RwLock::new(
      subscriber_fn2,
    )))
    .await;

  // Test reducer and subscriber by dispatching Add and AddPop actions sync & async w/ no
  // middlewares.
  store
    .dispatch_spawn(Action::Add(10, 10))
    .await;
  store
    .dispatch(Action::Add(1, 2))
    .await;
  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(3)
  );
  store
    .dispatch(Action::AddPop(1))
    .await;
  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(4)
  );
  store.clear_subscribers().await;

  // Test async middleware: my_async_mw_returns_none.
  store
    .add_middleware(Arc::new(RwLock::new(
      my_async_mw_returns_none,
    )))
    .await
    .dispatch(Action::AsyncMwReturnsNone_Add(
      1, 2,
    ))
    .await;
  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-1)
  );
  store
    .dispatch(Action::AsyncMwReturnsNone_AddPop(
      1,
    ))
    .await;
  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-2)
  );
  store
    .dispatch(Action::AsyncMwReturnsNone_Clear)
    .await;
  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-3)
  );

  // Test async middleware: my_async_mw_returns_action.
  store.clear_middlewares().await;
  shared_object_ref
    .lock()
    .unwrap()
    .clear();

  store
    .add_middleware(Arc::new(RwLock::new(
      my_async_mw_returns_action,
    )))
    .await
    .dispatch(Action::AsyncMwReturnsAction_SetState)
    .await;
  assert_eq!(
    store.get_state().await.stack.len(),
    0
  );
  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-4)
  );
}

struct AsyncMwReturnsNone {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for AsyncMwReturnsNone {
  async fn run(
    &self,
    action: Action,
    _store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
  ) -> Option<Action> {
    let mut stack = self
      .shared_object_ref
      .lock()
      .unwrap();
    match action {
      Action::AsyncMwReturnsNone_Add(_, _) => stack.push(-1),
      Action::AsyncMwReturnsNone_AddPop(_) => stack.push(-2),
      Action::AsyncMwReturnsNone_Clear => stack.push(-3),
      _ => {}
    }
    None
  }
}

struct AsyncMwReturnsAction {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for AsyncMwReturnsAction {
  async fn run(
    &self,
    action: Action,
    _store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
  ) -> Option<Action> {
    let mut stack = self
      .shared_object_ref
      .lock()
      .unwrap();
    match action {
      Action::AsyncMwReturnsAction_SetState => stack.push(-4),
      _ => {}
    }
    Some(Action::Clear)
  }
}

struct MyAsyncSubscriber {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncSubscriber<State> for MyAsyncSubscriber {
  async fn run(
    &self,
    state: State,
  ) {
    let mut stack = self
      .shared_object_ref
      .lock()
      .unwrap();
    if !state.stack.is_empty() {
      stack.push(state.stack[0]);
    }
  }
}

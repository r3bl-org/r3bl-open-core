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

use async_trait::async_trait;
use r3bl_rs_utils::redux::{
  AsyncMiddleware, AsyncReducer, AsyncSubscriber, Store, StoreStateMachine,
};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

/// ╭──────────────────────────────────────────────────────╮
/// │ Action enum.                                         │
/// ╰──────────────────────────────────────────────────────╯
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Action {
  // Reducer actions.
  Add(i32, i32),
  AddPop(i32),
  Clear,
  // Middleware actions for AsyncMwReturnsNone.
  MwReturnsNone_Add(i32, i32),
  MwReturnsNone_AddPop(i32),
  MwReturnsNone_Clear,
  // Middleware actions for AsyncMwReturnsAction.
  MwReturnsAction_SetState,
  // For Default impl.
  Noop,
}

impl Default for Action {
  fn default() -> Self {
    Action::Noop
  }
}

/// ╭──────────────────────────────────────────────────────╮
/// │ State struct.                                        │
/// ╰──────────────────────────────────────────────────────╯
#[derive(Clone, Default, PartialEq, Debug, Hash)]
pub struct State {
  pub stack: Vec<i32>,
}

// TODO: Write integration tests for history.

/// ╭──────────────────────────────────────────────────────╮
/// │ Main test runner.                                    │
/// ╰──────────────────────────────────────────────────────╯
#[tokio::test]
async fn test_redux_store_works_for_main_use_cases() {
  // This shared object is used to collect results from the subscriber & middleware &
  // reducer functions & test it later.
  let shared_object_ref = Arc::new(Mutex::new(Vec::<i32>::new()));

  // Create the store.
  let mut store = Store::<State, Action>::default();

  test_reducer_and_subscriber(&shared_object_ref, &mut store).await;
  test_mw_returns_none(&shared_object_ref, &mut store).await;
  test_mw_returns_action(&shared_object_ref, &mut store).await;
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test helpers: Reset shared object.                   │
/// ╰──────────────────────────────────────────────────────╯
fn reset_shared_object(shared_object_ref: &Arc<Mutex<Vec<i32>>>) {
  shared_object_ref
    .lock()
    .unwrap()
    .clear();
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test helpers: Reset store.                           │
/// ╰──────────────────────────────────────────────────────╯
async fn reset_store(store: &mut Store<State, Action>) -> &mut Store<State, Action> {
  store
    .clear_reducers()
    .await
    .clear_subscribers()
    .await
    .clear_middlewares()
    .await;
  store
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test async subscriber: [MySubscriber].               │
/// ╰──────────────────────────────────────────────────────╯
async fn test_reducer_and_subscriber(
  shared_object_ref: &Arc<Mutex<Vec<i32>>>,
  store: &mut Store<State, Action>,
) {
  let my_subscriber = MySubscriber {
    shared_object_ref: shared_object_ref.clone(),
  };

  reset_shared_object(shared_object_ref);

  // Setup store w/ only reducer & subscriber (no middlewares).
  reset_store(store)
    .await
    .add_reducer(MyReducer::new())
    .await
    .add_subscriber(Arc::new(RwLock::new(
      my_subscriber,
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
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test async middleware: [MwReturnsNone].              │
/// ╰──────────────────────────────────────────────────────╯
async fn test_mw_returns_none(
  shared_object_ref: &Arc<Mutex<Vec<i32>>>,
  store: &mut Store<State, Action>,
) {
  let mw_returns_none = MwReturnsNone {
    shared_object_ref: shared_object_ref.clone(),
  };

  reset_shared_object(shared_object_ref);

  // Reconfigure store.
  reset_store(store)
    .await
    .add_middleware(Arc::new(RwLock::new(
      mw_returns_none,
    )))
    .await
    .dispatch(Action::MwReturnsNone_Add(1, 2))
    .await;

  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-1)
  );

  store
    .dispatch(Action::MwReturnsNone_AddPop(1))
    .await;

  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-2)
  );

  store
    .dispatch(Action::MwReturnsNone_Clear)
    .await;

  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-3)
  );
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test async middleware: [MwReturnsAction].            │
/// ╰──────────────────────────────────────────────────────╯
async fn test_mw_returns_action(
  shared_object_ref: &Arc<Mutex<Vec<i32>>>,
  store: &mut Store<State, Action>,
) {
  let mw_returns_action = MwReturnsAction {
    shared_object_ref: shared_object_ref.clone(),
  };

  reset_shared_object(shared_object_ref);

  // Since the reducers are removed, the `Action::Clear` returned by the following
  // middleware will be ignored.
  reset_store(store)
    .await
    .add_middleware(Arc::new(RwLock::new(
      mw_returns_action,
    )))
    .await
    .dispatch(Action::MwReturnsAction_SetState)
    .await;

  assert_eq!(
    store.get_state().await.stack.len(),
    1
  );

  assert_eq!(
    shared_object_ref
      .lock()
      .unwrap()
      .pop(),
    Some(-4)
  );
}

/// ╭──────────────────────────────────────────────────────╮
/// │ MwReturnsNone.                                       │
/// ╰──────────────────────────────────────────────────────╯
struct MwReturnsNone {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for MwReturnsNone {
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
      Action::MwReturnsNone_Add(_, _) => stack.push(-1),
      Action::MwReturnsNone_AddPop(_) => stack.push(-2),
      Action::MwReturnsNone_Clear => stack.push(-3),
      _ => {}
    }
    None
  }
}

/// ╭──────────────────────────────────────────────────────╮
/// │ MwReturnsAction.                                     │
/// ╰──────────────────────────────────────────────────────╯
struct MwReturnsAction {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for MwReturnsAction {
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
      Action::MwReturnsAction_SetState => stack.push(-4),
      _ => {}
    }
    Some(Action::Clear)
  }
}

/// ╭──────────────────────────────────────────────────────╮
/// │ MySubscriber.                                        │
/// ╰──────────────────────────────────────────────────────╯
struct MySubscriber {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncSubscriber<State> for MySubscriber {
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

/// ╭──────────────────────────────────────────────────────╮
/// │ MyReducer.                                           │
/// ╰──────────────────────────────────────────────────────╯
#[derive(Default)]
struct MyReducer;

#[async_trait]
impl AsyncReducer<State, Action> for MyReducer {
  async fn run(
    &self,
    action: &Action,
    state: &State,
  ) -> State {
    match action {
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
    }
  }
}

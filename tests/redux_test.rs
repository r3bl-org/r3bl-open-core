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
use r3bl_rs_utils::{
  fire_and_forget,
  redux::{AsyncMiddleware, AsyncReducer, AsyncSubscriber, Store, StoreStateMachine},
};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// ╭──────────────────────────────────────────────────────╮
/// │ Action enum.                                         │
/// ╰──────────────────────────────────────────────────────╯
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Action {
  // Reducer actions.
  Add(i32, i32),
  AddPop(i32),
  Reset,
  Clear,
  // Middleware actions for MwExampleNoSpawn.
  MwExampleNoSpawn_Foo(i32, i32),
  MwExampleNoSpawn_Bar(i32),
  MwExampleNoSpawn_Baz,
  // Middleware actions for MwExampleSpawns.
  MwExampleSpawns_ModifySharedObject_ResetState,
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
  let shared_vec = Arc::new(Mutex::new(Vec::<i32>::new()));

  // Create the store.
  let mut store = Store::<State, Action>::default();

  run_reducer_and_subscriber(&shared_vec, &mut store).await;
  run_mw_example_no_spawn(&shared_vec, &mut store).await;
  run_mw_example_spawns(&shared_vec, &mut store).await;
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test helpers: Reset shared object.                   │
/// ╰──────────────────────────────────────────────────────╯
async fn reset_shared_object(shared_vec: &Arc<Mutex<Vec<i32>>>) {
  shared_vec.lock().await.clear();
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
/// │ Test helpers: 1ms delay                              │
/// ╰──────────────────────────────────────────────────────╯
async fn delay_for_spawned_mw_to_execute() {
  tokio::time::sleep(tokio::time::Duration::from_millis(
    1,
  ))
  .await;
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test async subscriber: [MySubscriber].               │
/// ╰──────────────────────────────────────────────────────╯
/// 1. Test reducer and subscriber by dispatching `Add` and `AddPop` actions
/// 2. No middlewares.
async fn run_reducer_and_subscriber(
  shared_vec: &Arc<Mutex<Vec<i32>>>,
  store: &mut Store<State, Action>,
) {
  // Setup store w/ only reducer & subscriber (no middlewares).
  let my_subscriber = MySubscriber {
    shared_vec: shared_vec.clone(),
  };
  reset_shared_object(shared_vec).await;
  reset_store(store)
    .await
    .add_reducer(MyReducer::new())
    .await
    .add_subscriber(Arc::new(RwLock::new(
      my_subscriber,
    )))
    .await;

  store.dispatch_spawn(Action::Add(1, 2));
  delay_for_spawned_mw_to_execute().await;

  assert_eq!(
    shared_vec.lock().await.pop(),
    Some(3)
  );

  store.dispatch_spawn(Action::AddPop(1));
  delay_for_spawned_mw_to_execute().await;

  assert_eq!(
    shared_vec.lock().await.pop(),
    Some(4)
  );

  // Clean up the store's state.
  store.dispatch_spawn(Action::Clear);
  delay_for_spawned_mw_to_execute().await;

  assert_eq!(
    store.get_state().await.stack.len(),
    0
  );
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test async middleware: [MwExampleNoSpawn].           │
/// ╰──────────────────────────────────────────────────────╯
/// 1. Does not involve any reducers or subscribers.
/// 2. Just this middleware which modifies the `shared_vec`.
async fn run_mw_example_no_spawn(
  shared_vec: &Arc<Mutex<Vec<i32>>>,
  store: &mut Store<State, Action>,
) {
  let mw_returns_none = MwExampleNoSpawn {
    shared_vec: shared_vec.clone(),
  };

  reset_shared_object(shared_vec).await;

  //
  //
  reset_store(store)
    .await
    .add_middleware(Arc::new(RwLock::new(
      mw_returns_none,
    )))
    .await
    .dispatch_spawn(Action::MwExampleNoSpawn_Foo(1, 2));
  delay_for_spawned_mw_to_execute().await;

  assert_eq!(
    shared_vec.lock().await.pop(),
    Some(-1)
  );

  store.dispatch_spawn(Action::MwExampleNoSpawn_Bar(1));
  delay_for_spawned_mw_to_execute().await;

  assert_eq!(
    shared_vec.lock().await.pop(),
    Some(-2)
  );

  store.dispatch_spawn(Action::MwExampleNoSpawn_Baz);
  delay_for_spawned_mw_to_execute().await;

  assert_eq!(
    shared_vec.lock().await.pop(),
    Some(-3)
  );
}

/// ╭──────────────────────────────────────────────────────╮
/// │ Test async middleware: [MwExampleSpawns].            │
/// ╰──────────────────────────────────────────────────────╯
/// Involves use of both `MwExampleSpawns` mw & `MyReducer` reducer. This middleware
/// spawns a new task that:
/// 1. Adds `-4` to the `shared_vec`.
/// 2. Then dispatches an action to `MyReducer` that resets the store w/ `[-100]`.
async fn run_mw_example_spawns(
  shared_vec: &Arc<Mutex<Vec<i32>>>,
  store: &mut Store<State, Action>,
) {
  let mw_returns_action = MwExampleSpawns {
    shared_vec: shared_vec.clone(),
  };

  reset_shared_object(shared_vec).await;

  reset_store(store)
    .await
    .add_reducer(MyReducer::new())
    .await
    .add_middleware(Arc::new(RwLock::new(
      mw_returns_action,
    )))
    .await
    .dispatch_spawn(Action::MwExampleSpawns_ModifySharedObject_ResetState);
  delay_for_spawned_mw_to_execute().await;

  assert_eq!(shared_vec.lock().await.len(), 1);
  assert_eq!(
    shared_vec
      .lock()
      .await
      .first()
      .unwrap(),
    &-4
  );

  let state = store.get_state().await;
  let stack = state.stack.first().unwrap();
  assert_eq!(*stack, -100);
}

/// ╭──────────────────────────────────────────────────────╮
/// │ MwExampleNoSpawn.                                    │
/// ╰──────────────────────────────────────────────────────╯
struct MwExampleNoSpawn {
  pub shared_vec: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for MwExampleNoSpawn {
  async fn run(
    &self,
    action: Action,
    _: Arc<RwLock<StoreStateMachine<State, Action>>>,
  ) {
    let mut shared_vec = self.shared_vec.lock().await;
    match action {
      Action::MwExampleNoSpawn_Foo(_, _) => shared_vec.push(-1),
      Action::MwExampleNoSpawn_Bar(_) => shared_vec.push(-2),
      Action::MwExampleNoSpawn_Baz => shared_vec.push(-3),
      _ => {}
    }
  }
}

/// ╭──────────────────────────────────────────────────────╮
/// │ MwExampleSpawns.                                     │
/// ╰──────────────────────────────────────────────────────╯
struct MwExampleSpawns {
  pub shared_vec: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for MwExampleSpawns {
  async fn run(
    &self,
    action: Action,
    store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
  ) {
    let so_arc_clone = self.shared_vec.clone();
    fire_and_forget!({
      let mut shared_vec = so_arc_clone.lock().await;
      match action {
        Action::MwExampleSpawns_ModifySharedObject_ResetState => {
          shared_vec.push(-4);
          store_ref
            .write()
            .await
            .dispatch_action(Action::Reset, store_ref.clone())
            .await;
        }
        _ => {}
      }
    });
  }
}

/// ╭──────────────────────────────────────────────────────╮
/// │ MySubscriber.                                        │
/// ╰──────────────────────────────────────────────────────╯
struct MySubscriber {
  pub shared_vec: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncSubscriber<State> for MySubscriber {
  async fn run(
    &self,
    state: State,
  ) {
    let mut stack = self.shared_vec.lock().await;
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
      Action::Reset => State { stack: vec![-100] },
      _ => state.clone(),
    }
  }
}

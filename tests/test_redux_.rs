/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::sync::Arc;

use async_trait::async_trait;
use r3bl_rs_utils::{redux::{AsyncReducer, AsyncSubscriber, Store},
                    SharedStore};
use tokio::sync::RwLock;

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Clone)]
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
  fn default() -> Self { Action::Noop }
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct State {
  pub stack: Vec<i32>,
}

#[tokio::test]
async fn test_redux_store_works_for_main_use_cases() {
  // This shared object is used to collect results from the subscriber &
  // middleware & reducer functions & test it later.
  let shared_vec = Arc::new(RwLock::new(Vec::<i32>::new()));

  // Create the store.
  let mut _store = Store::<State, Action>::default();
  let shared_store: SharedStore<State, Action> = Arc::new(RwLock::new(_store));

  run_reducer_and_subscriber(&shared_vec, &shared_store.clone()).await;
}

async fn reset_shared_object(shared_vec: &Arc<RwLock<Vec<i32>>>) {
  shared_vec.write().await.clear();
}

async fn reset_store(shared_store: &SharedStore<State, Action>) {
  shared_store.write().await.clear_reducers().await;
  shared_store.write().await.clear_subscribers().await;
  shared_store.write().await.clear_middlewares().await;
}

async fn run_reducer_and_subscriber(
  shared_vec: &Arc<RwLock<Vec<i32>>>, shared_store: &SharedStore<State, Action>,
) {
  // Setup store w/ only reducer & subscriber (no middlewares).
  let my_subscriber = MySubscriber {
    shared_vec: shared_vec.clone(),
  };
  reset_shared_object(shared_vec).await;
  reset_store(shared_store).await;

  shared_store
    .write()
    .await
    .add_reducer(MyReducer::new())
    .await
    .add_subscriber(Box::new(my_subscriber))
    .await;

  shared_store
    .write()
    .await
    .dispatch_action(Action::Add(1, 2))
    .await;

  assert_eq!(shared_vec.write().await.pop(), Some(3));

  shared_store
    .write()
    .await
    .dispatch_action(Action::AddPop(1))
    .await;

  assert_eq!(shared_vec.write().await.pop(), Some(4));

  // Clean up the store's state.
  shared_store
    .write()
    .await
    .dispatch_action(Action::Clear)
    .await;

  let state = shared_store.read().await.get_state();
  assert_eq!(state.stack.len(), 0);
}

struct MySubscriber {
  pub shared_vec: Arc<RwLock<Vec<i32>>>,
}

#[async_trait]
impl AsyncSubscriber<State> for MySubscriber {
  async fn run(&self, state: State) {
    let mut stack = self.shared_vec.write().await;
    if !state.stack.is_empty() {
      stack.push(state.stack[0]);
    }
  }
}

#[derive(Default)]
struct MyReducer;

#[async_trait]
impl AsyncReducer<State, Action> for MyReducer {
  async fn run(&self, action: &Action, state: &State) -> State {
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

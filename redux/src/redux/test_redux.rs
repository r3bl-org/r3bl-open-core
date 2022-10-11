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

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use async_trait::async_trait;
  use tokio::{sync::RwLock, task::JoinHandle};

  use crate::{redux::{AsyncMiddleware, AsyncMiddlewareSpawns, AsyncReducer, AsyncSubscriber, Store},
              spawn_dispatch_action,
              SharedStore};

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ Action enum.                                         │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
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

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ State struct.                                        │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  #[derive(Clone, Default, PartialEq, Eq, Debug)]
  pub struct State {
    pub stack: Vec<i32>,
  }

  // FUTURE: Write integration tests for history.

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ Main test runner.                                    │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  #[tokio::test]
  async fn test_redux_store_works_for_main_use_cases() {
    // This shared object is used to collect results from the subscriber &
    // middleware & reducer functions & test it later.
    let shared_vec = Arc::new(RwLock::new(Vec::<i32>::new()));

    // Create the store.
    let mut _store = Store::<State, Action>::default();
    let shared_store: SharedStore<State, Action> = Arc::new(RwLock::new(_store));

    run_reducer_and_subscriber(&shared_vec, &shared_store.clone()).await;
    run_mw_example_no_spawn(&shared_vec, &shared_store.clone()).await;
    run_mw_example_spawns(&shared_vec, &shared_store.clone()).await;
  }

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ Test helpers: Reset shared object.                   │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  async fn reset_shared_object(shared_vec: &Arc<RwLock<Vec<i32>>>) { shared_vec.write().await.clear(); }

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ Test helpers: Reset store.                           │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  async fn reset_store(shared_store: &SharedStore<State, Action>) {
    shared_store.write().await.clear_reducers().await;
    shared_store.write().await.clear_subscribers().await;
    shared_store.write().await.clear_middlewares().await;
  }

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ Test async subscriber: [MySubscriber].               │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  /// 1. Test reducer and subscriber by dispatching `Add` and `AddPop` actions
  /// 2. No middlewares.
  async fn run_reducer_and_subscriber(shared_vec: &Arc<RwLock<Vec<i32>>>, shared_store: &SharedStore<State, Action>) {
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

    shared_store.write().await.dispatch_action(Action::Add(1, 2)).await;

    assert_eq!(shared_vec.write().await.pop(), Some(3));

    shared_store.write().await.dispatch_action(Action::AddPop(1)).await;

    assert_eq!(shared_vec.write().await.pop(), Some(4));

    // Clean up the store's state.
    shared_store.write().await.dispatch_action(Action::Clear).await;

    let state = shared_store.read().await.get_state();
    assert_eq!(state.stack.len(), 0);
  }

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ Test async middleware: [MwExampleNoSpawn].           │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  /// 1. Does not involve any reducers or subscribers.
  /// 2. Just this middleware which modifies the `shared_vec`.
  async fn run_mw_example_no_spawn(shared_vec: &Arc<RwLock<Vec<i32>>>, shared_store: &SharedStore<State, Action>) {
    let mw_returns_none = MwExampleNoSpawn {
      shared_vec: shared_vec.clone(),
    };

    reset_shared_object(shared_vec).await;

    reset_store(shared_store).await;

    shared_store
      .write()
      .await
      .add_middleware(Box::new(mw_returns_none))
      .await
      .dispatch_action(Action::MwExampleNoSpawn_Foo(1, 2))
      .await;

    assert_eq!(shared_vec.write().await.pop(), Some(-1));

    shared_store
      .write()
      .await
      .dispatch_action(Action::MwExampleNoSpawn_Bar(1))
      .await;

    assert_eq!(shared_vec.write().await.pop(), Some(-2));

    shared_store
      .write()
      .await
      .dispatch_action(Action::MwExampleNoSpawn_Baz)
      .await;

    assert_eq!(shared_vec.write().await.pop(), Some(-3));
  }

  async fn delay_for_spawned_mw_to_execute() { tokio::time::sleep(tokio::time::Duration::from_millis(0)).await; }

  /// ```
  /// ╭──────────────────────────────────────────────────────╮
  /// │ Test async middleware: [MwExampleSpawns].            │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  /// Involves use of both `MwExampleSpawns` mw & `MyReducer` reducer. This
  /// middleware spawns a new task that:
  /// 1. Adds `-4` to the `shared_vec`.
  /// 2. Then dispatches an action to `MyReducer` that resets the store w/
  /// `[-100]`.
  async fn run_mw_example_spawns(shared_vec: &Arc<RwLock<Vec<i32>>>, shared_store: &SharedStore<State, Action>) {
    let mw_returns_action = MwExampleSpawns {
      shared_vec: shared_vec.clone(),
    };
    reset_store(shared_store).await;
    reset_shared_object(shared_vec).await;

    shared_store
      .write()
      .await
      .add_reducer(MyReducer::new())
      .await
      .add_middleware_spawns(Box::new(mw_returns_action))
      .await;

    spawn_dispatch_action!(shared_store, Action::MwExampleSpawns_ModifySharedObject_ResetState);

    delay_for_spawned_mw_to_execute().await;

    // .dispatch_action(Action::MwExampleSpawns_ModifySharedObject_ResetState)
    // .await;

    assert_eq!(shared_vec.read().await.len(), 1);
    assert_eq!(shared_vec.read().await.first().unwrap(), &-4);

    let state = shared_store.read().await.get_state();
    let stack = state.stack.first().unwrap();
    assert_eq!(*stack, -100);
  }

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ MwExampleNoSpawn.                                    │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  struct MwExampleNoSpawn {
    pub shared_vec: Arc<RwLock<Vec<i32>>>,
  }

  #[async_trait]
  impl AsyncMiddleware<State, Action> for MwExampleNoSpawn {
    async fn run(&self, action: Action, _state: State) -> Option<Action> {
      let mut shared_vec = self.shared_vec.write().await;
      match action {
        Action::MwExampleNoSpawn_Foo(_, _) => shared_vec.push(-1),
        Action::MwExampleNoSpawn_Bar(_) => shared_vec.push(-2),
        Action::MwExampleNoSpawn_Baz => shared_vec.push(-3),
        _ => {}
      }
      None
    }
  }

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ MwExampleSpawns.                                     │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
  struct MwExampleSpawns {
    pub shared_vec: Arc<RwLock<Vec<i32>>>,
  }

  #[async_trait]
  impl AsyncMiddlewareSpawns<State, Action> for MwExampleSpawns {
    #[allow(clippy::all)]
    async fn run(&self, action: Action, _state: State) -> JoinHandle<Option<Action>> {
      let so_arc_clone = self.shared_vec.clone();
      tokio::spawn(async move {
        let mut shared_vec = so_arc_clone.write().await;
        match action {
          Action::MwExampleSpawns_ModifySharedObject_ResetState => {
            shared_vec.push(-4);
            return Some(Action::Reset);
          }
          _ => {}
        }
        None
      })
    }
  }

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ MySubscriber.                                        │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
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

  /// ```text
  /// ╭──────────────────────────────────────────────────────╮
  /// │ MyReducer.                                           │
  /// ╰──────────────────────────────────────────────────────╯
  /// ```
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
}

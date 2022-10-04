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

//! # References
//! Here are some references to get you started.
//!
//! 1. [In depth guide on this Redux
//!    implementation](https://developerlife.com/2022/03/12/rust-redux/).
//! 2. [Code example of an address book using
//!    Redux](https://github.com/r3bl-org/address-book-with-redux-tui).
//! 3. [Code example of TUI apps using Redux](https://github.com/r3bl-org/r3bl-cmdr).
//!
//! # Short example
//!
//! ```ignore
//! use std::sync::Arc;
//!
//! use async_trait::async_trait;
//! use r3bl_rs_utils::{redux::{AsyncReducer, AsyncSubscriber, Store},
//!                     SharedStore};
//! use tokio::sync::RwLock;
//!
//! #[allow(non_camel_case_types)]
//! #[derive(Debug, PartialEq, Eq, Clone)]
//! pub enum Action {
//!   // Reducer actions.
//!   Add(i32, i32),
//!   AddPop(i32),
//!   Reset,
//!   Clear,
//!   // Middleware actions for MwExampleNoSpawn.
//!   MwExampleNoSpawn_Foo(i32, i32),
//!   MwExampleNoSpawn_Bar(i32),
//!   MwExampleNoSpawn_Baz,
//!   // Middleware actions for MwExampleSpawns.
//!   MwExampleSpawns_ModifySharedObject_ResetState,
//!   // For Default impl.
//!   Noop,
//! }
//!
//! impl Default for Action {
//!   fn default() -> Self { Action::Noop }
//! }
//!
//! #[derive(Clone, Default, PartialEq, Eq, Debug)]
//! pub struct State {
//!   pub stack: Vec<i32>,
//! }
//!
//! #[tokio::test]
//! async fn test_redux_store_works_for_main_use_cases() {
//!   // This shared object is used to collect results from the subscriber &
//!   // middleware & reducer functions & test it later.
//!   let shared_vec = Arc::new(RwLock::new(Vec::<i32>::new()));
//!
//!   // Create the store.
//!   let mut _store = Store::<State, Action>::default();
//!   let shared_store: SharedStore<State, Action> = Arc::new(RwLock::new(_store));
//!
//!   run_reducer_and_subscriber(&shared_vec, &shared_store.clone()).await;
//! }
//!
//! async fn reset_shared_object(shared_vec: &Arc<RwLock<Vec<i32>>>) {
//!   shared_vec.write().await.clear();
//! }
//!
//! async fn reset_store(shared_store: &SharedStore<State, Action>) {
//!   shared_store.write().await.clear_reducers().await;
//!   shared_store.write().await.clear_subscribers().await;
//!   shared_store.write().await.clear_middlewares().await;
//! }
//!
//! async fn run_reducer_and_subscriber(
//!   shared_vec: &Arc<RwLock<Vec<i32>>>, shared_store: &SharedStore<State, Action>,
//! ) {
//!   // Setup store w/ only reducer & subscriber (no middlewares).
//!   let my_subscriber = MySubscriber {
//!     shared_vec: shared_vec.clone(),
//!   };
//!   reset_shared_object(shared_vec).await;
//!   reset_store(shared_store).await;
//!
//!   shared_store
//!     .write()
//!     .await
//!     .add_reducer(MyReducer::new())
//!     .await
//!     .add_subscriber(Box::new(my_subscriber))
//!     .await;
//!
//!   shared_store
//!     .write()
//!     .await
//!     .dispatch_action(Action::Add(1, 2))
//!     .await;
//!
//!   assert_eq!(shared_vec.write().await.pop(), Some(3));
//!
//!   shared_store
//!     .write()
//!     .await
//!     .dispatch_action(Action::AddPop(1))
//!     .await;
//!
//!   assert_eq!(shared_vec.write().await.pop(), Some(4));
//!
//!   // Clean up the store's state.
//!   shared_store
//!     .write()
//!     .await
//!     .dispatch_action(Action::Clear)
//!     .await;
//!
//!   let state = shared_store.read().await.get_state();
//!   assert_eq!(state.stack.len(), 0);
//! }
//!
//! struct MySubscriber {
//!   pub shared_vec: Arc<RwLock<Vec<i32>>>,
//! }
//!
//! #[async_trait]
//! impl AsyncSubscriber<State> for MySubscriber {
//!   async fn run(&self, state: State) {
//!     let mut stack = self.shared_vec.write().await;
//!     if !state.stack.is_empty() {
//!       stack.push(state.stack[0]);
//!     }
//!   }
//! }
//!
//! #[derive(Default)]
//! struct MyReducer;
//!
//! #[async_trait]
//! impl AsyncReducer<State, Action> for MyReducer {
//!   async fn run(&self, action: &Action, state: &State) -> State {
//!     match action {
//!       Action::Add(a, b) => {
//!         let sum = a + b;
//!         State { stack: vec![sum] }
//!       }
//!       Action::AddPop(a) => {
//!         let sum = a + state.stack[0];
//!         State { stack: vec![sum] }
//!       }
//!       Action::Clear => State { stack: vec![] },
//!       Action::Reset => State { stack: vec![-100] },
//!       _ => state.clone(),
//!     }
//!   }
//! }
//! ```
//!
//! # In depth
//!
//! `Store` is thread safe and asynchronous (using Tokio). You have to implement `async` traits in
//! order to use it, by defining your own reducer, subscriber, and middleware trait objects. You
//! also have to supply the Tokio runtime, this library will not create its own runtime. However,
//! for best results, it is best to use the multithreaded Tokio runtime.
//!
//! Once you setup your Redux store w/ your reducer, subscriber, and middleware, you can use it by
//! calling `spawn_dispatch_action!( store, action )`. This kicks off a parallel Tokio task that
//! will run the middleware functions, reducer functions, and finally the subscriber functions. So
//! this will not block the thread of whatever code you call this from. The
//! `spawn_dispatch_action!()` macro itself is not `async`. So you can call it from non `async`
//! code, however you still have to provide a Tokio executor / runtime, without which you will get a
//! panic when `spawn_dispatch_action!()` is called.
//!
//! ### Middlewares
//!
//! Your middleware (`async` trait implementations) will be run concurrently or in parallel via
//! Tokio tasks. You get to choose which `async` trait to implement to do one or the other. And
//! regardless of which kind you implement the `Action` that is optionally returned will be
//! dispatched to the Redux store at the end of execution of all the middlewares (for that
//! particular `spawn_dispatch_action!()` call).
//!
//! 1. `AsyncMiddlewareSpawns<State, Action>` - Your middleware has to use `tokio::spawn` to run
//!    `async` blocks in a [separate
//!    thread](https://docs.rs/tokio/latest/tokio/task/index.html#spawning) and return a
//!    `JoinHandle` that contains an `Option<Action>`. A macro
//!    [`fire_and_forget!`](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/macro.fire_and_forget.html)
//!    is provided so that you can easily spawn parallel blocks of code in your `async` functions.
//!    These are added to the store via a call to `add_middleware_spawns(...)`.
//!
//! 2. `AsyncMiddleware<State, Action>` - They are will all be run together concurrently using
//!    [`futures::join_all()`](https://docs.rs/futures/latest/futures/future/fn.join_all.html).
//!    These are added to the store via a call to `add_middleware(...)`.
//!
//! ### Subscribers
//!
//! The subscribers will be run asynchronously via Tokio tasks. They are all run together
//! concurrently but not in parallel, using
//! [`futures::join_all()`](https://docs.rs/futures/latest/futures/future/fn.join_all.html).
//!
//! ### Reducers
//!
//! The reducer functions are also are `async` functions that are run in the tokio runtime. They're
//! also run one after another in the order in which they're added.
//!
//! ⚡ **Any functions or blocks that you write which uses the Redux library will have to be marked
//! `async` as well. And you will have to spawn the Tokio runtime by using the `#[tokio::main]`
//! macro. If you use the default runtime then Tokio will use multiple threads and its task stealing
//! implementation to give you parallel and concurrent behavior. You can also use the single
//! threaded runtime; its really up to you.**
//!
//! 1. To create middleware you have to implement the `AsyncMiddleware<S,A>` trait or
//!    `AsyncMiddlewareSpawns<S,A>` trait. Please read the [`AsyncMiddleware`
//!    docs](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/redux/async_middleware/trait.AsyncMiddleware.html)
//!    for examples of both. The `run()` method is passed two arguments: the `State` and the
//!    `Action`.
//!
//!    1. For `AsyncMiddlewareSpawns<S,A>` in your `run()` implementation you have to use the
//!       [`fire_and_forget!`](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/macro.fire_and_forget.html)
//!       macro to surround your code. And this will return a `JoinHandle<Option<A>>`.
//!    2. For `AsyncMiddleware<S,A>` in your `run()` implementation you just have to return an
//!       `Option<A>>`.
//!
//! 2. To create reducers you have to implement the `AsyncReducer` trait.
//!
//!    - These should be [pure
//!      functions](https://redux.js.org/understanding/thinking-in-redux/three-principles#changes-are-made-with-pure-functions)
//!      and simply return a new `State` object.
//!    - The `run()` method will be passed two arguments: a ref to `Action` and ref to `State`.
//!
//! 3. To create subscribers you have to implement the `AsyncSubscriber` trait.
//!
//!    - The `run()` method will be passed a `State` object as an argument.
//!    - It returns nothing `()`.
//!
//! ### Summary
//!
//! Here's the gist of how to make & use one of these:
//!
//! 1. Create a struct. Make it derive `Default`. Or you can add your own properties / fields to
//!    this struct, and construct it yourself, or even provide a constructor function.
//!    - A default constructor function `new()` is provided for you by the trait.
//!    - Just follow how that works for when you need to make your own constructor function for a
//!      struct w/ your own properties.
//! 2. Implement the `AsyncMiddleware`, `AsyncMiddlewareSpawns`, `AsyncReducer`, or
//!    `AsyncSubscriber` trait on your struct.
//! 3. Register this struct w/ the store using one of the `add_middleware()`,
//!    `add_middleware_spawns()`, `add_reducer()`, or `add_subscriber()` methods. You can register
//!    as many of these as you like.
//!    - If you have a struct w/ no properties, you can just use the default `::new()` method to
//!      create an instance and pass that to the `add_???()` methods.
//!    - If you have a struct w/ custom properties, you can either implement your own constructor
//!      function or use the following as an argument to the `add_???()` methods:
//!      `Box::new($YOUR_STRUCT))`.
//!
//! ### Examples
//!
//! 💡 There are lots of examples in the
//! [tests](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/test_redux.rs) for this
//! library and in this [CLI application](https://github.com/r3bl-org/address-book-with-redux-tui/)
//! built using it.
//!
//! Here's an example of how to use it. Let's start w/ the import statements.
//!
//! ```ignore
//! /// Imports.
//! use async_trait::async_trait;
//! use r3bl_rs_utils::redux::{
//!   AsyncMiddlewareSpawns, AsyncMiddleware, AsyncReducer,
//!   AsyncSubscriber, Store, StoreStateMachine,
//! };
//! use std::sync::{Arc, Mutex};
//! use tokio::sync::RwLock;
//! ```
//!
//! 1. Make sure to have the `tokio` and `async-trait` crates installed as well as `r3bl_rs_utils`
//!    in your `Cargo.toml` file.
//! 2. Here's an example
//!    [`Cargo.toml`](https://github.com/nazmulidris/rust_scratch/blob/main/address-book-with-redux/Cargo.toml).
//!
//! Let's say we have the following action enum, and state struct.
//!
//! ```ignore
//! /// Action enum.
//! #[derive(Debug, PartialEq, Eq, Clone)]
//! pub enum Action {
//!   Add(i32, i32),
//!   AddPop(i32),
//!   Clear,
//!   MiddlewareCreateClearAction,
//!   Noop,
//! }
//!
//! impl Default for Action {
//!   fn default() -> Self {
//!     Action::Noop
//!   }
//! }
//!
//! /// State.
//! #[derive(Clone, Default, PartialEq, Debug)]
//! pub struct State {
//!   pub stack: Vec<i32>,
//! }
//! ```
//!
//! Here's an example of the reducer function.
//!
//! ```ignore
//! /// Reducer function (pure).
//! #[derive(Default)]
//! struct MyReducer;
//!
//! #[async_trait]
//! impl AsyncReducer<State, Action> for MyReducer {
//!   async fn run(
//!     &self,
//!     action: &Action,
//!     state: &State,
//!   ) -> State {
//!     match action {
//!       Action::Add(a, b) => {
//!         let sum = a + b;
//!         State { stack: vec![sum] }
//!       }
//!       Action::AddPop(a) => {
//!         let sum = a + state.stack[0];
//!         State { stack: vec![sum] }
//!       }
//!       Action::Clear => State { stack: vec![] },
//!       _ => state.clone(),
//!     }
//!   }
//! }
//! ```
//!
//! Here's an example of an async subscriber function (which are run in parallel after an action is
//! dispatched). The following example uses a lambda that captures a shared object. This is a pretty
//! common pattern that you might encounter when creating subscribers that share state in your
//! enclosing block or scope.
//!
//! ```ignore
//! /// This shared object is used to collect results from the subscriber
//! /// function & test it later.
//! let shared_object = Arc::new(Mutex::new(Vec::<i32>::new()));
//!
//! #[derive(Default)]
//! struct MySubscriber {
//!   pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
//! }
//!
//! #[async_trait]
//! impl AsyncSubscriber<State> for MySubscriber {
//!   async fn run(
//!     &self,
//!     state: State,
//!   ) {
//!     let mut stack = self
//!       .shared_object_ref
//!       .lock()
//!       .unwrap();
//!     if !state.stack.is_empty() {
//!       stack.push(state.stack[0]);
//!     }
//!   }
//! }
//!
//! let my_subscriber = MySubscriber {
//!   shared_object_ref: shared_object_ref.clone(),
//! };
//! ```
//!
//! Here are two types of async middleware functions. One that returns an action (which will get
//! dispatched once this middleware returns), and another that doesn't return anything (like a
//! logger middleware that just dumps the current action to the console). Note that both these
//! functions share the `shared_object` reference from above.
//!
//! ```ignore
//! /// This shared object is used to collect results from the subscriber
//! /// function & test it later.
//! #[derive(Default)]
//! struct MwExampleNoSpawn {
//!   pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
//! }
//!
//! #[async_trait]
//! impl AsyncMiddleware<State, Action> for MwExampleNoSpawn {
//!   async fn run(
//!     &self,
//!     action: Action,
//!     _store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
//!   ) {
//!     let mut stack = self
//!       .shared_object_ref
//!       .lock()
//!       .unwrap();
//!     match action {
//!       Action::MwExampleNoSpawn_Add(_, _) => stack.push(-1),
//!       Action::MwExampleNoSpawn_AddPop(_) => stack.push(-2),
//!       Action::MwExampleNoSpawn_Clear => stack.push(-3),
//!       _ => {}
//!     }
//!     None
//!   }
//! }
//!
//! let mw_example_no_spawn = MwExampleNoSpawn {
//!   shared_object_ref: shared_object_ref.clone(),
//! };
//!
//! /// This shared object is used to collect results from the subscriber
//! /// function & test it later.
//! #[derive(Default)]
//! struct MwExampleSpawns {
//!   pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
//! }
//!
//! #[async_trait]
//! impl AsyncMiddlewareSpawns<State, Action> for MwExampleSpawns {
//!   async fn run(
//!     &self,
//!     action: Action,
//!     store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
//!   ) -> JoinHandle<Option<Action>> {
//!     fire_and_forget!(
//!       {
//!         let mut stack = self
//!           .shared_object_ref
//!           .lock()
//!           .unwrap();
//!         match action {
//!           Action::MwExampleSpawns_ModifySharedObject_ResetState => {
//!             shared_vec.push(-4);
//!             return Some(Action::Reset);
//!           }
//!           _ => {}
//!         }
//!         None
//!       }
//!     );
//!   }
//! }
//!
//! let mw_example_spawns = MwExampleSpawns {
//!   shared_object_ref: shared_object_ref.clone(),
//! };
//! ```
//!
//! Here's how you can setup a store with the above reducer, middleware, and subscriber functions.
//!
//! ```ignore
//! // Setup store.
//! let mut store = Store::<State, Action>::default();
//! store
//!   .add_reducer(MyReducer::new()) // Note the use of `::new()` here.
//!   .await
//!   .add_subscriber(Box::new(         // We aren't using `::new()` here
//!     my_subscriber,                  // because the struct has properties.
//!   ))
//!   .await
//!   .add_middleware_spawns(Box::new(  // We aren't using `::new()` here
//!     mw_example_spawns,              // because the struct has properties.
//!   ))
//!   .await
//!   .add_middleware(Box::new(         // We aren't using `::new()` here
//!     mw_example_no_spawn,            // because the struct has properties.
//!   ))
//!   .await;
//! ```
//!
//! Finally here's an example of how to dispatch an action in a test. You can dispatch actions in
//! parallel using `spawn_dispatch_action!()` which is "fire and forget" meaning that the caller
//! won't block or wait for the `spawn_dispatch_action!()` to return.
//!
//! ```ignore
//! // Test reducer and subscriber by dispatching `Add`, `AddPop`, `Clear` actions in parallel.
//! spawn_dispatch_action!( store, Action::Add(1, 2) );
//! assert_eq!(shared_object.lock().unwrap().pop(), Some(3));
//!
//! spawn_dispatch_action!( store, Action::AddPop(1) );
//! assert_eq!(shared_object.lock().unwrap().pop(), Some(4));
//!
//! spawn_dispatch_action!( store, Action::Clear );
//! assert_eq!(store.get_state().stack.len(), 0);
//! ```
//!

pub mod async_middleware;
pub mod async_reducer;
pub mod async_subscriber;
pub mod store;

// Re-export.
pub use async_middleware::*;
pub use async_reducer::*;
pub use async_subscriber::*;
pub use store::*;

// Tests.
mod test_redux;

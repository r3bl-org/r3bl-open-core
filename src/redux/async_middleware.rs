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

use async_trait::async_trait;
use tokio::task::JoinHandle;

/// ```text
/// ╭──────────────────────────────────────────────────────╮
/// │ MwExampleNoSpawn example                             │
/// ╰──────────────────────────────────────────────────────╯
/// ```
/// ```ignore
/// struct MwExampleNoSpawn {
///   pub shared_vec: Arc<Mutex<Vec<i32>>>,
/// }
///
/// #[async_trait]
/// impl AsyncMiddleware<State, Action> for MwExampleNoSpawn {
///   async fn run(
///     &self,
///     action: Action,
///     _state: State,
///   ) -> Option<Action> {
///     let mut shared_vec = self.shared_vec.lock().await;
///     match action {
///       Action::MwExampleNoSpawn_Foo(_, _) => shared_vec.push(-1),
///       Action::MwExampleNoSpawn_Bar(_) => shared_vec.push(-2),
///       Action::MwExampleNoSpawn_Baz => shared_vec.push(-3),
///       _ => {}
///     }
///     None
///   }
/// }
/// ```
#[async_trait]
pub trait AsyncMiddleware<S, A>
where
  S: Sync + Send,
  A: Sync + Send,
{
  async fn run(&self, action: A, state: S) -> Option<A>;

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  #[allow(clippy::all)]
  fn new() -> Box<AsyncMiddlewareTraitObject<S, A>>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Box::new(Self::default())
  }
}

pub type AsyncMiddlewareTraitObject<S, A> = dyn AsyncMiddleware<S, A> + Send + Sync;
pub type AsyncMiddlewareItem<S, A> = Box<dyn AsyncMiddleware<S, A> + Send + Sync>;
pub type AsyncMiddlewareVec<S, A> = Vec<AsyncMiddlewareItem<S, A>>;

/// ```text
/// ╭──────────────────────────────────────────────────────╮
/// │ MwExampleSpawns example                              │
/// ╰──────────────────────────────────────────────────────╯
/// ```
/// ```ignore
/// struct MwExampleSpawns {
///   pub shared_vec: Arc<Mutex<Vec<i32>>>,
/// }
///
/// #[async_trait]
/// impl AsyncMiddlewareSpawns<State, Action> for MwExampleSpawns {
///   async fn run(
///     &self,
///     action: Action,
///     _state: State,
///   ) -> JoinHandle<Option<Action>> {
///     let shared_obj_arc_clone = self.shared_vec.clone();
///     tokio::spawn(async move {
///       let mut shared_vec = shared_obj_arc_clone.lock().await;
///       match action {
///         Action::MwExampleSpawns_ModifySharedObject_ResetState => {
///           shared_vec.push(-4);
///           return Some(Action::Reset);
///         }
///         _ => {}
///       }
///       None
///     })
///   }
/// }
/// ```
#[async_trait]
pub trait AsyncMiddlewareSpawns<S, A>
where
  S: Sync + Send,
  A: Sync + Send,
{
  async fn run(&self, action: A, state: S) -> JoinHandle<Option<A>>;

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  #[allow(clippy::all)]
  fn new() -> AsyncMiddlewareSpawnsItem<S, A>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Box::new(Self::default())
  }
}

pub type AsyncMiddlewareSpawnsTraitObject<S, A> = dyn AsyncMiddlewareSpawns<S, A> + Send + Sync;
pub type AsyncMiddlewareSpawnsItem<S, A> = Box<dyn AsyncMiddlewareSpawns<S, A> + Send + Sync>;
pub type AsyncMiddlewareSpawnsVec<S, A> = Vec<AsyncMiddlewareSpawnsItem<S, A>>;

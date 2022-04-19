/*
 Copyright 2022 R3BL LLC

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

      https://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
*/

use std::sync::Arc;
use super::StoreStateMachine;
use async_trait::async_trait;
use tokio::sync::RwLock;

/// Your code in this trait implementation is able to deadlock in the following situation.
/// 1. A write lock to the store is already held when this function is called by the Redux
/// store.
/// 2. This write lock gets acquired in the [`Store::dispatch`] method. Be careful when
/// dispatching actions from the `run()` method's thread.
/// 3. The async read write lock that's held [`tokio::sync::RwLock`] is NOT reentrant.
///
/// Here's an example that will deadlock.
///
/// ```ignore
/// #[async_trait]
/// impl AsyncMiddleware<State, Action> for AddAsyncCmdMw {
///   async fn run(
///     &self,
///     action: Action,
///     store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
///   ) -> Option<Action> {
///     if let Action::Mw(Mw::AsyncAddCmd) = action {
///       let fake_data = fake_contact_data_api()
///         .await
///         .unwrap_or_else(|_| FakeContactData {
///           name: "Foo Bar".to_string(),
///           phone_h: "123-456-7890".to_string(),
///           email_u: "foo".to_string(),
///           email_d: "bar.com".to_string(),
///           ..FakeContactData::default()
///         });
///       let action = Action::Std(Std::AddContact(
///         format!("{}", fake_data.name),
///         format!(
///           "{}@{}",
///           fake_data.email_u, fake_data.email_d
///         ),
///         format!("{}", fake_data.phone_h),
///       ));
///
///       /* ⚠️ Do not do this. ⚠️ */
///       {
///         let mut my_store = store_ref.write().await; // Deadlock!
///         my_store
///           .dispatch_action(action, store_ref.clone())
///           .await;
///       }
///
///       /* The following avoids this deadlock. */
///       // return Some(action);
///     }
///     None
///   }
/// }
/// ```
///
/// To avoid this situation, just return the action from the `run()` method. And this will
/// safely be dispatched for you w/out deadlock.
///
/// However there are situations where you want to manage your own tasks in parallel and
/// then generate an action or actions when those tasks have completed. In this case, you
/// can manage your own tasks and you can opt-out of returning anything by returning
/// `None` and run your code in a block run by `fire_and_forget!` macro.
///
/// If you want to call the deadlock block above, use the following instead & do not call
/// `return Some(action);`:
///
/// ```ignore
/// use r3bl_rs_utils::fire_and_forget;
///
/// fire_and_forget! { /* block above will not deadlock */ });
/// ```
///
/// By the time the spawned task is executed (and has to acquire its own write or read
/// lock) the held write lock will be dropped & deadlock won't ensue. Be aware that you
/// are responsible for dispatching actions (if any) from your spawned task's thread.
/// Otherwise the Redux store won't know what your tasks have done.
#[async_trait]
pub trait AsyncMiddleware<S, A>
where
  S: Sync + Send,
  A: Sync + Send,
{
  async fn run(
    &self,
    action: A,
    store_ref: Arc<RwLock<StoreStateMachine<S, A>>>,
  );

  /// https://doc.rust-lang.org/book/ch10-02-traits.html
  fn new() -> Arc<RwLock<dyn AsyncMiddleware<S, A> + Send + Sync + 'static>>
  where
    Self: Default + Sized + Sync + Send + 'static,
  {
    Arc::new(RwLock::new(Self::default()))
  }
}

#[derive(Default)]
pub struct AsyncMiddlewareVec<S, A> {
  pub vec: Vec<Arc<RwLock<dyn AsyncMiddleware<S, A> + Send + Sync>>>,
}

impl<S, A> AsyncMiddlewareVec<S, A> {
  pub fn push(
    &mut self,
    middleware: Arc<RwLock<dyn AsyncMiddleware<S, A> + Send + Sync>>,
  ) {
    self.vec.push(middleware);
  }

  pub fn clear(&mut self) {
    self.vec.clear();
  }
}

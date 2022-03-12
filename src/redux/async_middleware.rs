// Imports.
use std::{
  marker::{Send, Sync},
  sync::{Arc, RwLock},
};
use tokio::task::JoinHandle;

/// Excellent resources on lifetimes and returning references:
/// 1. https://stackoverflow.com/questions/59442080/rust-pass-a-function-reference-to-threads
/// 2. https://stackoverflow.com/questions/68547268/cannot-borrow-data-in-an-arc-as-mutable
/// 3. https://willmurphyscode.net/2018/04/25/fixing-a-simple-lifetime-error-in-rust/
pub type SafeMiddlewareFn<A> = Arc<RwLock<dyn FnMut(A) -> Option<A> + Sync + Send>>;
//                             ^^^^^^^^^^                             ^^^^^^^^^^^
//                             Safe to pass      Declare`FnMut` has thread safety
//                             around.           requirement to rust compiler.

pub struct SafeMiddlewareFnWrapper<A> {
  fn_mut: SafeMiddlewareFn<A>,
}

impl<A: Sync + Send + 'static> SafeMiddlewareFnWrapper<A> {
  pub fn new(
    fn_mut: impl FnMut(A) -> Option<A> + Send + Sync + 'static
  ) -> SafeMiddlewareFnWrapper<A> {
    SafeMiddlewareFnWrapper::set(Arc::new(RwLock::new(fn_mut)))
  }

  fn set(fn_mut: SafeMiddlewareFn<A>) -> Self {
    Self { fn_mut }
  }

  /// Get a clone of the `fn_mut` field (which holds a thread safe `FnMut`).
  pub fn get(&self) -> SafeMiddlewareFn<A> {
    self.fn_mut.clone()
  }

  /// This is an async function. Make sure to use `await` on the return value.
  pub fn spawn(
    &self,
    action: A,
  ) -> JoinHandle<Option<A>> {
    let arc_lock_fn_mut = self.get();
    tokio::spawn(async move {
      let mut fn_mut = arc_lock_fn_mut.write().unwrap();
      fn_mut(action)
    })
  }
}

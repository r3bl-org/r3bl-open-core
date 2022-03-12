use std::sync::{Arc, RwLock};
use tokio::task::JoinHandle;

/// Subscriber function.
pub type SafeSubscriberFn<S> = Arc<RwLock<dyn FnMut(S) + Sync + Send>>;

pub struct SafeSubscriberFnWrapper<S> {
  fn_mut: SafeSubscriberFn<S>,
}

impl<S: Sync + Send + 'static> SafeSubscriberFnWrapper<S> {
  pub fn new(
    fn_mut: impl FnMut(S) -> () + Send + Sync + 'static
  ) -> SafeSubscriberFnWrapper<S> {
    SafeSubscriberFnWrapper::set(Arc::new(RwLock::new(fn_mut)))
  }

  fn set(fn_mut: SafeSubscriberFn<S>) -> Self {
    Self { fn_mut }
  }

  pub fn get(&self) -> SafeSubscriberFn<S> {
    self.fn_mut.clone()
  }

  pub fn spawn(
    &self,
    state: S,
  ) -> JoinHandle<()> {
    let arc_lock_fn_mut = self.get();
    tokio::spawn(async move {
      let mut fn_mut = arc_lock_fn_mut.write().unwrap();
      fn_mut(state)
    })
  }
}

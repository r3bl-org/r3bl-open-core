use std::sync::Arc;

/// Reducer function.
pub type ReducerFn<S, A> = dyn Fn(&S, &A) -> S;

pub struct ReducerFnWrapper<S, A> {
  fn_mut: Arc<ReducerFn<S, A>>,
}

impl<S, A> ReducerFnWrapper<S, A> {
  pub fn new(
    fn_mut: impl Fn(&S, &A) -> S + Send + Sync + 'static
  ) -> ReducerFnWrapper<S, A> {
    ReducerFnWrapper::set(Arc::new(fn_mut))
  }

  fn set(fn_mut: Arc<ReducerFn<S, A>>) -> Self {
    Self { fn_mut }
  }

  pub fn get(&self) -> Arc<ReducerFn<S, A>> {
    self.fn_mut.clone()
  }
}

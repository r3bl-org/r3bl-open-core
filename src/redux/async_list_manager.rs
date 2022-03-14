use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SafeListManager<T>
where
  T: Sync + Send + 'static,
{
  list: SafeList<T>,
}

pub type SafeList<T> = Arc<RwLock<Vec<T>>>;

impl<T> Default for SafeListManager<T>
where
  T: Sync + Send + 'static,
{
  fn default() -> Self {
    Self {
      list: Default::default(),
    }
  }
}

impl<T> SafeListManager<T>
where
  T: Sync + Send + 'static,
{
  pub fn get(&self) -> SafeList<T> {
    self.list.clone()
  }

  pub async fn push(
    &mut self,
    item: T,
  ) {
    let arc = self.get();
    let mut locked_list = arc.write().await;
    locked_list.push(item);
  }

  pub async fn clear(&mut self) {
    let arc = self.get();
    let mut locked_list = arc.write().await;
    locked_list.clear();
  }
}

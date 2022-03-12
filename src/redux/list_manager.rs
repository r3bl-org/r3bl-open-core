use std::slice::Iter;

pub struct ListManager<T> {
  subscribers: Vec<T>,
}

impl<T> ListManager<T> {
  pub fn new() -> Self {
    Self {
      subscribers: Vec::new(),
    }
  }

  pub fn push(
    &mut self,
    item: T,
  ) {
    self.subscribers.push(item);
  }

  pub fn clear(&mut self) {
    self.subscribers.clear();
  }

  pub fn iter(&self) -> Iter<T> {
    self.subscribers.iter()
  }
}

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

//! Type aliases to improve code readability.

use std::{
  collections::HashMap,
  sync::{Arc, RwLock, Weak},
};

use super::{Arena, Node};

pub trait HasId: Sync + Send {
  type IdType;

  /// Returns (a clone of) the id.
  fn get_id(&self) -> Self::IdType;

  /// Returns an `Option::Some` containing *a clone of) the id.
  fn into_some(&self) -> Option<Self::IdType>;
}

impl HasId for usize {
  type IdType = usize;

  /// Returns a clone of the id.
  fn get_id(&self) -> usize {
    self.clone()
  }

  /// Returns an `Option::Some` containing a clone of the id.
  fn into_some(&self) -> Option<Self::IdType> {
    Some(self.get_id())
  }
}

// Type aliases for readability.
pub type NodeRef<T> = Arc<RwLock<Node<T>>>;
pub type WeakNodeRef<T> = Weak<RwLock<Node<T>>>;
pub type ArenaMap<T> = HashMap<usize, NodeRef<T>>;

pub type ResultUidList = Option<Vec<usize>>;

// Filter lambda signature.
pub type FilterFn<T> = dyn Fn(usize, T) -> bool + Send + Sync;

// Parallel support.
pub type ShareableArena<T> = Arc<RwLock<Arena<T>>>;
pub type WalkerFn<T> = dyn Fn(usize, T) + Send + Sync;

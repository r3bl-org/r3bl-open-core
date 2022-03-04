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

//! [`Arena`] is defined here.

use std::{
  collections::HashMap,
  fmt::Debug,
  sync::{atomic::AtomicUsize, Arc, RwLock},
};

use crate::utils::{
  call_if_some, unwrap_arc_read_lock_and_call, unwrap_arc_write_lock_and_call, with_mut,
};
use crate::utils::{ReadGuarded, WriteGuarded};

use super::{
  arena_types::HasId, ArenaMap, FilterFn, NodeRef, ResultUidList, WeakNodeRef,
};
/// This struct represents a node in a tree. It may have a parent. It can hold multiple children.
/// And it has a payload. It also has an id that uniquely identifies it. An [`Arena`] or
/// [`super::MTArena`] is used to hold nodes.
#[derive(Debug, Clone)]
pub struct Node<T>
where
  T: Debug + Clone + Send + Sync,
{
  pub id: usize,
  pub parent: Option<usize>,
  pub children: Vec<usize>,
  pub payload: T,
}

impl<T> HasId for Node<T>
where
  T: Debug + Clone + Send + Sync,
{
  type IdType = usize;

  /// Delegate this to `self.id`, which is type `usize`.
  fn get_id(&self) -> usize {
    self.id.get_id()
  }

  /// Delegate this to `self.id`, which is type `usize`.
  fn into_some(&self) -> Option<usize> {
    self.id.into_some()
  }
}

/// Data structure to store & manipulate a (non-binary) tree of data in memory. It can be used as
/// the basis to implement a plethora of different data structures. The non-binary tree is just one
/// example of what can be built using the underlying code.
///
/// 1. [Wikipedia definition of memory
///    arena](https://en.wikipedia.org/wiki/Region-based_memory_management)
/// 2. You can learn more about how this library was built from this [developerlife.com
///    article](https://developerlife.com/2022/02/24/rust-non-binary-tree/).
///
/// # Examples
///
/// ## Basic usage
///
/// ```rust
/// use r3bl_rs_utils::{
///   tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
///   utils::{style_primary, style_prompt},
/// };
///
/// let mut arena = Arena::<usize>::new();
/// let node_1_value = 42 as usize;
/// let node_1_id = arena.add_new_node(node_1_value, None);
/// println!("{} {:#?}", style_primary("node_1_id"), node_1_id);
/// assert_eq!(node_1_id, 0);
/// ```
///
/// ## Get weak and strong references from the arena (tree), and tree walking
///
/// ```rust
/// use r3bl_rs_utils::{
///   tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
///   utils::{style_primary, style_prompt},
/// };
///
/// let mut arena = Arena::<usize>::new();
/// let node_1_value = 42 as usize;
/// let node_1_id = arena.add_new_node(node_1_value, None);
///
/// {
///   assert!(arena.get_node_arc(node_1_id).is_some());
///   let node_1_ref = dbg!(arena.get_node_arc(node_1_id).unwrap());
///   let node_1_ref_weak = arena.get_node_arc_weak(node_1_id).unwrap();
///   assert_eq!(node_1_ref.read().unwrap().payload, node_1_value);
///   assert_eq!(
///     node_1_ref_weak.upgrade().unwrap().read().unwrap().payload,
///     42
///   );
/// }
///
/// {
///   let node_id_dne = 200 as usize;
///   assert!(arena.get_node_arc(node_id_dne).is_none());
/// }
///
/// {
///   let node_1_id = 0 as usize;
///   let node_list = dbg!(arena.tree_walk_dfs(node_1_id).unwrap());
///   assert_eq!(node_list.len(), 1);
///   assert_eq!(node_list, vec![0]);
/// }
/// ```
/// 📜 There are more complex ways of using [`Arena`] and [`super::MTArena`]. Please look at these
/// extensive integration tests that put them thru their paces
/// [here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/tree_memory_arena_test.rs).
#[derive(Debug)]
pub struct Arena<T>
where
  T: Debug + Clone + Send + Sync,
{
  map: RwLock<ArenaMap<T>>,
  atomic_counter: AtomicUsize,
}

impl<T> Arena<T>
where
  T: Debug + Clone + Send + Sync,
{
  /// If no matching nodes can be found returns `None`.
  pub fn filter_all_nodes_by(
    &self,
    filter_fn: &FilterFn<T>,
  ) -> ResultUidList {
    let map: ReadGuarded<ArenaMap<T>> = self.map.read().unwrap();
    let filtered_map = map
      .iter()
      .filter(|(id, node_ref)| filter_fn(**id, node_ref.read().unwrap().payload.clone()))
      .map(|(id, _node_ref)| *id)
      .collect::<Vec<usize>>();
    match filtered_map.len() {
      0 => None,
      _ => Some(filtered_map),
    }
  }

  /// If `node_id` can't be found, returns `None`.
  pub fn get_children_of(
    &self,
    node_id: usize,
  ) -> ResultUidList {
    if !self.node_exists(node_id) {
      return None;
    }
    let node_to_lookup = self.get_node_arc(node_id)?;
    let node_to_lookup: ReadGuarded<Node<T>> = node_to_lookup.read().unwrap(); // Safe to call unwrap.
    let children_uids = &node_to_lookup.children;
    Some(children_uids.clone())
  }

  /// If `node_id` can't be found, returns `None`.
  pub fn get_parent_of(
    &self,
    node_id: usize,
  ) -> Option<usize> {
    if !self.node_exists(node_id) {
      return None;
    }
    let node_to_lookup = self.get_node_arc(node_id)?;
    let node_to_lookup: ReadGuarded<Node<T>> = node_to_lookup.read().unwrap(); // Safe to call unwrap.
    return node_to_lookup.parent.clone();
  }

  pub fn node_exists(
    &self,
    node_id: usize,
  ) -> bool {
    self.map.read().unwrap().contains_key(&node_id.get_id())
  }

  pub fn has_parent(
    &self,
    node_id: usize,
  ) -> bool {
    if self.node_exists(node_id) {
      let parent_id_opt = self.get_parent_of(node_id);
      if parent_id_opt.is_some() {
        let parent_id = parent_id_opt.unwrap();
        return self.node_exists(parent_id);
      }
    }
    return false;
  }

  /// If `node_id` can't be found, returns `None`.
  pub fn delete_node(
    &self,
    node_id: usize,
  ) -> ResultUidList {
    if !self.node_exists(node_id) {
      return None;
    }
    let deletion_list = self.tree_walk_dfs(node_id)?;

    // Note - this lambda expects that `parent_id` exists.
    let remove_node_id_from_parent = |parent_id: usize| {
      let parent_node_arc_opt = self.get_node_arc(parent_id);
      unwrap_arc_write_lock_and_call(&parent_node_arc_opt.unwrap(), &mut |parent_node| {
        parent_node
          .children
          .retain(|child_id| *child_id != node_id.get_id());
      });
    };

    // If `node_id` has a parent, remove `node_id` its children, otherwise skip this step.
    if self.has_parent(node_id) {
      remove_node_id_from_parent(self.get_parent_of(node_id).unwrap()); // Safe to unwrap.
    }

    // Actually delete the nodes in the deletion list.
    let mut map: WriteGuarded<ArenaMap<T>> = self.map.write().unwrap(); // Safe to unwrap.
    deletion_list.iter().for_each(|id| {
      map.remove(id);
    });

    // Pass the deletion list back.
    Some(deletion_list.clone())
  }

  /// DFS graph walking: <https://developerlife.com/2018/08/16/algorithms-in-kotlin-5/>
  /// DFS tree walking: <https://stephenweiss.dev/algorithms-depth-first-search-dfs#handling-non-binary-trees>
  pub fn tree_walk_dfs(
    &self,
    node_id: usize,
  ) -> ResultUidList {
    if !self.node_exists(node_id) {
      return None;
    }
    let mut collected_nodes: Vec<usize> = vec![];
    let mut stack: Vec<usize> = vec![node_id.get_id().clone()];

    while let Some(node_id) = stack.pop() {
      // Question mark operator works below, since it returns a `Option` to `while let ...`.
      // Basically skip to the next item in the `stack` if `node_id` can't be found.
      let node_ref = self.get_node_arc(node_id)?;
      unwrap_arc_read_lock_and_call(&node_ref, &mut |node| {
        collected_nodes.push(node.get_id());
        stack.extend(node.children.iter().cloned());
      });
    }

    match collected_nodes.len() {
      0 => None,
      _ => Some(collected_nodes),
    }
  }

  /// If `node_id` can't be found, returns `None`.
  /// More info on `Option.map()`: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=d5a54a042fea085ef8c9122b7ea47c6a>
  pub fn get_node_arc_weak(
    &self,
    node_id: usize,
  ) -> Option<WeakNodeRef<T>> {
    if !self.node_exists(node_id) {
      return None;
    }
    self
      .map
      .read()
      .unwrap()
      .get(&node_id.get_id()) // Returns `None` if `node_id` doesn't exist.
      .map(|node_ref| Arc::downgrade(&node_ref)) // Runs if `node_ref` is some, else returns `None`.
  }

  /// If `node_id` can't be found, returns `None`.
  /// More info on `Option.map()`: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=d5a54a042fea085ef8c9122b7ea47c6a>
  pub fn get_node_arc(
    &self,
    node_id: usize,
  ) -> Option<NodeRef<T>> {
    if !self.node_exists(node_id) {
      return None;
    }
    self
      .map
      .read()
      .unwrap()
      .get(&node_id.get_id()) // Returns `None` if `node_id` doesn't exist.
      .map(|node_ref| Arc::clone(&node_ref)) // Runs if `node_ref` is some, else returns `None`.
  }

  /// Note `data` is cloned to avoid `data` being moved.
  /// If `parent_id` can't be found, it panics.
  pub fn add_new_node(
    &mut self,
    data: T,
    parent_id_opt: Option<usize>,
  ) -> usize {
    let parent_id_arg_provided = parent_id_opt.is_some();

    // Check to see if `parent_id` exists.
    if parent_id_arg_provided {
      let parent_id = parent_id_opt.unwrap();
      if !self.node_exists(parent_id) {
        panic!("Parent node doesn't exist.");
      }
    }

    let new_node_id = self.generate_uid();

    with_mut(&mut self.map.write().unwrap(), &mut |map| {
      let value = Arc::new(RwLock::new(Node {
        id: new_node_id,
        parent: if parent_id_arg_provided {
          let parent_id = parent_id_opt.unwrap();
          Some(parent_id.get_id())
        } else {
          None
        },
        children: vec![],
        payload: data.clone(),
      }));
      map.insert(new_node_id, value);
    });

    if let Some(parent_id) = parent_id_opt {
      let parent_node_arc_opt = self.get_node_arc(parent_id);
      call_if_some(&parent_node_arc_opt, &|parent_node_arc| {
        unwrap_arc_write_lock_and_call(&parent_node_arc, &mut |parent_node| {
          parent_node.children.push(new_node_id);
        });
      });
    }

    // Return the node identifier.
    return new_node_id;
  }

  fn generate_uid(&self) -> usize {
    self
      .atomic_counter
      .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
  }

  pub fn new() -> Self {
    Arena {
      map: RwLock::new(HashMap::new()),
      atomic_counter: AtomicUsize::new(0),
    }
  }
}

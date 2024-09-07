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

//! [`Arena`] is defined here.

use std::{collections::{HashMap, VecDeque},
          fmt::Debug,
          sync::{atomic::AtomicUsize, Arc, RwLock}};

use super::{arena_types::HasId,
            ArenaMap,
            FilterFn,
            NodeRef,
            ResultUidList,
            WeakNodeRef};
use crate::utils::{call_if_some,
                   unwrap_arc_read_lock_and_call,
                   unwrap_arc_write_lock_and_call,
                   with_mut};

/// This struct represents a node in a tree.
///
/// - It may have a parent.
/// - It can hold multiple children.
/// - And it has a payload.
/// - It also has an id that uniquely identifies it. An [`Arena`] or [`super::MTArena`] is
///   used to hold nodes.
#[derive(Debug)]
pub struct Node<T>
where
    T: Debug + Clone + Send + Sync,
{
    pub id: usize,
    pub parent_id: Option<usize>,
    pub children_ids: VecDeque<usize>,
    pub payload: T,
}

impl<T> HasId for Node<T>
where
    T: Debug + Clone + Send + Sync,
{
    type IdType = usize;

    /// Delegate this to `self.id`, which is type `usize`.
    fn get_id(&self) -> usize { self.id.get_id() }
}

/// Data structure to store & manipulate a (non-binary) tree of data in memory.
///
/// It can be used as the basis to implement a plethora of different data structures. The
/// non-binary tree is just one example of what can be built using the underlying code.
///
/// 1. [Wikipedia definition of memory
///    arena](https://en.wikipedia.org/wiki/Region-based_memory_management)
/// 2. You can learn more about how this library was built from this [developerlife.com
///    article](https://developerlife.com/2022/02/24/rust-non-binary-tree/).
///
/// # Basic usage
///
/// ```rust
/// use r3bl_rs_utils::tree_memory_arena::{Arena, HasId, MTArena, ResultUidList};
/// use r3bl_rs_utils_core::{style_primary, style_prompt};
///
/// let mut arena = Arena::<usize>::new();
/// let node_1_value = 42 as usize;
/// let node_1_id = arena.add_new_node(node_1_value, None);
/// println!("{} {:#?}", style_primary("node_1_id"), node_1_id);
/// assert_eq!(node_1_id, 0);
/// ```
///
/// # Get weak and strong references from the arena (tree), and tree walking
///
/// ```rust
/// use r3bl_rs_utils::tree_memory_arena::{Arena, HasId, MTArena, ResultUidList};
/// use r3bl_rs_utils_core::{style_primary, style_prompt};
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
///
/// 📜 There are more complex ways of using [`Arena`] and [`super::MTArena`]. Please look
/// at these extensive integration tests that put them thru their paces
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
    #[allow(clippy::unwrap_in_result)]
    pub fn filter_all_nodes_by(&self, filter_fn: &FilterFn<T>) -> ResultUidList {
        if let Ok(map /* ReadGuarded<'_, ArenaMap<T>> */) = self.map.read() {
            let filtered_map = map
                .iter()
                .filter(|(id, node_ref)| {
                    filter_fn(**id, node_ref.read().unwrap().payload.clone())
                })
                .map(|(id, _node_ref)| *id)
                .collect::<VecDeque<usize>>();
            match filtered_map.len() {
                0 => None,
                _ => Some(filtered_map),
            }
        } else {
            None
        }
    }

    /// If `node_id` can't be found, returns `None`.
    pub fn get_children_of(&self, node_id: usize) -> ResultUidList {
        if !self.node_exists(node_id) {
            return None;
        }

        let node_to_lookup = self.get_node_arc(node_id)?;

        let result = if let Ok(node_to_lookup /* ReadGuarded<'_, Node<T>> */) =
            node_to_lookup.read()
        {
            if node_to_lookup.children_ids.is_empty() {
                return None;
            }
            Some(node_to_lookup.children_ids.clone())
        } else {
            None
        };

        result
    }

    /// If `node_id` can't be found, returns `None`.
    pub fn get_parent_of(&self, node_id: usize) -> Option<usize> {
        if !self.node_exists(node_id) {
            return None;
        }

        let node_to_lookup = self.get_node_arc(node_id)?;

        let result = if let Ok(node_to_lookup /* ReadGuarded<'_, Node<T>> */) =
            node_to_lookup.read()
        {
            node_to_lookup.parent_id
        } else {
            None
        };

        result
    }

    pub fn node_exists(&self, node_id: usize) -> bool {
        self.map.read().unwrap().contains_key(&node_id.get_id())
    }

    pub fn has_parent(&self, node_id: usize) -> bool {
        if self.node_exists(node_id) {
            let parent_id_opt = self.get_parent_of(node_id);
            if let Some(parent_id) = parent_id_opt {
                return self.node_exists(parent_id);
            }
        }
        false
    }

    /// If `node_id` can't be found, returns `None`.
    pub fn delete_node(&self, node_id: usize) -> ResultUidList {
        if !self.node_exists(node_id) {
            return None;
        }
        let deletion_list = self.tree_walk_dfs(node_id)?;

        // Note - this lambda expects that `parent_id` exists.
        let remove_node_id_from_parent = |parent_id: usize| {
            let parent_node_arc_opt = self.get_node_arc(parent_id);
            if let Some(parent_node_arc) = parent_node_arc_opt {
                if let Ok(mut parent_node /* WriteGuarded<'_, Node<T>> */) =
                    parent_node_arc.write()
                {
                    parent_node
                        .children_ids
                        .retain(|child_id| *child_id != node_id);
                }
            }
        };

        // If `node_id` has a parent, remove `node_id` its children, otherwise skip this
        // step.
        if self.has_parent(node_id) {
            if let Some(parent_id) = self.get_parent_of(node_id) {
                remove_node_id_from_parent(parent_id);
            }
        }

        // Actually delete the nodes in the deletion list.
        if let Ok(mut map /* WriteGuarded<'_, ArenaMap<T>> */) = self.map.write() {
            for node_id in &deletion_list {
                map.remove(node_id);
            }
        }
        // Pass the deletion list back.
        deletion_list.into()
    }

    /// - [DFS graph walking](https://developerlife.com/2018/08/16/algorithms-in-kotlin-5/)
    /// - [DFS tree walking](https://stephenweiss.dev/algorithms-depth-first-search-dfs#handling-non-binary-trees)
    pub fn tree_walk_dfs(&self, node_id: usize) -> ResultUidList {
        if !self.node_exists(node_id) {
            return None;
        }

        let mut stack: VecDeque<usize> = VecDeque::from([node_id.get_id()]);

        let mut it: VecDeque<usize> = VecDeque::new();

        while let Some(node_id) = stack.pop_back() {
            // Question mark operator works below, since it returns a `Option` to `while let ...`.
            // Basically skip to the next item in the `stack` if `node_id` can't be found.
            let node_ref = self.get_node_arc(node_id)?;
            unwrap_arc_read_lock_and_call(&node_ref, &mut |node| {
                it.push_back(node.get_id());
                // Note that the children ordering has to be flipped! You want to perform the
                // traversal from RIGHT -> LEFT (not LEFT -> RIGHT).
                // PTAL: <https://developerlife.com/assets/algo-ts-2-images/depth-first-search.svg>
                for child_id in node.children_ids.iter().rev() {
                    stack.push_back(*child_id);
                }
            });
        }

        match it.len() {
            0 => None,
            _ => Some(it),
        }
    }

    /// - [BFS graph walking](https://developerlife.com/2018/08/16/algorithms-in-kotlin-5/)
    /// - [BFS tree walking](https://stephenweiss.dev/algorithms-depth-first-search-dfs#handling-non-binary-trees)
    pub fn tree_walk_bfs(&self, node_id: usize) -> ResultUidList {
        if !self.node_exists(node_id) {
            return None;
        }

        let mut queue: VecDeque<usize> = VecDeque::from([node_id.get_id()]);

        let mut it: VecDeque<usize> = VecDeque::new();

        while let Some(node_id) = queue.pop_front() {
            // Question mark operator works below, since it returns a `Option` to `while let ...`.
            // Basically skip to the next item in the `stack` if `node_id` can't be found.
            let node_ref = self.get_node_arc(node_id)?;
            unwrap_arc_read_lock_and_call(&node_ref, &mut |node| {
                it.push_back(node.get_id());
                for child_id in node.children_ids.iter() {
                    queue.push_back(*child_id);
                }
            });
        }

        match it.len() {
            0 => None,
            _ => Some(it),
        }
    }

    /// If `node_id` can't be found, returns `None`. More info on
    /// [`Option.map()`](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=d5a54a042fea085ef8c9122b7ea47c6a)
    pub fn get_node_arc_weak(&self, node_id: usize) -> Option<WeakNodeRef<T>> {
        if !self.node_exists(node_id) {
            return None;
        }
        if let Ok(map) = self.map.read() {
            map.get(&node_id.get_id()) // Returns `None` if `node_id` doesn't exist.
                .map(Arc::downgrade) // Runs if `node_ref` is some, else returns `None`.
        } else {
            None
        }
    }

    /// If `node_id` can't be found, returns `None`. More info on
    /// [`Option.map()`](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=d5a54a042fea085ef8c9122b7ea47c6a)
    pub fn get_node_arc(&self, node_id: usize) -> Option<NodeRef<T>> {
        if !self.node_exists(node_id) {
            return None;
        }
        if let Ok(map) = self.map.read() {
            map.get(&node_id.get_id()).cloned() // Runs if `node_ref` is some, else returns `None`.
        } else {
            None
        }
    }

    /// Note `data` is cloned to avoid `data` being moved. If `parent_id` can't be found, it panics.
    pub fn add_new_node(&mut self, payload: T, maybe_parent_id: Option<usize>) -> usize {
        let parent_id_arg_provided = maybe_parent_id.is_some();

        // Check to see if `parent_id` exists.
        if parent_id_arg_provided {
            let parent_id = maybe_parent_id.unwrap();
            if !self.node_exists(parent_id) {
                panic!("Parent node doesn't exist.");
            }
        }

        let new_node_id = self.generate_uid();

        // Create a new node from payload & add it to the arena.
        with_mut(&mut self.map.write().unwrap(), &mut |map| {
            let value = Arc::new(RwLock::new(Node {
                id: new_node_id,
                parent_id: if parent_id_arg_provided {
                    let parent_id = maybe_parent_id.unwrap();
                    Some(parent_id.get_id())
                } else {
                    None
                },
                children_ids: VecDeque::new(),
                payload: payload.clone(),
            }));
            map.insert(new_node_id, value);
        });

        // Deal w/ adding this newly created node to the parent's children list.
        if let Some(parent_id) = maybe_parent_id {
            let maybe_parent_node_arc = self.get_node_arc(parent_id);
            call_if_some(&maybe_parent_node_arc, &|parent_node_arc| {
                unwrap_arc_write_lock_and_call(parent_node_arc, &mut |parent_node| {
                    // Preserve the natural order of insertion.
                    parent_node.children_ids.push_back(new_node_id);
                });
            });
        }

        // Return the node identifier.
        new_node_id
    }

    fn generate_uid(&self) -> usize {
        self.atomic_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn new() -> Self {
        Arena {
            map: RwLock::new(HashMap::new()),
            atomic_counter: AtomicUsize::new(0),
        }
    }
}

impl<T> Default for Arena<T>
where
    T: Debug + Clone + Send + Sync,
{
    fn default() -> Self { Self::new() }
}

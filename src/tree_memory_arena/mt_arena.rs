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

//! [`MTArena`] is defined here.

use std::{fmt::Debug,
          marker::{Send, Sync},
          sync::{Arc, RwLock},
          thread::{spawn, JoinHandle}};

use super::{arena::Arena, Node, ResultUidList, ShareableArena, WalkerFn};
use crate::utils::ReadGuarded;

/// [`MTArena`] is built on top of [`Arena`] but with support for sharing the
/// arena between threads. Also supports tree walking on a separate thread w/ a
/// lambda that's supplied.
///
/// 1. [Wikipedia definition of memory
///    arena](https://en.wikipedia.org/wiki/Region-based_memory_management)
/// 2. You can learn more about how this library was built from this
/// [developerlife.com    article](https://developerlife.com/2022/02/24/rust-non-binary-tree/).
///
/// # Examples
///
/// ```rust
/// use std::{sync::Arc,
///           thread::{self, JoinHandle}};
///
/// use r3bl_rs_utils::tree_memory_arena::{Arena, HasId, MTArena, ResultUidList};
/// use r3bl_rs_utils_core::{style_primary, style_prompt};
///
/// type ThreadResult = Vec<usize>;
/// type Handles = Vec<JoinHandle<ThreadResult>>;
///
/// let mut handles: Handles = Vec::new();
/// let arena = MTArena::<String>::new();
///
/// // Thread 1 - add root. Spawn and wait (since the 2 threads below need the root).
/// {
///   let arena_arc = arena.get_arena_arc();
///   let thread = thread::spawn(move || {
///     let mut arena_write = arena_arc.write().unwrap();
///     let root = arena_write.add_new_node("foo".to_string(), None);
///     vec![root]
///   });
///   thread.join().unwrap();
/// }
///
/// // Perform tree walking in parallel. Note the lambda does capture many enclosing variable context.
/// {
///   let arena_arc = arena.get_arena_arc();
///   let fn_arc = Arc::new(move |uid, payload| {
///     println!(
///       "{} {} {} Arena weak_count:{} strong_count:{}",
///       style_primary("walker_fn - closure"),
///       uid,
///       payload,
///       Arc::weak_count(&arena_arc),
///       Arc::weak_count(&arena_arc)
///     );
///   });
///
///   // Walk tree w/ a new thread using arc to lambda.
///   {
///     let thread_handle: JoinHandle<ResultUidList> = arena.tree_walk_parallel(0, fn_arc.clone());
///
///     let result_node_list = thread_handle.join().unwrap();
///     println!("{:#?}", result_node_list);
///   }
///
///   // Walk tree w/ a new thread using arc to lambda.
///   {
///     let thread_handle: JoinHandle<ResultUidList> = arena.tree_walk_parallel(1, fn_arc.clone());
///
///     let result_node_list = thread_handle.join().unwrap();
///     println!("{:#?}", result_node_list);
///   }
/// }
/// ```
/// 📜 There are more complex ways of using [`super::Arena`] and [`MTArena`].
/// Please look at these extensive integration tests that put them thru their
/// paces [here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/tree_memory_arena_test.rs).
#[derive(Debug)]
pub struct MTArena<T>
where
    T: Debug + Send + Sync + Clone + 'static,
{
    arena_arc: ShareableArena<T>,
}

impl<T> MTArena<T>
where
    T: Debug + Send + Sync + Clone + 'static,
{
    pub fn new() -> Self {
        MTArena {
            arena_arc: Arc::new(RwLock::new(Arena::new())),
        }
    }

    pub fn get_arena_arc(&self) -> ShareableArena<T> { self.arena_arc.clone() }

    /// `walker_fn` is a closure that captures variables. It is wrapped in an
    /// `Arc` to be able to clone that and share it across threads.
    /// More info:
    /// 1. SO thread: <https://stackoverflow.com/a/36213377/2085356>
    /// 2. Scoped threads: <https://docs.rs/crossbeam/0.3.0/crossbeam/struct.Scope.html>
    pub fn tree_walk_parallel(
        &self,
        node_id: usize,
        walker_fn: Arc<WalkerFn<T>>,
    ) -> JoinHandle<ResultUidList> {
        let arena_arc = self.get_arena_arc();
        let walker_fn_arc = walker_fn.clone();

        spawn(move || {
            let read_guard: ReadGuarded<'_, Arena<T>> = arena_arc.read().unwrap();
            let return_value = read_guard.tree_walk_dfs(node_id);

            // While walking the tree, in a separate thread, call the `walker_fn` for each
            // node.
            if let Some(result_list) = return_value.clone() {
                result_list.into_iter().for_each(|uid| {
                    let node_arc_opt = read_guard.get_node_arc(uid);
                    if let Some(node_arc) = node_arc_opt {
                        let node_ref: ReadGuarded<'_, Node<T>> = node_arc.read().unwrap();
                        walker_fn_arc(uid, node_ref.payload.clone());
                    }
                });
            }

            return_value
        })
    }
}

impl<T> Default for MTArena<T>
where
    T: Debug + Send + Sync + Clone + 'static,
{
    fn default() -> Self { Self::new() }
}

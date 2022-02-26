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

//! This module contains a non-binary tree implementation that is thread safe, and even supports
//! parallel tree walking w/ the [`MTArena`] variant.
//!
//! # Examples
//!
//! ## Basic usage
//!
//! ```rust
//! use r3bl_rs_utils::{
//!   tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
//!   utils::{style_primary, style_prompt},
//! };
//!
//! let mut arena = Arena::<usize>::new();
//! let node_1_value = 42 as usize;
//! let node_1_id = arena.add_new_node(node_1_value, None);
//! println!("{} {:#?}", style_primary("node_1_id"), node_1_id);
//! assert_eq!(node_1_id, 0);
//! ```
//!
//! ## Get weak and strong references from the arena (tree), and tree walking
//!
//! ```rust
//! use r3bl_rs_utils::{
//!   tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
//!   utils::{style_primary, style_prompt},
//! };
//!
//! let mut arena = Arena::<usize>::new();
//! let node_1_value = 42 as usize;
//! let node_1_id = arena.add_new_node(node_1_value, None);
//!
//! {
//!   assert!(arena.get_node_arc(&node_1_id).is_some());
//!   let node_1_ref = dbg!(arena.get_node_arc(&node_1_id).unwrap());
//!   let node_1_ref_weak = arena.get_node_arc_weak(&node_1_id).unwrap();
//!   assert_eq!(node_1_ref.read().unwrap().payload, node_1_value);
//!   assert_eq!(
//!     node_1_ref_weak.upgrade().unwrap().read().unwrap().payload,
//!     42
//!   );
//! }
//!
//! {
//!   let node_id_dne = 200 as usize;
//!   assert!(arena.get_node_arc(&node_id_dne).is_none());
//! }
//!
//! {
//!   let node_1_id = 0 as usize;
//!   let node_list = dbg!(arena.tree_walk_dfs(&node_1_id).unwrap());
//!   assert_eq!(node_list.len(), 1);
//!   assert_eq!(node_list, vec![0]);
//! }
//! ```
//!
//! ## You can even tree walk in a separate thread
//!
//! ```rust
//! use std::{
//!   sync::Arc,
//!   thread::{self, JoinHandle},
//! };
//!
//! use r3bl_rs_utils::{
//!   tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
//!   utils::{style_primary, style_prompt},
//! };
//!
//! type ThreadResult = Vec<usize>;
//! type Handles = Vec<JoinHandle<ThreadResult>>;
//!
//! let mut handles: Handles = Vec::new();
//! let arena = MTArena::<String>::new();
//!
//! // Thread 1 - add root. Spawn and wait (since the 2 threads below need the root).
//! {
//!   let arena_arc = arena.get_arena_arc();
//!   let thread = thread::spawn(move || {
//!     let mut arena_write = arena_arc.write().unwrap();
//!     let root = arena_write.add_new_node("foo".to_string(), None);
//!     vec![root]
//!   });
//!   thread.join().unwrap();
//! }
//!
//! // Perform tree walking in parallel. Note the lamda does capture many enclosing variable context.
//! {
//!   let arena_arc = arena.get_arena_arc();
//!   let fn_arc = Arc::new(move |uid, payload| {
//!     println!(
//!       "{} {} {} Arena weak_count:{} strong_count:{}",
//!       style_primary("walker_fn - closure"),
//!       uid,
//!       payload,
//!       Arc::weak_count(&arena_arc),
//!       Arc::weak_count(&arena_arc)
//!     );
//!   });
//!
//!   // Walk tree w/ a new thread using arc to lambda.
//!   {
//!     let thread_handle: JoinHandle<ResultUidList> =
//!       arena.tree_walk_parallel(&0, fn_arc.clone());
//!
//!     let result_node_list = thread_handle.join().unwrap();
//!     println!("{:#?}", result_node_list);
//!   }
//!
//!   // Walk tree w/ a new thread using arc to lambda.
//!   {
//!     let thread_handle: JoinHandle<ResultUidList> =
//!       arena.tree_walk_parallel(&1, fn_arc.clone());
//!
//!     let result_node_list = thread_handle.join().unwrap();
//!     println!("{:#?}", result_node_list);
//!   }
//! }
//! ```
//!
//! 📜 There are more complex ways of using this `Arena`. Please look at these extensive integration
//! tests that put the `Arena` API thru its paces
//! [here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/tree_memory_arena_test.rs).

pub mod arena;
pub mod arena_types;
pub mod mt_arena;

// Module re-exports:
// <https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#documentation-comments-as-tests>

// Re-export the following modules:
pub use arena::*; // Arena.
pub use arena_types::*; // HasId, Node, NodeRef, WeakNodeRef, ReadGuarded, WriteGuarded, ArenaMap, FilterFn, ResultUidList
pub use mt_arena::*; // MTArena.

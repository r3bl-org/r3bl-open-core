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

//! This module contains:
//! 1. [`arena`] - a non-binary tree implementation that is thread safe.
//! 2. [`mt_arena`] - a variant of the tree that supports parallel tree walking.
//!
//! 💡 You can learn more about how this library was built from this [developerlife.com
//! article](https://developerlife.com/2022/02/24/rust-non-binary-tree/).
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

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

//! This module contains [`Arena`] a non-binary tree implementation that is
//! thread safe, and [`MTArena`] a variant of the tree that supports parallel
//! tree walking.
//!
//! 💡 You can learn more about how this library was built from this
//! [developerlife.com article](https://developerlife.com/2022/02/24/rust-non-binary-tree/).
//!
//! 📜 There are more complex ways of using [`Arena`] and [`MTArena`]. Please
//! look at these extensive integration tests that put them thru their paces
//! [here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/tree_memory_arena_test.rs).

// Attach sources.
pub mod arena;
pub mod arena_types;
pub mod mt_arena;

// Re-export.
pub use arena::*; // Arena.
pub use arena_types::*; // Arena type aliases.
pub use mt_arena::*; // MTArena.

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

//! This library provides utility functions:
//!  1. Thread safe asynchronous Redux library (uses Tokio to run subscribers
//! and middleware in     separate tasks). The reducer functions are run in
//! sequence (not in Tokio tasks).  2. Declarative macros, and procedural macros
//! (both function like and derive) to avoid     having to write lots of
//! boilerplate code for many common (and complex) tasks.  3. Non binary tree
//! data structure inspired by memory arenas, that is thread safe and
//!     supports parallel tree walking.
//!  4. Functions to unwrap deeply nested objects inspired by Kotlin scope
//! functions.  5. Capabilities to make it easier to build TUIs (Text User
//! Interface apps) in Rust. This     is currently experimental and is being
//! actively developed.
//!
//! > 💡 To learn more about this library, please read how it was built on
//! > [developerlife.com](https://developerlife.com):
//! >
//! > 1. https://developerlife.com/2022/02/24/rust-non-binary-tree/
//! > 2. https://developerlife.com/2022/03/12/rust-redux/
//! > 3. https://developerlife.com/2022/03/30/rust-proc-macro/
//!
//! > 💡 You can also read all the Rust content on
//! > [developerlife.com here](https://developerlife.com/category/Rust/). Also,
//! > the equivalent
//! > of this library is available for TypeScript and is called
//! > [r3bl-ts-utils](https://github.com/r3bl-org/r3bl-ts-utils/).

// Attach the following files to the library module.
pub mod redux;
pub mod tree_memory_arena;
pub mod tui;
pub mod utils;

// Re-export.
// Re-export from core and macro (so users of public crate can use them w/out
// having to add dependency on each core and macro).
pub use r3bl_rs_utils_core::*;
pub use r3bl_rs_utils_macro::*;
pub use redux::*;
pub use tree_memory_arena::*;
pub use tui::*;
pub use utils::*;

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

//! This crate provides utility functions to do the following:
//! 1. Non binary tree data structure that is safe to use across threads and supports parallel tree
//!    walking & processing (inspired by [memory
//!    arena](https://en.wikipedia.org/wiki/Region-based_memory_management).
//! 2. Kotlin inspired [scope functions](https://kotlinlang.org/docs/scope-functions.html) that make
//!    it easy to work w/ wrapped values (wrapped in [`std::sync::Arc`]<[`std::sync::RwLock`]>, or
//!    [`Option`], etc).
//! 3. Print colored text to the terminal. And create TUI (text user interface) applications,
//!    similar to what you can do w/ [Ink, React, and TypeScript on
//!    Node.js](https://github.com/r3bl-org/r3bl-ts-utils).

// Attach the following files to the library module.
pub mod tree_memory_arena;
pub mod utils;

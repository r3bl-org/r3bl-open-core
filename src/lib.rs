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

//! This crate provides lots of useful functionality to help you build TUI (text user interface)
//! apps, along w/ general niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉:
//!
//! 1. Thread-safe & fully asynchronous [Redux](#2-redux) library (using Tokio to run subscribers
//!    and middleware in separate tasks). The reducer functions are run sequentially.
//! 2. Loosely coupled & fully asynchronous [TUI framework](#6-tui-coming-soon) to make it possible
//!    (and easy) to build sophisticated TUIs (Text User Interface apps) in Rust. This is currently
//!    under [active development](#61-tuicore).
//! 3. Lots of [declarative macros](#31-declarative), and [procedural macros](#32-procedural) (both
//!    function like and derive) to avoid having to write lots of boilerplate code for many common
//!    (and complex) tasks.
//! 4. [Non binary tree data](#4-treememoryarena-non-binary-tree-data-structure) structure inspired
//!    by memory arenas, that is thread safe and supports parallel tree walking.
//! 5. Utility functions to improve [ergonomics](#5-utils) of commonly used patterns in Rust
//!    programming, ranging from things like colorizing `stdout`, `stderr` output, to having less
//!    noisy `Result` and `Error` types.
//!
//! ## Learn more about how this library is built
//!
//! 🦜 Here are some articles (on [developerlife.com](https://developerlife.com)) about how this
//! crate is made:
//! 1. <https://developerlife.com/2022/02/24/rust-non-binary-tree/>
//! 2. <https://developerlife.com/2022/03/12/rust-redux/>
//! 3. <https://developerlife.com/2022/03/30/rust-proc-macro/>
//!
//! 🦀 You can also find all the Rust related content on developerlife.com
//! [here](https://developerlife.com/category/Rust/).
//!
//! 🤷‍♂️ Fun fact: before we built this crate, we built a library that is similar in spirit for
//! TypeScript (for TUI apps on Node.js) called
//! [r3bl-ts-utils](https://github.com/r3bl-org/r3bl-ts-utils/). We have since switched to Rust
//! 🦀🎉.

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

// Attach the following files to the library module.
pub mod redux;
pub mod tree_memory_arena;
pub mod tui;
pub mod utils;

// Re-export from core and macro (so users of public crate can use them w/out having to
// add dependency on each core and macro).
pub use r3bl_rs_utils_core::*;
pub use r3bl_rs_utils_macro::*;
pub use redux::*;
pub use tree_memory_arena::*;
pub use tui::*;
pub use utils::*;

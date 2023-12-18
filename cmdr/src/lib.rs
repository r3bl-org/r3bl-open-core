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

//! # r3bl-cmdr
//!
//! This TUI (text user interface) app showcases the use of the [`r3bl_rs_utils`
//! crate](https://crates.io/crates/r3bl_rs_utils). It contains quite a few sample apps which are
//! meant to be relevant use cases that are relevant for developer workflows (who are remote, and
//! work w/ teams).
//!
//! The [`r3bl_rs_utils` crate](https://crates.io/crates/r3bl_rs_utils) allows you to build fully
//! async (parallel and concurrent via Tokio) TUI apps with a modern API that integrates the best of
//! frontend web development.
//!
//! Here are some framework highlights:
//! - The entire TUI framework itself supports concurrency & parallelism (user input, rendering,
//!   etc. are generally non blocking).
//! - You can use:
//!   - something like Flexbox for responsive layout.
//!   - something like CSS for styling.
//!   - Redux for state management (fully async, concurrent & parallel).
//!   - A lolcat implementation w/ a rainbow color-wheel palette.

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

pub const DEVELOPMENT_MODE: bool = true;

pub mod giti;
pub mod rc;

pub use giti::*;
pub use rc::*;

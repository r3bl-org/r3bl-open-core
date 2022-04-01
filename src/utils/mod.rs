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

//! This module contains a lot of utility functions that are meant to:
//! 1. Increase the ergonomics of using wrapped values in Rust
//! 2. Colorizing console output.
//! 3. Easy to work w/ lazy hash maps.
//! 4. Easy to work w/ readline.
//! 5. Interrogation of types.

pub mod color_text;
pub mod lazy;
pub mod safe_unwrap;
pub mod tty;
pub mod type_utils;
pub mod decl_macros;
pub mod async_safe_share_mutate;

// Module re-exports:
// <https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#documentation-comments-as-tests>

// Re-export the following modules:
pub use color_text::styles::*;
pub use color_text::*;
pub use lazy::*;
pub use safe_unwrap::*;
pub use tty::*;
pub use type_utils::*;
pub use decl_macros::*;
pub use async_safe_share_mutate::*;

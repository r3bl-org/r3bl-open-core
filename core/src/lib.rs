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

//! This crate is a dependency of the following crates:
//! 1. [`r3bl_rs_utils_macro`](https://crates.io/crates/r3bl_rs_utils_macro) (procedural macros)
//! 2. [`r3bl_rs_utils`](https://crates.io/crates/r3bl_rs_utils)
//!
//! Due to the [requirements of proc macros being in a separate
//! crate](https://developerlife.com/2022/03/30/rust-proc-macro/#add-an-internal-or-core-crate),
//! this breakdown of one crate into multiple crates is necessary:
//! 1. Put some code in a separate crate (`r3bl_rs_utils_core`) that is used by other crates.
//! 2. Put the proc macros in a separate crate (`r3bl_rs_utils_macro`). This crate also depends on
//!    the `r3bl_rs_utils_core` crate.
//! 3. Finally, make the "public" crate (`r3bl_rs_utils`) depend on the other two.
//!
//! As a way to hide this kind of layering from the users of the "main" `r3bl_rs_utils` crate, all
//! the modules tend to be re-exported, making them available from the "main" or top-level crate;
//! more info on this
//! [here](https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html?highlight=module%20re-export#re-exporting-names-with-pub-use).

// Connect to source file.
pub mod async_safe_share_mutate;
pub mod color_text;
pub mod common;
pub mod decl_macros;
pub mod tui_core;

// Re-export.
pub use async_safe_share_mutate::*;
pub use color_text::{styles::*, *};
pub use common::*;
pub use decl_macros::*;
pub use tui_core::*;

// Tests.
mod test_decl_macros;
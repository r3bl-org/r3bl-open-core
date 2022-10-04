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

//! All the modules in the `r3bl_rs_utils_core` crate are in support of the `tui` module in the
//! "main" [`r3bl_rs_utils`](https://crates.io/crates/r3bl_rs_utils) crate.

// Attach sources.
pub mod ansi;
pub mod dimens;
pub mod graphemes;
pub mod lolcat;
pub mod styles;

// Re-export.
pub use ansi::*;
pub use dimens::*;
pub use graphemes::*;
pub use lolcat::*;
pub use styles::*;

// Tests.
mod test_ansi_text;

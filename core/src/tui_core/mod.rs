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
pub mod color_wheel;
pub mod color_wheel_core;
pub mod constants;
pub mod dimens;
pub mod graphemes;
pub mod tui_style;
pub mod tui_styled_text;

// Re-export.
pub use color_wheel::*;
pub use color_wheel_core::*;
pub use constants::*;
pub use dimens::*;
pub use graphemes::*;
pub use tui_style::*;
pub use tui_styled_text::*;

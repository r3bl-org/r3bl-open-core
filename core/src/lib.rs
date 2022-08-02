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

// FIXME: add short documentation here explaining why core exists

// Connect to source file.
pub mod common;
pub mod async_safe_share_mutate;
pub mod decl_macros;
pub mod color_text;
pub mod tui_core;

// Re-export.
pub use async_safe_share_mutate::*;
pub use color_text::{styles::*, *};
pub use common::*;
pub use decl_macros::*;
pub use tui_core::*;

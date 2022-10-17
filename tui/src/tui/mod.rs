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

/// This is the global `DEBUG` const. It is possible to create local (module scoped) `DEBUG` const.
/// However, you would have to use that symbol explicitly in the relevant module, eg:
/// - `use $crate::terminal_lib_backends::DEBUG;`
///
/// If set to `true`:
/// 1. Enables or disables file logging for entire module.
/// 2. If a call to [r3bl_rs_utils_core::log!] fails, then it will print the error to stderr.
pub const DEBUG_TUI_MOD: bool = true;

// Enable or disable debug logging for this `terminal_lib_backends` module.
pub const DEBUG_SHOW_PIPELINE: bool = true;
pub const DEBUG_SHOW_PIPELINE_EXPANDED: bool = false;
pub const DEBUG_SHOW_TERMINAL_BACKEND: bool = false;

// Attach sources.
pub mod editor;
pub mod layout;
pub mod misc_types;
pub mod rsx;
pub mod syntax_highlighting;
pub mod terminal_lib_backends;
pub mod terminal_window;

// Re-export.
pub use editor::*;
pub use layout::*;
pub use misc_types::*;
pub use rsx::*;
pub use syntax_highlighting::*;
pub use terminal_lib_backends::*;
pub use terminal_window::*;

// Tests.
mod test_make_style_macro;
mod test_style;
mod test_tui_serde;

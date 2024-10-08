/*
 *   Copyright (c) 2023-2024 R3BL LLC
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

/// This is the global `DEBUG` const. It is possible to create local (module scoped)
/// `DEBUG` const. However, you would have to use that symbol explicitly in the relevant
/// module, eg:
/// - `use $crate::terminal_lib_backends::DEBUG_TUI...;`
///
/// If set to `true` it enables file logging for entire module.
pub const DEBUG_TUI_MOD: bool = true;

/// Enable or disable MD parser debug logging. This makes the parser very slow when
/// enabled.
pub const DEBUG_MD_PARSER: bool = false;
/// This is for running tests on the MD parser. No need to enable logging for this to
/// work.
pub const DEBUG_MD_PARSER_STDOUT: bool = false;

/// Enable or disable syntax highlighting debug logging.
pub const DEBUG_TUI_SYN_HI: bool = false;

/// Enable or disable select, copy, paste debug logging.
pub const DEBUG_TUI_COPY_PASTE: bool = false;

/// Enable or disable compositor debug logging.
pub const DEBUG_TUI_COMPOSITOR: bool = false;

// Enable or disable debug logging for this `terminal_lib_backends` module.
pub const DEBUG_TUI_SHOW_PIPELINE: bool = false;

pub const DEBUG_TUI_SHOW_PIPELINE_EXPANDED: bool = false;

/// Controls input event debugging [crate::InputDeviceExt], and execution of render ops
/// [crate::queue_render_op!] debugging output.
pub const DEBUG_TUI_SHOW_TERMINAL_BACKEND: bool = false;

// Attach sources.
pub mod animator;
pub mod dialog;
pub mod editor;
pub mod global_constants;
pub mod layout;
pub mod md_parser;
pub mod misc;
pub mod rsx;
pub mod syntax_highlighting;
pub mod terminal_lib_backends;
pub mod terminal_window;

// Re-export.
pub use animator::*;
pub use dialog::*;
pub use editor::*;
pub use global_constants::*;
pub use layout::*;
pub use md_parser::*;
pub use misc::*;
pub use rsx::*;
pub use syntax_highlighting::*;
pub use terminal_lib_backends::*;
pub use terminal_window::*;

// Tests.
mod test_make_style_macro;
mod test_tui_serde;

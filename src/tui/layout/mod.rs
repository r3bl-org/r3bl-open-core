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
/// 2. If a call to [crate::log!] fails, then it will print the error to stderr.
pub const DEBUG: bool = true;

// Attach source files.
pub mod flex_box;
pub mod layout_error;
pub mod layout_management;
pub mod surface;

// Re-export the public items.
pub use flex_box::*;
pub use layout_error::*;
pub use layout_management::*;
pub use surface::*;

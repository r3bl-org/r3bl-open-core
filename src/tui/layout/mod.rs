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

/// If set to true, and the [crate::log!] fails, then it will print the error to
/// stderr. Also enables or disables file logging for entire module.
pub const DEBUG: bool = true;

// Attach source files.
pub mod layout_error;
pub mod layout_management;
pub mod tw_box;
pub mod tw_surface;

// Re-export the public items.
pub use layout_error::*;
pub use layout_management::*;
pub use tw_box::*;
pub use tw_surface::*;

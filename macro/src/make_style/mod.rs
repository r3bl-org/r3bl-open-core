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

/// This generates DEBUG messages when compiling (running cargo build).
const DEBUG_MAKE_STYLE_MOD: bool = false;

// Attach sources.
pub mod codegen;
pub mod entry_point;
pub mod meta;
pub mod syntax_parse;

// Re-export.
pub(crate) use codegen::*; /* Internal. */
pub use entry_point::*; /* External. */
pub(crate) use meta::*; /* Internal. */

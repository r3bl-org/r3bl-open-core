/*
 *   Copyright (c) 2023 R3BL LLC
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

//! This module holds the integration or glue code that ties together:
//! 1. [crate::md_parser] - Responsible for parsing markdown into a [crate::MdDocument] data
//!    structure.
//! 2. [crate::syntax_highlighting] - Responsible for converting a [crate::MdDocument] into a list
//!    of tuples of [r3bl_rs_utils_core::TuiStyle] and [String].
//! 3. [crate::editor] - Responsible for displaying the [crate::MdDocument] to the user.

// Attach.
pub mod md_parser_stylesheet;
pub mod md_parser_syn_hi_impl;

// Re-export.
pub use md_parser_stylesheet::*;
pub use md_parser_syn_hi_impl::*;

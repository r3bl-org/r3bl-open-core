/*
 *   Copyright (c) 2025 R3BL LLC
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

//! Test data module for markdown parser compatibility testing.
//!
//! # Important: Null Padding Requirement
//!
//! **WARNING**: The `&str` constants provided by this module CANNOT be used directly with
//! the markdown parser! The parser now requires input from `ZeroCopyGapBuffer` which
//! enforces a "null padding invariant" where lines end with `\n` followed by zero or more
//! `\0` characters.
//!
//! ## Required Conversion
//!
//! Before using any test data from this module, you MUST convert it to
//! `ZeroCopyGapBuffer`:
//!
//! ```rust,ignore
//! use crate::{convert_str_to_gap_buffer, convert_vec_lines_to_gap_buffer};
//!
//! // For string constants:
//! let gap_buffer = convert_str_to_gap_buffer(SOME_TEST_CONSTANT);
//! let result = parse_markdown(&gap_buffer);
//!
//! // For vec of GCString lines:
//! let gap_buffer = convert_vec_lines_to_gap_buffer(&vec_of_gc_strings);
//! let result = parse_markdown(&gap_buffer);
//! ```
//!
//! ## Module Organization
//!
//! This module organizes test inputs by complexity and content type:
//! - `invalid_inputs`: Edge cases and malformed syntax
//! - `valid_small_inputs`: Simple formatting and single lines
//! - `valid_medium_inputs`: Multi-paragraph and structured content
//! - `valid_large_inputs`: Complex nested structures
//! - `valid_jumbo_inputs`: Real-world files and comprehensive documents

pub mod invalid_inputs;
pub mod valid_jumbo_inputs;
pub mod valid_large_inputs;
pub mod valid_medium_inputs;
pub mod valid_small_inputs;

// Re-export all constants for easy access
pub use invalid_inputs::*;
pub use valid_jumbo_inputs::*;
pub use valid_large_inputs::*;
pub use valid_medium_inputs::*;
pub use valid_small_inputs::*;

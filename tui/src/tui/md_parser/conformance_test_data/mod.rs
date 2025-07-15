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
//! This module organizes test inputs by complexity and content type:
//! - `invalid_inputs`: Edge cases and malformed syntax
//! - `valid_small_inputs`: Simple formatting and single lines  
//! - `valid_medium_inputs`: Multi-paragraph and structured content
//! - `valid_large_inputs`: Complex nested structures
//! - `valid_jumbo_inputs`: Real-world files and comprehensive documents

pub mod invalid_inputs;
pub mod valid_small_inputs;
pub mod valid_medium_inputs;
pub mod valid_large_inputs;
pub mod valid_jumbo_inputs;

// Re-export all constants for easy access
pub use invalid_inputs::*;
pub use valid_small_inputs::*;
pub use valid_medium_inputs::*;
pub use valid_large_inputs::*;
pub use valid_jumbo_inputs::*;

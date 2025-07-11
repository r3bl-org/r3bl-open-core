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

//! Jumbo-sized markdown inputs for performance testing.
//!
//! These inputs contain real-world large markdown files for performance benchmarking:
//! - Comprehensive API documentation with complex structures
//! - Technical guides with extensive code examples
//! - Large documents with Unicode, tables, and mixed content
//!
//! These files test parser performance with real-world content complexity.

/// Comprehensive API documentation with complex markdown structures.
/// This represents the largest category of real-world markdown documents,
/// containing extensive technical content, code blocks, tables, and Unicode characters.
pub const REAL_WORLD_EDITOR_CONTENT: &str =
    include_str!("real_world_files/jumbo_api_documentation.md");

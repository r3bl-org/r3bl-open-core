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

//! Large-scale markdown inputs for comprehensive testing.
//!
//! These inputs represent larger documents with complex structures and mixed content:
//! - Long multi-paragraph documents
//! - Complex nested structures
//! - Large amounts of content approximating real-world usage

/// Real-world large markdown document with complex structure, Unicode, and varied
/// content. This tests comprehensive parsing capabilities including tables, nested lists,
/// code blocks, and international characters from an actual technical documentation file.
pub const COMPLEX_NESTED_DOCUMENT: &str =
    include_str!("real_world_files/large_complex_document.md");

/// Real-world tutorial document with comprehensive Rust programming examples.
/// This tests parsing of extensive code blocks, nested structures, and technical
/// documentation patterns commonly found in programming tutorials and guides.
pub const TUTORIAL_DOCUMENT: &str = include_str!("real_world_files/medium_blog_post.md");

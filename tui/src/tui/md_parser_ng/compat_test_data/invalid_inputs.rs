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

//! Invalid and edge case markdown inputs for testing parser robustness.
//!
//! These inputs test how both parsers handle malformed syntax, edge cases,
//! and boundary conditions. Both parsers should fail consistently on these inputs.

/// Malformed markdown syntax with invalid headings, unclosed code blocks, and invalid checkboxes
pub const MALFORMED_SYNTAX: &str = "###not a heading\n```notclosed\n- [  invalid checkbox\n*not bold text";

/// Unclosed formatting markers that should be handled gracefully
pub const UNCLOSED_FORMATTING: &str = "This has *unclosed bold\nThis has _unclosed italic\nThis has `unclosed code";

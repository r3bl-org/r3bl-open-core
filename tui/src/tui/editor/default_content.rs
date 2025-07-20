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

//! Default content for editor components and examples.
//!
//! This module provides predefined markdown content that can be used to initialize
//! editor components with meaningful default content for examples, demos, and tests.
//!
//! The content is loaded from external markdown files to ensure consistency between
//! examples and test data, maintaining a single source of truth for shared content.

/// Real-world editor content for the `ex_editor` example and related tests.
///
/// This content demonstrates various markdown features including:
/// - Metadata headers (@title, @tags, @authors, @date)
/// - Headings with emojis for visual appeal
/// - Numbered and bulleted lists with nested items
/// - Code blocks with syntax highlighting (TypeScript and Rust)
/// - Inline code snippets and formatting
/// - Links to external resources
/// - Task lists with checkboxes
/// - Mixed formatting combinations
///
/// The content is loaded from `ex_editor.md` which serves as the authoritative
/// source for this example content. This ensures that both the editor example
/// and the parser tests use identical content, maintaining consistency and
/// preventing data drift between different parts of the codebase.
///
/// # Usage
///
/// ```no_run
/// use r3bl_tui::{EX_EDITOR_CONTENT, parse_markdown};
///
/// // Split content into lines for editor buffer initialization
/// let lines: Vec<&str> = EX_EDITOR_CONTENT.lines().collect();
///
/// // Use directly for parser testing
/// let parsed = parse_markdown(EX_EDITOR_CONTENT);
/// ```
pub const EX_EDITOR_CONTENT: &str =
    include_str!("../md_parser/conformance_test_data/real_world_files/ex_editor.md");

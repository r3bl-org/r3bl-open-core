// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
/// use r3bl_tui::{EX_EDITOR_CONTENT, parse_markdown, ZeroCopyGapBuffer};
///
/// // Split content into lines for editor buffer initialization
/// let lines: Vec<&str> = EX_EDITOR_CONTENT.lines().collect();
///
/// // Use for parser testing - convert to ZeroCopyGapBuffer first
/// let gap_buffer = ZeroCopyGapBuffer::from(EX_EDITOR_CONTENT);
/// let parsed = parse_markdown(&gap_buffer);
/// ```
pub const EX_EDITOR_CONTENT: &str =
    include_str!("../md_parser/conformance_test_data/real_world_files/ex_editor.md");

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

//! Small and simple markdown inputs for basic functionality testing.
//!
//! These inputs test individual markdown features in isolation:
//! - Basic text handling (empty strings, single lines)
//! - Simple formatting (bold, italic, inline code)
//! - Basic elements (links, images, metadata)
//! - Unicode and special characters

// Basic text handling
pub const EMPTY_STRING: &str = "";
pub const ONLY_NEWLINES: &str = "\n\n\n";
pub const SINGLE_LINE_NO_NEWLINE: &str = "Hello World";
pub const SINGLE_LINE_WITH_NEWLINE: &str = "Hello World\n";

// Simple inline code
pub const SIMPLE_INLINE_CODE: &str = "first\n`second`";
pub const INLINE_CODE_VARIATIONS: &str = "`simple code`\n`code with spaces`\n`code-with-dashes`\n`code_with_underscores`";
pub const INLINE_CODE_WITH_UNICODE: &str = "`code 🎯`";

// Basic formatting
pub const BOLD_TEXT: &str = "This is *bold* text";
pub const ITALIC_TEXT: &str = "This is _italic_ text";
pub const MIXED_FORMATTING: &str = "Mix of *bold* and _italic_ and `code`";

// Basic elements
pub const LINKS: &str = "Check out [Rust](https://rust-lang.org) website";
pub const IMAGES: &str = "![Alt text](https://example.com/image.png)";

// Metadata
pub const METADATA_TITLE: &str = "@title: My Document Title";
pub const METADATA_TAGS: &str = "@tags: rust, programming, tutorial";
pub const METADATA_AUTHORS: &str = "@authors: John Doe, Jane Smith";
pub const METADATA_DATE: &str = "@date: 2025-01-01";

// Unicode and special characters
pub const SPECIAL_CHARACTERS: &str = "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?";
pub const UNICODE_CONTENT: &str = "Unicode: 🦀 Rust, 📝 Markdown, 🚀 Fast parsing\nEmoji in `code 🎯`";

// Simple emoji headings
pub const EMOJI_H1_SIMPLE: &str = "# Heading with emoji 😀";
pub const EMOJI_H2_SIMPLE: &str = "## Subheading with emoji 😀";
pub const EMOJI_MULTIPLE: &str = "# Multiple emojis 😀🚀📝";

// Real-world small content using include_str! macro
/// Real-world small markdown document representing a typical quick start guide.
/// This tests realistic small-scale documentation patterns with proper formatting,
/// code blocks, and practical content structure.
pub const SMALL_REAL_WORLD_CONTENT: &str = include_str!("real_world_files/small_quick_start.md");

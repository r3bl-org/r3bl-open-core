// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Medium-complexity markdown inputs with structured content.
//!
//! These inputs test multi-paragraph documents, lists, headings, and code blocks:
//! - Multi-line text and headings
//! - Various list types (ordered, unordered, nested, checkboxes)
//! - Code blocks with different languages
//! - Mixed content and formatting patterns

// Multi-line content.
pub const MULTIPLE_LINES: &str = "First line\nSecond line\nThird line";
pub const HEADING_BASIC: &str = "# Main Heading\nSome content";
pub const MULTIPLE_HEADINGS: &str = "# H1\n## H2\n### H3\nContent";
pub const ALL_HEADING_LEVELS: &str = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6";

// Lists
pub const UNORDERED_LIST_SIMPLE: &str = "- Item 1\n- Item 2\n- Item 3";
pub const ORDERED_LIST_SIMPLE: &str = "1. First\n2. Second\n3. Third";
pub const NESTED_UNORDERED_LIST: &str =
    "- Top level\n  - Nested item\n    - Deep nested\n- Back to top";
pub const NESTED_ORDERED_LIST: &str =
    "1. First\n  2. Nested second\n     Content\n    3. Nested third\n2. Second top";
pub const CHECKBOXES: &str = "- [ ] Unchecked\n- [x] Checked\n- [X] Also checked";
pub const MIXED_LIST_TYPES: &str =
    "- Unordered item\n1. Ordered item\n- [ ] Checkbox item\n2. Another ordered";

// Code blocks
pub const CODE_BLOCK_BASH: &str = "```bash\necho \"Hello World\"\nls -la\n```";
pub const CODE_BLOCK_RUST: &str =
    "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```";
pub const CODE_BLOCK_NO_LANGUAGE: &str = "```\nSome code\nwithout language\n```";
pub const EMPTY_CODE_BLOCK: &str = "```rust\n```";

// Formatting patterns.
pub const FORMATTING_EDGE_CASES: &str =
    "*start bold*\n_start italic_\n`start code`\nend *bold*\nend _italic_\nend `code`";
pub const NESTED_FORMATTING: &str =
    "This is *bold with `code` inside*\nThis is _italic with `code` inside_";

// Edge cases
pub const EDGE_CASE_EMPTY_LINES: &str = "Line 1\n\n\nLine 2\n\n";
pub const EDGE_CASE_WHITESPACE_LINES: &str = "Line 1\n   \n\t\nLine 2";
pub const EDGE_CASE_TRAILING_SPACES: &str =
    "Line with trailing spaces   \nAnother line  ";

// Emoji positioning.
pub const EMOJI_START_MIDDLE_END: &str =
    "# 😀 Emoji at start\n## Middle 😀 emoji\n### Emoji at end 😀";

/// Real-world medium complexity markdown content from an actual blog post about Rust.
/// This tests typical blog post structure with headings, code blocks, lists, and links.
pub const BLOG_POST_DOCUMENT: &str = include_str!("real_world_files/medium_blog_post.md");

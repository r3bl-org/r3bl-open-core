// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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

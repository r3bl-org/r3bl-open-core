// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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

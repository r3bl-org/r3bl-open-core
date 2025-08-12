// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Invalid and edge case markdown inputs for testing parser robustness.
//!
//! These inputs test how both parsers handle malformed syntax, edge cases,
//! and boundary conditions. Both parsers should fail consistently on these inputs.

/// Malformed markdown syntax with invalid headings, unclosed code blocks, and invalid
/// checkboxes
pub const MALFORMED_SYNTAX: &str =
    "###not a heading\n```notclosed\n- [  invalid checkbox\n*not bold text";

/// Unclosed formatting markers that should be handled gracefully
pub const UNCLOSED_FORMATTING: &str =
    "This has *unclosed bold\nThis has _unclosed italic\nThis has `unclosed code";

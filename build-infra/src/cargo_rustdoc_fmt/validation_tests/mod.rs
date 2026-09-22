// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! End-to-end validation tests for `cargo-rustdoc-fmt`.
//!
//! # Testing Philosophy
//!
//! This test suite follows a pragmatic two-tier approach:
//!
//! ```text
//!           /\
//!          /  \    complete_file_tests.rs:
//!         /    \   • Component interaction tests
//!        / E2E  \  • Full file processing scenarios
//!       /  Tests \ • Parse CST → format → reconstruct
//!      /          \
//!     /------------\
//!    /     Unit     \    formatters/ (link.rs, table.rs, etc.):
//!   /      Tests     \   • Fast, isolated edge cases
//!  /                  \  • Empty input, no links, no tables
//! /────────────────────\
//! ```
//!
//! ## Tier 1: Unit Tests (in module files)
//! - **Location**: Embedded in [`formatters`] (`link.rs`, `table.rs`, `code_fence.rs`,
//!   `technical_term/`), [`cst`], [`processor`], and [`cli_arg`]
//! - **Purpose**: Test individual functions with simple inputs
//! - **Input**: Minimal hardcoded strings
//! - **Assertions**: Basic behavior (`contains`, `is_empty`, simple checks)
//! - **Speed**: Milliseconds
//! - **Examples**: Empty tables, single links, basic edge cases
//!
//! ## Tier 2: End-to-End Tests
//! - **Location**: [`complete_file_tests`]
//! - **Purpose**: Validate real-world usage with complete Rust files
//! - **Input**: Complete `.rs` files with rustdoc comments
//! - **Assertions**: Full pipeline behavior with realistic data
//! - **Speed**: Fast (< 1 second total)
//! - **Examples**: Files with tables, links, mixed comment types, unicode
//!
//! # Why This Approach?
//!
//! - **Simplicity**: Only two test layers, easy to understand and maintain
//! - **Real-world focus**: End-to-end tests use actual Rust files users will format
//! - **Fast feedback**: Unit tests catch basic bugs, E2E tests validate integration
//! - **No duplication**: Each component tested in isolation (units) and together (E2E)
//! - **Clear failures**: Unit test fails → function bug; E2E test fails → integration
//!   issue
//!
//! # What We Don't Test
//!
//! - **Isolated component validation**: Components are tested via unit tests (basic) and
//!   E2E tests (realistic). We don't need middle-layer "component-only" tests with
//!   realistic data, as those can behave differently than real usage.
//!
//! [`cli_arg`]: crate::cargo_rustdoc_fmt::cli_arg
//! [`complete_file_tests`]: complete_file_tests
//! [`cst`]: crate::cargo_rustdoc_fmt::cst
//! [`formatters`]: crate::cargo_rustdoc_fmt::formatters
//! [`processor`]: crate::cargo_rustdoc_fmt::processor

#[cfg(any(test, doc))]
mod complete_file_tests;

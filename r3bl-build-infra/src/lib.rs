// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![cfg_attr(not(test), deny(clippy::unwrap_in_result))]

//! # R3BL Build Infrastructure
//!
//! Build tools and utilities designed for R3BL projects, but usable in any Rust project.
//!
//! ## cargo-rustdoc-fmt
//!
//! A cargo subcommand that formats markdown tables and converts inline links to
//! reference-style links within Rust documentation comments (`///` and `//!`).
//!
//! ### Features
//!
//! - **Table Formatting**: Aligns markdown table columns for readability
//! - **Link Conversion**: Converts inline markdown links to reference-style links,
//!   keeping documentation cleaner
//! - **Workspace Support**: Process entire Rust workspaces or specific files
//! - **Check Mode**: Verify formatting without modifying files (useful for CI)
//! - **Selective Formatting**: Choose to format only tables, only links, or both
//! - **Git Integration**: Auto-detects changed files in git working tree
//!
//! ### Installation
//!
//! From a workspace containing this crate:
//!
//! ```bash
//! cargo install --path r3bl-build-infra
//! ```
//!
//! ### Usage Examples
//!
//! **Format git-changed files** (default - auto-detects staged/unstaged changes):
//! ```bash
//! cargo rustdoc-fmt
//! ```
//!
//! **Format entire workspace**:
//! ```bash
//! cargo rustdoc-fmt --workspace
//! ```
//!
//! **Format specific files**:
//! ```bash
//! cargo rustdoc-fmt src/lib.rs src/main.rs
//! ```
//!
//! **Format a directory**:
//! ```bash
//! cargo rustdoc-fmt src/
//! ```
//!
//! **Check formatting without modifying** (useful for CI):
//! ```bash
//! cargo rustdoc-fmt --check
//! ```
//!
//! **Only format tables** (skip link conversion):
//! ```bash
//! cargo rustdoc-fmt --tables-only
//! ```
//!
//! **Only convert links** (skip table formatting):
//! ```bash
//! cargo rustdoc-fmt --links-only
//! ```
//!
//! **Verbose output**:
//! ```bash
//! cargo rustdoc-fmt --verbose
//! ```
//!
//! **Combine options**:
//! ```bash
//! cargo rustdoc-fmt --check --verbose src/
//! ```
//!
//! ### What It Does
//!
//! #### Table Formatting
//!
//! Markdown tables in rustdoc comments are reformatted with consistent column widths.
//!
//! **Before:**
//! ```rust
//! //! | A | B |
//! //! |---|---|
//! //! | Short | Very Long Text |
//! ```
//!
//! **After:**
//! ```rust
//! //! | A     | B              |
//! //! |-------|----------------|
//! //! | Short | Very Long Text |
//! ```
//!
//! #### Link Conversion
//!
//! Inline markdown links are converted to reference-style links using the link text
//! as the reference identifier, reducing visual clutter in documentation.
//!
//! **Before:**
//! ```rust
//! //! See [docs](https://example.com) and [Rust](https://rust-lang.org).
//! ```
//!
//! **After:**
//! ```rust
//! //! See [docs] and [Rust].
//! //!
//! //! [docs]: https://example.com
//! //! [Rust]: https://rust-lang.org
//! ```
//!
//! ### Git Integration
//!
//! When run without arguments, `cargo-rustdoc-fmt` intelligently determines which files
//! to format:
//!
//! 1. **If there are staged/unstaged changes**: Formats only those changed files
//! 2. **If working tree is clean**: Formats files from the most recent commit
//! 3. **If not in a git repository**: Formats the entire workspace
//!
//! This makes it perfect for pre-commit hooks and development workflows.
//!
//! ### CI Integration
//!
//! Add to your continuous integration pipeline to enforce formatting standards:
//!
//! ```bash
//! cargo rustdoc-fmt --check
//! ```
//!
//! Exits with code 1 if formatting is needed, allowing CI to fail the build.
//!
//! **Example GitHub Actions step**:
//!
//! ```yaml
//! - name: Check rustdoc formatting
//!   run: cargo rustdoc-fmt --check --verbose
//! ```
//!
//! ### Architecture
//!
//! The project follows a multi-tool design pattern (similar to the `cmdr/` crate).
//! Currently implements `cargo-rustdoc-fmt`, with support for adding additional
//! build tools in the future without refactoring.
//!
//! **Module structure:**
//! - `src/lib.rs` - Library root
//! - `src/bin/cargo-rustdoc-fmt.rs` - Binary entry point
//! - `src/cargo_rustdoc_fmt/` - Tool implementation
//!   - `cli_arg.rs` - CLI argument parsing
//!   - `extractor.rs` - Extract rustdoc blocks from source
//!   - `table_formatter.rs` - Format markdown tables
//!   - `link_converter.rs` - Convert inline to reference-style links
//!   - `processor.rs` - Orchestrate file processing
//!   - `types.rs` - Type definitions
//!   - `ui_str.rs` - User-facing messages
//! - `src/common/` - Shared utilities
//!   - `git_utils.rs` - Git integration
//!   - `workspace_utils.rs` - Workspace discovery and file finding
//!
//! ### Implementation Notes
//!
//! Currently uses `pulldown-cmark` for markdown parsing. This will be migrated to
//! `r3bl_tui::md_parser` once table support is added to that parser, achieving full
//! R3BL infrastructure dogfooding.

// Attach all modules.
pub mod cargo_rustdoc_fmt;
pub mod common;

// Re-export commonly used items.
pub use common::*;

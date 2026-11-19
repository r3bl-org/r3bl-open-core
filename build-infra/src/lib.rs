// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

// Enforce strict error handling in production library code only. Tests and examples are
// allowed to use .unwrap() (workspace `Cargo.toml` config allows it). The `cfg_attr`
// ensures test code within the library can also use .unwrap() freely.
#![cfg_attr(not(test), deny(clippy::unwrap_in_result))]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

// XMARK: Prevent rustfmt from reformatting entire file.
// The `custom_inner_attributes` nightly feature enables `#![rustfmt::skip]` (replacing
// `#![cfg_attr(rustfmt, rustfmt_skip)]`).
#![feature(custom_inner_attributes)]
#![rustfmt::skip]

//! # Why R3BL?
//!
//! <img
//! src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true"
//! height="256px">
//!
//! <!-- R3BL TUI library & suite of apps focused on developer productivity -->
//!
//! <span style="color: #FD2F53; background: linear-gradient(90deg, #FD2F53, #285BFB); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent;">R3BL TUI library &amp; suite of apps focused on developer productivity</span>
//!
//! # Table of contents
//!
//! <!-- TOC -->
//!
//! - [Introduction](#introduction)
//! - [cargo-rustdoc-fmt](#cargo-rustdoc-fmt)
//!   - [Features](#features)
//!   - [Installation](#installation)
//!   - [Usage Examples](#usage-examples)
//!   - [What It Does](#what-it-does)
//!   - [Git Integration](#git-integration)
//!   - [CI Integration](#ci-integration)
//! - [Architecture](#architecture)
//! - [Changelog](#changelog)
//! - [Learn how these crates are built, provide
//!   feedback](#learn-how-these-crates-are-built-provide-feedback)
//!
//! <!-- /TOC -->
//!
//! # Introduction
//!
//! Build tools and utilities designed for R3BL projects, but usable in any Rust project.
//! This crate provides `cargo-rustdoc-fmt`: a cargo subcommand that formats Markdown
//! tables, converts inline links to reference-style links, auto-links known technical
//! terms, and normalizes code fence language tags in rustdoc comments.
//!
//! Please read the main [README.md] of the `r3bl-open-core` monorepo and workspace to get
//! a better understanding of the context in which this crate is meant to exist.
//!
//! # cargo-rustdoc-fmt
//!
//! A cargo subcommand that formats Markdown tables, converts inline links to
//! reference-style links, auto-links known technical terms, and normalizes code fence
//! language tags within Rust documentation comments (`///` and `//!`).
//!
//! ## Features
//!
//! - **Table Formatting**: Aligns Markdown table columns and pads separator cells for
//!   readability
//! - **Link Conversion**: Converts inline Markdown links to sorted bottom-of-block
//!   reference-style links, keeping documentation cleaner
//! - **Technical Term Linking**: Automatically upgrades backticked technical terms
//!   (e.g., `` `ANSI` ``, `` `CSI` ``, `` `PTY` ``) to reference-style links backed by the
//!   embedded dictionary
//! - **Code Fence Normalization**: Strips redundant `rust` language tags from opening
//!   code fences (` ```rust ` → ` ``` ` and ` ```rust,<attr> ` → ` ```<attr> `) since
//!   rustdoc treats unlabeled code fences as Rust by default
//! - **Line-Range Formatting**: Surgically format only specific 1-based line ranges
//!   (`--lines` / `--lines-force`) while leaving the rest of the file untouched
//! - **Workspace Support**: Process entire Rust workspaces or specific files
//! - **Check & Dry-Run Modes**: Verify formatting without modifying files (`--check` for
//!   CI, `--dry-run` to preview which files would be processed)
//! - **Selective Formatting**: Choose to format only tables (`--tables-only`), only links
//!   (`--links-only`), only terms (`--terms-only`), only code fences (`--fences-only`),
//!   or all passes
//! - **Skip Controls**: Exclude files via in-file `// rustdoc-fmt: skip` directives or
//!   `--skip-folder-matches`, and optionally skip the trailing `cargo fmt` pass with
//!   `--skip-cargo-fmt`
//! - **Git Integration**: Auto-detects changed files in git working tree
//!
//! ## Installation
//!
//! From [crates.io]:
//!
//! ```bash
//! cargo install r3bl-build-infra
//! ```
//!
//! Or from source (in a workspace containing this crate):
//!
//! ```bash
//! cargo install --path build-infra --force
//! ```
//!
//! ## Usage Examples
//!
//! **Format git-changed files** (default: auto-detects staged and unstaged changes):
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
//! **Surgically format specific 1-based line ranges** (bypassing `// rustdoc-fmt: skip`):
//! ```bash
//! cargo rustdoc-fmt --lines-force 400-445 src/lib.rs
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
//! **Show which files would be processed without making changes**:
//! ```bash
//! cargo rustdoc-fmt --dry-run
//! ```
//!
//! **Selective formatting passes**:
//! ```bash
//! cargo rustdoc-fmt --tables-only
//! cargo rustdoc-fmt --links-only
//! cargo rustdoc-fmt --terms-only
//! cargo rustdoc-fmt --fences-only
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
//! ## What It Does
//!
//! **Table Formatting**
//!
//! Markdown tables in rustdoc comments are reformatted with consistent column widths and
//! space-padded separator cells.
//!
//! **Before:**
//! ```
//! //! | A | B |
//! //! |---|---|
//! //! | Short | Very Long Text |
//! ```
//!
//! **After:**
//! ```
//! //! | A     | B              |
//! //! | ----- | -------------- |
//! //! | Short | Very Long Text |
//! ```
//!
//! **Link Conversion**
//!
//! Inline Markdown links are converted to sorted reference-style links at the bottom of
//! the doc block using the link text as the reference identifier, reducing visual clutter
//! in documentation.
//!
//! **Before:**
//! ```
//! //! See [docs](https://example.com) and [Rust](https://rust-lang.org).
//! ```
//!
//! **After:**
//! ```
//! //! See [docs] and [Rust].
//! //!
//! //! [docs]: https://example.com
//! //! [Rust]: https://rust-lang.org
//! ```
//!
//! **Technical Term Linking**
//!
//! Known backticked technical terms are automatically linked using the embedded
//! technical term dictionary, keeping existing link targets fresh and canonical.
//!
//! **Before:**
//! ```
//! //! Uses `ANSI` escape sequences over a `PTY`.
//! ```
//!
//! **After:**
//! ```
//! //! Uses [`ANSI`] escape sequences over a [`PTY`].
//! //!
//! //! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! //! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! ```
//!
//! **Code Fence Normalization**
//!
//! Redundant `rust` language tags on opening code fences in rustdoc comments are stripped
//! (` ```rust ` → ` ``` ` and ` ```rust,<attr> ` → ` ```<attr> `, such as
//! ` ```rust,no_run ` → ` ```no_run `) since rustdoc treats unlabeled code fences as Rust
//! by default.
//!
//! ## Git Integration
//!
//! When run without arguments, `cargo-rustdoc-fmt` intelligently determines which files
//! to format:
//!
//! 1. **If there are staged or unstaged changes**: Formats only those changed files
//! 2. **If working tree is clean**: Formats files from the most recent commit
//! 3. **If not in a git repository**: Formats the entire workspace
//!
//! This makes it perfect for pre-commit hooks and development workflows.
//!
//! ## CI Integration
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
//! # Architecture
//!
//! `cargo-rustdoc-fmt` is built around a lossless two-level Concrete Syntax Tree (CST)
//! using direct byte/slice scanners (zero regex dependencies):
//!
//! ```text
//! .rs source file
//!   └── Level 1: SourceFileCst (preserves LineEndings & TrailingNewline)
//!         ├── FileChunk::NonDoc        ──► preserved 100% byte-for-byte
//!         └── FileChunk::DocBlock      ──► Level 2: DocBlockCst
//!               ├── DocNode::Paragraph             (link conversion & term linking)
//!               ├── DocNode::Table                 (GFM column alignment & padding)
//!               ├── DocNode::CodeFence             (fence tag normalization; body protected)
//!               ├── DocNode::HtmlComment           (protected: e.g., <!-- TOC --> blocks)
//!               ├── DocNode::ReferenceDefinitions  (collected, deduplicated & sorted at bottom)
//!               └── DocNode::BlankLine
//! ```
//!
//! - **Byte-for-byte preservation**: Non-doc Rust code, fenced code block bodies, inline
//!   code spans, HTML comment blocks, indentation prefixes, and `LF`/`CRLF` line endings
//!   are never altered.
//! - **Surgical line-range formatting**: Every CST node tracks its exact 0-based
//!   `RangeInclusive<CRow>` line span in the source file, allowing `--lines` and
//!   `--lines-force` to format only the nodes intersecting the requested range while
//!   emitting all other nodes verbatim from their original raw lines.
//!
//! # Changelog
//!
//! Please check out the [changelog] to see how the crate has evolved over time.
//!
//! # Learn how these crates are built, provide feedback
//!
//! To learn how we built this crate, please take a look at the following resources.
//! - If you like consuming video content, here's our [YT channel]. Please consider
//!   [subscribing].
//! - If you like consuming written content, here's our developer [site].
//!
//! [changelog]:
//!     https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl-build-infra
//! [crates.io]: https://crates.io/crates/r3bl-build-infra
//! [README.md]: https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md
//! [site]: https://developerlife.com/
//! [subscribing]:
//!     https://www.youtube.com/channel/UCMcsxfCwzwDevc3NRqFgfEg?sub_confirmation=1
//! [YT channel]: https://www.youtube.com/@developerlifecom

// Attach all modules.
pub mod cargo_rustdoc_fmt;
pub mod common;

// Re-export commonly used items.
pub use cargo_rustdoc_fmt::*;
pub use common::*;

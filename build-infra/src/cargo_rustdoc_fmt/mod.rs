// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Rust documentation formatting tools.
//!
//! This module provides functionality to format Markdown tables, convert inline links
//! to reference-style links, auto-link known technical terms, and normalize code fence
//! language tags within Rust documentation comments.
//!
//! # Architecture
//!
//! Uses a lossless two-level Concrete Syntax Tree ([`SourceFileCst`] and [`DocBlockCst`])
//! to partition Rust source files and format Markdown tables, inline links, technical
//! terms, and code fences within rustdoc comments (supporting both whole-file and
//! surgical `--lines` / `--lines-force` formatting) while preserving untouched code and
//! comments byte-for-byte.

pub mod cli_arg;
pub mod constants;
pub mod cst;
pub mod formatters;
pub mod processor;
pub mod types;
pub mod ui_str;

#[cfg(test)]
pub mod validation_tests;

// Re-export public API for flat module interface.
pub use cli_arg::*;
pub use constants::*;
pub use cst::*;
pub use formatters::*;
pub use processor::*;
pub use types::*;
pub use ui_str::*;

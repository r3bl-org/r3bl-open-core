// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Rust documentation formatting tools.
//!
//! This module provides functionality to format markdown tables and
//! convert links within Rust documentation comments.
//!
//! # Current Implementation
//!
//! Uses `pulldown-cmark` for markdown parsing. This will be migrated to
//! `r3bl_tui::md_parser` once table support is added to that parser.

pub mod cli_arg;
pub mod content_protector;
pub mod extractor;
pub mod link_converter;
pub mod processor;
pub mod table_formatter;
pub mod types;
pub mod ui_str;

#[cfg(test)]
pub mod validation_tests;

// Re-export public API for flat module interface (like cmdr/).
pub use cli_arg::*;
pub use content_protector::*;
pub use extractor::*;
pub use link_converter::*;
pub use processor::*;
pub use table_formatter::*;
pub use types::*;
pub use ui_str::*;

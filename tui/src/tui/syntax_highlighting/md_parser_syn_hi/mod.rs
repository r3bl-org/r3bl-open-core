// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module holds the integration or glue code that ties together:
//! 1. [`crate::md_parser`] - Responsible for parsing Markdown into a
//!    [`crate::MdDocument`] data structure.
//! 2. [`crate::syntax_highlighting`] - Responsible for converting a [`crate::MdDocument`]
//!    into a list of tuples of [`crate::TuiStyle`] and [String].
//! 3. [`crate::editor`] - Responsible for displaying the [`crate::MdDocument`] to the
//!    user.

// Attach.
pub mod md_parser_stylesheet;
pub mod md_parser_syn_hi_impl;

// Re-export.
pub use md_parser_stylesheet::*;
pub use md_parser_syn_hi_impl::*;

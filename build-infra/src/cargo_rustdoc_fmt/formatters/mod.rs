// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Formatting transformations that mutate [`DocBlockCst`] and [`DocNode`] in-place.
//!
//! Each formatter implements one opinionated R3BL rustdoc rule and marks modified CST
//! nodes with [`FormatStatus::Modified`]:
//! - [`format_table_node`]: Aligns Markdown table columns and separator dashes.
//! - [`convert_links_in_doc_block`]: Rewrites inline `[text](url)` links into
//!   reference-style `[text]` links and consolidates sorted `[label]: target` definitions
//!   at the bottom of the doc block.
//! - [`link_terms_in_doc_block`]: Upgrades known technical terms from
//!   [`TechnicalTermDictionary`] to linked `` [`Term`] `` form with canonical reference
//!   definitions.
//! - [`normalize_code_fence_node`]: Strips redundant `rust` language tags from opening
//!   code fences (```` ```rust ```` -> ```` ``` ````).
//!
//! [`DocBlockCst`]: crate::cargo_rustdoc_fmt::DocBlockCst
//! [`DocNode`]: crate::cargo_rustdoc_fmt::DocNode
//! [`FormatStatus::Modified`]: crate::cargo_rustdoc_fmt::FormatStatus::Modified

mod code_fence;
mod link;
mod table;
mod technical_term;

pub use code_fence::*;
pub use link::*;
pub use table::*;
#[cfg(test)]
pub(crate) use technical_term::link_known_terms_for_test;
pub use technical_term::*;

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Upgrades known technical terms in rustdoc blocks to backticked+linked form.
//!
//! For each known term in the registry, this module:
//! 1. Finds plain-text or backticked-only occurrences across paragraphs and tables
//! 2. Upgrades ALL occurrences to `` [`Term`] `` form while protecting code fences,
//!    inline code spans, existing links, images, and TOC blocks
//! 3. Reconciles, sorts, and positions canonical link target definitions at the bottom of
//!    the doc block

mod link_terms_entry_point;
mod markdown_spans;
mod ref_defs;
mod technical_term_dictionary;
mod upgrade_term;

#[cfg(test)]
pub(crate) use link_terms_entry_point::link_known_terms_for_test;
pub use link_terms_entry_point::*;
pub use technical_term_dictionary::*;

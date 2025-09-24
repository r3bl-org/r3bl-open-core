// Copyright (c) 2024 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unified trait system for grapheme-aware string and document operations.
//!
//! This module provides traits that enable consistent handling of grapheme clusters
//! across different string and document implementations in this codebase. These traits
//! serve as a common interface for working with Unicode text in a grapheme-cluster-aware
//! manner.
//!
//! ## Purpose
//!
//! The unifying traits in this module are designed to provide:
//!
//! - **Future extensibility**: If additional document storage types are added beyond
//!   [`ZeroCopyGapBuffer`], they can implement these traits for seamless integration.
//! - **Generic document operations**: New code can work with document-like structures
//!   generically through the [`GraphemeDoc`] and [`GraphemeDocMut`] traits, without being
//!   tied to specific implementations.
//! - **Cross-implementation interoperability**: Enables potential interoperability
//!   between [`ZeroCopyGapBuffer`] and [`Vec<GCStringOwned>`] if needed, allowing code to
//!   work with either storage format.
//!
//! ## Core Traits
//!
//! - [`GraphemeString`]: Single-line grapheme-aware string operations.
//! - [`GraphemeDoc`]: Multi-line document read operations.
//! - [`GraphemeDocMut`]: Multi-line document mutation operations.
//!
//! ## Implementation Status
//!
//! Currently, the editor codebase primarily uses [`ZeroCopyGapBuffer`] as a
//! concrete type for performance and type safety reasons. These traits provide an
//! abstraction layer for future flexibility without requiring immediate migration of
//! existing code.
//!
//! [`ZeroCopyGapBuffer`]: crate::ZeroCopyGapBuffer

// Attach
pub mod grapheme_doc;
pub mod grapheme_string;
pub mod grapheme_string_owned_ext;
pub mod seg_content;

// Re-export
pub use grapheme_doc::*;
pub use grapheme_string::*;
pub use grapheme_string_owned_ext::*;
pub use seg_content::*;

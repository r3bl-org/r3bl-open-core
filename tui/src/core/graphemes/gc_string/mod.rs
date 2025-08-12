// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`GCStringOwned`] implementation for Unicode grapheme cluster handling.
//!
//! This module contains the [`GCStringOwned`] type that provides Unicode-safe string
//! operations with grapheme cluster support. It's used throughout the TUI system
//! for text formatting, clipping, and rendering operations.
//!
//! See the [module docs](crate::graphemes) for comprehensive information about Unicode
//! handling, grapheme clusters, and the three types of indices used in this system.

// Submodules
pub mod document;
pub mod owned;

// Re-exports
pub use document::*;
pub use owned::*;

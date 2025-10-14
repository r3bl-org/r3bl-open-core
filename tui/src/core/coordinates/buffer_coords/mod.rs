// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! 0-based buffer coordinates for internal application logic.
//!
//! This module provides coordinate types used for:
//! - Indexing into [`OffscreenBuffer`] and [`ZeroCopyGapBuffer`]
//! - Internal TUI framework logic
//! - Crossterm terminal operations (0-based, converts to [`u16`])
//!
//! All types in this module use 0-based indexing and wrap [`ChUnit`] ([`u16`]).
//!
//! # Core Types
//!
//! **Generic coordinate types**:
//! - [`Index`]: Generic 0-based position type
//! - [`Length`]: Generic 1-based size/count type
//!
//! **Concrete index types (0-based positions)**:
//! - [`ColIndex`]: Column position (0-based)
//! - [`RowIndex`]: Row position (0-based)
//!
//! **Concrete dimension types (1-based sizes)**:
//! - [`ColWidth`]: Column width/count (1-based)
//! - [`RowHeight`]: Row height/count (1-based)
//!
//! **Composite types**:
//! - [`Pos`]: Position combining row and column indices
//! - [`Size`]: Dimension combining width and height
//! - [`CaretRaw`], [`CaretScrAdj`]: Cursor position with scroll adjustment semantics
//! - [`ScrOfs`]: Scroll offset (semantically a position)
//!
//! # Macros
//!
//! This module includes declarative macros for generating boilerplate implementations:
//! - [`generate_index_type_impl!`]: For index types (0-based)
//! - [`generate_length_type_impl!`]: For length/dimension types (1-based)
//!
//! [`ChUnit`]: crate::ChUnit
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`ZeroCopyGapBuffer`]: crate::ZeroCopyGapBuffer
//! [`generate_index_type_impl!`]: crate::generate_index_type_impl
//! [`generate_length_type_impl!`]: crate::generate_length_type_impl

// Attach source files.
pub mod index_and_length_impl_macros;
pub mod index;
pub mod length;
pub mod caret;
pub mod col_index;
pub mod col_width;
pub mod pos;
pub mod row_height;
pub mod row_index;
pub mod scr_ofs;
pub mod size;

// Re-export types and constructors.
pub use index::*;
pub use length::*;
pub use caret::*;
pub use col_index::*;
pub use col_width::*;
pub use pos::*;
pub use row_height::*;
pub use row_index::*;
pub use scr_ofs::*;
pub use size::*;

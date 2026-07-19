// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! 64-bit [`Canvas`] coordinates for document buffer storage and memory addressing.
//!
//! This module provides strongly-typed coordinate newtypes used for:
//! - Memory buffer storage addressing ([`OfsBuf`] and [`ZeroCopyGapBuffer`])
//! - Continuous 2D offscreen [`Canvas`] coordinates (64-bit [`usize`])
//! - Decoupling storage capacity from 16-bit viewport rendering limits
//!
//! All types in this module wrap [`usize`] storage coordinates.
//!
//! # Core Types
//!
//! **Generic coordinate types**:
//! - [`CIndex`]: Generic 0-based index type in [`Canvas`] storage space
//! - [`CLength`]: Generic 1-based size/count type in [`Canvas`] storage space
//!
//! **Concrete index types (0-based positions)**:
//! - [`CCol`]: Absolute column position (0-based)
//! - [`CRow`]: Absolute row/line position (0-based)
//!
//! **Concrete dimension types (1-based sizes)**:
//! - [`CWidth`]: Column display width/extent (1-based)
//! - [`CHeight`]: Row count/vertical extent (1-based)
//!
//! **Composite types**:
//! - [`CPos`]: 2D absolute position combining column and row indices
//! - [`CSize`]: 2D dimension combining width and height
//! - [`CBoundingBox`]: 2D rectangular spatial boundary on the [`Canvas`]
//!
//! # Macros
//!
//! This module includes declarative macros for generating boilerplate implementations:
//! - [`generate_canvas_index_type_impl!`]: For index types (0-based)
//! - [`generate_canvas_length_type_impl!`]: For length/dimension types (1-based)
//!
//! [`Canvas`]: mod@crate::core::coordinates::canvas
//! [`CBoundingBox`]: crate::CBoundingBox
//! [`CCol`]: crate::CCol
//! [`CHeight`]: crate::CHeight
//! [`CIndex`]: crate::CIndex
//! [`CLength`]: crate::CLength
//! [`CPos`]: crate::CPos
//! [`CRow`]: crate::CRow
//! [`CSize`]: crate::CSize
//! [`CWidth`]: crate::CWidth
//! [`generate_canvas_index_type_impl!`]: crate::generate_canvas_index_type_impl
//! [`generate_canvas_length_type_impl!`]: crate::generate_canvas_length_type_impl
//! [`OfsBuf`]: crate::tui::OfsBuf
//! [`ZeroCopyGapBuffer`]: crate::ZeroCopyGapBuffer

#![rustfmt::skip]

// Attach source files.
mod c_bounding_box;
mod c_col;
mod c_height;
mod c_index;
mod c_length;
mod c_pos;
mod c_row;
mod c_size;
mod c_width;
mod macros;

// Re-export types and constructors.
pub use c_bounding_box::*;
pub use c_col::*;
pub use c_height::*;
pub use c_index::*;
pub use c_length::*;
pub use c_pos::*;
pub use c_row::*;
pub use c_size::*;
pub use c_width::*;

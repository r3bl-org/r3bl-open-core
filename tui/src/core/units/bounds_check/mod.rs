// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Bounds checking utilities for terminal user interface index validation.
//!
//! This module provides a comprehensive system for validating index positions against
//! various bounds, specifically designed for TUI applications where precise position
//! validation is crucial for rendering and user interaction.
//!
//! # Core Concepts
//!
//! The module implements two distinct paradigms for bounds checking:
//!
//! ## Array-Style Bounds Checking (`check_overflows`)
//!
//! Traditional array bounds checking where an index is valid if it's less than the
//! maximum length. Returns [`Within`](BoundsOverflowStatus::Within) for safe access or
//! [`Overflowed`](BoundsOverflowStatus::Overflowed) when bounds are exceeded.
//!
//! ## Content Position Checking (`check_content_position`)
//!
//! Content-aware position checking essential for text editing and cursor positioning.
//! Returns [`ContentPositionStatus`] variants indicating the relationship between an
//! index and content boundaries, including start, within, end, and beyond positions.
//!
//! # Type System
//!
//! The bounds checking system is built around two main type categories that ensure
//! type safety and prevent incorrect comparisons:
//!
//! ## Index Types (0-based position indicators)
//!
//! Types that implement [`IndexMarker`] represent positions within content, starting
//! from 0:
//! - [`Index`] - Generic 0-based position
//! - [`RowIndex`] - Row position in a terminal grid
//! - [`ColIndex`] - Column position in a terminal grid
//!
//! ## Length Types (1-based size measurements)
//!
//! Types that implement [`LengthMarker`] represent sizes or extents, starting from 1:
//! - [`Length`] - Generic 1-based size
//! - [`RowHeight`] - Height of terminal content
//! - [`ColWidth`] - Width of terminal content
//!
//! ## Type Safety Guarantees
//!
//! The trait system enforces several important constraints:
//! - Only index types can be bounds-checked against length types
//! - Each length type has a corresponding index type via [`LengthMarker::IndexType`]
//! - Automatic conversion between compatible types via [`LengthMarker::convert_to_index`]
//! - Prevents accidental comparisons between incompatible types (e.g., row vs column)
//!
//! # Key Components
//!
//! - [`BoundsCheck`] trait: Core functionality for both checking paradigms
//! - [`BoundsOverflowStatus`] enum: Results for array-style bounds checking
//! - [`ContentPositionStatus`] enum: Results for content position checking
//! - [`LengthMarker::is_overflowed_by`] method: Convenient overflow checking from length
//!   perspective ("Does this length get overflowed by this index?")
//! - [`IndexMarker::overflows`] method: Convenient overflow checking from index
//!   perspective ("Does this index overflow this length?")
//!
//! ## Implementations
//!
//! The module provides a single generic implementation of [`BoundsCheck`] that works
//! with any index type implementing [`IndexMarker`] and any length type implementing
//! [`LengthMarker`]. This eliminates code duplication and ensures consistent behavior
//! across all unit types.
//!
//! Individual types implement the required marker traits in their respective modules:
//! - [`UnitCompare`] - Enables numeric conversions for comparison operations
//! - [`IndexMarker`] - Identifies 0-based position types
//! - [`LengthMarker`] - Identifies 1-based size types with index correspondence
//!
//! # Usage Examples
//!
//! ```
//! use r3bl_tui::{BoundsCheck, ContentPositionStatus, IndexMarker, LengthMarker, idx, len};
//!
//! let content_length = len(10);
//! let cursor_pos = idx(8);
//!
//! // Content position checking for text editing
//! match cursor_pos.check_content_position(content_length) {
//!     ContentPositionStatus::AtStart => println!("Cursor at start"),
//!     ContentPositionStatus::Within => println!("Cursor on content"),
//!     ContentPositionStatus::AtEnd => println!("Cursor at end"),
//!     ContentPositionStatus::Beyond => println!("Invalid position"),
//! }
//!
//! // Array-style overflow checking - two equivalent approaches:
//!
//! // Approach 1: Length perspective - "Does this length get overflowed by this index?"
//! if !content_length.is_overflowed_by(cursor_pos) {
//!     // Safe to access content[cursor_pos]
//! }
//!
//! // Approach 2: Index perspective - "Does this index overflow this length?"
//! if !cursor_pos.overflows(content_length) {
//!     // Safe to access content[cursor_pos]
//! }
//! ```
//!
//! [`RowIndex`]: crate::RowIndex
//! [`ColIndex`]: crate::ColIndex
//! [`RowHeight`]: crate::RowHeight
//! [`ColWidth`]: crate::ColWidth
//! [`Index`]: crate::Index
//! [`Length`]: crate::Length
//! [`dimens`]: crate::dimens

// Attach.
pub mod bounds_check_core;
pub mod content_position_ext;
pub mod length_and_index_markers;
pub mod range_ext;
pub mod result_enums;

// Re-export.
pub use bounds_check_core::*;
pub use content_position_ext::*;
pub use length_and_index_markers::*;
pub use range_ext::*;
pub use result_enums::*;

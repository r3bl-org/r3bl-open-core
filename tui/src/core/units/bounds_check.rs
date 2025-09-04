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
//! maximum length. This is used when checking if an index can access existing content:
//! - Index `i` is valid for length `n` if `i < n`
//! - Index `i` overflows if `i >= n`
//! - Used for: accessing array elements, validating content indices
//!
//! > See the [`crate::dimens_bounds_check_impl`] and [`crate::unit_bounds_check_impl`]
//! > modules for trait implementations. The `dimens` module contains implementations
//! > for `RowIndex`, `ColIndex` types with `RowHeight`, `ColWidth`, etc. The `units`
//! > module contains implementations for the generic `Index` and `Length` types.
//!
//! ## Content Position Checking (`check_content_position`)
//!
//! Content-aware position checking that distinguishes between positions within content,
//! at content boundaries, and beyond content. This is essential for text editing and
//! cursor positioning:
//! - `Within`: Index points to existing content (`i < length`)
//! - `Boundary`: Index is at the end boundary (`i == length`) - valid for
//!   cursor/insertion
//! - `Beyond`: Index exceeds content boundaries (`i > length`) - invalid position
//!
//! > See the [`crate::dimens_bounds_check_impl`] and [`crate::unit_bounds_check_impl`]
//! > modules for trait implementations. The `dimens` module contains implementations
//! > for `RowIndex`, `ColIndex` types with `RowHeight`, `ColWidth`, etc. The `units`
//! > module contains implementations for the generic `Index` and `Length` types.
//!
//! # Key Components
//!
//! - [`BoundsCheck`] trait: Core functionality for both checking paradigms
//! - [`BoundsStatus`] enum: Results for array-style bounds checking
//! - [`PositionStatus`] enum: Results for content position checking
//! - [`check_overflows!`](crate::check_overflows) macro: Concise syntax for overflow
//!   checking
//!
//! # Usage Patterns
//!
//! ## Text Editor Cursor Validation
//! ```
//! use r3bl_tui::{BoundsCheck, PositionStatus, idx, len};
//!
//! let line_length = len(10);
//! let cursor_pos = idx(8);
//!
//! match cursor_pos.check_content_position(line_length) {
//!     PositionStatus::Within => println!("Cursor on character"),
//!     PositionStatus::Boundary => println!("Cursor at end of line"),
//!     PositionStatus::Beyond => println!("Invalid cursor position"),
//! }
//! ```
//!
//! ## Array Access Validation
//! ```
//! use r3bl_tui::{check_overflows, BoundsCheck, idx, len};
//!
//! let data_length = len(5);
//! let index = idx(3);
//!
//! if !check_overflows!(index, data_length) {
//!     // Safe to access data[index]
//! }
//! ```
//!
//! ## Terminal Coordinate Validation
//! ```
//! use r3bl_tui::{BoundsCheck, BoundsStatus, RowIndex, RowHeight};
//!
//! let row = RowIndex::new(15);
//! let screen_height = RowHeight::new(20);
//!
//! if row.check_overflows(screen_height) == BoundsStatus::Within {
//!     // Safe to render at this row
//! }
//! ```

/// Result of array-style bounds checking operations.
///
/// Used with [`BoundsCheck::check_overflows`] to determine if an index can safely
/// access array content. See the [module documentation](self) for details on the
/// bounds checking paradigms.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, BoundsStatus, idx, len};
///
/// let index = idx(5);
/// let length = len(10);
/// assert_eq!(index.check_overflows(length), BoundsStatus::Within);
///
/// let large_index = idx(10);
/// assert_eq!(large_index.check_overflows(length), BoundsStatus::Overflowed);
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BoundsStatus {
    /// Indicates that an index is within the bounds of a length or another index.
    Within,
    /// Indicates that an index has overflowed the bounds of a length or another index.
    Overflowed,
}

/// Concise macro for array-style bounds overflow checking.
///
/// Returns `true` if the index overflows the bounds (`index >= max`), `false` if within
/// bounds. Equivalent to `index.check_overflows(max) == BoundsStatus::Overflowed`.
///
/// See the [module documentation](self) for the difference between overflow checking
/// and content position checking.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{check_overflows, idx, len};
///
/// if check_overflows!(idx(5), len(10)) {
///     // Handle overflow case
/// }
///
/// // Replaces verbose syntax:
/// # use r3bl_tui::{BoundsCheck, BoundsStatus, ColIndex, ColWidth};
/// # let index = ColIndex::new(5);
/// # let width = ColWidth::new(10);
/// // if index.check_overflows(width) == BoundsStatus::Overflowed { ... }
/// if check_overflows!(index, width) { /* ... */ }
/// ```
#[macro_export]
macro_rules! check_overflows {
    ($index:expr, $max:expr) => {
        $index.check_overflows($max) == $crate::BoundsStatus::Overflowed
    };
}

/// Core trait for index bounds validation in TUI applications.
///
/// Provides both array-style bounds checking and content position checking.
/// See the [module documentation](self) for detailed explanations of both paradigms.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, BoundsStatus, RowIndex, RowHeight};
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(row_index.check_overflows(height), BoundsStatus::Overflowed);
/// ```
pub trait BoundsCheck<OtherType> {
    /// Performs array-style bounds checking.
    ///
    /// Returns `BoundsStatus::Within` if the index can safely access content,
    /// `BoundsStatus::Overflowed` if the index would exceed array bounds.
    ///
    /// See the [module documentation](self) for detailed explanation of bounds checking.
    fn check_overflows(&self, max: OtherType) -> BoundsStatus;

    /// Performs content position checking.
    ///
    /// Returns `PositionStatus` indicating whether the index is within content,
    /// at a content boundary, or beyond content boundaries.
    ///
    /// See the [module documentation](self) for detailed explanation of content position
    /// checking.
    fn check_content_position(&self, content_length: OtherType) -> PositionStatus;
}

/// Result of content position checking operations.
///
/// Used with [`BoundsCheck::check_content_position`] to determine the relationship
/// between an index and content boundaries. Essential for text editing and cursor
/// positioning where boundary conditions require different handling.
///
/// See the [module documentation](self) for detailed explanation of content position
/// checking and use cases for each variant.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, PositionStatus, idx, len};
///
/// let content_length = len(5);
///
/// assert_eq!(idx(3).check_content_position(content_length), PositionStatus::Within);
/// assert_eq!(idx(5).check_content_position(content_length), PositionStatus::Boundary);
/// assert_eq!(idx(7).check_content_position(content_length), PositionStatus::Beyond);
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PositionStatus {
    /// Index points to existing content (`index < length`).
    Within,

    /// Index is at the content boundary (`index == length`), valid for cursor/insertion.
    Boundary,

    /// Index exceeds content boundaries (`index > length`), requires correction.
    Beyond,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{idx, len};

    #[test]
    fn test_bounds_status_equality() {
        assert_eq!(BoundsStatus::Within, BoundsStatus::Within);
        assert_eq!(BoundsStatus::Overflowed, BoundsStatus::Overflowed);
        assert_ne!(BoundsStatus::Within, BoundsStatus::Overflowed);
    }

    #[test]
    fn test_bounds_status_copy() {
        let status1 = BoundsStatus::Within;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = BoundsStatus::Overflowed;
        let status4 = status3;
        assert_eq!(status3, status4);
    }

    #[test]
    fn test_bounds_status_debug() {
        assert_eq!(format!("{:?}", BoundsStatus::Within), "Within");
        assert_eq!(format!("{:?}", BoundsStatus::Overflowed), "Overflowed");
    }

    #[test]
    fn test_position_status_equality() {
        assert_eq!(PositionStatus::Within, PositionStatus::Within);
        assert_eq!(PositionStatus::Boundary, PositionStatus::Boundary);
        assert_eq!(PositionStatus::Beyond, PositionStatus::Beyond);
        assert_ne!(PositionStatus::Within, PositionStatus::Boundary);
        assert_ne!(PositionStatus::Boundary, PositionStatus::Beyond);
        assert_ne!(PositionStatus::Within, PositionStatus::Beyond);
    }

    #[test]
    fn test_position_status_copy() {
        let status1 = PositionStatus::Within;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = PositionStatus::Boundary;
        let status4 = status3;
        assert_eq!(status3, status4);

        let status5 = PositionStatus::Beyond;
        let status6 = status5;
        assert_eq!(status5, status6);
    }

    #[test]
    fn test_position_status_debug() {
        assert_eq!(format!("{:?}", PositionStatus::Within), "Within");
        assert_eq!(format!("{:?}", PositionStatus::Boundary), "Boundary");
        assert_eq!(format!("{:?}", PositionStatus::Beyond), "Beyond");
    }

    #[test]
    fn test_check_overflows_macro() {
        use crate::{ColIndex, ColWidth};

        // Test basic cases with Index/Length
        assert!(!check_overflows!(idx(1), len(3)), "Within bounds");
        assert!(check_overflows!(idx(3), len(3)), "At boundary");
        assert!(check_overflows!(idx(5), len(3)), "Beyond bounds");
        assert!(
            !check_overflows!(idx(0), len(0)),
            "Empty collection edge case"
        );

        // Test with typed indices
        assert!(
            !check_overflows!(ColIndex::new(5), ColWidth::new(10)),
            "Typed indices within bounds"
        );
        assert!(
            check_overflows!(ColIndex::new(10), ColWidth::new(10)),
            "Typed indices at boundary"
        );

        // Verify macro matches direct method calls
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            assert_eq!(
                check_overflows!(index, length),
                index.check_overflows(length) == BoundsStatus::Overflowed,
                "Macro should match direct method for index {index_val} and length {length_val}"
            );
        }
    }

    #[test]
    fn test_check_content_position_basic() {
        let content_length = len(5);

        // Within content
        assert_eq!(
            idx(0).check_content_position(content_length),
            PositionStatus::Within
        );
        assert_eq!(
            idx(2).check_content_position(content_length),
            PositionStatus::Within
        );
        assert_eq!(
            idx(4).check_content_position(content_length),
            PositionStatus::Within
        );

        // At boundary
        assert_eq!(
            idx(5).check_content_position(content_length),
            PositionStatus::Boundary
        );

        // Beyond content
        assert_eq!(
            idx(6).check_content_position(content_length),
            PositionStatus::Beyond
        );
        assert_eq!(
            idx(10).check_content_position(content_length),
            PositionStatus::Beyond
        );
    }

    #[test]
    fn test_check_content_position_edge_cases() {
        // Zero-length content
        let zero_length = len(0);
        assert_eq!(
            idx(0).check_content_position(zero_length),
            PositionStatus::Boundary
        );
        assert_eq!(
            idx(1).check_content_position(zero_length),
            PositionStatus::Beyond
        );

        // Single element content
        let single_length = len(1);
        assert_eq!(
            idx(0).check_content_position(single_length),
            PositionStatus::Within
        );
        assert_eq!(
            idx(1).check_content_position(single_length),
            PositionStatus::Boundary
        );
        assert_eq!(
            idx(2).check_content_position(single_length),
            PositionStatus::Beyond
        );
    }

    #[test]
    fn test_check_content_position_with_typed_indices() {
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        // Test with ColIndex/ColWidth
        let col_width = ColWidth::new(3);
        assert_eq!(
            ColIndex::new(0).check_content_position(col_width),
            PositionStatus::Within
        );
        assert_eq!(
            ColIndex::new(2).check_content_position(col_width),
            PositionStatus::Within
        );
        assert_eq!(
            ColIndex::new(3).check_content_position(col_width),
            PositionStatus::Boundary
        );
        assert_eq!(
            ColIndex::new(4).check_content_position(col_width),
            PositionStatus::Beyond
        );

        // Test with RowIndex/RowHeight
        let row_height = RowHeight::new(2);
        assert_eq!(
            RowIndex::new(0).check_content_position(row_height),
            PositionStatus::Within
        );
        assert_eq!(
            RowIndex::new(1).check_content_position(row_height),
            PositionStatus::Within
        );
        assert_eq!(
            RowIndex::new(2).check_content_position(row_height),
            PositionStatus::Boundary
        );
        assert_eq!(
            RowIndex::new(3).check_content_position(row_height),
            PositionStatus::Beyond
        );

        // Test with Index/Index
        let other_index = idx(4);
        assert_eq!(
            idx(2).check_content_position(other_index),
            PositionStatus::Within
        );
        assert_eq!(
            idx(4).check_content_position(other_index),
            PositionStatus::Boundary
        );
        assert_eq!(
            idx(5).check_content_position(other_index),
            PositionStatus::Beyond
        );
    }
}

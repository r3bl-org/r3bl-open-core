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
//! # Key Components
//!
//! - [`BoundsCheck`] trait: Core functionality for both checking paradigms
//! - [`BoundsOverflowStatus`] enum: Results for array-style bounds checking
//! - [`ContentPositionStatus`] enum: Results for content position checking
//! - [`check_overflows!`](crate::check_overflows) macro: Concise syntax for overflow
//!   checking
//!
//! ## Implementations
//!
//! See [`crate::dimens_bounds_check_impl`] and [`crate::unit_bounds_check_impl`]
//! modules for trait implementations. The [`dimens`] module contains implementations
//! for [`RowIndex`], [`ColIndex`] types with [`RowHeight`], [`ColWidth`], etc. The
//! `units` module contains implementations for the generic [`Index`] and [`Length`]
//! types.
//!
//! # Usage Example
//!
//! ```
//! use r3bl_tui::{BoundsCheck, ContentPositionStatus, check_overflows, idx, len};
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
//! // Array-style overflow checking
//! if !check_overflows!(cursor_pos, content_length) {
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

/// Result of array-style bounds checking operations.
///
/// Used with [`BoundsCheck::check_overflows`] to determine if an index can safely
/// access array content. See the [module documentation](self) for details on the
/// bounds checking paradigms.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, idx, len};
///
/// let index = idx(5);
/// let length = len(10);
/// assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Within);
///
/// let large_index = idx(10);
/// assert_eq!(large_index.check_overflows(length), BoundsOverflowStatus::Overflowed);
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BoundsOverflowStatus {
    /// Indicates that an index is within the bounds of a length or another index.
    Within,
    /// Indicates that an index has overflowed the bounds of a length or another index.
    Overflowed,
}

/// Concise macro for array-style bounds overflow checking.
///
/// Returns `true` if the index overflows the bounds (`index >= max`), `false` if within
/// bounds. Equivalent to `index.check_overflows(max) ==
/// BoundsOverflowStatus::Overflowed`.
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
/// # use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, ColIndex, ColWidth};
/// # let index = ColIndex::new(5);
/// # let width = ColWidth::new(10);
/// // if index.check_overflows(width) == BoundsOverflowStatus::Overflowed { ... }
/// if check_overflows!(index, width) { /* ... */ }
/// ```
#[macro_export]
macro_rules! check_overflows {
    ($index:expr, $max:expr) => {
        $index.check_overflows($max) == $crate::BoundsOverflowStatus::Overflowed
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
/// use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, RowIndex, RowHeight};
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(row_index.check_overflows(height), BoundsOverflowStatus::Overflowed);
/// ```
pub trait BoundsCheck<OtherType> {
    /// Performs array-style bounds checking.
    ///
    /// Returns `BoundsOverflowStatus::Within` if the index can safely access content,
    /// `BoundsOverflowStatus::Overflowed` if the index would exceed array bounds.
    ///
    /// See the [module documentation](self) for detailed explanation of bounds checking.
    fn check_overflows(&self, max: OtherType) -> BoundsOverflowStatus;

    /// Performs content position checking.
    ///
    /// Returns `ContentPositionStatus` indicating whether the index is within content,
    /// at a content boundary, or beyond content boundaries.
    ///
    /// See the [module documentation](self) for detailed explanation of content position
    /// checking.
    fn check_content_position(&self, content_length: OtherType) -> ContentPositionStatus;
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
/// use r3bl_tui::{BoundsCheck, ContentPositionStatus, idx, len};
///
/// let content_length = len(5);
///
/// assert_eq!(idx(0).check_content_position(content_length), ContentPositionStatus::AtStart);
/// assert_eq!(idx(3).check_content_position(content_length), ContentPositionStatus::Within);
/// assert_eq!(idx(5).check_content_position(content_length), ContentPositionStatus::AtEnd);
/// assert_eq!(idx(7).check_content_position(content_length), ContentPositionStatus::Beyond);
/// ```
///
/// ## Why RowContentPositionStatus exists separate from this enum
///
/// [`crate::RowContentPositionStatus`] has different semantics for handling row positions
/// in a terminal buffer, which is why it does not use this enum.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ContentPositionStatus {
    /// Index is at the start of content (`index == 0`). For empty content, this takes
    /// precedence over `AtEnd`.
    AtStart,

    /// Index points to existing content (`0 < index < length`).
    Within,

    /// Index is at the content end boundary (`index == length && index > 0`), valid for
    /// cursor/insertion.
    AtEnd,

    /// Index exceeds content boundaries (`index > length`), requires correction.
    Beyond,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{idx, len};

    #[test]
    fn test_bounds_overflow_status_equality() {
        assert_eq!(BoundsOverflowStatus::Within, BoundsOverflowStatus::Within);
        assert_eq!(
            BoundsOverflowStatus::Overflowed,
            BoundsOverflowStatus::Overflowed
        );
        assert_ne!(
            BoundsOverflowStatus::Within,
            BoundsOverflowStatus::Overflowed
        );
    }

    #[test]
    fn test_bounds_overflow_status_copy() {
        let status1 = BoundsOverflowStatus::Within;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = BoundsOverflowStatus::Overflowed;
        let status4 = status3;
        assert_eq!(status3, status4);
    }

    #[test]
    fn test_bounds_overflow_status_debug() {
        assert_eq!(format!("{:?}", BoundsOverflowStatus::Within), "Within");
        assert_eq!(
            format!("{:?}", BoundsOverflowStatus::Overflowed),
            "Overflowed"
        );
    }

    #[test]
    fn test_position_status_equality() {
        assert_eq!(
            ContentPositionStatus::AtStart,
            ContentPositionStatus::AtStart
        );
        assert_eq!(ContentPositionStatus::Within, ContentPositionStatus::Within);
        assert_eq!(ContentPositionStatus::AtEnd, ContentPositionStatus::AtEnd);
        assert_eq!(ContentPositionStatus::Beyond, ContentPositionStatus::Beyond);
        assert_ne!(
            ContentPositionStatus::AtStart,
            ContentPositionStatus::Within
        );
        assert_ne!(ContentPositionStatus::Within, ContentPositionStatus::AtEnd);
        assert_ne!(ContentPositionStatus::AtEnd, ContentPositionStatus::Beyond);
        assert_ne!(
            ContentPositionStatus::AtStart,
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_position_status_copy() {
        let status1 = ContentPositionStatus::AtStart;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = ContentPositionStatus::Within;
        let status4 = status3;
        assert_eq!(status3, status4);

        let status5 = ContentPositionStatus::AtEnd;
        let status6 = status5;
        assert_eq!(status5, status6);

        let status7 = ContentPositionStatus::Beyond;
        let status8 = status7;
        assert_eq!(status7, status8);
    }

    #[test]
    fn test_position_status_debug() {
        assert_eq!(format!("{:?}", ContentPositionStatus::AtStart), "AtStart");
        assert_eq!(format!("{:?}", ContentPositionStatus::Within), "Within");
        assert_eq!(format!("{:?}", ContentPositionStatus::AtEnd), "AtEnd");
        assert_eq!(format!("{:?}", ContentPositionStatus::Beyond), "Beyond");
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
                index.check_overflows(length) == BoundsOverflowStatus::Overflowed,
                "Macro should match direct method for index {index_val} and length {length_val}"
            );
        }
    }

    #[test]
    fn test_check_content_position_basic() {
        let content_length = len(5);

        // At start
        assert_eq!(
            idx(0).check_content_position(content_length),
            ContentPositionStatus::AtStart
        );

        // Within content
        assert_eq!(
            idx(2).check_content_position(content_length),
            ContentPositionStatus::Within
        );
        assert_eq!(
            idx(4).check_content_position(content_length),
            ContentPositionStatus::Within
        );

        // At end boundary
        assert_eq!(
            idx(5).check_content_position(content_length),
            ContentPositionStatus::AtEnd
        );

        // Beyond content
        assert_eq!(
            idx(6).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
        assert_eq!(
            idx(10).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_check_content_position_edge_cases() {
        // Zero-length content - AtStart takes precedence
        let zero_length = len(0);
        assert_eq!(
            idx(0).check_content_position(zero_length),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            idx(1).check_content_position(zero_length),
            ContentPositionStatus::Beyond
        );

        // Single element content
        let single_length = len(1);
        assert_eq!(
            idx(0).check_content_position(single_length),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            idx(1).check_content_position(single_length),
            ContentPositionStatus::AtEnd
        );
        assert_eq!(
            idx(2).check_content_position(single_length),
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_check_content_position_with_typed_indices() {
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        // Test with ColIndex/ColWidth
        let col_width = ColWidth::new(3);
        assert_eq!(
            ColIndex::new(0).check_content_position(col_width),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            ColIndex::new(2).check_content_position(col_width),
            ContentPositionStatus::Within
        );
        assert_eq!(
            ColIndex::new(3).check_content_position(col_width),
            ContentPositionStatus::AtEnd
        );
        assert_eq!(
            ColIndex::new(4).check_content_position(col_width),
            ContentPositionStatus::Beyond
        );

        // Test with RowIndex/RowHeight
        let row_height = RowHeight::new(2);
        assert_eq!(
            RowIndex::new(0).check_content_position(row_height),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            RowIndex::new(1).check_content_position(row_height),
            ContentPositionStatus::Within
        );
        assert_eq!(
            RowIndex::new(2).check_content_position(row_height),
            ContentPositionStatus::AtEnd
        );
        assert_eq!(
            RowIndex::new(3).check_content_position(row_height),
            ContentPositionStatus::Beyond
        );

        // Test with Index/Index
        let other_index = idx(4);
        assert_eq!(
            idx(0).check_content_position(other_index),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            idx(2).check_content_position(other_index),
            ContentPositionStatus::Within
        );
        assert_eq!(
            idx(4).check_content_position(other_index),
            ContentPositionStatus::AtEnd
        );
        assert_eq!(
            idx(5).check_content_position(other_index),
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_position_status_empty_content_precedence() {
        // Test that AtStart takes precedence over AtEnd for empty content
        let empty_length = len(0);
        assert_eq!(
            idx(0).check_content_position(empty_length),
            ContentPositionStatus::AtStart
        );

        // Test with typed indices too
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        let empty_col_width = ColWidth::new(0);
        assert_eq!(
            ColIndex::new(0).check_content_position(empty_col_width),
            ContentPositionStatus::AtStart
        );

        let empty_row_height = RowHeight::new(0);
        assert_eq!(
            RowIndex::new(0).check_content_position(empty_row_height),
            ContentPositionStatus::AtStart
        );
    }

    #[test]
    fn test_position_status_comprehensive() {
        // Test all combinations for a length-3 content
        let content_length = len(3);

        // AtStart: index == 0
        assert_eq!(
            idx(0).check_content_position(content_length),
            ContentPositionStatus::AtStart
        );

        // Within: 0 < index < length
        assert_eq!(
            idx(1).check_content_position(content_length),
            ContentPositionStatus::Within
        );
        assert_eq!(
            idx(2).check_content_position(content_length),
            ContentPositionStatus::Within
        );

        // AtEnd: index == length && index > 0
        assert_eq!(
            idx(3).check_content_position(content_length),
            ContentPositionStatus::AtEnd
        );

        // Beyond: index > length
        assert_eq!(
            idx(4).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
        assert_eq!(
            idx(10).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
    }
}

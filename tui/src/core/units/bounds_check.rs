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
//! use r3bl_tui::{BoundsCheck, ContentPositionStatus, idx, len};
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
    /// Indicates that an index is within the bounds of a length.
    Within,
    /// Indicates that an index has overflowed the bounds of a length.
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
/// use r3bl_tui::{idx, len};
///
/// if len(10).is_overflowed_by(idx(5)) {
///     // Handle overflow case
/// }
///
/// // More convenient than verbose syntax:
/// # use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, ColIndex, ColWidth};
/// # let index = ColIndex::new(5);
/// # let width = ColWidth::new(10);
/// // if index.check_overflows(width) == BoundsOverflowStatus::Overflowed { ... }
/// if width.is_overflowed_by(index) { /* ... */ }
/// ```
/// Core trait for unit comparison operations.
///
/// Provides standardized methods to convert unit types to common numeric types
/// for comparison operations. This trait enables generic implementations of
/// bounds checking across different unit types.
pub trait UnitCompare {
    /// Convert the unit to a usize value for numeric comparison, usually for array
    /// indexing operations.
    fn as_usize(&self) -> usize;

    /// Convert the unit to a u16 value for crossterm compatibility and other terminal and
    /// pty based operations.
    fn as_u16(&self) -> u16;
}

/// Marker trait for index-type units (0-based position indicators).
///
/// This trait identifies types that represent positions or indices within
/// content, such as `RowIndex`, `ColIndex`, and `Index`. These are 0-based
/// values where the first position is index 0.
///
/// Each index type has a corresponding length type via [`LengthType`](Self::LengthType),
/// enabling safe bounds checking operations in both directions.
///
/// See the [module documentation](self) Type System section for details on
/// how index types relate to length types and the type safety guarantees.
pub trait IndexMarker: UnitCompare {
    /// The corresponding length type for this index type.
    type LengthType: LengthMarker<IndexType = Self>;

    /// Answers the question: "Does this index overflow this length?"
    ///
    /// Check if this index overflows the given length's bounds.
    /// Returns true if the index is greater than or equal to the length.
    ///
    /// This is the inverse of [`LengthMarker::is_overflowed_by`] and provides
    /// a natural way to express bounds checking from the index's perspective.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{col, width};
    ///
    /// let index = col(10);
    /// let max_width = width(10);
    /// assert!(index.overflows(max_width));  // At boundary - overflows
    ///
    /// let smaller_index = col(5);
    /// assert!(!smaller_index.overflows(max_width));  // Within bounds
    /// ```
    fn overflows(&self, length: Self::LengthType) -> bool
    where
        Self: PartialOrd + Sized + Copy,
    {
        length.is_overflowed_by(*self)
    }
}

/// Marker trait for length-type units (1-based size measurements).
///
/// This trait identifies types that represent sizes or lengths of content,
/// such as `RowHeight`, `ColWidth`, and `Length`. These are 1-based values
/// where a length of 1 means "one unit of size".
///
/// Each length type has a corresponding index type via [`IndexType`](Self::IndexType),
/// enabling safe bounds checking operations.
///
/// See the [module documentation](self) Type System section for details on
/// how length types relate to index types and the type safety guarantees.
pub trait LengthMarker: UnitCompare {
    /// The corresponding index type for this length type.
    type IndexType: IndexMarker;

    /// Convert this length to the corresponding index type.
    ///
    /// This typically involves subtracting 1 from the length value since
    /// lengths are 1-based and indices are 0-based.
    fn convert_to_index(&self) -> Self::IndexType;

    /// Answers the question: "Does this length get overflowed by this index?"
    ///
    /// Check if the given index would overflow this length's bounds.
    /// Returns true if the index is greater than or equal to the length.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{col, width};
    ///
    /// let max_col = width(10);
    /// assert!(!max_col.is_overflowed_by(col(5)));  // Within bounds
    /// assert!(max_col.is_overflowed_by(col(10)));  // At boundary - overflows
    /// assert!(max_col.is_overflowed_by(col(15)));  // Beyond boundary
    /// ```
    fn is_overflowed_by(&self, index: Self::IndexType) -> bool
    where
        Self::IndexType: PartialOrd,
    {
        // Special case: empty collection (length 0) has no valid indices
        if self.as_usize() == 0 {
            return true;
        }
        index > self.convert_to_index()
    }
}

/// Core trait for index bounds validation in TUI applications.
///
/// Provides both array-style bounds checking and content position checking.
/// See the [module documentation](self) for detailed explanations of both paradigms.
///
/// This trait is now generic over length types that implement `LengthMarker`,
/// and can only be implemented by index types that implement `IndexMarker`.
/// This ensures type safety and prevents incorrect comparisons between incompatible
/// types.
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
pub trait BoundsCheck<OtherType: LengthMarker>
where
    Self: IndexMarker,
{
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

/// Generic implementation of [`BoundsCheck`] for any [`IndexMarker`] type with
/// [`LengthMarker`] type.
///
/// This single implementation works with all index and length types that implement the
/// required marker traits, eliminating code duplication and ensuring consistent behavior.
/// The trait system guarantees type safety by only allowing compatible index-length
/// pairs.
impl<I, L> BoundsCheck<L> for I
where
    I: IndexMarker + PartialOrd + Copy,
    L: LengthMarker<IndexType = I>,
{
    fn check_overflows(&self, length: L) -> BoundsOverflowStatus {
        let this = *self;
        let other = length.convert_to_index();
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(&self, content_length: L) -> ContentPositionStatus {
        let position = self.as_usize();
        let length = content_length.as_usize();

        if position > length {
            ContentPositionStatus::Beyond
        } else if position == 0 {
            ContentPositionStatus::AtStart
        } else if position == length {
            ContentPositionStatus::AtEnd
        } else {
            ContentPositionStatus::Within
        }
    }
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
/// ## Why [`RowContentPositionStatus`](crate::RowContentPositionStatus) exists separate from this enum
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
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

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
    fn test_is_overflowed_by() {
        // Test basic cases with Index/Length
        assert!(!len(3).is_overflowed_by(idx(1)), "Within bounds");
        assert!(len(3).is_overflowed_by(idx(3)), "At boundary");
        assert!(len(3).is_overflowed_by(idx(5)), "Beyond bounds");
        assert!(
            len(0).is_overflowed_by(idx(0)),
            "Empty collection edge case"
        );

        // Test with typed dimensions
        assert!(
            !ColWidth::new(10).is_overflowed_by(ColIndex::new(5)),
            "Typed indices within bounds"
        );
        assert!(
            ColWidth::new(10).is_overflowed_by(ColIndex::new(10)),
            "Typed indices at boundary"
        );
        assert!(
            !RowHeight::new(5).is_overflowed_by(RowIndex::new(3)),
            "Row indices within bounds"
        );
        assert!(
            RowHeight::new(5).is_overflowed_by(RowIndex::new(5)),
            "Row indices at boundary"
        );

        // Verify method matches existing check_overflows behavior
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            assert_eq!(
                length.is_overflowed_by(index),
                index.check_overflows(length) == BoundsOverflowStatus::Overflowed,
                "New method should match existing behavior for index {index_val} and length {length_val}"
            );
        }
    }

    #[test]
    fn test_overflows() {
        // Test basic cases with Index/Length - should mirror is_overflowed_by results
        assert!(!idx(1).overflows(len(3)), "Within bounds");
        assert!(idx(3).overflows(len(3)), "At boundary");
        assert!(idx(5).overflows(len(3)), "Beyond bounds");
        assert!(idx(0).overflows(len(0)), "Empty collection edge case");

        // Test with typed dimensions
        assert!(
            !ColIndex::new(5).overflows(ColWidth::new(10)),
            "Typed indices within bounds"
        );
        assert!(
            ColIndex::new(10).overflows(ColWidth::new(10)),
            "Typed indices at boundary"
        );
        assert!(
            !RowIndex::new(3).overflows(RowHeight::new(5)),
            "Row indices within bounds"
        );
        assert!(
            RowIndex::new(5).overflows(RowHeight::new(5)),
            "Row indices at boundary"
        );

        // Verify method matches is_overflowed_by behavior (inverse relationship)
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            assert_eq!(
                index.overflows(length),
                length.is_overflowed_by(index),
                "overflows() should match is_overflowed_by() for index {index_val} and length {length_val}"
            );
        }

        // Test with specific typed combinations
        let col_cases = [(0, 5), (4, 5), (5, 5), (6, 5)];
        for (index_val, width_val) in col_cases {
            let col_index = ColIndex::new(index_val);
            let col_width = ColWidth::new(width_val);
            assert_eq!(
                col_index.overflows(col_width),
                col_width.is_overflowed_by(col_index),
                "ColIndex::overflows should match ColWidth::is_overflowed_by for index {index_val} and width {width_val}"
            );
        }

        let row_cases = [(0, 3), (2, 3), (3, 3), (4, 3)];
        for (index_val, height_val) in row_cases {
            let row_index = RowIndex::new(index_val);
            let row_height = RowHeight::new(height_val);
            assert_eq!(
                row_index.overflows(row_height),
                row_height.is_overflowed_by(row_index),
                "RowIndex::overflows should match RowHeight::is_overflowed_by for index {index_val} and height {height_val}"
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

        // Test with Index/Index removed as this is no longer supported.
        // BoundsCheck is now specifically for Index-to-Length comparisons only.
    }

    #[test]
    fn test_position_status_empty_content_precedence() {
        use super::*;

        // Test that AtStart takes precedence over AtEnd for empty content
        let empty_length = len(0);
        assert_eq!(
            idx(0).check_content_position(empty_length),
            ContentPositionStatus::AtStart
        );

        // Test with typed indices too

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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core traits for type-safe bounds checking operations.
//!
//! This module defines the foundational traits that enable the bounds checking system:
//! - [`UnitCompare`] - Numeric conversion operations for unit types
//! - [`IndexMarker`] - Identifies 0-based position types (like `ColIndex`, `RowIndex`)
//! - [`LengthMarker`] - Identifies 1-based size types (like `ColWidth`, `RowHeight`)
//!
//! These traits work together to provide type safety and prevent incorrect
//! comparisons between incompatible types (e.g., row vs column indices).
//!
//! See the [`bounds_check` module documentation](crate::core::units::bounds_check)
//! for details on the type system and safety guarantees.

use std::{cmp::min, ops::Sub};

use crate::{ArrayAccessBoundsStatus, Length, len};

/// Core trait for unit comparison operations.
///
/// Provides standardized methods to convert unit types to common numeric types
/// for comparison operations. This trait enables generic implementations of
/// bounds checking across different unit types.
pub trait UnitCompare: From<usize> + From<u16> {
    /// Convert the unit to a usize value for numeric comparison, usually for array
    /// indexing operations.
    fn as_usize(&self) -> usize;

    /// Convert the unit to a u16 value for crossterm compatibility and other terminal and
    /// pty based operations.
    fn as_u16(&self) -> u16;

    /// Check if the unit value is zero.
    fn is_zero(&self) -> bool { self.as_usize() == 0 }
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
/// See the [module documentation](crate::core::units::bounds_check) "Type System"
/// section for details on how index types relate to length types and the type safety
/// guarantees.
pub trait IndexMarker: UnitCompare {
    /// The corresponding length type for this index type.
    ///
    /// The constraint `LengthMarker<IndexType = Self>` creates a bidirectional
    /// relationship: this ensures that the length type's `IndexType` points back to
    /// this same index type, preventing type mismatches like `ColIndex` ↔
    /// `RowHeight`.
    type LengthType: LengthMarker<IndexType = Self>;

    /// Convert this index to the corresponding length type.
    ///
    /// This typically involves adding 1 to the index value since
    /// indices are 0-based and lengths are 1-based.
    ///
    /// ```text
    /// Index=5 (0-based) to length (1-based) conversion:
    ///
    ///                           index=5 (0-based)
    ///                                 ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │   │   │   │   │   │
    ///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    /// Length:     1   2   3   4   5   6   7   8   9   10
    /// (1-based)                       ↑
    ///                  convert_to_length() = 6 (1-based)
    /// ```
    fn convert_to_length(&self) -> Self::LengthType;

    /// Answers the question: "Does this index overflow this length?"
    ///
    /// Check if this index overflows the given length's bounds.
    /// This is the inverse of [`LengthMarker::is_overflowed_by`] and provides
    /// a natural way to express bounds checking from the index's perspective.
    ///
    /// ```text
    /// Checking if index overflows length:
    ///
    ///                           index=5 (0-based)   index=10 (0-based)
    ///                                 ↓                   ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    /// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///           │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ × │ × │ × │
    ///           ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///           ├────────── within bounds ──────────────┼─ overflow ┘
    ///           └────────── length=10 (1-based) ────────┘
    ///
    /// overflows(length=10) = true (index 10 overflows length 10)
    /// overflows(length=5)  = false (index 5 within length 10)
    /// ```
    ///
    /// # Returns
    /// true if the index is greater than or equal to the length.
    ///
    /// # See Also
    /// For detailed status information with pattern matching capabilities, use
    /// [`crate::BoundsCheck::check_array_access_bounds()`] which returns a
    /// [`crate::ArrayAccessBoundsStatus`] enum. This method is a convenience wrapper
    /// designed for simple boolean conditions.
    ///
    /// Both methods are semantically equivalent:
    /// - `index.overflows(length)` returns `bool`
    /// - `index.check_array_access_bounds(length) == ArrayAccessBoundsStatus::Overflowed`
    ///   returns `bool`
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, BoundsCheck, ArrayAccessBoundsStatus, col, width};
    ///
    /// let index = col(10);
    /// let max_width = width(10);
    ///
    /// // Simple boolean check - use this method:
    /// if index.overflows(max_width) {
    ///     println!("Index out of bounds");
    /// }
    ///
    /// // For pattern matching - use check_array_access_bounds():
    /// match index.check_array_access_bounds(max_width) {
    ///     ArrayAccessBoundsStatus::Within => println!("Safe to access"),
    ///     ArrayAccessBoundsStatus::Overflowed => println!("Index out of bounds"),
    ///     ArrayAccessBoundsStatus::Underflowed => println!("Index underflowed"),
    /// }
    ///
    /// let smaller_index = col(5);
    /// assert!(!smaller_index.overflows(max_width));  // Within bounds
    /// ```
    fn overflows(&self, arg_length: impl Into<Self::LengthType>) -> bool
    where
        Self: PartialOrd + Sized + Copy,
    {
        let length: Self::LengthType = arg_length.into();
        length.is_overflowed_by(*self)
    }

    /// Check if this index underflows (goes below) the given minimum bound.
    ///
    /// This is useful for checking if a position would go negative or below
    /// a starting position when moving backwards, such as in scrolling logic.
    ///
    /// ```text
    /// Checking if index underflows minimum:
    ///
    ///           min_bound=3
    ///                ↓
    /// Index:   0   1   2   3   4   5   6   7   8   9
    ///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///        │ × │ × │ × │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │
    ///        ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
    ///        └ underflow ┼──── valid range ──────────┘
    ///
    /// underflows(min=3) for index=2  = true  (below minimum)
    /// underflows(min=3) for index=3  = false (at minimum, valid)
    /// underflows(min=3) for index=5  = false (above minimum)
    /// ```
    ///
    /// # Returns
    /// Returns true if this index is less than the minimum bound.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, col, row};
    ///
    /// let min_col = col(3);
    /// assert!(col(0).underflows(min_col));  // 0 < 3
    /// assert!(col(2).underflows(min_col));  // 2 < 3
    /// assert!(!col(3).underflows(min_col)); // 3 == 3 (at boundary)
    /// assert!(!col(5).underflows(min_col)); // 5 > 3
    /// ```
    fn underflows(&self, min_bound: impl Into<Self>) -> bool
    where
        Self: PartialOrd + Sized,
    {
        let min: Self = min_bound.into();
        *self < min
    }

    /// Check bounds against both minimum and maximum values.
    ///
    /// This provides comprehensive bounds checking that can detect underflow,
    /// valid positions, and overflow in a single operation.
    ///
    /// # Returns
    /// - [`crate::ArrayAccessBoundsStatus::Underflowed`] if index < min
    /// - [`crate::ArrayAccessBoundsStatus::Within`] if min <= index < max_length
    /// - [`crate::ArrayAccessBoundsStatus::Overflowed`] if index >= max_length
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, ArrayAccessBoundsStatus, col, width};
    ///
    /// let min_col = col(2);
    /// let max_width = width(8);
    ///
    /// assert_eq!(col(1).check_bounds_range(min_col, max_width), ArrayAccessBoundsStatus::Underflowed);
    /// assert_eq!(col(5).check_bounds_range(min_col, max_width), ArrayAccessBoundsStatus::Within);
    /// assert_eq!(col(8).check_bounds_range(min_col, max_width), ArrayAccessBoundsStatus::Overflowed);
    /// ```
    fn check_bounds_range(
        &self,
        arg_min: impl Into<Self>,
        max: Self::LengthType,
    ) -> ArrayAccessBoundsStatus
    where
        Self: PartialOrd + Sized + Copy,
    {
        let min_bound: Self = arg_min.into();

        if *self < min_bound {
            ArrayAccessBoundsStatus::Underflowed
        } else if self.overflows(max) {
            ArrayAccessBoundsStatus::Overflowed
        } else {
            ArrayAccessBoundsStatus::Within
        }
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
/// See the [module documentation](crate::core::units::bounds_check) "Type System"
/// section for details on how length types relate to index types and the type safety
/// guarantees.
pub trait LengthMarker: UnitCompare {
    /// The corresponding index type for this length type.
    ///
    /// The constraint `IndexMarker<LengthType = Self>` creates a bidirectional
    /// relationship: this ensures that the index type's `LengthType` points back to
    /// this same length type, preventing type mismatches like `ColWidth` ↔
    /// `RowIndex`.
    type IndexType: IndexMarker<LengthType = Self>;

    /// Convert this length to the corresponding index type.
    ///
    /// This typically involves subtracting 1 from the length value since
    /// lengths are 1-based and indices are 0-based.
    ///
    /// ```text
    /// Length=10 to index conversion:
    ///           ┌────────── length=10 (1-based) ────────┐
    /// Length:     1   2   3   4   5   6   7   8   9   10
    /// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │   │   │   │   │ ␩ │
    ///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    /// Index:      0   1   2   3   4   5   6   7   8   9
    /// (0-based)                                       ↑
    ///                                         convert_to_index() = 9
    /// ```
    fn convert_to_index(&self) -> Self::IndexType {
        let value = self.as_usize().saturating_sub(1);
        Self::IndexType::from(value)
    }

    /// Answers the question: "Does this length get overflowed by this index?"
    ///
    /// Check if the given index would overflow this length's bounds.
    ///
    /// Example - Checking overflow for length=10
    ///
    /// ```text
    ///                                             boundary
    ///                                                 │
    /// Index:    0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///         │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✗ │ ✗ │ ✗ │
    ///         ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///         ├────────── valid indices ──────────────┼─ overflow ┘
    ///         └────────── length=10 (1-based) ────────┘
    ///
    /// is_overflowed_by(5)  = false (within bounds)
    /// is_overflowed_by(9)  = false (last valid index)
    /// is_overflowed_by(10) = true (at boundary)
    /// is_overflowed_by(11) = true (beyond boundary)
    /// ```
    ///
    /// # Returns
    ///
    /// Returns true if the index is greater than or equal to the length.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, col, row, width};
    ///
    /// let max_col = width(10);
    /// assert!(!max_col.is_overflowed_by(col(5)));  // Within bounds
    /// assert!(max_col.is_overflowed_by(col(10)));  // At boundary - overflows
    /// assert!(max_col.is_overflowed_by(col(15)));  // Beyond boundary
    ///
    /// // Pos (row + col) automatically converts to ColIndex
    /// assert!(!max_col.is_overflowed_by(row(0) + col(5)));  // Pos converts to ColIndex
    /// assert!(max_col.is_overflowed_by(row(2) + col(10)));  // Pos at boundary - overflows
    /// ```
    fn is_overflowed_by(&self, arg_index: impl Into<Self::IndexType>) -> bool
    where
        Self::IndexType: PartialOrd,
    {
        let index: Self::IndexType = arg_index.into();
        // Special case: empty collection (length 0) has no valid indices.
        if self.as_usize() == 0 {
            return true;
        }
        index > self.convert_to_index()
    }

    /// Calculate the remaining space from the given index to the end of this length.
    ///
    /// ```text
    /// With max_width=10:
    ///
    ///                 index=3 (0-based)
    ///                       ↓
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///         │   │   │   │ × │ × │ × │ × │ × │ × │ × │
    ///         ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
    ///         │           └───── 7 chars remain ──────┤
    ///         └────────── width=10 (1-based) ─────────┘
    ///
    /// remaining_from(3)  = 7 (chars from index 3 to 9)
    /// remaining_from(9)  = 1 (only position 9 remains)
    /// remaining_from(10) = 0 (at boundary, nothing remains)
    /// ```
    ///
    /// # Returns
    /// The number of units between the index and the boundary defined by this
    /// length. For example, if this is a `ColWidth` of 10 and the index is at column 3,
    /// this returns a Length of 7 (columns 3-9, inclusive).
    ///
    /// Returns Length(0) if the index is at or beyond the boundary.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, col, row, width, len};
    ///
    /// let max_width = width(10);
    /// assert_eq!(max_width.remaining_from(col(3)), len(7));  // 7 columns remain
    /// assert_eq!(max_width.remaining_from(col(10)), len(0)); // At boundary
    /// assert_eq!(max_width.remaining_from(col(15)), len(0)); // Beyond boundary
    ///
    /// // Pos (row + col) automatically converts to ColIndex
    /// assert_eq!(max_width.remaining_from(row(0) + col(3)), len(7));  // Pos converts to ColIndex
    /// assert_eq!(max_width.remaining_from(row(1) + col(10)), len(0)); // Pos at boundary
    /// ```
    fn remaining_from(&self, arg_index: impl Into<Self::IndexType>) -> Length
    where
        Self::IndexType: PartialOrd + Sub<Output = Self::IndexType> + Copy,
        <Self::IndexType as IndexMarker>::LengthType: Into<Length>,
    {
        let index: Self::IndexType = arg_index.into();
        if self.is_overflowed_by(index) {
            len(0)
        } else {
            // Get max index for this length.
            let max_index = self.convert_to_index();
            // Calculate num of chars from cursor to boundary (as index difference).
            let chars_remaining_as_index = max_index - index;
            // Convert from 0-based index difference to 1-based length.
            chars_remaining_as_index.convert_to_length().into()
        }
    }

    /// Clamps this length to a maximum value.
    ///
    /// ```text
    /// Clamping operation with max_length=7:
    ///
    /// Case 1: length=5 (within bounds)
    /// ┌───── length=5 ─────┐
    /// │ 1   2   3   4   5 │ 6   7 ← max_length boundary
    /// ├───┬───┬───┬───┬───┼───┬───┤
    /// │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │   │   │
    /// └───┴───┴───┴───┴───┴───┴───┘
    ///
    /// Result: clamp_to(5, max=7) = 5 (no change - within bounds)
    ///
    /// Case 2: length=10 (exceeds bounds)
    /// ┌───────────── length=10 ──────────────┐
    /// │ 1   2   3   4   5   6   7 │ 8   9   10 (trimmed)
    /// ├───┬───┬───┬───┬───┬───┬───┼───┬───┬───┤
    /// │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ × │ × │ × │
    /// └───┴───┴───┴───┴───┴───┴───┼───┴───┴───┘
    ///                             └─ max_length=7 boundary
    ///
    /// Result: clamp_to(10, max=7) = 7 (clamped to maximum)
    /// ```
    ///
    /// # Returns
    ///
    /// The smaller of this length or the maximum length provided.
    /// This is commonly used when constraining operations to available space
    /// or buffer boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::{LengthMarker, len};
    ///
    /// // Length within bounds - no change
    /// let small_length = len(5);
    /// let max_allowed = len(10);
    /// assert_eq!(small_length.clamp_to(max_allowed), len(5));
    ///
    /// // Length exceeds bounds - gets clamped
    /// let large_length = len(15);
    /// let max_allowed = len(10);
    /// assert_eq!(large_length.clamp_to(max_allowed), len(10));
    ///
    /// // Equal lengths - returns the same value
    /// let equal_length = len(8);
    /// let max_allowed = len(8);
    /// assert_eq!(equal_length.clamp_to(max_allowed), len(8));
    /// ```
    #[must_use]
    fn clamp_to(&self, arg_max_length: impl Into<Self>) -> Self
    where
        Self: Copy + Ord,
    {
        let max_length: Self = arg_max_length.into();
        min(*self, max_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrayAccessBoundsStatus, BoundsCheck, ColIndex, ColWidth, RowHeight,
                RowIndex, idx, len};

    mod overflow_operations_tests {
        use super::*;

        #[test]
        fn test_is_overflowed_by() {
            // Test basic cases with Index/Length.
            assert!(!len(3).is_overflowed_by(idx(1)), "Within bounds");
            assert!(len(3).is_overflowed_by(idx(3)), "At boundary");
            assert!(len(3).is_overflowed_by(idx(5)), "Beyond bounds");
            assert!(
                len(0).is_overflowed_by(idx(0)),
                "Empty collection edge case"
            );

            // Test with typed dimensions.
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

            // Verify method matches existing check_overflows behavior.
            let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
            for (index_val, length_val) in test_cases {
                let index = idx(index_val);
                let length = len(length_val);
                assert_eq!(
                    length.is_overflowed_by(index),
                    index.check_array_access_bounds(length)
                        == ArrayAccessBoundsStatus::Overflowed,
                    "New method should match existing behavior for index {index_val} and length {length_val}"
                );
            }
        }

        #[test]
        fn test_underflows() {
            use crate::{col, row};

            // Test column underflow
            let min_col = col(3);
            assert!(col(0).underflows(min_col)); // 0 < 3
            assert!(col(2).underflows(min_col)); // 2 < 3
            assert!(!col(3).underflows(min_col)); // 3 == 3 (at boundary)
            assert!(!col(5).underflows(min_col)); // 5 > 3

            // Test row underflow
            let min_row = row(5);
            assert!(row(4).underflows(min_row)); // 4 < 5
            assert!(!row(5).underflows(min_row)); // 5 == 5
            assert!(!row(10).underflows(min_row)); // 10 > 5

            // Test with Index/Length
            let min_index = idx(7);
            assert!(idx(3).underflows(min_index)); // 3 < 7
            assert!(idx(6).underflows(min_index)); // 6 < 7
            assert!(!idx(7).underflows(min_index)); // 7 == 7
            assert!(!idx(10).underflows(min_index)); // 10 > 7
        }

        #[test]
        fn test_check_bounds_range() {
            use crate::{ArrayAccessBoundsStatus, col, width};

            let min_col = col(2);
            let max_width = width(8);

            // Test underflow
            assert_eq!(
                col(0).check_bounds_range(min_col, max_width),
                ArrayAccessBoundsStatus::Underflowed
            );
            assert_eq!(
                col(1).check_bounds_range(min_col, max_width),
                ArrayAccessBoundsStatus::Underflowed
            );

            // Test within bounds
            assert_eq!(
                col(2).check_bounds_range(min_col, max_width),
                ArrayAccessBoundsStatus::Within
            );
            assert_eq!(
                col(5).check_bounds_range(min_col, max_width),
                ArrayAccessBoundsStatus::Within
            );
            assert_eq!(
                col(7).check_bounds_range(min_col, max_width),
                ArrayAccessBoundsStatus::Within
            );

            // Test overflow
            assert_eq!(
                col(8).check_bounds_range(min_col, max_width),
                ArrayAccessBoundsStatus::Overflowed
            );
            assert_eq!(
                col(10).check_bounds_range(min_col, max_width),
                ArrayAccessBoundsStatus::Overflowed
            );

            // Test edge cases with zero minimum
            let min_zero = col(0);
            assert_eq!(
                col(0).check_bounds_range(min_zero, max_width),
                ArrayAccessBoundsStatus::Within
            );
            assert_eq!(
                col(7).check_bounds_range(min_zero, max_width),
                ArrayAccessBoundsStatus::Within
            );
            assert_eq!(
                col(8).check_bounds_range(min_zero, max_width),
                ArrayAccessBoundsStatus::Overflowed
            );
        }

        #[test]
        fn test_overflows() {
            // Test basic cases with Index/Length - should mirror is_overflowed_by results
            assert!(!idx(1).overflows(len(3)), "Within bounds");
            assert!(idx(3).overflows(len(3)), "At boundary");
            assert!(idx(5).overflows(len(3)), "Beyond bounds");
            assert!(idx(0).overflows(len(0)), "Empty collection edge case");

            // Test with typed dimensions.
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

            // Test with specific typed combinations.
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
    }

    mod length_operations_tests {
        use super::*;

        #[test]
        fn test_remaining_from() {
            // Test basic cases with Length/Index.
            assert_eq!(
                len(10).remaining_from(idx(3)),
                len(7),
                "Normal case: 7 chars remain from index 3 to 9"
            );
            assert_eq!(
                len(10).remaining_from(idx(9)),
                len(1),
                "Edge case: only 1 char remains at last position"
            );
            assert_eq!(
                len(10).remaining_from(idx(10)),
                len(0),
                "Boundary case: at boundary, nothing remains"
            );
            assert_eq!(
                len(10).remaining_from(idx(15)),
                len(0),
                "Overflow case: beyond boundary, nothing remains"
            );

            // Test edge case: empty length.
            assert_eq!(
                len(0).remaining_from(idx(0)),
                len(0),
                "Empty collection: no chars remain"
            );
            assert_eq!(
                len(0).remaining_from(idx(5)),
                len(0),
                "Empty collection with overflow: no chars remain"
            );

            // Test with typed dimensions - ColWidth/ColIndex.
            let col_width = ColWidth::new(10);
            assert_eq!(
                col_width.remaining_from(ColIndex::new(3)),
                len(7),
                "ColWidth: 7 chars remain from col 3"
            );
            assert_eq!(
                col_width.remaining_from(ColIndex::new(9)),
                len(1),
                "ColWidth: 1 char remains at last col"
            );
            assert_eq!(
                col_width.remaining_from(ColIndex::new(10)),
                len(0),
                "ColWidth: at boundary"
            );
            assert_eq!(
                col_width.remaining_from(ColIndex::new(15)),
                len(0),
                "ColWidth: beyond boundary"
            );

            // Test with typed dimensions - RowHeight/RowIndex.
            let row_height = RowHeight::new(5);
            assert_eq!(
                row_height.remaining_from(RowIndex::new(2)),
                len(3),
                "RowHeight: 3 rows remain from row 2"
            );
            assert_eq!(
                row_height.remaining_from(RowIndex::new(4)),
                len(1),
                "RowHeight: 1 row remains at last row"
            );
            assert_eq!(
                row_height.remaining_from(RowIndex::new(5)),
                len(0),
                "RowHeight: at boundary"
            );
            assert_eq!(
                row_height.remaining_from(RowIndex::new(10)),
                len(0),
                "RowHeight: beyond boundary"
            );

            // Test single element case.
            assert_eq!(
                len(1).remaining_from(idx(0)),
                len(1),
                "Single element: 1 char remains from start"
            );
            assert_eq!(
                len(1).remaining_from(idx(1)),
                len(0),
                "Single element: at boundary"
            );

            // Test specific examples from documentation.
            let max_width = ColWidth::new(10);
            assert_eq!(
                max_width.remaining_from(ColIndex::new(3)),
                len(7),
                "Doc example: remaining_from(3) = 7"
            );
            assert_eq!(
                max_width.remaining_from(ColIndex::new(9)),
                len(1),
                "Doc example: remaining_from(9) = 1"
            );
            assert_eq!(
                max_width.remaining_from(ColIndex::new(10)),
                len(0),
                "Doc example: remaining_from(10) = 0"
            );
        }
    }

    mod conversion_tests {
        use super::*;

        #[test]
        fn test_convert_to_length() {
            // Test basic index to length conversion (0-based to 1-based).
            assert_eq!(
                idx(0).convert_to_length(),
                len(1),
                "Index 0 converts to length 1"
            );
            assert_eq!(
                idx(5).convert_to_length(),
                len(6),
                "Index 5 converts to length 6"
            );
            assert_eq!(
                idx(9).convert_to_length(),
                len(10),
                "Index 9 converts to length 10"
            );
            assert_eq!(
                idx(100).convert_to_length(),
                len(101),
                "Index 100 converts to length 101"
            );

            // Test with typed dimensions - ColIndex to ColWidth.
            assert_eq!(
                ColIndex::new(0).convert_to_length(),
                ColWidth::new(1),
                "ColIndex 0 to ColWidth 1"
            );
            assert_eq!(
                ColIndex::new(5).convert_to_length(),
                ColWidth::new(6),
                "ColIndex 5 to ColWidth 6"
            );
            assert_eq!(
                ColIndex::new(9).convert_to_length(),
                ColWidth::new(10),
                "ColIndex 9 to ColWidth 10"
            );
            assert_eq!(
                ColIndex::new(999).convert_to_length(),
                ColWidth::new(1000),
                "ColIndex 999 to ColWidth 1000"
            );

            // Test with typed dimensions - RowIndex to RowHeight.
            assert_eq!(
                RowIndex::new(0).convert_to_length(),
                RowHeight::new(1),
                "RowIndex 0 to RowHeight 1"
            );
            assert_eq!(
                RowIndex::new(2).convert_to_length(),
                RowHeight::new(3),
                "RowIndex 2 to RowHeight 3"
            );
            assert_eq!(
                RowIndex::new(4).convert_to_length(),
                RowHeight::new(5),
                "RowIndex 4 to RowHeight 5"
            );
            assert_eq!(
                RowIndex::new(49).convert_to_length(),
                RowHeight::new(50),
                "RowIndex 49 to RowHeight 50"
            );

            // Test that the conversion is consistent - converting back should work.
            let original_index = idx(42);
            let converted_length = original_index.convert_to_length();
            let back_to_index = converted_length.convert_to_index();
            assert_eq!(
                back_to_index, original_index,
                "Round-trip conversion should be consistent"
            );

            // Test with typed round-trip conversions.
            let col_index = ColIndex::new(7);
            let col_width = col_index.convert_to_length();
            let back_to_col_index = col_width.convert_to_index();
            assert_eq!(
                back_to_col_index, col_index,
                "ColIndex round-trip should be consistent"
            );

            let row_index = RowIndex::new(3);
            let row_height = row_index.convert_to_length();
            let back_to_row_index = row_height.convert_to_index();
            assert_eq!(
                back_to_row_index, row_index,
                "RowIndex round-trip should be consistent"
            );
        }

        #[test]
        fn test_convert_to_index() {
            // Test basic length to index conversion (1-based to 0-based).
            assert_eq!(
                len(1).convert_to_index(),
                idx(0),
                "Length 1 converts to index 0"
            );
            assert_eq!(
                len(6).convert_to_index(),
                idx(5),
                "Length 6 converts to index 5"
            );
            assert_eq!(
                len(10).convert_to_index(),
                idx(9),
                "Length 10 converts to index 9"
            );
            assert_eq!(
                len(101).convert_to_index(),
                idx(100),
                "Length 101 converts to index 100"
            );

            // Test with typed dimensions - ColWidth to ColIndex.
            assert_eq!(
                ColWidth::new(1).convert_to_index(),
                ColIndex::new(0),
                "ColWidth 1 to ColIndex 0"
            );
            assert_eq!(
                ColWidth::new(6).convert_to_index(),
                ColIndex::new(5),
                "ColWidth 6 to ColIndex 5"
            );
            assert_eq!(
                ColWidth::new(10).convert_to_index(),
                ColIndex::new(9),
                "ColWidth 10 to ColIndex 9"
            );
            assert_eq!(
                ColWidth::new(1000).convert_to_index(),
                ColIndex::new(999),
                "ColWidth 1000 to ColIndex 999"
            );

            // Test with typed dimensions - RowHeight to RowIndex.
            assert_eq!(
                RowHeight::new(1).convert_to_index(),
                RowIndex::new(0),
                "RowHeight 1 to RowIndex 0"
            );
            assert_eq!(
                RowHeight::new(3).convert_to_index(),
                RowIndex::new(2),
                "RowHeight 3 to RowIndex 2"
            );
            assert_eq!(
                RowHeight::new(5).convert_to_index(),
                RowIndex::new(4),
                "RowHeight 5 to RowIndex 4"
            );
            assert_eq!(
                RowHeight::new(50).convert_to_index(),
                RowIndex::new(49),
                "RowHeight 50 to RowIndex 49"
            );

            // Test that the conversion is consistent - converting back should work.
            let original_length = len(42);
            let converted_index = original_length.convert_to_index();
            let back_to_length = converted_index.convert_to_length();
            assert_eq!(
                back_to_length, original_length,
                "Round-trip conversion should be consistent"
            );

            // Test with typed round-trip conversions.
            let col_width = ColWidth::new(8);
            let col_index = col_width.convert_to_index();
            let back_to_col_width = col_index.convert_to_length();
            assert_eq!(
                back_to_col_width, col_width,
                "ColWidth round-trip should be consistent"
            );

            let row_height = RowHeight::new(4);
            let row_index = row_height.convert_to_index();
            let back_to_row_height = row_index.convert_to_length();
            assert_eq!(
                back_to_row_height, row_height,
                "RowHeight round-trip should be consistent"
            );

            // Test edge case: Length 0 should convert to... well, this might not be
            // implemented but if it is, it should be consistent with the type system.
            // Note: Length 0 might be a special case that needs separate handling.
        }

        #[test]
        fn test_as_usize() {
            // Test basic index types conversion to usize.
            assert_eq!(idx(0).as_usize(), 0, "Index 0 as usize");
            assert_eq!(idx(5).as_usize(), 5, "Index 5 as usize");
            assert_eq!(idx(100).as_usize(), 100, "Index 100 as usize");
            assert_eq!(idx(999).as_usize(), 999, "Index 999 as usize");

            // Test basic length types conversion to usize.
            assert_eq!(len(1).as_usize(), 1, "Length 1 as usize");
            assert_eq!(len(6).as_usize(), 6, "Length 6 as usize");
            assert_eq!(len(10).as_usize(), 10, "Length 10 as usize");
            assert_eq!(len(1000).as_usize(), 1000, "Length 1000 as usize");

            // Test typed index conversions - ColIndex.
            assert_eq!(ColIndex::new(0).as_usize(), 0, "ColIndex 0 as usize");
            assert_eq!(ColIndex::new(5).as_usize(), 5, "ColIndex 5 as usize");
            assert_eq!(ColIndex::new(80).as_usize(), 80, "ColIndex 80 as usize");
            assert_eq!(
                ColIndex::new(1024).as_usize(),
                1024,
                "ColIndex 1024 as usize"
            );

            // Test typed index conversions - RowIndex.
            assert_eq!(RowIndex::new(0).as_usize(), 0, "RowIndex 0 as usize");
            assert_eq!(RowIndex::new(3).as_usize(), 3, "RowIndex 3 as usize");
            assert_eq!(RowIndex::new(25).as_usize(), 25, "RowIndex 25 as usize");
            assert_eq!(RowIndex::new(768).as_usize(), 768, "RowIndex 768 as usize");

            // Test typed length conversions - ColWidth.
            assert_eq!(ColWidth::new(1).as_usize(), 1, "ColWidth 1 as usize");
            assert_eq!(ColWidth::new(10).as_usize(), 10, "ColWidth 10 as usize");
            assert_eq!(ColWidth::new(80).as_usize(), 80, "ColWidth 80 as usize");
            assert_eq!(
                ColWidth::new(1920).as_usize(),
                1920,
                "ColWidth 1920 as usize"
            );

            // Test typed length conversions - RowHeight.
            assert_eq!(RowHeight::new(1).as_usize(), 1, "RowHeight 1 as usize");
            assert_eq!(RowHeight::new(5).as_usize(), 5, "RowHeight 5 as usize");
            assert_eq!(RowHeight::new(30).as_usize(), 30, "RowHeight 30 as usize");
            assert_eq!(
                RowHeight::new(1080).as_usize(),
                1080,
                "RowHeight 1080 as usize"
            );

            // Test edge cases.
            assert_eq!(len(0).as_usize(), 0, "Length 0 as usize");
            assert_eq!(ColWidth::new(0).as_usize(), 0, "ColWidth 0 as usize");
            assert_eq!(RowHeight::new(0).as_usize(), 0, "RowHeight 0 as usize");

            // Test that as_usize preserves the underlying numeric value.
            for value in [0, 1, 5, 10, 42, 100, 999] {
                assert_eq!(
                    idx(value).as_usize(),
                    value,
                    "Index {value} preserves value"
                );
                assert_eq!(
                    len(value).as_usize(),
                    value,
                    "Length {value} preserves value"
                );
                assert_eq!(
                    ColIndex::new(value).as_usize(),
                    value,
                    "ColIndex {value} preserves value"
                );
                assert_eq!(
                    ColWidth::new(value).as_usize(),
                    value,
                    "ColWidth {value} preserves value"
                );
                assert_eq!(
                    RowIndex::new(value).as_usize(),
                    value,
                    "RowIndex {value} preserves value"
                );
                assert_eq!(
                    RowHeight::new(value).as_usize(),
                    value,
                    "RowHeight {value} preserves value"
                );
            }
        }

        #[test]
        fn test_clamp_to() {
            // Test basic clamp operations with Length/Length.
            assert_eq!(
                LengthMarker::clamp_to(&len(5), len(10)),
                len(5),
                "Length within bounds - no change"
            );
            assert_eq!(
                LengthMarker::clamp_to(&len(15), len(10)),
                len(10),
                "Length exceeds bounds - gets clamped"
            );
            assert_eq!(
                LengthMarker::clamp_to(&len(8), len(8)),
                len(8),
                "Equal lengths - returns the same value"
            );
            assert_eq!(
                LengthMarker::clamp_to(&len(0), len(5)),
                len(0),
                "Zero length within bounds"
            );
            assert_eq!(
                LengthMarker::clamp_to(&len(0), len(0)),
                len(0),
                "Zero length with zero max"
            );

            // Test with typed length dimensions - ColWidth.
            let col_width_5 = ColWidth::new(5);
            let col_width_10 = ColWidth::new(10);
            let col_width_15 = ColWidth::new(15);

            assert_eq!(
                LengthMarker::clamp_to(&col_width_5, col_width_10),
                col_width_5,
                "ColWidth within bounds - no change"
            );
            assert_eq!(
                LengthMarker::clamp_to(&col_width_15, col_width_10),
                col_width_10,
                "ColWidth exceeds bounds - gets clamped"
            );
            assert_eq!(
                LengthMarker::clamp_to(&col_width_10, col_width_10),
                col_width_10,
                "ColWidth equals bounds - returns max"
            );

            // Test with typed length dimensions - RowHeight.
            let row_height_3 = RowHeight::new(3);
            let row_height_5 = RowHeight::new(5);
            let row_height_7 = RowHeight::new(7);
            let row_height_15 = RowHeight::new(15);
            let row_height_20 = RowHeight::new(20);

            assert_eq!(
                LengthMarker::clamp_to(&row_height_3, row_height_15),
                row_height_3,
                "RowHeight within bounds - no change"
            );
            assert_eq!(
                LengthMarker::clamp_to(&row_height_7, row_height_5),
                row_height_5,
                "RowHeight exceeds smaller bounds - gets clamped"
            );
            assert_eq!(
                LengthMarker::clamp_to(&row_height_20, row_height_15),
                row_height_15,
                "RowHeight exceeds larger bounds - gets clamped"
            );

            // Test edge cases.
            assert_eq!(
                LengthMarker::clamp_to(&len(1), len(1)),
                len(1),
                "Single element case"
            );
            assert_eq!(
                LengthMarker::clamp_to(&len(100), len(1)),
                len(1),
                "Large value clamped to small max"
            );

            // Test that clamp_to always returns a value <= both inputs.
            let test_cases = [(5, 10), (10, 5), (0, 10), (10, 0), (7, 7), (100, 50)];
            for (length_val, max_val) in test_cases {
                let length = len(length_val);
                let max_length = len(max_val);
                let result = LengthMarker::clamp_to(&length, max_length);

                assert!(
                    result.as_usize() <= length.as_usize(),
                    "clamp_to({length_val}, {max_val}) result should be <= original length"
                );
                assert!(
                    result.as_usize() <= max_length.as_usize(),
                    "clamp_to({length_val}, {max_val}) result should be <= max_length"
                );
            }
        }

        #[test]
        fn test_as_u16() {
            // Test basic index types conversion to u16.
            assert_eq!(idx(0).as_u16(), 0, "Index 0 as u16");
            assert_eq!(idx(5).as_u16(), 5, "Index 5 as u16");
            assert_eq!(idx(100).as_u16(), 100, "Index 100 as u16");
            assert_eq!(idx(999).as_u16(), 999, "Index 999 as u16");

            // Test basic length types conversion to u16.
            assert_eq!(len(1).as_u16(), 1, "Length 1 as u16");
            assert_eq!(len(6).as_u16(), 6, "Length 6 as u16");
            assert_eq!(len(10).as_u16(), 10, "Length 10 as u16");
            assert_eq!(len(1000).as_u16(), 1000, "Length 1000 as u16");

            // Test typed index conversions - ColIndex.
            assert_eq!(ColIndex::new(0).as_u16(), 0, "ColIndex 0 as u16");
            assert_eq!(ColIndex::new(5).as_u16(), 5, "ColIndex 5 as u16");
            assert_eq!(ColIndex::new(80).as_u16(), 80, "ColIndex 80 as u16");
            assert_eq!(ColIndex::new(1024).as_u16(), 1024, "ColIndex 1024 as u16");

            // Test typed index conversions - RowIndex.
            assert_eq!(RowIndex::new(0).as_u16(), 0, "RowIndex 0 as u16");
            assert_eq!(RowIndex::new(3).as_u16(), 3, "RowIndex 3 as u16");
            assert_eq!(RowIndex::new(25).as_u16(), 25, "RowIndex 25 as u16");
            assert_eq!(RowIndex::new(768).as_u16(), 768, "RowIndex 768 as u16");

            // Test typed length conversions - ColWidth.
            assert_eq!(ColWidth::new(1).as_u16(), 1, "ColWidth 1 as u16");
            assert_eq!(ColWidth::new(10).as_u16(), 10, "ColWidth 10 as u16");
            assert_eq!(ColWidth::new(80).as_u16(), 80, "ColWidth 80 as u16");
            assert_eq!(ColWidth::new(1920).as_u16(), 1920, "ColWidth 1920 as u16");

            // Test typed length conversions - RowHeight.
            assert_eq!(RowHeight::new(1).as_u16(), 1, "RowHeight 1 as u16");
            assert_eq!(RowHeight::new(5).as_u16(), 5, "RowHeight 5 as u16");
            assert_eq!(RowHeight::new(30).as_u16(), 30, "RowHeight 30 as u16");
            assert_eq!(RowHeight::new(1080).as_u16(), 1080, "RowHeight 1080 as u16");

            // Test edge cases.
            assert_eq!(len(0).as_u16(), 0, "Length 0 as u16");
            assert_eq!(ColWidth::new(0).as_u16(), 0, "ColWidth 0 as u16");
            assert_eq!(RowHeight::new(0).as_u16(), 0, "RowHeight 0 as u16");

            // Test terminal-typical values (crossterm compatibility).
            assert_eq!(ColWidth::new(80).as_u16(), 80, "Standard terminal width 80");
            assert_eq!(ColWidth::new(120).as_u16(), 120, "Wide terminal width 120");
            assert_eq!(
                RowHeight::new(24).as_u16(),
                24,
                "Standard terminal height 24"
            );
            assert_eq!(RowHeight::new(50).as_u16(), 50, "Tall terminal height 50");

            // Test u16 max boundary (65535).
            assert_eq!(len(65535).as_u16(), 65535, "Length u16::MAX as u16");
            assert_eq!(
                ColWidth::new(65535).as_u16(),
                65535,
                "ColWidth u16::MAX as u16"
            );
            assert_eq!(
                RowHeight::new(65535).as_u16(),
                65535,
                "RowHeight u16::MAX as u16"
            );

            // Test that as_u16 preserves the underlying numeric value for typical ranges.
            for value in [0, 1, 5, 10, 42, 80, 100, 120, 1024] {
                assert_eq!(
                    idx(value).as_u16(),
                    u16::try_from(value).unwrap(),
                    "Index {value} preserves value"
                );
                assert_eq!(
                    len(value).as_u16(),
                    u16::try_from(value).unwrap(),
                    "Length {value} preserves value"
                );
                assert_eq!(
                    ColIndex::new(value).as_u16(),
                    u16::try_from(value).unwrap(),
                    "ColIndex {value} preserves value"
                );
                assert_eq!(
                    ColWidth::new(value).as_u16(),
                    u16::try_from(value).unwrap(),
                    "ColWidth {value} preserves value"
                );
                assert_eq!(
                    RowIndex::new(value).as_u16(),
                    u16::try_from(value).unwrap(),
                    "RowIndex {value} preserves value"
                );
                assert_eq!(
                    RowHeight::new(value).as_u16(),
                    u16::try_from(value).unwrap(),
                    "RowHeight {value} preserves value"
                );
            }
        }

        #[test]
        #[allow(clippy::too_many_lines)]
        fn test_is_zero() {
            // Test basic index types - zero values.
            assert!(idx(0).is_zero(), "Index 0 should be zero");
            assert!(!idx(1).is_zero(), "Index 1 should not be zero");
            assert!(!idx(5).is_zero(), "Index 5 should not be zero");
            assert!(!idx(100).is_zero(), "Index 100 should not be zero");

            // Test basic length types - zero and non-zero values.
            assert!(len(0).is_zero(), "Length 0 should be zero");
            assert!(!len(1).is_zero(), "Length 1 should not be zero");
            assert!(!len(5).is_zero(), "Length 5 should not be zero");
            assert!(!len(100).is_zero(), "Length 100 should not be zero");

            // Test typed index types - ColIndex.
            assert!(ColIndex::new(0).is_zero(), "ColIndex 0 should be zero");
            assert!(!ColIndex::new(1).is_zero(), "ColIndex 1 should not be zero");
            assert!(
                !ColIndex::new(10).is_zero(),
                "ColIndex 10 should not be zero"
            );
            assert!(
                !ColIndex::new(80).is_zero(),
                "ColIndex 80 should not be zero"
            );

            // Test typed index types - RowIndex.
            assert!(RowIndex::new(0).is_zero(), "RowIndex 0 should be zero");
            assert!(!RowIndex::new(1).is_zero(), "RowIndex 1 should not be zero");
            assert!(!RowIndex::new(5).is_zero(), "RowIndex 5 should not be zero");
            assert!(
                !RowIndex::new(25).is_zero(),
                "RowIndex 25 should not be zero"
            );

            // Test typed length types - ColWidth.
            assert!(ColWidth::new(0).is_zero(), "ColWidth 0 should be zero");
            assert!(!ColWidth::new(1).is_zero(), "ColWidth 1 should not be zero");
            assert!(
                !ColWidth::new(10).is_zero(),
                "ColWidth 10 should not be zero"
            );
            assert!(
                !ColWidth::new(80).is_zero(),
                "ColWidth 80 should not be zero"
            );
            assert!(
                !ColWidth::new(120).is_zero(),
                "ColWidth 120 should not be zero"
            );

            // Test typed length types - RowHeight.
            assert!(RowHeight::new(0).is_zero(), "RowHeight 0 should be zero");
            assert!(
                !RowHeight::new(1).is_zero(),
                "RowHeight 1 should not be zero"
            );
            assert!(
                !RowHeight::new(5).is_zero(),
                "RowHeight 5 should not be zero"
            );
            assert!(
                !RowHeight::new(24).is_zero(),
                "RowHeight 24 should not be zero"
            );
            assert!(
                !RowHeight::new(50).is_zero(),
                "RowHeight 50 should not be zero"
            );

            // Test edge cases and boundary values.
            assert!(
                !idx(usize::MAX).is_zero(),
                "Index usize::MAX should not be zero"
            );
            assert!(
                !len(usize::MAX).is_zero(),
                "Length usize::MAX should not be zero"
            );
            assert!(
                !ColIndex::new(u16::MAX as usize).is_zero(),
                "ColIndex u16::MAX should not be zero"
            );
            assert!(
                !RowIndex::new(u16::MAX as usize).is_zero(),
                "RowIndex u16::MAX should not be zero"
            );
            assert!(
                !ColWidth::new(u16::MAX as usize).is_zero(),
                "ColWidth u16::MAX should not be zero"
            );
            assert!(
                !RowHeight::new(u16::MAX as usize).is_zero(),
                "RowHeight u16::MAX should not be zero"
            );

            // Test consistency with as_usize() == 0 (the implementation).
            for value in [0, 1, 5, 10, 42, 100, 999] {
                assert_eq!(
                    idx(value).is_zero(),
                    idx(value).as_usize() == 0,
                    "Index {value} is_zero should match as_usize() == 0"
                );
                assert_eq!(
                    len(value).is_zero(),
                    len(value).as_usize() == 0,
                    "Length {value} is_zero should match as_usize() == 0"
                );
                assert_eq!(
                    ColIndex::new(value).is_zero(),
                    ColIndex::new(value).as_usize() == 0,
                    "ColIndex {value} is_zero should match as_usize() == 0"
                );
                assert_eq!(
                    ColWidth::new(value).is_zero(),
                    ColWidth::new(value).as_usize() == 0,
                    "ColWidth {value} is_zero should match as_usize() == 0"
                );
                assert_eq!(
                    RowIndex::new(value).is_zero(),
                    RowIndex::new(value).as_usize() == 0,
                    "RowIndex {value} is_zero should match as_usize() == 0"
                );
                assert_eq!(
                    RowHeight::new(value).is_zero(),
                    RowHeight::new(value).as_usize() == 0,
                    "RowHeight {value} is_zero should match as_usize() == 0"
                );
            }
        }
    }
}

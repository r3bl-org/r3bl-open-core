// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core bounds checking trait and implementation.
//!
//! This module provides the [`BoundsCheck`] trait which defines the interface for
//! both array-style bounds checking and content position checking operations.
//!
//! The trait has a generic implementation that works with any index type implementing
//! [`IndexMarker`] and any length type implementing [`LengthMarker`], ensuring
//! type safety and eliminating code duplication.
//!
//! See the [module documentation] for details on the different bounds checking paradigms.
//!
//! [module documentation]: mod@crate::core::units::bounds_check

use super::{length_and_index_markers::{IndexMarker, LengthMarker},
            result_enums::{ArrayAccessBoundsStatus, CursorPositionBoundsStatus}};

/// Core trait for index bounds validation in TUI applications.
///
/// Provides both array-style bounds checking and cursor position checking.
/// See the [module documentation] for detailed explanations of both paradigms.
///
/// This trait is generic over length types that implement `LengthMarker`,
/// and can only be implemented by index types that implement `IndexMarker`.
/// This ensures type safety and prevents incorrect comparisons between incompatible
/// types.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, ArrayAccessBoundsStatus, RowIndex, RowHeight};
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(row_index.check_array_access_bounds(height), ArrayAccessBoundsStatus::Overflowed);
/// ```
///
/// [module documentation]: mod@crate::core::units::bounds_check
pub trait BoundsCheck<LengthType: LengthMarker>
where
    Self: IndexMarker,
{
    /// Performs comprehensive bounds checking.
    ///
    /// See the [module documentation] for detailed explanation of bounds checking.
    ///
    /// ```text
    /// Array-style bounds checking:
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
    /// check_bounds(length=5)  = Within
    /// check_bounds(length=10) = Overflowed
    /// ```
    ///
    /// # Returns
    /// - [`ArrayAccessBoundsStatus::Within`] if the index can safely access content,
    /// - [`ArrayAccessBoundsStatus::Overflowed`] if the index would exceed array bounds.
    ///
    /// # See Also
    /// For simple boolean overflow checking, use [`overflows`]
    /// which returns a `bool` instead of an enum. This method is designed for cases
    /// where you need to pattern match on the result or explicitly handle the status
    /// information.
    ///
    /// ```rust
    /// use r3bl_tui::{BoundsCheck, ArrayAccessBoundsStatus, IndexMarker, idx, len};
    ///
    /// let index = idx(5);
    /// let length = len(10);
    ///
    /// // For pattern matching or explicit status handling:
    /// match index.check_array_access_bounds(length) {
    ///     ArrayAccessBoundsStatus::Within => println!("Safe to access"),
    ///     ArrayAccessBoundsStatus::Overflowed => println!("Out of bounds"),
    ///     ArrayAccessBoundsStatus::Underflowed => println!("Below minimum"),
    /// }
    ///
    /// // For simple boolean checks:
    /// if !index.overflows(length) {
    ///     println!("Safe to access");
    /// }
    /// ```
    ///
    /// [module documentation]: mod@crate::core::units::bounds_check
    /// [`overflows`]: crate::IndexMarker::overflows
    fn check_array_access_bounds(
        &self,
        arg_max: impl Into<LengthType>,
    ) -> ArrayAccessBoundsStatus;

    /// Performs cursor position bounds checking.
    ///
    /// See the [`bounds_check` module documentation] for detailed explanation of cursor
    /// position checking.
    ///
    /// ```text
    /// Cursor position checking:
    ///
    /// Self
    /// Index:      0   1   2   3   4   5   6   7   8   9   10  11
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │ S │ W │ W │ W │ W │ W │ W │ W │ W │ W │ E │ B │
    ///           ├─▲─┴─▲─┴───┴───┴───┴───┴───┴───┴───┴─▲─┴─▲─┼─▲─┘
    ///           │ │   │                               │   │ │ │
    ///           │Start│                               │  End│Beyond
    ///           │     └────────── Within ─────────────┘     │
    ///           └───────────── content_length=10 ───────────┘
    ///
    /// S = AtStart (index=0)
    /// W = Within (1 ≤ index < 10)
    /// E = AtEnd (index=10)
    /// B = Beyond (index > 10)
    /// ```
    ///
    /// # Returns
    /// [`CursorPositionBoundsStatus`] indicating whether the index is within content,
    /// at a content boundary, or beyond content boundaries.
    ///
    /// [`bounds_check` module documentation]: mod@crate::core::units::bounds_check
    fn check_cursor_position_bounds(
        &self,
        arg_content_length: impl Into<LengthType>,
    ) -> CursorPositionBoundsStatus;
}

/// Generic implementation of [`BoundsCheck`] for any [`IndexMarker`] type with
/// [`LengthMarker`] type.
///
/// This single implementation works with all index and length types that implement the
/// required marker traits, eliminating code duplication and ensuring consistent behavior.
/// The trait system guarantees type safety by only allowing compatible index-length
/// pairs.
impl<IndexType, LengthType> BoundsCheck<LengthType> for IndexType
where
    IndexType: IndexMarker<LengthType = LengthType> + PartialOrd + Copy,
    LengthType: LengthMarker<IndexType = IndexType>,
{
    fn check_array_access_bounds(
        &self,
        arg_max: impl Into<LengthType>,
    ) -> ArrayAccessBoundsStatus {
        let length = arg_max.into();
        // Delegate to overflows() for single source of truth
        // This ensures consistency with all bounds checking methods.
        if self.overflows(length) {
            ArrayAccessBoundsStatus::Overflowed
        } else {
            ArrayAccessBoundsStatus::Within
        }
    }

    fn check_cursor_position_bounds(
        &self,
        arg_content_length: impl Into<LengthType>,
    ) -> CursorPositionBoundsStatus {
        let content_length = arg_content_length.into();
        let position = self.as_usize();
        let length = content_length.as_usize();

        if position > length {
            CursorPositionBoundsStatus::Beyond
        } else if position == 0 {
            CursorPositionBoundsStatus::AtStart
        } else if position == length {
            CursorPositionBoundsStatus::AtEnd
        } else {
            CursorPositionBoundsStatus::Within
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

    #[test]
    fn test_check_cursor_position_bounds_basic() {
        let content_length = len(5);

        // At start.
        assert_eq!(
            idx(0).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::AtStart
        );

        // Within content.
        assert_eq!(
            idx(2).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::Within
        );
        assert_eq!(
            idx(4).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::Within
        );

        // At end boundary.
        assert_eq!(
            idx(5).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::AtEnd
        );

        // Beyond content.
        assert_eq!(
            idx(6).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::Beyond
        );
        assert_eq!(
            idx(10).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::Beyond
        );
    }

    #[test]
    fn test_check_cursor_position_bounds_edge_cases() {
        // Zero-length content - AtStart takes precedence.
        let zero_length = len(0);
        assert_eq!(
            idx(0).check_cursor_position_bounds(zero_length),
            CursorPositionBoundsStatus::AtStart
        );
        assert_eq!(
            idx(1).check_cursor_position_bounds(zero_length),
            CursorPositionBoundsStatus::Beyond
        );

        // Single element content.
        let single_length = len(1);
        assert_eq!(
            idx(0).check_cursor_position_bounds(single_length),
            CursorPositionBoundsStatus::AtStart
        );
        assert_eq!(
            idx(1).check_cursor_position_bounds(single_length),
            CursorPositionBoundsStatus::AtEnd
        );
        assert_eq!(
            idx(2).check_cursor_position_bounds(single_length),
            CursorPositionBoundsStatus::Beyond
        );
    }

    #[test]
    fn test_check_cursor_position_bounds_with_typed_indices() {
        // Test with ColIndex/ColWidth.
        let col_width = ColWidth::new(3);
        assert_eq!(
            ColIndex::new(0).check_cursor_position_bounds(col_width),
            CursorPositionBoundsStatus::AtStart
        );
        assert_eq!(
            ColIndex::new(2).check_cursor_position_bounds(col_width),
            CursorPositionBoundsStatus::Within
        );
        assert_eq!(
            ColIndex::new(3).check_cursor_position_bounds(col_width),
            CursorPositionBoundsStatus::AtEnd
        );
        assert_eq!(
            ColIndex::new(4).check_cursor_position_bounds(col_width),
            CursorPositionBoundsStatus::Beyond
        );

        // Test with RowIndex/RowHeight.
        let row_height = RowHeight::new(2);
        assert_eq!(
            RowIndex::new(0).check_cursor_position_bounds(row_height),
            CursorPositionBoundsStatus::AtStart
        );
        assert_eq!(
            RowIndex::new(1).check_cursor_position_bounds(row_height),
            CursorPositionBoundsStatus::Within
        );
        assert_eq!(
            RowIndex::new(2).check_cursor_position_bounds(row_height),
            CursorPositionBoundsStatus::AtEnd
        );
        assert_eq!(
            RowIndex::new(3).check_cursor_position_bounds(row_height),
            CursorPositionBoundsStatus::Beyond
        );
    }

    /// Comprehensive tests to ensure consistency between all bounds checking methods:
    /// - `check_array_access_bounds()`
    /// - `overflows()`
    /// - `is_overflowed_by()`
    #[test]
    fn test_bounds_checking_consistency() {
        // Test critical boundary cases with generic Index/Length.
        let test_cases = [
            // (index, length, expected_overflows).
            (0, 1, false), // First valid index
            (0, 5, false), // First valid index in larger array
            (4, 5, false), // Last valid index (length-1)
            (5, 5, true),  // First invalid index (length)
            (6, 5, true),  // Beyond bounds
            (0, 0, true),  // Empty collection edge case
            (1, 0, true),  // Index in empty collection
        ];

        for (index_val, length_val, expected_overflows) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);

            // Test overflows() method.
            let overflows_result = index.overflows(length);
            assert_eq!(
                overflows_result, expected_overflows,
                "overflows mismatch for idx({index_val}).overflows(len({length_val}))"
            );

            // Test is_overflowed_by() method (inverse relationship).
            let is_overflowed_result = length.is_overflowed_by(index);
            assert_eq!(
                is_overflowed_result, expected_overflows,
                "is_overflowed_by mismatch for len({length_val}).is_overflowed_by(idx({index_val}))"
            );

            // Test check_array_access_bounds() consistency.
            let bounds_status = index.check_array_access_bounds(length);
            let expected_status = if expected_overflows {
                ArrayAccessBoundsStatus::Overflowed
            } else {
                ArrayAccessBoundsStatus::Within
            };
            assert_eq!(
                bounds_status, expected_status,
                "check_array_access_bounds mismatch for idx({index_val}).check_array_access_bounds(len({length_val}))"
            );
        }
    }

    #[test]
    fn test_typed_bounds_checking_consistency() {
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        // Test with ColIndex/ColWidth
        let col_cases = [
            (0, 3, false), // First valid
            (2, 3, false), // Last valid
            (3, 3, true),  // First invalid
            (0, 0, true),  // Empty
        ];

        for (index_val, width_val, expected_overflows) in col_cases {
            let col_index = ColIndex::new(index_val);
            let col_width = ColWidth::new(width_val);

            let overflows_result = col_index.overflows(col_width);
            let is_overflowed_result = col_width.is_overflowed_by(col_index);
            let bounds_status = col_index.check_array_access_bounds(col_width);

            assert_eq!(
                overflows_result, expected_overflows,
                "ColIndex overflows mismatch for {index_val}:{width_val}"
            );
            assert_eq!(
                is_overflowed_result, expected_overflows,
                "ColWidth is_overflowed_by mismatch for {width_val}:{index_val}"
            );

            let expected_status = if expected_overflows {
                ArrayAccessBoundsStatus::Overflowed
            } else {
                ArrayAccessBoundsStatus::Within
            };
            assert_eq!(
                bounds_status, expected_status,
                "ColIndex check_array_access_bounds mismatch for {index_val}:{width_val}"
            );
        }

        // Test with RowIndex/RowHeight
        let row_cases = [
            (0, 2, false), // First valid
            (1, 2, false), // Last valid
            (2, 2, true),  // First invalid
        ];

        for (index_val, height_val, expected_overflows) in row_cases {
            let row_index = RowIndex::new(index_val);
            let row_height = RowHeight::new(height_val);

            let overflows_result = row_index.overflows(row_height);
            let is_overflowed_result = row_height.is_overflowed_by(row_index);
            let bounds_status = row_index.check_array_access_bounds(row_height);

            assert_eq!(
                overflows_result, expected_overflows,
                "RowIndex overflows mismatch for {index_val}:{height_val}"
            );
            assert_eq!(
                is_overflowed_result, expected_overflows,
                "RowHeight is_overflowed_by mismatch for {height_val}:{index_val}"
            );

            let expected_status = if expected_overflows {
                ArrayAccessBoundsStatus::Overflowed
            } else {
                ArrayAccessBoundsStatus::Within
            };
            assert_eq!(
                bounds_status, expected_status,
                "RowIndex check_array_access_bounds mismatch for {index_val}:{height_val}"
            );
        }
    }
}

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
//! See the [`bounds_check` module documentation](crate::core::units::bounds_check)
//! for details on the different bounds checking paradigms.

use super::{length_and_index_markers::{IndexMarker, LengthMarker},
            result_enums::{ArrayAccessBoundsStatus, CursorPositionBoundsStatus}};

/// Core trait for index bounds validation in TUI applications.
///
/// Provides both array-style bounds checking and cursor position checking.
/// See the [`bounds_check` module documentation](crate::core::units::bounds_check)
/// for detailed explanations of both paradigms.
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
pub trait BoundsCheck<LengthType: LengthMarker>
where
    Self: IndexMarker,
{
    /// Performs comprehensive bounds checking.
    ///
    /// See the [`bounds_check` module documentation](crate::core::units::bounds_check)
    /// for detailed explanation of bounds checking.
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
    /// For simple boolean overflow checking, use [`crate::IndexMarker::overflows()`]
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
    fn check_array_access_bounds(&self, max: LengthType) -> ArrayAccessBoundsStatus;

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
        content_length: LengthType,
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
    IndexType: IndexMarker + PartialOrd + Copy,
    LengthType: LengthMarker<IndexType = IndexType>,
{
    fn check_array_access_bounds(&self, length: LengthType) -> ArrayAccessBoundsStatus {
        let this = *self;
        let other = length.convert_to_index();

        // For now, we only check overflow since indices are typically unsigned.
        // Future versions might add underflow checking for signed scenarios.
        if this > other {
            ArrayAccessBoundsStatus::Overflowed
        } else {
            ArrayAccessBoundsStatus::Within
        }
    }

    fn check_cursor_position_bounds(
        &self,
        content_length: LengthType,
    ) -> CursorPositionBoundsStatus {
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

        // At start
        assert_eq!(
            idx(0).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::AtStart
        );

        // Within content
        assert_eq!(
            idx(2).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::Within
        );
        assert_eq!(
            idx(4).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::Within
        );

        // At end boundary
        assert_eq!(
            idx(5).check_cursor_position_bounds(content_length),
            CursorPositionBoundsStatus::AtEnd
        );

        // Beyond content
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
        // Test with ColIndex/ColWidth
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

        // Test with RowIndex/RowHeight
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
}

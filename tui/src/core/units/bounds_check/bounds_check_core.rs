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
            result_enums::{BoundsOverflowStatus, ContentPositionStatus}};

/// Core trait for index bounds validation in TUI applications.
///
/// Provides both array-style bounds checking and content position checking.
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
/// use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, RowIndex, RowHeight};
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(row_index.check_overflows(height), BoundsOverflowStatus::Overflowed);
/// ```
pub trait BoundsCheck<LengthType: LengthMarker>
where
    Self: IndexMarker,
{
    /// Performs array-style bounds checking.
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
    /// check_overflows(length=5)  = Within
    /// check_overflows(length=10) = Overflowed
    /// ```
    ///
    /// # Returns
    /// - [`BoundsOverflowStatus::Within`] if the index can safely access content,
    /// - [`BoundsOverflowStatus::Overflowed`] if the index would exceed array bounds.
    fn check_overflows(&self, max: LengthType) -> BoundsOverflowStatus;

    /// Performs content position checking.
    ///
    /// See the [`bounds_check` module documentation](crate::core::units::bounds_check)
    /// for detailed explanation of content position checking.
    ///
    /// ```text
    /// Content position checking:
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
    /// [`ContentPositionStatus`] indicating whether the index is within content,
    /// at a content boundary, or beyond content boundaries.
    fn check_content_position(&self, content_length: LengthType)
    -> ContentPositionStatus;
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
    fn check_overflows(&self, length: LengthType) -> BoundsOverflowStatus {
        let this = *self;
        let other = length.convert_to_index();
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(
        &self,
        content_length: LengthType,
    ) -> ContentPositionStatus {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

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
        // Zero-length content - AtStart takes precedence.
        let zero_length = len(0);
        assert_eq!(
            idx(0).check_content_position(zero_length),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            idx(1).check_content_position(zero_length),
            ContentPositionStatus::Beyond
        );

        // Single element content.
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
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides several result enums for different bounds checking scenarios:
//! - [`ArrayOverflowResult`] - Results for simple array access bounds checking (within,
//!   overflow)
//! - [`ArrayUnderflowResult`] - Results for checking if an index is below a minimum bound
//! - [`RangeBoundsResult`] - Results for range bounds checking (underflow, within,
//!   overflow)
//! - [`RangeValidityStatus`] - Results for validating range structure and bounds
//!   correctness
//! - [`CursorPositionBoundsStatus`] - Results for cursor position bounds checking

/// Result of simple array access bounds checking `[0, length)`.
///
/// Used with [`overflows()`] to determine if an index is within valid bounds for
/// accessing array elements. This enum provides a two-state result type that matches what
/// array access actually needs: either the index is valid (within bounds) or it
/// overflows.
///
/// Unlike [`RangeBoundsResult`], this type only has two variants because array access
/// always starts at index 0 - there's no concept of "underflow" when checking `[0,
/// length)`. See the [Interval Notation] section in the module documentation for notation
/// details.
///
/// ## Examples
///
/// ```
/// use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, idx, len};
///
/// let index = idx(5);
/// let length = len(10);
/// assert_eq!(index.overflows(length), ArrayOverflowResult::Within);
///
/// let large_index = idx(10);
/// assert_eq!(large_index.overflows(length), ArrayOverflowResult::Overflowed);
/// ```
///
/// [`overflows()`]: crate::ArrayBoundsCheck::overflows
/// [`RangeBoundsResult`]: crate::RangeBoundsResult
/// [Interval Notation]: mod@crate::core::coordinates::bounds_check#interval-notation
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ArrayOverflowResult {
    /// Index is within valid bounds for array access.
    Within,

    /// Index has overflowed (exceeded maximum bounds).
    Overflowed,
}

/// Result of array underflow bounds checking.
///
/// Used with [`underflows`] to determine if an index is below a minimum bound.
/// This is a two-state result type used for checking if an index has underflowed
/// (gone below) a given minimum boundary.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{ArrayBoundsCheck, ArrayUnderflowResult, row};
///
/// let min_row = row(3);
/// assert_eq!(row(2).underflows(min_row), ArrayUnderflowResult::Underflowed);
/// assert_eq!(row(3).underflows(min_row), ArrayUnderflowResult::Within);
/// assert_eq!(row(5).underflows(min_row), ArrayUnderflowResult::Within);
/// ```
///
/// [`underflows`]: crate::ArrayBoundsCheck::underflows
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ArrayUnderflowResult {
    /// Index is at or above the minimum bound.
    Within,

    /// Index has underflowed (below minimum bound).
    Underflowed,
}

/// Result of range bounds checking `[min, max)` or `[min, max]`.
///
/// See the [Interval Notation] section in the module documentation for notation details.
///
/// Used with [`check_index_is_within`] to determine if an index
/// falls within a specific range. This three-state result type can distinguish between
/// three cases: below the range (underflow), within the range, or above the range
/// (overflow).
///
/// Unlike [`ArrayOverflowResult`], this type has three variants because range checking
/// involves both a minimum and maximum bound - an index can be below min (underflow),
/// between min and max (within), or at/above max (overflow).
///
/// # Examples
///
/// ```
/// use r3bl_tui::{RangeBoundsExt, RangeBoundsResult, idx};
///
/// let index = idx(5);
/// let range = idx(3)..idx(8);
///
/// // Check within range [3, 8)
/// assert_eq!(range.check_index_is_within(index), RangeBoundsResult::Within);
///
/// let low_index = idx(2);
/// assert_eq!(range.check_index_is_within(low_index), RangeBoundsResult::Underflowed);
///
/// let high_index = idx(8);
/// assert_eq!(range.check_index_is_within(high_index), RangeBoundsResult::Overflowed);
/// ```
///
/// [`check_index_is_within`]: crate::RangeBoundsExt::check_index_is_within
/// [`ArrayOverflowResult`]: crate::ArrayOverflowResult
/// [Interval Notation]: mod@crate::core::coordinates::bounds_check#interval-notation
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RangeBoundsResult {
    /// Index has underflowed (below minimum bounds).
    Underflowed,

    /// Index is within valid range bounds.
    Within,

    /// Index has overflowed (exceeded maximum bounds).
    Overflowed,
}

/// Result of range structure validation operations.
///
/// Used with [`check_range_is_valid_for_length`] to determine if a range is well-formed
/// and valid for a given buffer/content length. This captures WHY a range might be
/// invalid, enabling precise error handling without re-checking.
///
/// Unlike simple boolean validation, this enum distinguishes between different failure
/// modes, allowing callers to provide specific error messages or take corrective action.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{RangeBoundsExt, RangeValidityStatus, col, width};
///
/// let buffer_length = width(10);
///
/// // Valid range
/// let range = col(2)..col(7);
/// assert_eq!(range.check_range_is_valid_for_length(buffer_length), RangeValidityStatus::Valid);
///
/// // Inverted range
/// let inverted = col(8)..col(3);
/// assert_eq!(inverted.check_range_is_valid_for_length(buffer_length), RangeValidityStatus::Inverted);
///
/// // Start out of bounds
/// let bad_start = col(15)..col(20);
/// assert_eq!(bad_start.check_range_is_valid_for_length(buffer_length), RangeValidityStatus::StartOutOfBounds);
///
/// // End out of bounds
/// let bad_end = col(5)..col(15);
/// assert_eq!(bad_end.check_range_is_valid_for_length(buffer_length), RangeValidityStatus::EndOutOfBounds);
/// ```
///
/// [`check_range_is_valid_for_length`]: crate::RangeBoundsExt::check_range_is_valid_for_length
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RangeValidityStatus {
    /// Range is well-formed and valid for the given buffer length.
    Valid,

    /// Range is inverted (start > end).
    Inverted,

    /// Start index is out of bounds (start >= `buffer_length`).
    StartOutOfBounds,

    /// End index is out of bounds.
    /// - For `Range<I>` (exclusive): end > `buffer_length`
    /// - For `RangeInclusive<I>` (inclusive): end >= `buffer_length`
    EndOutOfBounds,
}

/// Result of cursor position bounds checking operations.
///
/// Used with [`check_cursor_position_bounds`] to determine the relationship between an
/// index and content boundaries. Essential for text editing and cursor positioning where
/// distinguishing between "at end" and "beyond" is crucial.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{CursorBoundsCheck, CursorPositionBoundsStatus, idx, len};
///
/// let content = len(5);
///
/// assert_eq!(content.check_cursor_position_bounds(idx(0)), CursorPositionBoundsStatus::AtStart);
/// assert_eq!(content.check_cursor_position_bounds(idx(3)), CursorPositionBoundsStatus::Within);
/// assert_eq!(content.check_cursor_position_bounds(idx(5)), CursorPositionBoundsStatus::AtEnd);
/// assert_eq!(content.check_cursor_position_bounds(idx(7)), CursorPositionBoundsStatus::Beyond);
/// ```
///
/// [`check_cursor_position_bounds`]: crate::CursorBoundsCheck::check_cursor_position_bounds
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CursorPositionBoundsStatus {
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
    use crate::{ArrayBoundsCheck, idx, len};

    mod array_overflow_result_tests {
        use super::*;

        #[test]
        fn test_array_overflow_result_equality() {
            assert_eq!(ArrayOverflowResult::Within, ArrayOverflowResult::Within);
            assert_eq!(
                ArrayOverflowResult::Overflowed,
                ArrayOverflowResult::Overflowed
            );
            assert_ne!(ArrayOverflowResult::Within, ArrayOverflowResult::Overflowed);
        }

        #[test]
        fn test_array_overflow_result_copy() {
            let status1 = ArrayOverflowResult::Within;
            let status2 = status1;
            assert_eq!(status1, status2);

            let status3 = ArrayOverflowResult::Overflowed;
            let status4 = status3;
            assert_eq!(status3, status4);
        }

        #[test]
        fn test_array_overflow_result_debug() {
            assert_eq!(format!("{:?}", ArrayOverflowResult::Within), "Within");
            assert_eq!(
                format!("{:?}", ArrayOverflowResult::Overflowed),
                "Overflowed"
            );
        }

        #[test]
        fn test_array_overflow_result_with_overflows() {
            // Verify that overflows() returns ArrayOverflowResult
            let index = idx(5);
            let length = len(10);
            assert_eq!(index.overflows(length), ArrayOverflowResult::Within);

            let large_index = idx(10);
            assert_eq!(
                large_index.overflows(length),
                ArrayOverflowResult::Overflowed
            );
        }
    }

    mod array_underflow_result_tests {
        use super::*;

        #[test]
        fn test_array_underflow_result_equality() {
            assert_eq!(ArrayUnderflowResult::Within, ArrayUnderflowResult::Within);
            assert_eq!(
                ArrayUnderflowResult::Underflowed,
                ArrayUnderflowResult::Underflowed
            );
            assert_ne!(
                ArrayUnderflowResult::Within,
                ArrayUnderflowResult::Underflowed
            );
        }

        #[test]
        fn test_array_underflow_result_copy() {
            let status1 = ArrayUnderflowResult::Within;
            let status2 = status1;
            assert_eq!(status1, status2);

            let status3 = ArrayUnderflowResult::Underflowed;
            let status4 = status3;
            assert_eq!(status3, status4);
        }

        #[test]
        fn test_array_underflow_result_debug() {
            assert_eq!(format!("{:?}", ArrayUnderflowResult::Within), "Within");
            assert_eq!(
                format!("{:?}", ArrayUnderflowResult::Underflowed),
                "Underflowed"
            );
        }
    }

    mod range_bounds_result_tests {
        use super::*;
        use crate::RangeBoundsExt;

        #[test]
        fn test_range_bounds_result_equality() {
            assert_eq!(RangeBoundsResult::Within, RangeBoundsResult::Within);
            assert_eq!(RangeBoundsResult::Overflowed, RangeBoundsResult::Overflowed);
            assert_eq!(
                RangeBoundsResult::Underflowed,
                RangeBoundsResult::Underflowed
            );
            assert_ne!(RangeBoundsResult::Within, RangeBoundsResult::Overflowed);
            assert_ne!(RangeBoundsResult::Within, RangeBoundsResult::Underflowed);
            assert_ne!(
                RangeBoundsResult::Overflowed,
                RangeBoundsResult::Underflowed
            );
        }

        #[test]
        fn test_range_bounds_result_copy() {
            let status1 = RangeBoundsResult::Within;
            let status2 = status1;
            assert_eq!(status1, status2);

            let status3 = RangeBoundsResult::Overflowed;
            let status4 = status3;
            assert_eq!(status3, status4);

            let status5 = RangeBoundsResult::Underflowed;
            let status6 = status5;
            assert_eq!(status5, status6);
        }

        #[test]
        fn test_range_bounds_result_debug() {
            assert_eq!(format!("{:?}", RangeBoundsResult::Within), "Within");
            assert_eq!(format!("{:?}", RangeBoundsResult::Overflowed), "Overflowed");
            assert_eq!(
                format!("{:?}", RangeBoundsResult::Underflowed),
                "Underflowed"
            );
        }

        #[test]
        fn test_range_bounds_result_with_check_index_is_within() {
            // Verify that check_index_is_within returns RangeBoundsResult
            let range = idx(3)..idx(8);

            let index = idx(5);
            assert_eq!(
                range.check_index_is_within(index),
                RangeBoundsResult::Within
            );

            let low_index = idx(2);
            assert_eq!(
                range.check_index_is_within(low_index),
                RangeBoundsResult::Underflowed
            );

            let high_index = idx(8);
            assert_eq!(
                range.check_index_is_within(high_index),
                RangeBoundsResult::Overflowed
            );
        }
    }
}

#[cfg(test)]
mod cursor_position_bounds_status_tests {
    use super::*;
    use crate::{ColIndex, ColWidth, CursorBoundsCheck, RowHeight, RowIndex, idx, len};

    #[test]
    fn test_cursor_position_bounds_status_equality() {
        assert_eq!(
            CursorPositionBoundsStatus::AtStart,
            CursorPositionBoundsStatus::AtStart
        );
        assert_eq!(
            CursorPositionBoundsStatus::Within,
            CursorPositionBoundsStatus::Within
        );
        assert_eq!(
            CursorPositionBoundsStatus::AtEnd,
            CursorPositionBoundsStatus::AtEnd
        );
        assert_eq!(
            CursorPositionBoundsStatus::Beyond,
            CursorPositionBoundsStatus::Beyond
        );
        assert_ne!(
            CursorPositionBoundsStatus::AtStart,
            CursorPositionBoundsStatus::Within
        );
        assert_ne!(
            CursorPositionBoundsStatus::Within,
            CursorPositionBoundsStatus::AtEnd
        );
        assert_ne!(
            CursorPositionBoundsStatus::AtEnd,
            CursorPositionBoundsStatus::Beyond
        );
        assert_ne!(
            CursorPositionBoundsStatus::AtStart,
            CursorPositionBoundsStatus::Beyond
        );
    }

    #[test]
    fn test_cursor_position_bounds_status_copy() {
        let status1 = CursorPositionBoundsStatus::AtStart;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = CursorPositionBoundsStatus::Within;
        let status4 = status3;
        assert_eq!(status3, status4);

        let status5 = CursorPositionBoundsStatus::AtEnd;
        let status6 = status5;
        assert_eq!(status5, status6);

        let status7 = CursorPositionBoundsStatus::Beyond;
        let status8 = status7;
        assert_eq!(status7, status8);
    }

    #[test]
    fn test_cursor_position_bounds_status_debug() {
        assert_eq!(
            format!("{:?}", CursorPositionBoundsStatus::AtStart),
            "AtStart"
        );
        assert_eq!(
            format!("{:?}", CursorPositionBoundsStatus::Within),
            "Within"
        );
        assert_eq!(format!("{:?}", CursorPositionBoundsStatus::AtEnd), "AtEnd");
        assert_eq!(
            format!("{:?}", CursorPositionBoundsStatus::Beyond),
            "Beyond"
        );
    }

    #[test]
    fn test_cursor_position_bounds_status_empty_content_precedence() {
        // Test that AtStart takes precedence over AtEnd for empty content.
        let empty_length = len(0);
        assert_eq!(
            empty_length.check_cursor_position_bounds(idx(0)),
            CursorPositionBoundsStatus::AtStart
        );

        // Test with typed indices too.

        let empty_col_width = ColWidth::new(0);
        assert_eq!(
            empty_col_width.check_cursor_position_bounds(ColIndex::new(0)),
            CursorPositionBoundsStatus::AtStart
        );

        let empty_row_height = RowHeight::new(0);
        assert_eq!(
            empty_row_height.check_cursor_position_bounds(RowIndex::new(0)),
            CursorPositionBoundsStatus::AtStart
        );
    }

    #[test]
    fn test_cursor_position_bounds_status_comprehensive() {
        // Test all combinations for a length-3 content.
        let content_length = len(3);

        // AtStart: index == 0
        assert_eq!(
            content_length.check_cursor_position_bounds(idx(0)),
            CursorPositionBoundsStatus::AtStart
        );

        // Within: 0 < index < length
        assert_eq!(
            content_length.check_cursor_position_bounds(idx(1)),
            CursorPositionBoundsStatus::Within
        );
        assert_eq!(
            content_length.check_cursor_position_bounds(idx(2)),
            CursorPositionBoundsStatus::Within
        );

        // AtEnd: index == length && index > 0
        assert_eq!(
            content_length.check_cursor_position_bounds(idx(3)),
            CursorPositionBoundsStatus::AtEnd
        );

        // Beyond: index > length
        assert_eq!(
            content_length.check_cursor_position_bounds(idx(4)),
            CursorPositionBoundsStatus::Beyond
        );
        assert_eq!(
            content_length.check_cursor_position_bounds(idx(10)),
            CursorPositionBoundsStatus::Beyond
        );
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Status enums for bounds checking operations.
//!
//! This module provides the result types for bounds checking operations:
//! - [`ArrayAccessBoundsStatus`] - Results for array access bounds checking (underflow,
//!   within, overflow)
//! - [`CursorPositionBoundsStatus`] - Results for cursor position bounds checking
//!
//! See the [module documentation] for details on the different bounds checking paradigms.
//!
//! [module documentation]: mod@crate::core::units::bounds_check

/// Result of array access bounds checking operations.
///
/// Used with [`check_array_access_bounds`] to determine if an index
/// is within valid bounds for accessing array elements, has underflowed (gone below
/// minimum), or overflowed (exceeded maximum). See the [module documentation] for details on the bounds checking
/// paradigms.
///
/// [`check_array_access_bounds`]: crate::BoundsCheck::check_array_access_bounds
/// [module documentation]: mod@crate::core::units::bounds_check
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, ArrayAccessBoundsStatus, idx, len};
///
/// let index = idx(5);
/// let length = len(10);
/// assert_eq!(index.check_array_access_bounds(length), ArrayAccessBoundsStatus::Within);
///
/// let large_index = idx(10);
/// assert_eq!(large_index.check_array_access_bounds(length), ArrayAccessBoundsStatus::Overflowed);
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ArrayAccessBoundsStatus {
    /// Index has underflowed (below minimum bounds).
    /// Used when checking against a minimum bound and the index falls below it.
    Underflowed,

    /// Index is within valid bounds.
    Within,

    /// Index has overflowed (exceeded maximum bounds).
    Overflowed,
}

/// Result of cursor position bounds checking operations.
///
/// Used with [`check_cursor_position_bounds`] to determine the
/// relationship between an index and content boundaries. Essential for text editing and
/// cursor positioning where distinguishing between "at end" and "beyond" is crucial.
///
/// See the [module documentation] for details on
/// cursor position checking vs array-style bounds checking.
///
/// [`check_cursor_position_bounds`]: crate::BoundsCheck::check_cursor_position_bounds
/// [module documentation]: mod@crate::core::units::bounds_check
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, CursorPositionBoundsStatus, idx, len};
///
/// let content = len(5);
///
/// assert_eq!(idx(0).check_cursor_position_bounds(content), CursorPositionBoundsStatus::AtStart);
/// assert_eq!(idx(3).check_cursor_position_bounds(content), CursorPositionBoundsStatus::Within);
/// assert_eq!(idx(5).check_cursor_position_bounds(content), CursorPositionBoundsStatus::AtEnd);
/// assert_eq!(idx(7).check_cursor_position_bounds(content), CursorPositionBoundsStatus::Beyond);
/// ```
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
    use crate::{BoundsCheck, ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

    mod array_access_bounds_status_tests {
        use super::*;

        #[test]
        fn test_array_access_bounds_status_equality() {
            assert_eq!(
                ArrayAccessBoundsStatus::Within,
                ArrayAccessBoundsStatus::Within
            );
            assert_eq!(
                ArrayAccessBoundsStatus::Overflowed,
                ArrayAccessBoundsStatus::Overflowed
            );
            assert_eq!(
                ArrayAccessBoundsStatus::Underflowed,
                ArrayAccessBoundsStatus::Underflowed
            );
            assert_ne!(
                ArrayAccessBoundsStatus::Within,
                ArrayAccessBoundsStatus::Overflowed
            );
            assert_ne!(
                ArrayAccessBoundsStatus::Within,
                ArrayAccessBoundsStatus::Underflowed
            );
            assert_ne!(
                ArrayAccessBoundsStatus::Overflowed,
                ArrayAccessBoundsStatus::Underflowed
            );
        }

        #[test]
        fn test_array_access_bounds_status_copy() {
            let status1 = ArrayAccessBoundsStatus::Within;
            let status2 = status1;
            assert_eq!(status1, status2);

            let status3 = ArrayAccessBoundsStatus::Overflowed;
            let status4 = status3;
            assert_eq!(status3, status4);

            let status5 = ArrayAccessBoundsStatus::Underflowed;
            let status6 = status5;
            assert_eq!(status5, status6);
        }

        #[test]
        fn test_array_access_bounds_status_debug() {
            assert_eq!(format!("{:?}", ArrayAccessBoundsStatus::Within), "Within");
            assert_eq!(
                format!("{:?}", ArrayAccessBoundsStatus::Overflowed),
                "Overflowed"
            );
            assert_eq!(
                format!("{:?}", ArrayAccessBoundsStatus::Underflowed),
                "Underflowed"
            );
        }
    }

    mod cursor_position_bounds_status_tests {
        use super::*;

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
                idx(0).check_cursor_position_bounds(empty_length),
                CursorPositionBoundsStatus::AtStart
            );

            // Test with typed indices too.

            let empty_col_width = ColWidth::new(0);
            assert_eq!(
                ColIndex::new(0).check_cursor_position_bounds(empty_col_width),
                CursorPositionBoundsStatus::AtStart
            );

            let empty_row_height = RowHeight::new(0);
            assert_eq!(
                RowIndex::new(0).check_cursor_position_bounds(empty_row_height),
                CursorPositionBoundsStatus::AtStart
            );
        }

        #[test]
        fn test_cursor_position_bounds_status_comprehensive() {
            // Test all combinations for a length-3 content.
            let content_length = len(3);

            // AtStart: index == 0
            assert_eq!(
                idx(0).check_cursor_position_bounds(content_length),
                CursorPositionBoundsStatus::AtStart
            );

            // Within: 0 < index < length
            assert_eq!(
                idx(1).check_cursor_position_bounds(content_length),
                CursorPositionBoundsStatus::Within
            );
            assert_eq!(
                idx(2).check_cursor_position_bounds(content_length),
                CursorPositionBoundsStatus::Within
            );

            // AtEnd: index == length && index > 0
            assert_eq!(
                idx(3).check_cursor_position_bounds(content_length),
                CursorPositionBoundsStatus::AtEnd
            );

            // Beyond: index > length
            assert_eq!(
                idx(4).check_cursor_position_bounds(content_length),
                CursorPositionBoundsStatus::Beyond
            );
            assert_eq!(
                idx(10).check_cursor_position_bounds(content_length),
                CursorPositionBoundsStatus::Beyond
            );
        }
    }
}

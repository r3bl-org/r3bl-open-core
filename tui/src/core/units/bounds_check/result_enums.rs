// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Status enums for bounds checking operations.
//!
//! This module provides the result types for bounds checking operations:
//! - [`BoundsOverflowStatus`] - Results for array-style bounds checking
//! - [`ContentPositionStatus`] - Results for content position checking
//!
//! See the [`bounds_check` module documentation](crate::core::units::bounds_check) for
//! details on the different bounds checking paradigms.

/// Result of array-style bounds checking operations.
///
/// Used with [`crate::BoundsCheck::check_overflows`] to determine if an index can safely
/// access array content. See the [module documentation](crate::core::units::bounds_check)
/// for details on the bounds checking paradigms.
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

/// Result of content position checking operations.
///
/// Used with [`crate::BoundsCheck::check_content_position`] to determine the relationship
/// between an index and content boundaries. Essential for text editing and cursor
/// positioning where distinguishing between "at end" and "beyond" is crucial.
///
/// See the [module documentation](crate::core::units::bounds_check) for details on
/// content position checking vs array-style bounds checking.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, ContentPositionStatus, idx, len};
///
/// let content = len(5);
///
/// assert_eq!(idx(0).check_content_position(content), ContentPositionStatus::AtStart);
/// assert_eq!(idx(3).check_content_position(content), ContentPositionStatus::Within);
/// assert_eq!(idx(5).check_content_position(content), ContentPositionStatus::AtEnd);
/// assert_eq!(idx(7).check_content_position(content), ContentPositionStatus::Beyond);
/// ```
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
    use crate::{BoundsCheck, ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

    mod bounds_overflow_status_tests {
        use super::*;

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
    }

    mod content_position_status_tests {
        use super::*;

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
        fn test_position_status_empty_content_precedence() {
            // Test that AtStart takes precedence over AtEnd for empty content.
            let empty_length = len(0);
            assert_eq!(
                idx(0).check_content_position(empty_length),
                ContentPositionStatus::AtStart
            );

            // Test with typed indices too.

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
            // Test all combinations for a length-3 content.
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
}

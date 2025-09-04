// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module defines the [`BoundsCheck`] trait and its implementations for [`Index`]
//! and [`Length`]. It provides a way to check if an index is within the bounds of a
//! length or another index.
//!
//! Here's an example of how to use the [`BoundsCheck`] trait to check if an index is
//! within the bounds of a length:
//!
//! ```
//! use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, Index, Length, idx, len};
//! let index = idx(10);
//! let length = len(10);
//! let bounds_status = index.check_overflows(length);
//! assert_eq!(bounds_status, BoundsOverflowStatus::Overflowed);
//! ```
//!
//! Here's an example of how to use the [`BoundsCheck`] trait to check if an index is
//! within the bounds of another index:
//!
//! ```
//! use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, Index, idx};
//! let index1 = idx(10);
//! let index2 = idx(10);
//! let bounds_status = index1.check_overflows(index2);
//! assert_eq!(bounds_status, BoundsOverflowStatus::Within);
//! ```


use super::{BoundsCheck, BoundsOverflowStatus, Index, Length, ContentPositionStatus};

impl BoundsCheck<Length> for Index {
    fn check_overflows(&self, length: Length) -> BoundsOverflowStatus {
        let this = *self;
        let other = length.convert_to_index() /*-1*/;
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(&self, content_length: Length) -> ContentPositionStatus {
        let this_value = self.as_usize();
        let length = content_length.as_usize();

        if this_value > length {
            ContentPositionStatus::Beyond
        } else if this_value == 0 {
            ContentPositionStatus::AtStart
        } else if this_value == length {
            ContentPositionStatus::AtEnd
        } else {
            ContentPositionStatus::Within
        }
    }
}

impl BoundsCheck<Index> for Index {
    fn check_overflows(&self, other: Index) -> BoundsOverflowStatus {
        let this = *self;
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(&self, content_length: Index) -> ContentPositionStatus {
        let this_value = self.as_usize();
        let length = content_length.as_usize();

        if this_value > length {
            ContentPositionStatus::Beyond
        } else if this_value == 0 {
            ContentPositionStatus::AtStart
        } else if this_value == length {
            ContentPositionStatus::AtEnd
        } else {
            ContentPositionStatus::Within
        }
    }
}

#[cfg(test)]
mod tests_check_overflows {
    use super::*;
    use crate::{idx, len};

    #[test]
    fn test_index_bounds_check_length() {
        let length = len(10);

        let index = idx(0);
        assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Within);

        let index = idx(9);
        assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Within);

        let index = idx(10);
        assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Overflowed);

        let index = idx(11);
        assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Overflowed);
    }

    #[test]
    fn test_index_bounds_check_index() {
        let index2 = idx(10);

        let index1 = idx(0);
        assert_eq!(index1.check_overflows(index2), BoundsOverflowStatus::Within);

        let index1 = idx(9);
        assert_eq!(index1.check_overflows(index2), BoundsOverflowStatus::Within);

        let index1 = idx(10);
        assert_eq!(index1.check_overflows(index2), BoundsOverflowStatus::Within);

        let index1 = idx(11);
        assert_eq!(index1.check_overflows(index2), BoundsOverflowStatus::Overflowed);
    }

    #[test]
    fn test_index_bounds_check_zero_length() {
        let length = len(0);
        let index = idx(0);
        assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Within);

        let index = idx(1);
        assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Overflowed);
    }

    #[test]
    fn test_index_bounds_check_zero_index() {
        let index2 = idx(0);

        let index1 = idx(0);
        assert_eq!(index1.check_overflows(index2), BoundsOverflowStatus::Within);

        let index1 = idx(1);
        assert_eq!(index1.check_overflows(index2), BoundsOverflowStatus::Overflowed);
    }
}

#[cfg(test)]
mod tests_check_content_position {
    use super::*;
    use crate::{idx, len};

    #[test]
    fn test_index_content_position_check_length() {
        let length = len(5);

        // At start
        let index = idx(0);
        assert_eq!(index.check_content_position(length), ContentPositionStatus::AtStart);

        // Within content (indices 1-4 are valid content positions)
        let index = idx(2);
        assert_eq!(index.check_content_position(length), ContentPositionStatus::Within);

        let index = idx(4);
        assert_eq!(index.check_content_position(length), ContentPositionStatus::Within);

        // At end (index 5 is end of content for length 5)
        let index = idx(5);
        assert_eq!(
            index.check_content_position(length),
            ContentPositionStatus::AtEnd
        );

        // Beyond content
        let index = idx(6);
        assert_eq!(index.check_content_position(length), ContentPositionStatus::Beyond);

        let index = idx(10);
        assert_eq!(index.check_content_position(length), ContentPositionStatus::Beyond);
    }

    #[test]
    fn test_index_content_position_vs_array_position() {
        let length = len(3);
        let index = idx(2);

        // For content checking: index 2 is within content for length 3
        assert_eq!(index.check_content_position(length), ContentPositionStatus::Within);

        // Content end boundary is at index 3
        let boundary_index = idx(3);
        assert_eq!(
            boundary_index.check_content_position(length),
            ContentPositionStatus::AtEnd
        );
    }

    #[test]
    fn test_index_content_position_check_zero_length() {
        let length = len(0);

        // For zero length content, index 0 should be AtStart (precedence over AtEnd)
        let index = idx(0);
        assert_eq!(
            index.check_content_position(length),
            ContentPositionStatus::AtStart
        );

        let index = idx(1);
        assert_eq!(index.check_content_position(length), ContentPositionStatus::Beyond);
    }
}

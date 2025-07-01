/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! This module defines the [`BoundsCheck`] trait and its implementations for [`Index`]
//! and [`Length`]. It provides a way to check if an index is within the bounds of a
//! length or another index.
//!
//! Here's an example of how to use the [`BoundsCheck`] trait to check if an index is
//! within the bounds of a length:
//!
//! ```
//! use r3bl_tui::{BoundsCheck, BoundsStatus, Index, Length, idx, len};
//! let index = idx(10);
//! let length = len(10);
//! let bounds_status = index.check_overflows(length);
//! assert_eq!(bounds_status, BoundsStatus::Overflowed);
//! ```
//!
//! Here's an example of how to use the [`BoundsCheck`] trait to check if an index is
//! within the bounds of another index:
//!
//! ```
//! use r3bl_tui::{BoundsCheck, BoundsStatus, Index, idx};
//! let index1 = idx(10);
//! let index2 = idx(10);
//! let bounds_status = index1.check_overflows(index2);
//! assert_eq!(bounds_status, BoundsStatus::Within);
//! ```

use std::cmp::Ordering;

use super::{BoundsCheck, BoundsStatus, Index, Length, PositionStatus};

impl BoundsCheck<Length> for Index {
    fn check_overflows(&self, length: Length) -> BoundsStatus {
        let this = *self;
        let other = length.convert_to_index() /*-1*/;
        if this > other {
            BoundsStatus::Overflowed
        } else {
            BoundsStatus::Within
        }
    }

    fn check_content_position(&self, content_length: Length) -> PositionStatus {
        let this = *self;
        let length = content_length.as_usize();

        match this.as_usize().cmp(&length) {
            Ordering::Less => PositionStatus::Within,
            Ordering::Equal => PositionStatus::Boundary,
            Ordering::Greater => PositionStatus::Beyond,
        }
    }
}

impl BoundsCheck<Index> for Index {
    fn check_overflows(&self, other: Index) -> BoundsStatus {
        let this = *self;
        if this > other {
            BoundsStatus::Overflowed
        } else {
            BoundsStatus::Within
        }
    }

    fn check_content_position(&self, content_length: Index) -> PositionStatus {
        let this = *self;
        let length = content_length.as_usize();

        match this.as_usize().cmp(&length) {
            Ordering::Less => PositionStatus::Within,
            Ordering::Equal => PositionStatus::Boundary,
            Ordering::Greater => PositionStatus::Beyond,
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
        assert_eq!(index.check_overflows(length), BoundsStatus::Within);

        let index = idx(9);
        assert_eq!(index.check_overflows(length), BoundsStatus::Within);

        let index = idx(10);
        assert_eq!(index.check_overflows(length), BoundsStatus::Overflowed);

        let index = idx(11);
        assert_eq!(index.check_overflows(length), BoundsStatus::Overflowed);
    }

    #[test]
    fn test_index_bounds_check_index() {
        let index2 = idx(10);

        let index1 = idx(0);
        assert_eq!(index1.check_overflows(index2), BoundsStatus::Within);

        let index1 = idx(9);
        assert_eq!(index1.check_overflows(index2), BoundsStatus::Within);

        let index1 = idx(10);
        assert_eq!(index1.check_overflows(index2), BoundsStatus::Within);

        let index1 = idx(11);
        assert_eq!(index1.check_overflows(index2), BoundsStatus::Overflowed);
    }

    #[test]
    fn test_index_bounds_check_zero_length() {
        let length = len(0);
        let index = idx(0);
        assert_eq!(index.check_overflows(length), BoundsStatus::Within);

        let index = idx(1);
        assert_eq!(index.check_overflows(length), BoundsStatus::Overflowed);
    }

    #[test]
    fn test_index_bounds_check_zero_index() {
        let index2 = idx(0);

        let index1 = idx(0);
        assert_eq!(index1.check_overflows(index2), BoundsStatus::Within);

        let index1 = idx(1);
        assert_eq!(index1.check_overflows(index2), BoundsStatus::Overflowed);
    }
}

#[cfg(test)]
mod tests_check_content_position {
    use super::*;
    use crate::{idx, len};

    #[test]
    fn test_index_content_position_check_length() {
        let length = len(5);

        // Within content (indices 0-4 are valid content positions)
        let index = idx(0);
        assert_eq!(index.check_content_position(length), PositionStatus::Within);

        let index = idx(2);
        assert_eq!(index.check_content_position(length), PositionStatus::Within);

        let index = idx(4);
        assert_eq!(index.check_content_position(length), PositionStatus::Within);

        // At boundary (index 5 is end of content for length 5)
        let index = idx(5);
        assert_eq!(
            index.check_content_position(length),
            PositionStatus::Boundary
        );

        // Beyond content
        let index = idx(6);
        assert_eq!(index.check_content_position(length), PositionStatus::Beyond);

        let index = idx(10);
        assert_eq!(index.check_content_position(length), PositionStatus::Beyond);
    }

    #[test]
    fn test_index_content_position_vs_array_position() {
        let length = len(3);
        let index = idx(2);

        // For content checking: index 2 is within content for length 3
        assert_eq!(index.check_content_position(length), PositionStatus::Within);

        // Content boundary is at index 3
        let boundary_index = idx(3);
        assert_eq!(
            boundary_index.check_content_position(length),
            PositionStatus::Boundary
        );
    }

    #[test]
    fn test_index_content_position_check_zero_length() {
        let length = len(0);

        // For zero length content, index 0 should be boundary (end of empty content)
        let index = idx(0);
        assert_eq!(
            index.check_content_position(length),
            PositionStatus::Boundary
        );

        let index = idx(1);
        assert_eq!(index.check_content_position(length), PositionStatus::Beyond);
    }
}

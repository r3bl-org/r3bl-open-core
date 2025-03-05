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
//! ```rust
//! use r3bl_core::{BoundsCheck, BoundsStatus, Index, Length, idx, len};
//! let index = idx(10);
//! let length = len(10);
//! let bounds_status = index.check_overflows(length);
//! assert_eq!(bounds_status, BoundsStatus::Overflowed);
//! ```
//!
//! Here's an example of how to use the [`BoundsCheck`] trait to check if an index is
//! within the bounds of another index:
//!
//! ```rust
//! use r3bl_core::{BoundsCheck, BoundsStatus, Index, idx};
//! let index1 = idx(10);
//! let index2 = idx(10);
//! let bounds_status = index1.check_overflows(index2);
//! assert_eq!(bounds_status, BoundsStatus::Within);
//! ```

use super::{BoundsCheck, BoundsStatus, Index, Length};

impl BoundsCheck<Length> for Index {
    /// Used to be `col_index >= width`.
    /// And: `a >= b` === `a > b-1`.
    /// So: `col_index > width - 1`.
    fn check_overflows(&self, length: Length) -> BoundsStatus {
        let this = *self;
        let other = length.convert_to_index() /*-1*/;
        if this > other {
            BoundsStatus::Overflowed
        } else {
            BoundsStatus::Within
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
}
#[cfg(test)]
mod tests {
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

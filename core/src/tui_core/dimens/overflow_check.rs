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

use super::{ColIndex, ColWidth, RowHeight, RowIndex};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BoundsContainmentResult {
    Within,
    Overflowed,
}

// REVIEW: [ ] use this in place of (col/row) index and width/height overflow checks. See inc_caret_row() as example

/// This trait "formalizes" the concept of checking for overflow. More specifically when
/// an index (row or col index) overflows a length (width or height).
///
/// When `a` and `b` are both unsigned integers, the following are equivalent:
/// - `a >= b`
/// - `a >  b-1`
///
/// So, the following are equivalent:
/// - `row_index >= height`
/// - `row_index > height - 1`
///
/// # Examples
///
/// ```rust
/// use r3bl_core::{
///     BoundsContainmentCheck, BoundsContainmentResult,
///     RowHeight, RowIndex, ColIndex, ColWidth
/// };
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(
///     row_index.check_overflows(height),
///     BoundsContainmentResult::Overflowed);
///
/// let col_index = ColIndex::new(3);
/// let width = ColWidth::new(5);
/// assert_eq!(
///     col_index.check_overflows(width),
///     BoundsContainmentResult::Within);
/// ```
pub trait BoundsContainmentCheck<OtherType> {
    fn check_overflows(&self, max: OtherType) -> BoundsContainmentResult;
}

impl BoundsContainmentCheck<RowHeight> for RowIndex {
    /// Used to be `row_index >= height`.
    /// And: `a >= b` === `a > b-1`.
    /// So: `row_index > height - 1`.
    fn check_overflows(&self, height: RowHeight) -> BoundsContainmentResult {
        let this = *self;
        let other = height.convert_to_row_index() /*-1*/;
        if this > other {
            BoundsContainmentResult::Overflowed
        } else {
            BoundsContainmentResult::Within
        }
    }
}

impl BoundsContainmentCheck<ColWidth> for ColIndex {
    /// Used to be `col_index >= width`.
    /// And: `a >= b` === `a > b-1`.
    /// So: `col_index > width - 1`.
    fn check_overflows(&self, width: ColWidth) -> BoundsContainmentResult {
        let this = *self;
        let other = width.convert_to_col_index() /*-1*/;
        if this > other {
            BoundsContainmentResult::Overflowed
        } else {
            BoundsContainmentResult::Within
        }
    }
}

#[cfg(test)]
mod tests_overflow_check {
    use super::*;
    use crate::{col, height, row, width};

    #[test]
    fn test_col_width_for_col_index() {
        let within = [col(0), col(1), col(2), col(3), col(4)];
        let overflowed = [col(5), col(6), col(7)];
        let width = width(5);

        for col_index in within.iter() {
            assert_eq!(
                col_index.check_overflows(width),
                BoundsContainmentResult::Within
            );
        }

        for col_index in overflowed.iter() {
            assert_eq!(
                col_index.check_overflows(width),
                BoundsContainmentResult::Overflowed
            );
        }
    }

    #[test]
    fn test_row_height_for_row_index() {
        let within = [row(0), row(1), row(2), row(3), row(4)];
        let overflowed = [row(5), row(6), row(7)];
        let height = height(5);

        for row_index in within.iter() {
            assert_eq!(
                row_index.check_overflows(height),
                BoundsContainmentResult::Within
            );
        }

        for row_index in overflowed.iter() {
            assert_eq!(
                row_index.check_overflows(height),
                BoundsContainmentResult::Overflowed
            );
        }
    }
}

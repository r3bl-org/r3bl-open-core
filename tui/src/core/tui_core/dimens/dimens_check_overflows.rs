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
use crate::{BoundsCheck, BoundsStatus, PositionStatus};

impl BoundsCheck<RowHeight> for RowIndex {
    fn check_overflows(&self, height: RowHeight) -> BoundsStatus {
        let this = *self;
        let other = height.convert_to_row_index() /*-1*/;
        if this > other {
            BoundsStatus::Overflowed
        } else {
            BoundsStatus::Within
        }
    }

    fn check_content_position(&self, content_length: RowHeight) -> PositionStatus {
        let this = *self;
        let length = content_length.as_usize();

        if this.as_usize() < length {
            PositionStatus::Within
        } else if this.as_usize() == length {
            PositionStatus::Boundary
        } else {
            PositionStatus::Beyond
        }
    }
}

impl BoundsCheck<ColWidth> for ColIndex {
    fn check_overflows(&self, width: ColWidth) -> BoundsStatus {
        let this = *self;
        let other = width.convert_to_col_index() /*-1*/;
        if this > other {
            BoundsStatus::Overflowed
        } else {
            BoundsStatus::Within
        }
    }

    fn check_content_position(&self, content_length: ColWidth) -> PositionStatus {
        let this = *self;
        let length = content_length.as_usize();

        if this.as_usize() < length {
            PositionStatus::Within
        } else if this.as_usize() == length {
            PositionStatus::Boundary
        } else {
            PositionStatus::Beyond
        }
    }
}

impl BoundsCheck<RowIndex> for RowIndex {
    fn check_overflows(&self, other: RowIndex) -> BoundsStatus {
        let this = *self;
        if this > other {
            BoundsStatus::Overflowed
        } else {
            BoundsStatus::Within
        }
    }

    fn check_content_position(&self, content_length: RowIndex) -> PositionStatus {
        let this = *self;
        let length = content_length.as_usize();

        if this.as_usize() < length {
            PositionStatus::Within
        } else if this.as_usize() == length {
            PositionStatus::Boundary
        } else {
            PositionStatus::Beyond
        }
    }
}

#[cfg(test)]
mod tests_bounds_check_overflows {
    use super::*;
    use crate::{col, height, row, width};

    #[test]
    fn test_col_width_for_col_index() {
        let within = [col(0), col(1), col(2), col(3), col(4)];
        let overflowed = [col(5), col(6), col(7)];
        let width = width(5);

        for col_index in &within {
            assert_eq!(col_index.check_overflows(width), BoundsStatus::Within);
        }

        for col_index in &overflowed {
            assert_eq!(col_index.check_overflows(width), BoundsStatus::Overflowed);
        }
    }

    #[test]
    fn test_row_height_for_row_index() {
        let within = [row(0), row(1), row(2), row(3), row(4)];
        let overflowed = [row(5), row(6), row(7)];
        let height = height(5);

        for row_index in &within {
            assert_eq!(row_index.check_overflows(height), BoundsStatus::Within);
        }

        for row_index in &overflowed {
            assert_eq!(row_index.check_overflows(height), BoundsStatus::Overflowed);
        }
    }

    #[test]
    fn test_row_index_for_row_index() {
        let within = [row(0), row(1), row(2), row(3), row(4), row(5)];
        let overflowed = [row(6), row(7)];
        let max = row(5);

        for row_index in &within {
            assert_eq!(row_index.check_overflows(max), BoundsStatus::Within);
        }

        for row_index in &overflowed {
            assert_eq!(row_index.check_overflows(max), BoundsStatus::Overflowed);
        }
    }
}

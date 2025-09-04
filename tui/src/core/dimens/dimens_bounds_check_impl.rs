// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{ColIndex, ColWidth, RowHeight, RowIndex};
use crate::{BoundsCheck, BoundsOverflowStatus, ContentPositionStatus};

impl From<(/* position */ u16, /* length */ u16)> for ContentPositionStatus {
    fn from((position, length): (u16, u16)) -> Self {
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

impl BoundsCheck<RowHeight> for RowIndex {
    fn check_overflows(&self, height: RowHeight) -> BoundsOverflowStatus {
        let this = *self;
        let other = height.convert_to_row_index() /*-1*/;
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(&self, content_length: RowHeight) -> ContentPositionStatus {
        (self.as_u16(), content_length.as_u16()).into()
    }
}

impl BoundsCheck<ColWidth> for ColIndex {
    fn check_overflows(&self, width: ColWidth) -> BoundsOverflowStatus {
        let this = *self;
        let other = width.convert_to_col_index() /*-1*/;
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(&self, content_length: ColWidth) -> ContentPositionStatus {
        (self.as_u16(), content_length.as_u16()).into()
    }
}

impl BoundsCheck<RowIndex> for RowIndex {
    fn check_overflows(&self, other: RowIndex) -> BoundsOverflowStatus {
        let this = *self;
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(&self, content_length: RowIndex) -> ContentPositionStatus {
        (self.as_u16(), content_length.as_u16()).into()
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
            assert_eq!(col_index.check_overflows(width), BoundsOverflowStatus::Within);
        }

        for col_index in &overflowed {
            assert_eq!(col_index.check_overflows(width), BoundsOverflowStatus::Overflowed);
        }
    }

    #[test]
    fn test_row_height_for_row_index() {
        let within = [row(0), row(1), row(2), row(3), row(4)];
        let overflowed = [row(5), row(6), row(7)];
        let height = height(5);

        for row_index in &within {
            assert_eq!(row_index.check_overflows(height), BoundsOverflowStatus::Within);
        }

        for row_index in &overflowed {
            assert_eq!(row_index.check_overflows(height), BoundsOverflowStatus::Overflowed);
        }
    }

    #[test]
    fn test_row_index_for_row_index() {
        let within = [row(0), row(1), row(2), row(3), row(4), row(5)];
        let overflowed = [row(6), row(7)];
        let max = row(5);

        for row_index in &within {
            assert_eq!(row_index.check_overflows(max), BoundsOverflowStatus::Within);
        }

        for row_index in &overflowed {
            assert_eq!(row_index.check_overflows(max), BoundsOverflowStatus::Overflowed);
        }
    }
}

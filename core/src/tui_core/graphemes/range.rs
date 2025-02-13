/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::{cmp::{self},
          fmt::Debug};

use crate::{CaretScrAdj, ColIndex, ColWidth, RowIndex, ScrOfs, width};

/// Represents a range of characters in a line.
///
/// The range is not inclusive of the item at the end index, which means that when you
/// call [clip_to_range](crate::UnicodeString::clip_to_range) the item at the end index
/// will not be part of the result (this is shown in the example below). The indices are
/// all display column indices, not logical ones.
///
/// ```text
/// 0 1 2 3 4 5 6 7 8 9
/// h e ▓ ▓ o   w o r l
///   ┬     ┬
///   │     │
///   │     ⎩end_display_col_index
///   ⎩start_display_col_index
/// ```
/// - `▓▓` = `😃`
/// - [clip_to_range](crate::UnicodeString::clip_to_range): "e😃"
#[derive(Default, Clone, PartialEq, Copy, size_of::SizeOf)]
// BUG: [ ] introduce scroll adjusted type
pub struct SelectionRange {
    /// This is not "raw", this is "scroll adjusted". This represents the display width at
    /// which the selection starts. The display width is used, in order to support
    /// variable width characters.
    pub start_disp_col_idx_scr_adj: ColIndex,
    /// This is not "raw", this is "scroll adjusted". This represents the display width at
    /// which the selection ends. The display width is used, in order to support variable
    /// width characters. The end index is not inclusive when the selection range is
    /// resolved into a result (string).
    pub end_disp_col_idx_scr_adj: ColIndex,
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum ScrollOffsetColLocationInRange {
    Overflow,
    Underflow,
}

impl SelectionRange {
    /// Due to the nature of selection ranges, the index values are actually display
    /// widths. And sometimes it is useful to type cast them as a width, eg: when using
    /// with [crate::UnicodeString::clip_to_range].
    pub fn get_start_display_col_index_as_width(&self) -> ColWidth {
        width(*self.start_disp_col_idx_scr_adj)
    }
}

impl SelectionRange {
    pub fn locate_scroll_offset_col(
        &self,
        scroll_offset: ScrOfs,
    ) -> ScrollOffsetColLocationInRange {
        if self.start_disp_col_idx_scr_adj >= scroll_offset.col_index {
            ScrollOffsetColLocationInRange::Underflow
        } else {
            ScrollOffsetColLocationInRange::Overflow
        }
    }
}

mod convert {
    use super::*;

    impl From<(ColIndex, ColIndex)> for SelectionRange {
        fn from((start, end): (ColIndex, ColIndex)) -> Self {
            Self {
                start_disp_col_idx_scr_adj: start,
                end_disp_col_idx_scr_adj: end,
            }
        }
    }
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum CaretLocationInRange {
    Overflow,
    Underflow,
    Contained,
}

/// Note this must derive [Eq]. More info [here](https://stackoverflow.com/a/68900245/2085356).
#[derive(Clone, PartialEq, Eq, Copy, Debug, size_of::SizeOf)]
pub enum CaretMovementDirection {
    Up,
    Down,
    Left,
    Right,
    Overlap,
}

#[cfg(test)]
mod tests_range {
    use super::*;
    use crate::{assert_eq2, col};

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ┬     ┬
    ///   │     │
    ///   │     ⎩4 = end_display_col_index
    ///   ⎩1 = start_display_col_index
    /// ```
    /// - [UnicodeString::clip_to_range](UnicodeString::clip_to_range): "ell"
    #[test]
    fn test_locate() {
        let range = {
            let start = col(1);
            let end = col(4);
            SelectionRange::new(start, end)
        };
        assert_eq2!(range.locate_column(col(0)), CaretLocationInRange::Underflow);
        assert_eq2!(range.locate_column(col(1)), CaretLocationInRange::Contained);
        assert_eq2!(range.locate_column(col(2)), CaretLocationInRange::Contained);
        assert_eq2!(range.locate_column(col(3)), CaretLocationInRange::Contained);
        assert_eq2!(range.locate_column(col(4)), CaretLocationInRange::Overflow);
        assert_eq2!(range.locate_column(col(5)), CaretLocationInRange::Overflow);
    }
}

impl SelectionRange {
    pub fn caret_movement_direction(
        previous_caret_display_position: CaretScrAdj,
        current_caret_display_position: CaretScrAdj,
    ) -> CaretMovementDirection {
        let previous_caret_display_row_index = previous_caret_display_position.row_index;
        let current_caret_display_row_index = current_caret_display_position.row_index;
        let previous_caret_display_col_index = previous_caret_display_position.col_index;
        let current_caret_display_col_index = current_caret_display_position.col_index;
        if previous_caret_display_row_index == current_caret_display_row_index {
            Self::caret_movement_direction_left_right(
                previous_caret_display_col_index,
                current_caret_display_col_index,
            )
        } else {
            Self::caret_movement_direction_up_down(
                previous_caret_display_row_index,
                current_caret_display_row_index,
            )
        }
    }

    pub fn caret_movement_direction_up_down(
        previous_caret_display_row_index: RowIndex,
        current_caret_display_row_index: RowIndex,
    ) -> CaretMovementDirection {
        match current_caret_display_row_index.cmp(&previous_caret_display_row_index) {
            cmp::Ordering::Greater => CaretMovementDirection::Down,
            cmp::Ordering::Less => CaretMovementDirection::Up,
            cmp::Ordering::Equal => CaretMovementDirection::Overlap,
        }
    }

    pub fn caret_movement_direction_left_right(
        previous_caret_display_col_index: ColIndex,
        current_caret_display_col_index: ColIndex,
    ) -> CaretMovementDirection {
        match current_caret_display_col_index.cmp(&previous_caret_display_col_index) {
            cmp::Ordering::Greater => CaretMovementDirection::Right,
            cmp::Ordering::Less => CaretMovementDirection::Left,
            cmp::Ordering::Equal => CaretMovementDirection::Overlap,
        }
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ┬     ┬
    ///   │     │
    ///   │     ⎩4 = end_display_col_index
    ///   ⎩1 = start_display_col_index
    /// ```
    /// - [UnicodeString::clip_to_range](crate::UnicodeString::clip_to_range): "ell"
    pub fn locate_column(
        &self,
        caret_display_col_index: ColIndex,
    ) -> CaretLocationInRange {
        if caret_display_col_index < self.start_disp_col_idx_scr_adj {
            CaretLocationInRange::Underflow
        } else if caret_display_col_index >= self.end_disp_col_idx_scr_adj {
            CaretLocationInRange::Overflow
        } else {
            CaretLocationInRange::Contained
        }
    }

    /// Alternatively you can also just use a tuple of [ColIndex] to represent the range.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use r3bl_core::graphemes::range::SelectionRange;
    /// use r3bl_core::col;
    /// let range_1: SelectionRange = ( col(1), col(4) ).into();
    /// let range_2 = SelectionRange::new(col(1), col(4));
    /// ```
    pub fn new(
        start_display_col_index: ColIndex,
        end_display_col_index: ColIndex,
    ) -> Self {
        Self {
            start_disp_col_idx_scr_adj: start_display_col_index,
            end_disp_col_idx_scr_adj: end_display_col_index,
        }
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ┬     ┬
    ///   │     │
    ///   │     ⎩end_display_col_index + col_amt
    ///   ⎩start_display_col_index
    /// ```
    pub fn grow_end_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.end_disp_col_idx_scr_adj += col_amt;
        copy
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ┬     ┬
    ///   │     │
    ///   │     ⎩ end_display_col_index - col_amt
    ///   ⎩start_display_col_index
    /// ```
    pub fn shrink_end_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.end_disp_col_idx_scr_adj -= col_amt;
        copy
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ┬     ┬
    ///   │     │
    ///   │     ⎩end_display_col_index
    ///   ⎩start_display_col_index - col_amt
    /// ```
    pub fn grow_start_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.start_disp_col_idx_scr_adj -= col_amt;
        copy
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ┬     ┬
    ///   │     │
    ///   │     ⎩end_display_col_index
    ///   ⎩start_display_col_index - col_amt
    /// ```
    pub fn shrink_start_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.start_disp_col_idx_scr_adj += col_amt;
        copy
    }
}

mod range_impl_debug_format {
    use super::*;

    impl Debug for SelectionRange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "[start_disp_col_idx_scr_adj: {start:?}, end_disp_col_idx_scr_adj: {end:?}]",
                start = self.start_disp_col_idx_scr_adj,
                end = self.end_disp_col_idx_scr_adj
            )
        }
    }
}

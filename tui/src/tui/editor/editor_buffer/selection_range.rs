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

use crate::{caret_scr_adj,
            row,
            width,
            CaretScrAdj,
            ChUnitPrimitiveType,
            ColIndex,
            ColWidth,
            GCString,
            ScrOfs};

/// Represents a range of characters in a line. The col indices are scroll adjusted (and
/// not raw). The row indices are not used, and clobbered with [ChUnitPrimitiveType::MAX].
///
/// The range is not inclusive of the item at the end index, which means that when you
/// call [crate::SelectionRange::clip_to_range()] the item at the end index will not be
/// part of the result (this is shown in the example below). The indices are all display
/// column indices, not logical ones.
///
/// ```text
/// ╭0123456789╮
/// 0he▓▓o worl│
/// ╰─┬──┬─────╯
///   │  │
///   │  ⎩end_display_col_index
///   ⎩start_display_col_index
/// ```
///
/// - `"▓▓"` = `"😃"`
/// - [crate::SelectionRange::clip_to_range()] : "e😃"
///
/// This range can't be instantiated directly via the struct, you have to use the tuple
/// conversion. Even though the struct holds two [CaretScrAdj] values, it does not use the
/// [crate::RowIndex] fields.
#[derive(Default, Clone, PartialEq, Copy)]
pub struct SelectionRange {
    /// This is not "raw", this is "scroll adjusted".
    /// - It represents the display width at which the selection starts.
    /// - The [crate::RowIndex] field is not used and is clobbered with
    ///   [ChUnitPrimitiveType::MAX] after initialization.
    /// - The display width is used, to support variable width characters. `UTF-8`
    ///   encoding uses between 1 and 4 bytes to encode a character, e.g.: `"H"` is 1
    ///   byte, and `"😄"` is 4 bytes. And visually they can occupy 1 or more spaces,
    ///   e.g.: `"H"` is 1 space wide, and `"😄"` is two spaces wide
    ///   [crate::GCString::width()] and [crate::GCString::width_char()].
    start: CaretScrAdj,
    /// This is not "raw", this is "scroll adjusted".
    /// - It represents the display width at which the selection ends. The display width
    ///   is used, to support variable width characters.
    /// - The end index is not inclusive when the selection range is resolved into a
    ///   result (string).
    /// - The display width is used, to support variable width characters. `UTF-8`
    ///   encoding uses between 1 and 4 bytes to encode a character, e.g.: `"H"` is 1
    ///   byte, and `"😄"` is 4 bytes. And visually they can occupy 1 or more spaces,
    ///   e.g. `"H"` is 1 space wide, and `"😄"` is two spaces wide
    ///   [crate::GCString::width()] and [crate::GCString::width_char()].
    end: CaretScrAdj,
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum ScrollOffsetColLocationInRange {
    Overflow,
    Underflow,
}

/// The only way to construct a [SelectionRange] is by converting a tuple of [CaretScrAdj]
/// values into a [SelectionRange].
mod convert {
    use super::*;

    impl From<(CaretScrAdj, CaretScrAdj)> for SelectionRange {
        /// The [crate::RowIndex] fields of each tuple value are not used. They are just
        /// set to the maximum value of [ChUnitPrimitiveType].
        fn from((start, end): (CaretScrAdj, CaretScrAdj)) -> Self {
            let start = caret_scr_adj(start.col_index + row(ChUnitPrimitiveType::MAX));
            let end = caret_scr_adj(end.col_index + row(ChUnitPrimitiveType::MAX));
            Self { start, end }
        }
    }
}

impl SelectionRange {
    pub fn start(&self) -> ColIndex { self.start.col_index }

    pub fn end(&self) -> ColIndex { self.end.col_index }

    /// Due to the nature of selection ranges, the index values are actually display
    /// widths. And sometimes it is useful to type cast them as a width, e.g.: when using
    /// with [crate::SelectionRange::clip_to_range()].
    pub fn get_start_display_col_index_as_width(&self) -> ColWidth {
        width(*self.start.col_index)
    }

    /// Returns a tuple of the start and end display column indices. This is just a
    /// convenience function that prevents the need to access the fields directly to get
    /// the two [ColIndex] values.
    pub fn as_tuple(&self) -> (ColIndex, ColIndex) {
        (self.start.col_index, self.end.col_index)
    }

    /// Uses `SelectionRange` to calculate width and simply calls
    /// [crate::GCString::clip()].
    pub fn clip_to_range<'a>(&self, us: &'a GCString) -> &'a str {
        let (start_display_col_index, end_display_col_index) = self.as_tuple();
        let max_display_width_col_count =
            width(*(end_display_col_index - start_display_col_index));
        us.clip(start_display_col_index, max_display_width_col_count)
    }
}

impl SelectionRange {
    pub fn locate_scroll_offset_col(
        &self,
        scroll_offset: ScrOfs,
    ) -> ScrollOffsetColLocationInRange {
        if self.start.col_index >= scroll_offset.col_index {
            ScrollOffsetColLocationInRange::Underflow
        } else {
            ScrollOffsetColLocationInRange::Overflow
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
#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub enum CaretMovementDirection {
    Up,
    Down,
    Left,
    Right,
    Overlap,
}

impl SelectionRange {
    pub fn caret_movement_direction(
        prev: CaretScrAdj,
        curr: CaretScrAdj,
    ) -> CaretMovementDirection {
        if prev.row_index == curr.row_index {
            Self::caret_movement_direction_left_right(prev, curr)
        } else {
            Self::caret_movement_direction_up_down(prev, curr)
        }
    }

    pub fn caret_movement_direction_up_down(
        prev: CaretScrAdj,
        curr: CaretScrAdj,
    ) -> CaretMovementDirection {
        match curr.row_index.cmp(&prev.row_index) {
            cmp::Ordering::Greater => CaretMovementDirection::Down,
            cmp::Ordering::Less => CaretMovementDirection::Up,
            cmp::Ordering::Equal => CaretMovementDirection::Overlap,
        }
    }

    pub fn caret_movement_direction_left_right(
        prev: CaretScrAdj,
        curr: CaretScrAdj,
    ) -> CaretMovementDirection {
        match curr.col_index.cmp(&prev.col_index) {
            cmp::Ordering::Greater => CaretMovementDirection::Right,
            cmp::Ordering::Less => CaretMovementDirection::Left,
            cmp::Ordering::Equal => CaretMovementDirection::Overlap,
        }
    }

    /// ```text
    /// ╭0123456789╮
    /// │hello worl│
    /// ╰─┬──┬─────╯
    ///   │  │
    ///   │  ⎩4 = end_display_col_index
    ///   ⎩1 = start_display_col_index
    /// ```
    /// - [crate::SelectionRange::clip_to_range()] : "ell"
    pub fn locate_column(&self, caret: CaretScrAdj) -> CaretLocationInRange {
        if caret.col_index < self.start.col_index {
            CaretLocationInRange::Underflow
        } else if caret.col_index >= self.end.col_index {
            CaretLocationInRange::Overflow
        } else {
            CaretLocationInRange::Contained
        }
    }

    /// Alternatively you can also just use a tuple of [ColIndex] to represent the range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use r3bl_tui::SelectionRange;
    /// # use r3bl_tui::{col, row, caret_scr_adj};
    /// let range_1: SelectionRange = (
    ///     caret_scr_adj(row(0) + col(1)),
    ///     caret_scr_adj(row(0) + col(4))
    /// ).into();
    /// let range_2 = SelectionRange::new(
    ///     caret_scr_adj(row(0) + col(1)),
    ///     caret_scr_adj(row(0) + col(4))
    /// );
    /// ```
    pub fn new(start: CaretScrAdj, end: CaretScrAdj) -> Self { Self { start, end } }

    /// ```text
    /// ╭0123456789╮
    /// │hello worl│
    /// ╰─┬──┬─────╯
    ///   │  │
    ///   │  ⎩end_display_col_index + col_amt
    ///   ⎩start_display_col_index
    /// ```
    pub fn grow_end_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.end.col_index += col_amt;
        copy
    }

    /// ```text
    /// ╭0123456789╮
    /// │hello worl│
    /// ╰─┬──┬─────╯
    ///   │  │
    ///   │  ⎩end_display_col_index - col_amt
    ///   ⎩start_display_col_index
    /// ```
    pub fn shrink_end_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.end.col_index -= col_amt;
        copy
    }

    /// ```text
    /// ╭0123456789╮
    /// │hello worl│
    /// ╰─┬──┬─────╯
    ///   │  │
    ///   │  ⎩end_display_col_index
    ///   ⎩start_display_col_index - col_amt
    /// ```
    pub fn grow_start_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.start.col_index -= col_amt;
        copy
    }

    /// ```text
    /// ╭0123456789╮
    /// │hello worl│
    /// ╰─┬──┬─────╯
    ///   │  │
    ///   │  ⎩end_display_col_index
    ///   ⎩start_display_col_index - col_amt
    /// ```
    pub fn shrink_start_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.start.col_index += col_amt;
        copy
    }
}

mod range_impl_debug_format {
    use super::*;

    impl Debug for SelectionRange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "[start.col_index: {start:?}, end.col_index: {end:?}]",
                start = self.start.col_index,
                end = self.end.col_index
            )
        }
    }
}

#[cfg(test)]
mod tests_range {
    use super::*;
    use crate::{assert_eq2, caret_scr_adj, col, row};

    /// ```text
    /// ╭0123456789╮
    /// │hello worl│
    /// ╰─┬──┬─────╯
    ///   │  │
    ///   │  ⎩4 = end_display_col_index
    ///   ⎩1 = start_display_col_index
    /// ```
    /// - [GCString::clip_to_range](GCString::clip_to_range): "ell"
    #[test]
    fn test_locate() {
        let range = {
            let start = caret_scr_adj(col(1) + row(0));
            let end = caret_scr_adj(col(4) + row(0));
            SelectionRange::new(start, end)
        };
        assert_eq2!(
            range.locate_column(caret_scr_adj(col(0) + row(0))),
            CaretLocationInRange::Underflow
        );
        assert_eq2!(
            range.locate_column(caret_scr_adj(col(1) + row(0))),
            CaretLocationInRange::Contained
        );
        assert_eq2!(
            range.locate_column(caret_scr_adj(col(2) + row(0))),
            CaretLocationInRange::Contained
        );
        assert_eq2!(
            range.locate_column(caret_scr_adj(col(3) + row(0))),
            CaretLocationInRange::Contained
        );
        assert_eq2!(
            range.locate_column(caret_scr_adj(col(4) + row(0))),
            CaretLocationInRange::Overflow
        );
        assert_eq2!(
            range.locate_column(caret_scr_adj(col(5) + row(0))),
            CaretLocationInRange::Overflow
        );
    }

    #[test]
    fn test_tuple() {
        let range = {
            let start = caret_scr_adj(col(1) + row(0));
            let end = caret_scr_adj(col(4) + row(0));
            SelectionRange::new(start, end)
        };
        assert_eq2!(range.as_tuple(), (col(1), col(4)));
    }
}

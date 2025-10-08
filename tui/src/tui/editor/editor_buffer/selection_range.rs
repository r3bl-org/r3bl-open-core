// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CaretScrAdj, ChUnitPrimitiveType, ColIndex, ColWidth, GapBufferLine, ScrOfs,
            caret_scr_adj, row, width};
use std::{cmp::{self},
          fmt::Debug};

/// Represents a range of characters in a line. The col indices are scroll adjusted (and
/// not raw). The row indices are not used, and clobbered with
/// [`ChUnitPrimitiveType::MAX`].
///
/// The range is not inclusive of the item at the end index, which means that when you
/// call [`SelectionRange::clip_to_range_str()`] the item at the end index will not be
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
/// - [`SelectionRange::clip_to_range_str()`] : "e😃"
///
/// ## Selection Range Semantics
///
/// When checking if an index is within a selection, inclusive range checking is typically
/// used. Here's how text selection works with character-level precision:
///
/// ```text
/// Text Selection Example:
/// Original text: "The quick brown fox jumps"
/// Selected text: "quick brown" (indices 4-14 inclusive)
///
///       selection_start=4                      selection_end=14
///               ↓                                      ↓
/// Index:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Char:   │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │ x │
///         └───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┴───┘
///                         ╰───────────── selected range ──────────────╯
///
/// Checking if index is selected (using inclusive range):
/// - (start..=end).contains(&idx(3))  → false (before selection)
/// - (start..=end).contains(&idx(4))  → true  (at start boundary)
/// - (start..=end).contains(&idx(9))  → true  (within selection)
/// - (start..=end).contains(&idx(14)) → true  (at end boundary)
/// - (start..=end).contains(&idx(15)) → false (after selection)
/// ```
///
/// This range can't be instantiated directly via the struct, you have to use the tuple
/// conversion. Even though the struct holds two [`CaretScrAdj`] values, it does not use
/// the [`crate::RowIndex`] fields.
#[derive(Default, Clone, PartialEq, Copy)]
pub struct SelectionRange {
    /// This is not "raw", this is "scroll adjusted".
    /// - It represents the display width at which the selection starts.
    /// - The [`crate::RowIndex`] field is not used and is clobbered with
    ///   [`ChUnitPrimitiveType::MAX`] after initialization.
    /// - The display width is used, to support variable width characters. `UTF-8`
    ///   encoding uses between 1 and 4 bytes to encode a character, e.g.: `"H"` is 1
    ///   byte, and `"😄"` is 4 bytes. And visually they can occupy 1 or more spaces,
    ///   e.g.: `"H"` is 1 space wide, and `"😄"` is two spaces wide
    ///   [`crate::GCString::width()`] and [`crate::GCString::width_char()`].
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
    ///   [`crate::GCString::width()`] and [`crate::GCString::width_char()`].
    end: CaretScrAdj,
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum ScrollOffsetColLocationInRange {
    Overflow,
    Underflow,
}

/// The only way to construct a [`SelectionRange`] is by converting a tuple of
/// [`CaretScrAdj`] values into a [`SelectionRange`].
mod convert {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<(CaretScrAdj, CaretScrAdj)> for SelectionRange {
        /// The [`crate::RowIndex`] fields of each tuple value are not used. They are just
        /// set to the maximum value of [`ChUnitPrimitiveType`].
        fn from((start, end): (CaretScrAdj, CaretScrAdj)) -> Self {
            let start = caret_scr_adj(start.col_index + row(ChUnitPrimitiveType::MAX));
            let end = caret_scr_adj(end.col_index + row(ChUnitPrimitiveType::MAX));
            Self { start, end }
        }
    }
}

impl SelectionRange {
    #[must_use]
    pub fn start(&self) -> ColIndex { self.start.col_index }

    #[must_use]
    pub fn end(&self) -> ColIndex { self.end.col_index }

    /// Due to the nature of selection ranges, the index values are actually display
    /// widths. And sometimes it is useful to type cast them as a width, e.g.: when using
    /// with [`SelectionRange::clip_to_range_str()`].
    #[must_use]
    pub fn get_start_display_col_index_as_width(&self) -> ColWidth {
        width(*self.start.col_index)
    }

    /// Returns a tuple of the start and end display column indices. This is just a
    /// convenience function that prevents the need to access the fields directly to get
    /// the two [`ColIndex`] values.
    #[must_use]
    pub fn as_tuple(&self) -> (ColIndex, ColIndex) {
        (self.start.col_index, self.end.col_index)
    }

    /// Clip a line to this selection range using [`crate::LineMetadata`] for
    /// Unicode-safe clipping.
    ///
    /// This method extracts a substring from the line content based on this selection's
    /// display column range, properly handling Unicode grapheme clusters and multi-width
    /// characters. It delegates to [`crate::LineMetadata::clip_to_range()`] for
    /// the actual Unicode-safe clipping.
    ///
    /// # Arguments
    /// * `line_with_info` - Line content and metadata from `get_line()`
    ///
    /// # Returns
    /// A string slice containing the selected text
    ///
    /// # Example
    /// ```rust
    /// use r3bl_tui::{SelectionRange, caret_scr_adj, col, row, ZeroCopyGapBuffer};
    ///
    /// # let mut buffer = ZeroCopyGapBuffer::new();
    /// # buffer.add_line();
    /// let selection = SelectionRange::new(
    ///     caret_scr_adj(col(2) + row(0)),
    ///     caret_scr_adj(col(6) + row(0))
    /// );
    /// let line = buffer.get_line(row(0)).unwrap();
    /// let selected_text = selection.clip_to_range_str(line);
    /// ```
    #[must_use]
    pub fn clip_to_range_str<'a>(&self, line: GapBufferLine<'a>) -> &'a str {
        let content = line.content();
        let line_info = line.info();
        let (start_display_col_index, end_display_col_index) = self.as_tuple();
        let max_display_width_col_count =
            width(*(end_display_col_index - start_display_col_index));
        line_info.clip_to_range(
            content,
            start_display_col_index,
            max_display_width_col_count,
        )
    }
}

impl SelectionRange {
    #[must_use]
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
    #[must_use]
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

    #[must_use]
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

    #[must_use]
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
    /// - [`SelectionRange::clip_to_range_str()`] : "ell"
    #[must_use]
    pub fn locate_column(&self, caret: CaretScrAdj) -> CaretLocationInRange {
        // Selection ranges use exclusive upper bound semantics [start, end)
        // This means end position is NOT included in the selection
        if caret.col_index < self.start.col_index {
            CaretLocationInRange::Underflow
        } else if caret.col_index >= self.end.col_index {
            CaretLocationInRange::Overflow
        } else {
            CaretLocationInRange::Contained
        }
    }

    /// Alternatively you can also just use a tuple of [`ColIndex`] to represent the
    /// range.
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
    #[must_use]
    pub fn new(start: CaretScrAdj, end: CaretScrAdj) -> Self { Self { start, end } }

    /// ```text
    /// ╭0123456789╮
    /// │hello worl│
    /// ╰─┬──┬─────╯
    ///   │  │
    ///   │  ⎩end_display_col_index + col_amt
    ///   ⎩start_display_col_index
    /// ```
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn shrink_start_by(&self, col_amt: ColWidth) -> Self {
        let mut copy = *self;
        copy.start.col_index += col_amt;
        copy
    }
}

mod range_impl_debug_format {
    #[allow(clippy::wildcard_imports)]
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
    use crate::{assert_eq2, col};

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

// Copyright (c) 2023-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CCol, CPos, CRow, GapBufferLine, RangeExclusive};
use std::fmt::Debug;

/// Represents a range of characters on a single line in [`Canvas`] column coordinates
/// ([`CCol`]).
///
/// # Per-Line Building Block
///
/// [`SelectionLine`] is the **per-line building block** of text selection. It represents
/// a 1D column selection range (`start_col..end_col`) on a single line (`row`).
///
/// For multi-line selections across multiple rows, see [`SelectionContainer`], which
/// stores a list of [`SelectionLine`] items.
///
/// [`SelectionLine`] stores `row` as [`CRow`], `start_col` as [`CCol`], and `end_col` as
/// [`CCol`].
///
/// The range is not inclusive of the item at the end index, which means that when you
/// call [`SelectionLine::clip_to_range_str()`] the item at the end index will not be
/// part of the result (this is shown in the example below). The indices are all display
/// column indices, not logical ones.
///
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
/// - [`SelectionLine::clip_to_range_str()`] : "e😃"
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
///                   selection_start=4                      selection_end=14
///                           ↓                                      ↓
/// Index:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Char:   │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │ x │
///         └───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┴───┘
///                         ╰───────────── selected range ──────────────╯
///
/// Checking if index is selected (using inclusive range):
/// - (start..=end).contains(&idx(3u16))  → false (before selection)
/// - (start..=end).contains(&idx(4u16))  → true  (at start boundary)
/// - (start..=end).contains(&idx(9u16))  → true  (within selection)
/// - (start..=end).contains(&idx(14u16)) → true  (at end boundary)
/// - (start..=end).contains(&idx(15u16)) → false (after selection)
/// ```
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [`CRow`]: crate::CRow
/// [`SelectionContainer`]: crate::SelectionContainer
#[derive(Default, Clone, PartialEq, Eq)]
pub struct SelectionLine {
    /// Represents the line row position ([`CRow`]).
    pub row: CRow,

    /// Represents the exclusive column range ([`RangeExclusive<CCol>`]) for the
    /// selection on this line.
    pub col_range: RangeExclusive<CCol>,
}

mod convert {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<(CRow, RangeExclusive<CCol>)> for SelectionLine {
        fn from((row, col_range): (CRow, RangeExclusive<CCol>)) -> SelectionLine {
            SelectionLine { row, col_range }
        }
    }
}

/// Creates a new [`SelectionLine`] on the given row spanning the exclusive column range
/// `col_range`.
#[must_use]
pub fn c_sel_line(row: CRow, col_range: RangeExclusive<CCol>) -> SelectionLine {
    (row, col_range).into()
}

/// Accessors, string clipping methods.
impl SelectionLine {
    /// Returns the row index ([`CRow`]) for this line selection.
    #[must_use]
    pub fn get_row(&self) -> CRow { self.row }

    /// Returns the starting column index ([`CCol`]) of the selection range.
    #[must_use]
    pub fn get_start(&self) -> CCol { self.col_range.start }

    /// Returns the exclusive end column index ([`CCol`]) of the selection range.
    #[must_use]
    pub fn get_end(&self) -> CCol { self.col_range.end }

    /// Clip a line to this selection range using [`crate::LineMetadata`] for Unicode-safe
    /// clipping.
    ///
    /// This method extracts a substring from the line content based on this selection's
    /// display column range, properly handling Unicode grapheme clusters and multi-width
    /// characters. It delegates to [`crate::LineMetadata::clip_to_range()`] for the
    /// actual Unicode-safe clipping.
    ///
    /// # Arguments
    /// * `line_with_info` - Line content and metadata from `get_line()`
    ///
    /// # Returns
    /// A string slice containing the selected text
    ///
    /// # Example
    /// ```rust
    /// use r3bl_tui::{c_col, c_row, c_sel_line, ZeroCopyGapBuffer};
    ///
    /// let mut buffer = ZeroCopyGapBuffer::default();
    /// buffer.add_line();
    /// let selection = c_sel_line(
    ///     c_row(0),
    ///     c_col(2)..c_col(6),
    /// );
    /// let line = buffer.get_line(c_row(0)).expect("conversion error");
    /// let selected_text = selection.clip_to_range_str(line);
    /// ```
    #[must_use]
    pub fn clip_to_range_str<'a>(&self, line: GapBufferLine<'a>) -> &'a str {
        let content = line.content();
        let line_info = line.info();
        line_info.clip_to_range(content, self.col_range.clone())
    }
}

/// Viewport-relative positioning & horizontal scrolling clipping.
impl SelectionLine {
    /// Determines whether the start column of this selection line is visible at or to the
    /// right of the viewport origin (`vp_origin.col_index`).
    ///
    /// ```text
    /// Case 1: VisibleInsideVP (selection_start=5 >= vp_origin=3)
    ///
    ///             vp_origin=3  selection_start=5  selection_end=9
    ///                       ↓       ↓               ↓
    /// Index:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Char:   │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │ x │
    ///         └───┴───┴───┼───┴───┼───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    ///                     │       ╰── selected ───╯
    ///                     ╰─────────────── viewport window ───────────────────────────────►
    ///
    /// Case 2: NotVisibleAtLeftOfVPOrigin (selection_start=1 < vp_origin=4)
    ///
    ///     selection_start=1   vp_origin=4         selection_end=9
    ///               ↓           ↓                   ↓
    /// Index:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Char:   │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │ x │
    ///         └───┼───┴───┴───┴───┼───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    ///             ╰── selected ───┼───────────────╯
    ///                             ╰─────── viewport window ───────────────────────────────►
    /// ```
    ///
    /// # Returns
    /// - [`SelectionStartVPLocation::VisibleInsideVP`] if `start >= vp_origin.col_index`
    /// - [`SelectionStartVPLocation::NotVisibleAtLeftOfVPOrigin`] if `start <
    ///   vp_origin.col_index`
    #[must_use]
    pub fn locate_start_rel_to_vp_origin(
        &self,
        vp_origin: CPos,
    ) -> SelectionStartVPLocation {
        if (vp_origin.col_index..).contains(&self.col_range.start) {
            SelectionStartVPLocation::VisibleInsideVP
        } else {
            SelectionStartVPLocation::NotVisibleAtLeftOfVPOrigin
        }
    }

    /// Clips the left edge of this selection line to `vp_origin.col_index` if the
    /// selection began to the left of the visible viewport window during horizontal
    /// scrolling.
    ///
    /// ```text
    /// Before clipping (selection: cols 1..9, vp_origin=4):
    ///
    ///   selection_start=1     vp_origin=4         selection_end=9
    ///               ↓           ↓                   ↓
    /// Index:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Char:   │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │ x │
    ///         └───┼───┴───┴───┴───┼───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    ///             ╰── unclipped ──┼───────────────╯
    ///                             ╰─────── viewport window ───────────────────────────────►
    ///
    /// After clip_left_to_vp_origin:
    ///
    ///                    clipped_start=4       selection_end=9
    ///                           ↓                   ↓
    /// Index:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Char:   │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │ x │
    ///         └───┴───┴───┴───┼───┴───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    ///                         ╰── clipped ────────╯
    ///                         ╰─────────── viewport window ───────────────────────────────►
    /// ```
    ///
    /// # Arguments
    /// * `vp_origin` - Viewport origin on the canvas ([`CPos`])
    /// * `row_index` - Target row index ([`CRow`]) for the clipped selection line
    ///
    /// # Returns
    /// A new [`SelectionLine`] clamped to start no earlier than `vp_origin.col_index`.
    #[must_use]
    pub fn clip_left_to_vp_origin(
        &self,
        vp_origin: CPos,
        row_index: CRow,
    ) -> SelectionLine {
        match self.locate_start_rel_to_vp_origin(vp_origin) {
            SelectionStartVPLocation::VisibleInsideVP => self.clone(),
            SelectionStartVPLocation::NotVisibleAtLeftOfVPOrigin => SelectionLine {
                row: row_index,
                col_range: vp_origin.col_index..self.col_range.end,
            },
        }
    }
}

/// Debug Formatting.
impl Debug for SelectionLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[row: {row:?}, col_range: {range:?}]",
            row = self.row,
            range = self.col_range
        )
    }
}

/// Indicates whether the start of a selection line is visible inside the viewport window
/// or begins to the left of the viewport origin (`vp_origin.col_index`).
#[derive(Clone, PartialEq, Copy, Debug)]
pub enum SelectionStartVPLocation {
    /// The selection start column is at or to the right of `vp_origin.col_index`.
    VisibleInsideVP,
    /// The selection start column is to the left of `vp_origin.col_index`.
    NotVisibleAtLeftOfVPOrigin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ZeroCopyGapBuffer, assert_eq2, c_col, c_pos, c_row};

    #[test]
    fn test_clip_to_range_str() {
        let buffer = ZeroCopyGapBuffer::from("Hello 😃 world");
        let line = buffer.get_line(c_row(0)).expect("conversion error");

        // "Hello " is cols 0..6
        let sel_hello = c_sel_line(c_row(0), c_col(0)..c_col(6));
        assert_eq2!(sel_hello.clip_to_range_str(line), "Hello ");

        // "😃" spans 2 display cols (cols 6..8)
        let sel_emoji = c_sel_line(c_row(0), c_col(6)..c_col(8));
        assert_eq2!(sel_emoji.clip_to_range_str(line), "😃");

        // " " is col 8..9
        let sel_space = c_sel_line(c_row(0), c_col(8)..c_col(9));
        assert_eq2!(sel_space.clip_to_range_str(line), " ");

        // "world" is cols 9..14
        let sel_world = c_sel_line(c_row(0), c_col(9)..c_col(14));
        assert_eq2!(sel_world.clip_to_range_str(line), "world");
    }

    #[test]
    fn test_locate_start_rel_to_vp_origin() {
        let range = c_sel_line(c_row(0), c_col(5)..c_col(10));

        // Viewport origin at col 5 -> selection start col 5 is visible (VisibleInsideVP).
        let vp_origin = c_pos(c_col(5), c_row(0));
        assert_eq2!(
            range.locate_start_rel_to_vp_origin(vp_origin),
            SelectionStartVPLocation::VisibleInsideVP
        );

        // Viewport origin at col 8 -> selection start col 5 is scrolled off left
        // (NotVisibleAtLeftOfVPOrigin).
        let vp_origin = c_pos(c_col(8), c_row(0));
        assert_eq2!(
            range.locate_start_rel_to_vp_origin(vp_origin),
            SelectionStartVPLocation::NotVisibleAtLeftOfVPOrigin
        );
    }

    #[test]
    fn test_clip_left_to_vp_origin() {
        let range = c_sel_line(c_row(0), c_col(5)..c_col(10));

        // When selection start is inside VP, no clipping occurs.
        let vp_origin = c_pos(c_col(3), c_row(0));
        let clipped = range.clip_left_to_vp_origin(vp_origin, c_row(0));
        assert_eq2!(clipped.get_start(), c_col(5));
        assert_eq2!(clipped.get_end(), c_col(10));

        // When selection start is at left of VP origin, clipped start becomes
        // vp_origin.col_index.
        let vp_origin = c_pos(c_col(8), c_row(0));
        let clipped = range.clip_left_to_vp_origin(vp_origin, c_row(0));
        assert_eq2!(clipped.get_start(), c_col(8));
        assert_eq2!(clipped.get_end(), c_col(10));
    }

    #[test]
    fn test_c_sel_line_and_from_conversion() {
        let sel1 = c_sel_line(c_row(0), c_col(2)..c_col(8));
        let sel2: SelectionLine = (c_row(0), c_col(2)..c_col(8)).into();
        assert_eq2!(sel1, sel2);
        assert_eq2!(sel1.get_row(), c_row(0));
        assert_eq2!(sel1.get_start(), c_col(2));
        assert_eq2!(sel1.get_end(), c_col(8));
    }

    #[test]
    fn test_debug_format() {
        let range = c_sel_line(c_row(1), c_col(2)..c_col(5));
        let formatted = format!("{range:?}");
        assert!(formatted.contains("col_range") && formatted.contains("row"));
    }
}

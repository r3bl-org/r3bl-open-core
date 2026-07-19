// Copyright (c) 2023-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CCaret, CRow, DeleteSelectionWith, EditorBuffer, GetMemSize, InlineString,
            InlineVec, ItemsOwned, SelectionLine, c_caret, fg_color,
            glyphs::{CUT_GLYPH, DIRECTION_GLYPH, ELLIPSIS_GLYPH, TIRE_MARKS_GLYPH,
                     VERT_LINE_DASHED_GLYPH},
            inline_string, join, tui_color};
use smallvec::smallvec;
use std::{fmt::Debug, mem::size_of};

/// Represents a selection across the editor buffer, containing one or more per-line
/// selections ([`SelectionLine`]).
///
/// # Selection Container
///
/// [`SelectionContainer`] is the **selection container** in the editor buffer. It stores
/// an optional `anchor_caret` ([`CCaret`]) and a sorted list of per-line
/// [`SelectionLine`] items.
///
/// - **Per-Line Building Block**: [`SelectionLine`] handles the 1D column selection
///   (`start_col..end_col`) on a single line (`row`).
/// - **Selection Container**: [`SelectionContainer`] aggregates those per-line ranges
///   into a complete selection spanning multiple rows.
///
/// # Deterministic Selection Algorithm
///
/// Selections are computed deterministically from `anchor_caret` and `active_caret`. For
/// detailed architectural documentation and [`ASCII`] visual diagrams of the selection
/// engine, see the [`AnchorState`].
///
/// [`AnchorState`]: super::AnchorState
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CCaret`]: crate::CCaret
/// [`CRow`]: crate::CRow
#[derive(Clone, PartialEq, Default)]
pub struct SelectionContainer {
    /// Origin caret where selection began, or [`None`] when selection is inactive or
    /// inferred from existing selection boundaries.
    pub anchor_caret: Option<CCaret>,

    /// Sorted list of per-line selection ranges ([`SelectionLine`]), ordered by `row`.
    ///
    /// [`SelectionLine`]: crate::SelectionLine
    pub list: InlineVec<SelectionLine>,
}

mod sizing {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GetMemSize for SelectionContainer {
        fn get_mem_size(&self) -> usize {
            let item_size = size_of::<SelectionLine>();
            let list_len = self.list.len();
            let list_size = item_size * list_len;
            let field_anchor_size = size_of::<Option<CCaret>>();
            list_size + field_anchor_size
        }
    }
}

// Functionality.
impl SelectionContainer {
    /// Returns the canvas caret position ([`CCaret`]) at the start of the first selection
    /// line in the container.
    ///
    /// This is typically used to place the caret after a deletion or backspace operation
    /// on the selected text.
    ///
    /// # Arguments
    /// * `_with` - The deletion method ([`DeleteSelectionWith`]).
    ///
    /// # Returns
    /// - `Some(CCaret)` at the start of the first line selection if one exists.
    /// - `None` if the selection container is empty.
    ///
    /// [`CCaret`]: crate::CCaret
    /// [`DeleteSelectionWith`]: crate::DeleteSelectionWith
    #[must_use]
    pub fn get_c_caret_at_start_of_range(
        &self,
        _with: DeleteSelectionWith,
    ) -> Option<CCaret> {
        let first_selection_line = self.list.first()?;
        Some(c_caret(
            first_selection_line.get_start() + first_selection_line.row,
        ))
    }

    /// Extracts and clips the selected text slices for each selected row from the given
    /// [`EditorBuffer`].
    ///
    /// Iterates through all [`SelectionLine`] items in the container, extracts each
    /// corresponding line from the buffer, and clips it to the selected column range.
    ///
    /// # Arguments
    /// * `buffer` - Reference to the [`EditorBuffer`] containing the line text.
    ///
    /// # Returns
    /// An [`InlineVec`] of `(CRow, &'a str)` pairs representing each selected row and its
    /// selected text slice.
    ///
    /// [`CRow`]: crate::CRow
    /// [`EditorBuffer`]: crate::EditorBuffer
    /// [`InlineVec`]: crate::InlineVec
    /// [`SelectionLine`]: crate::SelectionLine
    #[must_use]
    pub fn get_selected_lines<'a>(
        &self,
        buffer: &'a EditorBuffer,
    ) -> InlineVec<(CRow, &'a str)> {
        let mut acc = InlineVec::new();
        let lines = buffer.get_lines();

        for item in &self.list {
            if let Some(line_with_info) = lines.get_line(item.row) {
                let sel_text = item.clip_to_range_str(line_with_info);
                acc.push((item.row, sel_text));
            }
        }

        acc
    }

    /// Returns a slice view of the underlying [`SelectionLine`] items in the container.
    ///
    /// [`SelectionLine`]: crate::SelectionLine
    #[must_use]
    pub fn as_slice(&self) -> &[SelectionLine] { self.list.as_slice() }

    /// Returns `true` if there are no active [`SelectionLine`] items in the container.
    ///
    /// [`SelectionLine`]: crate::SelectionLine
    #[must_use]
    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    /// Returns the first and last [`SelectionLine`] in the selection, or [`None`] if
    /// empty.
    ///
    /// - If the selection contains 1 line, both `first` and `last` point to that line.
    /// - If the selection contains 2+ lines, `first` and `last` point to the respective
    ///   boundaries.
    /// - If the selection is empty, returns [`None`].
    ///
    /// [`SelectionLine`]: crate::SelectionLine
    #[must_use]
    pub fn boundaries(&self) -> Option<(&SelectionLine, &SelectionLine)> {
        match self.as_slice() {
            [] => None,
            [single] => Some((single, single)),
            [first, .., last] => Some((first, last)),
        }
    }

    /// Clears all selection lines in the container and resets [`anchor_caret`] to
    /// [`None`].
    ///
    /// [`anchor_caret`]: Self::anchor_caret
    pub fn clear(&mut self) {
        self.list.clear();
        self.anchor_caret = None;
    }

    /// Returns the number of [`SelectionLine`] items (selected rows) in the container.
    ///
    /// [`SelectionLine`]: crate::SelectionLine
    #[must_use]
    pub fn len(&self) -> usize { self.list.len() }

    /// Returns an iterator over references to the [`SelectionLine`] items in the
    /// container.
    ///
    /// [`SelectionLine`]: crate::SelectionLine
    pub fn iter(&self) -> impl Iterator<Item = &SelectionLine> { self.list.iter() }

    /// Finds and returns the [`SelectionLine`] for the specified row index ([`CRow`]),
    /// if one exists.
    ///
    /// # Arguments
    /// * `row_index` - The canvas row position ([`CRow`]) to look up.
    ///
    /// # Returns
    /// - `Some(SelectionLine)` if a selection exists on that row.
    /// - `None` otherwise.
    ///
    /// [`CRow`]: crate::CRow
    /// [`SelectionLine`]: crate::SelectionLine
    #[must_use]
    pub fn get(&self, row_index: CRow) -> Option<SelectionLine> {
        self.list.iter().find_map(|item| {
            if item.row == row_index {
                Some(item.clone())
            } else {
                None
            }
        })
    }

    /// Inserts or updates a [`SelectionLine`] in the list, ensuring the internal list
    /// remains sorted by `row`.
    ///
    /// If a selection already exists on the same row, it is replaced with the new range.
    ///
    /// # Arguments
    /// * `arg_line_selection` - A [`SelectionLine`] or type convertible into one (such as
    ///   `(CRow, RangeExclusive<CCol>)`).
    ///
    /// [`SelectionLine`]: crate::SelectionLine
    pub fn insert(&mut self, arg_line_selection: impl Into<SelectionLine>) {
        let line_selection = arg_line_selection.into();
        if let Some(existing_pos) = self
            .list
            .iter()
            .position(|item| item.row == line_selection.row)
        {
            self.list[existing_pos] = line_selection;
        } else {
            self.list.push(line_selection);
        }
        self.list.sort_by_key(|item| item.row);
    }
}

// Formatter for Debug and Display.
mod impl_debug_format {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    const PAD_LEFT: &str = "      ";
    const EMPTY_STR: &str = "--empty--";

    impl SelectionContainer {
        /// Formats the selection container into an [`InlineString`] with terminal color
        /// styling for human-readable debugging output.
        ///
        /// [`InlineString`]: crate::InlineString
        #[must_use]
        pub fn to_formatted_string(&self) -> InlineString {
            let mut selection_list_string = self.to_unformatted_string();

            let is_empty = selection_list_string
                .iter()
                .any(|line| line.contains(EMPTY_STR));

            // Format the output.
            for line in selection_list_string.iter_mut() {
                let (fg, bg) = if is_empty {
                    (tui_color!(frozen_blue), tui_color!(dark_gray))
                } else {
                    (tui_color!(lizard_green), tui_color!(dark_gray))
                };
                let fmt_line = fg_color(fg, line).bg_color(bg).to_small_str();
                *line = fmt_line;
            }
            for line in selection_list_string.iter_mut() {
                *line = inline_string!("{PAD_LEFT}{line}");
            }

            let selection_list_string = join!(
                from: selection_list_string,
                each: item,
                delim: "\n",
                format: "{item}",
            );

            inline_string! {
"Selection: [
{selection_list_string}
{PAD_LEFT}]"
            }
        }

        /// Generates unformatted debug strings representing each [`SelectionLine`] and
        /// the `anchor_caret` state.
        ///
        /// # Returns
        /// An [`ItemsOwned`] vector of [`InlineString`] items.
        ///
        /// [`InlineString`]: crate::InlineString
        /// [`ItemsOwned`]: crate::ItemsOwned
        /// [`SelectionLine`]: crate::SelectionLine
        #[must_use]
        pub fn to_unformatted_string(&self) -> ItemsOwned {
            let mut vec_output: InlineVec<InlineString> = {
                let mut acc = smallvec![];
                for item in &self.list {
                    acc.push(inline_string!(
                        "{first_ch} {sep}row: {row_idx:?}, col: [{col_start:?}{dots}{col_end:?}]{sep}",
                        first_ch = CUT_GLYPH,
                        sep = VERT_LINE_DASHED_GLYPH,
                        row_idx = item.row,
                        dots = ELLIPSIS_GLYPH,
                        col_start = item.get_start(),
                        col_end = item.get_end()
                    ));
                }
                acc
            };

            if vec_output.is_empty() {
                vec_output.push(inline_string!(
                    "{a} {b}{c}{d}",
                    a = TIRE_MARKS_GLYPH,
                    b = VERT_LINE_DASHED_GLYPH,
                    c = EMPTY_STR,
                    d = VERT_LINE_DASHED_GLYPH
                ));
            }

            vec_output.push(inline_string!(
                "{ch} anchor_caret: {anchor_caret:?}",
                ch = DIRECTION_GLYPH,
                anchor_caret = self.anchor_caret
            ));

            vec_output.into()
        }
    }

    // Other trait impls.
    impl Debug for SelectionContainer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.to_formatted_string())
        }
    }
}

#[cfg(test)]
mod tests_selection {
    use super::*;
    use crate::{assert_eq2, c_col, c_row, c_sel_line};

    #[test]
    fn test_anchor_caret() {
        let mut selection = SelectionContainer::default();
        assert_eq2!(selection.anchor_caret, None);
        assert!(selection.is_empty());

        let anchor = c_caret(c_col(2) + c_row(1));
        selection.anchor_caret = Some(anchor);
        assert_eq2!(selection.anchor_caret, Some(anchor));

        selection.insert((c_row(1), c_col(2)..c_col(8)));
        assert_eq2!(selection.len(), 1);

        selection.clear();
        assert_eq2!(selection.anchor_caret, None);
        assert!(selection.is_empty());
    }

    #[test]
    fn test_selection_operations() {
        let mut selection = SelectionContainer::default();
        let sel1 = c_sel_line(c_row(2), c_col(0)..c_col(5));
        let sel2 = c_sel_line(c_row(0), c_col(2)..c_col(8));

        selection.insert(sel1.clone());
        selection.insert(sel2.clone());

        // Sorting check
        assert_eq2!(selection.len(), 2);
        assert_eq2!(
            selection.iter().map(|item| item.row).collect::<Vec<_>>(),
            vec![c_row(0), c_row(2)]
        );

        // Retrieval check
        assert_eq2!(selection.get(c_row(0)), Some(sel2.clone()));
        assert_eq2!(selection.get(c_row(1)), None);
        assert_eq2!(selection.get(c_row(2)), Some(sel1.clone()));

        // Caret at start of range
        let start_caret =
            selection.get_c_caret_at_start_of_range(DeleteSelectionWith::Backspace);
        assert_eq2!(start_caret, Some(c_caret(c_col(2) + c_row(0))));

        // Iteration
        let items: Vec<_> = selection.iter().collect();
        assert_eq2!(items.len(), 2);
        assert_eq2!(items[0], &sel2);
        assert_eq2!(items[1], &sel1);
    }

    #[test]
    fn test_get_selected_lines() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Hello world", "R3BL TUI", "Rust language"]);

        let mut selection = SelectionContainer::default();
        selection.insert((c_row(0), c_col(0)..c_col(5)));
        selection.insert((c_row(2), c_col(0)..c_col(4)));

        let selected = selection.get_selected_lines(&buffer);
        assert_eq2!(selected.len(), 2);
        assert_eq2!(selected[0], (c_row(0), "Hello"));
        assert_eq2!(selected[1], (c_row(2), "Rust"));
    }

    #[test]
    fn test_insert_updates_existing_row() {
        let mut selection = SelectionContainer::default();
        selection.insert((c_row(0), c_col(0)..c_col(5)));
        assert_eq2!(selection.len(), 1);

        // Overwrite row 0 with a new range
        selection.insert((c_row(0), c_col(2)..c_col(10)));
        assert_eq2!(selection.len(), 1);
        assert_eq2!(
            selection.get(c_row(0)),
            Some(c_sel_line(c_row(0), c_col(2)..c_col(10)))
        );
    }

    #[test]
    fn test_get_c_caret_at_start_of_range_empty() {
        let selection = SelectionContainer::default();
        assert_eq2!(
            selection.get_c_caret_at_start_of_range(DeleteSelectionWith::Backspace),
            None
        );
    }

    #[test]
    fn test_selection_boundaries() {
        let mut selection = SelectionContainer::default();
        // Empty case.
        assert_eq2!(selection.boundaries(), None);

        // Single-line case.
        let sel1 = c_sel_line(c_row(0), c_col(2)..c_col(8));
        selection.insert(sel1.clone());
        assert_eq2!(selection.boundaries(), Some((&sel1, &sel1)));

        // Multi-line case.
        let sel2 = c_sel_line(c_row(3), c_col(0)..c_col(5));
        selection.insert(sel2.clone());
        assert_eq2!(selection.boundaries(), Some((&sel1, &sel2)));

        // 3-line case.
        let sel_mid = c_sel_line(c_row(1), c_col(0)..c_col(10));
        selection.insert(sel_mid);
        assert_eq2!(selection.boundaries(), Some((&sel1, &sel2)));
    }

    #[test]
    fn test_mem_size_and_debug_format() {
        let mut selection = SelectionContainer::default();
        selection.insert((c_row(0), c_col(0)..c_col(5)));

        assert!(selection.get_mem_size() > 0);

        let debug_str = format!("{selection:?}");
        assert!(debug_str.contains("Selection:"));
    }
}

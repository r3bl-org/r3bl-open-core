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
use std::fmt::Debug;

use r3bl_ansi_color::{fg_rgb_color, rgb_color};
use r3bl_core::{caret_scr_adj,
                glyphs::{CUT_GLYPH,
                         DIRECTION_GLYPH,
                         ELLIPSIS_GLYPH,
                         TIRE_MARKS_GLYPH,
                         VERT_LINE_DASHED_GLYPH},
                inline_string,
                join,
                usize,
                CaretScrAdj,
                GetMemSize,
                InlineString,
                InlineVec,
                RowIndex};
use sizing::VecRowIndex;
use smallvec::{smallvec, SmallVec};

use crate::{CaretMovementDirection, DeleteSelectionWith, EditorBuffer, SelectionRange};

mod sizing {
    use super::*;
    pub(crate) type VecRowIndex = SmallVec<[RowIndex; ROW_INDEX_SIZE]>;

    const ROW_INDEX_SIZE: usize = 32;

    impl GetMemSize for SelectionList {
        fn get_mem_size(&self) -> usize {
            let item_size = std::mem::size_of::<SelectionListItem>();
            let list_len = self.list.len();
            let list_size = item_size * list_len;
            let field_direction_size =
                std::mem::size_of::<Option<CaretMovementDirection>>();
            list_size + field_direction_size
        }
    }
}

/// Key is the row index, value is the selected range in that line (display col index
/// range). This list is always sorted by row index.
///
/// Note that both column indices are:
/// - [r3bl_core::CaretScrAdj]
/// - And not [r3bl_core::CaretRaw]
#[derive(Clone, PartialEq, Default)]
pub struct SelectionList {
    list: InlineVec<SelectionListItem>,
    maybe_previous_direction: Option<CaretMovementDirection>,
}

pub type SelectionListItem = (RowIndex, SelectionRange);

#[test]
fn test_selection_map_direction_change() {
    use r3bl_core::assert_eq2;

    use super::*;

    // Not set.
    {
        let map = SelectionList {
            maybe_previous_direction: None,
            ..Default::default()
        };

        let current_direction = CaretMovementDirection::Down;
        let actual = map.has_caret_movement_direction_changed(current_direction);
        let expected = DirectionChangeResult::DirectionIsTheSame;

        assert_eq2!(actual, expected);
    }

    // Different.
    {
        let map = SelectionList {
            maybe_previous_direction: Some(CaretMovementDirection::Up),
            ..Default::default()
        };

        let current_direction = CaretMovementDirection::Down;
        let actual = map.has_caret_movement_direction_changed(current_direction);
        let expected = DirectionChangeResult::DirectionHasChanged;

        assert_eq2!(actual, expected);
    }

    // Same.
    {
        let map = SelectionList {
            maybe_previous_direction: Some(CaretMovementDirection::Down),
            ..Default::default()
        };

        let current_direction = CaretMovementDirection::Down;
        let actual = map.has_caret_movement_direction_changed(current_direction);
        let expected = DirectionChangeResult::DirectionIsTheSame;

        assert_eq2!(actual, expected);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionChangeResult {
    DirectionHasChanged,
    DirectionIsTheSame,
}

// Functionality.
impl SelectionList {
    pub fn get_caret_at_start_of_range_scroll_adjusted(
        &self,
        _with: DeleteSelectionWith, /* Makes no difference for now. */
    ) -> Option<CaretScrAdj> {
        // Row is the first row in the map.
        // Column is the last row of the range.
        let indices = self.get_ordered_indices();
        let first_row_index = indices.first()?;
        let last_row_index = indices.last()?;
        let pos = {
            let sel_range = self.get(*last_row_index)?;
            let col_index = sel_range.start();
            let row_index = *first_row_index;
            col_index + row_index
        };
        Some(caret_scr_adj(pos))
    }

    pub fn get_selected_lines<'a>(
        &self,
        buffer: &'a EditorBuffer,
    ) -> InlineVec<(RowIndex, &'a str)> {
        let mut acc = InlineVec::new();

        let lines = buffer.get_lines();
        let ordered_row_indices = self.get_ordered_indices();

        for row_index in ordered_row_indices {
            if let Some(sel_range) = self.get(row_index) {
                if let Some(line_gcs) = lines.get(usize(row_index)) {
                    let sel_text = sel_range.clip_to_range(line_gcs);
                    acc.push((row_index, sel_text));
                }
            }
        }

        acc
    }

    /// This is used by the editor to get the ordered row indices, so they can be used to
    /// iterate through the selection map for selecting text.
    pub fn get_ordered_indices(&self) -> VecRowIndex {
        let mut acc = VecRowIndex::with_capacity(self.list.len());
        for (row_index, _) in &self.list {
            if !acc.contains(row_index) {
                acc.push(*row_index);
            }
        }
        acc
    }

    /// Primarily for testing.
    pub fn get_ordered_list(&self) -> &InlineVec<(RowIndex, SelectionRange)> {
        &self.list
    }

    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    pub fn clear(&mut self) {
        self.list.clear();
        self.maybe_previous_direction = None;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&RowIndex, &SelectionRange)> {
        self.list.iter().map(|(index, range)| (index, range))
    }

    pub fn get(&self, row_index: RowIndex) -> Option<SelectionRange> {
        self.list.iter().find_map(|(index, range)| {
            if *index == row_index {
                Some(*range)
            } else {
                None
            }
        })
    }

    /// Compares the given direction (`current_direction`) with the
    /// `maybe_previous_direction` field.
    /// - If there is no existing previous direction, it returns
    ///   [DirectionChangeResult::DirectionIsTheSame].
    /// - Otherwise it compares the two and returns [DirectionChangeResult] (whether
    ///   the direction has changed or not).
    pub fn has_caret_movement_direction_changed(
        &self,
        current_direction: CaretMovementDirection,
    ) -> DirectionChangeResult {
        if let Some(previous_direction) = self.maybe_previous_direction {
            if previous_direction != current_direction {
                return DirectionChangeResult::DirectionHasChanged;
            }
        }
        DirectionChangeResult::DirectionIsTheSame
    }

    /// The internal list is sorted once an insertion is made, so that `list` is always
    /// sorted.
    pub fn insert(
        &mut self,
        row_index: RowIndex,
        selection_range: SelectionRange,
        direction: CaretMovementDirection,
    ) {
        if let Some(existing_pos) =
            self.list.iter().position(|(index, _)| *index == row_index)
        {
            self.list[existing_pos] = (row_index, selection_range);
        } else {
            self.list.push((row_index, selection_range));
        }
        self.list.sort_by_key(|(row_index, _)| *row_index);
        self.update_previous_direction(direction);
    }

    pub fn remove(&mut self, row_index: RowIndex, direction: CaretMovementDirection) {
        if let Some(pos) = self.list.iter().position(|(index, _)| *index == row_index) {
            self.list.remove(pos);
        }
        self.update_previous_direction(direction);
    }

    pub fn update_previous_direction(&mut self, direction: CaretMovementDirection) {
        self.maybe_previous_direction = Some(direction);
    }

    pub fn remove_previous_direction(&mut self) { self.maybe_previous_direction = None; }

    /// Is there a selection range for the row_index of `row_index_arg` in the map?
    /// - The `list` field contains tuples of [RowIndex] and [SelectionRange].
    /// - So if the `row_index` can't be found in the map, it means that the row is not
    ///   selected, aka [RowLocationInSelectionList::Overflow].
    /// - Otherwise it means that some range of columns in that row is selected, aka
    ///   [RowLocationInSelectionList::Contained].
    pub fn locate_row(&self, query_row_index: RowIndex) -> RowLocationInSelectionList {
        for (row_index, _) in self.list.iter() {
            if &query_row_index == row_index {
                return RowLocationInSelectionList::Contained;
            }
        }
        RowLocationInSelectionList::Overflow
    }
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum RowLocationInSelectionList {
    Overflow,
    Contained,
}

// Formatter for Debug and Display.
mod impl_debug_format {
    use super::*;

    const PAD_LEFT: &str = "      ";
    const EMPTY_STR: &str = "--empty--";

    impl SelectionList {
        /// Get the output from [Self::to_unformatted_string] and format it with colors.
        /// And return that.
        pub fn to_formatted_string(&self) -> InlineString {
            let mut selection_list_string = self.to_unformatted_string();

            let is_empty = selection_list_string
                .iter()
                .any(|line| line.contains(EMPTY_STR));

            // Format the output.
            for line in selection_list_string.iter_mut() {
                let (fg_color, bg_color) = if is_empty {
                    (rgb_color!(frozen_blue), rgb_color!(dark_grey))
                } else {
                    (rgb_color!(lizard_green), rgb_color!(dark_grey))
                };
                let fmt_line = fg_rgb_color(fg_color, line)
                    .bg_rgb_color(bg_color)
                    .to_small_str();
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
"SelectionList: [
{selection_list_string}
{PAD_LEFT}]"
            }
        }

        /// Returns a [InlineVec] of [InlineString] that represent the selection map.
        pub fn to_unformatted_string(&self) -> InlineVec<InlineString> {
            let mut vec_output = {
                let mut acc = smallvec![];
                let sorted_indices = self.get_ordered_indices();
                for row_index in sorted_indices.iter() {
                    if let Some(selected_range) = self.get(*row_index) {
                        acc.push(inline_string!(
                            "{first_ch} {sep}row: {row_idx:?}, col: [{col_start:?}{dots}{col_end:?}]{sep}",
                            first_ch = CUT_GLYPH,
                            sep = VERT_LINE_DASHED_GLYPH,
                            row_idx = row_index,
                            dots = ELLIPSIS_GLYPH,
                            col_start = selected_range.start(),
                            col_end = selected_range.end()
                        ));
                    }
                }
                acc
            };

            if vec_output.is_empty() {
                vec_output.push(
                    inline_string!("{TIRE_MARKS_GLYPH} {VERT_LINE_DASHED_GLYPH}{EMPTY_STR}{VERT_LINE_DASHED_GLYPH}")
                );
            }

            vec_output.push(inline_string!(
                "{ch} prev_dir: {prev_dir:?}",
                ch = DIRECTION_GLYPH,
                prev_dir = self.maybe_previous_direction
            ));

            vec_output
        }
    }

    // Other trait impls.
    impl Debug for SelectionList {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.to_formatted_string())
        }
    }
}

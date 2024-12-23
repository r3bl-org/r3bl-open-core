/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::{collections::HashMap,
          fmt::{Debug, Display}};

use crossterm::style::Stylize;
use r3bl_core::{position,
                usize,
                CaretMovementDirection,
                ChUnit,
                Position,
                SelectionRange};
use serde::{Deserialize, Serialize};

use crate::{DeleteSelectionWith, EditorBuffer};

/// Key is the row index, value is the selected range in that line (display col index
/// range).
///
/// Note that both column indices are:
/// - [Scroll adjusted](crate::editor_buffer_struct::CaretKind::ScrollAdjusted).
/// - And not [raw](crate::editor_buffer_struct::CaretKind::Raw).
#[derive(Clone, PartialEq, Serialize, Deserialize, Default, size_of::SizeOf)]
pub struct SelectionMap {
    pub map: HashMap<RowIndex, SelectionRange>,
    pub maybe_previous_direction: Option<CaretMovementDirection>,
}

pub type RowIndex = ChUnit;

#[test]
fn test_selection_map_direction_change() {
    use r3bl_core::{assert_eq2, CaretMovementDirection};

    use super::*;

    // Not set.
    {
        let map = SelectionMap {
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
        let map = SelectionMap {
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
        let map = SelectionMap {
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
impl SelectionMap {
    pub fn get_caret_at_start_of_range(
        &self,
        _with: DeleteSelectionWith, /* Makes no difference for now. */
    ) -> Option<Position> {
        // Row is the first row in the map.
        // Column is the last row of the range.
        let indices = self.get_ordered_indices();
        let first_row_index = indices.first()?;
        let last_row_index = indices.first()?;
        Some(position!(
            col_index: self.map.get(last_row_index)?.start_display_col_index,
            row_index: *first_row_index
        ))
    }

    pub fn get_selected_lines<'a>(
        &self,
        buffer: &'a EditorBuffer,
    ) -> HashMap<RowIndex, &'a str> {
        let mut it = HashMap::new();

        let lines = buffer.get_lines();
        let row_indices = self.get_ordered_indices();

        for row_index in row_indices {
            if let Some(selection_range) = self.map.get(&row_index) {
                if let Some(line) = lines.get(usize(row_index)) {
                    let selected_text = line.clip_to_range(*selection_range);
                    it.insert(row_index, selected_text);
                }
            }
        }

        it
    }

    pub fn get_ordered_indices(&self) -> Vec<RowIndex> {
        let row_indices = {
            let mut it: Vec<ChUnit> = self.map.keys().copied().collect();
            it.sort();
            it
        };
        row_indices
    }

    pub fn is_empty(&self) -> bool { self.map.is_empty() }

    pub fn clear(&mut self) {
        self.map.clear();
        self.maybe_previous_direction = None;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&RowIndex, &SelectionRange)> {
        self.map.iter()
    }

    pub fn get(&self, row_index: RowIndex) -> Option<&SelectionRange> {
        self.map.get(&row_index)
    }

    /// Compares the given direction (`current_direction`) with the
    /// [maybe_previous_direction](Self::maybe_previous_direction).
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

    pub fn insert(
        &mut self,
        row_index: RowIndex,
        selection_range: SelectionRange,
        direction: CaretMovementDirection,
    ) {
        self.map.insert(row_index, selection_range);
        self.update_previous_direction(direction);
    }

    pub fn remove(&mut self, row_index: RowIndex, direction: CaretMovementDirection) {
        self.map.remove(&row_index);
        self.update_previous_direction(direction);
    }

    pub fn update_previous_direction(&mut self, direction: CaretMovementDirection) {
        self.maybe_previous_direction = Some(direction);
    }

    pub fn remove_previous_direction(&mut self) { self.maybe_previous_direction = None; }

    /// Is there a selection range for the row_index of `row_index_arg` in the map?
    /// - The [map](Self::map) contains key value pairs of [RowIndex] and
    ///   [SelectionRange].
    /// - So if the row_index can't be found in the map, it means that the row is not
    ///   selected, aka [RowLocationInSelectionMap::Overflow].
    /// - Otherwise it means that some range of columns in that row is selected, aka
    ///   [RowLocationInSelectionMap::Contained].
    pub fn locate_row(&self, row_index_arg: ChUnit) -> RowLocationInSelectionMap {
        for (row_index, _range) in self.map.iter() {
            if &row_index_arg == row_index {
                return RowLocationInSelectionMap::Contained;
            }
        }
        RowLocationInSelectionMap::Overflow
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Copy, Debug)]
pub enum RowLocationInSelectionMap {
    Overflow,
    Contained,
}

// Formatter for Debug and Display.
mod impl_debug_format {
    use super::*;
    const PAD_LEFT: &str = "      ";
    const EMPTY_STR: &str = "--empty--";

    use r3bl_core::glyphs::{CUT_GLYPH,
                            DIRECTION_GLYPH,
                            ELLIPSIS_GLYPH,
                            TIRE_MARKS_GLYPH,
                            VERT_LINE_DASHED_GLYPH};

    impl SelectionMap {
        pub fn to_formatted_string(&self) -> String {
            let mut selection_map_vec_output = self.to_unformatted_string();

            let is_empty = selection_map_vec_output
                .iter()
                .any(|line| line.contains(EMPTY_STR));

            // Format the output.
            for line in selection_map_vec_output.iter_mut() {
                if is_empty {
                    *line = line.to_string().blue().on_dark_grey().to_string();
                } else {
                    *line = line.to_string().green().on_dark_grey().to_string();
                }
            }
            for line in selection_map_vec_output.iter_mut() {
                *line = format!("{PAD_LEFT}{line}");
            }

            let selection_map_str = selection_map_vec_output.join("\n");

            let selection_map_str = format! {
"SelectionMap: [
{selection_map_str}
{PAD_LEFT}]"
            };

            selection_map_str
        }

        pub fn to_unformatted_string(&self) -> Vec<String> {
            let mut vec_output = {
                let mut it = vec![];
                let sorted_indices = self.get_ordered_indices();
                for row_index in sorted_indices.iter() {
                    if let Some(selected_range) = self.map.get(row_index) {
                        it.push(format!(
                            "{first_ch} {sep}row: {row_idx}, col: [{col_start}{dots}{col_end}]{sep}",
                            first_ch = CUT_GLYPH,
                            sep = VERT_LINE_DASHED_GLYPH,
                            row_idx = row_index,
                            dots = ELLIPSIS_GLYPH,
                            col_start = selected_range.start_display_col_index,
                            col_end = selected_range.end_display_col_index
                        ));
                    }
                }
                it
            };

            if vec_output.is_empty() {
                vec_output.push(
                    format!("{TIRE_MARKS_GLYPH} {VERT_LINE_DASHED_GLYPH}{EMPTY_STR}{VERT_LINE_DASHED_GLYPH}")
                );
            }

            vec_output.push(format!(
                "{ch} prev_dir: {prev_dir:?}",
                ch = DIRECTION_GLYPH,
                prev_dir = self.maybe_previous_direction
            ));

            vec_output
        }
    }

    // Other trait impls.
    impl Display for SelectionMap {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.to_formatted_string())
        }
    }

    impl Debug for SelectionMap {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.to_formatted_string())
        }
    }
}

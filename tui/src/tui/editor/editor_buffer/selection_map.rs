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

use std::collections::HashMap;

use crossterm::style::Stylize;
use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

/// Key is the row index, value is the selected range in that line (display col index
/// range).
///
/// Note that both column indices are:
/// - [Scroll adjusted](crate::editor_buffer_struct::CaretKind::ScrollAdjusted).
/// - And not [raw](crate::editor_buffer_struct::CaretKind::Raw).
#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize, Default)]
pub struct SelectionMap {
    pub map: HashMap<RowIndex, SelectionRange>,
    pub maybe_previous_direction: Option<CaretMovementDirection>,
}
pub type RowIndex = ChUnit;

#[test]
fn test_selection_map_direction_change() {
    use crate::selection_map_impl::DirectionChangeResult;

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

pub mod selection_map_impl {
    use std::fmt::{Debug, Display};

    use crossterm::style::StyledContent;

    use super::*;
    use crate::{DeleteSelectionWith, EditorBuffer};

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
            let indices = self.get_indices();
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
            let row_indices = self.get_indices();

            for row_index in row_indices {
                if let Some(selection_range) = self.map.get(&row_index) {
                    if let Some(line) = lines.get(ch!(@to_usize row_index)) {
                        let selected_text = line.clip_to_range(*selection_range);
                        it.insert(row_index, selected_text);
                    }
                }
            }

            it
        }

        pub fn get_indices(&self) -> Vec<RowIndex> {
            let row_indices = {
                let mut key_vec: Vec<ChUnit> = self.map.keys().copied().collect();
                key_vec.sort();
                key_vec
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

        pub fn remove_previous_direction(&mut self) {
            self.maybe_previous_direction = None;
        }

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

    #[derive(Clone, PartialEq, Serialize, Deserialize, GetSize, Copy, Debug)]
    pub enum RowLocationInSelectionMap {
        Overflow,
        Contained,
    }

    // Formatter for Debug and Display.
    mod debug_display {
        use super::*;

        impl SelectionMap {
            pub fn to_formatted_string(&self) -> StyledContent<String> {
                let selection_map_str = self.to_unformatted_string();
                if selection_map_str.contains("None") {
                    selection_map_str.white().on_dark_grey()
                } else {
                    selection_map_str.green().on_dark_grey()
                }
            }

            pub fn to_unformatted_string(&self) -> String {
                let mut vec_output = self
                    .map
                    .iter()
                    .map(|(row_index, selected_range)| {
                        format!(
                            "✂️ ┆row: {0} => start: {1}, end: {2}┆",
                            /* 0 */ row_index,
                            /* 1 */ selected_range.start_display_col_index,
                            /* 2 */ selected_range.end_display_col_index
                        )
                    })
                    .collect::<Vec<String>>();

                if vec_output.is_empty() {
                    vec_output.push("✂️ ┆--empty--┆".to_string());
                }

                vec_output
                    .push(format!("🧭 prev_dir: {:?}", self.maybe_previous_direction,));

                vec_output.join("\n    ")
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
}

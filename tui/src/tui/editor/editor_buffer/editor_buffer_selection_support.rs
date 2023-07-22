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

use std::{cmp, collections::HashMap};

use crossterm::style::Stylize;
use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::*;

/// Key is the row index, value is the selected range in that line (display col index
/// range).
///
/// Note that both column indices are [Scroll adjusted](CaretKind::ScrollAdjusted) and
/// not [raw](CaretKind::Raw)).
#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize, Default)]
pub struct SelectionMap {
    map: HashMap<RowIndex, SelectionRange>,
    maybe_previous_direction: Option<CaretMovementDirection>,
}
pub type RowIndex = ChUnit;

#[test]
fn test_direction_change() {
    use crate::editor_buffer_selection_support::selection_map_impl::DirectionChangeResult;

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

mod selection_map_impl {
    use std::fmt::{Debug, Display};

    use crossterm::style::StyledContent;

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DirectionChangeResult {
        DirectionHasChanged,
        DirectionIsTheSame,
    }

    // Functionality.
    impl SelectionMap {
        pub fn is_empty(&self) -> bool { self.map.is_empty() }

        pub fn clear(&mut self) { self.map.clear(); }

        pub fn iter(&self) -> impl Iterator<Item = (&RowIndex, &SelectionRange)> {
            self.map.iter()
        }

        pub fn get(&self, row_index: RowIndex) -> Option<&SelectionRange> {
            self.map.get(&row_index)
        }

        pub fn has_caret_movement_direction_changed(
            &self,
            current_direction: CaretMovementDirection,
        ) -> DirectionChangeResult {
            if self.maybe_previous_direction == Some(current_direction) {
                DirectionChangeResult::DirectionIsTheSame
            } else {
                DirectionChangeResult::DirectionHasChanged
            }
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
                let selection_map_str = {
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

                    vec_output.push(format!(
                        "🧭 prev_dir: {:?}",
                        self.maybe_previous_direction,
                    ));

                    let vec_output_str = vec_output.join(", ");

                    if vec_output_str.is_empty() {
                        "None".to_string()
                    } else {
                        vec_output_str
                    }
                };
                selection_map_str
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

pub struct EditorBufferApi;
impl EditorBufferApi {
    pub fn handle_selection_single_line_caret_movement(
        editor_buffer: &mut EditorBuffer,
        row_index: ChUnit,
        previous_caret_display_col_index: ChUnit,
        current_caret_display_col_index: ChUnit,
    ) {
        let previous = previous_caret_display_col_index;
        let current = current_caret_display_col_index;

        // Get the range for the row index. If it doesn't exist, create one & return early.
        let range = {
            let Some(range) = editor_buffer.get_selection_map().get(row_index) else {
                let new_range = SelectionRange {
                    start_display_col_index: cmp::min(previous, current),
                    end_display_col_index: cmp::max(previous, current),
                    };

                    let (_, _, _, selection_map) = editor_buffer.get_mut();
                selection_map.insert(
                    row_index,
                    new_range,
                    SelectionRange::caret_movement_direction_left_right(
                        previous, current,
                    ),
                );

                call_if_true!(
                    DEBUG_TUI_COPY_PASTE,
                    log_debug(format!("\n🍕🍕🍕 new selection: \n\t{}", new_range))
                );

                return;
            };
            *range // Copy & return it.
        };

        // Destructure range for easier access.
        let SelectionRange {
            start_display_col_index: range_start,
            end_display_col_index: range_end,
        } = range;

        call_if_true!(
            DEBUG_TUI_COPY_PASTE,
            log_debug(format!(
                "\n🍕🍕🍕 {0}:\n\t{1}: {2}, {3}: {4}\n\t{5}: {6}, {7}: {8}\n\t{9}: {10}, {11}: {12}, {13}: {14}",
                /* 0 */ "modify_existing_range_at_row_index",
                /* 1 */ "range_start",
                /* 2 */ range_start,
                /* 3 */ "range_end",
                /* 4 */ range_end,
                /* 5 */ "previous",
                /* 6 */ previous,
                /* 7 */ "current",
                /* 8 */ current,
                /* 9 */ "previous",
                /* 10 */ format!("{:?}", range.locate(previous)).black().on_dark_yellow(),
                /* 11 */ "current",
                /* 12 */ format!("{:?}", range.locate(current)).black().on_dark_cyan(),
                /* 13 */ "direction",
                /* 14 */ format!("{:?}", SelectionRange::caret_movement_direction_left_right(previous, current)).black().on_dark_green(),
        )));

        // Handle the movement of the caret and apply the appropriate changes to the range.
        match (
            range.locate(previous),
            range.locate(current),
            SelectionRange::caret_movement_direction_left_right(previous, current),
        ) {
            // Left + Shrink range end.
            (
                /* previous_caret */ CaretLocationInRange::Overflow,
                /* current_caret */ CaretLocationInRange::Contained,
                CaretMovementDirection::Left,
            ) => {
                let delta = previous - current;
                let new_range = range.shrink_end_by(delta);
                let (_, _, _, selection_map) = editor_buffer.get_mut();
                selection_map.insert(
                    row_index,
                    new_range,
                    SelectionRange::caret_movement_direction_left_right(
                        previous, current,
                    ),
                );
            }

            // Left + Grow range start.
            (
                /* previous_caret */ CaretLocationInRange::Contained,
                /* current_caret */ CaretLocationInRange::Underflow,
                CaretMovementDirection::Left,
            ) => {
                let delta = range_start - current;
                let new_range = range.grow_start_by(delta);
                let (_, _, _, selection_map) = editor_buffer.get_mut();
                selection_map.insert(
                    row_index,
                    new_range,
                    SelectionRange::caret_movement_direction_left_right(
                        previous, current,
                    ),
                );
            }

            // Right + Grow range end.
            (
                /* previous_caret */ CaretLocationInRange::Overflow,
                /* current_caret */ CaretLocationInRange::Overflow,
                CaretMovementDirection::Right,
            ) => {
                let delta = current - range_end;
                let new_range = range.grow_end_by(delta);
                let (_, _, _, selection_map) = editor_buffer.get_mut();
                selection_map.insert(
                    row_index,
                    new_range,
                    SelectionRange::caret_movement_direction_left_right(
                        previous, current,
                    ),
                );
            }

            // Right + Shrink range start.
            (
                /* previous_caret */ CaretLocationInRange::Contained,
                /* current_caret */
                CaretLocationInRange::Contained | CaretLocationInRange::Overflow,
                CaretMovementDirection::Right,
            ) => {
                let delta = current - range_start;
                let new_range = range.shrink_start_by(delta);
                let (_, _, _, selection_map) = editor_buffer.get_mut();
                selection_map.insert(
                    row_index,
                    new_range,
                    SelectionRange::caret_movement_direction_left_right(
                        previous, current,
                    ),
                );
            }

            // Catch all.
            (_, _, _) => {}
        }

        // Remove any range that is empty after caret movement changes have been
        // incorporated. Ok to do this since empty lines are handled by
        // `handle_selection_multiline_caret_movement`.
        if let Some(range) = editor_buffer.get_selection_map().get(row_index) {
            if range.start_display_col_index == range.end_display_col_index {
                let (_, _, _, selection_map) = editor_buffer.get_mut();
                selection_map.remove(
                    row_index,
                    SelectionRange::caret_movement_direction_left_right(
                        previous, current,
                    ),
                );
            }
        }
    }

    // TODO: implement multiline caret movement & selection changes
    // DBG: turn these comments into docs
    /*
    Preconditions:
    ---
    1. Required: There has to be at least 2 rows
    2. Optional: There may be 1 or more rows in the middle

    Algorithm:
    ---
    1. Get the range for the row indices between the previous and current caret row_index
    2. If the range spans multiple lines in the middle of the range, then simply add selections
       for the entire length of those lines into selection_map
    3. The first and last lines of the range may have partial selections, so we need to
       calculate the start and end display col indices for those lines. The direction of caret
       movement also factors into this. The start and end col caret index is used to determine
       how much of the first line and last line should be selected.
    4. First and last depends on the vertical direction. The ordering of the middle lines also
       depends on this vertical direction
    */
    pub fn handle_selection_multiline_caret_movement(
        editor_buffer: &mut EditorBuffer,
        previous_caret_display_position: Position,
        current_caret_display_position: Position,
    ) {
        let current = current_caret_display_position;
        let previous = previous_caret_display_position;

        // Validate preconditions.
        let caret_vertical_direction = SelectionRange::caret_movement_direction_up_down(
            previous.row_index,
            current.row_index,
        );
        if let CaretMovementDirection::Overlap = caret_vertical_direction {
            // Invalid state: There must be >= 2 rows, otherwise early return.
            return;
        }

        // DBG: remove
        log_debug(format!(
            "\n📜📜📜 {0}\n\t{1}, {2}, {3}, {4}",
            /* 0 */
            "handle multiline caret movement"
                .to_string()
                .red()
                .on_white(),
            /* 1 */
            format!("previous: {}", previous).cyan().on_dark_grey(),
            /* 2 */
            format!("current: {}", current).magenta().on_dark_grey(),
            /* 3 */
            format!("{:?}", editor_buffer.get_selection_map())
                .magenta()
                .on_dark_grey(),
            /* 4 */
            format!("{:?}", caret_vertical_direction)
                .magenta()
                .on_dark_grey(),
        ));

        // TODO: test that this works with Shift + PageUp, Shift + PageDown
        // Handle middle rows ( >= 3 rows ) if any. Only happens w/ Shift + Page Down/Up.
        if let 2.. = current.row_index.abs_diff(*previous.row_index) {
            let mut from = ch!(cmp::min(previous.row_index, current.row_index));
            let mut to = ch!(cmp::max(previous.row_index, current.row_index));

            // Skip the first and last lines in the range (middle rows).
            from += 1;
            to -= 1;

            let (lines, _, _, selection_map) = editor_buffer.get_mut();

            for row_index in from..to {
                let maybe_line = lines.get(ch!(@to_usize row_index));
                if let Some(line) = maybe_line {
                    // FIXME: handle empty line selection
                    let line_display_width = line.display_width;
                    if line_display_width > ch!(0) {
                        selection_map.insert(
                            row_index,
                            SelectionRange {
                                start_display_col_index: ch!(0),
                                end_display_col_index: line_display_width + 1,
                            },
                                caret_vertical_direction,
                        );
                    } else {
                        selection_map.insert(
                            row_index,
                            SelectionRange {
                                start_display_col_index: ch!(0),
                                end_display_col_index: ch!(0),
                            },
                                caret_vertical_direction,
                        );
                    }
                }

                // DBG: remove
                log_debug(format!(
                    "\n🌈🌈🌈process middle line:\n\t{0}, {1}",
                    /* 0 */ row_index.to_string().magenta().on_white(),
                    /* 1 */
                    maybe_line
                        .unwrap_or(&US::from("invalid line index"))
                        .string
                        .clone()
                        .black()
                        .on_white(),
                ));
            }
        }

        // Handle first and last lines in the range.
        match caret_vertical_direction {
            // TODO: handle direction change from Up to Down
            CaretMovementDirection::Down => {
                let first = previous;
                let last = current;

                let first_row = match editor_buffer
                    .get_selection_map()
                    .get(first.row_index)
                {
                    // First row is in selection map.
                    Some(_) => {
                        let start = ch!(0);
                        let end = editor_buffer.get_line_display_width(first.row_index);
                        SelectionRange::new(start, end)
                    }
                    // First row is not in selection map.
                    None => {
                        let start = previous.col_index;
                        let end = editor_buffer.get_line_display_width(first.row_index);
                        SelectionRange::new(start, end)
                    }
                };

                let last_row = {
                    let start = ch!(0);
                    let end = current.col_index;
                    SelectionRange::new(start, end)
                };

                let (_, _, _, selection_map) = editor_buffer.get_mut();
                selection_map.insert(
                    first.row_index,
                    first_row,
                    SelectionRange::caret_movement_direction(previous, current),
                );
                selection_map.insert(
                    last.row_index,
                    last_row,
                    SelectionRange::caret_movement_direction(previous, current),
                );
            }
            // TODO: handle direction change from Down to Up

            // DBG: remove
            // previous: [col:2, row:2], current: [col:2, row:1],
            // ✂️ ┆row: 0 => start: 2, end: 61┆, ✂️ ┆row: 2 => start: 0, end: 2┆, ✂️ ┆row: 1 => start: 0, end: 61┆, Up
            CaretMovementDirection::Up => {
                let first = current; // DBG: row: 1
                let last = previous; // DBG: row: 2

                let last_row_selection_range_op: SelectionRangeOp = match editor_buffer
                    .get_selection_map()
                    .get(last.row_index)
                {
                    // Last row in selection map.
                    Some(_) => {
                        // TODO: direction change from Down to Up -> drop last row in map
                        let current_direction =
                            SelectionRange::caret_movement_direction(previous, current);

                        let direction_change_result = editor_buffer
                            .get_selection_map()
                            .has_caret_movement_direction_changed(current_direction);

                        // DBG: remove
                        log_debug(format!(
                            "\n🌴🌴🌴 {0}\n\t{1}, {2}, {3}",
                            /* 0 */
                            "handle multiline caret movement"
                                .to_string()
                                .red()
                                .on_white(),
                            /* 1 */
                            format!("current_direction: {:?}", current_direction)
                                .cyan()
                                .on_dark_grey(),
                            /* 2 */
                            format!(
                                "previous_direction: {:?}",
                                editor_buffer
                                    .get_selection_map()
                                    .maybe_previous_direction
                            )
                            .cyan()
                            .on_dark_grey(),
                            /* 3 */
                            format!(
                                "direction_change_result: {:?}",
                                direction_change_result
                            )
                            .yellow()
                            .on_dark_grey(),
                        ));

                        match direction_change_result {
                            selection_map_impl::DirectionChangeResult::DirectionHasChanged => {
                                SelectionRangeOp::Remove { row_index: last.row_index }
                            }

                        // BUG: if no direction change selection fills in last row if go
                        // down, down, down, up, up (fails)
                        // If the last_row is in selection_map, drop it
                        selection_map_impl::DirectionChangeResult::DirectionIsTheSame => {
                                let start = ch!(0);
                                let end =
                                    editor_buffer.get_line_display_width(last.row_index);
                                SelectionRangeOp::Insert {
                                    range: SelectionRange::new(start, end),
                                    row_index: last.row_index
                                }
                            }
                        }
                    }
                    // Last row not in selection map.
                    None => {
                        let start = ch!(0);
                        let end = current.col_index;
                        SelectionRangeOp::Insert {
                            range: SelectionRange::new(start, end),
                            row_index: last.row_index,
                        }
                    }
                };

                let first_row_selection_range_op: SelectionRangeOp =
                    match editor_buffer.get_selection_map().get(first.row_index) {
                        // first row in selection map.
                        Some(_) => {
                            let start = ch!(0);
                            let end = current.col_index;
                            SelectionRangeOp::Insert {
                                range: SelectionRange::new(start, end),
                                row_index: first.row_index,
                            }
                        }
                        // first row not in selection map.
                        None => {
                            let start = current.col_index;
                            let end =
                                editor_buffer.get_line_display_width(first.row_index);
                            SelectionRangeOp::Insert {
                                range: SelectionRange::new(start, end),
                                row_index: first.row_index,
                            }
                        }
                    };

                // Actually modify selection_map given the SelectionRangeOp for first &
                // last row.
                let (_, _, _, selection_map) = editor_buffer.get_mut();
                let direction =
                    SelectionRange::caret_movement_direction(previous, current);

                let range_op_vec =
                    vec![first_row_selection_range_op, last_row_selection_range_op];
                for range_op in range_op_vec {
                    match range_op {
                        SelectionRangeOp::Insert { range, row_index } => {
                            selection_map.insert(row_index, range, direction);
                        }
                        SelectionRangeOp::Remove { row_index } => {
                            selection_map.remove(row_index, direction)
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Special case to handle the situation where up / down movement has resulted in the top
    /// or bottom of the document to be hit, so that further movement up / down isn't possible,
    /// but the caret might jump left or right.
    pub fn handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document(
        editor_buffer: &mut EditorBuffer,
        previous_caret_display_position: Position,
        current_caret_display_position: Position,
    ) {
        let current = current_caret_display_position;
        let previous = previous_caret_display_position;

        // Precondition check: Only run if the row previous and current row indices are same.
        if current.row_index != previous.row_index {
            return;
        }

        let row_index = current.row_index; // Same as previous.row_index.
        let (lines, _, _, selection_map) = editor_buffer.get_mut();

        // DBG: remove
        log_debug(format!(
            "\n📜🔼🔽 {0}\n\t{1}, {2}, {3}, {4}",
            /* 0 */
            "handle multiline caret movement"
                .to_string()
                .red()
                .on_white(),
            /* 1 */
            format!("previous: {}", previous).cyan().on_dark_grey(),
            /* 2 */
            format!("current: {}", current).yellow().on_dark_grey(),
            /* 3 */
            format!("row_index: {}", row_index).green().on_dark_grey(),
            /* 4 */
            format!("{:?}", selection_map).magenta().on_dark_grey(),
        ));

        match current.col_index.cmp(&previous.col_index) {
            cmp::Ordering::Less => {
                match selection_map.get(row_index) {
                    // Extend range to left (caret moved up and hit the top).
                    Some(range) => {
                        let start = ch!(0);
                        let end = range.end_display_col_index;
                        selection_map.insert(
                            row_index,
                            SelectionRange {
                                start_display_col_index: start,
                                end_display_col_index: end,
                            },
                            SelectionRange::caret_movement_direction(previous, current),
                        );
                    }
                    // Create range to left (caret moved up and hit the top).
                    None => {
                        let start = ch!(0);
                        let end = previous.col_index;
                        selection_map.insert(
                            row_index,
                            SelectionRange {
                                start_display_col_index: start,
                                end_display_col_index: end,
                            },
                            SelectionRange::caret_movement_direction(previous, current),
                        );
                    }
                }
            }
            cmp::Ordering::Greater => match selection_map.get(row_index) {
                // Extend range to right (caret moved down and hit bottom).
                Some(range) => {
                    if let Some(line) = lines.get(ch!(@to_usize row_index)) {
                        let start = range.start_display_col_index;
                        let end = line.display_width;
                        selection_map.insert(
                            row_index,
                            SelectionRange {
                                start_display_col_index: start,
                                end_display_col_index: end,
                            },
                            SelectionRange::caret_movement_direction(previous, current),
                        );
                    }
                }
                // Create range to right (caret moved down and hit bottom).
                None => {
                    let start = previous.col_index;
                    let end = current.col_index;
                    selection_map.insert(
                        row_index,
                        SelectionRange {
                            start_display_col_index: start,
                            end_display_col_index: end,
                        },
                        SelectionRange::caret_movement_direction(previous, current),
                    );
                }
            },
            _ => {}
        }
    }
}

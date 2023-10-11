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

use std::cmp;

use crossterm::style::Stylize;
use r3bl_rs_utils_core::*;

use self::selection_map_impl::{DirectionChangeResult, RowLocationInSelectionMap::*};
use crate::*;

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
                /* 10 */ format!("{:?}", range.locate_column(previous)).black().on_dark_yellow(),
                /* 11 */ "current",
                /* 12 */ format!("{:?}", range.locate_column(current)).black().on_dark_cyan(),
                /* 13 */ "direction",
                /* 14 */
                format!(
                    "{:?}",
                    SelectionRange::caret_movement_direction_left_right(previous, current)
                )
                .black()
                .on_dark_green(),
            ))
        );

        // BM: for reference, algo for left, right selection
        // Handle the movement of the caret and apply the appropriate changes to the range.
        match (
            range.locate_column(previous),
            range.locate_column(current),
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

    // BM: implement multiline selection changes (up/down, and later page up/page down)
    /// Precondition: there has to be at least 2 rows.
    fn handle_two_lines(
        editor_buffer: &mut EditorBuffer,
        previous_caret_display_position: Position,
        current_caret_display_position: Position,
    ) {
        let current = current_caret_display_position;
        let previous = previous_caret_display_position;

        // Validate preconditions.
        let caret_vertical_movement_direction =
            SelectionRange::caret_movement_direction_up_down(
                previous.row_index,
                current.row_index,
            );
        if let CaretMovementDirection::Overlap = caret_vertical_movement_direction {
            // Invalid state: There must be >= 2 rows, otherwise early return.
            return;
        }

        let (_lines, _, _, selection_map) = editor_buffer.get_mut();
        let locate_previous_row_index = selection_map.locate_row(previous.row_index);
        let locate_current_row_index = selection_map.locate_row(current.row_index);
        let has_caret_movement_direction_changed = selection_map
            .has_caret_movement_direction_changed(caret_vertical_movement_direction);

        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            log_debug(format!(
                "\n📜📜📜 {0}\n\t{1}, {2}\n\t{3}\n\t{4}\n\t{5}\n\t{6}\n\t{7}",
                /* 0: heading */
                "handle multiline caret movement"
                    .to_string()
                    .red()
                    .on_white(),
                /* 1: previous */
                format!("👈 previous: {}", previous).cyan().on_dark_grey(),
                /* 2: current */
                format!("👉 current: {}", current).magenta().on_dark_grey(),
                /* 3: selection_map */
                format!("{:?}", editor_buffer.get_selection_map())
                    .magenta()
                    .on_dark_grey(),
                /* 4: locate_previous_row_index */
                format!("locate_previous_row_index: {:?}", locate_previous_row_index)
                    .cyan()
                    .on_dark_grey(),
                /* 5: locate_current_row_index, */
                format!("locate_current_row_index: {:?}", locate_current_row_index,)
                    .magenta()
                    .on_dark_grey(),
                /* 6: caret_vertical_movement_direction, */
                format!(
                    "caret_vertical_movement_direction: {:?}",
                    caret_vertical_movement_direction,
                )
                .green()
                .on_dark_grey(),
                /* 7: has_caret_movement_direction_changed, */
                format!(
                    "has_caret_movement_direction_changed: {:?}",
                    has_caret_movement_direction_changed,
                )
                .yellow()
                .on_dark_grey(),
            ));
        });

        match (
            locate_previous_row_index,
            locate_current_row_index,
            caret_vertical_movement_direction,
            has_caret_movement_direction_changed,
        ) {
            // DirectionIsTheSame: No selection, then Shift+Down.
            // DirectionHasChanged: No selection -> Shift+Down -> Shift+Up -> Shift+Down.
            (
                /* previous_caret */ Overflow,
                /* current_caret */ Overflow,
                CaretMovementDirection::Down,
                DirectionChangeResult::DirectionIsTheSame
                | DirectionChangeResult::DirectionHasChanged,
            ) => multiline_select_helpers::start_select_down(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionHasChanged: No selection -> Shift+Up -> Shift+Down -> Shift+Up.
            (
                /* previous_caret */ Overflow,
                /* current_caret */ Overflow,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionIsTheSame
                | DirectionChangeResult::DirectionHasChanged,
            ) => multiline_select_helpers::start_select_up(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionIsTheSame: Previous selection with Shift+Down, then Shift+Down.
            // DirectionHasChanged: No selection -> Shift+Left/Right -> Shift+Down.
            (
                /* previous_caret */ Contained,
                /* current_caret */ Overflow,
                CaretMovementDirection::Down,
                DirectionChangeResult::DirectionIsTheSame
                | DirectionChangeResult::DirectionHasChanged,
            ) => multiline_select_helpers::continue_select_down(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // Position caret below empty line, Shift+Up, Shift+Up, Shift+Up, Shift+Down.
            (
                /* previous_caret */ Overflow,
                /* current_caret */ Contained,
                CaretMovementDirection::Down,
                DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_select_down(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionIsTheSame: Previous selection with Shift+Up, then Shift+Up.
            // DirectionHasChanged: // No selection -> Shift+Left/Right -> Shift+Up.
            (
                /* previous_caret */ Contained,
                /* current_caret */ Overflow,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionIsTheSame
                | DirectionChangeResult::DirectionHasChanged,
            ) => multiline_select_helpers::continue_select_up(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // Position caret above empty line, Shift+Down, Shift+Down, Shift+Down, Shift+Up.
            (
                /* previous_caret */ Overflow,
                /* current_caret */ Contained,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_select_up(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionHasChanged: Previous selection with Shift+Down, then Shift+Up.
            // DirectionIsTheSame: Previous selection with Shift+Down, then Shift+Up, then Shift+Up.
            (
                /* previous_caret */ Contained,
                /* current_caret */ Contained,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionHasChanged
                | DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_direction_change_select_up(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionHasChanged: Previous selection with Shift+Up, then Shift+Up, then Shift+Down.
            // DirectionIsTheSame: Previous selection with Shift+Up, then Shift+Down, then Shift+Down.
            (
                /* previous_caret */ Contained,
                /* current_caret */ Contained,
                CaretMovementDirection::Down,
                DirectionChangeResult::DirectionHasChanged
                | DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_direction_change_select_down(
                previous,
                current,
                editor_buffer,
                caret_vertical_movement_direction,
            ),
            // Catchall.
            _ => {
                call_if_true!(
                    DEBUG_TUI_COPY_PASTE,
                    log_debug(format!(
                        "\n📜📜📜⚾⚾⚾ {0}",
                        /* 0: heading */
                        "handle multiline caret movement Catchall"
                            .to_string()
                            .bold()
                            .yellow()
                            .on_dark_green(),
                    ))
                );
            }
        }
    }

    /// Precondition: there has to be at least 2 rows.
    pub fn handle_selection_multiline_caret_movement(
        editor_buffer: &mut EditorBuffer,
        previous_caret_display_position: Position,
        current_caret_display_position: Position,
    ) {
        let current = current_caret_display_position;
        let previous = previous_caret_display_position;

        // Validate preconditions.
        let caret_vertical_movement_direction =
            SelectionRange::caret_movement_direction_up_down(
                previous.row_index,
                current.row_index,
            );
        if let CaretMovementDirection::Overlap = caret_vertical_movement_direction {
            // Invalid state: There must be >= 2 rows, otherwise early return.
            return;
        }

        // For the rows between previous and current caret, call
        // handle_selection_single_line_caret_movement() on each row.
        match caret_vertical_movement_direction {
            // ```text
            // R ┌──────────┐
            // 0 ▸C         │ ← Current caret
            // 1 │P         │ ← Previous caret
            //   └▴─────────┘
            //   C0123456789
            // ```
            CaretMovementDirection::Up => {
                for row_index in current.row_index.value..previous.row_index.value {
                    let current_row_index = row_index;
                    let previous_row_index = row_index + 1;
                    Self::handle_two_lines(
                        editor_buffer,
                        position!(col_index: previous.col_index, row_index: previous_row_index),
                        position!(col_index: current.col_index, row_index: current_row_index),
                    );
                }
            }
            // ```text
            // R ┌──────────┐
            // 0 │P         │ ← Previous caret
            // 1 ▸C         │ ← Current caret
            //   └▴─────────┘
            //   C0123456789
            // ```
            CaretMovementDirection::Down => {
                for row_index in previous.row_index.value..current.row_index.value {
                    let previous_row_index = row_index;
                    let current_row_index = row_index + 1;
                    Self::handle_two_lines(
                        editor_buffer,
                        position!(col_index: previous.col_index, row_index: previous_row_index),
                        position!(col_index: current.col_index, row_index: current_row_index),
                    );
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

        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            log_debug(format!(
                "\n📜🔼🔽 {0}\n\t{1}, {2}, {3}, {4}",
                /* 0 */
                "handle multiline caret movement hit top or bottom of document"
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
            ))
        });

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

mod multiline_select_helpers {
    use super::*;

    /// No existing selection, up, no direction change:
    /// - Add first row selection range.
    /// - Add last row selection range.
    pub fn start_select_down(
        previous: Position,
        current: Position,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = previous;
        let last = current;

        add_first_and_last_row(
            first,
            last,
            editor_buffer,
            caret_vertical_movement_direction,
        );
    }

    /// No existing selection, up, no direction change:
    /// - Add first row selection range.
    /// - Add last row selection range.
    pub fn start_select_up(
        previous: Position,
        current: Position,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = current;
        let last = previous;

        add_first_and_last_row(
            first,
            last,
            editor_buffer,
            caret_vertical_movement_direction,
        );
    }

    fn add_first_and_last_row(
        first: Position,
        last: Position,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first_row_range = {
            let start_col = first.col_index;
            let end_col = editor_buffer.get_line_display_width(first.row_index);
            SelectionRange::new(start_col, end_col)
        };

        let last_row_range = {
            let start_col = ch!(0);
            let end_col = last.col_index;
            SelectionRange::new(start_col, end_col)
        };

        let (_, _, _, selection_map) = editor_buffer.get_mut();
        selection_map.insert(
            first.row_index,
            first_row_range,
            caret_vertical_movement_direction,
        );
        selection_map.insert(
            last.row_index,
            last_row_range,
            caret_vertical_movement_direction,
        );
    }

    /// Pre-existing selection, down, no direction change:
    /// - Add last row selection range.
    /// - Modify first row selection range.
    pub fn continue_select_down(
        previous: Position,
        current: Position,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = previous;
        let last = current;

        let first_line_width = editor_buffer.get_line_display_width(first.row_index);

        // Mutably borrow the selection map.
        let (_, _, _, selection_map) = editor_buffer.get_mut();

        // Extend the existing range (in selection map) for the first row to end of line.
        if let Some(first_row_range) = selection_map.get(first.row_index) {
            let start_col = first_row_range.start_display_col_index;
            let end_col = first_line_width;
            let new_first_row_range = SelectionRange {
                start_display_col_index: start_col,
                end_display_col_index: end_col,
            };
            selection_map.insert(
                first.row_index,
                new_first_row_range,
                caret_vertical_movement_direction,
            );
        }

        // Add the new last row range to selection map.
        let last_row_range = {
            let start_col = ch!(0);
            let end_col = last.col_index;
            SelectionRange::new(start_col, end_col)
        };
        selection_map.insert(
            last.row_index,
            last_row_range,
            caret_vertical_movement_direction,
        );
    }

    /// Pre-existing selection, up, no direction change:
    /// - Add first row selection range.
    /// - Modify last row selection range.
    pub fn continue_select_up(
        previous: Position,
        current: Position,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = current;
        let last = previous;

        let first_line_width = editor_buffer.get_line_display_width(first.row_index);

        // Mutably borrow the selection map.
        let (_, _, _, selection_map) = editor_buffer.get_mut();

        // FIRST ROW.
        if let Some(first_row_range) = selection_map.get(first.row_index) {
            // Extend the existing range (in selection map) for the first row to end of line.
            let updated_first_row_range = SelectionRange {
                start_display_col_index: first_row_range.start_display_col_index,
                end_display_col_index: first_line_width,
            };
            selection_map.insert(
                first.row_index,
                updated_first_row_range,
                caret_vertical_movement_direction,
            );
        } else {
            // Add the new first row range to selection map.
            let new_first_row_range = {
                let start_col = first.col_index;
                let end_col = first_line_width;
                SelectionRange::new(start_col, end_col)
            };
            selection_map.insert(
                first.row_index,
                new_first_row_range,
                caret_vertical_movement_direction,
            );
        }

        // LAST ROW.
        if let Some(last_row_range) = selection_map.get(last.row_index) {
            // Extend the existing range (in selection map) for the last row to start of line.
            let start_col = ch!(0);
            let end_col = last_row_range.end_display_col_index;
            let updated_last_row_range = SelectionRange {
                start_display_col_index: start_col,
                end_display_col_index: end_col,
            };
            selection_map.insert(
                last.row_index,
                updated_last_row_range,
                caret_vertical_movement_direction,
            );
        } else {
            // Add the new last row range to selection map.
            let new_last_row_range = {
                let start_col = ch!(0);
                let end_col = last.col_index;
                SelectionRange::new(start_col, end_col)
            };
            selection_map.insert(
                last.row_index,
                new_last_row_range,
                caret_vertical_movement_direction,
            );
        }
    }

    /// Pre-existing selection, up, direction change:
    /// - Drop the last row selection range.
    /// - Modify first row selection range.
    pub fn continue_direction_change_select_up(
        previous: Position,
        current: Position,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = current;
        let last = previous;

        // Mutably borrow the selection map.
        let (_, _, _, selection_map) = editor_buffer.get_mut();

        // Drop the existing range (in selection map) for the last row.
        if selection_map.get(last.row_index).is_some() {
            selection_map.remove(last.row_index, caret_vertical_movement_direction);
        }

        // Change the existing range (in selection map) for the first row.
        if let Some(first_row_range) = selection_map.get(first.row_index) {
            let lhs = first_row_range.start_display_col_index;
            let rhs = first.col_index;
            match lhs.cmp(&rhs) {
                cmp::Ordering::Equal => {
                    selection_map
                        .remove(first.row_index, caret_vertical_movement_direction);
                }
                cmp::Ordering::Less | cmp::Ordering::Greater => {
                    selection_map.insert(
                        first.row_index,
                        SelectionRange {
                            start_display_col_index: lhs.min(rhs),
                            end_display_col_index: lhs.max(rhs),
                        },
                        caret_vertical_movement_direction,
                    );
                }
            }
        }
    }

    /// Pre-existing selection, up, direction change:
    /// - Drop the first row selection range.
    /// - Modify last row selection range.
    pub fn continue_direction_change_select_down(
        previous: Position,
        current: Position,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = previous;
        let last = current;

        // Mutably borrow the selection map.
        let (_, _, _, selection_map) = editor_buffer.get_mut();

        // Drop the existing range (in selection map) for the first row.
        if selection_map.get(first.row_index).is_some() {
            selection_map.remove(first.row_index, caret_vertical_movement_direction);
        }

        // Change the existing range (in selection map) for the last row.
        if let Some(last_row_range) = selection_map.get(last.row_index) {
            let lhs = last.col_index;
            let rhs = last_row_range.end_display_col_index;
            let row_index = last.row_index;
            match lhs.cmp(&rhs) {
                cmp::Ordering::Equal => {
                    selection_map.remove(row_index, caret_vertical_movement_direction)
                }
                cmp::Ordering::Greater | cmp::Ordering::Less => selection_map.insert(
                    row_index,
                    SelectionRange {
                        start_display_col_index: rhs.min(lhs),
                        end_display_col_index: rhs.max(lhs),
                    },
                    caret_vertical_movement_direction,
                ),
            }
        }
    }
}

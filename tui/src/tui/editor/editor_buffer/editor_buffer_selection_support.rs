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
use r3bl_rs_utils_core::*;

use crate::*;

/// Key is the row index, value is the selected range in that line (display col index
/// range).
///
/// Note that both column indices are [Scroll adjusted](CaretKind::ScrollAdjusted) and
/// not [raw](CaretKind::Raw)).
pub type SelectionMap = HashMap<RowIndex, SelectionRange>;
pub type RowIndex = ChUnit;

pub struct EditorBufferApi;
impl EditorBufferApi {
    pub fn update_selection_based_on_caret_movement(
        editor_buffer: &mut EditorBuffer,
        caret_previous: Position,
        caret_current: Position,
    ) {
        match caret_previous.row_index.cmp(&caret_current.row_index) {
            cmp::Ordering::Equal => {
                create_new_or_modify_existing_range_at_row_index(
                    editor_buffer,
                    caret_previous.row_index, // Same as caret_current.row_index.
                    caret_previous.col_index,
                    caret_current.col_index,
                )
            }
            _ => {
                Self::handle_multiline_caret_movement();
            }
        }
    }

    // TODO: implement multiline caret movement & selection changes
    fn handle_multiline_caret_movement() {
        // DBG: remove
        log_debug("\n🍕🍕🍕 multiline caret movement not implemented yet".to_string());
    }
}

fn create_new_or_modify_existing_range_at_row_index(
    editor_buffer: &mut EditorBuffer,
    row_index: ChUnit,
    previous_caret: ChUnit,
    current_caret: ChUnit,
) {
    // Get the range for the row index. If it doesn't exist, create one & return early.
    let range = {
        let Some(range) = editor_buffer.get_selection_map().get(&row_index)
            else {
                let new_range = SelectionRange {
                    start_display_col_index: cmp::min(previous_caret, current_caret),
                    end_display_col_index: cmp::max(previous_caret, current_caret),
                };

                editor_buffer
                    .get_selection_map_mut()
                    .insert(row_index, new_range);

                call_if_true!(
                    DEBUG_TUI_COPY_PASTE,
                    log_debug(format!("\n🍕🍕🍕 new selection: \n\t{}", new_range))
                );

                return
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
            /* 6 */ previous_caret,
            /* 7 */ "current",
            /* 8 */ current_caret,
            /* 9 */ "previous",
            /* 10 */ format!("{:?}", range.locate(previous_caret)).black().on_dark_yellow(),
            /* 11 */ "current",
            /* 12 */ format!("{:?}", range.locate(current_caret)).black().on_dark_cyan(),
            /* 13 */ "direction",
            /* 14 */ format!("{:?}", SelectionRange::caret_movement_direction(previous_caret, current_caret)).black().on_dark_green(),
    )));

    // Handle the movement of the caret and apply the appropriate changes to the range.
    match (
        range.locate(previous_caret),
        range.locate(current_caret),
        SelectionRange::caret_movement_direction(previous_caret, current_caret),
    ) {
        // Left + Shrink range end.
        (
            /* previous_caret */ CaretLocationInRange::Overflow,
            /* current_caret */ CaretLocationInRange::Contained,
            CaretMovementDirection::Left,
        ) => {
            let delta = previous_caret - current_caret;
            let new_range = range.shrink_end_by(delta);
            editor_buffer
                .get_selection_map_mut()
                .insert(row_index, new_range);
        }

        // Left + Grow range start.
        (
            /* previous_caret */ CaretLocationInRange::Contained,
            /* current_caret */ CaretLocationInRange::Underflow,
            CaretMovementDirection::Left,
        ) => {
            let delta = range_start - current_caret;
            let new_range = range.grow_start_by(delta);
            editor_buffer
                .get_selection_map_mut()
                .insert(row_index, new_range);
        }

        // Right + Grow range end.
        (
            /* previous_caret */ CaretLocationInRange::Overflow,
            /* current_caret */ CaretLocationInRange::Overflow,
            CaretMovementDirection::Right,
        ) => {
            let delta = current_caret - range_end;
            let new_range = range.grow_end_by(delta);
            editor_buffer
                .get_selection_map_mut()
                .insert(row_index, new_range);
        }

        // Right + Shrink range start.
        (
            /* previous_caret */ CaretLocationInRange::Contained,
            /* current_caret */
            CaretLocationInRange::Contained | CaretLocationInRange::Overflow,
            CaretMovementDirection::Right,
        ) => {
            let delta = current_caret - range_start;
            let new_range = range.shrink_start_by(delta);
            editor_buffer
                .get_selection_map_mut()
                .insert(row_index, new_range);
        }

        // Catch all.
        (_, _, _) => {}
    }

    // Remove any range that is empty after caret movement changes have been incoroprated.
    if let Some(range) = editor_buffer.get_selection_map().get(&row_index) {
        if range.start_display_col_index == range.end_display_col_index {
            editor_buffer.get_selection_map_mut().remove(&row_index);
        }
    }
}

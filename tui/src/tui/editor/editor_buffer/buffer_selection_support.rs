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

use std::cmp;

use crossterm::style::Stylize;
use r3bl_core::{call_if_true,
                caret_scr_adj,
                col,
                height,
                row,
                string_storage,
                usize,
                width,
                CaretLocationInRange,
                CaretMovementDirection,
                CaretScrAdj,
                ColIndex,
                RowIndex,
                SelectionRange};

use super::{selection_list::RowLocationInSelectionList, EditorBuffer};
use crate::{caret_scroll_index, DirectionChangeResult, DEBUG_TUI_COPY_PASTE};

// REVIEW: [ ] this is public
pub fn handle_selection_single_line_caret_movement(
    buffer: &mut EditorBuffer,
    row_index: RowIndex,
    previous_caret_display_col_index: ColIndex,
    current_caret_display_col_index: ColIndex,
) {
    let previous = previous_caret_display_col_index;
    let current = current_caret_display_col_index;

    // Get the range for the row index. If it doesn't exist, create one & return early.
    let range = {
        let Some(range) = buffer.get_selection_list().get(row_index) else {
            let new_range = SelectionRange {
                start_disp_col_idx_scr_adj: cmp::min(previous, current),
                end_disp_col_idx_scr_adj: cmp::max(previous, current),
            };

            let buffer_mut = buffer.get_mut(width(0) + height(0));

            buffer_mut.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(previous, current),
            );

            call_if_true!(DEBUG_TUI_COPY_PASTE, {
                tracing::debug!("\n🍕🍕🍕 new selection: \n\t{it:?}", it = new_range);
            });

            return;
        };
        range // Copy & return it.
    };

    // Destructure range for easier access.
    let SelectionRange {
        start_disp_col_idx_scr_adj: range_start,
        end_disp_col_idx_scr_adj: range_end,
    } = range;

    call_if_true!(DEBUG_TUI_COPY_PASTE, {
        tracing::debug!(
                    "\n🍕🍕🍕 {a}:\n\t{b}: {c:?}, {d}: {e:?}\n\t{f}: {g:?}, {h}: {i:?}\n\t{j}: {k}, {l}: {m}, {n}: {o}",
                    a = "modify_existing_range_at_row_index",
                    b = "range_start",
                    c = range_start,
                    d = "range_end",
                    e = range_end,
                    f = "previous",
                    g = previous,
                    h = "current",
                    i = current,
                    j = "previous",
                    k = format!("{:?}", range.locate_column(previous)).black().on_dark_yellow(),
                    l = "current",
                    m = format!("{:?}", range.locate_column(current)).black().on_dark_cyan(),
                    n = "direction",
                    o =
                    format!(
                        "{:?}",
                        SelectionRange::caret_movement_direction_left_right(previous, current)
                    )
                    .black()
                    .on_dark_green(),
                )
    });

    // XMARK: For reference, algo for left, right selection
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
            let new_range = range.shrink_end_by(width(*delta));

            let buffer_mut = buffer.get_mut(width(0) + height(0));
            buffer_mut.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(previous, current),
            );
        }

        // Left + Grow range start.
        (
            /* previous_caret */ CaretLocationInRange::Contained,
            /* current_caret */ CaretLocationInRange::Underflow,
            CaretMovementDirection::Left,
        ) => {
            let delta = range_start - current;
            let new_range = range.grow_start_by(width(*delta));

            let buffer_mut = buffer.get_mut(width(0) + height(0));
            buffer_mut.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(previous, current),
            );
        }

        // Right + Grow range end.
        (
            /* previous_caret */ CaretLocationInRange::Overflow,
            /* current_caret */ CaretLocationInRange::Overflow,
            CaretMovementDirection::Right,
        ) => {
            let delta = current - range_end;
            let new_range = range.grow_end_by(width(*delta));

            let buffer_mut = buffer.get_mut(width(0) + height(0));
            buffer_mut.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(previous, current),
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
            let new_range = range.shrink_start_by(width(*delta));

            let buffer_mut = buffer.get_mut(width(0) + height(0));
            buffer_mut.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(previous, current),
            );
        }

        // Catch all.
        (_, _, _) => {}
    }

    // Remove any range that is empty after caret movement changes have been
    // incorporated. Ok to do this since empty lines are handled by
    // `handle_selection_multiline_caret_movement`.
    if let Some(range) = buffer.get_selection_list().get(row_index) {
        if range.start_disp_col_idx_scr_adj == range.end_disp_col_idx_scr_adj {
            let buffer_mut = buffer.get_mut(width(0) + height(0));
            buffer_mut.sel_list.remove(
                row_index,
                SelectionRange::caret_movement_direction_left_right(previous, current),
            );
        }
    }
}

/// Precondition: there has to be at least 2 rows.
// REVIEW: [ ] this is public
pub fn handle_selection_multiline_caret_movement(
    editor_buffer: &mut EditorBuffer,
    prev_caret_scr_adj: CaretScrAdj,
    curr_caret_scr_adj: CaretScrAdj,
) {
    let curr = curr_caret_scr_adj;
    let prev = prev_caret_scr_adj;

    // Validate preconditions.
    let caret_vertical_movement_direction =
        SelectionRange::caret_movement_direction_up_down(prev.row_index, curr.row_index);
    if let CaretMovementDirection::Overlap = caret_vertical_movement_direction {
        // Invalid state: There must be >= 2 rows, otherwise early return.
        return;
    }

    // For the rows between previous and current caret, call
    // handle_selection_single_line_caret_movement() on each row.
    match caret_vertical_movement_direction {
        // ```text
        // R ┌──────────┐
        // 0 ❱C         │ ← Current caret
        // 1 │P         │ ← Previous caret
        //   └⮬─────────┘
        //   C0123456789
        // ```
        CaretMovementDirection::Up => {
            for row_index in curr.row_index.value..prev.row_index.value {
                let previous_row_index = row(row_index + 1);
                let prev_pos = prev.col_index + previous_row_index;

                let current_row_index = row(row_index);
                let curr_pos = curr.col_index + current_row_index;

                multiline_select_helpers::handle_two_lines(
                    editor_buffer,
                    caret_scr_adj(prev_pos),
                    caret_scr_adj(curr_pos),
                );
            }
        }
        // ```text
        // R ┌──────────┐
        // 0 │P         │ ← Previous caret
        // 1 ❱C         │ ← Current caret
        //   └⮬─────────┘
        //   C0123456789
        // ```
        CaretMovementDirection::Down => {
            for row_index in prev.row_index.value..curr.row_index.value {
                let previous_row_index = row(row_index);
                let prev_pos = prev.col_index + previous_row_index;

                let current_row_index = row(row_index + 1);
                let curr_pos = curr.col_index + current_row_index;

                multiline_select_helpers::handle_two_lines(
                    editor_buffer,
                    caret_scr_adj(prev_pos),
                    caret_scr_adj(curr_pos),
                );
            }
        }
        _ => {}
    }
}

/// Special case to handle the situation where up / down movement has resulted in the top
/// or bottom of the document to be hit, so that further movement up / down isn't possible,
/// but the caret might jump left or right.
// REVIEW: [ ] this is public
pub fn handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document(
    editor_buffer: &mut EditorBuffer,
    previous_caret_display_position: CaretScrAdj,
    current_caret_display_position: CaretScrAdj,
) {
    let curr = current_caret_display_position;
    let prev = previous_caret_display_position;

    // Precondition check: Only run if the row previous and current row indices are same.
    if curr.row_index != prev.row_index {
        return;
    }

    let row_index = curr.row_index; // Same as previous.row_index.

    let buffer_mut = editor_buffer.get_mut(width(0) + height(0));

    call_if_true!(DEBUG_TUI_COPY_PASTE, {
        let message = "📜🔼🔽 handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document";
        let details = string_storage!(
            "\n{a}\n\t{b}, {c}, {d}, {e}",
            /* 0 */
            a = "handle multiline caret movement hit top or bottom of document"
                .to_string()
                .red()
                .on_white(),
            /* 1: previous */
            b = format!("previous: {:?}", prev).cyan().on_dark_grey(),
            /* 2: current */
            c = format!("current: {:?}", curr).yellow().on_dark_grey(),
            /* 3: row_index */
            d = format!("row_index: {:?}", row_index).green().on_dark_grey(),
            /* 4: selection_map */
            e = format!("{:?}", buffer_mut.sel_list)
                .magenta()
                .on_dark_grey(),
        );
        // % is Display, ? is Debug.
        tracing::debug! {
            message = %message,
            details = %details,
        }
    });

    match curr.col_index.cmp(&prev.col_index) {
        cmp::Ordering::Less => {
            match buffer_mut.sel_list.get(row_index) {
                // Extend range to left (caret moved up and hit the top).
                Some(range) => {
                    let start = col(0);
                    let end = range.end_disp_col_idx_scr_adj;
                    buffer_mut.sel_list.insert(
                        row_index,
                        SelectionRange {
                            start_disp_col_idx_scr_adj: start,
                            end_disp_col_idx_scr_adj: end,
                        },
                        SelectionRange::caret_movement_direction(prev, curr),
                    );
                }
                // Create range to left (caret moved up and hit the top).
                None => {
                    let start = col(0);
                    let end = prev.col_index;
                    buffer_mut.sel_list.insert(
                        row_index,
                        SelectionRange {
                            start_disp_col_idx_scr_adj: start,
                            end_disp_col_idx_scr_adj: end,
                        },
                        SelectionRange::caret_movement_direction(prev, curr),
                    );
                }
            }
        }
        cmp::Ordering::Greater => {
            match buffer_mut.sel_list.get(row_index) {
                // Extend range to right (caret moved down and hit bottom).
                Some(range) => {
                    if let Some(line_us) = buffer_mut.lines.get(usize(row_index)) {
                        let start = range.start_disp_col_idx_scr_adj;
                        // For selection, go one col index past the end of the line,
                        // since selection range is not inclusive of the end index.
                        let end = caret_scroll_index::scroll_col_index_for_width(
                            line_us.display_width,
                        );
                        buffer_mut.sel_list.insert(
                            row_index,
                            SelectionRange {
                                start_disp_col_idx_scr_adj: start,
                                end_disp_col_idx_scr_adj: end,
                            },
                            SelectionRange::caret_movement_direction(prev, curr),
                        );
                    }
                }
                // Create range to right (caret moved down and hit bottom).
                None => {
                    let start = prev.col_index;
                    let end = curr.col_index;
                    buffer_mut.sel_list.insert(
                        row_index,
                        SelectionRange {
                            start_disp_col_idx_scr_adj: start,
                            end_disp_col_idx_scr_adj: end,
                        },
                        SelectionRange::caret_movement_direction(prev, curr),
                    );
                }
            }
        }
        _ => {}
    }
}

mod multiline_select_helpers {
    use super::*;

    // XMARK: Impl multiline selection changes (up/down, and later page up/page down)
    /// Precondition: there has to be at least 2 rows.
    pub fn handle_two_lines(
        editor_buffer: &mut EditorBuffer,
        previous_caret_display_position: CaretScrAdj,
        current_caret_display_position: CaretScrAdj,
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

        let locate_previous_row_index = editor_buffer
            .get_selection_list()
            .locate_row(previous.row_index);
        let locate_current_row_index = editor_buffer
            .get_selection_list()
            .locate_row(current.row_index);
        let has_caret_movement_direction_changed = editor_buffer
            .get_selection_list()
            .has_caret_movement_direction_changed(caret_vertical_movement_direction);

        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            let message = "📜📜📜 handle_two_lines";
            let details = string_storage!(
                "\n📜📜📜 {a}\n\t{b}, {c}\n\t{d}\n\t{e}\n\t{f}\n\t{g}\n\t{h}",
                /* heading */
                a = "handle multiline caret movement"
                    .to_string()
                    .red()
                    .on_white(),
                /* previous */
                b = format!("👈 previous: {:?}", previous).cyan().on_dark_grey(),
                /* current */
                c = format!("👉 current: {:?}", current)
                    .magenta()
                    .on_dark_grey(),
                /* selection_map */
                d = format!("{:?}", editor_buffer.get_selection_list())
                    .magenta()
                    .on_dark_grey(),
                /* locate_previous_row_index */
                e = format!("locate_previous_row_index: {:?}", locate_previous_row_index)
                    .cyan()
                    .on_dark_grey(),
                /* locate_current_row_index, */
                f = format!("locate_current_row_index: {:?}", locate_current_row_index,)
                    .magenta()
                    .on_dark_grey(),
                /* caret_vertical_movement_direction, */
                g = format!(
                    "caret_vertical_movement_direction: {:?}",
                    caret_vertical_movement_direction,
                )
                .green()
                .on_dark_grey(),
                /* has_caret_movement_direction_changed, */
                h = format!(
                    "has_caret_movement_direction_changed: {:?}",
                    has_caret_movement_direction_changed,
                )
                .yellow()
                .on_dark_grey(),
            );
            // % is Display, ? is Debug.
            tracing::debug! {
                message = %message,
                details = %details
            };
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
                /* previous_caret */ RowLocationInSelectionList::Overflow,
                /* current_caret */ RowLocationInSelectionList::Overflow,
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
                /* previous_caret */ RowLocationInSelectionList::Overflow,
                /* current_caret  */ RowLocationInSelectionList::Overflow,
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
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret  */ RowLocationInSelectionList::Overflow,
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
                /* previous_caret */ RowLocationInSelectionList::Overflow,
                /* current_caret  */ RowLocationInSelectionList::Contained,
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
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret  */ RowLocationInSelectionList::Overflow,
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
                /* previous_caret */ RowLocationInSelectionList::Overflow,
                /* current_caret  */ RowLocationInSelectionList::Contained,
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
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret  */ RowLocationInSelectionList::Contained,
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
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret  */ RowLocationInSelectionList::Contained,
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
                call_if_true!(DEBUG_TUI_COPY_PASTE, {
                    tracing::debug!(
                        "\n📜📜📜⚾⚾⚾ {0}",
                        /* 0: heading */
                        "handle multiline caret movement Catchall"
                            .to_string()
                            .bold()
                            .yellow()
                            .on_dark_green(),
                    )
                });
            }
        }
    }

    /// No existing selection, up, no direction change:
    /// - Add first row selection range.
    /// - Add last row selection range.
    pub fn start_select_down(
        previous: CaretScrAdj,
        current: CaretScrAdj,
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
        previous: CaretScrAdj,
        current: CaretScrAdj,
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
        first: CaretScrAdj,
        last: CaretScrAdj,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first_row_range: SelectionRange = {
            let start_col = first.col_index;
            let end_col =
                editor_buffer.get_line_display_width_at_row_index(first.row_index);

            (
                start_col,
                // Go one col index past the end of the width, since selection range is
                // not inclusive of end index.
                caret_scroll_index::scroll_col_index_for_width(end_col),
            )
                .into()
        };

        let last_row_range: SelectionRange = {
            let start_col = col(0);
            let end_col = last.col_index;
            (start_col, end_col).into()
        };

        let buffer_mut = editor_buffer.get_mut(width(0) + height(0));
        buffer_mut.sel_list.insert(
            first.row_index,
            first_row_range,
            caret_vertical_movement_direction,
        );
        buffer_mut.sel_list.insert(
            last.row_index,
            last_row_range,
            caret_vertical_movement_direction,
        );
    }

    /// Pre-existing selection, down, no direction change:
    /// - Add last row selection range.
    /// - Modify first row selection range.
    pub fn continue_select_down(
        previous: CaretScrAdj,
        current: CaretScrAdj,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = previous;
        let last = current;

        let first_line_width =
            editor_buffer.get_line_display_width_at_row_index(first.row_index);

        // Mutably borrow the selection map.
        let buffer_mut = editor_buffer.get_mut(width(0) + height(0));

        // Extend the existing range (in selection map) for the first row to end of line.
        if let Some(first_row_range) = buffer_mut.sel_list.get(first.row_index) {
            let start_col = first_row_range.start_disp_col_idx_scr_adj;
            // Go one col index past the end of the width, since selection range is not
            // inclusive of end index.
            let end_col =
                caret_scroll_index::scroll_col_index_for_width(first_line_width);
            let new_first_row_range = SelectionRange {
                start_disp_col_idx_scr_adj: start_col,
                end_disp_col_idx_scr_adj: end_col,
            };
            buffer_mut.sel_list.insert(
                first.row_index,
                new_first_row_range,
                caret_vertical_movement_direction,
            );
        }

        // Add the new last row range to selection map.
        let last_row_range: SelectionRange = {
            let start_col = col(0);
            let end_col = last.col_index;
            (start_col, end_col).into()
        };
        buffer_mut.sel_list.insert(
            last.row_index,
            last_row_range,
            caret_vertical_movement_direction,
        );
    }

    /// Pre-existing selection, up, no direction change:
    /// - Add first row selection range.
    /// - Modify last row selection range.
    pub fn continue_select_up(
        previous: CaretScrAdj,
        current: CaretScrAdj,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = current;
        let last = previous;

        let first_line_width =
            editor_buffer.get_line_display_width_at_row_index(first.row_index);

        // Mutably borrow the selection map.
        let buffer_mut = editor_buffer.get_mut(width(0) + height(0));

        // FIRST ROW.
        if let Some(first_row_range) = buffer_mut.sel_list.get(first.row_index) {
            // Extend the existing range (in selection map) for the first row to end of line.
            let updated_first_row_range = SelectionRange {
                start_disp_col_idx_scr_adj: first_row_range.start_disp_col_idx_scr_adj,
                end_disp_col_idx_scr_adj:
                // Go one col index past the end of the width, since selection range is
                // not inclusive of end index.
                caret_scroll_index::scroll_col_index_for_width(first_line_width),
            };
            buffer_mut.sel_list.insert(
                first.row_index,
                updated_first_row_range,
                caret_vertical_movement_direction,
            );
        } else {
            // Add the new first row range to selection map.
            let new_first_row_range: SelectionRange = {
                let start_col = first.col_index;
                let end_col = first_line_width;
                (
                    start_col,
                    // Go one col index past the end of the width, since selection range is
                    // not inclusive of end index.
                    caret_scroll_index::scroll_col_index_for_width(end_col),
                )
                    .into()
            };
            buffer_mut.sel_list.insert(
                first.row_index,
                new_first_row_range,
                caret_vertical_movement_direction,
            );
        }

        // LAST ROW.
        if let Some(last_row_range) = buffer_mut.sel_list.get(last.row_index) {
            // Extend the existing range (in selection map) for the last row to start of line.
            let start_col = col(0);
            let end_col = last_row_range.end_disp_col_idx_scr_adj;
            let updated_last_row_range = SelectionRange {
                start_disp_col_idx_scr_adj: start_col,
                end_disp_col_idx_scr_adj: end_col,
            };
            buffer_mut.sel_list.insert(
                last.row_index,
                updated_last_row_range,
                caret_vertical_movement_direction,
            );
        } else {
            // Add the new last row range to selection map.
            let new_last_row_range: SelectionRange = {
                let start_col = col(0);
                let end_col = last.col_index;
                (start_col, end_col).into()
            };
            buffer_mut.sel_list.insert(
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
        previous: CaretScrAdj,
        current: CaretScrAdj,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = current;
        let last = previous;

        // Mutably borrow the selection map.
        let buffer_mut = editor_buffer.get_mut(width(0) + height(0));

        // Drop the existing range (in selection map) for the last row.
        if buffer_mut.sel_list.get(last.row_index).is_some() {
            buffer_mut
                .sel_list
                .remove(last.row_index, caret_vertical_movement_direction);
        }

        // Change the existing range (in selection map) for the first row.
        if let Some(first_row_range) = buffer_mut.sel_list.get(first.row_index) {
            let lhs = first_row_range.start_disp_col_idx_scr_adj;
            let rhs = first.col_index;
            match lhs.cmp(&rhs) {
                cmp::Ordering::Equal => {
                    buffer_mut
                        .sel_list
                        .remove(first.row_index, caret_vertical_movement_direction);
                }
                cmp::Ordering::Less | cmp::Ordering::Greater => {
                    buffer_mut.sel_list.insert(
                        first.row_index,
                        SelectionRange {
                            start_disp_col_idx_scr_adj: lhs.min(rhs),
                            end_disp_col_idx_scr_adj: lhs.max(rhs),
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
        previous: CaretScrAdj,
        current: CaretScrAdj,
        editor_buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = previous;
        let last = current;

        // Mutably borrow the selection map.
        let buffer_mut = editor_buffer.get_mut(width(0) + height(0));

        // Drop the existing range (in selection map) for the first row.
        if buffer_mut.sel_list.get(first.row_index).is_some() {
            buffer_mut
                .sel_list
                .remove(first.row_index, caret_vertical_movement_direction);
        }

        // Change the existing range (in selection map) for the last row.
        if let Some(last_row_range) = buffer_mut.sel_list.get(last.row_index) {
            let lhs = last.col_index;
            let rhs = last_row_range.end_disp_col_idx_scr_adj;
            let row_index = last.row_index;
            match lhs.cmp(&rhs) {
                cmp::Ordering::Equal => buffer_mut
                    .sel_list
                    .remove(row_index, caret_vertical_movement_direction),
                cmp::Ordering::Greater | cmp::Ordering::Less => {
                    buffer_mut.sel_list.insert(
                        row_index,
                        SelectionRange {
                            start_disp_col_idx_scr_adj: rhs.min(lhs),
                            end_disp_col_idx_scr_adj: rhs.max(lhs),
                        },
                        caret_vertical_movement_direction,
                    )
                }
            }
        }
    }
}

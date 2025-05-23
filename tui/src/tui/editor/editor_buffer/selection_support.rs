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

use super::{selection_list::RowLocationInSelectionList, EditorBuffer};
use crate::{caret_scr_adj,
            caret_scroll_index,
            col,
            dim,
            fg_blue,
            fg_cyan,
            fg_green,
            fg_magenta,
            fg_red,
            fg_yellow,
            height,
            inline_string,
            row,
            underline,
            usize,
            width,
            CaretLocationInRange,
            CaretMovementDirection,
            CaretScrAdj,
            ChUnitPrimitiveType,
            DirectionChangeResult,
            SelectionRange,
            Size,
            DEBUG_TUI_COPY_PASTE};

/// Usually [EditorBuffer::get_mut()] and [EditorBuffer::get_mut_no_drop()] need a
/// viewport to be passed in (from the [crate::EditorEngine]). However, in this module, we
/// don't need a viewport, nor do we have access to the [crate::EditorEngine], so we use
/// this dummy viewport.
pub fn dummy_viewport() -> Size {
    width(ChUnitPrimitiveType::MAX) + height(ChUnitPrimitiveType::MAX)
}

pub fn handle_selection_single_line_caret_movement(
    buffer: &mut EditorBuffer,
    prev: CaretScrAdj,
    curr: CaretScrAdj,
) {
    let row_index = prev.row_index;
    let prev_col_index = prev.col_index;
    let curr_col_index = curr.col_index;

    // Get the range for the row index. If it doesn't exist, create one & return early.
    let range = {
        let Some(range) = buffer.get_selection_list().get(row_index) else {
            let new_range: SelectionRange = (
                caret_scr_adj(row_index + cmp::min(prev_col_index, curr_col_index)),
                caret_scr_adj(row_index + cmp::max(prev_col_index, curr_col_index)),
            )
                .into();

            let buffer_mut = buffer.get_mut(dummy_viewport());

            buffer_mut.inner.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(
                    caret_scr_adj(prev_col_index + row_index),
                    caret_scr_adj(curr_col_index + row_index),
                ),
            );

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "🍕🍕🍕 new selection",
                    new_range = ?new_range
                );
            });

            return;
        };
        range // Copy & return it.
    };

    // Destructure range for easier access.
    let (range_start, range_end) = range.as_tuple();

    DEBUG_TUI_COPY_PASTE.then(|| {
        tracing::debug!(
                    "\n🍕🍕🍕 {a}:\n\t{b}: {c:?}, {d}: {e:?}\n\t{f}: {g:?}, {h}: {i:?}\n\t{j}: {k}, {l}: {m}, {n}: {o}",
                    a = "modify_existing_range_at_row_index",
                    b = "range_start",
                    c = range_start,
                    d = "range_end",
                    e = range_end,
                    f = "prev_col_index",
                    g = prev_col_index,
                    h = "curr_col_index",
                    i = curr_col_index,
                    j = "previous",
                    k = dim(&inline_string!("{:?}", range.locate_column(prev))),
                    l = "current",
                    m = underline(&inline_string!("{:?}", range.locate_column(curr))),
                    n = "direction",
                    o = fg_green(&inline_string!("{:?}", SelectionRange::caret_movement_direction_left_right(prev, curr)))
                )
    });

    // XMARK: For reference, algo for left, right selection

    // Handle the movement of the caret and apply the appropriate changes to the range.
    match (
        range.locate_column(prev),
        range.locate_column(curr),
        SelectionRange::caret_movement_direction_left_right(prev, curr),
    ) {
        // Left + Shrink range end.
        (
            /* previous_caret */ CaretLocationInRange::Overflow,
            /* current_caret */ CaretLocationInRange::Contained,
            CaretMovementDirection::Left,
        ) => {
            let delta = prev_col_index - curr_col_index;
            let new_range = range.shrink_end_by(width(*delta));

            let buffer_mut = buffer.get_mut(dummy_viewport());
            buffer_mut.inner.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(prev, curr),
            );
        }

        // Left + Grow range start.
        (
            /* previous_caret */ CaretLocationInRange::Contained,
            /* current_caret */ CaretLocationInRange::Underflow,
            CaretMovementDirection::Left,
        ) => {
            let delta = range_start - curr_col_index;
            let new_range = range.grow_start_by(width(*delta));

            let buffer_mut = buffer.get_mut(dummy_viewport());
            buffer_mut.inner.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(prev, curr),
            );
        }

        // Right + Grow range end.
        (
            /* previous_caret */ CaretLocationInRange::Overflow,
            /* current_caret */ CaretLocationInRange::Overflow,
            CaretMovementDirection::Right,
        ) => {
            let delta = curr_col_index - range_end;
            let new_range = range.grow_end_by(width(*delta));

            let buffer_mut = buffer.get_mut(dummy_viewport());
            buffer_mut.inner.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(prev, curr),
            );
        }

        // Right + Shrink range start.
        (
            /* previous_caret */ CaretLocationInRange::Contained,
            /* current_caret */
            CaretLocationInRange::Contained | CaretLocationInRange::Overflow,
            CaretMovementDirection::Right,
        ) => {
            let delta = curr_col_index - range_start;
            let new_range = range.shrink_start_by(width(*delta));

            let buffer_mut = buffer.get_mut(dummy_viewport());
            buffer_mut.inner.sel_list.insert(
                row_index,
                new_range,
                SelectionRange::caret_movement_direction_left_right(prev, curr),
            );
        }

        // Catch all.
        (_, _, _) => {}
    }

    // Remove any range that is empty after caret movement changes have been
    // incorporated. Ok to do this since empty lines are handled by
    // `handle_selection_multiline_caret_movement`.
    if let Some(range) = buffer.get_selection_list().get(row_index) {
        if range.start() == range.end() {
            {
                let buffer_mut = buffer.get_mut(dummy_viewport());
                buffer_mut.inner.sel_list.remove(
                    row_index,
                    SelectionRange::caret_movement_direction_left_right(prev, curr),
                );
            }
        }
    }
}

/// Precondition: there has to be at least 2 rows.
pub fn handle_selection_multiline_caret_movement(
    buffer: &mut EditorBuffer,
    prev: CaretScrAdj,
    curr: CaretScrAdj,
) {
    // Validate preconditions.
    let caret_vertical_movement_direction =
        SelectionRange::caret_movement_direction_up_down(prev, curr);
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
                    buffer,
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
                    buffer,
                    caret_scr_adj(prev_pos),
                    caret_scr_adj(curr_pos),
                );
            }
        }
        _ => {}
    }
}

/// Special case to handle the situation where up / down movement has resulted in the top
/// or bottom of the document to be hit, so that further movement up / down isn't
/// possible, but the caret might jump left or right.
pub fn handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document(
    buffer: &mut EditorBuffer,
    prev: CaretScrAdj,
    curr: CaretScrAdj,
) {
    // Precondition check: Only run if the row previous and current row indices are same.
    if curr.row_index != prev.row_index {
        return;
    }

    let row_index = curr.row_index; // Same as previous.row_index.

    let buffer_mut = buffer.get_mut(dummy_viewport());

    DEBUG_TUI_COPY_PASTE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug! {
            message = "📜🔼🔽 handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document",
            details = %inline_string!(
                "\n{a}\n\t{b}, {c}, {d}, {e}",
                /* 0 */
                a = fg_red("handle multiline caret movement hit top or bottom of document"),
                /* 1: previous */
                b = fg_cyan(&inline_string!("previous: {:?}", prev)),
                /* 2: current */
                c = fg_yellow(&inline_string!("current: {:?}", curr)),
                /* 3: row_index */
                d = fg_green(&inline_string!("row_index: {:?}", row_index)),
                /* 4: selection_map */
                e = fg_magenta(&inline_string!("{:?}", buffer_mut.inner.sel_list))
            ),
        }
    });

    match curr.col_index.cmp(&prev.col_index) {
        cmp::Ordering::Less => {
            match buffer_mut.inner.sel_list.get(row_index) {
                // Extend range to left (caret moved up and hit the top).
                Some(range) => {
                    let start = caret_scr_adj(col(0) + row_index);
                    let end = caret_scr_adj(range.end() + row_index);
                    buffer_mut.inner.sel_list.insert(
                        row_index,
                        (start, end).into(),
                        SelectionRange::caret_movement_direction(prev, curr),
                    );
                }
                // Create range to left (caret moved up and hit the top).
                None => {
                    let start = caret_scr_adj(col(0) + row_index);
                    let end = prev;
                    buffer_mut.inner.sel_list.insert(
                        row_index,
                        (start, end).into(),
                        SelectionRange::caret_movement_direction(prev, curr),
                    );
                }
            }
        }
        cmp::Ordering::Greater => {
            match buffer_mut.inner.sel_list.get(row_index) {
                // Extend range to right (caret moved down and hit bottom).
                Some(range) => {
                    if let Some(line_gcs) = buffer_mut.inner.lines.get(usize(row_index)) {
                        let start = caret_scr_adj(range.start() + row_index);
                        let end = {
                            // For selection, go one col index past the end of the line,
                            // since selection range is not inclusive of the end index.
                            let end_col_index = caret_scroll_index::col_index_for_width(
                                line_gcs.display_width,
                            );
                            caret_scr_adj(end_col_index + row_index)
                        };
                        buffer_mut.inner.sel_list.insert(
                            row_index,
                            (start, end).into(),
                            SelectionRange::caret_movement_direction(prev, curr),
                        );
                    }
                }
                // Create range to right (caret moved down and hit bottom).
                None => {
                    let start = prev;
                    let end = curr;
                    buffer_mut.inner.sel_list.insert(
                        row_index,
                        (start, end).into(),
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
        buffer: &mut EditorBuffer,
        prev: CaretScrAdj,
        curr: CaretScrAdj,
    ) {
        // Validate preconditions.
        let caret_vertical_movement_direction =
            SelectionRange::caret_movement_direction_up_down(prev, curr);
        if let CaretMovementDirection::Overlap = caret_vertical_movement_direction {
            // Invalid state: There must be >= 2 rows, otherwise early return.
            return;
        }

        let locate_previous_row_index =
            buffer.get_selection_list().locate_row(prev.row_index);
        let locate_current_row_index =
            buffer.get_selection_list().locate_row(curr.row_index);
        let has_caret_movement_direction_changed = buffer
            .get_selection_list()
            .has_caret_movement_direction_changed(caret_vertical_movement_direction);

        DEBUG_TUI_COPY_PASTE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "📜📜📜 handle_two_lines",
                details = %inline_string!(
                    "\n📜📜📜 {a}\n\t{b}, {c}\n\t{d}\n\t{e}\n\t{f}\n\t{g}\n\t{h}",
                    /* heading */
                    a = fg_red("handle multiline caret movement"),
                    /* previous */
                    b = fg_cyan(&inline_string!("👈 previous: {:?}", prev)),
                    /* current */
                    c = fg_magenta(&inline_string!("👉 current: {:?}", curr)),
                    /* selection_map */
                    d = fg_magenta(&inline_string!("{:?}", buffer.get_selection_list())),
                    /* locate_previous_row_index */
                    e = fg_cyan(&inline_string!("locate_previous_row_index: {:?}", locate_previous_row_index)),
                    /* locate_current_row_index, */
                    f = fg_green(&inline_string!("locate_current_row_index: {:?}", locate_current_row_index)),
                    /* caret_vertical_movement_direction, */
                    g = fg_blue(&inline_string!(
                        "caret_vertical_movement_direction: {:?}",
                        caret_vertical_movement_direction,
                    )),
                    /* has_caret_movement_direction_changed, */
                    h = fg_yellow(&inline_string!(
                        "has_caret_movement_direction_changed: {:?}",
                        has_caret_movement_direction_changed,
                    ))
                )
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
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionHasChanged: No selection -> Shift+Up -> Shift+Down -> Shift+Up.
            (
                /* previous_caret */ RowLocationInSelectionList::Overflow,
                /* current_caret */ RowLocationInSelectionList::Overflow,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionIsTheSame
                | DirectionChangeResult::DirectionHasChanged,
            ) => multiline_select_helpers::start_select_up(
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionIsTheSame: Previous selection with Shift+Down, then Shift+Down.
            // DirectionHasChanged: No selection -> Shift+Left/Right -> Shift+Down.
            (
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret */ RowLocationInSelectionList::Overflow,
                CaretMovementDirection::Down,
                DirectionChangeResult::DirectionIsTheSame
                | DirectionChangeResult::DirectionHasChanged,
            ) => multiline_select_helpers::continue_select_down(
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // Position caret below empty line, Shift+Up, Shift+Up, Shift+Up, Shift+Down.
            (
                /* previous_caret */ RowLocationInSelectionList::Overflow,
                /* current_caret */ RowLocationInSelectionList::Contained,
                CaretMovementDirection::Down,
                DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_select_down(
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionIsTheSame: Previous selection with Shift+Up, then Shift+Up.
            // DirectionHasChanged: // No selection -> Shift+Left/Right -> Shift+Up.
            (
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret */ RowLocationInSelectionList::Overflow,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionIsTheSame
                | DirectionChangeResult::DirectionHasChanged,
            ) => multiline_select_helpers::continue_select_up(
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // Position caret above empty line, Shift+Down, Shift+Down, Shift+Down,
            // Shift+Up.
            (
                /* previous_caret */ RowLocationInSelectionList::Overflow,
                /* current_caret */ RowLocationInSelectionList::Contained,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_select_up(
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionHasChanged: Previous selection with Shift+Down, then Shift+Up.
            // DirectionIsTheSame: Previous selection with Shift+Down, then Shift+Up, then
            // Shift+Up.
            (
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret */ RowLocationInSelectionList::Contained,
                CaretMovementDirection::Up,
                DirectionChangeResult::DirectionHasChanged
                | DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_direction_change_select_up(
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // DirectionHasChanged: Previous selection with Shift+Up, then Shift+Up, then
            // Shift+Down. DirectionIsTheSame: Previous selection with
            // Shift+Up, then Shift+Down, then Shift+Down.
            (
                /* previous_caret */ RowLocationInSelectionList::Contained,
                /* current_caret */ RowLocationInSelectionList::Contained,
                CaretMovementDirection::Down,
                DirectionChangeResult::DirectionHasChanged
                | DirectionChangeResult::DirectionIsTheSame,
            ) => multiline_select_helpers::continue_direction_change_select_down(
                prev,
                curr,
                buffer,
                caret_vertical_movement_direction,
            ),
            // Catchall.
            _ => {
                DEBUG_TUI_COPY_PASTE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "📜📜📜⚾⚾⚾ handle multiline caret movement Catchall"
                    );
                });
            }
        }
    }

    /// No existing selection, up, no direction change:
    /// - Add first row selection range.
    /// - Add last row selection range.
    pub fn start_select_down(
        prev: CaretScrAdj,
        curr: CaretScrAdj,
        buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = prev;
        let last = curr;

        add_first_and_last_row(first, last, buffer, caret_vertical_movement_direction);
    }

    /// No existing selection, up, no direction change:
    /// - Add first row selection range.
    /// - Add last row selection range.
    pub fn start_select_up(
        prev: CaretScrAdj,
        curr: CaretScrAdj,
        buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = curr;
        let last = prev;

        add_first_and_last_row(first, last, buffer, caret_vertical_movement_direction);
    }

    fn add_first_and_last_row(
        first: CaretScrAdj,
        last: CaretScrAdj,
        buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first_row_range = {
            let first_row_index = first.row_index;
            let start_col = first.col_index;
            let end_col = buffer.get_line_display_width_at_row_index(first_row_index);
            let start = caret_scr_adj(start_col + first_row_index);
            let end = {
                // Go one col index past the end of the width, since selection range is
                // not inclusive of end index.
                let col_index = caret_scroll_index::col_index_for_width(end_col);
                caret_scr_adj(col_index + first_row_index)
            };
            (start, end).into()
        };

        let last_row_range = {
            let last_row_index = last.row_index;
            let start = caret_scr_adj(col(0) + last_row_index);
            let end = caret_scr_adj(last.col_index + last_row_index);
            (start, end).into()
        };

        let buffer_mut = buffer.get_mut(dummy_viewport());
        buffer_mut.inner.sel_list.insert(
            first.row_index,
            first_row_range,
            caret_vertical_movement_direction,
        );
        buffer_mut.inner.sel_list.insert(
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
        buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = previous;
        let last = current;

        let first_line_width =
            buffer.get_line_display_width_at_row_index(first.row_index);

        // Mutably borrow the selection map.
        let buffer_mut = buffer.get_mut(dummy_viewport());

        // Extend the existing range (in selection map) for the first row to end of line.
        if let Some(first_row_range) = buffer_mut.inner.sel_list.get(first.row_index) {
            let start = caret_scr_adj(first_row_range.start() + first.row_index);
            let end = {
                // Go one col index past the end of the width, since selection range is
                // not inclusive of end index.
                let end_col = caret_scroll_index::col_index_for_width(first_line_width);
                caret_scr_adj(end_col + first.row_index)
            };
            let new_first_row_range = (start, end).into();
            buffer_mut.inner.sel_list.insert(
                first.row_index,
                new_first_row_range,
                caret_vertical_movement_direction,
            );
        }

        // Add the new last row range to selection map.
        let last_row_range: SelectionRange = {
            let start_col = col(0);
            let end_col = last.col_index;
            let start = caret_scr_adj(start_col + last.row_index);
            let end = caret_scr_adj(end_col + last.row_index);
            (start, end).into()
        };
        buffer_mut.inner.sel_list.insert(
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
        buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = current;
        let last = previous;

        let first_line_width =
            buffer.get_line_display_width_at_row_index(first.row_index);

        // Mutably borrow the selection map.
        let buffer_mut = buffer.get_mut(dummy_viewport());

        // FIRST ROW.
        if let Some(first_row_range) = buffer_mut.inner.sel_list.get(first.row_index) {
            // Extend the existing range (in selection map) for the first row to end of
            // line.
            let start = caret_scr_adj(first_row_range.start() + first.row_index);
            let end = {
                // Go one col index past the end of the width, since selection range is
                // not inclusive of end index.
                caret_scr_adj(
                    caret_scroll_index::col_index_for_width(first_line_width)
                        + first.row_index,
                )
            };
            let updated_first_row_range = (start, end).into();
            buffer_mut.inner.sel_list.insert(
                first.row_index,
                updated_first_row_range,
                caret_vertical_movement_direction,
            );
        } else {
            // Add the new first row range to selection map.
            let new_first_row_range: SelectionRange = {
                let start_col = first.col_index;
                let end_col = first_line_width;
                let start = caret_scr_adj(start_col + first.row_index);
                let end = {
                    // Go one col index past the end of the width, since selection range
                    // is not inclusive of end index.
                    caret_scr_adj(
                        caret_scroll_index::col_index_for_width(end_col)
                            + first.row_index,
                    )
                };
                (start, end).into()
            };
            buffer_mut.inner.sel_list.insert(
                first.row_index,
                new_first_row_range,
                caret_vertical_movement_direction,
            );
        }

        // LAST ROW.
        if let Some(last_row_range) = buffer_mut.inner.sel_list.get(last.row_index) {
            // Extend the existing range (in selection map) for the last row to start of
            // line.
            let start = caret_scr_adj(col(0) + last.row_index);
            let end = caret_scr_adj(last_row_range.end() + last.row_index);
            let updated_last_row_range = (start, end).into();
            buffer_mut.inner.sel_list.insert(
                last.row_index,
                updated_last_row_range,
                caret_vertical_movement_direction,
            );
        } else {
            // Add the new last row range to selection map.
            let start = caret_scr_adj(col(0) + last.row_index);
            let end = last;
            let new_last_row_range = (start, end).into();
            buffer_mut.inner.sel_list.insert(
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
        buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = current;
        let last = previous;

        // Mutably borrow the selection map.
        let buffer_mut = buffer.get_mut(dummy_viewport());

        // Drop the existing range (in selection map) for the last row.
        if buffer_mut.inner.sel_list.get(last.row_index).is_some() {
            buffer_mut
                .inner
                .sel_list
                .remove(last.row_index, caret_vertical_movement_direction);
        }

        // Change the existing range (in selection map) for the first row.
        if let Some(first_row_range) = buffer_mut.inner.sel_list.get(first.row_index) {
            let lhs = first_row_range.start();
            let rhs = first.col_index;
            match lhs.cmp(&rhs) {
                cmp::Ordering::Equal => {
                    buffer_mut
                        .inner
                        .sel_list
                        .remove(first.row_index, caret_vertical_movement_direction);
                }
                cmp::Ordering::Less | cmp::Ordering::Greater => {
                    let start = caret_scr_adj(lhs.min(rhs) + first.row_index);
                    let end = caret_scr_adj(lhs.max(rhs) + first.row_index);
                    buffer_mut.inner.sel_list.insert(
                        first.row_index,
                        (start, end).into(),
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
        buffer: &mut EditorBuffer,
        caret_vertical_movement_direction: CaretMovementDirection,
    ) {
        let first = previous;
        let last = current;

        // Mutably borrow the selection map.
        let buffer_mut = buffer.get_mut(dummy_viewport());

        // Drop the existing range (in selection map) for the first row.
        if buffer_mut.inner.sel_list.get(first.row_index).is_some() {
            buffer_mut
                .inner
                .sel_list
                .remove(first.row_index, caret_vertical_movement_direction);
        }

        // Change the existing range (in selection map) for the last row.
        if let Some(last_row_range) = buffer_mut.inner.sel_list.get(last.row_index) {
            let lhs = last.col_index;
            let rhs = last_row_range.end();
            let row_index = last.row_index;
            match lhs.cmp(&rhs) {
                cmp::Ordering::Equal => buffer_mut
                    .inner
                    .sel_list
                    .remove(row_index, caret_vertical_movement_direction),
                cmp::Ordering::Greater | cmp::Ordering::Less => {
                    let start = caret_scr_adj(rhs.min(lhs) + row_index);
                    let end = caret_scr_adj(rhs.max(lhs) + row_index);
                    buffer_mut.inner.sel_list.insert(
                        row_index,
                        (start, end).into(),
                        caret_vertical_movement_direction,
                    )
                }
            }
        }
    }
}

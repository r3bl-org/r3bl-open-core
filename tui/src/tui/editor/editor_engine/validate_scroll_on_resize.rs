/*
 *   Copyright (c) 2025 R3BL LLC
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

use r3bl_core::col;

use crate::{caret_scroll_index, EditorArgsMut};

/// Check whether caret is vertically within the viewport.
/// - If it isn't then scroll by mutating:
///   1. [crate::EditorContent::caret_raw]'s row , so it is within the viewport.
///   2. [crate::EditorContent::scr_ofs]'s row, to actually apply scrolling.
/// - Otherwise, no changes are made.
pub fn validate_scroll_on_resize(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;
    validate_vertical_scroll(EditorArgsMut { buffer, engine });
    validate_horizontal_scroll(EditorArgsMut { buffer, engine });
}

/// Handle vertical scrolling (make sure caret is within viewport).
///
/// Check whether caret is in the viewport.
/// - If to top of viewport, then adjust scroll_offset & set it.
/// - If to bottom of viewport, then adjust scroll_offset & set it.
/// - If in viewport, then do nothing.
///
/// ```text
///                    +0--------------------+
///                    0                     |
///                    |        above        | <- caret_row_adj
///                    |                     |
///                    +--- scroll_offset ---+
///              ->    |         ↑           |      ↑
///              |     |                     |      |
///   caret.row_index  |     |      within vp      |  vp height
///              |     |                     |      |
///              ->    |         ↓           |      ↓
///                    +--- scroll_offset ---+
///                    |    + vp height      |
///                    |                     |
///                    |        below        | <- caret_row_adj
///                    |                     |
///                    +---------------------+
/// ```
fn validate_vertical_scroll(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    let viewport = engine.viewport();

    // Make sure that caret can't go past the bottom of the buffer.
    {
        let caret_row = buffer.get_caret_scr_adj().row_index;
        let spilled_row_index =
            caret_scroll_index::scroll_row_index_for_height(buffer.len());
        let overflows_buffer = caret_row > spilled_row_index;
        if overflows_buffer {
            let diff = spilled_row_index - caret_row;
            let buffer_mut = buffer.get_mut(viewport);
            buffer_mut.caret_raw.row_index -= diff;
        }
    }

    // Make sure that scroll_offset can't go past the bottom of the buffer.
    {
        let scr_ofs_row = buffer.get_scr_ofs().row_index;
        let spilled_row_index =
            caret_scroll_index::scroll_row_index_for_height(buffer.len());
        let overflows_buffer = scr_ofs_row > spilled_row_index;
        if overflows_buffer {
            let diff = spilled_row_index - scr_ofs_row;
            let buffer_mut = buffer.get_mut(viewport);
            buffer_mut.scr_ofs.row_index -= diff;
        }
    }

    let caret_row = buffer.get_caret_scr_adj().row_index;
    let scr_ofs_row = buffer.get_scr_ofs().row_index;

    let is_caret_row_within_viewport =
        caret_row >= scr_ofs_row && caret_row <= (scr_ofs_row + viewport.row_height);
    let is_caret_row_above_viewport = caret_row < scr_ofs_row;

    // REVIEW: [ ] replace use of bool w/ enum
    match (is_caret_row_within_viewport, is_caret_row_above_viewport) {
        (true, _) => {
            // Caret is within viewport, do nothing.
        }
        (false, true) => {
            // Caret is above viewport.
            let row_diff = scr_ofs_row - caret_row;
            let buffer_mut = buffer.get_mut(viewport);
            buffer_mut.scr_ofs.row_index -= row_diff;
            buffer_mut.caret_raw.row_index += row_diff;
        }
        (false, false) => {
            // Caret is below viewport.
            let row_diff = caret_row - (scr_ofs_row + viewport.row_height);
            let buffer_mut = buffer.get_mut(viewport);
            buffer_mut.scr_ofs.row_index += row_diff;
            buffer_mut.caret_raw.row_index -= row_diff;
        }
    }
}

/// Handle horizontal scrolling (make sure caret is within viewport).
///
/// Check whether caret is in the viewport.
/// - If to left of viewport, then adjust scroll_offset & set it.
/// - If to right of viewport, then adjust scroll_offset & set it.
/// - If in viewport, then do nothing.
///
/// ```text
///           <-   vp width   ->
/// +0--------+----------------+---------->
/// 0         |                |
/// | left of |<-  within vp ->| right of
/// |         |                |
/// +---------+----------------+---------->
///       scroll_offset    scroll_offset
///                        + vp width
/// ```
fn validate_horizontal_scroll(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    let viewport = engine.viewport();

    let caret_col = buffer.get_caret_scr_adj().col_index;
    let scr_ofs_col = buffer.get_scr_ofs().col_index;

    let is_caret_col_abs_within_viewport =
        caret_col >= scr_ofs_col && caret_col < scr_ofs_col + viewport.col_width;

    match is_caret_col_abs_within_viewport {
        true => {
            // Caret is within viewport, nothing to do.
        }
        false => {
            // Caret is outside viewport.
            let buffer_mut = buffer.get_mut(viewport);

            if caret_col < scr_ofs_col {
                // Caret is to the left of viewport.
                buffer_mut.scr_ofs.col_index = caret_col;
                buffer_mut.caret_raw.col_index = col(0);
            } else {
                // Caret is to the right of viewport.
                let viewport_width = buffer_mut.vp.col_width;
                buffer_mut.scr_ofs.col_index = caret_col - viewport_width + col(1);
                buffer_mut.caret_raw.col_index = viewport_width.convert_to_col_index();
            }
        }
    }
}

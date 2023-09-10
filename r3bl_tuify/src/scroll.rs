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

//! ### Vertical scrolling and viewport
//!
//! ```text
//!                    +0--------------------+
//!                    0                     |
//!                    |        above        | <- caret_row_scroll_adj
//!                    |                     |
//!                    +--- scroll_offset ---+
//!              ->    |         ↑           |      ↑
//!   raw_caret  |     |                     |      |
//!   _row_index |     |      within vp      |  vp height
//!              |     |                     |      |
//!              ->    |         ↓           |      ↓
//!                    +--- scroll_offset ---+
//!                    |    + vp height      |
//!                    |                     |
//!                    |        below        | <- caret_row_scroll_adj
//!                    |                     |
//!                    +---------------------+
//! ```

use r3bl_rs_utils_core::*;

pub fn get_scroll_adjusted_row_index(
    raw_caret_row_index: ChUnit,
    scroll_offset_row_index: ChUnit,
) -> ChUnit {
    raw_caret_row_index + scroll_offset_row_index
}

pub fn locate_cursor_in_viewport(
    raw_caret_row_index: ChUnit,
    scroll_offset_row_index: ChUnit,
    display_height: ChUnit,
    items_size: ChUnit,
) -> CaretVerticalViewportLocation {
    let caret_row_scroll_adj = raw_caret_row_index + scroll_offset_row_index;

    if caret_row_scroll_adj == ch!(0) {
        CaretVerticalViewportLocation::AtAbsoluteTop
    } else if caret_row_scroll_adj < scroll_offset_row_index {
        CaretVerticalViewportLocation::AboveTopOfViewport
    } else if caret_row_scroll_adj == scroll_offset_row_index {
        CaretVerticalViewportLocation::AtTopOfViewport
    }
    // When comparing height or width or size to index, we need to subtract 1.
    else if caret_row_scroll_adj == (display_height - 1) {
        CaretVerticalViewportLocation::AtBottomOfViewport
    }
    // When comparing height or width or size to index, we need to subtract 1.
    else if caret_row_scroll_adj > (display_height - 1) {
        if caret_row_scroll_adj == (items_size - 1) {
            CaretVerticalViewportLocation::AtAbsoluteBottom
        } else {
            CaretVerticalViewportLocation::BelowBottomOfViewport
        }
    } else {
        CaretVerticalViewportLocation::InMiddleOfViewport
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CaretVerticalViewportLocation {
    AtAbsoluteTop,
    AboveTopOfViewport,
    AtTopOfViewport,
    InMiddleOfViewport,
    AtBottomOfViewport,
    BelowBottomOfViewport,
    AtAbsoluteBottom,
}

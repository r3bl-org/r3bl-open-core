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
//! When you call [locate_cursor_in_viewport] function, it returns a location that is
//! relative to the viewport. The viewport is the visible area of the terminal.
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
//!
//! What the [CaretVerticalViewportLocation] enum represents:
//!
//! ```text
//!    +0--------------------+ <- AtAbsoluteTop
//!    0                     |
//!    |        above        | <- AboveTopOfViewport
//!    |                     |
//!    +--- scroll_offset ---+ <- AtTopOfViewport
//!    |         ↑           |
//!    |                     |
//!    |      within vp      | <- InMiddleOfViewport
//!    |                     |
//!    |         ↓           |
//!    +--- scroll_offset ---+ <- AtBottomOfViewport
//!    |    + vp height      |
//!    |                     |
//!    |        below        | <- BelowBottomOfViewport
//!    |                     |
//!    +---------------------+ <- AtAbsoluteBottom
//! ```

use crossterm::style::Stylize;
use r3bl_rs_utils_core::*;

use crate::{HEADER_HEIGHT, TRACE};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CaretVerticalViewportLocation {
    AtAbsoluteTop,
    AboveTopOfViewport,
    AtTopOfViewport,
    InMiddleOfViewport,
    AtBottomOfViewport,
    BelowBottomOfViewport,
    AtAbsoluteBottom,
    NotFound,
}

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
    let abs_row_index =
        get_scroll_adjusted_row_index(raw_caret_row_index, scroll_offset_row_index);

    call_if_true!(TRACE, {
        log_debug(format!(
            "locate_cursor_in_viewport(): raw_caret_row_index: {}, scroll_offset_row_index: {}, abs_row_index: {}, display_height: {}, items_size: {}",
            raw_caret_row_index, scroll_offset_row_index, abs_row_index, display_height, items_size
        ).green().to_string());
    });

    // Note the ordering of the statements below matters.
    if abs_row_index == items_size - 1 {
        // AtAbsoluteBottom takes precedence over AtAbsoluteTop when there is only one item.
        CaretVerticalViewportLocation::AtAbsoluteBottom
    } else if abs_row_index == ch!(0) {
        CaretVerticalViewportLocation::AtAbsoluteTop
    } else if abs_row_index < scroll_offset_row_index {
        CaretVerticalViewportLocation::AboveTopOfViewport
    } else if abs_row_index == scroll_offset_row_index {
        CaretVerticalViewportLocation::AtTopOfViewport
    }
    // When comparing height or width or size to index, we need to subtract 1.
    else if abs_row_index > scroll_offset_row_index
        && abs_row_index < (scroll_offset_row_index + display_height-2)
    {
        CaretVerticalViewportLocation::InMiddleOfViewport
    } else if abs_row_index == (scroll_offset_row_index + display_height - ch!(HEADER_HEIGHT) - 1) {
        CaretVerticalViewportLocation::AtBottomOfViewport
    } else if abs_row_index > (scroll_offset_row_index + display_height - ch!(HEADER_HEIGHT) - 1) {
        CaretVerticalViewportLocation::BelowBottomOfViewport
    } else {
        CaretVerticalViewportLocation::NotFound
    }
}

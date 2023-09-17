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

use r3bl_rs_utils_core::*;

use crate::*;

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct State {
    pub max_display_height: ChUnit,
    pub max_display_width: ChUnit,
    /// This is not adjusted for
    /// [scroll_offset_row_index](State::scroll_offset_row_index).
    pub raw_caret_row_index: ChUnit,
    pub scroll_offset_row_index: ChUnit,
    pub items: Vec<String>,
    pub selected_items: Vec<String>,
}

impl State {
    pub fn get_selected_index(&self) -> ChUnit {
        get_scroll_adjusted_row_index(
            self.raw_caret_row_index,
            self.scroll_offset_row_index,
        )
    }

    pub fn locate_cursor_in_viewport(&self) -> CaretVerticalViewportLocation {
        locate_cursor_in_viewport(
            self.raw_caret_row_index,
            self.scroll_offset_row_index,
            self.max_display_height,
            self.items.len().into(),
        )
    }
}

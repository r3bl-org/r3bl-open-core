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
    /// Does not include the header row.
    pub max_display_height: ChUnit,
    pub max_display_width: ChUnit,
    /// This is not adjusted for
    /// [scroll_offset_row_index](State::scroll_offset_row_index).
    pub raw_caret_row_index: ChUnit,
    pub scroll_offset_row_index: ChUnit,
    pub items: Vec<String>,
    pub selected_items: Vec<String>,
    pub header: String,
    pub selection_mode: SelectionMode,
    pub resize_hint: Option<ResizeHint>,
    pub window_size: Option<Size>,
}

impl CalculateResizeHint for State {
    fn set_size(&mut self, new_size: Size) {
        self.window_size = Some(new_size);
        self.clear_resize_hint();
    }

    fn get_resize_hint(&self) -> Option<ResizeHint> {
        self.resize_hint.clone()
    }

    fn set_resize_hint(&mut self, new_size: Size) {
        self.resize_hint = if let Some(old_size) = self.window_size {
            if new_size != old_size {
                if (new_size.col_count > old_size.col_count)
                    || (new_size.row_count > old_size.row_count)
                {
                    Some(ResizeHint::GotBigger)
                } else if (new_size.col_count < old_size.col_count)
                    || (new_size.row_count < old_size.row_count)
                {
                    Some(ResizeHint::GotSmaller)
                } else {
                    Some(ResizeHint::NoChange)
                }
            } else {
                None
            }
        } else {
            None
        };

        if self.window_size.is_some() {
            self.window_size = Some(new_size);
        }
    }

    fn clear_resize_hint(&mut self) {
        self.resize_hint = None;
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub enum ResizeHint {
    GotBigger,
    GotSmaller,
    #[default]
    NoChange,
}

impl State {
    /// This the row index that currently has keyboard focus.
    pub fn get_focused_index(&self) -> ChUnit {
        get_scroll_adjusted_row_index(self.raw_caret_row_index, self.scroll_offset_row_index)
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

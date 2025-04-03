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

use r3bl_core::{CaretRaw, Height, InlineVec, ScrOfs, Size, Width};

use super::{Header, HowToChoose};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct State<'a> {
    /// Does not include the header row.
    pub max_display_height: Height,
    pub max_display_width: Width,
    /// This is not adjusted for [Self::scroll_offset_row_index].
    pub raw_caret_row_index: CaretRaw,
    pub scroll_offset_row_index: ScrOfs,
    pub items: &'a [&'a str],
    pub selected_items: InlineVec<&'a str>,
    pub selected_indices: InlineVec<usize>,
    pub header: Header<'a>,
    pub selection_mode: HowToChoose,
    /// This is used to determine if the terminal has been resized.
    pub resize_hint: Option<ResizeHint>,
    /// This is used to determine if the terminal has been resized.
    pub window_size: Option<Size>,
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub enum ResizeHint {
    GotBigger,
    GotSmaller,
    #[default]
    NoChange,
}

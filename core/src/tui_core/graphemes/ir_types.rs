/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use super::{UnicodeString, UnicodeStringExt as _};
use crate::{ColIndex, ColWidth};

/// This represents a slice of the original [UnicodeString] and owns data. This is used to
/// represent segments of the original string that are returned as a result of various
/// computations, eg: [UnicodeString::get_string_at_right_of_display_col_index], etc.
///
/// We need an owned struct (since we're returning a slice that is dropped by the function
/// that creates it, not as a result of mutation).
#[derive(Debug, PartialEq, Eq)]
pub struct UnicodeStringSegmentSliceResult {
    /// The grapheme cluster slice, as a [UnicodeString]. This is a copy of the slice from
    /// the original string.
    pub seg_text: UnicodeString,
    /// The display width of the slice.
    pub seg_display_width: ColWidth,
    /// The display col index at which this grapheme cluster starts in the original string.
    pub display_col_at_which_seg_starts: ColIndex,
}

impl UnicodeStringSegmentSliceResult {
    pub fn new(text: &str, width: ColWidth, display_col_start: ColIndex) -> Self {
        Self {
            seg_text: text.unicode_string(),
            seg_display_width: width,
            display_col_at_which_seg_starts: display_col_start,
        }
    }
}

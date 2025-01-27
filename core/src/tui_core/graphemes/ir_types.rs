/*
 *   Copyright (c) 2022 R3BL LLC
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
use crate::ChUnit;

/// We need a [String] (since we're returning a slice of a temporary
/// [crate::UnicodeString] that is dropped by the function that creates it, not as a
/// result of mutation).
#[derive(Debug, PartialEq, Eq)]
pub struct UnicodeStringSegmentSliceResult {
    pub unicode_string: UnicodeString,
    pub display_width: ChUnit,
    pub display_col_at_which_this_seg_starts: ChUnit,
}

impl UnicodeStringSegmentSliceResult {
    pub fn new(
        text: &str,
        display_width: ChUnit,
        display_col_at_which_this_seg_starts: ChUnit,
    ) -> Self {
        Self {
            unicode_string: text.unicode_string(),
            display_width,
            display_col_at_which_this_seg_starts,
        }
    }
}

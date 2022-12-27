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

use crate::*;

/// We need a [String] (since we're returning a slice of a temporary [UnicodeString] that is
/// dropped by the function that creates it, not as a result of mutation).
#[derive(Debug, PartialEq, Eq)]
pub struct UnicodeStringSegmentSliceResult {
    pub unicode_string_seg: UnicodeString,
    pub unicode_width: ChUnit,
    pub display_col_at_which_seg_starts: ChUnit,
}

mod unicode_string_segment_slice_result_impl {
    use super::*;

    impl UnicodeStringSegmentSliceResult {
        pub fn new(
            string: &str,
            unicode_width: ChUnit,
            display_col_at_which_this_segment_starts: ChUnit,
        ) -> Self {
            Self {
                unicode_string_seg: string.into(),
                unicode_width,
                display_col_at_which_seg_starts: display_col_at_which_this_segment_starts,
            }
        }
    }
}

/// We need a [String] (since we're returning a new [String] as a result of this [UnicodeString]
/// mutation).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct NewUnicodeStringResult {
    pub new_unicode_string: UnicodeString,
    pub unicode_width: ChUnit,
}

mod new_unicode_string_result_impl {
    use super::*;

    impl NewUnicodeStringResult {
        pub fn new(new_string: String, unicode_width: ChUnit) -> Self {
            Self {
                new_unicode_string: new_string.into(),
                unicode_width,
            }
        }
    }
}

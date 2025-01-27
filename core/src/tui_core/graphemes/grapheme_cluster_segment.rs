/*
 *   Copyright (c) 2024 R3BL LLC
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

use super::UnicodeString;
use crate::{ChUnit, usize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, size_of::SizeOf)]
/// The actual grapheme cluster string slice &[str], is derived using
/// `start_index`..`end_index` from the original string slice used to generate the
/// [UnicodeString]. Eg: "H", "📦", "🙏🏽".
pub struct GraphemeClusterSegment {
    // PERF: [x] perf (remove alloc)
    /// The start index, in the string slice, used to generate the [UnicodeString] that this
    /// grapheme cluster represents.
    pub start_index: ChUnit,

    /// The end index, in the string slice, used to generate the [UnicodeString] that this
    /// grapheme cluster represents.
    pub end_index: ChUnit,

    /// The byte offset (in the original string slice) of the start of this grapheme cluster.
    pub byte_offset: ChUnit,

    /// Display width of the grapheme cluster calculated using
    /// [unicode_width::UnicodeWidthChar]. The display width (aka `unicode_width`)
    /// may not the same as the byte size [Self::byte_size].
    pub unicode_width: ChUnit,

    /// The index of this entry in the [UnicodeString::vec_segment].
    pub logical_index: ChUnit,

    /// The number of bytes this grapheme cluster occupies in the original string slice.
    /// The display width, aka [Self::unicode_width], may not the same as the byte size.
    pub byte_size: usize,

    /// Display col at which this grapheme cluster starts.
    pub display_col_offset: ChUnit,
}

impl GraphemeClusterSegment {
    /// Get the string slice for the grapheme cluster segment. Closely related to
    /// [UnicodeString::get_str].
    pub fn get_str<'a>(&self, string: &'a str) -> &'a str {
        let start_index = usize(self.start_index);
        let end_index = usize(self.end_index);
        &string[start_index..end_index]
    }
}

impl UnicodeString {
    /// Get the string slice for the grapheme cluster segment. Closely related to
    /// [GraphemeClusterSegment::get_str].
    pub fn get_str<'a>(
        &self, /* not actually used, but allows get_str() to be a method */
        string: &'a str,
        seg: &GraphemeClusterSegment,
    ) -> &'a str {
        let start_index = crate::usize(seg.start_index);
        let end_index = crate::usize(seg.end_index);
        &string[start_index..end_index]
    }
}

/// Macro to call [crate::GraphemeClusterSegment::get_str] on a
/// [crate::GraphemeClusterSegment] and [UnicodeString].
#[macro_export]
macro_rules! seg_str {
    ($seg:expr, $unicode_string:expr) => {
        $crate::GraphemeClusterSegment::get_str(&$seg, &$unicode_string.string)
    };
}

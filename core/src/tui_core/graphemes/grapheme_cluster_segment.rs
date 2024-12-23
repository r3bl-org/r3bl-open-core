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

use serde::{Deserialize, Serialize};

use super::UnicodeString;
use crate::{ChUnit, usize};

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Hash, size_of::SizeOf, Serialize, Deserialize,
)]
/// The actual grapheme cluster &[str]` is derived using `start_index`..`end_index` from
/// the [UnicodeString::string]. Eg: "H", "📦", "🙏🏽".
pub struct GraphemeClusterSegment {
    // PERF: [x] perf (remove alloc)
    /// The start index of the [UnicodeString::string]  that this grapheme cluster
    /// represents.
    pub start_index: ChUnit,
    /// The end index of the [UnicodeString::string] that this grapheme cluster
    /// represents.
    pub end_index: ChUnit,
    /// The byte offset (in the original string) of the start of the grapheme cluster.
    pub byte_offset: ChUnit,
    /// Display width of the [UnicodeString::string] via [`unicode_width::UnicodeWidthChar`].
    pub unicode_width: ChUnit,
    /// The index of this entry in the [UnicodeString::vec_segment].
    pub logical_index: ChUnit,
    /// The number of bytes the [UnicodeString::string] takes up in memory.
    pub byte_size: usize,
    /// Display col at which this grapheme cluster starts.
    pub display_col_offset: ChUnit,
}

impl GraphemeClusterSegment {
    pub fn get_str<'a>(&self, text: &'a str) -> &'a str {
        &text[usize(self.start_index)..usize(self.end_index)]
    }
}

impl UnicodeString {
    pub fn get_str(&self, seg: &GraphemeClusterSegment) -> &str {
        seg.get_str(&self.string)
    }
}

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
use crate::ChUnit;

#[derive(
    Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Hash, size_of::SizeOf,
)]
pub struct GraphemeClusterSegment {
    /// The actual grapheme cluster `&str`. Eg: "H", "📦", "🙏🏽".
    pub string: String,
    /// The byte offset (in the original string) of the start of the `grapheme_cluster`.
    pub byte_offset: usize,
    /// Display width of the `string` via [`unicode_width::UnicodeWidthChar`].
    pub unicode_width: ChUnit,
    /// The index of this entry in the `grapheme_cluster_segment_vec`.
    pub logical_index: usize,
    /// The number of bytes the `string` takes up in memory.
    pub byte_size: usize,
    /// Display col at which this grapheme cluster starts.
    pub display_col_offset: ChUnit,
}

impl GraphemeClusterSegment {
    /// Convert [&str] to [GraphemeClusterSegment]. This is used to create a new [String] after the
    /// [UnicodeString] is modified.
    pub fn new(chunk: &str) -> GraphemeClusterSegment {
        let my_string: String = chunk.to_string();
        let unicode_string = UnicodeString::from(my_string);
        let result = unicode_string.vec_segment.first().unwrap().clone();
        result
    }
}

impl From<&str> for GraphemeClusterSegment {
    fn from(s: &str) -> Self { GraphemeClusterSegment::new(s) }
}

impl From<String> for GraphemeClusterSegment {
    fn from(s: String) -> Self { GraphemeClusterSegment::new(&s) }
}

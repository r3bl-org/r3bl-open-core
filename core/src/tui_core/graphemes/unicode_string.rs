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
use std::{fmt::Display,
          ops::{Deref, DerefMut}};

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

use super::GraphemeClusterSegment;
use crate::{ChUnit, MicroVecBackingStore, SmallStringBackingStore, ch};

#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct UnicodeString {
    // PERF: [ ] perf
    pub string: SmallStringBackingStore,
    pub vec_segment: MicroVecBackingStore<GraphemeClusterSegment>,
    pub byte_size: usize,
    pub grapheme_cluster_segment_count: usize,
    pub display_width: ChUnit,
}

impl Default for UnicodeString {
    fn default() -> Self {
        UnicodeString {
            string: SmallStringBackingStore::new(),
            vec_segment: MicroVecBackingStore::new(),
            byte_size: 0,
            grapheme_cluster_segment_count: 0,
            display_width: ch(0),
        }
    }
}

// PERF: [ ] perf
impl size_of::SizeOf for UnicodeString {
    fn size_of_children(&self, context: &mut size_of::Context) {
        /* string */
        context.add(self.string.len());
        /* vec_segment */
        context.add(self.vec_segment.size_of().total_bytes());
        /* byte_size */
        context.add(std::mem::size_of::<usize>());
        /* grapheme_cluster_segment_count */
        context.add(std::mem::size_of::<usize>());
        /* display_width */
        context.add(std::mem::size_of::<ChUnit>());
    }
}

impl UnicodeString {
    /// Constructor function that creates a [UnicodeString] from a string slice.
    pub fn new(this: &str) -> UnicodeString {
        let mut total_byte_offset = 0;
        let mut total_grapheme_cluster_count = 0;
        let mut my_unicode_width_offset_accumulator: ChUnit = ch(0);

        let iter = this.grapheme_indices(true).enumerate();
        let size = iter.clone().count();
        let mut my_unicode_string_segments = MicroVecBackingStore::with_capacity(size);

        for (grapheme_cluster_index, (byte_offset, grapheme_cluster_str)) in iter {
            let unicode_width =
                ch(UnicodeString::str_display_width(grapheme_cluster_str));
            my_unicode_string_segments.push(GraphemeClusterSegment {
                start_index: ch(byte_offset),
                end_index: ch(byte_offset) + ch(grapheme_cluster_str.len()),
                byte_offset: ch(byte_offset),
                unicode_width,
                logical_index: ch(grapheme_cluster_index),
                byte_size: grapheme_cluster_str.len(),
                display_col_offset: my_unicode_width_offset_accumulator,
            });
            my_unicode_width_offset_accumulator += unicode_width;
            total_byte_offset = byte_offset;
            total_grapheme_cluster_count = grapheme_cluster_index;
        }

        UnicodeString {
            string: this.into(),
            vec_segment: my_unicode_string_segments,
            display_width: my_unicode_width_offset_accumulator,
            byte_size: if total_byte_offset > 0 {
                total_byte_offset + 1 /* size = byte_offset (index) + 1 */
            } else {
                total_byte_offset
            },
            grapheme_cluster_segment_count: if total_grapheme_cluster_count > 0 {
                total_grapheme_cluster_count + 1 /* count = grapheme_cluster_index + 1 */
            } else {
                total_grapheme_cluster_count
            },
        }
    }

    pub fn get_char_width(arg: char) -> ChUnit {
        UnicodeWidthChar::width(arg).unwrap_or(0).into()
    }
}

impl Deref for UnicodeString {
    type Target = MicroVecBackingStore<GraphemeClusterSegment>;

    fn deref(&self) -> &Self::Target { &self.vec_segment }
}

impl DerefMut for UnicodeString {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.vec_segment }
}

impl Display for UnicodeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}

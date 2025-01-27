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

use std::ops::{Deref, DerefMut};

use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::GraphemeClusterSegment;
use crate::{ChUnit, StringStorage, ch};

/// The unicode `UnicodeString` struct and other files in this module
/// [mod@crate::tui_core::graphemes] related functionality for handling Unicode strings
/// with grapheme cluster segmentation. It includes methods for creating and manipulating
/// Unicode strings, as well as calculating their (display) width and (memory) size, which
/// might not be the same.
///
/// The `UnicodeString` struct is designed to store Unicode strings efficiently, keeping
/// track of grapheme clusters and their properties such as byte offsets, display width,
/// and logical indices. This allows for accurate and efficient manipulation and rendering
/// of Unicode strings in terminal user interfaces (TUIs).
///
/// # Performance, memory latency, access, allocation
///
/// For performance reasons, the `UnicodeString` struct owns the underlying string data.
/// The vector of grapheme cluster segments is stored separately to avoid unnecessary
/// allocations and copying. This design allows for efficient access to individual
/// grapheme clusters and their properties, such as display width and byte size.
///
/// We tried making a variant of `UnicodeString` that does not own any data (the
/// underlying string) but this design produced much slower performance due to the need
/// for repeated dereferencing of the string data that was in a different location (non
/// local to `UnicodeString`) via a different struct. This was unintuitive, as we were
/// expecting the lack of allocation to prove faster, but it turned out to be slower!
/// Intuition around performance is not reliable, and it is best to measure and test each
/// design choice.
///
/// Having said this, the [GraphemeClusterSegment] struct is designed to be as lightweight
/// as possible, with only the necessary properties for representing a grapheme cluster.
/// It does not own any data and only stores references to the original string slice. This
/// does not impact performance significantly, due to the nature in which it is used. So
/// this design choice (no ownership and slicing into an existing struct) work for this
/// use case it does not work for the `UnicodeString` struct.
///
/// # Features
///
/// - `UnicodeString`: Struct for representing Unicode strings with grapheme cluster
///   segmentation.
/// - [Self::new]: A constructor function for creating a `UnicodeString` from a string
///   slice.
/// - [Self::char_display_width]: A utility function for calculating the display width of
///   a character.
/// - [Self::str_display_width]: A utility function for calculating the display width of a
///   string slice.
///
/// # Traits
///
/// The `UnicodeString` struct implements the following traits:
/// - `Deref`: For dereferencing `UnicodeString` instances. When the `*` operator is used,
///    the underlying `MicroVecBackingStore<GraphemeClusterSegment>` is returned. This is
///    really important to note when using `len()`, which will return the number of
///    grapheme clusters and this is not the same as the `display_width` of the
///    `UnicodeString`.
/// - `DerefMut`: For mutable dereferencing of `UnicodeString` instances.
/// - `Default`: For creating a default `UnicodeString` instance.
/// - `PartialEq`: For comparing two `UnicodeString` instances for equality.
/// - `Eq`: For checking if two `UnicodeString` instances are equal.
/// - `Hash`: For hashing `UnicodeString` instances.
/// - `Clone`: For creating a copy of a `UnicodeString` instance.
/// - `Debug`: For debugging `UnicodeString` instances.
/// - `SizeOf`: For calculating the size of `UnicodeString` instances.
///
/// # Dependencies
///
/// This module relies on the following external crates:
///
/// - [unicode_segmentation]: For splitting strings into grapheme clusters.
/// - [unicode_width]: For calculating the display width of Unicode characters.
///
/// # Example
///
/// ```rust
/// use r3bl_core::tui_core::graphemes::unicode_string::UnicodeString;
///
/// let unicode_str = UnicodeString::new("Hello, 世界");
/// println!("Display width: {it:?}", it = unicode_str.display_width);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct UnicodeString {
    // PERF: [ ] perf
    pub string: StringStorage,
    pub vec_segment: sizing::VecSegment,
    pub byte_size: usize,
    pub grapheme_cluster_segment_count: usize,
    // REFACTOR: [x] replace all usages of .display_width w/ .display_width()
    pub display_width: ChUnit,
}

mod sizing {
    use super::*;
    pub type VecSegment = SmallVec<[GraphemeClusterSegment; VEC_SEGMENT_SIZE]>;
    const VEC_SEGMENT_SIZE: usize = 28;
}

impl Default for UnicodeString {
    fn default() -> Self {
        UnicodeString {
            string: StringStorage::new(),
            vec_segment: sizing::VecSegment::new(),
            byte_size: 0,
            grapheme_cluster_segment_count: 0,
            display_width: ch(0),
        }
    }
}

// PERF: [ ] perf
impl size_of::SizeOf for UnicodeString {
    fn size_of_children(&self, context: &mut size_of::Context) {
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

impl AsRef<str> for &UnicodeString {
    fn as_ref(&self) -> &str { &self.string }
}

impl UnicodeString {
    /// Constructor function that creates a [UnicodeString] from a string slice.
    pub fn new(this: &str) -> UnicodeString {
        let mut total_byte_offset = 0;
        let mut total_grapheme_cluster_count = 0;
        let mut my_unicode_width_offset_accumulator: ChUnit = ch(0);

        let iter = this.grapheme_indices(true).enumerate();
        let size = iter.clone().count();
        let mut my_unicode_string_segments = sizing::VecSegment::with_capacity(size);

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

    pub fn char_display_width(character: char) -> ChUnit {
        ch(UnicodeWidthChar::width(character).unwrap_or(0))
    }

    pub fn str_display_width(string: &str) -> ChUnit {
        ch(UnicodeWidthStr::width(string))
    }
}

impl Deref for UnicodeString {
    type Target = sizing::VecSegment;

    fn deref(&self) -> &Self::Target { &self.vec_segment }
}

impl DerefMut for UnicodeString {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.vec_segment }
}

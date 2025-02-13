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

use std::ops::{Deref, DerefMut};

use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::GraphemeClusterSegment;
use crate::{ChUnit, ColWidth, StringStorage, ch, col, width};

/// The `UnicodeString` struct and other files in this module
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
///   the underlying `MicroVecBackingStore<GraphemeClusterSegment>` is returned. This is
///   really important to note when using `len()`, which will return the number of
///   grapheme clusters and this is not the same as the `display_width` of the
///   `UnicodeString`.
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
    pub display_width: ColWidth,
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
            display_width: width(0),
        }
    }
}

// PERF: [ ] perf
impl size_of::SizeOf for UnicodeString {
    fn size_of_children(&self, context: &mut size_of::Context) {
        /* vec_segment */
        context.add(size_of_val(&self.vec_segment));
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
        // This is used both for the width and display col index.
        let mut unicode_width_offset_acc: ChUnit = ch(0);

        let iter = this.grapheme_indices(true).enumerate();
        let size = iter.clone().count();
        let mut unicode_string_segments = sizing::VecSegment::with_capacity(size);

        for (grapheme_cluster_index, (byte_offset, grapheme_cluster_str)) in iter {
            let unicode_width = UnicodeString::str_display_width(grapheme_cluster_str);
            unicode_string_segments.push(GraphemeClusterSegment {
                start_byte_index: ch(byte_offset),
                end_byte_index: ch(byte_offset) + ch(grapheme_cluster_str.len()),
                unicode_width,
                logical_index: ch(grapheme_cluster_index),
                byte_size: grapheme_cluster_str.len(),
                start_display_col_index: col(unicode_width_offset_acc), // Used as ColIndex here.
            });
            unicode_width_offset_acc += *unicode_width;
            total_byte_offset = byte_offset;
            total_grapheme_cluster_count = grapheme_cluster_index;
        }

        UnicodeString {
            string: this.into(),
            vec_segment: unicode_string_segments,
            display_width: width(unicode_width_offset_acc), // Used as WidthColCount here.
            byte_size: if total_byte_offset > 0 {
                /* size = byte_offset (index) + 1 */
                total_byte_offset + 1
            } else {
                total_byte_offset
            },
            grapheme_cluster_segment_count: if total_grapheme_cluster_count > 0 {
                /* count = grapheme_cluster_index + 1 */
                total_grapheme_cluster_count + 1
            } else {
                total_grapheme_cluster_count
            },
        }
    }

    pub fn char_display_width(character: char) -> ColWidth {
        width(UnicodeWidthChar::width(character).unwrap_or(0))
    }

    pub fn str_display_width(string: &str) -> ColWidth {
        width(UnicodeWidthStr::width(string))
    }
}

impl Deref for UnicodeString {
    type Target = sizing::VecSegment;

    fn deref(&self) -> &Self::Target { &self.vec_segment }
}

impl DerefMut for UnicodeString {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.vec_segment }
}

#[cfg(test)]
mod tests {
    use crate::{ChUnit,
                GraphemeClusterSegment,
                UnicodeStringExt as _,
                assert_eq2,
                ch,
                col,
                width};

    const TEST_STRING: &str = "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿.";

    #[test]
    fn test_unicode_string_ext() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();

        // Check overall sizes and counts on the `UnicodeString` struct.
        assert_eq2!(string_us.len(), 11);
        assert_eq2!(string_us.grapheme_cluster_segment_count, 11);
        assert_eq2!(string_us.byte_size, string.len());
        assert_eq2!(string_us.display_width, width(15));
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_grapheme_cluster_segment() {
        fn assert_segment(
            string: &str,
            seg: &GraphemeClusterSegment,
            start_byte_index: usize,
            unicode_width: ChUnit,
            logical_index: usize,
            byte_size: usize,
            match_against: &str,
        ) {
            let segment_string = seg.get_str(string);
            assert_eq2!(segment_string, match_against);
            assert_eq2!(seg.start_byte_index, ch(start_byte_index));
            assert_eq2!(seg.unicode_width, width(unicode_width));
            assert_eq2!(seg.logical_index, ch(logical_index));
            assert_eq2!(seg.byte_size, byte_size);
        }

        let string = TEST_STRING;
        let string_us = string.unicode_string();

        // Check the individual `GraphemeClusterSegment` structs.
        assert_segment(string, &string_us[00], 00, 01.into(), 00, 01, "H");
        assert_segment(string, &string_us[01], 01, 01.into(), 01, 01, "i");
        assert_segment(string, &string_us[02], 02, 01.into(), 02, 01, " ");
        assert_segment(string, &string_us[03], 03, 02.into(), 03, 04, "😃");
        assert_segment(string, &string_us[04], 07, 01.into(), 04, 01, " ");
        assert_segment(string, &string_us[05], 08, 02.into(), 05, 04, "📦");
        assert_segment(string, &string_us[06], 12, 01.into(), 06, 01, " ");
        assert_segment(string, &string_us[07], 13, 02.into(), 07, 08, "🙏🏽");
        assert_segment(string, &string_us[08], 21, 01.into(), 08, 01, " ");
        assert_segment(string, &string_us[09], 22, 02.into(), 09, 26, "👨🏾‍🤝‍👨🏿");
        assert_segment(string, &string_us[10], 48, 01.into(), 10, 01, ".");
    }

    #[rustfmt::skip]
    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_logical_index_tofro_display_col() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();

        // Spot check some individual grapheme clusters at logical indices (the previous test does this exhaustively).
        assert_eq2!(string_us.at_logical_index(00).unwrap().get_str(string), "H");
        assert_eq2!(string_us.at_logical_index(01).unwrap().get_str(string), "i");
        assert_eq2!(string_us.at_logical_index(10).unwrap().get_str(string), ".");

        // Convert display column to logical index.
        assert_eq2!(string_us.at_display_col_index(col(00)).unwrap().get_str(string), "H");
        assert_eq2!(string_us.at_display_col_index(col(01)).unwrap().get_str(string), "i");
        assert_eq2!(string_us.at_display_col_index(col(02)).unwrap().get_str(string), " ");
        assert_eq2!(string_us.at_display_col_index(col(03)).unwrap().get_str(string), "😃");
        assert_eq2!(string_us.at_display_col_index(col(04)).unwrap().get_str(string), "😃");
        assert_eq2!(string_us.at_display_col_index(col(05)).unwrap().get_str(string), " ");
        assert_eq2!(string_us.at_display_col_index(col(06)).unwrap().get_str(string), "📦");
        assert_eq2!(string_us.at_display_col_index(col(07)).unwrap().get_str(string), "📦");
        assert_eq2!(string_us.at_display_col_index(col(08)).unwrap().get_str(string), " ");
        assert_eq2!(string_us.at_display_col_index(col(09)).unwrap().get_str(string), "🙏🏽");
        assert_eq2!(string_us.at_display_col_index(col(10)).unwrap().get_str(string), "🙏🏽");
        assert_eq2!(string_us.at_display_col_index(col(11)).unwrap().get_str(string), " ");
        assert_eq2!(string_us.at_display_col_index(col(12)).unwrap().get_str(string), "👨🏾‍🤝‍👨🏿");
        assert_eq2!(string_us.at_display_col_index(col(13)).unwrap().get_str(string), "👨🏾‍🤝‍👨🏿");
        assert_eq2!(string_us.at_display_col_index(col(14)).unwrap().get_str(string), ".");

        // Spot check convert logical index to display column.
        assert_eq2!(string_us.logical_index_at_display_col_index(col(0)).unwrap(), 0); // "H"
        assert_eq2!(string_us.logical_index_at_display_col_index(col(1)).unwrap(), 1); // "i"
        assert_eq2!(string_us.logical_index_at_display_col_index(col(2)).unwrap(), 2); // " "
        assert_eq2!(string_us.logical_index_at_display_col_index(col(3)).unwrap(), 3); // "😃"
        assert_eq2!(string_us.logical_index_at_display_col_index(col(4)).unwrap(), 3); // (same as above)
        assert_eq2!(string_us.logical_index_at_display_col_index(col(5)).unwrap(), 4); // " "

        // Spot check convert display col to logical index.
        assert_eq2!(string_us.display_col_index_at_logical_index(0).unwrap(), col(0)); // "H"
        assert_eq2!(string_us.display_col_index_at_logical_index(1).unwrap(), col(1)); // "i"
        assert_eq2!(string_us.display_col_index_at_logical_index(2).unwrap(), col(2)); // " "
        assert_eq2!(string_us.display_col_index_at_logical_index(3).unwrap(), col(3)); // "😃"
        assert_eq2!(string_us.display_col_index_at_logical_index(4).unwrap(), col(5));
        // " "
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_truncate_to_fit_display_cols() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();

        assert_eq2! {string_us.truncate_end_to_fit_width(width(00)), ""};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(01)), "H"};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(02)), "Hi"};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(03)), "Hi "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(04)), "Hi "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(05)), "Hi 😃"};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(06)), "Hi 😃 "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(07)), "Hi 😃 "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(08)), "Hi 😃 📦"};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(09)), "Hi 😃 📦 "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(10)), "Hi 😃 📦 "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(11)), "Hi 😃 📦 🙏🏽"};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(12)), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(13)), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(14)), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿"};
        assert_eq2! {string_us.truncate_end_to_fit_width(width(15)), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_truncate_end_by_n_col() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();

        assert_eq2! {string_us.truncate_end_by_n_col(width(01)), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿"};
        assert_eq2! {string_us.truncate_end_by_n_col(width(02)), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(03)), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(04)), "Hi 😃 📦 🙏🏽"};
        assert_eq2! {string_us.truncate_end_by_n_col(width(05)), "Hi 😃 📦 "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(06)), "Hi 😃 📦 "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(07)), "Hi 😃 📦"};
        assert_eq2! {string_us.truncate_end_by_n_col(width(08)), "Hi 😃 "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(09)), "Hi 😃 "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(10)), "Hi 😃"};
        assert_eq2! {string_us.truncate_end_by_n_col(width(11)), "Hi "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(12)), "Hi "};
        assert_eq2! {string_us.truncate_end_by_n_col(width(13)), "Hi"};
        assert_eq2! {string_us.truncate_end_by_n_col(width(14)), "H"};
        assert_eq2! {string_us.truncate_end_by_n_col(width(15)), ""};
        assert_eq2! {string_us.truncate_end_by_n_col(width(16)), ""};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_truncate_start_by_n_col() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();

        assert_eq2! {string_us.truncate_start_by_n_col(width(01)), "i 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {string_us.truncate_start_by_n_col(width(02)), " 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {string_us.truncate_start_by_n_col(width(03)), "😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {string_us.truncate_start_by_n_col(width(04)), " 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {string_us.truncate_start_by_n_col(width(05)), " 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {string_us.truncate_start_by_n_col(width(06)), "📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string2_insert_at_display_col() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();
        assert_eq2!(string_us.display_width, width(15));

        // Insert "😃" at display col 1, just after `H`.
        let (new_string, display_width_of_inserted_chunk) =
            string_us.insert_chunk_at_display_col(col(1), "😃");

        let new_string = new_string.as_str();
        let new_string_us = new_string.unicode_string();

        assert_eq2! {display_width_of_inserted_chunk, width(2)};
        assert_eq2! {new_string_us.truncate_start_by_n_col(width(00)), "H😃i 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {new_string_us.truncate_start_by_n_col(width(01)), "😃i 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string2_delete_at_display_col() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();
        assert_eq2!(string_us.display_width, width(15));

        // Delete "i" at display col 1, just after `H`.
        let Some(new_string) = string_us.delete_char_at_display_col(col(1)) else {
            panic!("Failed to delete char");
        };

        let new_string = new_string.as_str();
        let new_string_us = new_string.unicode_string();

        assert_eq2! {new_string_us.display_width, width(14)};
        assert_eq2! {new_string_us.truncate_start_by_n_col(width(00)), "H 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {new_string_us.truncate_start_by_n_col(width(01)), " 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string2_split_at_display_col() {
        let string = TEST_STRING;
        let string_us = string.unicode_string();
        assert_eq2!(string_us.display_width, width(15));

        // Split at display col 4.
        let Some((lhs_string, rhs_string)) = string_us.split_at_display_col(col(4))
        else {
            panic!("Failed to split unicode string");
        };

        let lhs_us = lhs_string.as_str().unicode_string();
        let rhs_us = rhs_string.as_str().unicode_string();

        assert_eq2! {lhs_us.display_width, width(3)};
        assert_eq2! {rhs_us.display_width, width(12)};

        assert_eq2! {lhs_us.truncate_start_by_n_col(width(00)), "Hi "};
        assert_eq2! {rhs_us.truncate_start_by_n_col(width(00)), "😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }
}

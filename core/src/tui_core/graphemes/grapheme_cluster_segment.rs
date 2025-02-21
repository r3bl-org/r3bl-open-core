/*
 *   Copyright (c) 2024-2025 R3BL LLC
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
use crate::{ChUnit, ColIndex, ColWidth, usize};

/// # Grapheme Cluster Segment
///
/// Represents a segment of a grapheme cluster within a [UnicodeString].
///
/// 1. This struct does not allocate anything and is [Copy].
/// 2. The [UnicodeString] owns the memory and this struct is a "view" into parts of it,
///    where each part is a grapheme cluster, and each of them is represented by this
///    struct.
///
/// This struct provides information about a single grapheme cluster, including its byte
/// indices within the original string, its display width, its logical index within the
/// [UnicodeString], its byte size, and its starting display column index.
///
/// ## Fields
///
/// - `start_byte_index`: The starting byte index of the grapheme cluster within the
///   original string.
/// - `end_byte_index`: The ending byte index of the grapheme cluster within the original
///   string.
/// - `unicode_width`: The display width of the grapheme cluster, as calculated by
///   [unicode_width::UnicodeWidthChar].
/// - `logical_index`: The index of this grapheme cluster within the
///   [UnicodeString::vec_segment] vector.
/// - `byte_size`: The number of bytes this grapheme cluster occupies in the original
///   string.
/// - `start_display_col_index`: The display column index at which this grapheme cluster
///   starts in the original string.
///
/// ## Purpose
///
/// The `GraphemeClusterSegment` struct is used to efficiently represent and manipulate
/// grapheme clusters within a [UnicodeString]. It allows for easy access to the
/// underlying string slice, as well as information about its display width and position.
///
/// # UTF-8 is variable length encoding
///
/// Rust uses `UTF-8` to represent text in [String]. `UTF-8` is a variable width encoding,
/// so each character can take up a different number of bytes, between 1 and 4, and 1 byte
/// is 8 bits; this is why we use [Vec] of [u8] to represent a [String].
///
/// For example, the character `H` takes up 1 byte. `UTF-8` is also backward compatible
/// with `ASCII`, meaning that the first 128 characters (the ASCII characters) are
/// represented using the same single byte as in ASCII. So the character `H` is
/// represented by the same byte value in `UTF-8` as it is in `ASCII`. This is why `UTF-8`
/// is so popular, as it allows for the representation of all the characters in the
/// Unicode standard, while still being able to represent `ASCII` characters in the same
/// way.
///
/// A grapheme cluster is a user-perceived character. Grapheme clusters can take up many
/// more bytes, eg 4 bytes or 2 or 3, etc. Here are some examples:
/// - `😃` takes up 4 bytes.
/// - `📦` also takes up 4 bytes.
/// - `🙏🏽` takes up 4 bytes, but it is a compound grapheme cluster.
/// - `H` takes up only 1 byte.
///
/// Videos:
///
/// - [Live coding video on Rust String](https://youtu.be/7I11degAElQ?si=xPDIhITDro7Pa_gq)
/// - [UTF-8 encoding video](https://youtu.be/wIVmDPc16wA?si=D9sTt_G7_mBJFLmc)
///
/// ## Usage
///
/// This struct is primarily used internally by the [UnicodeString] struct. However, it
/// can also be used directly to access information about individual grapheme clusters.
///
/// ## Example
///
/// ```rust
/// use r3bl_core::{UnicodeString, ch, col, width};
/// let unicode_string = UnicodeString::new("📦🙏🏽");
/// if let Some(segment) = unicode_string.vec_segment.first() {
///     assert_eq!(segment.start_byte_index, ch(0));
///     assert_eq!(segment.end_byte_index, ch(4));
///     assert_eq!(segment.unicode_width, width(2));
///     assert_eq!(segment.logical_index, ch(0));
///     assert_eq!(segment.byte_size, 4);
///     assert_eq!(segment.start_display_col_index, col(0));
/// }
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, size_of::SizeOf)]
pub struct GraphemeClusterSegment {
    /// The start index (bytes), in the string slice, used to generate the [UnicodeString]
    /// that this grapheme cluster represents.
    pub start_byte_index: ChUnit,

    /// The end index (bytes), in the string slice, used to generate the [UnicodeString]
    /// that this grapheme cluster represents.
    pub end_byte_index: ChUnit,

    /// Display width of the grapheme cluster calculated using
    /// [unicode_width::UnicodeWidthChar]. The display width (aka `unicode_width`) may not
    /// the same as the byte size [Self::byte_size].
    pub unicode_width: ColWidth,

    /// The index of this entry in the [UnicodeString::vec_segment].
    pub logical_index: ChUnit,

    /// The number of bytes this grapheme cluster occupies in the original string slice.
    /// The display width, aka [Self::unicode_width], may not the same as the byte size.
    pub byte_size: usize,

    /// Display col index [ColIndex] (in the original string slice) at which this grapheme
    /// cluster starts. The "offset" in the name means that this is relative to the start
    /// of the original string slice.
    /// - It is used to determine whether a given display col index [ColIndex] is within
    ///   the bounds of this grapheme cluster or not, eg:
    ///   [UnicodeString::get_string_at_display_col_index()],
    ///   [UnicodeString::is_display_col_index_in_middle_of_grapheme_cluster], etc.
    /// - It is used to perform conversions to and from `logical_index` to
    ///   `start_display_col_index`. [UnicodeString::display_col_index_at_logical_index]
    pub start_display_col_index: ColIndex,
}

impl GraphemeClusterSegment {
    /// Get the string slice for the grapheme cluster segment. Closely related to
    /// [UnicodeString::get_str].
    pub fn get_str<'a>(&self, string: &'a str) -> &'a str {
        let start_index = usize(self.start_byte_index);
        let end_index = usize(self.end_byte_index);
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
        let start_index = crate::usize(seg.start_byte_index);
        let end_index = crate::usize(seg.end_byte_index);
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

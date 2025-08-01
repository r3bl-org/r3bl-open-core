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

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use super::SegIndex;
use crate::{ChUnit, ColIndex, ColWidth, Length, usize};

/// `Seg` represents a grapheme cluster segment within a [`crate::GCStringOwned`].
///
/// This struct is the bridge between the three types of indices used in Unicode text
/// handling. Each `Seg` contains all the information needed to convert between
/// [`ByteIndex`](crate::ByteIndex), [`SegIndex`], and [`ColIndex`].
///
/// A Unicode "grapheme" is a user-perceived character.
/// - For `UTF-8` encoded text, a grapheme can be a single byte or up to 4 bytes.
/// - A "grapheme cluster" can be multiple graphemes or code points or Unicode scalar
///   values.
///
/// - For example, the `😀` emoji is a single grapheme cluster, also represented by a
///   single code point.
/// - However, the `🙏🏽` emoji is a jumbo emoji that is an amalgamation of multiple code
///   points `'🙏' + '🏽'`.
/// - The single letter "A" (U+0041) is a grapheme cluster consisting of one code point.
/// - The letter "á" can be represented as a single code point (U+00E1) or as a
///   combination of "a" (U+0061) followed by a combining acute accent (U+0301). In the
///   latter case, the grapheme cluster is the combination of the two code points "a" +
///   "´".
/// - Emoji like "👨‍👩‍👧‍👦" (family) are often represented by a sequence of multiple code points
///   (in this case, several characters joined by zero-width joiners). The entire sequence
///   forms a single grapheme cluster.
///
/// Let's take the example of `🙏🏽`. This is a jumbo emoji that is an amalgamation of
/// multiple code points.
/// - If you use [`str::chars()`] to parse this, you would get two separate [char]: `'🙏'`
///   + `'🏽'`.
/// - However, [`unicode_segmentation::UnicodeSegmentation`] represents this as a single
///   grapheme cluster. This is why we use [`unicode_segmentation::UnicodeSegmentation`]
///   to handle grapheme clusters.
///
/// # Performance, memory latency, access, allocation
///
/// 1. This struct does not allocate anything and is [Copy].
/// 2. The [`crate::GCStringOwned`] owns the memory, and this struct is a "view" into
///    parts of it, where each part is a grapheme cluster, and each of them is represented
///    by this struct.
///
/// This struct provides information about a single grapheme cluster, including its byte
/// indices within the original string, its display width, its logical index within the
/// [`crate::GCStringOwned`], its byte size, and its starting display column index.
///
/// ## Fields and Their Relationship to Index Types
///
/// - `start_byte_index` & `end_byte_index`: Define the [`ByteIndex`](crate::ByteIndex)
///   range for this segment. These are used when converting from `ByteIndex` to
///   `SegIndex`.
/// - `seg_index`: The [`SegIndex`] of this segment. This is its position in the logical
///   sequence of grapheme clusters.
/// - `start_display_col_index`: The [`ColIndex`] where this segment begins on screen.
///   Combined with `display_width`, this defines the `ColIndex` range.
/// - `display_width`: The number of terminal columns this segment occupies (as
///   [`ColWidth`]).
/// - `bytes_size`: The size in bytes (for convenience, equals `end_byte_index` -
///   `start_byte_index`).
///
/// ## Purpose
///
/// The `Seg` struct efficiently represents and manipulates grapheme clusters within a
/// [`crate::GCStringOwned`]. It provides access to the underlying string slice along
/// with display width and position information. See the [module
/// documentation](crate::graphemes) for comprehensive information about UTF-8 encoding,
/// grapheme clusters, and the three types of indices used in this system.
///
/// ## Usage
///
/// This struct is primarily used internally by the [`crate::GCStringOwned`] struct.
/// However, it can also be used directly to access information about individual grapheme
/// clusters.
///
/// ## Example
///
/// ```
/// use r3bl_tui::{GCStringOwned, ch, col, width, seg_index};
/// let u_str: GCStringOwned = "📦🙏🏽".into();
/// if let Some(segment) = u_str.segments.first() {
///     assert_eq!(segment.start_byte_index, ch(0));
///     assert_eq!(segment.end_byte_index, ch(4));
///     assert_eq!(segment.display_width, width(2));
///     assert_eq!(segment.seg_index, seg_index(0));
///     assert_eq!(segment.bytes_size.as_usize(), 4);
///     assert_eq!(segment.start_display_col_index, col(0));
/// }
/// ```
///
/// See the [module docs](crate::graphemes) for
/// comprehensive information about Unicode handling, grapheme clusters, and the three
/// types of indices used in this system.
#[derive(Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct Seg {
    /// The start index (bytes), in the string slice, used to generate the
    /// [`crate::GCStringOwned`] that this grapheme cluster represents.
    pub start_byte_index: ChUnit,

    /// The end index (bytes), in the string slice, used to generate the
    /// [`crate::GCStringOwned`] that this grapheme cluster represents.
    pub end_byte_index: ChUnit,

    /// Display width of the grapheme cluster calculated using
    /// [`unicode_width::UnicodeWidthChar`]. The display width (aka `unicode_width`) may
    /// not be the same as the byte size [`Self::bytes_size`].
    pub display_width: ColWidth,

    /// The index of this entry in the [`crate::GCStringOwned::segments`].
    pub seg_index: SegIndex,

    /// The number of bytes this grapheme cluster occupies in the original string slice.
    /// The display width, aka [`Self::display_width`], may not be the same as the byte
    /// size.
    pub bytes_size: Length,

    /// Display col index [`ColIndex`] (in the original string slice) at which this
    /// grapheme cluster starts. The "offset" in the name means that this is relative
    /// to the start of the original string slice.
    /// - It is used to determine whether a given display col index [`ColIndex`] is
    ///   within the bounds of this grapheme cluster or not.
    pub start_display_col_index: ColIndex,
}

/// Pretty print for [`crate::Seg`] that is compact and easier to read. The default
/// implementation takes up too much space and makes it difficult to debug.
impl Debug for Seg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Seg[{s_i:>2}] ┆ ■□ byte: [{b_b:>2}, {b_e:>2}] size: {b_s} ┆ disp ⮼ col({d_i:>2}) ← width({d_w:>2}) →",
            s_i = **self.seg_index,
            b_b = *self.start_byte_index,
            b_e = *self.end_byte_index,
            b_s = self.bytes_size.as_usize(),
            d_i = **self.start_display_col_index,
            d_w = **self.display_width,
        )
    }
}

impl Display for Seg {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Seg[{}]", **self.seg_index)
    }
}

impl Seg {
    /// Get the string slice for the grapheme cluster segment. The `string` parameter is
    /// any type that can be converted into a `&str`, such as [`crate::GCStringOwned`].
    pub fn get_str<'a>(&self, arg_str: &'a (impl AsRef<str> + ?Sized)) -> &'a str {
        let str = arg_str.as_ref();
        let start_index = usize(self.start_byte_index);
        let end_index = usize(self.end_byte_index);
        &str[start_index..end_index]
    }
}

#[cfg(test)]
mod tests {
    use crate::{GCStringOwned, ch, col, seg_index, width};

    #[test]
    fn test_single_grapheme_cluster() {
        let grapheme_string: GCStringOwned = "📦".into();
        if let Some(segment) = grapheme_string.segments.first() {
            assert_eq!(segment.start_byte_index, ch(0));
            assert_eq!(segment.end_byte_index, ch(4));
            assert_eq!(segment.display_width, width(2));
            assert_eq!(segment.seg_index, seg_index(0));
            assert_eq!(segment.bytes_size.as_usize(), 4);
            assert_eq!(segment.start_display_col_index, col(0));
            assert_eq!(segment.get_str(&grapheme_string), "📦");
        }
    }

    #[test]
    fn test_multiple_grapheme_clusters() {
        let grapheme_string: GCStringOwned = "📦🙏🏽".into();
        assert_eq!(grapheme_string.segments.len(), 2);

        let segment1 = &grapheme_string.segments[0];
        assert_eq!(segment1.start_byte_index, ch(0));
        assert_eq!(segment1.end_byte_index, ch(4));
        assert_eq!(segment1.display_width, width(2));
        assert_eq!(segment1.seg_index, seg_index(0));
        assert_eq!(segment1.bytes_size.as_usize(), 4);
        assert_eq!(segment1.start_display_col_index, col(0));
        assert_eq!(segment1.get_str(&grapheme_string), "📦");

        let segment2 = &grapheme_string.segments[1];
        assert_eq!(segment2.start_byte_index, ch(4));
        assert_eq!(segment2.end_byte_index, ch(12));
        assert_eq!(segment2.display_width, width(2));
        assert_eq!(segment2.seg_index, seg_index(1));
        assert_eq!(segment2.bytes_size.as_usize(), 8);
        assert_eq!(segment2.start_display_col_index, col(2));
        assert_eq!(segment2.get_str(&grapheme_string), "🙏🏽");
    }

    #[test]
    #[allow(clippy::unicode_not_nfc)] // Intentionally testing decomposed form
    fn test_combining_grapheme_cluster() {
        let grapheme_string: GCStringOwned = "á".into(); // 'a' + combining acute accent
        if let Some(segment) = grapheme_string.segments.first() {
            assert_eq!(segment.start_byte_index, ch(0));
            assert_eq!(segment.end_byte_index, ch(3));
            assert_eq!(segment.display_width, width(1));
            assert_eq!(segment.seg_index, seg_index(0));
            assert_eq!(segment.bytes_size.as_usize(), 3);
            assert_eq!(segment.start_display_col_index, col(0));
            assert_eq!(segment.get_str(&grapheme_string), "á");
        }
    }

    #[test]
    fn test_seg_str() {
        let grapheme_string: GCStringOwned = "📦🙏🏽".into();
        if let Some(segment) = grapheme_string.segments.first() {
            assert_eq!(segment.get_str(&grapheme_string), "📦");
        }
    }
}

/*
 *   Copyright (c) 2025 R3BL LLC
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

use std::{borrow::Cow, fmt::Debug};

use smallstr::SmallString;
use smallvec::{Array, SmallVec};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::iterator::GCStringIterator;
use crate::{ChUnit, ColIndex, ColWidth, GCString, GetMemSize, InlineString,
            Seg, SegIndex, SegWidth, build_segments_for_str,
            calculate_display_width, ch, gc_string_common::GCStringData,
            gc_string_owned, seg_width, width};

/// `GCStringOwned` represents a [String] as a sequence of grapheme cluster segments.
/// 
/// This struct owns its string data and provides efficient access to grapheme clusters
/// through pre-computed segment metadata. See the [module documentation](crate::graphemes)
/// for comprehensive information about Unicode handling, grapheme clusters, and the three
/// types of indices used in this system.
///
/// # Key Design Notes
///
/// - **Ownership**: This struct owns its string data for performance reasons. Testing
///   showed that non-owning variants with external references were significantly slower
///   due to memory access latency.
/// - **Iterators**: Provides both `iter()` for `&str` segments and `seg_iter()` for 
///   detailed [`Seg`] metadata.
/// - **Deref**: Derefs to `SegmentArray` - note that `len()` returns grapheme count,
///   not display width.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct GCStringOwned {
    pub string: InlineString,
    pub segments: gc_string_owned_sizing::SegmentArray,
    pub display_width: ColWidth,
    pub bytes_size: ChUnit,
}

/// Static sizing information for the `GCStringOwned` struct. This is used to calculate
/// the stack size of the struct (before it is [`smallvec::SmallVec::spilled`] to the
/// heap, if it becomes necessary).
pub mod gc_string_owned_sizing {
    use super::{ColWidth, GCStringOwned, GetMemSize, Seg, SmallVec};

    pub type SegmentArray = SmallVec<[Seg; VEC_SEGMENT_SIZE]>;
    pub const VEC_SEGMENT_SIZE: usize = 28;

    impl GetMemSize for GCStringOwned {
        fn get_mem_size(&self) -> usize {
            let string_size = self.bytes_size.as_usize();
            let segments_size = self.segments.len() * std::mem::size_of::<Seg>();
            let display_width_field_size = std::mem::size_of::<ColWidth>();
            string_size + segments_size + display_width_field_size
        }
    }
}

mod iterator {
    use super::{GCStringIterator, GCStringOwned, Seg};

    impl GCStringOwned {
        /// This is used to get the [`Self::segments`] of the grapheme string. This is
        /// used for debugging and testing purposes, in addition to low level
        /// implementation of things (like rendering) in the `r3bl_tui` crate. If
        /// you don't care about these details and simply want a sequence of
        /// `&str`, then use the [`Self::iter`] method to get an iterator over the
        /// grapheme segments.
        pub fn seg_iter(&self) -> impl Iterator<Item = &Seg> { self.segments.iter() }

        /// Returns an iterator over the grapheme segments in the `GCStringOwned` as a
        /// sequence of `&str`. You don't have to worry about the [Seg] struct. If
        /// you care about the internal details, use the [`Self::seg_iter()`]
        /// method that returns an iterator over the [`Self::segments`].
        #[must_use]
        pub fn iter(&self) -> GCStringIterator<'_> { GCStringIterator::new(self) }

        /// Returns the segment at the given index.
        #[must_use]
        pub fn get_segment(&self, index: usize) -> Option<&str> {
            self.segments.get(index).map(|seg| seg.get_str(self))
        }
    }
}

/// This struct is returned by the methods in the [`GCStringOwned`] `at_display_col_index`
/// module.
///
/// It represents a slice of the original [`GCStringOwned`] and owns data. It is used to
/// represent segments of the original string that are returned as a result of various
/// computations, eg: `GCStringOwned::get_string_at_right_of()`, etc.
///
/// We need an owned struct (since we're returning a slice that is dropped by the function
/// that creates it, not as a result of mutation).
#[derive(PartialEq, Eq)]
pub struct SegStringOwned {
    /// The grapheme cluster slice, as a [`GCStringOwned`]. This is a copy of the slice
    /// from the original string.
    pub string: GCStringOwned,
    /// The display width of the slice.
    pub width: ColWidth,
    /// The display col index at which this grapheme cluster starts in the original
    /// string.
    pub start_at: crate::ColIndex,
}

mod seg_string_result_impl {
    use super::{Debug, GCStringOwned, Seg, SegStringOwned, gc_string_owned};

    /// Easily convert a [Seg] and a [`GCStringOwned`] into a [`SegStringOwned`].
    impl From<(Seg, &GCStringOwned)> for SegStringOwned {
        fn from((seg, gs): (Seg, &GCStringOwned)) -> SegStringOwned {
            SegStringOwned {
                string: gc_string_owned(seg.get_str(gs)),
                width: seg.display_width,
                start_at: seg.start_display_col_index,
            }
        }
    }

    /// Short and readable debug output for [`SegStringOwned`].
    impl Debug for SegStringOwned {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "SegString {{ str: {:?} ┆ width: {:?} ┆ starts_at_col: {:?} }}",
                self.string.string, self.width, self.start_at
            )
        }
    }
}

mod basic {
    use super::{Array, ChUnit, ColIndex, ColWidth, Cow, GCString, GCStringData,
                GCStringOwned, Seg, SegIndex, SegStringOwned, SegWidth, SmallString,
                UnicodeWidthChar, UnicodeWidthStr, build_segments_for_str,
                calculate_display_width, ch, gc_string_owned_sizing, seg_width,
                wide_segments::ContainsWideSegments, width};

    impl AsRef<str> for GCStringOwned {
        fn as_ref(&self) -> &str { &self.string }
    }

    impl std::ops::Deref for GCStringOwned {
        type Target = gc_string_owned_sizing::SegmentArray;

        fn deref(&self) -> &Self::Target { &self.segments }
    }

    impl std::ops::DerefMut for GCStringOwned {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.segments }
    }

    impl<A: Array<Item = u8>> From<SmallString<A>> for GCStringOwned {
        fn from(value: SmallString<A>) -> Self { GCStringOwned::new(value.as_str()) }
    }

    impl<A: Array<Item = u8>> From<&SmallString<A>> for GCStringOwned {
        fn from(value: &SmallString<A>) -> Self { GCStringOwned::new(value.as_str()) }
    }

    impl From<Cow<'_, str>> for GCStringOwned {
        fn from(value: Cow<'_, str>) -> Self { GCStringOwned::new(value) }
    }

    impl From<&str> for GCStringOwned {
        fn from(value: &str) -> Self { GCStringOwned::new(value) }
    }

    impl From<&&str> for GCStringOwned {
        fn from(value: &&str) -> Self { GCStringOwned::new(value) }
    }

    impl From<String> for GCStringOwned {
        fn from(value: String) -> Self { GCStringOwned::new(value) }
    }

    impl From<&String> for GCStringOwned {
        fn from(value: &String) -> Self { GCStringOwned::new(value) }
    }

    impl GCStringData for GCStringOwned {
        fn string_data(&self) -> &str { &self.string }

        fn segments_iter(&self) -> impl DoubleEndedIterator<Item = &Seg> {
            self.segments.iter()
        }

        fn display_width(&self) -> ColWidth { self.display_width }

        fn bytes_size(&self) -> ChUnit { self.bytes_size }

        fn segments_len(&self) -> usize { self.segments.len() }

        fn get_segment(&self, index: usize) -> Option<&Seg> { self.segments.get(index) }
    }

    impl GCString for GCStringOwned {
        type StringResult = SegStringOwned;

        fn len(&self) -> SegWidth { crate::gc_string_common::gc_len(self) }

        fn is_empty(&self) -> bool { crate::gc_string_common::gc_is_empty(self) }

        fn get_max_seg_index(&self) -> SegIndex {
            crate::gc_string_common::gc_get_max_seg_index(self)
        }

        fn get(&self, seg_index: impl Into<SegIndex>) -> Option<Seg> {
            crate::gc_string_common::gc_get(self, seg_index)
        }
        fn seg_iter(&self) -> Box<dyn Iterator<Item = &Seg> + '_> {
            Box::new(self.segments.iter())
        }

        fn iter(&self) -> Box<dyn Iterator<Item = Seg> + '_> {
            Box::new(self.segments.iter().copied())
        }

        fn as_str(&self) -> &str { crate::gc_string_common::gc_as_str(self) }

        fn display_width(&self) -> ColWidth {
            crate::gc_string_common::gc_display_width(self)
        }

        fn bytes_size(&self) -> ChUnit { crate::gc_string_common::gc_bytes_size(self) }

        fn contains_wide_segments(&self) -> ContainsWideSegments {
            crate::gc_string_common::gc_contains_wide_segments(self)
        }

        fn trunc_end_to_fit(&self, col_width: impl Into<ColWidth>) -> &str {
            crate::gc_string_common::gc_trunc_end_to_fit(self, col_width)
        }

        fn trunc_end_by(&self, col_width: impl Into<ColWidth>) -> &str {
            crate::gc_string_common::gc_trunc_end_by(self, col_width)
        }

        fn trunc_start_by(&self, col_width: impl Into<ColWidth>) -> &str {
            crate::gc_string_common::gc_trunc_start_by(self, col_width)
        }

        fn get_string_at(
            &self,
            col_index: impl Into<ColIndex>,
        ) -> Option<Self::StringResult> {
            self.get_string_at(col_index)
        }

        fn get_string_at_right_of(
            &self,
            col_index: impl Into<ColIndex>,
        ) -> Option<Self::StringResult> {
            self.get_string_at_right_of(col_index)
        }

        fn get_string_at_left_of(
            &self,
            col_index: impl Into<ColIndex>,
        ) -> Option<Self::StringResult> {
            self.get_string_at_left_of(col_index)
        }

        fn get_string_at_end(&self) -> Option<Self::StringResult> {
            self.get_string_at_end()
        }
    }

    impl GCStringOwned {
        /// Constructor function that creates a [`GCStringOwned`] from a string slice. The
        /// actual grapheme cluster segment parsing is done using
        /// [`unicode_segmentation::UnicodeSegmentation`]. This is far more sophisticated
        /// than just using [`str::chars()`]. And it handles grapheme cluster segments and
        /// not just code points / Unicode scalar values. This handles things like jumbo
        /// emoji like `🙏🏽`.
        pub fn new(arg_str: impl AsRef<str>) -> GCStringOwned {
            let str = arg_str.as_ref();

            // Use the extracted segment building function
            let segments = build_segments_for_str(str);
            let display_width = calculate_display_width(&segments);
            let bytes_size = ch(str.len());

            GCStringOwned {
                string: str.into(),
                segments,
                display_width,
                bytes_size,
            }
        }

        /// Returns the number of grapheme clusters in this grapheme string. This is the
        /// the same as the length of the [`Self::segments`].
        #[must_use]
        pub fn len(&self) -> SegWidth { self.segments.len().into() }

        #[must_use]
        pub fn is_empty(&self) -> bool { self.len() == seg_width(0) }

        /// Returns the maximum segment index of this grapheme string.
        #[must_use]
        pub fn get_max_seg_index(&self) -> SegIndex { self.len().convert_to_seg_index() }

        /// Utility function to calculate the display width of a character or string
        /// slice.
        pub fn width(arg_str: impl AsRef<str>) -> ColWidth {
            let str = arg_str.as_ref();
            width(UnicodeWidthStr::width(str))
        }

        #[must_use]
        pub fn width_char(c: char) -> ColWidth {
            let value = UnicodeWidthChar::width(c).unwrap_or(0);
            width(value)
        }

        /// Given the grapheme cluster segment index, return the corresponding [Seg]
        /// struct.
        ///
        /// The `index` argument can be different types like [`crate::ColIndex`] and
        /// [`crate::ByteIndex`], which can both be converted to [`SegIndex`] by
        /// [`std::ops::Add`]ing it to a [`GCStringOwned`].
        ///
        /// Here's a visual depiction of the different indices.
        ///
        /// *How it appears in the terminal (displayed)*:
        ///
        /// ```text
        /// R ╭──────────────╮
        /// 0 │Hi📦XelLo🙏🏽Bye│
        ///   ╰──────────────╯
        ///  DC01234567890123 : index (0 based)
        /// ```
        ///
        /// *Detailed breakdown*:
        ///
        /// ```text
        /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
        /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
        ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
        ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
        ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
        ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
        ///
        /// ❯ DC: display column index | DW: display width
        /// ❯ R: row index | SI: segment index
        /// ```
        pub fn get(&self, arg_seg_index: impl Into<SegIndex>) -> Option<Seg> {
            let index: SegIndex = arg_seg_index.into();
            self.segments.get(crate::usize(*index)).copied()
        }
    }
}


/// Methods for easily detecting wide segments in the grapheme string.
pub mod wide_segments {
    use super::{Debug, GCStringOwned, width};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ContainsWideSegments {
        Yes,
        No,
    }

    impl GCStringOwned {
        /// Checks if the `GCStringOwned` contains any wide segments. A wide segment is
        /// defined as a segment with a display width greater than 1, eg: `📦` or `🙏🏽`.
        #[must_use]
        pub fn contains_wide_segments(&self) -> ContainsWideSegments {
            if self.segments.iter().any(|seg| seg.display_width > width(1)) {
                ContainsWideSegments::Yes
            } else {
                ContainsWideSegments::No
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str;

    use super::*;
    use crate::{byte_index, col, seg_index, wide_segments::ContainsWideSegments};

    /// Helper function to create a [`SegString`] for testing. Keeps the width of the
    /// lines of code in each test to a minimum (for easier readability).
    fn ssr(
        arg_gc_string: impl Into<GCStringOwned>,
        width: ColWidth,
        start_at: ColIndex,
    ) -> SegStringOwned {
        SegStringOwned {
            string: arg_gc_string.into(),
            width,
            start_at,
        }
    }

    fn w(string: &str) -> ColWidth { GCStringOwned::width(string) }

    /// *How it appears in the terminal (displayed)*:
    ///
    /// ```text
    /// R ╭──────────────╮
    /// 0 │Hi📦XelLo🙏🏽Bye│
    ///   ╰──────────────╯
    ///  DC01234567890123 : index (0 based)
    /// ```
    ///
    /// *Detailed breakdown*:
    ///
    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    const TEST_STR: &str = "Hi📦XelLo🙏🏽Bye";

    #[test]
    fn test_sanity_of_test_str() {
        let gs = gc_string_owned(TEST_STR);

        assert!(!gs.is_empty());
        assert!(gs.contains_wide_segments() == ContainsWideSegments::Yes);

        /* max col index is 13, ie, width - 1 */
        assert_eq!(gs.display_width, width(14));
        assert_eq!(gs.display_width.convert_to_col_index(), col(13));

        /* max seg index is 11, len() - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(*gs.len() - ch(1)));
        assert_eq!(gs.get_max_seg_index(), gs.len().convert_to_seg_index());

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_insert_chunk_at_display_col() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            ("🚀", col(00), "🚀Hi📦XelLo🙏🏽Bye", w("🚀")),
            ("🚀", col(01), "H🚀i📦XelLo🙏🏽Bye", w("🚀")),
            ("🚀", col(02), "Hi🚀📦XelLo🙏🏽Bye", w("🚀")), /* `📦` is 2 display cols
                                                           * wide */
            ("🚀", col(03), "Hi🚀📦XelLo🙏🏽Bye", w("🚀")), /* `📦` is 2 display cols
                                                           * wide */
            ("🚀", col(04), "Hi📦🚀XelLo🙏🏽Bye", w("🚀")),
            ("🚀", col(05), "Hi📦X🚀elLo🙏🏽Bye", w("🚀")),
            ("🚀", col(06), "Hi📦Xe🚀lLo🙏🏽Bye", w("🚀")),
            ("🚀", col(07), "Hi📦Xel🚀Lo🙏🏽Bye", w("🚀")),
            ("🚀", col(08), "Hi📦XelL🚀o🙏🏽Bye", w("🚀")), /* `🙏🏽` is 2 display cols
                                                           * wide */
            ("🚀", col(09), "Hi📦XelLo🚀🙏🏽Bye", w("🚀")), /* `🙏🏽` is 2 display cols
                                                           * wide */
            ("🚀", col(10), "Hi📦XelLo🚀🙏🏽Bye", w("🚀")),
            ("🚀", col(11), "Hi📦XelLo🙏🏽🚀Bye", w("🚀")),
            ("🚀", col(12), "Hi📦XelLo🙏🏽B🚀ye", w("🚀")),
            ("🚀", col(13), "Hi📦XelLo🙏🏽By🚀e", w("🚀")),
            ("🚀", col(14), "Hi📦XelLo🙏🏽Bye🚀", w("🚀")), /* ▐ at the end */
            ("🚀", col(15), "Hi📦XelLo🙏🏽Bye🚀", w("🚀")), /* ❯ past the end */
            ("🚀", col(16), "Hi📦XelLo🙏🏽Bye🚀", w("🚀")), /* ❯ past the end */
        ];

        for (chunk, insert_at, expected_str, exp_chunk_width) in test_cases {
            let (actual_str, actual_width) = gs.insert_chunk_at_col(insert_at, chunk);
            assert_eq!(actual_str, expected_str);
            assert_eq!(actual_width, exp_chunk_width);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_delete_char_at_display_col() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (col(00), Some("i📦XelLo🙏🏽Bye".into())),
            (col(01), Some("H📦XelLo🙏🏽Bye".into())),
            (col(02), Some("HiXelLo🙏🏽Bye".into())), /* `📦` is 2 display cols wide */
            (col(03), Some("HiXelLo🙏🏽Bye".into())), /* `📦` is 2 display cols wide */
            (col(04), Some("Hi📦elLo🙏🏽Bye".into())),
            (col(05), Some("Hi📦XlLo🙏🏽Bye".into())),
            (col(06), Some("Hi📦XeLo🙏🏽Bye".into())),
            (col(07), Some("Hi📦Xelo🙏🏽Bye".into())),
            (col(08), Some("Hi📦XelL🙏🏽Bye".into())),
            (col(09), Some("Hi📦XelLoBye".into())), /* `🙏🏽` is 2 display cols wide */
            (col(10), Some("Hi📦XelLoBye".into())), /* `🙏🏽` is 2 display cols wide */
            (col(11), Some("Hi📦XelLo🙏🏽ye".into())),
            (col(12), Some("Hi📦XelLo🙏🏽Be".into())),
            (col(13), Some("Hi📦XelLo🙏🏽By".into())), /* ▐ at the end */
            (col(14), None),                         /* ❯ past the end */
            (col(15), None),                         /* ❯ past the end */
        ];

        for (col_index, exp_result) in test_cases {
            let result = gs.delete_char_at_col(col_index);
            assert_eq!(exp_result, result);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_split_at_display_col() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        let test_cases = [
            (col(0), Some(("".into(), "Hi📦XelLo🙏🏽Bye".into()))),
            (col(1), Some(("H".into(), "i📦XelLo🙏🏽Bye".into()))),
            (col(2), Some(("Hi".into(), "📦XelLo🙏🏽Bye".into()))), /* `📦` is 2 display
                                                                   * cols wide */
            (col(3), Some(("Hi".into(), "📦XelLo🙏🏽Bye".into()))), /* `📦` is 2 display
                                                                   * cols wide */
            (col(4), Some(("Hi📦".into(), "XelLo🙏🏽Bye".into()))),
            (col(5), Some(("Hi📦X".into(), "elLo🙏🏽Bye".into()))),
            (col(6), Some(("Hi📦Xe".into(), "lLo🙏🏽Bye".into()))),
            (col(7), Some(("Hi📦Xel".into(), "Lo🙏🏽Bye".into()))),
            (col(8), Some(("Hi📦XelL".into(), "o🙏🏽Bye".into()))),
            (col(9), Some(("Hi📦XelLo".into(), "🙏🏽Bye".into()))), /* `🙏🏽` is 2 display
                                                                   * cols wide */
            (col(10), Some(("Hi📦XelLo".into(), "🙏🏽Bye".into()))), /* `🙏🏽` is 2 display cols wide */
            (col(11), Some(("Hi📦XelLo🙏🏽".into(), "Bye".into()))),
            (col(12), Some(("Hi📦XelLo🙏🏽B".into(), "ye".into()))),
            (col(13), Some(("Hi📦XelLo🙏🏽By".into(), "e".into()))), /* ▐ at the end */
            (col(14), None),                                       /* ❯ past the end */
            (col(15), None),                                       /* ❯ past the end */
            (col(16), None),                                       /* ❯ past the end */
        ];

        for (col_index, expected) in test_cases {
            let result = gs.split_at_display_col(col_index);
            assert_eq!(result, expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_get_string_at_end() {
        let test_cases = [
            (TEST_STR, Some(ssr("e", width(1), col(13)))),
            ("Hi", Some(ssr("i", width(1), col(1)))),
            ("H", Some(ssr("H", width(1), col(0)))),
            ("📦", Some(ssr("📦", width(2), col(0)))),
            ("🙏🏽", Some(ssr("🙏🏽", width(2), col(0)))),
            ("", None),
        ];

        for (input, expected) in test_cases {
            let gs = gc_string_owned(input);
            let end = gs.get_string_at_end();
            assert_eq!(end, expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_get_string_at_left_of_display_col_index() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (col(00), None),
            (col(01), Some(ssr("H", width(1), col(0)))),
            (col(02), Some(ssr("i", width(1), col(1)))),
            (col(03), Some(ssr("i", width(1), col(1)))), /* `📦` is 2 display cols
                                                          * wide */
            (col(04), Some(ssr("📦", width(2), col(2)))), /* `📦` is 2 display cols
                                                           * wide */
            (col(05), Some(ssr("X", width(1), col(4)))),
            (col(06), Some(ssr("e", width(1), col(5)))),
            (col(07), Some(ssr("l", width(1), col(6)))),
            (col(08), Some(ssr("L", width(1), col(7)))),
            (col(09), Some(ssr("o", width(1), col(8)))),
            (col(10), Some(ssr("o", width(1), col(8)))), /* `🙏🏽` is 2 display cols
                                                          * wide */
            (col(11), Some(ssr("🙏🏽", width(2), col(9)))), /* `🙏🏽` is 2 display cols
                                                           * wide */
            (col(12), Some(ssr("B", width(1), col(11)))),
            (col(13), Some(ssr("y", width(1), col(12)))), /* ▐ max display width
                                                           * required by line */
            /* ❯ No "e" at the end */
            (col(14), None), /* ❯ exceeding display width */
            (col(15), None), /* ❯ exceeding display width */
            (col(16), None), /* ❯ exceeding display width */
        ];

        for (display_col_index, expected) in test_cases {
            let at_left = gs.get_string_at_left_of(display_col_index);
            assert_eq!(at_left, expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_get_string_at_right_of() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            /* ← No "H" at the start */
            (col(00), Some(ssr("i", width(1), col(1)))),
            (col(01), Some(ssr("📦", width(2), col(2)))),
            (col(02), Some(ssr("X", width(1), col(4)))), /* `📦` is 2 display cols
                                                          * wide */
            (col(03), Some(ssr("X", width(1), col(4)))), /* `📦` is 2 display cols
                                                          * wide */
            (col(04), Some(ssr("e", width(1), col(5)))),
            (col(05), Some(ssr("l", width(1), col(6)))),
            (col(06), Some(ssr("L", width(1), col(7)))),
            (col(07), Some(ssr("o", width(1), col(8)))),
            (col(08), Some(ssr("🙏🏽", width(2), col(9)))),
            (col(09), Some(ssr("B", width(1), col(11)))), /* `🙏🏽` is 2 display cols
                                                           * wide */
            (col(10), Some(ssr("B", width(1), col(11)))), /* `🙏🏽` is 2 display cols
                                                           * wide */
            (col(11), Some(ssr("y", width(1), col(12)))),
            (col(12), Some(ssr("e", width(1), col(13)))),
            (col(13), None), /* ▐ max display width required by line */
            (col(14), None), /* ❯ exceeding display width */
            (col(15), None), /* ❯ exceeding display width */
            (col(16), None), /* ❯ exceeding display width */
        ];

        for (display_col_index, expected) in test_cases {
            let result = gs.get_string_at_right_of(display_col_index);
            assert_eq!(result, expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_in_middle_of_cluster() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        let test_cases = [
            (col(0), None),
            (col(1), None),
            (col(2), None),
            (col(3), gs.get(seg_index(2))), /* `📦` is 2 display cols wide */
            (col(4), None),
            (col(5), None),
            (col(6), None),
            (col(7), None),
            (col(8), None),
            (col(9), None),
            (col(10), gs.get(seg_index(8))), /* `🙏🏽` is 2 display cols wide */
            (col(11), None),
            (col(12), None),
            (col(13), None), /* ▐ max display width required by line */
            (col(14), None), /* ❯ exceeding display width */
            (col(15), None), /* ❯ exceeding display width */
            (col(16), None), /* ❯ exceeding display width */
        ];

        for (col_index, expected) in test_cases {
            let seg = gs.check_is_in_middle_of_grapheme(col_index);
            assert_eq!(seg, expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_get_string_at_col() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        let test_cases = [
            (col(0), Some(("H", width(1), col(0)))),
            (col(1), Some(("i", width(1), col(1)))),
            (col(2), Some(("📦", width(2), col(2)))), /* `📦` is 2 display cols wide */
            (col(3), None),                           /* `📦` is 2 display cols wide */
            (col(4), Some(("X", width(1), col(4)))),
            (col(5), Some(("e", width(1), col(5)))),
            (col(6), Some(("l", width(1), col(6)))),
            (col(7), Some(("L", width(1), col(7)))),
            (col(8), Some(("o", width(1), col(8)))),
            (col(9), Some(("🙏🏽", width(2), col(9)))), /* `🙏🏽` is 2 display cols wide */
            (col(10), None),                          /* `🙏🏽` is 2 display cols wide */
            (col(11), Some(("B", width(1), col(11)))),
            (col(12), Some(("y", width(1), col(12)))),
            (col(13), Some(("e", width(1), col(13)))), /* ▐ max display width required
                                                        * by line */
            (col(14), None), /* ❯ exceeding display width */
            (col(15), None), /* ❯ exceeding display width */
            (col(16), None), /* ❯ exceeding display width */
        ];

        for (given_display_col, expected) in test_cases {
            let result = gs.get_string_at(given_display_col);
            match expected {
                Some((exp_str, exp_width, exp_col)) => {
                    let result = result.unwrap();
                    assert_eq!(result.string, gc_string_owned(exp_str));
                    assert_eq!(result.width, exp_width);
                    assert_eq!(result.start_at, exp_col);
                }
                None => assert!(result.is_none()),
            }
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_clip() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        // cspell:disable
        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (col(00), width(00), ""),
            (col(00), width(01), "H"),
            (col(00), width(02), "Hi"),
            (col(00), width(03), "Hi"), /* `📦` is 2 display cols wide */
            (col(00), width(04), "Hi📦"), /* `📦` is 2 display cols wide */
            (col(00), width(05), "Hi📦X"),
            (col(00), width(06), "Hi📦Xe"),
            (col(00), width(07), "Hi📦Xel"),
            (col(00), width(08), "Hi📦XelL"),
            (col(00), width(09), "Hi📦XelLo"), /* `🙏🏽` is 2 display cols wide */
            (col(00), width(10), "Hi📦XelLo"), /* `🙏🏽` is 2 display cols wide */
            (col(00), width(11), "Hi📦XelLo🙏🏽"),
            (col(00), width(12), "Hi📦XelLo🙏🏽B"),
            (col(00), width(13), "Hi📦XelLo🙏🏽By"),
            (col(00), width(14), "Hi📦XelLo🙏🏽Bye"), /* ▐ max display width required by
                                                     * line */
            (col(00), width(15), "Hi📦XelLo🙏🏽Bye"), /* ❯ exceeding display width */
            (col(00), width(16), "Hi📦XelLo🙏🏽Bye"), /* ❯ exceeding display width */
        ];

        for (start_at, width, expected) in test_cases {
            let clipped_line = gs.clip(start_at, width);
            assert_eq!(clipped_line, expected);
        }
    }

    #[test]
    fn test_try_get_postfix_padding_for() {
        let gs = GCStringOwned::new("example");

        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (" ", 11, Some("    ".into())),
            (" ", 10, Some("   ".into())),
            (" ", 09, Some("  ".into())),
            (" ", 08, Some(" ".into())),
            (" ", 07, None),
            (" ", 06, None),
            (" ", 05, None),
            (" ", 04, None),
            (" ", 03, None),
            (" ", 02, None),
            (" ", 01, None),
            (" ", 00, None),
        ];

        for (spacer, width, expected) in test_cases {
            let padded_string = gs.try_get_postfix_padding_for(spacer, width);
            assert_eq!(padded_string, expected);
        }
    }

    #[test]
    fn test_pad_start_to_fit() {
        let gs = GCStringOwned::new("example");

        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (" ", 10, "   example"),
            (" ", 09, "  example"),
            (" ", 08, " example"),
            (" ", 07, "example"),
            (" ", 06, "example"),
            (" ", 05, "example"),
            (" ", 04, "example"),
            (" ", 03, "example"),
            (" ", 02, "example"),
            (" ", 01, "example"),
            (" ", 00, "example"),
        ];

        for (spacer, width, expected) in test_cases {
            let padded_string = gs.pad_start_to_fit(spacer, width);
            assert_eq!(padded_string, expected);
        }
    }

    #[test]
    fn test_pad_end_to_fit() {
        let gs = GCStringOwned::new("example");

        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (" ", 10, "example   "),
            (" ", 09, "example  "),
            (" ", 08, "example "),
            (" ", 07, "example"),
            (" ", 06, "example"),
            (" ", 05, "example"),
            (" ", 04, "example"),
            (" ", 03, "example"),
            (" ", 02, "example"),
            (" ", 01, "example"),
            (" ", 00, "example"),
        ];

        for (spacer, width, expected) in test_cases {
            let padded_string = gs.pad_end_to_fit(spacer, width);
            assert_eq!(&padded_string, expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_trunc_start() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        // cspell:disable
        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (width(00), "Hi📦XelLo🙏🏽Bye"),
            (width(01), "i📦XelLo🙏🏽Bye"),
            (width(02), "📦XelLo🙏🏽Bye"),
            (width(03), "XelLo🙏🏽Bye"), /* `📦` is 2 display cols wide */
            (width(04), "XelLo🙏🏽Bye"), /* `📦` is 2 display cols wide */
            (width(05), "elLo🙏🏽Bye"),
            (width(06), "lLo🙏🏽Bye"),
            (width(07), "Lo🙏🏽Bye"),
            (width(08), "o🙏🏽Bye"),
            (width(09), "🙏🏽Bye"),
            (width(10), "Bye"), /* `🙏🏽` is 2 display cols wide */
            (width(11), "Bye"), /* `🙏🏽` is 2 display cols wide */
            (width(12), "ye"),
            (width(13), "e"),
            (width(14), ""), /* ▐ max display width required by line */
            (width(15), ""), /* ❯ exceeding display width */
            (width(16), ""), /* ❯ exceeding display width */
            (width(17), ""), /* ❯ exceeding display width */
        ];
        // cspell::enable

        for (input_width, expected) in &test_cases {
            let truncated_line = gs.trunc_start_by(*input_width);
            assert_eq!(truncated_line, *expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_trunc_end() {
        let gs = gc_string_owned(TEST_STR);

        assert_eq!(w("📦"), width(2));
        assert_eq!(w("🙏🏽"), width(2));
        assert_eq!(w(TEST_STR), width(14)); /* max col index is 13, width - 1 */
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.get_max_seg_index(), seg_index(11));

        // cspell:disable
        #[allow(clippy::zero_prefixed_literal)]
        let test_cases = [
            (width(00), ""),
            (width(01), "H"),
            (width(02), "Hi"), /* `📦` is 2 display cols wide */
            (width(03), "Hi"), /* `📦` is 2 display cols wide */
            (width(04), "Hi📦"),
            (width(05), "Hi📦X"),
            (width(06), "Hi📦Xe"),
            (width(07), "Hi📦Xel"),
            (width(08), "Hi📦XelL"),
            (width(09), "Hi📦XelLo"), /* `🙏🏽` is 2 display cols wide */
            (width(10), "Hi📦XelLo"), /* `🙏🏽` is 2 display cols wide */
            (width(11), "Hi📦XelLo🙏🏽"),
            (width(12), "Hi📦XelLo🙏🏽B"),
            (width(13), "Hi📦XelLo🙏🏽By"),
            (width(14), "Hi📦XelLo🙏🏽Bye"), /* ▐ max display width required by line */
            (width(15), "Hi📦XelLo🙏🏽Bye"), /* ❯ exceeding display width */
            (width(16), "Hi📦XelLo🙏🏽Bye"), /* ❯ exceeding display width */
            (width(17), "Hi📦XelLo🙏🏽Bye"), /* ❯ exceeding display width */
        ];
        // cspell::enable

        for (input_width, expected) in &test_cases {
            let truncated_line = gs.trunc_end_to_fit(*input_width);
            assert_eq!(truncated_line, *expected);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_add_grapheme_string_and_col_index() {
        let gs = gc_string_owned(TEST_STR);

        // println!("{grapheme_string:#?}");

        assert_eq!("📦".len(), 4);

        #[allow(clippy::zero_prefixed_literal)]
        let valid_indices = [
            (col(00), seg_index(00), "H"),
            (col(01), seg_index(01), "i"),
            (col(02), seg_index(02), "📦"),
            (col(03), seg_index(02), "📦"),
            (col(04), seg_index(03), "X"),
            (col(05), seg_index(04), "e"),
            (col(06), seg_index(05), "l"),
            (col(07), seg_index(06), "L"),
            (col(08), seg_index(07), "o"),
            (col(09), seg_index(08), "🙏🏽"),
            (col(10), seg_index(08), "🙏🏽"),
            (col(11), seg_index(09), "B"),
            (col(12), seg_index(10), "y"),
            (col(13), seg_index(11), "e"),
        ];

        for (given_col_idx, exp_seg_idx, exp_str) in valid_indices {
            let result = (&gs + given_col_idx).unwrap();
            assert_eq!(result, exp_seg_idx);
            assert_eq!(gs.get(result).unwrap().get_str(&gs), exp_str);
        }

        let out_of_bounds_indices = [14, 15, 16, 17];
        for &index in &out_of_bounds_indices {
            assert_eq!((&gs + col(index)), None);
        }
    }

    /// ```text
    /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
    /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
    ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
    ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
    ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
    ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
    ///
    /// ❯ DC: display column index | DW: display width
    /// ❯ R: row index | SI: segment index
    /// ```
    #[test]
    fn test_add_grapheme_string_and_byte_index() {
        let gs = gc_string_owned(TEST_STR);

        // println!("{grapheme_string:#?}");

        assert_eq!("📦".len(), 4);

        #[allow(clippy::zero_prefixed_literal)]
        let valid_indices = [
            (byte_index(00), seg_index(00), "H"),
            (byte_index(01), seg_index(01), "i"),
            (byte_index(02), seg_index(02), "📦"),
            (byte_index(03), seg_index(02), "📦"),
            (byte_index(04), seg_index(02), "📦"),
            (byte_index(05), seg_index(02), "📦"),
            (byte_index(06), seg_index(03), "X"),
            (byte_index(07), seg_index(04), "e"),
            (byte_index(08), seg_index(05), "l"),
            (byte_index(09), seg_index(06), "L"),
            (byte_index(10), seg_index(07), "o"),
            (byte_index(11), seg_index(08), "🙏🏽"),
            (byte_index(12), seg_index(08), "🙏🏽"),
            (byte_index(13), seg_index(08), "🙏🏽"),
            (byte_index(14), seg_index(08), "🙏🏽"),
            (byte_index(15), seg_index(08), "🙏🏽"),
            (byte_index(16), seg_index(08), "🙏🏽"),
            (byte_index(17), seg_index(08), "🙏🏽"),
            (byte_index(18), seg_index(08), "🙏🏽"),
            (byte_index(19), seg_index(09), "B"),
            (byte_index(20), seg_index(10), "y"),
            (byte_index(21), seg_index(11), "e"),
        ];

        for (given_byte_idx, exp_seg_idx, exp_str) in valid_indices {
            let result = (&gs + given_byte_idx).unwrap();
            assert_eq!(result, seg_index(exp_seg_idx));
            assert_eq!(gs.get(result).unwrap().get_str(&gs), exp_str);
        }

        let out_of_bounds_indices = [22, 23, 24, 25];
        for &index in &out_of_bounds_indices {
            assert_eq!((&gs + byte_index(index)), None);
        }
    }

    #[test]
    fn test_add_grapheme_string_and_seg_index() {
        let gs = gc_string_owned(TEST_STR);

        let test_cases = [
            (seg_index(0), Some(col(0))),
            (seg_index(1), Some(col(1))),
            (seg_index(2), Some(col(2))),
            (seg_index(3), Some(col(4))),
            (seg_index(4), Some(col(5))),
            (seg_index(5), Some(col(6))),
            (seg_index(6), Some(col(7))),
            (seg_index(7), Some(col(8))),
            (seg_index(8), Some(col(9))),
            (seg_index(9), Some(col(11))),
            (seg_index(10), Some(col(12))),
            (seg_index(11), Some(col(13))),
            (seg_index(12), None),
            (seg_index(13), None),
        ];

        for (seg_idx, expected_col_idx) in test_cases {
            let display_col_index = &gs + seg_idx;
            assert_eq!(display_col_index, expected_col_idx);
        }
    }

    #[test]
    fn test_get_at_seg_index() {
        let gs = gc_string_owned(TEST_STR);
        for (i, seg) in gs.seg_iter().enumerate() {
            assert_eq!(gs.get(seg_index(i)), Some(*seg));
        }
    }

    #[test]
    fn test_unicode_width() {
        assert_eq!(w("a"), width(1));
        assert_eq!(w("😀"), width(2));
        assert_eq!(w("a"), width(1));
        assert_eq!(w("😀"), width(2));

        assert_eq!(GCStringOwned::width_char('a'), width(1));
        assert_eq!(GCStringOwned::width_char('😀'), width(2));
        assert_eq!(GCStringOwned::width_char('\0'), width(0)); /* null char is 0 width */
    }

    #[test]
    fn test_contains_wide_segments() {
        let test_cases = [
            (TEST_STR, ContainsWideSegments::Yes),
            ("Foo📦Bar", ContainsWideSegments::Yes),
            ("FooBarBaz", ContainsWideSegments::No),
        ];

        for (input, expected) in &test_cases {
            let gs = gc_string_owned(input);
            assert_eq!(gs.contains_wide_segments(), *expected);
        }
    }

    #[test]
    fn test_len_and_fields() {
        let gs = gc_string_owned(TEST_STR);
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.display_width, width(14));
        assert!(!gs.is_empty());

        let gs = gc_string_owned("");
        assert_eq!(gs.len(), seg_width(0));
        assert_eq!(gs.display_width, width(0));
        assert!(gs.is_empty());

        let gs = gc_string_owned("a");
        println!("{gs:#?}");
        assert_eq!(gs.len(), seg_width(1));
        assert_eq!(gs.display_width, width(1));
        assert!(!gs.is_empty());
    }
}

#[cfg(test)]
mod bench {
    extern crate test;
    use test::Bencher;

    use super::*;

    /// Benchmark: Creating `GCStringOwned` from ASCII text (no grapheme segmentation
    /// needed)
    #[bench]
    fn bench_gc_string_new_ascii_short(b: &mut Bencher) {
        let text = "Hello, world!";
        b.iter(|| {
            let _gs = GCStringOwned::new(text);
        });
    }

    /// Benchmark: Creating `GCStringOwned` from longer ASCII text
    #[bench]
    fn bench_gc_string_new_ascii_long(b: &mut Bencher) {
        let text =
            "The quick brown fox jumps over the lazy dog. Lorem ipsum dolor sit amet.";
        b.iter(|| {
            let _gs = GCStringOwned::new(text);
        });
    }

    /// Benchmark: Creating `GCStringOwned` from Unicode text with simple characters
    #[bench]
    fn bench_gc_string_new_unicode_simple(b: &mut Bencher) {
        let text = "Hello, 世界! こんにちは";
        b.iter(|| {
            let _gs = GCStringOwned::new(text);
        });
    }

    /// Benchmark: Creating `GCStringOwned` from Unicode text with complex grapheme
    /// clusters
    #[bench]
    fn bench_gc_string_new_unicode_complex(b: &mut Bencher) {
        let text = "Hi📦XelLo🙏🏽Bye";
        b.iter(|| {
            let _gs = GCStringOwned::new(text);
        });
    }

    /// Benchmark: Creating `GCStringOwned` from text with many emoji
    #[bench]
    fn bench_gc_string_new_emoji_heavy(b: &mut Bencher) {
        let text = "😀😃😄😁😆😅😂🤣😊😇🙂🙃😉😌😍🥰😘😗😙😚";
        b.iter(|| {
            let _gs = GCStringOwned::new(text);
        });
    }

    /// Benchmark: Creating `GCStringOwned` from typical log message (mostly ASCII)
    #[bench]
    fn bench_gc_string_new_log_message(b: &mut Bencher) {
        let text = "main_event_loop → Startup 🎉";
        b.iter(|| {
            let _gs = GCStringOwned::new(text);
        });
    }
}

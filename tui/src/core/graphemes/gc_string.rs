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

use std::{fmt::Debug,
          ops::{Add, Deref, DerefMut}};

use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{ByteIndex, ChUnit, ColIndex, ColWidth, InlineString, InlineVecStr, Seg,
            SegIndex, SegWidth, ch, col, join, pad_fmt, seg_index, seg_width, usize,
            width};

/// `GCString` represents a [String] as a sequence of grapheme cluster segments, and *not*
/// just scalar values or single code points, like `🙏`. This is to provide support for
/// "jumbo" emoji like `🙏🏽` is represented by a [`super::Seg`].
///
/// A Unicode "grapheme" is a user-perceived character. For `UTF-8` encoded text, a
/// grapheme can be a single byte or up to 4 bytes. A "grapheme cluster" can be multiple
/// graphemes or code points or Unicode scalar values.
///
/// - For example, the `😀` emoji is a single grapheme cluster which is also represented
///   by a single code point.
/// - However, the `🙏🏽` emoji is a jumbo emoji which is a amalgamation of multiple code
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
/// So there's a discrepancy in the following:
///
/// - The number of bytes in a grapheme cluster in memory.
/// - The number of display columns a grapheme cluster occupies in the display output.
///
/// This results in the byte index, display column index, and grapheme segment index all
/// being different when working with characters that are > 128 in ASCII encoding.
///
/// Here's a table that summarizes the differences:
///
/// | Grapheme | Byte size | Display column width | Code point value |
/// | -------- | --------- | -------------------- | ---------------- |
/// | `a`      | 1         | 1                    | 97               |
/// | `😀`     | 4         | 2                    | 128512           |
///
/// # Grapheme cluster segments
///
/// Why not just use [`str::chars()`] to get the grapheme cluster segments?
/// [`str::chars()`] is not sufficient for handling grapheme clusters. It only handles
/// Unicode scalar values, or code points, which are not the same as grapheme clusters.
/// For example, the `😀` emoji is a single grapheme cluster which is also represented by
/// a single code point. In this case, [`str::chars()`] is ok to use.
///
/// Let's take the example of `🙏🏽`. This is a jumbo emoji which is a amalgamation of
/// multiple code points.
/// - [`str::chars()`] would represent it two separate [char]: `'🙏' + '🏽'`.
/// - However, [`unicode_segmentation::UnicodeSegmentation`] represents this as a single
///   grapheme cluster.
///
/// This is why we use [`unicode_segmentation::UnicodeSegmentation`] to handle grapheme
/// clusters.
///
/// # Display column index, and segment index
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
/// # Performance, memory latency, access, allocation
///
/// For performance reasons, the `GCString` struct owns the underlying string data. The
/// vector of grapheme cluster segments is stored separately to avoid unnecessary
/// allocations and copying. This design allows for efficient access to individual
/// grapheme clusters and their properties, such as display width and byte size.
///
/// We tried making a variant of `GCString` that does not own any data (the underlying
/// string) but this design produced much slower performance due to the need for repeated
/// dereferencing of the string data that was in a different location (non local to
/// `GCString`) via a different struct. This was unintuitive, as we were expecting the
/// lack of allocation to prove faster, but it turned out to be slower! Intuition around
/// performance is not reliable, and it is best to measure and test each design choice.
///
/// Having said this, the [`super::Seg`] struct is designed to be as lightweight as
/// possible, with only the necessary properties for representing a grapheme cluster. It
/// does not own any data and only stores references to the original string slice. This
/// does not impact performance significantly, due to the nature in which it is used. So
/// this design choice (no ownership and slicing into an existing struct) work for this
/// use case it does not work for the `GCString` struct.
///
/// # Iterators
///
/// There are two iterators. One for users of the struct, and another for use in a more
/// un-abstract way, usually for internal use by the `r3bl_tui` codebase.
///
/// 1. [`Self::iter`]: Returns an iterator over the grapheme segments in the `GCString`.
///    This iterator returns the `&str` segments in the order they appear in the
///    underlying string. This makes it easy to iterate over the segments as `&str`
///    without knowing about the [Seg] struct.
/// 2. [`Self::seg_iter`]: Returns an iterator over the grapheme segments in the
///    `GCString`. This iterator returns the [Seg] segments in the order they appear in
///    the underlying string. This makes it easy to iterate over the segments as [Seg] and
///    all the low level details on byte offsets, display width, etc.
///
/// # Features
///
/// - `GCString`: Struct for representing Unicode strings with grapheme cluster
///   segmentation.
/// - [`Self::new`]: A constructor function for creating a `GCString` from a string slice.
/// - [`Self::width`]: A utility function for calculating the display width of a string
///   slice.
///
/// # Traits
///
/// The `GCString` struct implements the following traits:
/// - `Deref`: For dereferencing `GCString` instances. When the `*` operator is used, the
///   underlying `SegmentArray` is returned. This is really important to note when using
///   `len()`, which will return the number of grapheme clusters and this is not the same
///   as the `display_width` of the `GCString`.
/// - `DerefMut`: For mutable dereferencing of `GCString` instances.
/// - `Default`: For creating a default `GCString` instance.
/// - `PartialEq`: For comparing two `GCString` instances for equality.
/// - `Eq`: For checking if two `GCString` instances are equal.
/// - `Hash`: For hashing `GCString` instances.
/// - `Clone`: For creating a copy of a `GCString` instance.
/// - `Debug`: For debugging `GCString` instances.
/// - `SizeOf`: For calculating the size of `GCString` instances.
///
/// # Dependencies
///
/// This module relies on the following external crates:
///
/// - [`unicode_segmentation::UnicodeSegmentation`]: For splitting strings into grapheme
///   clusters.
/// - [`unicode_width::UnicodeWidthStr`]: For calculating the display width of Unicode
///   characters.
///
/// # Example
///
/// ```
/// use r3bl_tui::graphemes::GCString;
///
/// let ustr = GCString::new("Hello, 世界");
/// println!("Display width: {it:?}", it = ustr.display_width);
///
/// let first_seg = ustr.first().unwrap();
/// let first_seq_str = first_seg.get_str(&ustr);
/// assert_eq!(first_seq_str, "H");
///
/// let as_str = ustr.as_ref();
/// assert_eq!(as_str, "Hello, 世界");
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct GCString {
    pub string: InlineString,
    pub segments: sizing::SegmentArray,
    pub display_width: ColWidth,
    pub bytes_size: ChUnit,
}

mod iterator {
    use super::{GCString, Seg};

    #[derive(Debug)]
    pub struct GCStringIterator<'a> {
        gc_string: &'a GCString,
        index: usize,
    }

    impl<'a> Iterator for GCStringIterator<'a> {
        type Item = &'a str;

        fn next(&mut self) -> Option<Self::Item> {
            match self.gc_string.get_segment(self.index) {
                Some(segment) => {
                    self.index += 1;
                    Some(segment)
                }
                None => None, // Stop iteration when `get_segment` returns `None`.
            }
        }
    }

    impl GCString {
        /// This is used to get the [`Self::segments`] of the grapheme string. This is
        /// used for debugging and testing purposes, in addition to low level
        /// implementation of things (like rendering) in the `r3bl_tui` crate. If
        /// you don't care about these details and simply want a sequence of
        /// `&str`, then use the [`Self::iter`] method to get an iterator over the
        /// grapheme segments.
        pub fn seg_iter(&self) -> impl Iterator<Item = &Seg> { self.segments.iter() }

        /// Returns an iterator over the grapheme segments in the `GCString` as a sequence
        /// of `&str`. You don't have to worry about the [Seg] struct. If you care about
        /// the internal details, use the [`Self::seg_iter()`] method that returns an
        /// iterator over the [`Self::segments`].
        #[must_use]
        pub fn iter(&self) -> GCStringIterator<'_> {
            GCStringIterator {
                gc_string: self,
                index: 0,
            }
        }

        /// Returns the segment at the given index.
        #[must_use]
        pub fn get_segment(&self, index: usize) -> Option<&str> {
            self.segments.get(index).map(|seg| seg.get_str(self))
        }
    }

    /// This implementation allows the [`GCString`] to be used in a for loop directly.
    impl<'a> IntoIterator for &'a GCString {
        type Item = &'a str;
        type IntoIter = GCStringIterator<'a>;

        fn into_iter(self) -> Self::IntoIter { self.iter() }
    }
}

#[cfg(test)]
mod tests_iterator {
    use super::*;

    #[test]
    fn test_iterator() {
        let gc_string = GCString::new("Hello, 世界🥞👨‍👩‍👧‍👦🙏🏽");
        let mut iter = gc_string.iter();

        assert_eq!(iter.next(), Some("H"));
        assert_eq!(iter.next(), Some("e"));
        assert_eq!(iter.next(), Some("l"));
        assert_eq!(iter.next(), Some("l"));
        assert_eq!(iter.next(), Some("o"));
        assert_eq!(iter.next(), Some(","));
        assert_eq!(iter.next(), Some(" "));
        assert_eq!(iter.next(), Some("世"));
        assert_eq!(iter.next(), Some("界"));
        assert_eq!(iter.next(), Some("🥞"));
        assert_eq!(iter.next(), Some("👨‍👩‍👧‍👦"));
        assert_eq!(iter.next(), Some("🙏🏽"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_into_iterator_implementation() {
        let gc_string = GCString::new("Hello, 世界🥞");

        // Test that we can use the GCString directly in a for loop (this is why
        // IntoIterator is needed!)
        let mut collected = Vec::new();
        for segment in &gc_string {
            collected.push(segment.to_string());
        }

        assert_eq!(collected.len(), 10);
        assert_eq!(collected[0], "H");
        assert_eq!(collected[1], "e");
        assert_eq!(collected[6], " ");
        assert_eq!(collected[7], "世");
        assert_eq!(collected[8], "界");
        assert_eq!(collected[9], "🥞");

        // Test using for loop with explicit into_iter() call
        let mut explicit_collected = Vec::new();
        for segment in &gc_string {
            explicit_collected.push(segment.to_string());
        }
        assert_eq!(collected, explicit_collected);

        // Test using for loop to find specific graphemes
        let mut found_emoji = false;
        for segment in &gc_string {
            if segment == "🥞" {
                found_emoji = true;
                break;
            }
        }
        assert!(found_emoji);

        // Test using for loop with enumerate to get indices
        for (index, segment) in (&gc_string).into_iter().enumerate() {
            match index {
                0 => assert_eq!(segment, "H"),
                1 => assert_eq!(segment, "e"),
                7 => assert_eq!(segment, "世"),
                8 => assert_eq!(segment, "界"),
                9 => assert_eq!(segment, "🥞"),
                _ => {} // Other segments are valid too
            }
        }

        // Test using for loop to count specific types of characters
        let mut ascii_count = 0;
        let mut unicode_count = 0;
        for segment in &gc_string {
            if segment.is_ascii() {
                ascii_count += 1;
            } else {
                unicode_count += 1;
            }
        }
        assert_eq!(ascii_count, 7); // "H", "e", "l", "l", "o", ",", " "
        assert_eq!(unicode_count, 3); // "世", "界", "🥞"

        // Compare with manual iter() usage (without for loop)
        let iter_results: Vec<_> = gc_string.iter().map(ToString::to_string).collect();
        assert_eq!(iter_results, collected);
    }
}

pub fn grapheme_string(arg_from: impl Into<GCString>) -> GCString { arg_from.into() }

/// Static sizing information for the `GCString` struct. This is used to calculate
/// the stack size of the struct (before it is [`smallvec::SmallVec::spilled`] to the
/// heap, if it becomes necessary).
mod sizing {
    use super::{ColWidth, GCString, Seg, SmallVec};
    use crate::GetMemSize;

    pub type SegmentArray = SmallVec<[Seg; VEC_SEGMENT_SIZE]>;
    const VEC_SEGMENT_SIZE: usize = 28;

    impl GetMemSize for GCString {
        fn get_mem_size(&self) -> usize {
            let string_size = self.bytes_size.as_usize();
            let segments_size = self.segments.len() * std::mem::size_of::<Seg>();
            let display_width_field_size = std::mem::size_of::<ColWidth>();
            string_size + segments_size + display_width_field_size
        }
    }
}

/// Fundamental methods for working with grapheme strings.
mod basic {
    use super::{ChUnit, ColWidth, Deref, DerefMut, GCString, Seg, SegIndex, SegWidth,
                UnicodeSegmentation, UnicodeWidthChar, UnicodeWidthStr, ch, col,
                seg_width, sizing, width};

    impl AsRef<str> for GCString {
        fn as_ref(&self) -> &str { &self.string }
    }

    impl Deref for GCString {
        type Target = sizing::SegmentArray;

        fn deref(&self) -> &Self::Target { &self.segments }
    }

    impl DerefMut for GCString {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.segments }
    }

    impl<S> From<&S> for GCString
    where
        S: AsRef<str> + ?Sized,
    {
        fn from(arg: &S) -> Self { Self::new(arg.as_ref()) }
    }

    impl GCString {
        /// Constructor function that creates a [`GCString`] from a string slice. The
        /// actual grapheme cluster segment parsing is done using
        /// [`unicode_segmentation::UnicodeSegmentation`]. This is far more sophisticated
        /// than just using [`str::chars()`]. And it handles grapheme cluster segments and
        /// not just code points / Unicode scalar values. This handles things like jumbo
        /// emoji like `🙏🏽`.
        pub fn new(arg_str: impl AsRef<str>) -> GCString {
            let str = arg_str.as_ref();

            // ASCII fast path: avoid expensive grapheme segmentation
            if str.is_ascii() {
                let len = str.len();
                let mut segments = sizing::SegmentArray::with_capacity(len);

                // For ASCII, each char is exactly 1 byte and 1 display width
                for (i, _c) in str.chars().enumerate() {
                    segments.push(Seg {
                        start_byte_index: ch(i),
                        end_byte_index: ch(i + 1),
                        display_width: width(1),
                        seg_index: i.into(),
                        bytes_size: 1,
                        start_display_col_index: col(ch(i)),
                    });
                }

                return GCString {
                    string: str.into(),
                    segments,
                    display_width: width(len),
                    bytes_size: ch(len),
                };
            }

            // Unicode path: use grapheme segmentation
            let mut total_byte_offset: ChUnit = ch(0);
            // This is used both for the width and display col index.
            let mut unicode_width_offset_acc: ChUnit = ch(0);

            // Actually create the grapheme cluster segments using
            // unicode_segmentation::UnicodeSegmentation.
            let iter = str.grapheme_indices(true).enumerate();

            let size = iter.clone().count();
            let mut unicode_string_segments = sizing::SegmentArray::with_capacity(size);

            for (grapheme_cluster_index, (byte_offset, grapheme_cluster_str)) in iter {
                let display_width = GCString::width(grapheme_cluster_str);
                unicode_string_segments.push(Seg {
                    start_byte_index: ch(byte_offset),
                    end_byte_index: ch(byte_offset) + ch(grapheme_cluster_str.len()),
                    display_width,
                    seg_index: grapheme_cluster_index.into(),
                    bytes_size: grapheme_cluster_str.len(),
                    start_display_col_index: col(unicode_width_offset_acc), // Used as ColIndex here.
                });
                unicode_width_offset_acc += *display_width;
                total_byte_offset = ch(byte_offset);
            }

            GCString {
                string: str.into(),
                segments: unicode_string_segments,
                display_width: width(unicode_width_offset_acc), /* Used as WidthColCount here. */
                bytes_size: if total_byte_offset > ch(0) {
                    /* size = byte_offset (index) + 1 */
                    total_byte_offset + 1
                } else {
                    total_byte_offset
                },
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
        /// [`std::ops::Add`]ing it to a [`GCString`].
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

/// Methods to make it easy to work with getting owned string (from slices) at a given
/// display col index.
pub mod at_display_col_index {
    use super::{ColIndex, GCString, Seg, SegString, ch, seg_index};

    impl GCString {
        /// If the given `display_col_index` falls in the middle of a grapheme cluster,
        /// then return the [Seg] at that `display_col_index`. Otherwise return [None].
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
        pub fn check_is_in_middle_of_grapheme(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<Seg> {
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            if col != seg.start_display_col_index {
                return Some(seg);
            }
            None
        }

        /// Return the string and display width of the grapheme cluster segment at the
        /// given `display_col_index`. If this `display_col_index` falls in the middle of
        /// a grapheme cluster, then return [None].
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
        pub fn get_string_at(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<SegString> {
            // Convert display_col_index to seg_index.
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;

            // Get the segment at seg_index.
            let seg = self.get(seg_index_at_col)?;
            let seg_start_at = seg.start_display_col_index;
            (col == seg_start_at).then(|| {
                // The display_col_index is at the start of a grapheme cluster 👍.
                (seg, self).into()
            })
        }

        /// Return the string at the right of the given `display_col_index`. If the
        /// `display_col_index` is at the end of the string, then return [None]. If the
        /// `display_col_index` is in the middle of a grapheme cluster, then return the
        /// grapheme cluster segment that includes that `display_col_index`.
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
        pub fn get_string_at_right_of(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<SegString> {
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            (seg.seg_index < self.get_max_seg_index()).then(|| {
                let right_neighbor_seg = self.get(*seg.seg_index + ch(1))?;
                Some((right_neighbor_seg, self).into())
            })?
        }

        /// Return the string at the left of the given `display_col_index`. If the
        /// `display_col_index` is at the start of the string, or past the end of the
        /// string, then return [None]. If the `display_col_index` is in the middle of a
        /// grapheme cluster, then return the grapheme cluster segment that includes that
        /// `display_col_index`.
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
        pub fn get_string_at_left_of(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<SegString> {
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            (seg.seg_index > seg_index(0)).then(|| {
                let left_neighbor_seg = self.get(*seg.seg_index - ch(1))?;
                Some((left_neighbor_seg, self).into())
            })?
        }

        /// Return the last grapheme cluster segment in the grapheme string.
        /// If the grapheme string is empty, then return [None].
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
        #[must_use]
        pub fn get_string_at_end(&self) -> Option<SegString> {
            let seg = self.last()?;
            Some((*seg, self).into())
        }
    }
}

/// This struct is returned by the methods in this module [`mod@at_display_col_index`].
///
/// It represents a slice of the original [`GCString`] and owns data. It is used to
/// represent segments of the original string that are returned as a result of various
/// computations, eg: `r3bl_core::GCString::get_string_at_right_of()`, etc.
///
/// We need an owned struct (since we're returning a slice that is dropped by the function
/// that creates it, not as a result of mutation).
#[derive(PartialEq, Eq)]
pub struct SegString {
    /// The grapheme cluster slice, as a [`GCString`]. This is a copy of the slice
    /// from the original string.
    pub string: GCString,
    /// The display width of the slice.
    pub width: ColWidth,
    /// The display col index at which this grapheme cluster starts in the original
    /// string.
    pub start_at: ColIndex,
}

mod seg_string_result_impl {
    use super::{Debug, GCString, Seg, SegString, grapheme_string};

    /// Easily convert a [Seg] and a [`GCString`] into a [`SegString`].
    impl From<(Seg, &GCString)> for SegString {
        fn from((seg, gs): (Seg, &GCString)) -> SegString {
            SegString {
                string: grapheme_string(seg.get_str(gs)),
                width: seg.display_width,
                start_at: seg.start_display_col_index,
            }
        }
    }

    /// Short and readable debug output for [`SegString`].
    impl Debug for SegString {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "SegString {{ str: {:?} ┆ width: {:?} ┆ starts_at_col: {:?} }}",
                self.string.string, self.width, self.start_at
            )
        }
    }
}

/// Convert between different types of indices. This unifies the API so that different
/// index types are all converted into [`SegIndex`] for use with this struct. Here's the
/// list:
/// - [`GCString`] + [`ByteIndex`] = [Option]<[`SegIndex`]>
/// - [`GCString`] + [`ColIndex`] = [Option]<[`SegIndex`]>
/// - [`GCString`] + [`SegIndex`] = [Option]<[`ColIndex`]>
///
/// # Why These Conversions Are Essential
///
/// These conversion operators are the heart of Unicode text handling in the editor:
///
/// 1. **`ByteIndex` → `SegIndex`**: When we have a byte position (e.g., from a file
///    offset or string slice operation), we need to find which grapheme cluster it
///    belongs to. This is crucial for ensuring we never split a multi-byte character.
///
/// 2. **`ColIndex` → `SegIndex`**: When the user clicks at a screen position or we need
///    to render at a specific column, we must find which grapheme cluster is at that
///    display position. This handles wide characters correctly.
///
/// 3. **`SegIndex` → `ColIndex`**: When we have a logical character position and need to
///    know where it appears on screen. This is used for cursor positioning and rendering.
///
/// # Examples
///
/// ```text
/// String: "a😀b"
///
/// ByteIndex 0 → SegIndex 0 (start of 'a')
/// ByteIndex 1 → SegIndex 1 (start of '😀')
/// ByteIndex 2 → None (middle of '😀' - invalid!)
/// ByteIndex 5 → SegIndex 2 (start of 'b')
///
/// ColIndex 0 → SegIndex 0 ('a' at column 0)
/// ColIndex 1 → SegIndex 1 ('😀' starts at column 1)
/// ColIndex 2 → SegIndex 1 ('😀' spans columns 1-2)
/// ColIndex 3 → SegIndex 2 ('b' at column 3)
/// ```
mod convert {
    use super::{Add, ByteIndex, ColIndex, GCString, SegIndex, seg_index, usize};

    /// Convert a `byte_index` to a `seg_index`.
    ///
    /// Try and convert a [`GCString`] + [`ByteIndex`] to a grapheme index [`SegIndex`].
    impl Add<ByteIndex> for &GCString {
        type Output = Option<SegIndex>;

        /// Find the grapheme cluster segment (index) that is at the `byte_index` of the
        /// underlying string.
        fn add(self, byte_index: ByteIndex) -> Self::Output {
            let byte_index = *byte_index;
            for seg in &self.segments {
                let start = usize(seg.start_byte_index);
                let end = usize(seg.end_byte_index);
                if byte_index >= start && byte_index < end {
                    return Some(seg.seg_index);
                }
            }
            None
        }
    }

    /// Convert a `display_col_index` to a `seg_index`.
    ///
    /// Try and convert a [`GCString`] + [`ColIndex`] (display column index) to a
    /// grapheme index [`SegIndex`].
    impl Add<ColIndex> for &GCString {
        type Output = Option<SegIndex>;

        /// Find the grapheme cluster segment (index) that can be displayed at the
        /// `display_col_index` of the terminal.
        fn add(self, display_col_index: ColIndex) -> Self::Output {
            self.segments
                .iter()
                .find(|seg| {
                    let seg_display_width = seg.display_width;
                    let seg_start = seg.start_display_col_index;
                    let seg_end = seg_start + seg_display_width;
                    /* is within segment */
                    display_col_index >= seg_start && display_col_index < seg_end
                })
                .map(|seg| seg_index(seg.seg_index))
        }
    }

    /// Convert a `seg_index` to `display_col_index`.
    ///
    /// Try and convert a [`GCString`] + [`SegIndex`] to a [`ColIndex`] (display column
    /// index).
    impl Add<SegIndex> for &GCString {
        type Output = Option<ColIndex>;

        /// Find the display column index that corresponds to the grapheme cluster segment
        /// at the `seg_index`.
        fn add(self, seg_index: SegIndex) -> Self::Output {
            self.get(seg_index).map(|seg| seg.start_display_col_index)
        }
    }
}

/// Methods for easily detecting wide segments in the grapheme string.
pub mod wide_segments {
    use super::{Debug, GCString, width};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ContainsWideSegments {
        Yes,
        No,
    }

    impl GCString {
        /// Checks if the `GCString` contains any wide segments. A wide segment is
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

/// Methods for easily truncating grapheme cluster segments (at the end) for common TUI
/// use cases.
pub mod trunc_end {
    use super::{ColWidth, GCString, ch, usize};

    impl GCString {
        /// Returns a string slice from `self.string` w/ the segments removed from the end
        /// of the string that don't fit in the given viewport width (which is 1 based,
        /// and not 0 based). Note that the character at `display_col_count` *index* is
        /// NOT included in the result; please see the example below.
        ///
        /// ```text
        ///   ⎧ 3 ⎫ : size (or "width" or "col count" or "count", 1 based)
        /// R ╭───╮
        /// 0 │fir│st second
        ///   ╰───╯
        ///   C012┋345678901 : index (0 based)
        /// ```
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
        pub fn trunc_end_to_fit(&self, arg_col_width: impl Into<ColWidth>) -> &str {
            let mut avail_cols: ColWidth = arg_col_width.into();
            let mut string_end_byte_index = 0;

            for seg in self.seg_iter() {
                let seg_display_width = seg.display_width;
                if avail_cols < seg_display_width {
                    break;
                }
                string_end_byte_index += seg.bytes_size;
                avail_cols -= seg_display_width;
            }

            &self.string[..string_end_byte_index]
        }

        /// Removes some number of segments from the end of the string so that `col_count`
        /// (width) is skipped.
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
        pub fn trunc_end_by(&self, arg_col_width: impl Into<ColWidth>) -> &str {
            let mut countdown_col_count: ColWidth = arg_col_width.into();
            let mut string_end_byte_index = ch(0);

            let rev_iter = self.segments.iter().rev();

            for seg in rev_iter {
                let seg_display_width = seg.display_width;
                string_end_byte_index = seg.start_byte_index;
                countdown_col_count -= seg_display_width;
                if *countdown_col_count == ch(0) {
                    // We are done skipping.
                    break;
                }
            }

            &self.string[..usize(string_end_byte_index)]
        }
    }
}

/// Methods for easily truncating grapheme cluster segments (from the start) for common
/// TUI use cases.
pub mod trunc_start {
    use super::{ColWidth, GCString, ch};

    impl GCString {
        /// Removes segments from the start of the string so that `col_count` (width) is
        /// skipped.
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
        pub fn trunc_start_by(&self, arg_col_width: impl Into<ColWidth>) -> &str {
            let mut skip_col_count: ColWidth = arg_col_width.into();
            let mut string_start_byte_index = 0;

            for segment in self.seg_iter() {
                let seg_display_width = segment.display_width;
                if *skip_col_count == ch(0) {
                    // We are done skipping.
                    break;
                }

                // Skip segment.unicode_width.
                skip_col_count -= seg_display_width;
                string_start_byte_index += segment.bytes_size;
            }

            &self.string[string_start_byte_index..]
        }
    }
}

/// Methods for easily padding grapheme cluster segments for common TUI use cases.
mod pad {
    use super::{ColWidth, GCString, InlineString, pad_fmt, width};

    impl GCString {
        /// Returns a new [`InlineString`] that is the result of padding `self.string` to
        /// fit the given width w/ the given spacer character.
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
        pub fn pad_end_to_fit(
            &self,
            arg_pad_str: impl AsRef<str>,
            arg_col_width: impl Into<ColWidth>,
        ) -> InlineString {
            let pad_str = arg_pad_str.as_ref();
            let max_display_width: ColWidth = arg_col_width.into();
            let pad_count = max_display_width - self.display_width;
            let self_str = self.string.as_str();

            if pad_count > width(0) {
                let mut acc = InlineString::from(self_str);
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                acc
            } else {
                self_str.into()
            }
        }

        pub fn pad_start_to_fit(
            &self,
            arg_pad_str: impl AsRef<str>,
            arg_col_width: impl Into<ColWidth>,
        ) -> InlineString {
            let pad_str = arg_pad_str.as_ref();
            let max_display_width: ColWidth = arg_col_width.into();
            let pad_count = max_display_width - self.display_width;
            let self_str = self.string.as_str();

            if pad_count > width(0) {
                let mut acc = InlineString::new();
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                acc.push_str(self_str);
                acc
            } else {
                self_str.into()
            }
        }

        /// If `self.string`'s display width is less than `display_width`, this returns a
        /// padding [`InlineString`] consisting of the `pad_str` repeated to make up the
        /// difference. Otherwise, if `self.string` is already as wide or wider than
        /// `display_width`, it returns `None`.
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
        pub fn try_get_postfix_padding_for(
            &self,
            arg_pad_str: impl AsRef<str>,
            arg_col_width: impl Into<ColWidth>,
        ) -> Option<InlineString> {
            // Pad the line to the max cols w/ spaces. This removes any "ghost" carets
            // that were painted in a previous render.
            let pad_str = arg_pad_str.as_ref();
            let max_display_width: ColWidth = arg_col_width.into();

            if self.display_width < max_display_width {
                let pad_count = max_display_width - self.display_width;
                let mut acc = InlineString::new();
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                Some(acc)
            } else {
                None
            }
        }
    }
}

/// Methods for easily clipping grapheme cluster segments for common TUI use cases.
mod clip {
    use super::{ColIndex, ColWidth, GCString, ch};

    impl GCString {
        /// Clip the content starting from `arg_start_at_col_index` and take as many
        /// columns as possible until `arg_col_width` is reached.
        ///
        /// # Arguments
        /// - `arg_start_at_col_index`: This an index value.
        /// - `arg_col_width`: The is not an index value, but a size or count value.
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
        pub fn clip(
            &self,
            arg_start_at_col_index: impl Into<ColIndex>,
            arg_col_width: impl Into<ColWidth>,
        ) -> &str {
            let start_display_col_index: ColIndex = arg_start_at_col_index.into();
            let max_display_col_count: ColWidth = arg_col_width.into();

            let string_start_byte_index = {
                let mut it = 0;
                let mut skip_col_count = start_display_col_index;
                for seg in self.seg_iter() {
                    let seg_display_width = seg.display_width;
                    // Skip scroll_offset_col_index columns.
                    if *skip_col_count == ch(0) {
                        // We are done skipping.
                        break;
                    }

                    // Skip segment.unicode_width.
                    skip_col_count -= seg_display_width;
                    it += seg.bytes_size;
                }
                it
            };

            let string_end_byte_index = {
                let mut it = 0;
                let mut avail_col_count = max_display_col_count;
                let mut skip_col_count = start_display_col_index;
                for seg in self.seg_iter() {
                    let seg_display_width = seg.display_width;
                    // Skip scroll_offset_col_index columns (again).
                    if *skip_col_count == ch(0) {
                        if avail_col_count < seg_display_width {
                            break;
                        }
                        it += seg.bytes_size;
                        avail_col_count -= seg_display_width;
                    } else {
                        // Skip segment.unicode_width.
                        skip_col_count -= seg_display_width;
                        it += seg.bytes_size;
                    }
                }
                it
            };

            &self.string[string_start_byte_index..string_end_byte_index]
        }
    }
}

/// Methods for easily modifying grapheme cluster segments for common TUI use cases.
mod mutate {
    use super::{ColIndex, ColWidth, GCString, InlineString, InlineVecStr, ch, join,
                seg_width, usize, width};

    impl GCString {
        /// Inserts the given `chunk` in the correct position of the `string`, and returns
        /// a new ([`InlineString`], [`ColWidth`]) tuple:
        /// 1. The new [`InlineString`] produced containing the inserted chunk.
        /// 2. The unicode width / display width of the inserted `chunk`.
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
        pub fn insert_chunk_at_col(
            &self,
            arg_col_index: impl Into<ColIndex>,
            arg_chunk: impl AsRef<str>,
        ) -> (InlineString, ColWidth) {
            let chunk = arg_chunk.as_ref();

            // Create an array-vec of &str from self.vec_segment, using self.iter().
            let mut vec = InlineVecStr::with_capacity(self.len().as_usize() + 1);
            // Add each seg's &str to the acc.
            vec.extend(
                // Turn self.segments into a list of &str.
                self.seg_iter().map(|seg| seg.get_str(&self.string)),
            );

            // Get seg_index at display_col_index.
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = self + col;

            match seg_index_at_col {
                // Insert somewhere inside bounds of self.string.
                Some(seg_index) => vec.insert(usize(*seg_index), chunk),
                // Add to end of self.string.
                None => vec.push(chunk),
            }

            // Generate a new InlineString from acc and return it and the unicode width of
            // the character.
            (
                join!(from: vec, each: item, delim: "", format: "{item}"),
                GCString::width(chunk),
            )
        }

        /// Returns a new [`InlineString`] that is the result of deleting the character at
        /// the given `display_col_index`.
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
        pub fn delete_char_at_col(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<InlineString> {
            // There is no segment present (Deref trait makes `len()` apply to
            // `vec_segment`).
            if self.is_empty() {
                return None;
            }

            // There is only one segment present.
            if self.len() == seg_width(1) {
                return Some("".into());
            }

            // There are more than 1 segments present.

            // Get seg_index at display_col_index.
            let col: ColIndex = arg_col_index.into();
            let split_seg_index = (self + col)?;
            let split_seg_index = usize(*split_seg_index);

            let mut vec_left = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_left_display_width = width(0);
            {
                for seg_index in 0..split_seg_index {
                    let seg = *self.segments.get(seg_index)?;
                    let string = seg.get_str(&self.string);
                    vec_left.push(string);
                    str_left_display_width += seg.display_width;
                }
            }

            let mut vec_right = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_right_display_width = width(0);
            {
                // Drop one segment.
                let max_seg_index = self.len();
                for seg_index in (split_seg_index + 1)..max_seg_index.as_usize() {
                    let seg = *self.segments.get(seg_index)?;
                    let string = seg.get_str(&self.string);
                    vec_right.push(string);
                    str_right_display_width += seg.display_width;
                }
            }

            // Merge the two vectors.
            vec_left.append(&mut vec_right);
            Some(join!(from: vec_left, each: it, delim: "", format: "{it}"))
        }

        /// Splits the string at the given `display_col_index` and returns a tuple of the
        /// left and right parts of the split. If the `display_col_index` falls in the
        /// middle of a grapheme cluster, then the split is done at the start of the
        /// cluster.
        ///
        /// Returns two new tuples:
        /// 1. *left* [`InlineString`],
        /// 2. *right* [`InlineString`].
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
        pub fn split_at_display_col(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<(InlineString, InlineString)> {
            // Get seg_index at display_col_index.
            let col: ColIndex = arg_col_index.into();
            let split_seg_index = (self + col)?;
            let split_seg_index = usize(*split_seg_index);

            let mut acc_left = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_left_display_width = width(0);
            {
                for seg_index in 0..split_seg_index {
                    let seg = *self.segments.get(seg_index)?;
                    acc_left.push(seg.get_str(&self.string));
                    str_left_display_width += seg.display_width;
                }
            }

            let mut acc_right = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_right_unicode_width = width(0);
            {
                let max_seg_index = self.len();
                for seg_idx in split_seg_index..max_seg_index.as_usize() {
                    let seg = *self.segments.get(seg_idx)?;
                    acc_right.push(seg.get_str(&self.string));
                    str_right_unicode_width += seg.display_width;
                }
            }

            (*str_right_unicode_width > ch(0) || *str_left_display_width > ch(0)).then(
                || {
                    (
                        join!(from: acc_left, each: it, delim: "", format: "{it}"),
                        join!(from: acc_right, each: it, delim: "", format: "{it}"),
                    )
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str;

    use super::*;
    use crate::{byte_index, gc_string::wide_segments::ContainsWideSegments};

    /// Helper function to create a [`SegString`] for testing. Keeps the width of the
    /// lines of code in each test to a minimum (for easier readability).
    fn ssr(
        arg_gc_string: impl Into<GCString>,
        width: ColWidth,
        start_at: ColIndex,
    ) -> SegString {
        SegString {
            string: arg_gc_string.into(),
            width,
            start_at,
        }
    }

    fn w(string: &str) -> ColWidth { GCString::width(string) }

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
            let gs = grapheme_string(input);
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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
                    assert_eq!(result.string, grapheme_string(exp_str));
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
        let gs = grapheme_string(TEST_STR);

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
        let gs = GCString::new("example");

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
        let gs = GCString::new("example");

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
        let gs = GCString::new("example");

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);

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
        let gs = grapheme_string(TEST_STR);
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

        assert_eq!(GCString::width_char('a'), width(1));
        assert_eq!(GCString::width_char('😀'), width(2));
        assert_eq!(GCString::width_char('\0'), width(0)); /* null char is 0 width */
    }

    #[test]
    fn test_contains_wide_segments() {
        let test_cases = [
            (TEST_STR, ContainsWideSegments::Yes),
            ("Foo📦Bar", ContainsWideSegments::Yes),
            ("FooBarBaz", ContainsWideSegments::No),
        ];

        for (input, expected) in &test_cases {
            let gs = grapheme_string(input);
            assert_eq!(gs.contains_wide_segments(), *expected);
        }
    }

    #[test]
    fn test_len_and_fields() {
        let gs = grapheme_string(TEST_STR);
        assert_eq!(gs.len(), seg_width(12));
        assert_eq!(gs.display_width, width(14));
        assert!(!gs.is_empty());

        let gs = grapheme_string("");
        assert_eq!(gs.len(), seg_width(0));
        assert_eq!(gs.display_width, width(0));
        assert!(gs.is_empty());

        let gs = grapheme_string("a");
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

    /// Benchmark: Creating `GCString` from ASCII text (no grapheme segmentation needed)
    #[bench]
    fn bench_gc_string_new_ascii_short(b: &mut Bencher) {
        let text = "Hello, world!";
        b.iter(|| {
            let _gs = GCString::new(text);
        });
    }

    /// Benchmark: Creating `GCString` from longer ASCII text
    #[bench]
    fn bench_gc_string_new_ascii_long(b: &mut Bencher) {
        let text =
            "The quick brown fox jumps over the lazy dog. Lorem ipsum dolor sit amet.";
        b.iter(|| {
            let _gs = GCString::new(text);
        });
    }

    /// Benchmark: Creating `GCString` from Unicode text with simple characters
    #[bench]
    fn bench_gc_string_new_unicode_simple(b: &mut Bencher) {
        let text = "Hello, 世界! こんにちは";
        b.iter(|| {
            let _gs = GCString::new(text);
        });
    }

    /// Benchmark: Creating `GCString` from Unicode text with complex grapheme clusters
    #[bench]
    fn bench_gc_string_new_unicode_complex(b: &mut Bencher) {
        let text = "Hi📦XelLo🙏🏽Bye";
        b.iter(|| {
            let _gs = GCString::new(text);
        });
    }

    /// Benchmark: Creating `GCString` from text with many emoji
    #[bench]
    fn bench_gc_string_new_emoji_heavy(b: &mut Bencher) {
        let text = "😀😃😄😁😆😅😂🤣😊😇🙂🙃😉😌😍🥰😘😗😙😚";
        b.iter(|| {
            let _gs = GCString::new(text);
        });
    }

    /// Benchmark: Creating `GCString` from typical log message (mostly ASCII)
    #[bench]
    fn bench_gc_string_new_log_message(b: &mut Bencher) {
        let text = "main_event_loop → Startup 🎉";
        b.iter(|| {
            let _gs = GCString::new(text);
        });
    }
}

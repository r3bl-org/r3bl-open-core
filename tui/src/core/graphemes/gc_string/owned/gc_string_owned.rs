// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`GCStringOwned`] implementation for owned Unicode grapheme cluster strings.

use crate::{ByteIndex, CCol, ChUnit, CowInlineString, GraphemeString, GraphemeStringMut,
            InlineString, InlineVecStr, NarrowingCastToU16, RangeExt, Seg, SegContent,
            SegIndex, SegLength, SegmentArray, VPCol, VPWidth, ch,
            graphemes::unicode_segment::{build_segments_for_str,
                                         calculate_display_width},
            join, seg_index, seg_length, usize, vp_width};
use std::fmt::{Debug, Display, Formatter};

/// Wide segments detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainsWideSegment {
    Yes,
    No,
}

/// Owned version of a Unicode grapheme cluster string with pre-computed segment metadata.
///
/// This type owns both the string data and the grapheme cluster metadata, making it
/// suitable for cases where the string needs to be stored or passed around independently.
#[derive(Clone, PartialEq, Eq)]
pub struct GCStringOwned {
    /// The underlying string data (owned).
    pub string: InlineString,
    /// Pre-computed grapheme cluster segments.
    pub segments: SegmentArray,
    /// Display width of the entire string.
    pub display_width: VPWidth,
    /// Byte size of the string.
    pub bytes_size: ChUnit,
}

impl Debug for GCStringOwned {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GCStringOwned({:?})", self.string.as_str())
    }
}

impl Display for GCStringOwned {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string.as_str())
    }
}

impl AsRef<str> for GCStringOwned {
    fn as_ref(&self) -> &str { self.string.as_str() }
}

impl From<&str> for GCStringOwned {
    fn from(value: &str) -> GCStringOwned { GCStringOwned::new(value) }
}

impl From<String> for GCStringOwned {
    fn from(value: String) -> GCStringOwned { GCStringOwned::new(value) }
}

impl From<InlineString> for GCStringOwned {
    fn from(value: InlineString) -> GCStringOwned { GCStringOwned::new(value.as_str()) }
}

impl<'a> IntoIterator for &'a GCStringOwned {
    type Item = Seg;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, Seg>>;

    fn into_iter(self) -> Self::IntoIter { self.segments.iter().copied() }
}

impl From<&InlineString> for GCStringOwned {
    fn from(value: &InlineString) -> GCStringOwned { GCStringOwned::new(value.as_str()) }
}

impl From<&'_ &str> for GCStringOwned {
    fn from(value: &'_ &str) -> GCStringOwned { GCStringOwned::new(*value) }
}

impl From<&String> for GCStringOwned {
    fn from(value: &String) -> GCStringOwned { GCStringOwned::new(value.as_str()) }
}

impl GCStringOwned {
    /// Creates a new [`GCStringOwned`] from a string, computing grapheme cluster
    /// segments.
    pub fn new(input: impl AsRef<str>) -> Self {
        let string: InlineString = input.as_ref().into();
        let segments = build_segments_for_str(string.as_str());
        let display_width = calculate_display_width(&segments);
        let bytes_size = ChUnit::from(string.len());

        Self {
            string,
            segments,
            display_width,
            bytes_size,
        }
    }

    /// Gets the string as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str { self.string.as_str() }

    /// Gets the display width of the string. Also see [`width()`].
    ///
    /// [`width()`]: Self::width
    #[must_use]
    pub fn display_width(&self) -> VPWidth { self.display_width }

    /// Gets the byte size of the string.
    #[must_use]
    pub fn bytes_size(&self) -> ChUnit { self.bytes_size }

    /// Gets the number of grapheme clusters.
    #[must_use]
    pub fn len(&self) -> SegLength { self.segments.len().as_u16_narrowing().into() }

    /// Gets the number of grapheme cluster segments. This is the preferred method for
    /// semantic clarity.
    #[must_use]
    pub fn segment_count(&self) -> SegLength {
        self.segments.len().as_u16_narrowing().into()
    }

    /// Checks if the string is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.segments.is_empty() }

    /// Gets a segment by index.
    pub fn get(&self, seg_index: impl Into<SegIndex>) -> Option<Seg> {
        let index = seg_index.into().as_usize();
        self.segments.get(index).copied()
    }

    /// Gets the maximum segment index.
    #[must_use]
    pub fn get_max_seg_index(&self) -> SegIndex {
        if self.segments.is_empty() {
            SegIndex::from(0u16)
        } else {
            SegIndex::from((self.segments.len() - 1).as_u16_narrowing())
        }
    }

    /// Iterates over segments.
    pub fn iter(&self) -> impl Iterator<Item = Seg> + '_ { self.segments.iter().copied() }

    /// Gets display width of a single character (utility method).
    #[must_use]
    pub fn width_char(ch: char) -> VPWidth {
        use unicode_width::UnicodeWidthChar;
        VPWidth::from(UnicodeWidthChar::width(ch).unwrap_or(0).as_u16_narrowing())
    }

    /// Checks if this string contains wide segments (characters wider than 1 column).
    #[must_use]
    pub fn contains_wide_segment(&self) -> ContainsWideSegment {
        if self
            .segments
            .iter()
            .any(|seg| seg.display_width > vp_width(1))
        {
            ContainsWideSegment::Yes
        } else {
            ContainsWideSegment::No
        }
    }

    /// Iterates over the segments.
    pub fn seg_iter(&self) -> impl Iterator<Item = Seg> + '_ {
        self.segments.iter().copied()
    }

    /// Gets the display width of the string (alias for [`display_width()`]).
    ///
    /// [`display_width()`]: Self::display_width
    #[must_use]
    pub fn width(&self) -> VPWidth { self.display_width }

    /// Gets the last segment.
    #[must_use]
    pub fn last(&self) -> Option<Seg> { self.segments.last().copied() }

    /// Gets the byte index at a given segment index.
    #[must_use]
    pub fn get_byte_index(&self, seg_index: impl Into<SegIndex>) -> Option<ByteIndex> {
        self.get(seg_index).map(|seg| seg.start_byte_index)
    }
}

/// Result type for string operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegStringOwned {
    /// The grapheme cluster slice as `GCStringOwned` (owns both string and segments).
    pub string: GCStringOwned,
    /// The display width of the slice.
    pub width: VPWidth,
    /// The display col index at which this grapheme cluster starts.
    pub start_at: VPCol,
}

// GraphemeString trait implementation for GCStringOwned.
impl GraphemeString for GCStringOwned {
    type SegmentIterator<'a> = std::iter::Copied<std::slice::Iter<'a, Seg>>;
    type StringSlice<'a> = CowInlineString<'a>;

    fn as_str(&self) -> &str { self.as_str() }

    fn segments(&self) -> &[Seg] { &self.segments }

    fn display_width(&self) -> VPWidth { self.display_width }

    fn segment_count(&self) -> SegLength { self.segment_count() }

    fn byte_size(&self) -> ChUnit { self.bytes_size }

    fn get_seg(&self, index: SegIndex) -> Option<Seg> { self.get(index) }

    fn check_is_in_middle_of_grapheme(&self, col: VPCol) -> Option<Seg> {
        self.check_is_in_middle_of_grapheme(col)
    }

    fn get_seg_at(&self, col: VPCol) -> Option<SegContent<'_>> {
        let seg_string = self.get_string_at(col)?;
        let seg = self
            .segments
            .iter()
            .find(|seg| seg.start_display_col_index == seg_string.start_at)?;
        Some(SegContent {
            content: seg.get_str(self),
            seg: *seg,
        })
    }

    fn get_seg_right_of(&self, col: VPCol) -> Option<SegContent<'_>> {
        let seg_string = self.get_string_at_right_of(col)?;
        let seg = self
            .segments
            .iter()
            .find(|seg| seg.start_display_col_index == seg_string.start_at)?;
        Some(SegContent {
            content: seg.get_str(self),
            seg: *seg,
        })
    }

    fn get_seg_left_of(&self, col: VPCol) -> Option<SegContent<'_>> {
        let seg_string = self.get_string_at_left_of(col)?;
        let seg = self
            .segments
            .iter()
            .find(|seg| seg.start_display_col_index == seg_string.start_at)?;
        Some(SegContent {
            content: seg.get_str(self),
            seg: *seg,
        })
    }

    fn get_seg_at_end(&self) -> Option<SegContent<'_>> {
        self.last().map(|seg| SegContent {
            content: seg.get_str(self),
            seg,
        })
    }

    fn clip(&self, start_col: CCol, width: VPWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.clip(start_col, width))
    }

    fn trunc_end_to_fit(&self, width: VPWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_end_to_fit(width))
    }

    fn trunc_end_by(&self, width: VPWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_end_by(width))
    }

    fn trunc_start_by(&self, width: VPWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_start_by(width))
    }

    fn segments_iter(&self) -> Self::SegmentIterator<'_> { self.segments.iter().copied() }

    fn is_empty(&self) -> bool { self.is_empty() }

    fn last(&self) -> Option<Seg> { self.last() }

    fn contains_wide_segment(&self) -> ContainsWideSegment {
        self.contains_wide_segment()
    }
}

// GraphemeStringMut trait implementation for GCStringOwned.
impl GraphemeStringMut for GCStringOwned {
    type MutResult = GCStringOwned; // Returns new instances (immutable paradigm).

    fn insert_text(&mut self, col: VPCol, text: &str) -> Option<Self::MutResult> {
        // Create a new string with text inserted at the column.
        let (new_string, _width) = self.insert_chunk_at_col(col, text);
        Some(GCStringOwned::new(new_string))
    }

    fn delete_range(&mut self, start: VPCol, end: VPCol) -> Option<Self::MutResult> {
        // Split at start position.
        if let Some((left, _)) = self.split_at_display_col(start) {
            let left_string = GCStringOwned::new(left);

            // Split at end position to get the part after.
            if let Some((_, right)) = self.split_at_display_col(end) {
                // Combine left and right parts.
                let combined = format!("{}{}", left_string.as_str(), right);
                Some(GCStringOwned::new(combined))
            } else {
                // Nothing after end, just return the left part.
                Some(left_string)
            }
        } else {
            None
        }
    }

    fn replace_range(
        &mut self,
        start: VPCol,
        end: VPCol,
        text: &str,
    ) -> Option<Self::MutResult> {
        // First delete the range.
        let mut temp = self.delete_range(start, end)?;

        // Then insert the new text at the start position.
        temp.insert_text(start, text)
    }

    fn truncate(&mut self, col: VPCol) -> Option<Self::MutResult> {
        // Split at the column and return the left part.
        if let Some((left, _)) = self.split_at_display_col(col) {
            Some(GCStringOwned::new(left))
        } else {
            None
        }
    }
}

impl From<(Seg, &GCStringOwned)> for SegStringOwned {
    fn from((seg, gc_string): (Seg, &GCStringOwned)) -> SegStringOwned {
        let seg_str = seg.get_str(gc_string);
        SegStringOwned {
            string: GCStringOwned::new(seg_str),
            width: seg.display_width,
            start_at: seg.start_display_col_index,
        }
    }
}

/// Methods to make it easy to work with getting owned string (from slices) at a given
/// display col index.
pub mod at_display_col_index {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GCStringOwned {
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
            arg_col_index: impl Into<VPCol>,
        ) -> Option<Seg> {
            let col: VPCol = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            if col != seg.start_display_col_index {
                return Some(seg);
            }
            None
        }

        /// Returns the string and display width of the grapheme cluster segment at the
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
            arg_col_index: impl Into<VPCol>,
        ) -> Option<SegStringOwned> {
            // Convert display_col_index to seg_index.
            let col: VPCol = arg_col_index.into();
            let seg_index_at_col = (self + col)?;

            // Get the segment at seg_index.
            let seg = self.get(seg_index_at_col)?;
            let seg_start_at = seg.start_display_col_index;
            (col == seg_start_at).then(|| {
                // The display_col_index is at the start of a grapheme cluster 👍.
                (seg, self).into()
            })
        }

        /// Returns the string at the right of the given `display_col_index`. If the
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
            arg_col_index: impl Into<VPCol>,
        ) -> Option<SegStringOwned> {
            let col: VPCol = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            (seg.seg_index < self.get_max_seg_index()).then(|| {
                let right_neighbor_seg = self.get(*seg.seg_index + ch(1))?;
                Some((right_neighbor_seg, self).into())
            })?
        }

        /// Returns the string at the left of the given `display_col_index`. If the
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
            arg_col_index: impl Into<VPCol>,
        ) -> Option<SegStringOwned> {
            let col: VPCol = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            (seg.seg_index > seg_index(0)).then(|| {
                let left_neighbor_seg = self.get(*seg.seg_index - ch(1))?;
                Some((left_neighbor_seg, self).into())
            })?
        }

        /// Returns the last grapheme cluster segment in the grapheme string.
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
        pub fn get_string_at_end(&self) -> Option<SegStringOwned> {
            let seg = self.last()?;
            Some((seg, self).into())
        }
    }
}

/// Methods for easily modifying grapheme cluster segments for common TUI use cases.
pub mod mutate {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GCStringOwned {
        /// Inserts the given `chunk` in the correct position of the `string`, and returns
        /// a new ([`InlineString`], [`VPWidth`]) tuple:
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
            arg_col_index: impl Into<VPCol>,
            arg_chunk: impl AsRef<str>,
        ) -> (InlineString, VPWidth) {
            let chunk = arg_chunk.as_ref();

            // Create an array-vec of &str from self.vec_segment, using self.iter().
            let mut vec = InlineVecStr::with_capacity(self.len().as_usize() + 1);
            // Add each seg's &str to the acc.
            vec.extend(
                // Turn self.segments into a list of &str.
                self.seg_iter().map(|seg| seg.get_str(&self.string)),
            );

            // Get seg_index at display_col_index.
            let col: VPCol = arg_col_index.into();
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
                GCStringOwned::new(chunk).width(),
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
            arg_col_index: impl Into<VPCol>,
        ) -> Option<InlineString> {
            // There is no segment present (Deref trait makes `len()` apply to
            // `vec_segment`).
            if self.is_empty() {
                return None;
            }

            // There is only one segment present.
            if self.len() == seg_length(1u16) {
                return Some("".into());
            }

            // There are more than 1 segments present.

            // Get seg_index at display_col_index.
            let col: VPCol = arg_col_index.into();
            let split_seg_index = (self + col)?;

            let mut vec_left = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_left_display_width = vp_width(0);
            {
                let left_range = seg_index(0)..split_seg_index;
                for seg_idx in left_range.as_index_iter() {
                    let seg = *self.segments.get(seg_idx.as_usize())?;
                    let string = seg.get_str(&self.string);
                    vec_left.push(string);
                    str_left_display_width += seg.display_width;
                }
            }

            let mut vec_right = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_right_display_width = vp_width(0);
            {
                let max_seg_len = self.len();
                let right_range =
                    seg_index((split_seg_index.as_usize() + 1).as_u16_narrowing())
                        ..seg_index((max_seg_len.as_usize()).as_u16_narrowing());
                for seg_idx in right_range.as_index_iter() {
                    let seg = *self.segments.get(seg_idx.as_usize())?;
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
            arg_col_index: impl Into<VPCol>,
        ) -> Option<(InlineString, InlineString)> {
            // Get seg_index at display_col_index.
            let col: VPCol = arg_col_index.into();
            let split_seg_index = (self + col)?;

            let mut acc_left = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_left_display_width = vp_width(0);
            {
                let left_range = seg_index(0u16)..split_seg_index;
                for seg_idx in left_range.as_index_iter() {
                    let seg = *self.segments.get(seg_idx.as_usize())?;
                    acc_left.push(seg.get_str(&self.string));
                    str_left_display_width += seg.display_width;
                }
            }

            let mut acc_right = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_right_unicode_width = vp_width(0);
            {
                let max_seg_len = self.len();
                let seg_range = split_seg_index
                    ..seg_index((max_seg_len.as_usize()).as_u16_narrowing());
                for seg_idx in seg_range.as_index_iter() {
                    let seg = *self.segments.get(seg_idx.as_usize())?;
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
    use super::*;
    use crate::{byte_index, vp_col, vp_width};

    #[test]
    fn test_get_byte_index() {
        let ascii = GCStringOwned::from("hello");
        assert_eq!(ascii.get_byte_index(seg_index(0)), Some(byte_index(0)));
        assert_eq!(ascii.get_byte_index(seg_index(1)), Some(byte_index(1)));
        assert_eq!(ascii.get_byte_index(seg_index(4)), Some(byte_index(4)));
        assert_eq!(ascii.get_byte_index(seg_index(5)), None);

        let unicode = GCStringOwned::from("a😀b");
        assert_eq!(unicode.get_byte_index(seg_index(0)), Some(byte_index(0)));
        assert_eq!(unicode.get_byte_index(seg_index(1)), Some(byte_index(1)));
        assert_eq!(unicode.get_byte_index(seg_index(2)), Some(byte_index(5)));
        assert_eq!(unicode.get_byte_index(seg_index(3)), None);
    }

    #[test]
    fn test_check_is_in_middle_of_grapheme() {
        let s = GCStringOwned::from("a😀b");
        // 'a' starts at col 0, width 1.
        assert_eq!(s.check_is_in_middle_of_grapheme(vp_col(0)), None);
        // '😀' starts at col 1, width 2.
        assert_eq!(s.check_is_in_middle_of_grapheme(vp_col(1)), None);
        // col 2 is in the middle of '😀'.
        let mid = s.check_is_in_middle_of_grapheme(vp_col(2));
        assert!(mid.is_some());
        assert_eq!(mid.expect("mid segment").start_display_col_index, vp_col(1));
        // 'b' starts at col 3, width 1.
        assert_eq!(s.check_is_in_middle_of_grapheme(vp_col(3)), None);
        // Out of bounds.
        assert_eq!(s.check_is_in_middle_of_grapheme(vp_col(4)), None);
    }

    #[test]
    fn test_get_string_at() {
        let s = GCStringOwned::from("a😀b");
        let at_0 = s.get_string_at(vp_col(0));
        assert!(at_0.is_some());
        assert_eq!(at_0.expect("at 0").string.as_str(), "a");

        let at_1 = s.get_string_at(vp_col(1));
        assert!(at_1.is_some());
        assert_eq!(at_1.expect("at 1").string.as_str(), "😀");

        // Middle of '😀' returns None.
        assert_eq!(s.get_string_at(vp_col(2)), None);

        let at_3 = s.get_string_at(vp_col(3));
        assert!(at_3.is_some());
        assert_eq!(at_3.expect("at 3").string.as_str(), "b");

        // Out of bounds returns None.
        assert_eq!(s.get_string_at(vp_col(4)), None);
    }

    #[test]
    fn test_get_string_at_right_and_left_of() {
        let s = GCStringOwned::from("a😀b");

        // Right of col 0 ('a') is '😀'.
        let right_0 = s.get_string_at_right_of(vp_col(0));
        assert!(right_0.is_some());
        assert_eq!(right_0.expect("right 0").string.as_str(), "😀");

        // Right of col 1 ('😀') is 'b'.
        let right_1 = s.get_string_at_right_of(vp_col(1));
        assert!(right_1.is_some());
        assert_eq!(right_1.expect("right 1").string.as_str(), "b");

        // Right of col 3 ('b', last segment) is None.
        assert_eq!(s.get_string_at_right_of(vp_col(3)), None);

        // Left of col 0 ('a', first segment) is None.
        assert_eq!(s.get_string_at_left_of(vp_col(0)), None);

        // Left of col 1 ('😀') is 'a'.
        let left_1 = s.get_string_at_left_of(vp_col(1));
        assert!(left_1.is_some());
        assert_eq!(left_1.expect("left 1").string.as_str(), "a");

        // Left of col 3 ('b') is '😀'.
        let left_3 = s.get_string_at_left_of(vp_col(3));
        assert!(left_3.is_some());
        assert_eq!(left_3.expect("left 3").string.as_str(), "😀");
    }

    #[test]
    fn test_get_string_at_end() {
        let s = GCStringOwned::from("a😀b");
        let end = s.get_string_at_end();
        assert!(end.is_some());
        assert_eq!(end.expect("end").string.as_str(), "b");

        let empty = GCStringOwned::from("");
        assert_eq!(empty.get_string_at_end(), None);
    }

    #[test]
    fn test_insert_chunk_at_col() {
        let s = GCStringOwned::from("hello");
        let (res_start, w1) = s.insert_chunk_at_col(vp_col(0), "X");
        assert_eq!(res_start.as_str(), "Xhello");
        assert_eq!(w1, vp_width(1));

        let (res_mid, _) = s.insert_chunk_at_col(vp_col(2), "X");
        assert_eq!(res_mid.as_str(), "heXllo");

        let (res_end, _) = s.insert_chunk_at_col(vp_col(5), "X");
        assert_eq!(res_end.as_str(), "helloX");

        let s_emoji = GCStringOwned::from("a😀b");
        let (res_emoji, w2) = s_emoji.insert_chunk_at_col(vp_col(1), "✨");
        assert_eq!(res_emoji.as_str(), "a✨😀b");
        assert_eq!(w2, vp_width(2));
    }

    #[test]
    fn test_delete_char_at_col() {
        let empty = GCStringOwned::from("");
        assert_eq!(empty.delete_char_at_col(vp_col(0)), None);

        let single = GCStringOwned::from("a");
        assert_eq!(single.delete_char_at_col(vp_col(0)), Some("".into()));

        let s = GCStringOwned::from("hello");
        assert_eq!(s.delete_char_at_col(vp_col(0)), Some("ello".into()));
        assert_eq!(s.delete_char_at_col(vp_col(2)), Some("helo".into()));
        assert_eq!(s.delete_char_at_col(vp_col(4)), Some("hell".into()));

        let s_emoji = GCStringOwned::from("a😀b");
        assert_eq!(s_emoji.delete_char_at_col(vp_col(1)), Some("ab".into()));
    }

    #[test]
    fn test_split_at_display_col() {
        let empty = GCStringOwned::from("");
        assert_eq!(empty.split_at_display_col(vp_col(0)), None);

        let s = GCStringOwned::from("hello");
        let split_0 = s.split_at_display_col(vp_col(0));
        assert!(split_0.is_some());
        let (left, right) = split_0.expect("split 0");
        assert_eq!(left.as_str(), "");
        assert_eq!(right.as_str(), "hello");

        let split_2 = s.split_at_display_col(vp_col(2));
        assert!(split_2.is_some());
        let (left, right) = split_2.expect("split 2");
        assert_eq!(left.as_str(), "he");
        assert_eq!(right.as_str(), "llo");

        // Beyond bounds returns None.
        assert_eq!(s.split_at_display_col(vp_col(5)), None);
    }

    #[test]
    fn test_contains_wide_segment() {
        let ascii = GCStringOwned::from("hello");
        assert_eq!(ascii.contains_wide_segment(), ContainsWideSegment::No);

        let emoji = GCStringOwned::from("a😀b");
        assert_eq!(emoji.contains_wide_segment(), ContainsWideSegment::Yes);
    }

    #[test]
    fn test_get_max_seg_index() {
        let empty = GCStringOwned::from("");
        assert_eq!(empty.get_max_seg_index(), seg_index(0));

        let s = GCStringOwned::from("hello");
        assert_eq!(s.get_max_seg_index(), seg_index(4));
    }

    #[test]
    fn test_grapheme_string_trait_seg_methods() {
        let s = GCStringOwned::from("a😀b");

        // get_seg_at.
        let seg_at_1 = s.get_seg_at(vp_col(1));
        assert!(seg_at_1.is_some());
        assert_eq!(seg_at_1.expect("seg at 1").content, "😀");
        assert!(s.get_seg_at(vp_col(2)).is_none());

        // get_seg_right_of.
        let right = s.get_seg_right_of(vp_col(0));
        assert!(right.is_some());
        assert_eq!(right.expect("right of 0").content, "😀");

        // get_seg_left_of.
        let left = s.get_seg_left_of(vp_col(3));
        assert!(left.is_some());
        assert_eq!(left.expect("left of 3").content, "😀");

        // get_seg_at_end.
        let end = s.get_seg_at_end();
        assert!(end.is_some());
        assert_eq!(end.expect("seg at end").content, "b");

        let empty = GCStringOwned::from("");
        assert!(empty.get_seg_at_end().is_none());
    }

    #[test]
    fn test_grapheme_string_mut_trait() {
        let mut s = GCStringOwned::from("hello");

        // insert_text.
        let inserted = s.insert_text(vp_col(1), "XYZ");
        assert!(inserted.is_some());
        assert_eq!(inserted.expect("inserted").as_str(), "hXYZello");

        // delete_range.
        let deleted = s.delete_range(vp_col(1), vp_col(4));
        assert!(deleted.is_some());
        assert_eq!(deleted.expect("deleted").as_str(), "ho");

        // delete_range with end beyond bounds.
        let deleted_to_end = s.delete_range(vp_col(2), vp_col(10));
        assert!(deleted_to_end.is_some());
        assert_eq!(deleted_to_end.expect("deleted to end").as_str(), "he");

        // delete_range with start out of bounds.
        assert_eq!(s.delete_range(vp_col(10), vp_col(12)), None);

        // replace_range.
        let replaced = s.replace_range(vp_col(1), vp_col(4), "ABC");
        assert!(replaced.is_some());
        assert_eq!(replaced.expect("replaced").as_str(), "hABCo");

        // truncate.
        let truncated = s.truncate(vp_col(3));
        assert!(truncated.is_some());
        assert_eq!(truncated.expect("truncated").as_str(), "hel");

        // truncate out of bounds.
        assert_eq!(s.truncate(vp_col(10)), None);
    }
}

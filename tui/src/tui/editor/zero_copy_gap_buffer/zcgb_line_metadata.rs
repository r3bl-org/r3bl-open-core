// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line metadata structure and operations.
//!
//! This module contains the [`LineMetadata`] struct which stores all the metadata
//! for a single line in the gap buffer, including buffer position, capacity,
//! grapheme segments, and display information.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, ByteIndex, ByteLength, CCol, CIndex,
            CLength, CWidth, ContainsWideSegment, CursorBoundsCheck, GCStringOwned,
            LengthOps, NarrowingCastToU16, NumericValue, RangeBoundsExt,
            RangeBoundsResult, RangeConstructExt, RangeExclusive, RangeValidityStatus,
            SegStringOwned, byte_index, byte_len, c_index, c_width,
            core::coordinates::byte_index::ByteIndexRangeExt};

/// Represents a grapheme cluster segment within a continuous document line.
///
/// Unlike [`Seg`] (which uses 16-bit Viewport types), [`DocSeg`] uses 64-bit
/// [`Canvas`]-domain coordinate types ([`CCol`], [`CIndex`], [`CWidth`]) and
/// [`ByteLength`], allowing document lines to exceed 65,535 columns or grapheme clusters.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [`Seg`]: crate::Seg
#[derive(Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash, Debug)]
pub struct DocSeg {
    /// Start index in bytes within the line content.
    pub start_byte_index: ByteIndex,

    /// End index in bytes within the line content.
    pub end_byte_index: ByteIndex,

    /// Display width of the grapheme cluster in [`Canvas`] columns.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    pub display_width: CWidth,

    /// 0-based [`Canvas`] segment index of this grapheme on the line.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    pub seg_index: CIndex,

    /// Byte size of this grapheme cluster.
    pub bytes_size: ByteLength,

    /// Starting display column index in [`Canvas`] space.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    pub start_display_col_index: CCol,
}

impl DocSeg {
    /// Gets the string slice for the grapheme cluster segment.
    pub fn get_str<'a>(&self, arg_str: &'a (impl AsRef<str> + ?Sized)) -> &'a str {
        let str = arg_str.as_ref();
        let start_index = self.start_byte_index.as_usize();
        let end_index = self.end_byte_index.as_usize();
        &str[start_index..end_index]
    }

    #[must_use]
    pub fn contains_wide_segment(&self) -> ContainsWideSegment {
        if self.display_width > c_width(1) {
            ContainsWideSegment::Yes
        } else {
            ContainsWideSegment::No
        }
    }

    /// Converts this segment metadata into an owned [`SegStringOwned`] by extracting the
    /// corresponding slice from `content`.
    #[must_use]
    pub fn to_seg_string_owned(&self, content: &str) -> Option<SegStringOwned> {
        let byte_range: RangeExclusive<ByteIndex> =
            self.start_byte_index..self.end_byte_index;

        let content_len = byte_len(content.len());
        if byte_range.check_range_is_valid_for_length(content_len)
            != RangeValidityStatus::Valid
        {
            return None;
        }

        let seg_content = &content[byte_range.to_usize_range()];

        Some(SegStringOwned {
            string: GCStringOwned::from(seg_content),
            width: self.display_width.as_u16_narrowing().into(),
            start_at: self.start_display_col_index.as_u16_narrowing().into(),
        })
    }
}

/// Metadata for a single line in the buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct LineMetadata {
    /// Where this line starts in the buffer.
    pub buffer_start: ByteIndex,

    /// Actual content length in bytes (before '\n').
    pub content_byte_len: ByteLength,

    /// Allocated capacity for this line.
    pub capacity: ByteLength,

    /// Grapheme cluster segments for this line.
    pub grapheme_segments: Vec<DocSeg>,

    /// Display width of the line.
    pub display_width: CWidth,

    /// Number of grapheme clusters.
    pub grapheme_count: CLength,
}

/// Categorizes the spatial relationship between the caret and the next multi-column wide
/// [`grapheme cluster`] (jumbo emoji or [`CJK`] character) to the right.
///
/// See the [Grapheme Cluster Display Width Taxonomy] in the [`graphemes`] module for more
/// details.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CJK`]: https://en.wikipedia.org/wiki/CJK_characters
/// [`grapheme cluster`]: https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
/// [`graphemes`]: mod@crate::graphemes
/// [Grapheme Cluster Display Width Taxonomy]:
///     mod@crate::graphemes#grapheme-cluster-display-width-taxonomy
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WideSegmentLookahead {
    /// A multi-column wide [`grapheme cluster`] (jumbo emoji) starts IMMEDIATELY touching
    /// the current caret position (`start_col == caret_col + width_at_caret`).
    ///
    /// ```text
    /// 🙏🏽: [ ]  (jumbo emoji, display width 2)
    ///
    /// col:    0   1   2   3
    ///       ┌───┬───┬───┬───┐
    /// line: │ a │ [ │ ] │ b │
    ///       └───┴───┴───┴───┘
    ///         ▲   ▲
    ///         │   └─ Adjacent! "🙏🏽" starts at col 1 (caret col 0 + width 1)
    ///         └───── Caret is at col 0 ('a', width 1)
    /// ```
    ///
    /// [`grapheme cluster`]: https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
    ImmediatelyAdjacent(DocSeg),

    /// A multi-column wide [`grapheme cluster`] (jumbo emoji) exists further down the
    /// line, but is separated from the caret by single-width characters
    /// (`display_width == 1`, such as [`ASCII`] text, spaces, or single-width Unicode
    /// characters like `é`, `α`, `ñ`) (`start_col > caret_col + width_at_caret`).
    ///
    /// ```text
    /// 🙏🏽: [ ]  (jumbo emoji, display width 2)
    ///
    /// col:    0   1   2   3   4   5
    ///       ┌───┬───┬───┬───┬───┬───┐
    /// line: │ a │   │   │   │ [ │ ] │
    ///       └───┴───┴───┴───┴───┴───┘
    ///         ▲               ▲
    ///         │               └─ Distant! "🙏🏽" starts at col 4 (caret col 0 + width 1 = 1 != 4)
    ///         └─────────────── Caret is at col 0 ('a', width 1)
    /// ```
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`grapheme cluster`]: https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
    Distant(DocSeg),

    /// No multi-column wide [`grapheme cluster`]s (jumbo emojis) exist to the right of
    /// the caret on this line.
    ///
    /// [`grapheme cluster`]: https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
    None,
}

impl LineMetadata {
    /// Get the buffer range for this line's content (excluding null padding).
    ///
    /// # Returns
    ///
    /// A range that can be used to slice the buffer to get only the
    /// actual content bytes, not including the null padding that fills the
    /// remaining capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::ZeroCopyGapBuffer;
    /// use r3bl_tui::ByteIndexRangeExt;
    ///
    /// let mut buffer = ZeroCopyGapBuffer::default();
    /// buffer.add_line();
    ///
    /// // Get the line info and content range
    /// let line_info = buffer.get_line_info(0u16).expect("conversion error");
    /// let content_range = line_info.content_range();
    ///
    /// // For a newly created line, content should be empty (only newline is stored separately)
    /// assert_eq!(content_range.to_usize_range().len(), 0);
    /// ```
    #[must_use]
    pub fn content_range(&self) -> RangeExclusive<ByteIndex> {
        let start = self.buffer_start;
        // Convert CLength to ByteIndex preserving the numeric value (for exclusive range
        // end).
        let end = start + byte_index(self.content_byte_len.as_usize());

        // Create the range and validate using cursor bounds checking.
        let content_range = start..end;

        // Type-safe bounds checking: ensure content doesn't exceed allocated capacity.
        // Convert content_byte_len to its last valid index, then check if capacity would
        // be overflowed.
        debug_assert!(
            self.capacity
                .is_overflowed_by(self.content_byte_len.convert_to_index())
                == ArrayOverflowResult::Within,
            "content_byte_len ({}) overflows line capacity ({})",
            self.content_byte_len.as_usize(),
            self.capacity.as_usize()
        );

        content_range
    }

    /// Get the byte position for a given segment index
    ///
    /// This method converts a grapheme cluster index (segment index) to its
    /// corresponding byte position in the line buffer. It handles three cases:
    /// - Beginning of line (`seg_index` = 0) → returns byte position 0
    /// - End of line (`seg_index` >= `grapheme_segments.len()`) → returns content length
    ///   as byte position
    /// - Middle of line → returns the `start_byte_index` of the segment
    ///
    /// # Arguments
    /// * `seg_index` - The grapheme cluster index to convert
    ///
    /// # Returns
    /// The byte position where the grapheme at `seg_index` starts
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{ZeroCopyGapBuffer, c_index, c_row};
    ///
    /// let mut buffer = ZeroCopyGapBuffer::default();
    /// buffer.add_line();
    /// buffer.insert_text_at_grapheme(c_row(0), c_index(0u16), "Hello").expect("conversion error");
    ///
    /// let line_info = buffer.get_line_info(0u16).expect("conversion error");
    ///
    /// // Beginning of line
    /// assert_eq!(line_info.get_byte_index(c_index(0u16)).as_usize(), 0);
    ///
    /// // End of line
    /// assert_eq!(line_info.get_byte_index(c_index(5u16)).as_usize(), 5);
    /// ```
    #[must_use]
    pub fn get_byte_index(&self, arg_seg_index: impl Into<CIndex>) -> ByteIndex {
        let seg_index: CIndex = arg_seg_index.into();
        if seg_index.is_zero() {
            byte_index(0)
        } else if let Some(segment) = self.grapheme_segments.get(seg_index.as_usize()) {
            segment.start_byte_index
        } else {
            self.content_byte_len.eol_cursor_position()
        }
    }

    #[must_use]
    pub fn get_seg_index(&self, arg_byte_index: impl Into<ByteIndex>) -> CIndex {
        let byte_index: ByteIndex = arg_byte_index.into();
        if byte_index.is_zero() {
            return c_index(0usize);
        }

        if byte_index.overflows(self.content_byte_len) == ArrayOverflowResult::Overflowed
        {
            return c_index(self.grapheme_segments.len());
        }

        let idx = self.grapheme_segments.partition_point(|seg| {
            let range = (seg.start_byte_index, seg.bytes_size).to_exclusive_range();
            range.check_index_is_within(byte_index) == RangeBoundsResult::Overflowed
        });

        self.grapheme_segments.get(idx).map_or_else(
            || c_index(self.grapheme_segments.len()),
            |seg| seg.seg_index,
        )
    }

    #[must_use]
    pub fn check_is_in_middle_of_grapheme(
        &self,
        arg_col_index: impl Into<CCol>,
    ) -> Option<DocSeg> {
        let col_index: CCol = arg_col_index.into();
        let seg = self.get_seg_containing(col_index)?;
        if seg.display_width > c_width(1) && col_index != seg.start_display_col_index {
            Some(seg)
        } else {
            None
        }
    }

    /// Returns the grapheme cluster segment containing `col_index` (i.e. `start <=
    /// col_index < start + width`).
    #[must_use]
    pub fn get_seg_containing(&self, col_index: CCol) -> Option<DocSeg> {
        let idx = self.grapheme_segments.partition_point(|seg| {
            let range =
                (seg.start_display_col_index, seg.display_width).to_exclusive_range();
            range.check_index_is_within(col_index) == RangeBoundsResult::Overflowed
        });
        self.grapheme_segments.get(idx).copied().filter(|seg| {
            let range =
                (seg.start_display_col_index, seg.display_width).to_exclusive_range();
            range.check_index_is_within(col_index) == RangeBoundsResult::Within
        })
    }

    /// Gets the string slice for the grapheme cluster segment containing `col_index`.
    #[must_use]
    pub fn get_string_at(
        &self,
        content: &str,
        col_index: CCol,
    ) -> Option<SegStringOwned> {
        self.get_seg_containing(col_index)?
            .to_seg_string_owned(content)
    }

    /// Returns the [`DocSeg`] that starts EXACTLY at `col_index`.
    ///
    /// If `col_index` falls inside a multi-column grapheme cluster (limbo / no-man's
    /// land) rather than at its starting display column index, this method returns
    /// [`None`].
    ///
    /// # Visualizing Segment Start vs. Limbo / No-Man's Land
    ///
    /// Given a wide grapheme cluster (e.g. jumbo emoji `😀`, display width 2) starting at
    /// column index 4:
    ///
    /// ```text
    /// 😀 = ( )
    ///
    /// Column Index:  0   1   2   3   4   5   6   7
    ///              ┌───┬───┬───┬───┬───┬───┬───┬───┐
    ///              │   │   │   │   │ ( │ ) │   │   │
    ///              └───┴───┴───┴───┴───┴───┴───┴───┘
    ///                                ▲   ▲
    ///                                │   └─ col_index 5 (Limbo / No-Man's Land)
    ///                                │      → returns None
    ///                                └───── col_index 4 (Exact Segment Start)
    ///                                       → returns Some(DocSeg)
    /// ```
    ///
    /// - **`col_index = 4`**: Exact start of the grapheme cluster → returns
    ///   `Some(DocSeg)`
    /// - **`col_index = 5`**: Interior column of wide grapheme → returns `None`
    #[must_use]
    pub fn get_seg_at(&self, col_index: CCol) -> Option<DocSeg> {
        let skip_width = c_width(col_index.as_usize());
        let idx = self.grapheme_segments.partition_point(|seg| {
            seg.start_display_col_index.overflows(skip_width)
                == ArrayOverflowResult::Within
        });
        self.grapheme_segments
            .get(idx)
            .copied()
            .filter(|seg| seg.start_display_col_index == col_index)
    }

    /// Look ahead to the right of `col_index` for the next multi-column wide grapheme
    /// cluster (`display_width > 1`, e.g., jumbo emojis `🙏🏽` or CJK characters `汉`) and
    /// evaluate its adjacency relative to the caret.
    ///
    /// # Return Values
    /// - [`WideSegmentLookahead::ImmediatelyAdjacent`] if the wide emoji starts directly
    ///   touching the caret position (`start_col == col_index + unicode_width_at_caret`).
    /// - [`WideSegmentLookahead::Distant`] if the wide emoji exists further down the line
    ///   separated by single-width characters (`display_width == 1`, such as [`ASCII`]
    ///   text, spaces, or single-width Unicode characters like `é`, `α`, `ñ`).
    /// - [`WideSegmentLookahead::None`] if no wide emojis exist to the right of
    ///   `col_index`.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    pub fn lookahead_wide_segment_to_right(
        &self,
        col_index: CCol,
        unicode_width_at_caret: CWidth,
    ) -> WideSegmentLookahead {
        let expected_adjacent_col = col_index + unicode_width_at_caret;

        let col_limit = c_width(col_index.as_usize());
        let start_idx = self.grapheme_segments.partition_point(|seg| {
            col_limit.is_valid_cursor_position(seg.start_display_col_index)
        });

        for segment in &self.grapheme_segments[start_idx..] {
            if segment.display_width > c_width(1) {
                if segment.start_display_col_index == expected_adjacent_col {
                    return WideSegmentLookahead::ImmediatelyAdjacent(*segment);
                }
                return WideSegmentLookahead::Distant(*segment);
            }
        }

        WideSegmentLookahead::None
    }

    /// Returns the next multi-column wide grapheme cluster segment (such as a jumbo
    /// emoji `🙏🏽` or CJK character `汉`) strictly to the right of `col_index`.
    ///
    /// This method is specifically used for wide grapheme cluster lookahead and viewport
    /// scrolling adjustments. It ignores single-width characters (`display_width == 1`,
    /// including [`ASCII`] text, spaces, and single-width Unicode characters like `é`,
    /// `α`, `ñ`) and searches [`grapheme_segments`] for the first segment with
    /// [`display_width`] > 1 whose display range starts strictly after `col_index`.
    ///
    /// ```text
    /// 🙏🏽: [ ]
    /// 😀: ( )
    ///
    /// col:   0   1   2        65  66  67  68  69  70
    ///       ┌───┬───┬─── ~~~ ───┬───┬───┬───┬───┬───┐
    /// line: │ a │ b │ c         │ [ │ ] │ ( │ ) │ ░ │
    ///       └───┴───┴─── ~~~ ───┴───┴───┴───┴───┴───┘
    ///        ▲                   ▲       ▲
    ///        │                   │       └─ jumbo emoji "😀" (width 2, cols 68..70)
    ///        │                   └───────── jumbo emoji "🙏🏽" (width 2, cols 66..68)
    ///        └───────────────────────────── col_index = 0
    ///                                       get_seg_at_right_of(0) -> Some(DocSeg for "🙏🏽")
    /// ```
    ///
    /// The bounds check constructs a safe `RangeExclusive<CCol>` (`[start, start +
    /// width)`) and verifies `range.check_index_is_within(col_index) ==
    /// RangeBoundsResult::Underflowed` (which indicates `col_index < start`).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`CCol`]: crate::CCol
    /// [`display_width`]: DocSeg::display_width
    /// [`grapheme_segments`]: Self::grapheme_segments
    #[must_use]
    pub fn get_seg_at_right_of(&self, col_index: CCol) -> Option<DocSeg> {
        match self.lookahead_wide_segment_to_right(col_index, c_width(1)) {
            WideSegmentLookahead::ImmediatelyAdjacent(seg)
            | WideSegmentLookahead::Distant(seg) => Some(seg),
            WideSegmentLookahead::None => None,
        }
    }

    /// Gets the string slice for the grapheme cluster segment strictly to the right of
    /// `col_index`. See [`get_seg_at_right_of()`].
    ///
    /// [`get_seg_at_right_of()`]: Self::get_seg_at_right_of
    #[must_use]
    pub fn get_string_at_right_of(
        &self,
        content: &str,
        col_index: CCol,
    ) -> Option<SegStringOwned> {
        self.get_seg_at_right_of(col_index)?
            .to_seg_string_owned(content)
    }

    /// Returns the grapheme cluster segment strictly to the left of `col_index`.
    ///
    /// This method uses [`to_exclusive_range()`] to construct a safe
    /// `RangeExclusive<CCol>` (`[start, start + width)`) and checks if
    /// `range.check_index_is_within(col_index) == RangeBoundsResult::Overflowed`.
    ///
    /// # Visualizing Segment Bounds Check vs. Range Containment
    ///
    /// Given segment spanning columns 2 through 4 (range `[2, 5)`):
    ///
    /// ```text
    /// Column Index:  0   1   2   3   4   5   6   7
    ///              ┌───┬───┬───┬───┬───┬───┬───┬───┐
    ///              │   │   │ ▓ │ ▓ │ ▓ │   │   │   │
    ///              └───┴───┴───┴───┴───┴───┴───┴───┘
    ///                        ▲           ▲
    ///                        start=2     end=5
    ///
    /// - RangeBoundsResult::Within (start <= col_index < end):
    ///   col_index is INSIDE the segment (e.g. col 3). Used for get_seg_at().
    ///
    /// - RangeBoundsResult::Overflowed (col_index >= end):
    ///   col_index is PAST THE END of the segment (e.g. col 5 or 6).
    ///   The segment is to the left of col_index. Used for get_seg_at_left_of().
    /// ```
    ///
    /// [`to_exclusive_range()`]: crate::RangeConstructExt::to_exclusive_range
    #[must_use]
    pub fn get_seg_at_left_of(&self, col_index: CCol) -> Option<DocSeg> {
        let count = self.grapheme_segments.partition_point(|seg| {
            let range =
                (seg.start_display_col_index, seg.display_width).to_exclusive_range();
            range.check_index_is_within(col_index) == RangeBoundsResult::Overflowed
        });

        count
            .checked_sub(1)
            .and_then(|idx| self.grapheme_segments.get(idx).copied())
    }

    /// Gets the string slice for the grapheme cluster segment strictly to the left of
    /// `col_index`. See [`get_seg_at_left_of()`].
    ///
    /// [`get_seg_at_left_of()`]: Self::get_seg_at_left_of
    #[must_use]
    pub fn get_string_at_left_of(
        &self,
        content: &str,
        col_index: CCol,
    ) -> Option<SegStringOwned> {
        self.get_seg_at_left_of(col_index)?
            .to_seg_string_owned(content)
    }

    #[must_use]
    pub fn get_seg_at_end(&self) -> Option<DocSeg> {
        self.grapheme_segments.last().copied()
    }

    /// Gets the string slice for the last grapheme cluster segment on the line.
    /// See [`get_seg_at_end()`].
    ///
    /// [`get_seg_at_end()`]: Self::get_seg_at_end
    #[must_use]
    pub fn get_string_at_end(&self, content: &str) -> Option<SegStringOwned> {
        self.get_seg_at_end()?.to_seg_string_owned(content)
    }

    #[must_use]
    pub fn clip_to_range<'a>(
        &'a self,
        content: &'a str,
        col_range: impl Into<RangeExclusive<CCol>>,
    ) -> &'a str {
        let col_range: RangeExclusive<CCol> = col_range.into();
        let start_col_index = col_range.start;
        let max_col_width = c_width(
            col_range
                .end
                .as_usize()
                .saturating_sub(col_range.start.as_usize()),
        );

        if self.grapheme_segments.is_empty()
            || content.is_empty()
            || max_col_width.is_zero()
        {
            return "";
        }

        let skip_width = c_width(start_col_index.as_usize());
        let start_seg_idx = self.grapheme_segments.partition_point(|seg| {
            seg.start_display_col_index.overflows(skip_width)
                == ArrayOverflowResult::Within
        });

        let Some(start_seg) = self.grapheme_segments.get(start_seg_idx) else {
            return "";
        };

        let start_byte = start_seg.start_byte_index.as_usize();
        let start_col = start_seg.start_display_col_index;
        let max_col = start_col + max_col_width;
        let max_width = c_width(max_col.as_usize());

        let fit_count = self.grapheme_segments[start_seg_idx..].partition_point(|seg| {
            max_width
                .is_valid_cursor_position(seg.start_display_col_index + seg.display_width)
        });

        let Some(last_seg) = fit_count
            .checked_sub(1)
            .and_then(|offset| self.grapheme_segments.get(start_seg_idx + offset))
        else {
            return "";
        };

        let end_byte = last_seg.end_byte_index.as_usize();

        let byte_range: RangeExclusive<ByteIndex> =
            byte_index(start_byte)..byte_index(end_byte);

        match byte_range.check_range_is_valid_for_length(byte_len(content.len())) {
            RangeValidityStatus::Valid => {
                &content[byte_range.start.as_usize()..byte_range.end.as_usize()]
            }
            _ => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{RangeExt, WideSegmentLookahead, ZeroCopyGapBuffer, byte_index, c_col,
                c_index, c_row, c_width};

    #[test]
    fn test_get_byte_pos_beginning() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hello")
            .expect("conversion error");

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // Test beginning position.
        assert_eq!(line_info.get_byte_index(c_index(0u16)).as_usize(), 0);
    }

    #[test]
    fn test_get_byte_pos_end() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hello")
            .expect("conversion error");

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // Test end position (past last segment).
        assert_eq!(line_info.get_byte_index(c_index(5u16)).as_usize(), 5);
        assert_eq!(line_info.get_byte_index(c_index(10u16)).as_usize(), 5);
    }

    #[test]
    fn test_get_byte_pos_middle() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert text with multi-byte characters.
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "H😀llo")
            .expect("conversion error");

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // Test various positions.
        assert_eq!(line_info.get_byte_index(c_index(0u16)).as_usize(), 0); // Before 'H'
        assert_eq!(line_info.get_byte_index(c_index(1u16)).as_usize(), 1); // Before '😀'
        assert_eq!(line_info.get_byte_index(c_index(2u16)).as_usize(), 5); // Before 'l' (emoji is 4 bytes)
        assert_eq!(line_info.get_byte_index(c_index(3u16)).as_usize(), 6); // Before second 'l'
        assert_eq!(line_info.get_byte_index(c_index(4u16)).as_usize(), 7); // Before 'o'
        assert_eq!(line_info.get_byte_index(c_index(5u16)).as_usize(), 8); // End of string
    }

    #[test]
    fn test_get_byte_pos_empty_line() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // For empty line, any position should return 0.
        assert_eq!(line_info.get_byte_index(c_index(0u16)).as_usize(), 0);
        assert_eq!(line_info.get_byte_index(c_index(1u16)).as_usize(), 0);
        assert_eq!(line_info.get_byte_index(c_index(100u16)).as_usize(), 0);
    }

    #[test]
    fn test_get_seg_index_beginning() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hello")
            .expect("conversion error");

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // Test beginning position.
        assert_eq!(line_info.get_seg_index(byte_index(0)), c_index(0u16));
    }

    #[test]
    fn test_get_seg_index_end() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hello")
            .expect("conversion error");

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // Test end position (at or past content length).
        assert_eq!(line_info.get_seg_index(byte_index(5)), c_index(5u16));
        assert_eq!(line_info.get_seg_index(byte_index(10)), c_index(5u16));
    }

    #[test]
    fn test_get_seg_index_middle() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert text with emoji: "H😀llo".
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "H😀llo")
            .expect("conversion error");

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // Test various byte positions.
        assert_eq!(line_info.get_seg_index(byte_index(0)), c_index(0u16)); // Start of 'H'
        assert_eq!(line_info.get_seg_index(byte_index(1)), c_index(1u16)); // Start of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(2)), c_index(1u16)); // Middle of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(3)), c_index(1u16)); // Middle of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(4)), c_index(1u16)); // End of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(5)), c_index(2u16)); // Start of 'l'
        assert_eq!(line_info.get_seg_index(byte_index(6)), c_index(3u16)); // Start of second 'l'
        assert_eq!(line_info.get_seg_index(byte_index(7)), c_index(4u16)); // Start of 'o'
        assert_eq!(line_info.get_seg_index(byte_index(8)), c_index(5u16)); // End of string
    }

    #[test]
    fn test_get_seg_index_empty_line() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        // For empty line, any position should return 0.
        assert_eq!(line_info.get_seg_index(byte_index(0)), c_index(0u16));
        assert_eq!(line_info.get_seg_index(byte_index(1)), c_index(0u16));
        assert_eq!(line_info.get_seg_index(byte_index(100)), c_index(0u16));
    }

    #[test]
    fn test_get_seg_index_get_byte_pos_round_trip() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert text with various Unicode: "a👨‍👩‍👧‍👦b世界c".
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "a👨‍👩‍👧‍👦b世界c")
            .expect("conversion error");

        let line_info = buffer.get_line_info(0u16).expect("conversion error");

        let seg_range = ..line_info.grapheme_count;
        for seg_idx in seg_range.as_index_iter() {
            let byte_pos = line_info.get_byte_index(seg_idx);
            let seg_idx_back = line_info.get_seg_index(byte_pos);
            assert_eq!(
                seg_idx,
                seg_idx_back,
                "Round-trip failed for segment {seg_idx:?}: byte_pos={}",
                byte_pos.as_usize()
            );
        }
    }

    #[test]
    fn test_gap_buffer_line_info_clip_to_range() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Insert Unicode-rich content: "Hi📦XelLo🙏🏽Bye".
        // Display layout:
        // H(1) i(1) 📦(2) X(1) e(1) l(1) L(1) o(1) 🙏🏽(2) B(1) y(1) e(1) = 14 total width
        // Columns: 0    1   23     4    5   6   7   8   9A      B    C   D
        // (A=10,B=11,C=12,D=13)
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hi📦XelLo🙏🏽Bye")
            .expect("conversion error");

        let line = buffer.get_line(c_row(0)).expect("conversion error");
        let content = line.content();
        let line_info = line.info();

        // Test: Clip from start.
        let result = line_info.clip_to_range(content, c_col(0)..c_col(2));
        assert_eq!(result, "Hi");

        // Test: Clip emoji (starts at col 2, has width 2).
        let result = line_info.clip_to_range(content, c_col(2)..c_col(4));
        assert_eq!(result, "📦");

        // Test: Clip across emoji boundary.
        let result = line_info.clip_to_range(content, c_col(2)..c_col(6));
        assert_eq!(result, "📦Xe");

        // Test: Clip multi-width emoji 🙏🏽 (starts at col 9, has width 2).
        let result = line_info.clip_to_range(content, c_col(9)..c_col(11));
        assert_eq!(result, "🙏🏽");

        // Test: Clip including multi-width emoji.
        let result = line_info.clip_to_range(content, c_col(6)..c_col(11));
        assert_eq!(result, "lLo🙏🏽");

        // Test: Clip from middle to end.
        let result = line_info.clip_to_range(content, c_col(11)..c_col(21));
        assert_eq!(result, "Bye");

        // Test: Empty clip (beyond content).
        let result = line_info.clip_to_range(content, c_col(20)..c_col(25));
        assert_eq!(result, "");

        // Test: Zero width.
        let result = line_info.clip_to_range(content, c_col(5)..c_col(5));
        assert_eq!(result, "");

        // Test: Empty line.
        let mut empty_buffer = ZeroCopyGapBuffer::default();
        empty_buffer.add_line();
        let empty_line = empty_buffer.get_line(c_row(0)).expect("conversion error");
        let result = empty_line
            .info()
            .clip_to_range(empty_line.content(), c_col(0)..c_col(5));
        assert_eq!(result, "");
    }

    #[test]
    fn test_lookahead_wide_segment_to_right_evaluates_adjacency() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Content: "Hi📦X"
        // 'H' (col 0), 'i' (col 1), '📦' (cols 2..4, width 2), 'X' (col 4)
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hi📦X")
            .expect("conversion error");

        let line = buffer.get_line(c_row(0)).expect("conversion error");
        let line_info = line.info();

        // Query col 0 ('H', width 1, adjacent col 1): returns Distant('📦' at col 2)
        match line_info.lookahead_wide_segment_to_right(c_col(0), c_width(1)) {
            WideSegmentLookahead::Distant(seg) => {
                assert_eq!(seg.start_display_col_index, c_col(2));
                assert_eq!(seg.display_width, c_width(2));
            }
            other => panic!("expected Distant, got {other:?}"),
        }

        // Query col 1 ('i', width 1, adjacent col 2): returns ImmediatelyAdjacent('📦' at
        // col 2)
        match line_info.lookahead_wide_segment_to_right(c_col(1), c_width(1)) {
            WideSegmentLookahead::ImmediatelyAdjacent(seg) => {
                assert_eq!(seg.start_display_col_index, c_col(2));
                assert_eq!(seg.display_width, c_width(2));
            }
            other => panic!("expected ImmediatelyAdjacent, got {other:?}"),
        }

        // Query col 2 ('📦'): returns None (no wide emoji after col 2)
        assert_eq!(
            line_info.lookahead_wide_segment_to_right(c_col(2), c_width(2)),
            WideSegmentLookahead::None
        );
    }

    #[test]
    fn test_get_seg_at() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let empty_line = buffer.get_line(c_row(0)).expect("conversion error");
        assert!(empty_line.info().get_seg_at(c_col(0)).is_none());

        // Content: "Hi📦X"
        // 'H' (col 0, width 1), 'i' (col 1, width 1), '📦' (cols 2..4, width 2), 'X' (col
        // 4, width 1)
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hi📦X")
            .expect("conversion error");

        let line = buffer.get_line(c_row(0)).expect("conversion error");
        let line_info = line.info();

        assert_eq!(
            line_info
                .get_seg_at(c_col(0))
                .map(|s| s.start_display_col_index),
            Some(c_col(0))
        );
        assert_eq!(
            line_info
                .get_seg_at(c_col(1))
                .map(|s| s.start_display_col_index),
            Some(c_col(1))
        );
        assert_eq!(
            line_info
                .get_seg_at(c_col(2))
                .map(|s| s.start_display_col_index),
            Some(c_col(2))
        );
        // Col 3 is limbo / interior of '📦', not segment start.
        assert!(line_info.get_seg_at(c_col(3)).is_none());
        assert_eq!(
            line_info
                .get_seg_at(c_col(4))
                .map(|s| s.start_display_col_index),
            Some(c_col(4))
        );
        assert!(line_info.get_seg_at(c_col(5)).is_none());
    }

    #[test]
    fn test_get_seg_containing() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let empty_line = buffer.get_line(c_row(0)).expect("conversion error");
        assert!(empty_line.info().get_seg_containing(c_col(0)).is_none());

        // Content: "Hi📦X"
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hi📦X")
            .expect("conversion error");

        let line = buffer.get_line(c_row(0)).expect("conversion error");
        let line_info = line.info();

        assert_eq!(
            line_info
                .get_seg_containing(c_col(0))
                .map(|s| s.start_display_col_index),
            Some(c_col(0))
        );
        assert_eq!(
            line_info
                .get_seg_containing(c_col(1))
                .map(|s| s.start_display_col_index),
            Some(c_col(1))
        );
        assert_eq!(
            line_info
                .get_seg_containing(c_col(2))
                .map(|s| s.start_display_col_index),
            Some(c_col(2))
        );
        // Col 3 is interior of '📦', so it is contained in '📦'.
        assert_eq!(
            line_info
                .get_seg_containing(c_col(3))
                .map(|s| s.start_display_col_index),
            Some(c_col(2))
        );
        assert_eq!(
            line_info
                .get_seg_containing(c_col(4))
                .map(|s| s.start_display_col_index),
            Some(c_col(4))
        );
        assert!(line_info.get_seg_containing(c_col(5)).is_none());
    }

    #[test]
    fn test_check_is_in_middle_of_grapheme() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let empty_line = buffer.get_line(c_row(0)).expect("conversion error");
        assert!(
            empty_line
                .info()
                .check_is_in_middle_of_grapheme(c_col(0))
                .is_none()
        );

        // Content: "Hi📦X"
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hi📦X")
            .expect("conversion error");

        let line = buffer.get_line(c_row(0)).expect("conversion error");
        let line_info = line.info();

        // Narrow graphemes: cannot be in middle.
        assert!(line_info.check_is_in_middle_of_grapheme(c_col(0)).is_none());
        assert!(line_info.check_is_in_middle_of_grapheme(c_col(1)).is_none());

        // Wide grapheme start: not in middle.
        assert!(line_info.check_is_in_middle_of_grapheme(c_col(2)).is_none());

        // Wide grapheme interior: in middle.
        let middle_seg = line_info.check_is_in_middle_of_grapheme(c_col(3));
        assert!(middle_seg.is_some());
        assert_eq!(middle_seg.unwrap().start_display_col_index, c_col(2));

        // Narrow grapheme at col 4 and past end.
        assert!(line_info.check_is_in_middle_of_grapheme(c_col(4)).is_none());
        assert!(line_info.check_is_in_middle_of_grapheme(c_col(5)).is_none());
    }

    #[test]
    fn test_get_seg_at_left_of() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let empty_line = buffer.get_line(c_row(0)).expect("conversion error");
        assert!(empty_line.info().get_seg_at_left_of(c_col(0)).is_none());

        // Content: "Hi📦X"
        buffer
            .insert_text_at_grapheme(c_row(0), c_index(0u16), "Hi📦X")
            .expect("conversion error");

        let line = buffer.get_line(c_row(0)).expect("conversion error");
        let line_info = line.info();

        // Col 0: no segment to left.
        assert!(line_info.get_seg_at_left_of(c_col(0)).is_none());

        // Col 1: 'H' (spans [0, 1)) is strictly to left.
        assert_eq!(
            line_info
                .get_seg_at_left_of(c_col(1))
                .map(|s| s.start_display_col_index),
            Some(c_col(0))
        );

        // Col 2: 'i' (spans [1, 2)) is strictly to left.
        assert_eq!(
            line_info
                .get_seg_at_left_of(c_col(2))
                .map(|s| s.start_display_col_index),
            Some(c_col(1))
        );

        // Col 3: '📦' (spans [2, 4)) is not completely to left, so 'i' is the last
        // segment to left.
        assert_eq!(
            line_info
                .get_seg_at_left_of(c_col(3))
                .map(|s| s.start_display_col_index),
            Some(c_col(1))
        );

        // Col 4: '📦' (spans [2, 4)) ends at 4, so it is strictly to left.
        assert_eq!(
            line_info
                .get_seg_at_left_of(c_col(4))
                .map(|s| s.start_display_col_index),
            Some(c_col(2))
        );

        // Col 5: 'X' (spans [4, 5)) is strictly to left.
        assert_eq!(
            line_info
                .get_seg_at_left_of(c_col(5))
                .map(|s| s.start_display_col_index),
            Some(c_col(4))
        );

        // Col 10: past end, 'X' is still the last segment strictly to left.
        assert_eq!(
            line_info
                .get_seg_at_left_of(c_col(10))
                .map(|s| s.start_display_col_index),
            Some(c_col(4))
        );
    }
}

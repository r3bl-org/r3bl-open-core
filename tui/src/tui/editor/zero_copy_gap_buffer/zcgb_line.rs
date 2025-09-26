// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`GapBufferLine`] - A line view that combines content and metadata.
//!
//! This struct provides unified access to both line content and its associated
//! metadata, eliminating the need to work with split content/metadata APIs.

use super::LineMetadata;
use crate::{ChUnit, ColIndex, ColWidth, ContainsWideSegments, GraphemeString, Length,
            Seg, SegContent, SegIndex, SegLength, SegStringOwned, UnitCompare,
            byte_index, ch, width};

/// A line from the gap buffer containing both content and metadata.
///
/// This struct provides ergonomic access to line content and its associated
/// metadata for zero-copy operations.
///
/// # Usage
/// ```rust
/// # use r3bl_tui::{ZeroCopyGapBuffer, row, col};
/// # let mut buffer = ZeroCopyGapBuffer::new();
/// # buffer.add_line();
/// let line = buffer.get_line(row(0)).unwrap();
/// let content = line.content();
/// let metadata = line.info();
/// let width = line.info().display_width;
/// let seg_string = line.info().get_string_at(line.content(), col(0));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct GapBufferLine<'a> {
    content: &'a str,
    info: &'a LineMetadata,
}

impl<'a> GapBufferLine<'a> {
    /// Create a new `GapBufferLine`.
    #[must_use]
    pub fn new(content: &'a str, info: &'a LineMetadata) -> Self {
        Self { content, info }
    }

    /// Get the line content as a string slice.
    #[must_use]
    pub fn content(&self) -> &'a str { self.content }

    /// Get the line metadata.
    #[must_use]
    pub fn info(&self) -> &'a LineMetadata { self.info }

    /// Get the display width of the line.
    #[must_use]
    pub fn display_width(&self) -> ColWidth { self.info.display_width }

    /// Get the number of grapheme clusters in the line.
    #[must_use]
    pub fn grapheme_count(&self) -> Length { self.info.grapheme_count }

    /// Get the number of grapheme cluster segments.
    /// This is the preferred method for semantic clarity.
    #[must_use]
    pub fn segment_count(&self) -> SegLength {
        SegLength::from(self.info.grapheme_count.as_usize())
    }

    /// Get the segments (grapheme cluster information) for the line.
    #[must_use]
    pub fn segments(&self) -> &[Seg] { &self.info.segments }

    /// Check if the given column index falls in the middle of a grapheme cluster.
    #[must_use]
    pub fn check_is_in_middle_of_grapheme(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<Seg> {
        let col_index: ColIndex = arg_col_index.into();
        self.info.check_is_in_middle_of_grapheme(col_index)
    }

    /// Get the string at the given column index.
    #[must_use]
    pub fn get_string_at(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<SegStringOwned> {
        let col_index: ColIndex = arg_col_index.into();
        self.info.get_string_at(self.content, col_index)
    }

    /// Get the string at the right of the given column index.
    #[must_use]
    pub fn get_string_at_right_of(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<SegStringOwned> {
        let col_index: ColIndex = arg_col_index.into();
        self.info.get_string_at_right_of(self.content, col_index)
    }

    /// Get the string at the left of the given column index.
    #[must_use]
    pub fn get_string_at_left_of(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<SegStringOwned> {
        let col_index: ColIndex = arg_col_index.into();
        self.info.get_string_at_left_of(self.content, col_index)
    }

    /// Get the last grapheme cluster in the line.
    #[must_use]
    pub fn get_string_at_end(&self) -> Option<SegStringOwned> {
        self.info.get_string_at_end(self.content)
    }

    /// Check if the line is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.info.grapheme_count.is_zero() }

    /// Get content length in bytes.
    #[must_use]
    pub fn byte_len(&self) -> Length { self.info.content_len }
}

/// `GraphemeString` trait implementation for `GapBufferLine`
///
/// # Lifetime Relationships
///
/// The associated types use a lifetime parameter `'b` that represents the lifetime
/// of references returned by trait methods. The constraint `Self: 'b` ensures that
/// any borrowed data cannot outlive the `GapBufferLine` instance.
///
/// Since `GapBufferLine<'a>` contains references with lifetime `'a`, the constraint
/// `Self: 'b` implicitly requires `'a: 'b` (i.e., `'a` must outlive `'b`).
impl GraphemeString for GapBufferLine<'_> {
    /// Iterator over segments with lifetime `'b`.
    ///
    /// The lifetime `'b` is constrained by `Self: 'b`, which means the iterator
    /// cannot outlive the `GapBufferLine` it borrows from.
    type SegmentIterator<'b>
        = std::iter::Copied<std::slice::Iter<'b, Seg>>
    where
        Self: 'b;

    /// String slice with lifetime `'b`.
    ///
    /// The lifetime `'b` is constrained by `Self: 'b`, which means the string slice
    /// cannot outlive the `GapBufferLine` it borrows from.
    type StringSlice<'b>
        = &'b str
    where
        Self: 'b;

    fn as_str(&self) -> &str { self.content() }

    fn segments(&self) -> &[Seg] { self.segments() }

    fn display_width(&self) -> ColWidth { self.display_width() }

    fn segment_count(&self) -> SegLength { self.segment_count() }

    fn byte_size(&self) -> ChUnit { ch(self.content().len()) }

    fn get_seg(&self, index: SegIndex) -> Option<Seg> {
        self.info.segments.get(index.as_usize()).copied()
    }

    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg> {
        self.check_is_in_middle_of_grapheme(col)
    }

    /// Get the segment at the given column index.
    ///
    /// Returns a `SegContent` with an anonymous lifetime `'_` that is tied to
    /// the lifetime of `self`. This ensures the returned content reference
    /// cannot outlive the `GapBufferLine` instance.
    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at(col).and_then(|seg_string| {
            self.segments()
                .iter()
                .find(|seg| seg.start_display_col_index == seg_string.start_at)
                .map(|seg| SegContent {
                    content: seg.get_str(self.content()),
                    seg: *seg,
                })
        })
    }

    /// Get the segment at the right of the given column index.
    ///
    /// Returns a `SegContent` with an anonymous lifetime `'_` that is tied to
    /// the lifetime of `self`. This ensures the returned content reference
    /// cannot outlive the `GapBufferLine` instance.
    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_right_of(col).and_then(|seg_string| {
            self.segments()
                .iter()
                .find(|seg| seg.start_display_col_index == seg_string.start_at)
                .map(|seg| SegContent {
                    content: seg.get_str(self.content()),
                    seg: *seg,
                })
        })
    }

    /// Get the segment at the left of the given column index.
    ///
    /// Returns a `SegContent` with an anonymous lifetime `'_` that is tied to
    /// the lifetime of `self`. This ensures the returned content reference
    /// cannot outlive the `GapBufferLine` instance.
    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_left_of(col).and_then(|seg_string| {
            self.segments()
                .iter()
                .find(|seg| seg.start_display_col_index == seg_string.start_at)
                .map(|seg| SegContent {
                    content: seg.get_str(self.content()),
                    seg: *seg,
                })
        })
    }

    /// Get the segment at the end of the line.
    ///
    /// Returns a `SegContent` with an anonymous lifetime `'_` that is tied to
    /// the lifetime of `self`. This ensures the returned content reference
    /// cannot outlive the `GapBufferLine` instance.
    fn get_seg_at_end(&self) -> Option<SegContent<'_>> {
        self.get_string_at_end().and_then(|seg_string| {
            self.segments()
                .iter()
                .find(|seg| seg.start_display_col_index == seg_string.start_at)
                .map(|seg| SegContent {
                    content: seg.get_str(self.content()),
                    seg: *seg,
                })
        })
    }

    /// Clip the string to the given range.
    ///
    /// Returns a string slice with lifetime tied to `self`. The anonymous lifetime
    /// `'_` ensures the returned slice cannot outlive the `GapBufferLine` instance.
    fn clip(&self, start_col: ColIndex, width: ColWidth) -> Self::StringSlice<'_> {
        self.info.clip_to_range(self.content(), start_col, width)
    }

    /// Truncate from the end to fit within the given width.
    ///
    /// Returns a string slice with lifetime tied to `self`. The anonymous lifetime
    /// `'_` ensures the returned slice cannot outlive the `GapBufferLine` instance.
    fn trunc_end_to_fit(&self, width: ColWidth) -> Self::StringSlice<'_> {
        // Source of truth: GCStringOwned::trunc_end_to_fit algorithm
        let mut avail_cols = width;
        let mut string_end_byte_index = 0;

        for seg in self.segments() {
            let seg_display_width = seg.display_width;
            if avail_cols < seg_display_width {
                break;
            }
            string_end_byte_index += seg.bytes_size.as_usize();
            avail_cols -= seg_display_width;
        }

        &self.content()[..string_end_byte_index.min(self.content().len())]
    }

    /// Truncate from the end by the given width.
    ///
    /// Returns a string slice with lifetime tied to `self`. The anonymous lifetime
    /// `'_` ensures the returned slice cannot outlive the `GapBufferLine` instance.
    fn trunc_end_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        // Source of truth: GCStringOwned::trunc_end_by algorithm
        let mut countdown_col_count = width;
        let mut string_end_byte_index = byte_index(0);

        for seg in self.segments().iter().rev() {
            let seg_display_width = seg.display_width;
            string_end_byte_index = seg.start_byte_index;
            countdown_col_count -= seg_display_width;
            if *countdown_col_count == ch(0) {
                break;
            }
        }

        &self.content()[..string_end_byte_index.as_usize().min(self.content().len())]
    }

    /// Truncate from the start by the given width.
    ///
    /// Returns a string slice with lifetime tied to `self`. The anonymous lifetime
    /// `'_` ensures the returned slice cannot outlive the `GapBufferLine` instance.
    fn trunc_start_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        // Adapt GCStringOwned algorithm for starting from beginning.
        let mut skip_col_count = width;
        let mut string_start_byte_index = 0;

        for seg in self.segments() {
            let seg_display_width = seg.display_width;
            if *skip_col_count == ch(0) {
                break;
            }
            skip_col_count -= seg_display_width;
            string_start_byte_index += seg.bytes_size.as_usize();
        }

        &self.content()[string_start_byte_index.min(self.content().len())..]
    }

    /// Get an iterator over the segments.
    ///
    /// Returns an iterator with an anonymous lifetime `'_` that is tied to
    /// the lifetime of `self`. This ensures the iterator cannot outlive
    /// the `GapBufferLine` instance.
    fn segments_iter(&self) -> Self::SegmentIterator<'_> {
        self.segments().iter().copied()
    }

    fn is_empty(&self) -> bool { self.is_empty() }

    fn last(&self) -> Option<Seg> { self.segments().last().copied() }

    fn contains_wide_segments(&self) -> ContainsWideSegments {
        if self
            .segments()
            .iter()
            .any(|seg| seg.display_width > width(1))
        {
            ContainsWideSegments::Yes
        } else {
            ContainsWideSegments::No
        }
    }
}

#[cfg(test)]
mod test_fixtures {
    use crate::{ZeroCopyGapBuffer, row, seg_index};

    /// Helper to create a test buffer with sample content
    pub(super) fn create_test_buffer() -> ZeroCopyGapBuffer {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert Unicode-rich content: "Hello 👋 世界!".
        // This includes: ASCII, emoji, CJK characters, punctuation
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello 👋 世界!")
            .unwrap();

        buffer
    }
}

/// Content/Metadata Consistency Tests
/// Verify that the content and metadata are always in sync
#[cfg(test)]
mod tests_content_metadata_consistency {
    use unicode_segmentation::UnicodeSegmentation;

    use super::test_fixtures::create_test_buffer;
    use crate::{ZeroCopyGapBuffer, row};

    #[test]
    fn test_content_length_consistency() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Content byte length should match metadata.
        assert_eq!(line.content().len(), line.byte_len().as_usize());
        assert_eq!(line.content().len(), line.info().content_len.as_usize());
    }

    #[test]
    fn test_grapheme_count_consistency() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Count graphemes manually and compare with metadata.
        let manual_count = line.content().graphemes(true).count();

        assert_eq!(manual_count, line.grapheme_count().as_usize());
        assert_eq!(manual_count, line.info().grapheme_count.as_usize());
    }

    #[test]
    fn test_display_width_consistency() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Display width should be consistent between facade and metadata.
        assert_eq!(line.display_width(), line.info().display_width);
    }

    #[test]
    fn test_segments_consistency() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Segments should match between facade and metadata.
        assert_eq!(line.segments().len(), line.info().segments.len());
        assert_eq!(line.segments(), &line.info().segments[..]);

        // Number of segments should match grapheme count.
        assert_eq!(line.segments().len(), line.grapheme_count().as_usize());
    }

    #[test]
    fn test_empty_line_consistency() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line(); // Empty line
        let line = buffer.get_line(row(0)).unwrap();

        assert_eq!(line.content().len(), 0);
        assert_eq!(line.byte_len().as_usize(), 0);
        assert_eq!(line.grapheme_count().as_usize(), 0);
        assert_eq!(line.display_width().as_usize(), 0);
        assert!(line.segments().is_empty());
        assert!(line.is_empty());
    }
}

/// Empty Line Edge Cases Tests
/// Test all methods on empty lines return appropriate defaults
#[cfg(test)]
mod tests_empty_line_edge_cases {
    use crate::{ZeroCopyGapBuffer, col, len, row, seg_index, width};

    #[test]
    fn test_empty_line_is_empty() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let line = buffer.get_line(row(0)).unwrap();

        assert!(line.is_empty());
    }

    #[test]
    fn test_empty_line_dimensions() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let line = buffer.get_line(row(0)).unwrap();

        assert_eq!(line.byte_len(), len(0));
        assert_eq!(line.grapheme_count(), len(0));
        assert_eq!(line.display_width(), width(0));
    }

    #[test]
    fn test_empty_line_content_access() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let line = buffer.get_line(row(0)).unwrap();

        assert_eq!(line.content(), "");
        assert!(line.segments().is_empty());
    }

    #[test]
    fn test_empty_line_get_string_methods() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let line = buffer.get_line(row(0)).unwrap();

        // All get_string_at variants should return None for empty line.
        assert_eq!(line.get_string_at(col(0)), None);
        assert_eq!(line.get_string_at_right_of(col(0)), None);
        assert_eq!(line.get_string_at_left_of(col(0)), None);
        assert_eq!(line.get_string_at_end(), None);

        // Even with out-of-bounds indices.
        assert_eq!(line.get_string_at(col(10)), None);
        assert_eq!(line.get_string_at_right_of(col(10)), None);
        assert_eq!(line.get_string_at_left_of(col(10)), None);
    }

    #[test]
    fn test_empty_line_grapheme_checks() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let line = buffer.get_line(row(0)).unwrap();

        // Should not be in middle of grapheme for empty line.
        assert_eq!(line.check_is_in_middle_of_grapheme(col(0)), None);
        assert_eq!(line.check_is_in_middle_of_grapheme(col(5)), None);
    }

    #[test]
    fn test_transition_to_empty_line() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "test")
            .unwrap();

        // Verify line is not empty.
        let line = buffer.get_line(row(0)).unwrap();
        assert!(!line.is_empty());
        assert_eq!(line.grapheme_count(), len(4));

        // Clear the line by deleting all 4 graphemes.
        buffer
            .delete_range(row(0), seg_index(0), seg_index(4))
            .unwrap();

        // Verify line is now empty.
        let line = buffer.get_line(row(0)).unwrap();
        assert!(line.is_empty());
        assert_eq!(line.grapheme_count(), len(0));
        assert_eq!(line.byte_len(), len(0));
    }
}

/// Access Pattern Equivalence Tests
/// Ensure convenience methods provide the same results as direct API access
#[cfg(test)]
mod tests_access_pattern_equivalence {
    use super::test_fixtures::create_test_buffer;
    use crate::{UnitCompare, ZeroCopyGapBuffer, col, row, seg_index};

    #[test]
    fn test_display_width_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access.
        assert_eq!(line.display_width(), line.info().display_width);
    }

    #[test]
    fn test_grapheme_count_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access.
        assert_eq!(line.grapheme_count(), line.info().grapheme_count);
    }

    #[test]
    fn test_segments_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access.
        assert_eq!(line.segments(), &line.info().segments[..]);
    }

    #[test]
    fn test_byte_len_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access.
        assert_eq!(line.byte_len(), line.info().content_len);
    }

    #[test]
    fn test_get_string_at_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Test various column positions.
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call.
            let facade_result = line.get_string_at(col);
            let direct_result = line.info().get_string_at(line.content(), col);

            assert_eq!(
                facade_result, direct_result,
                "get_string_at mismatch at column {col_idx}"
            );
        }
    }

    #[test]
    fn test_get_string_at_right_of_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Test various column positions.
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call.
            let facade_result = line.get_string_at_right_of(col);
            let direct_result = line.info().get_string_at_right_of(line.content(), col);

            assert_eq!(
                facade_result, direct_result,
                "get_string_at_right_of mismatch at column {col_idx}"
            );
        }
    }

    #[test]
    fn test_get_string_at_left_of_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Test various column positions.
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call.
            let facade_result = line.get_string_at_left_of(col);
            let direct_result = line.info().get_string_at_left_of(line.content(), col);

            assert_eq!(
                facade_result, direct_result,
                "get_string_at_left_of mismatch at column {col_idx}"
            );
        }
    }

    #[test]
    fn test_get_string_at_end_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Facade method should equal direct LineMetadata call.
        let facade_result = line.get_string_at_end();
        let direct_result = line.info().get_string_at_end(line.content());

        assert_eq!(facade_result, direct_result);
    }

    #[test]
    fn test_check_is_in_middle_of_grapheme_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Test various column positions including middle of multi-width chars.
        for col_idx in 0..=(line.display_width().as_usize() + 2) {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call.
            let facade_result = line.check_is_in_middle_of_grapheme(col);
            let direct_result = line.info().check_is_in_middle_of_grapheme(col);

            assert_eq!(
                facade_result, direct_result,
                "check_is_in_middle_of_grapheme mismatch at column {col_idx}"
            );
        }
    }

    #[test]
    fn test_is_empty_equivalence() {
        // Test with non-empty line.
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        let facade_result = line.is_empty();
        let direct_result = line.info().grapheme_count.is_zero();

        assert_eq!(facade_result, direct_result);
        assert!(!facade_result); // Should not be empty

        // Test with empty line.
        let mut empty_buffer = ZeroCopyGapBuffer::new();
        empty_buffer.add_line();
        let empty_line = empty_buffer.get_line(row(0)).unwrap();

        let empty_facade_result = empty_line.is_empty();
        let empty_direct_result = empty_line.info().grapheme_count.is_zero();

        assert_eq!(empty_facade_result, empty_direct_result);
        assert!(empty_facade_result); // Should be empty
    }

    #[test]
    fn test_complex_unicode_equivalence() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert complex Unicode: family emoji, combining chars, etc.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "a👨‍👩‍👧‍👦é🇺🇸")
            .unwrap();

        let line = buffer.get_line(row(0)).unwrap();

        // Test all accessor methods with complex Unicode.
        assert_eq!(line.display_width(), line.info().display_width);
        assert_eq!(line.grapheme_count(), line.info().grapheme_count);
        assert_eq!(line.byte_len(), line.info().content_len);

        // Test string access methods.
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            assert_eq!(
                line.get_string_at(col),
                line.info().get_string_at(line.content(), col)
            );
        }
    }
}

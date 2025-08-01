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

//! [`GapBufferLine`] - A line view that combines content and metadata.
//!
//! This struct provides unified access to both line content and its associated
//! metadata, eliminating the need to work with split content/metadata APIs.

use super::LineMetadata;
use crate::{ColIndex, ColWidth, Length, Seg, SegStringOwned, len};

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

    /// Get the segments (grapheme cluster information) for the line.
    #[must_use]
    pub fn segments(&self) -> &[Seg] { &self.info.segments }

    /// Check if the given column index falls in the middle of a grapheme cluster.
    #[must_use]
    pub fn check_is_in_middle_of_grapheme(&self, col_index: ColIndex) -> Option<Seg> {
        self.info.check_is_in_middle_of_grapheme(col_index)
    }

    /// Get the string at the given column index.
    #[must_use]
    pub fn get_string_at(&self, col_index: ColIndex) -> Option<SegStringOwned> {
        self.info.get_string_at(self.content, col_index)
    }

    /// Get the string at the right of the given column index.
    #[must_use]
    pub fn get_string_at_right_of(&self, col_index: ColIndex) -> Option<SegStringOwned> {
        self.info.get_string_at_right_of(self.content, col_index)
    }

    /// Get the string at the left of the given column index.
    #[must_use]
    pub fn get_string_at_left_of(&self, col_index: ColIndex) -> Option<SegStringOwned> {
        self.info.get_string_at_left_of(self.content, col_index)
    }

    /// Get the last grapheme cluster in the line.
    #[must_use]
    pub fn get_string_at_end(&self) -> Option<SegStringOwned> {
        self.info.get_string_at_end(self.content)
    }

    /// Check if the line is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.info.grapheme_count == len(0) }

    /// Get content length in bytes.
    #[must_use]
    pub fn byte_len(&self) -> Length { self.info.content_len }
}

#[cfg(test)]
mod test_fixtures {
    use crate::{ZeroCopyGapBuffer, row, seg_index};

    /// Helper to create a test buffer with sample content
    pub(super) fn create_test_buffer() -> ZeroCopyGapBuffer {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert Unicode-rich content: "Hello 👋 世界!"
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

        // Content byte length should match metadata
        assert_eq!(line.content().len(), line.byte_len().as_usize());
        assert_eq!(line.content().len(), line.info().content_len.as_usize());
    }

    #[test]
    fn test_grapheme_count_consistency() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Count graphemes manually and compare with metadata
        let manual_count = line.content().graphemes(true).count();

        assert_eq!(manual_count, line.grapheme_count().as_usize());
        assert_eq!(manual_count, line.info().grapheme_count.as_usize());
    }

    #[test]
    fn test_display_width_consistency() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Display width should be consistent between facade and metadata
        assert_eq!(line.display_width(), line.info().display_width);
    }

    #[test]
    fn test_segments_consistency() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Segments should match between facade and metadata
        assert_eq!(line.segments().len(), line.info().segments.len());
        assert_eq!(line.segments(), &line.info().segments[..]);

        // Number of segments should match grapheme count
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

        // All get_string_at variants should return None for empty line
        assert_eq!(line.get_string_at(col(0)), None);
        assert_eq!(line.get_string_at_right_of(col(0)), None);
        assert_eq!(line.get_string_at_left_of(col(0)), None);
        assert_eq!(line.get_string_at_end(), None);

        // Even with out-of-bounds indices
        assert_eq!(line.get_string_at(col(10)), None);
        assert_eq!(line.get_string_at_right_of(col(10)), None);
        assert_eq!(line.get_string_at_left_of(col(10)), None);
    }

    #[test]
    fn test_empty_line_grapheme_checks() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let line = buffer.get_line(row(0)).unwrap();

        // Should not be in middle of grapheme for empty line
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

        // Verify line is not empty
        let line = buffer.get_line(row(0)).unwrap();
        assert!(!line.is_empty());
        assert_eq!(line.grapheme_count(), len(4));

        // Clear the line by deleting all 4 graphemes
        buffer
            .delete_range(row(0), seg_index(0), seg_index(4))
            .unwrap();

        // Verify line is now empty
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
    use crate::{ZeroCopyGapBuffer, col, len, row, seg_index};

    #[test]
    fn test_display_width_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access
        assert_eq!(line.display_width(), line.info().display_width);
    }

    #[test]
    fn test_grapheme_count_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access
        assert_eq!(line.grapheme_count(), line.info().grapheme_count);
    }

    #[test]
    fn test_segments_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access
        assert_eq!(line.segments(), &line.info().segments[..]);
    }

    #[test]
    fn test_byte_len_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Convenience method should equal direct access
        assert_eq!(line.byte_len(), line.info().content_len);
    }

    #[test]
    fn test_get_string_at_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Test various column positions
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call
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

        // Test various column positions
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call
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

        // Test various column positions
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call
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

        // Facade method should equal direct LineMetadata call
        let facade_result = line.get_string_at_end();
        let direct_result = line.info().get_string_at_end(line.content());

        assert_eq!(facade_result, direct_result);
    }

    #[test]
    fn test_check_is_in_middle_of_grapheme_equivalence() {
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        // Test various column positions including middle of multi-width chars
        for col_idx in 0..=(line.display_width().as_usize() + 2) {
            let col = col(col_idx);

            // Facade method should equal direct LineMetadata call
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
        // Test with non-empty line
        let buffer = create_test_buffer();
        let line = buffer.get_line(row(0)).unwrap();

        let facade_result = line.is_empty();
        let direct_result = line.info().grapheme_count == len(0);

        assert_eq!(facade_result, direct_result);
        assert!(!facade_result); // Should not be empty

        // Test with empty line
        let mut empty_buffer = ZeroCopyGapBuffer::new();
        empty_buffer.add_line();
        let empty_line = empty_buffer.get_line(row(0)).unwrap();

        let empty_facade_result = empty_line.is_empty();
        let empty_direct_result = empty_line.info().grapheme_count == len(0);

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

        // Test all accessor methods with complex Unicode
        assert_eq!(line.display_width(), line.info().display_width);
        assert_eq!(line.grapheme_count(), line.info().grapheme_count);
        assert_eq!(line.byte_len(), line.info().content_len);

        // Test string access methods
        for col_idx in 0..=line.grapheme_count().as_usize() {
            let col = col(col_idx);

            assert_eq!(
                line.get_string_at(col),
                line.info().get_string_at(line.content(), col)
            );
        }
    }
}

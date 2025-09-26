// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line metadata structure and operations.
//!
//! This module contains the [`LineMetadata`] struct which stores all the metadata
//! for a single line in the gap buffer, including buffer position, capacity,
//! grapheme segments, and display information.

use std::ops::Range;

use crate::{ByteIndex, ColIndex, ColWidth, GCStringOwned, IndexMarker, Length, Seg,
            SegIndex, SegStringOwned, SegmentArray, UnitCompare, byte_index};

/// Metadata for a single line in the buffer
#[derive(Debug, Clone, PartialEq)]
pub struct LineMetadata {
    /// Where this line starts in the buffer
    pub buffer_start_byte_index: ByteIndex,

    /// Actual content length in bytes (before '\n')
    pub content_len: Length,

    /// Allocated capacity for this line
    pub capacity: Length,

    /// Segment array for this line (grapheme cluster information)
    pub segments: SegmentArray,

    /// Display width of the line
    pub display_width: ColWidth,

    /// Number of grapheme clusters
    pub grapheme_count: Length,
}

impl LineMetadata {
    /// Get the buffer range for this line's content (excluding null padding)
    ///
    /// Returns a range that can be used to slice the buffer to get only the
    /// actual content bytes, not including the null padding that fills the
    /// remaining capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::ZeroCopyGapBuffer;
    ///
    /// let mut buffer = ZeroCopyGapBuffer::new();
    /// buffer.add_line();
    ///
    /// // Get the line info and content range
    /// let line_info = buffer.get_line_info(0).unwrap();
    /// let content_range = line_info.content_range();
    ///
    /// // For a newly created line, content should be empty (only newline is stored separately)
    /// assert_eq!(content_range.len(), 0);
    /// ```
    #[must_use]
    pub fn content_range(&self) -> Range<usize> {
        let start = self.buffer_start_byte_index.as_usize();
        let end = start + self.content_len.as_usize();
        start..end
    }

    /// Get the byte position for a given segment index
    ///
    /// This method converts a grapheme cluster index (segment index) to its
    /// corresponding byte position in the line buffer. It handles three cases:
    /// - Beginning of line (`seg_index` = 0) → returns byte position 0
    /// - End of line (`seg_index` >= `segments.len()`) → returns content length as byte
    ///   position
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
    /// use r3bl_tui::{ZeroCopyGapBuffer, seg_index, row};
    ///
    /// let mut buffer = ZeroCopyGapBuffer::new();
    /// buffer.add_line();
    /// buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hello").unwrap();
    ///
    /// let line_info = buffer.get_line_info(0).unwrap();
    ///
    /// // Beginning of line
    /// assert_eq!(line_info.get_byte_index(seg_index(0)).as_usize(), 0);
    ///
    /// // End of line
    /// assert_eq!(line_info.get_byte_index(seg_index(5)).as_usize(), 5);
    /// ```
    #[must_use]
    pub fn get_byte_index(&self, arg_seg_index: impl Into<SegIndex>) -> ByteIndex {
        let seg_index: SegIndex = arg_seg_index.into();
        if seg_index.is_zero() {
            // Beginning of line.
            byte_index(0)
        } else {
            if seg_index.overflows(self.segments.len()) {
                // End of line - return content length as byte position.
                byte_index(self.content_len.as_usize())
            } else {
                // Middle of line - return the start byte index of the target segment.
                let segment = &self.segments[seg_index.as_usize()];
                segment.start_byte_index // Already a ByteIndex!
            }
        }
    }

    /// Get the segment index for a given byte position
    ///
    /// This method converts a byte position to its corresponding grapheme cluster
    /// index (segment index). It handles three cases:
    /// - Beginning of line (`byte_ofs` = 0) → returns SegIndex(0)
    /// - End of line (`byte_ofs` >= `content_len`) → returns SegIndex(segments.len())
    /// - Middle of line → returns the `seg_index` of the segment containing the byte
    ///
    /// # Arguments
    /// * `byte_index` - The byte position within the line to convert
    ///
    /// # Returns
    /// The segment index where the byte position falls
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{ZeroCopyGapBuffer, byte_index, seg_index, row};
    ///
    /// let mut buffer = ZeroCopyGapBuffer::new();
    /// let line_idx = buffer.add_line();
    /// buffer.insert_text_at_grapheme(row(line_idx), seg_index(0), "H😀llo").unwrap();
    ///
    /// let line_info = buffer.get_line_info(line_idx).unwrap();
    ///
    /// // Beginning of line
    /// assert_eq!(line_info.get_seg_index(byte_index(0)), seg_index(0));
    ///
    /// // Middle of emoji (byte 3 is in the middle of the 4-byte emoji)
    /// assert_eq!(line_info.get_seg_index(byte_index(3)), seg_index(1));
    ///
    /// // After emoji
    /// assert_eq!(line_info.get_seg_index(byte_index(5)), seg_index(2));
    ///
    /// // End of line
    /// assert_eq!(line_info.get_seg_index(byte_index(8)), seg_index(5));
    /// ```
    #[must_use]
    pub fn get_seg_index(&self, arg_byte_index: impl Into<ByteIndex>) -> SegIndex {
        let byte_index: ByteIndex = arg_byte_index.into();
        // Handle edge cases.
        if byte_index.is_zero() {
            return crate::seg_index(0);
        }

        if byte_index.overflows(self.content_len) {
            return crate::seg_index(self.segments.len());
        }

        // Binary search through segments to find the one containing byte_index.
        // We could optimize this with binary search, but linear is fine for now
        // since lines typically have few segments.
        for segment in &self.segments {
            if byte_index.as_usize() >= segment.start_byte_index.as_usize()
                && byte_index.as_usize() < segment.end_byte_index.as_usize()
            {
                return segment.seg_index;
            }
        }

        // If we get here, byte_index is between segments (shouldn't happen with valid
        // UTF-8) Return the segment after the position.
        for segment in &self.segments {
            if byte_index.as_usize() < segment.start_byte_index.as_usize() {
                return segment.seg_index;
            }
        }

        // Fallback to end of line.
        crate::seg_index(self.segments.len())
    }

    /// Check if the given display column index falls in the middle of a grapheme cluster.
    ///
    /// This method ensures Unicode correctness by detecting when a cursor position
    /// would split a grapheme cluster (which is not allowed). It returns the segment
    /// that would be split if the column index is in the middle of a grapheme.
    ///
    /// This replaces the editor-specific `at_display_col_index` module functionality
    /// from `GCStringOwned` and integrates Unicode correctness directly into the
    /// gap buffer line metadata.
    ///
    /// # Arguments
    /// * `col_index` - The display column index to check
    ///
    /// # Returns
    /// * `Some(Seg)` if the column index falls in the middle of a grapheme cluster
    /// * `None` if the column index is at a valid cursor position (start of a grapheme)
    ///
    /// # Example
    /// ```rust
    /// use r3bl_tui::*;
    ///
    /// // For a line with "Hi📦" where 📦 is 2 columns wide:
    /// // Valid positions: 0 (before H), 1 (before i), 2 (before 📦), 4 (after 📦)
    /// // Invalid position: 3 (middle of 📦)
    ///
    /// # let mut buffer = ZeroCopyGapBuffer::new();
    /// # buffer.add_line();
    /// # buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hi📦").unwrap();
    /// # let line = buffer.get_line(row(0)).unwrap();
    /// # let line_info = line.info();
    ///
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(0)).is_none()); // Valid
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(1)).is_none()); // Valid
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(2)).is_none()); // Valid
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(3)).is_some()); // Invalid!
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(4)).is_none()); // Valid
    /// ```
    #[must_use]
    pub fn check_is_in_middle_of_grapheme(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<Seg> {
        let col_index: ColIndex = arg_col_index.into();
        // Find the segment that contains or would contain this column index.
        for seg in &self.segments {
            let seg_start = seg.start_display_col_index;
            let seg_end = seg_start + seg.display_width;

            // Check if the column index falls within this segment.
            if col_index >= seg_start && col_index < seg_end {
                // If it's not at the start of the segment, it's in the middle.
                if col_index != seg_start {
                    return Some(*seg);
                }
                // If it is at the start, this is a valid cursor position.
                return None;
            }
        }

        // Column index is beyond all segments (end of line) - valid position
        None
    }

    /// Get a string slice at the given column index.
    /// This method provides GCString-compatible behavior for editor operations.
    ///
    /// # Arguments
    /// * `content` - The line content as a string slice
    /// * `col_index` - The display column index to get string at
    ///
    /// # Returns
    /// A `SegStringOwned` representing the grapheme cluster at the specified column,
    /// or `None` if the column is out of bounds.
    ///
    /// # Usage Pattern
    /// ```rust
    /// # use r3bl_tui::{ZeroCopyGapBuffer, row, col};
    /// # let mut buffer = ZeroCopyGapBuffer::new();
    /// # buffer.add_line();
    /// let line = buffer.get_line(row(0)).unwrap();
    /// let seg_string = line.info().get_string_at(line.content(), col(0));
    /// ```
    #[must_use]
    pub fn get_string_at(
        &self,
        content: &str,
        col_index: ColIndex,
    ) -> Option<SegStringOwned> {
        // Find the segment at the given column index.
        let target_col = col_index.as_usize();

        for segment in &self.segments {
            let seg_start_col = segment.start_display_col_index.as_usize();
            let seg_width = segment.display_width.as_usize();

            if target_col >= seg_start_col && target_col < seg_start_col + seg_width {
                // Extract the segment's string content.
                let start_byte = segment.start_byte_index.as_usize();
                let end_byte = segment.end_byte_index.as_usize();
                let seg_content = &content[start_byte..end_byte];

                return Some(SegStringOwned {
                    string: GCStringOwned::from(seg_content),
                    width: segment.display_width,
                    start_at: segment.start_display_col_index,
                });
            }
        }

        None
    }

    /// Get a string slice to the right of the given column index.
    /// This method provides GCString-compatible behavior for editor operations.
    #[must_use]
    pub fn get_string_at_right_of(
        &self,
        content: &str,
        col_index: ColIndex,
    ) -> Option<SegStringOwned> {
        // Find the segment after the given column index.
        let target_col = col_index.as_usize();

        for segment in &self.segments {
            let seg_start_col = segment.start_display_col_index.as_usize();

            if seg_start_col > target_col {
                // This is the first segment to the right.
                let start_byte = segment.start_byte_index.as_usize();
                let end_byte = segment.end_byte_index.as_usize();
                let seg_content = &content[start_byte..end_byte];

                return Some(SegStringOwned {
                    string: GCStringOwned::from(seg_content),
                    width: segment.display_width,
                    start_at: segment.start_display_col_index,
                });
            }
        }

        None
    }

    /// Get a string slice to the left of the given column index.
    /// This method provides GCString-compatible behavior for editor operations.
    #[must_use]
    pub fn get_string_at_left_of(
        &self,
        content: &str,
        col_index: ColIndex,
    ) -> Option<SegStringOwned> {
        // Find the segment before the given column index.
        let target_col = col_index.as_usize();
        let mut last_valid_segment: Option<&Seg> = None;

        for segment in &self.segments {
            let seg_start_col = segment.start_display_col_index.as_usize();
            let seg_width = segment.display_width.as_usize();
            let seg_end_col = seg_start_col + seg_width;

            if seg_end_col <= target_col {
                last_valid_segment = Some(segment);
            } else {
                break;
            }
        }

        if let Some(segment) = last_valid_segment {
            let start_byte = segment.start_byte_index.as_usize();
            let end_byte = segment.end_byte_index.as_usize();
            let seg_content = &content[start_byte..end_byte];

            Some(SegStringOwned {
                string: GCStringOwned::from(seg_content),
                width: segment.display_width,
                start_at: segment.start_display_col_index,
            })
        } else {
            None
        }
    }

    /// Get the string at the end (last segment).
    /// This method provides GCString-compatible behavior for editor operations.
    #[must_use]
    pub fn get_string_at_end(&self, content: &str) -> Option<SegStringOwned> {
        let last_segment = self.segments.last()?;

        let start_byte = last_segment.start_byte_index.as_usize();
        let end_byte = last_segment.end_byte_index.as_usize();
        let seg_content = &content[start_byte..end_byte];

        Some(SegStringOwned {
            string: GCStringOwned::from(seg_content),
            width: last_segment.display_width,
            start_at: last_segment.start_display_col_index,
        })
    }

    /// Clip the line content to a specific display column range.
    ///
    /// This method extracts a substring from the line content based on display column
    /// indices, properly handling Unicode grapheme clusters and multi-width characters.
    /// This is the zero-copy equivalent of `GCStringOwned::clip()`.
    ///
    /// # Arguments
    /// * `content` - The line content as a string slice
    /// * `start_col_index` - The starting display column (0-based)
    /// * `max_col_width` - The maximum display width to include
    ///
    /// # Returns
    /// A string slice containing the clipped content, or empty string if the range is
    /// invalid
    ///
    /// # Unicode Safety
    /// This method properly handles:
    /// - Multi-byte UTF-8 characters
    /// - Emoji and other wide characters
    /// - Complex grapheme clusters (e.g., family emoji with zero-width joiners)
    /// - Characters with display width different from byte length
    ///
    /// # Example
    /// ```text
    /// Content: "Hi📦XelLo🙏🏽Bye"
    /// Columns:  0123456789AB  (A=10, B=11)
    ///
    /// clip_to_range(content, col(2), width(4)) → "📦Xe"
    /// clip_to_range(content, col(6), width(3)) → "Lo🙏🏽" (note: 🙏🏽 has width 2)
    /// ```
    #[must_use]
    pub fn clip_to_range<'a>(
        &'a self,
        content: &'a str,
        start_col_index: ColIndex,
        max_col_width: ColWidth,
    ) -> &'a str {
        if self.segments.is_empty() || content.is_empty() {
            return "";
        }

        // Find the starting byte index by skipping display columns.
        let string_start_byte_index = {
            let mut byte_index = 0;
            let mut skip_col_count = start_col_index;

            for seg in &self.segments {
                let seg_display_width = seg.display_width;

                // If we've skipped enough columns, stop here.
                if skip_col_count.is_zero() {
                    break;
                }

                // Skip this segment's width.
                skip_col_count -= seg_display_width;
                byte_index += seg.bytes_size.as_usize();
            }
            byte_index
        };

        // Find the ending byte index by consuming available column width.
        let string_end_byte_index = {
            let mut byte_index = 0;
            let mut avail_col_count = max_col_width;
            let mut skip_col_count = start_col_index;

            for seg in &self.segments {
                let seg_display_width = seg.display_width;

                // Are we still skipping columns to reach the start?
                if skip_col_count.is_zero() {
                    // We're in the content area - check if we have room for this segment.
                    if avail_col_count < seg_display_width {
                        // This segment would exceed our width limit.
                        break;
                    }
                    byte_index += seg.bytes_size.as_usize();
                    avail_col_count -= seg_display_width;
                } else {
                    // Still skipping to reach start position.
                    skip_col_count -= seg_display_width;
                    byte_index += seg.bytes_size.as_usize();
                }
            }
            byte_index
        };

        // Ensure we don't go out of bounds.
        let content_len = content.len();
        let start = string_start_byte_index.min(content_len);
        let end = string_end_byte_index.min(content_len);

        if start <= end {
            &content[start..end]
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ZeroCopyGapBuffer, byte_index, col, row, seg_index, width};

    #[test]
    fn test_get_byte_pos_beginning() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test beginning position.
        assert_eq!(line_info.get_byte_index(seg_index(0)).as_usize(), 0);
    }

    #[test]
    fn test_get_byte_pos_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test end position (past last segment)
        assert_eq!(line_info.get_byte_index(seg_index(5)).as_usize(), 5);
        assert_eq!(line_info.get_byte_index(seg_index(10)).as_usize(), 5);
    }

    #[test]
    fn test_get_byte_pos_middle() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with multi-byte characters.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "H😀llo")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test various positions.
        assert_eq!(line_info.get_byte_index(seg_index(0)).as_usize(), 0); // Before 'H'
        assert_eq!(line_info.get_byte_index(seg_index(1)).as_usize(), 1); // Before '😀'
        assert_eq!(line_info.get_byte_index(seg_index(2)).as_usize(), 5); // Before 'l' (emoji is 4 bytes)
        assert_eq!(line_info.get_byte_index(seg_index(3)).as_usize(), 6); // Before second 'l'
        assert_eq!(line_info.get_byte_index(seg_index(4)).as_usize(), 7); // Before 'o'
        assert_eq!(line_info.get_byte_index(seg_index(5)).as_usize(), 8); // End of string
    }

    #[test]
    fn test_get_byte_pos_empty_line() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        let line_info = buffer.get_line_info(0).unwrap();

        // For empty line, any position should return 0.
        assert_eq!(line_info.get_byte_index(seg_index(0)).as_usize(), 0);
        assert_eq!(line_info.get_byte_index(seg_index(1)).as_usize(), 0);
        assert_eq!(line_info.get_byte_index(seg_index(100)).as_usize(), 0);
    }

    #[test]
    fn test_get_seg_index_beginning() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test beginning position.
        assert_eq!(line_info.get_seg_index(byte_index(0)), seg_index(0));
    }

    #[test]
    fn test_get_seg_index_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert some text.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test end position (at or past content length)
        assert_eq!(line_info.get_seg_index(byte_index(5)), seg_index(5));
        assert_eq!(line_info.get_seg_index(byte_index(10)), seg_index(5));
    }

    #[test]
    fn test_get_seg_index_middle() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with emoji: "H😀llo".
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "H😀llo")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test various byte positions.
        assert_eq!(line_info.get_seg_index(byte_index(0)), seg_index(0)); // Start of 'H'
        assert_eq!(line_info.get_seg_index(byte_index(1)), seg_index(1)); // Start of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(2)), seg_index(1)); // Middle of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(3)), seg_index(1)); // Middle of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(4)), seg_index(1)); // End of '😀'
        assert_eq!(line_info.get_seg_index(byte_index(5)), seg_index(2)); // Start of 'l'
        assert_eq!(line_info.get_seg_index(byte_index(6)), seg_index(3)); // Start of second 'l'
        assert_eq!(line_info.get_seg_index(byte_index(7)), seg_index(4)); // Start of 'o'
        assert_eq!(line_info.get_seg_index(byte_index(8)), seg_index(5)); // End of string
    }

    #[test]
    fn test_get_seg_index_empty_line() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        let line_info = buffer.get_line_info(0).unwrap();

        // For empty line, any position should return 0.
        assert_eq!(line_info.get_seg_index(byte_index(0)), seg_index(0));
        assert_eq!(line_info.get_seg_index(byte_index(1)), seg_index(0));
        assert_eq!(line_info.get_seg_index(byte_index(100)), seg_index(0));
    }

    #[test]
    fn test_get_seg_index_get_byte_pos_round_trip() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with various Unicode: "a👨‍👩‍👧‍👦b世界c".
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "a👨‍👩‍👧‍👦b世界c")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test round-trip conversion for each segment.
        for i in 0..line_info.segments.len() {
            let seg_idx = seg_index(i);
            let byte_pos = line_info.get_byte_index(seg_idx);
            let seg_idx_back = line_info.get_seg_index(byte_pos);
            assert_eq!(
                seg_idx,
                seg_idx_back,
                "Round-trip failed for segment {}: byte_pos={}",
                i,
                byte_pos.as_usize()
            );
        }
    }

    #[test]
    fn test_gap_buffer_line_info_clip_to_range() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert Unicode-rich content: "Hi📦XelLo🙏🏽Bye".
        // Display layout:
        // H(1) i(1) 📦(2) X(1) e(1) l(1) L(1) o(1) 🙏🏽(2) B(1) y(1) e(1) = 14 total width
        // Columns: 0    1   23     4    5   6   7   8   9A      B    C   D
        // (A=10,B=11,C=12,D=13)
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hi📦XelLo🙏🏽Bye")
            .unwrap();

        let line = buffer.get_line(row(0)).unwrap();
        let content = line.content();
        let line_info = line.info();

        // Test: Clip from start
        let result = line_info.clip_to_range(content, col(0), width(2));
        assert_eq!(result, "Hi");

        // Test: Clip emoji (starts at col 2, has width 2)
        let result = line_info.clip_to_range(content, col(2), width(2));
        assert_eq!(result, "📦");

        // Test: Clip across emoji boundary
        let result = line_info.clip_to_range(content, col(2), width(4));
        assert_eq!(result, "📦Xe");

        // Test: Clip multi-width emoji 🙏🏽 (starts at col 9, has width 2)
        let result = line_info.clip_to_range(content, col(9), width(2));
        assert_eq!(result, "🙏🏽");

        // Test: Clip including multi-width emoji
        let result = line_info.clip_to_range(content, col(6), width(5));
        assert_eq!(result, "lLo🙏🏽");

        // Test: Clip from middle to end
        let result = line_info.clip_to_range(content, col(11), width(10));
        assert_eq!(result, "Bye");

        // Test: Empty clip (beyond content)
        let result = line_info.clip_to_range(content, col(20), width(5));
        assert_eq!(result, "");

        // Test: Zero width
        let result = line_info.clip_to_range(content, col(5), width(0));
        assert_eq!(result, "");

        // Test: Empty line
        let mut empty_buffer = ZeroCopyGapBuffer::new();
        empty_buffer.add_line();
        let empty_line = empty_buffer.get_line(row(0)).unwrap();
        let result =
            empty_line
                .info()
                .clip_to_range(empty_line.content(), col(0), width(5));
        assert_eq!(result, "");
    }
}

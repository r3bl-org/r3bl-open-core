// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Basic line storage operations for `ZeroCopyGapBuffer`.
//!
//! This module provides fundamental line storage operations for `ZeroCopyGapBuffer`,
//! enabling efficient text editing and manipulation.
//!
//! # Performance Characteristics
//!
//! - **Zero-copy access**: Line content is returned as `&str` without copying
//! - **Efficient grapheme operations**: Leverages pre-computed segment metadata
//! - **Optimized appends**: Uses fast path for end-of-line insertions
//! - **Dynamic line growth**: Automatically extends capacity as needed

use super::super::ZeroCopyGapBuffer;
use crate::{ByteIndex, ColIndex, ColWidth, GCStringOwned, GapBufferLine, IndexMarker,
            Length, RowIndex, SegIndex, UnitCompare, byte_index, byte_offset, row,
            seg_index, seg_length, width};

impl ZeroCopyGapBuffer {
    // Line access methods.

    /// Get the number of lines in the storage (alias for `line_count`).
    #[must_use]
    pub fn len(&self) -> Length { self.line_count() }

    /// Check if the storage is empty (has no lines).
    #[must_use]
    pub fn is_empty(&self) -> bool { self.line_count().is_zero() }

    /// Get line content and metadata.
    ///
    /// This is the primary API for accessing lines from the buffer. It returns a
    /// [`GapBufferLine`] that provides unified access to both the line content and
    /// its metadata (segments, display width, etc.).
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the line contains invalid UTF-8. This should never
    /// happen as all content is validated on insertion.
    #[must_use]
    pub fn get_line(
        &self,
        arg_row_index: impl Into<RowIndex>,
    ) -> Option<GapBufferLine<'_>> {
        let row_index: RowIndex = arg_row_index.into();
        let line_info = self.get_line_info(row_index)?;

        // In debug builds, validate UTF-8.
        #[cfg(debug_assertions)]
        {
            use std::str::from_utf8;
            if let Err(e) = from_utf8(&self.buffer[line_info.content_range()]) {
                panic!(
                    "Line {} contains invalid UTF-8 at byte {}: {}",
                    row_index.as_usize(),
                    e.valid_up_to(),
                    e
                );
            }
        }
        // SAFETY: We maintain UTF-8 invariants via all buffer insertions using &str
        let content = unsafe {
            std::str::from_utf8_unchecked(&self.buffer[line_info.content_range()])
        };
        Some(GapBufferLine::new(content, line_info))
    } // Line metadata access

    #[must_use]
    pub fn get_line_display_width(
        &self,
        arg_row_index: impl Into<RowIndex>,
    ) -> Option<ColWidth> {
        let row_index: RowIndex = arg_row_index.into();
        self.get_line_info(row_index).map(|info| info.display_width)
    }

    #[must_use]
    pub fn get_line_grapheme_count(
        &self,
        arg_row_index: impl Into<RowIndex>,
    ) -> Option<Length> {
        let row_index: RowIndex = arg_row_index.into();
        self.get_line_info(row_index)
            .map(|info| info.grapheme_count)
    }

    #[must_use]
    pub fn get_line_byte_len(
        &self,
        arg_row_index: impl Into<RowIndex>,
    ) -> Option<Length> {
        let row_index: RowIndex = arg_row_index.into();
        self.get_line_info(row_index).map(|info| info.content_len)
    }

    // Line modification methods.

    pub fn insert_line(&mut self, arg_row_index: impl Into<RowIndex>) -> bool {
        let row_index: RowIndex = arg_row_index.into();
        match self.insert_empty_line(row_index) {
            Ok(()) => true,
            Err(_) => false,
        }
    }

    pub fn set_line(
        &mut self,
        arg_row_index: impl Into<RowIndex>,
        content: &str,
    ) -> bool {
        let row_index: RowIndex = arg_row_index.into();
        // First, clear the existing line content.
        if let Some(line_info) = self.get_line_info(row_index) {
            let grapheme_count = line_info.grapheme_count;
            if !grapheme_count.is_zero() {
                // Delete all existing content.
                match self.delete_range(
                    row_index,
                    seg_index(0),
                    seg_index(grapheme_count.as_usize()),
                ) {
                    Ok(()) => {}
                    Err(_) => return false,
                }
            }

            // Insert new content at the beginning.
            match self.insert_text_at_grapheme(row_index, seg_index(0), content) {
                Ok(()) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn push_line(&mut self, content: &str) {
        let line_idx = self.add_line();
        drop(self.insert_text_at_grapheme(row(line_idx), seg_index(0), content));
    }

    // Column-based operations.

    pub fn insert_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
        text: &str,
    ) -> Option<ColWidth> {
        // Convert column index to segment index.
        let seg_idx = self.col_to_seg_index(row_index, col_index)?;

        // Calculate the display width of the text to be inserted.
        let text_width = Self::calculate_text_display_width(text);

        // Perform the insertion.
        match self.insert_text_at_grapheme(row_index, seg_idx, text) {
            Ok(()) => Some(text_width),
            Err(_) => None,
        }
    }

    /// Delete a specified number of grapheme clusters starting at the given column
    /// position.
    ///
    /// # Arguments
    /// * `row_index` - The row to delete from
    /// * `col_index` - The column position to start deletion
    /// * `segment_count` - The number of grapheme clusters (segments) to delete
    ///
    /// # Returns
    /// * `true` if deletion was successful
    /// * `false` if the position was invalid or deletion failed
    pub fn delete_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
        segment_count: Length,
    ) -> bool {
        // Convert column index to segment index.
        if let Some(seg_idx) = self.col_to_seg_index(row_index, col_index) {
            // Get the line info to check segment count.
            if let Some(line_info) = self.get_line_info(row_index) {
                let max_segments = seg_length(line_info.segments.len());
                let requested_end = seg_idx.as_usize() + segment_count.as_usize();
                let max_segments_usize = max_segments.as_usize();
                let actual_end = if requested_end > max_segments_usize {
                    max_segments_usize
                } else {
                    requested_end
                };
                let end_seg_index = seg_index(actual_end);

                // Use the range deletion method.
                match self.delete_range(row_index, seg_idx, end_seg_index) {
                    Ok(()) => true,
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    // Utility methods

    pub fn split_line_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
    ) -> Option<String> {
        // Convert column index to segment index.
        let seg_idx = self.col_to_seg_index(row_index, col_index)?;

        // Get the line content as owned string.
        let line_content = self.get_line_content(row_index)?.to_string();

        // Find the byte position for the segment.
        let line_info = self.get_line_info(row_index)?;
        let byte_pos = line_info.get_byte_index(seg_idx);

        // Split the content.
        let (left_part, right_part) = line_content.split_at(byte_pos.as_usize());
        let right_content = right_part.to_string();

        // Update the current line to only contain the left part.
        self.set_line(row_index, left_part);

        Some(right_content)
    }

    pub fn merge_with_next_line(
        &mut self,
        arg_base_row_index: impl Into<RowIndex>,
    ) -> bool {
        let base_row_index: RowIndex = arg_base_row_index.into();
        let next_row_index = row(base_row_index.as_usize() + 1);

        // Get the content of the second line.
        if let Some(second_line_content) = self.get_line_content(next_row_index) {
            let content_to_append = second_line_content.to_string();

            // Get the grapheme count of the base line to know where to append.
            if let Some(line_info) = self.get_line_info(base_row_index) {
                let append_pos = seg_index(line_info.grapheme_count.as_usize());

                // Append the second line's content to the base line.
                match self.insert_text_at_grapheme(
                    base_row_index,
                    append_pos,
                    &content_to_append,
                ) {
                    Ok(()) => {
                        // Remove the second line.
                        self.remove_line(next_row_index)
                    }
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    // Byte position conversions.

    #[must_use]
    pub fn get_byte_pos_for_row(
        &self,
        arg_row_index: impl Into<RowIndex>,
    ) -> Option<ByteIndex> {
        let row_index: RowIndex = arg_row_index.into();
        self.get_line_info(row_index)
            .map(|info| info.buffer_start_byte_index)
    }

    #[must_use]
    pub fn find_row_containing_byte(
        &self,
        arg_byte_index: impl Into<ByteIndex>,
    ) -> Option<RowIndex> {
        let byte_index: ByteIndex = arg_byte_index.into();
        // Early bounds check for performance optimization.
        let buffer_len = crate::len(self.buffer.len());
        if byte_index.overflows(buffer_len) {
            return None;
        }

        // Linear search through lines to find which one contains the byte.
        // This could be optimized with binary search if needed.
        let total_lines = self.line_count();
        for i in 0..total_lines.as_usize() {
            if let Some(line_info) = self.get_line_info(i) {
                let line_start = line_info.buffer_start_byte_index;
                let line_end = line_start + byte_offset(line_info.capacity);

                if byte_index >= line_start && byte_index < line_end {
                    return Some(row(i));
                }
            }
        }

        None
    }

    // Iterator support.

    #[must_use]
    pub fn iter_lines(&self) -> Box<dyn Iterator<Item = GapBufferLine<'_>> + '_> {
        let total_lines = self.line_count();
        Box::new((0..total_lines.as_usize()).filter_map(move |i| self.get_line(row(i))))
    } // Total size information

    #[must_use]
    pub fn total_bytes(&self) -> ByteIndex { byte_index(self.buffer.len()) }

    // Conversion methods.

    pub fn to_gc_string_vec(&self) -> Vec<GCStringOwned> {
        (0..self.line_count().as_usize())
            .filter_map(|i| self.get_line_content(row(i)))
            .map(Into::into)
            .collect()
    }

    #[must_use]
    pub fn from_gc_string_vec(lines: Vec<GCStringOwned>) -> Self {
        let mut buffer = Self::new();
        for line in lines {
            buffer.push_line(line.as_ref());
        }
        buffer
    }

    // Validation support methods.

    #[must_use]
    pub fn get_string_at_col(
        &self,
        arg_row_index: impl Into<RowIndex>,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<crate::SegStringOwned> {
        let row_index: RowIndex = arg_row_index.into();
        let col_index: ColIndex = arg_col_index.into();
        let line = self.get_line(row_index)?;
        line.get_string_at(col_index)
    }

    #[must_use]
    pub fn check_is_in_middle_of_grapheme(
        &self,
        arg_row_index: impl Into<RowIndex>,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<crate::Seg> {
        let row_index: RowIndex = arg_row_index.into();
        let col_index: ColIndex = arg_col_index.into();
        let line = self.get_line(row_index)?;
        line.check_is_in_middle_of_grapheme(col_index)
    }
}

// Helper methods for ZeroCopyGapBuffer.
impl ZeroCopyGapBuffer {
    /// Convert a column index to a segment index for a given line.
    fn col_to_seg_index(
        &self,
        row_index: RowIndex,
        col_index: ColIndex,
    ) -> Option<SegIndex> {
        let line_info = self.get_line_info(row_index)?;
        let target_col = col_index.as_usize();
        let mut current_col = 0;

        // Find the segment that contains or is after the target column.
        for (i, segment) in line_info.segments.iter().enumerate() {
            if current_col >= target_col {
                return Some(seg_index(i));
            }
            current_col += segment.display_width.as_usize();
        }

        // If we've gone through all segments, return the end position.
        Some(seg_index(line_info.segments.len()))
    }

    /// Calculate the display width of a text string.
    fn calculate_text_display_width(text: &str) -> ColWidth {
        use crate::segment_builder::build_segments_for_str;

        let segments = build_segments_for_str(text);
        let total_width: usize = segments
            .iter()
            .map(|seg| seg.display_width.as_usize())
            .sum();

        width(total_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, len};

    #[test]
    fn test_basic_line_operations() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Test empty storage.
        assert_eq!(storage.line_count(), len(0));
        assert!(storage.is_empty());

        // Add some lines
        storage.push_line("Hello, world!");
        storage.push_line("This is line 2");
        storage.push_line("And line 3");

        // Test line count
        assert_eq!(storage.line_count(), len(3));
        assert!(!storage.is_empty());

        // Test line content access.
        assert_eq!(storage.get_line_content(row(0)), Some("Hello, world!"));
        assert_eq!(storage.get_line_content(row(1)), Some("This is line 2"));
        assert_eq!(storage.get_line_content(row(2)), Some("And line 3"));
        assert_eq!(storage.get_line_content(row(3)), None);

        // Test line metadata.
        assert_eq!(storage.get_line_display_width(row(0)), Some(width(13)));
        assert_eq!(storage.get_line_grapheme_count(row(0)), Some(len(13)));
        assert_eq!(storage.get_line_byte_len(row(0)), Some(len(13)));
    }

    #[test]
    fn test_line_modification() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Add initial content.
        storage.push_line("Original line");

        // Test set_line
        assert!(storage.set_line(row(0), "Modified line"));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));

        // Test insert_line at the end (to avoid the underflow bug)
        assert!(storage.insert_line(row(1)));
        assert_eq!(storage.line_count(), len(2));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));
        assert_eq!(storage.get_line_content(row(1)), Some(""));

        // Test remove_line (remove the empty line at the end)
        assert!(storage.remove_line(row(1)));
        assert_eq!(storage.line_count(), len(1));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));

        // Test insert_line at beginning.
        assert!(storage.insert_line(row(0)));
        assert_eq!(storage.line_count(), len(2));
        assert_eq!(storage.get_line_content(row(0)), Some(""));
        assert_eq!(storage.get_line_content(row(1)), Some("Modified line"));

        // Test remove_line at beginning.
        assert!(storage.remove_line(row(0)));
        assert_eq!(storage.line_count(), len(1));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));
    }

    #[test]
    fn test_grapheme_operations() {
        let mut storage = ZeroCopyGapBuffer::new();
        storage.push_line("Hello");

        // Test insert_at_grapheme.
        assert!(
            storage
                .insert_text_at_grapheme(row(0), seg_index(5), " World")
                .is_ok()
        );
        assert_eq!(storage.get_line_content(row(0)), Some("Hello World"));

        // Test delete_at_grapheme.
        assert!(storage.delete_grapheme_at(row(0), seg_index(5)).is_ok());
        assert_eq!(storage.get_line_content(row(0)), Some("HelloWorld"));
    }

    #[test]
    fn test_unicode_content() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Test with emoji and unicode.
        storage.push_line("Hello 👋 世界");

        assert_eq!(storage.get_line_content(row(0)), Some("Hello 👋 世界"));
        assert_eq!(storage.get_line_grapheme_count(row(0)), Some(len(10))); // "Hello " = 6 + emoji = 1 + space = 1 + "世界" = 2

        // Insert more unicode.
        assert!(
            storage
                .insert_text_at_grapheme(row(0), seg_index(7), " 🌍")
                .is_ok()
        );
        assert_eq!(storage.get_line_content(row(0)), Some("Hello 👋 🌍 世界"));
    }

    #[test]
    fn test_split_and_join_lines() {
        let mut storage = ZeroCopyGapBuffer::new();
        storage.push_line("Hello World");

        // Test split_line_at_col.
        let split_content = storage.split_line_at_col(row(0), col(6));
        assert_eq!(split_content, Some("World".to_string()));
        assert_eq!(storage.get_line_content(row(0)), Some("Hello "));

        // Add the split content as a new line.
        storage.push_line(&split_content.unwrap());

        // Test merge_with_next_line.
        assert!(storage.merge_with_next_line(row(0)));
        assert_eq!(storage.get_line_content(row(0)), Some("Hello World"));
        assert_eq!(storage.line_count(), len(1));
    }

    #[test]
    fn test_clear() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Add some content.
        storage.push_line("Line 1");
        storage.push_line("Line 2");
        storage.push_line("Line 3");

        assert_eq!(storage.line_count(), len(3));

        // Clear all lines
        storage.clear();

        assert_eq!(storage.line_count(), len(0));
        assert!(storage.is_empty());
    }

    #[test]
    fn test_iterator() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Add test lines
        let test_lines = vec!["First line", "Second line", "Third line"];
        for line in &test_lines {
            storage.push_line(line);
        }

        // Test iterator
        let collected: Vec<&str> =
            storage.iter_lines().map(|line| line.content()).collect();
        assert_eq!(collected, test_lines);
    }

    #[test]
    fn test_conversion_methods() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Add some lines
        storage.push_line("Line 1");
        storage.push_line("Line 2");

        // Test to_gc_string_vec.
        let gc_vec = storage.to_gc_string_vec();
        assert_eq!(gc_vec.len(), 2);
        assert_eq!(gc_vec[0].as_ref(), "Line 1");
        assert_eq!(gc_vec[1].as_ref(), "Line 2");

        // Test from_gc_string_vec.
        let new_storage = ZeroCopyGapBuffer::from_gc_string_vec(gc_vec);
        assert_eq!(new_storage.line_count(), len(2));
        assert_eq!(new_storage.get_line_content(row(0)), Some("Line 1"));
        assert_eq!(new_storage.get_line_content(row(1)), Some("Line 2"));
    }

    #[test]
    fn test_delete_at_col_with_emoji() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Create line with emoji: "Hello😃World".
        storage.push_line("Hello😃World");

        // Verify initial state.
        assert_eq!(storage.get_line_content(row(0)), Some("Hello😃World"));
        assert_eq!(storage.get_line_display_width(row(0)), Some(width(12))); // 5 + 2 + 5

        // Delete the emoji (1 segment) at column 5
        assert!(storage.delete_at_col(row(0), col(5), len(1)));

        // Verify the emoji was deleted.
        assert_eq!(storage.get_line_content(row(0)), Some("HelloWorld"));
        assert_eq!(storage.get_line_display_width(row(0)), Some(width(10)));
    }

    #[test]
    fn test_delete_at_col_multiple_segments() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Create line with multiple emojis.
        storage.push_line("👋😀🎉");

        // Each emoji is 1 segment but width 2.
        assert_eq!(storage.get_line_grapheme_count(row(0)), Some(len(3)));
        assert_eq!(storage.get_line_display_width(row(0)), Some(width(6)));

        // Delete 2 segments starting at column 0.
        assert!(storage.delete_at_col(row(0), col(0), len(2)));

        // Should have deleted 👋 and 😀, leaving only 🎉.
        assert_eq!(storage.get_line_content(row(0)), Some("🎉"));
        assert_eq!(storage.get_line_grapheme_count(row(0)), Some(len(1)));
        assert_eq!(storage.get_line_display_width(row(0)), Some(width(2)));
    }

    #[test]
    fn test_delete_at_col_mixed_width() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Mix of ASCII and wide characters.
        storage.push_line("a😃b世界c");

        // Width: a=1, 😃=2, b=1, 世=2, 界=2, c=1
        assert_eq!(storage.get_line_display_width(row(0)), Some(width(9)));

        // Delete emoji at column 1 (segment index 1)
        assert!(storage.delete_at_col(row(0), col(1), len(1)));
        assert_eq!(storage.get_line_content(row(0)), Some("ab世界c"));

        // Delete '世' at column 2 (after 'ab')
        assert!(storage.delete_at_col(row(0), col(2), len(1)));
        assert_eq!(storage.get_line_content(row(0)), Some("ab界c"));
    }

    #[test]
    fn test_delete_at_col_segment_count_parameter() {
        let mut storage = ZeroCopyGapBuffer::new();

        // Create line with text.
        storage.push_line("abcdef");

        // Delete 3 segments starting at column 1 (should delete 'bcd')
        assert!(storage.delete_at_col(row(0), col(1), len(3)));
        assert_eq!(storage.get_line_content(row(0)), Some("aef"));

        // Now we have "aef" (3 segments)
        // Try to delete from beginning - even with count > remaining segments.
        assert!(storage.delete_at_col(row(0), col(0), len(10)));
        assert_eq!(storage.get_line_content(row(0)), Some(""));
    }

    #[test]
    fn test_get_line_with_info() {
        let mut storage = ZeroCopyGapBuffer::new();
        storage.push_line("Hello 👋 World");

        // Test get_line method.
        let line = storage.get_line(row(0)).unwrap();
        assert_eq!(line.content(), "Hello 👋 World");
        assert!(line.info().grapheme_count.as_usize() > 0);
        assert!(line.info().display_width.as_usize() > 0);

        // Test GCString-compatible methods.
        let seg_string = line.info().get_string_at(line.content(), col(6)).unwrap();
        assert_eq!(seg_string.string.as_ref(), "👋");
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Basic line storage operations for [`ZeroCopyGapBuffer`].
//!
//! This module provides fundamental line storage operations for [`ZeroCopyGapBuffer`],
//! enabling efficient text editing and manipulation.
//!
//! # Performance Characteristics
//!
//! - **Zero-copy access**: Line content is returned as `&str` without copying
//! - **Efficient grapheme operations**: Leverages pre-computed segment metadata
//! - **Optimized appends**: Uses fast path for end-of-line insertions
//! - **Dynamic line growth**: Automatically extends capacity as needed

use super::super::ZeroCopyGapBuffer;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ByteIndex, ByteIndexRangeExt,
            ByteLength, CCol, CIndex, CLength, CRow, CWidth, DocSeg, GCStringOwned,
            GapBufferLine, LineMetadata, RangeExt, SegStringOwned, byte_len,
            byte_offset, c_col, c_index, c_len, c_row, c_width,
            segment_builder::build_segments_for_str};
use std::ops::Range;

impl ZeroCopyGapBuffer {
    // Line access methods.

    /// Get the number of lines in the storage (alias for `get_line_count`).
    #[must_use]
    pub fn get_c_len(&self) -> CLength { c_len(self.get_line_count().as_usize()) }

    /// Checks if the storage is empty (has no lines).
    #[must_use]
    pub fn is_empty(&self) -> bool { self.get_line_count().as_usize() == 0 }

    /// Get line content and metadata.
    ///
    /// This is the primary API for accessing lines from the buffer. It returns a
    /// [`GapBufferLine`] that provides unified access to both the line content and
    /// its metadata (segments, display width, etc.).
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the line contains invalid [`UTF-8`]. This should never
    /// happen as all content is validated on insertion.
    ///
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn get_line(&self, arg_row_index: impl Into<CRow>) -> Option<GapBufferLine<'_>> {
        let row_index: CRow = arg_row_index.into();
        let line_info = self.get_line_info(row_index)?;

        // In debug builds, validate UTF-8.
        #[cfg(debug_assertions)]
        {
            let content_range = line_info.content_range();
            if let Err(e) =
                std::str::from_utf8(&self.buffer[content_range.to_usize_range()])
            {
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
            let content_range = line_info.content_range();
            std::str::from_utf8_unchecked(&self.buffer[content_range.to_usize_range()])
        };
        Some(GapBufferLine::new(content, line_info))
    }

    // Line metadata access
    #[must_use]
    pub fn get_line_display_width(
        &self,
        arg_row_index: impl Into<CRow>,
    ) -> Option<CWidth> {
        let row_index: CRow = arg_row_index.into();
        self.get_line_info(row_index).map(|info| info.display_width)
    }

    /// Gets line display width at given row index, returning `c_width(0)` if out of
    /// bounds.
    #[must_use]
    pub fn get_line_display_width_at_row_index(
        &self,
        arg_row_index: impl Into<CRow>,
    ) -> CWidth {
        self.get_line_display_width(arg_row_index)
            .unwrap_or(c_width(0))
    }

    /// Gets the maximum row index of the buffer, returning `c_row(0)` if empty.
    #[must_use]
    pub fn get_max_row_index(&self) -> CRow {
        c_row(self.get_line_count().as_usize().saturating_sub(1))
    }

    /// Checks if the line at `arg_row_index` is empty or out of bounds.
    #[must_use]
    pub fn is_line_empty(&self, arg_row_index: impl Into<CRow>) -> bool {
        self.get_line_display_width_at_row_index(arg_row_index)
            .is_empty()
    }

    /// Checks if `arg_row_index` is within the valid line bounds of the buffer.
    #[must_use]
    pub fn is_valid_row_index(&self, arg_row_index: impl Into<CRow>) -> bool {
        let row_index: CRow = arg_row_index.into();
        self.get_line_info(row_index).is_some()
    }

    /// Gets line content at given row index, returning `""` if out of bounds.
    #[must_use]
    pub fn get_line_content_or_empty(&self, arg_row_index: impl Into<CRow>) -> &str {
        self.get_line_content(arg_row_index).unwrap_or("")
    }

    #[must_use]
    pub fn get_line_grapheme_count(
        &self,
        arg_row_index: impl Into<CRow>,
    ) -> Option<CLength> {
        let row_index: CRow = arg_row_index.into();
        self.get_line_info(row_index)
            .map(|info| info.grapheme_count)
    }

    #[must_use]
    pub fn get_line_byte_len(
        &self,
        arg_row_index: impl Into<CRow>,
    ) -> Option<ByteLength> {
        let row_index: CRow = arg_row_index.into();
        self.get_line_info(row_index)
            .map(|info| info.content_byte_len)
    }

    // Line modification methods.

    /// Insert a new empty line at the specified row index.
    ///
    /// # Arguments
    ///
    /// * `arg_row_index` - Row index converted into [`CRow`].
    ///
    /// # Returns
    ///
    /// [`Some(CRow)`] with the row index of the newly inserted line if successful,
    /// or [`None`] if the row index was out of bounds.
    ///
    /// [`Some(CRow)`]: CRow
    pub fn insert_line(&mut self, arg_row_index: impl Into<CRow>) -> Option<CRow> {
        let row_index: CRow = arg_row_index.into();
        match self.insert_empty_line(row_index) {
            Ok(()) => Some(row_index),
            Err(_) => None,
        }
    }

    /// Replaces the content of an existing line at the specified row index.
    ///
    /// # Arguments
    ///
    /// * `arg_row_index` - Row index converted into [`CRow`].
    /// * `content` - New text content to set for the line.
    ///
    /// # Returns
    ///
    /// [`Some(())`] if the row existed and content was replaced, or [`None`] if the
    /// row index was out of bounds.
    ///
    /// [`Some(())`]: Option::Some
    pub fn set_line(
        &mut self,
        arg_row_index: impl Into<CRow>,
        content: &str,
    ) -> Option<()> {
        let row_index: CRow = arg_row_index.into();
        let line_info = self.get_line_info(row_index)?;
        let grapheme_count = line_info.grapheme_count;

        if !grapheme_count.is_empty() {
            let delete_res = self.delete_range(
                row_index,
                c_index(0u16),
                c_index(grapheme_count.as_usize()),
            );
            if delete_res.is_err() {
                return None;
            }
        }

        let insert_res = self.insert_text_at_grapheme(row_index, c_index(0u16), content);
        if insert_res.is_err() {
            return None;
        }

        Some(())
    }

    pub fn push_line(&mut self, content: &str) {
        let line_idx = self.add_line();
        drop(self.insert_text_at_grapheme(c_row(line_idx), c_index(0u16), content));
    }

    // Column-based operations.

    pub fn insert_at_col(
        &mut self,
        row_index: CRow,
        col_index: CCol,
        text: &str,
    ) -> Option<CWidth> {
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
    ///
    /// * `row_index` - The row to delete from
    /// * `col_index` - The column position to start deletion
    /// * `segment_count` - The number of grapheme clusters (segments) to delete
    ///
    /// # Returns
    ///
    /// [`Some(())`] when deletion was successful, or [`None`] if the position was invalid
    /// or deletion failed.
    ///
    /// [`Some(())`]: Option::Some
    pub fn delete_at_col(
        &mut self,
        row_index: CRow,
        col_index: CCol,
        segment_count: CLength,
    ) -> Option<()> {
        let seg_idx = self.col_to_seg_index(row_index, col_index)?;
        let line_info = self.get_line_info(row_index)?;
        let max_segments = c_len(line_info.grapheme_segments.len());
        let requested_end = seg_idx.as_usize() + segment_count.as_usize();
        let max_segments_usize = max_segments.as_usize();
        let actual_end = if requested_end > max_segments_usize {
            max_segments_usize
        } else {
            requested_end
        };
        let end_seg_index = c_index(actual_end);

        let delete_res = self.delete_range(row_index, seg_idx, end_seg_index);
        if delete_res.is_err() {
            return None;
        }

        Some(())
    }

    // Utility methods

    /// Splits a line at the given column index.
    ///
    /// # Arguments
    ///
    /// * `row_index` - The row to split
    /// * `col_index` - The column position to split at
    ///
    /// # Returns
    ///
    /// [`Some(String)`] containing the right-hand portion of the split line if
    /// successful, or [`None`] if the row or column position was invalid.
    ///
    /// [`Some(String)`]: String
    pub fn split_line_at_col(
        &mut self,
        row_index: CRow,
        col_index: CCol,
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
        let _unused = self.set_line(row_index, left_part);

        Some(right_content)
    }

    /// Merges the line below `arg_base_row_index` into `arg_base_row_index` and removes
    /// the second line.
    ///
    /// # Arguments
    ///
    /// * `arg_base_row_index` - Base line index converted into [`CRow`].
    ///
    /// # Returns
    ///
    /// [`Some(LineMetadata)`] of the removed second line if successful,
    /// or [`None`] if `base_row_index` or the next row is out of bounds.
    ///
    /// [`Some(LineMetadata)`]: LineMetadata
    pub fn merge_with_next_line(
        &mut self,
        arg_base_row_index: impl Into<CRow>,
    ) -> Option<LineMetadata> {
        let base_row_index: CRow = arg_base_row_index.into();
        let next_row_index = base_row_index + 1;

        let second_line_content = self.get_line_content(next_row_index)?;
        let second_line_text = second_line_content.to_string();

        let line_info = self.get_line_info(base_row_index)?;
        let append_pos = c_index(line_info.grapheme_count.as_usize());

        let insert_result =
            self.insert_text_at_grapheme(base_row_index, append_pos, &second_line_text);
        let Ok(()) = insert_result else {
            return None;
        };

        self.remove_line(next_row_index)
    }

    // Byte position conversions.

    #[must_use]
    pub fn get_byte_pos_for_row(
        &self,
        arg_row_index: impl Into<CRow>,
    ) -> Option<ByteIndex> {
        let row_index: CRow = arg_row_index.into();
        self.get_line_info(row_index).map(|info| info.buffer_start)
    }

    #[must_use]
    pub fn find_row_containing_byte(
        &self,
        arg_byte_index: impl Into<ByteIndex>,
    ) -> Option<CRow> {
        let byte_index: ByteIndex = arg_byte_index.into();
        // Early bounds check for performance optimization.
        let buffer_len = byte_len(self.buffer.len());
        if byte_index.overflows(buffer_len) == ArrayOverflowResult::Overflowed {
            return None;
        }

        // Linear search through lines to find which one contains the byte.
        // This could be optimized with binary search if needed.
        let total_lines = self.get_line_count();
        let line_range = ..total_lines;
        for row_idx in line_range.as_index_iter() {
            if let Some(line_info) = self.get_line_info(row_idx) {
                // Create a type-safe byte range for this line.
                let line_byte_range: Range<ByteIndex> = line_info.buffer_start
                    ..(line_info.buffer_start
                        + byte_offset(line_info.capacity.as_usize()));

                if line_byte_range.contains(&byte_index) {
                    return Some(row_idx);
                }
            }
        }

        None
    }

    // Iterator support.

    /// Return an iterator over all lines in the buffer.
    pub fn iter_lines(&self) -> impl Iterator<Item = GapBufferLine<'_>> + '_ {
        // Create a type-safe range spanning all line indices in the buffer
        // (0..get_line_count).
        let line_range = ..self.get_line_count();

        // Convert the range into a row-index iterator, fetching each valid line.
        line_range
            .as_index_iter()
            .filter_map(move |row_idx| self.get_line(row_idx))
    }

    // Conversion methods.

    pub fn to_gc_string_vec(&self) -> Vec<GCStringOwned> {
        let line_range = ..self.get_line_count();
        line_range
            .as_index_iter()
            .filter_map(|row_idx| self.get_line_content(row_idx))
            .map(Into::into)
            .collect()
    }

    #[must_use]
    pub fn from_gc_string_vec(lines: Vec<GCStringOwned>) -> Self {
        let mut buffer = Self::default();
        for line in lines {
            buffer.push_line(line.as_ref());
        }
        buffer
    }

    // Validation support methods.

    #[must_use]
    pub fn get_string_at_col(
        &self,
        arg_row_index: impl Into<CRow>,
        arg_col_index: impl Into<CCol>,
    ) -> Option<SegStringOwned> {
        let row_index: CRow = arg_row_index.into();
        let col_index: CCol = arg_col_index.into();
        let line = self.get_line(row_index)?;
        line.get_string_at(col_index)
    }

    #[must_use]
    pub fn is_in_middle_of_grapheme(
        &self,
        arg_row_index: impl Into<CRow>,
        arg_col_index: impl Into<CCol>,
    ) -> Option<DocSeg> {
        let row_index: CRow = arg_row_index.into();
        let col_index: CCol = arg_col_index.into();
        let line = self.get_line(row_index)?;
        line.check_is_in_middle_of_grapheme(c_col(col_index.as_usize()))
    }
}

// Helper methods for ZeroCopyGapBuffer.
impl ZeroCopyGapBuffer {
    /// Converts a column index to a segment index for a given line.
    fn col_to_seg_index(&self, row_index: CRow, col_index: CCol) -> Option<CIndex> {
        let line_info = self.get_line_info(row_index)?;
        let target_col = col_index.as_usize();
        let mut current_col = 0;

        // Find the segment that contains or is after the target column.
        for (i, segment) in line_info.grapheme_segments.iter().enumerate() {
            if current_col >= target_col {
                return Some(c_index(i));
            }
            current_col += segment.display_width.as_usize();
        }

        // If we've gone through all segments, return the end position.
        Some(c_index(line_info.grapheme_segments.len()))
    }

    /// Calculate the display width of a text string.
    fn calculate_text_display_width(text: &str) -> CWidth {
        let segments = build_segments_for_str(text);
        let total_width: usize = segments
            .iter()
            .map(|seg| seg.display_width.as_usize())
            .sum();

        c_width(total_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byte_len, c_col, c_height, c_len};

    #[test]
    fn test_basic_line_operations() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Test empty storage.
        assert_eq!(storage.get_line_count(), c_height(0));
        assert!(storage.is_empty());

        // Add some lines
        storage.push_line("Hello, world!");
        storage.push_line("This is line 2");
        storage.push_line("And line 3");

        // Test line count and max row index
        assert_eq!(storage.get_line_count(), c_height(3));
        assert_eq!(storage.get_max_row_index(), c_row(2));
        assert!(!storage.is_empty());

        // Test line validity and content access.
        assert!(storage.is_valid_row_index(c_row(0)));
        assert!(storage.is_valid_row_index(c_row(2)));
        assert!(!storage.is_valid_row_index(c_row(3)));

        assert_eq!(storage.get_line_content(c_row(0)), Some("Hello, world!"));
        assert_eq!(storage.get_line_content_or_empty(c_row(0)), "Hello, world!");
        assert_eq!(storage.get_line_content(c_row(1)), Some("This is line 2"));
        assert_eq!(storage.get_line_content(c_row(2)), Some("And line 3"));
        assert_eq!(storage.get_line_content(c_row(3)), None);
        assert_eq!(storage.get_line_content_or_empty(c_row(3)), "");

        // Test line empty check and display width metadata.
        assert!(!storage.is_line_empty(c_row(0)));
        assert!(storage.is_line_empty(c_row(99)));

        // Test line metadata.
        assert_eq!(storage.get_line_display_width(c_row(0)), Some(c_width(13)));
        assert_eq!(
            storage.get_line_display_width_at_row_index(c_row(0)),
            c_width(13)
        );
        assert_eq!(
            storage.get_line_display_width_at_row_index(c_row(99)),
            c_width(0)
        );
        assert_eq!(storage.get_line_grapheme_count(c_row(0)), Some(c_len(13)));
        assert_eq!(storage.get_line_byte_len(c_row(0)), Some(byte_len(13)));
    }

    #[test]
    fn test_line_modification() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Add initial content.
        storage.push_line("Original line");

        // Test set_line
        assert_eq!(storage.set_line(c_row(0), "Modified line"), Some(()));
        assert_eq!(storage.get_line_content(c_row(0)), Some("Modified line"));

        // Test insert_line at the end (to avoid the underflow bug)
        assert_eq!(storage.insert_line(c_row(1)), Some(c_row(1)));
        assert_eq!(storage.get_line_count(), c_height(2));
        assert_eq!(storage.get_line_content(c_row(0)), Some("Modified line"));
        assert_eq!(storage.get_line_content(c_row(1)), Some(""));

        // Test remove_line (remove the empty line at the end)
        assert!(storage.remove_line(c_row(1)).is_some());
        assert_eq!(storage.get_line_count(), c_height(1));
        assert_eq!(storage.get_line_content(c_row(0)), Some("Modified line"));

        // Test insert_line at beginning.
        assert_eq!(storage.insert_line(c_row(0)), Some(c_row(0)));
        assert_eq!(storage.get_line_count(), c_height(2));
        assert_eq!(storage.get_line_content(c_row(0)), Some(""));
        assert_eq!(storage.get_line_content(c_row(1)), Some("Modified line"));

        // Test remove_line at beginning.
        assert!(storage.remove_line(c_row(0)).is_some());
        assert_eq!(storage.get_line_count(), c_height(1));
        assert_eq!(storage.get_line_content(c_row(0)), Some("Modified line"));
    }

    #[test]
    fn test_grapheme_operations() {
        let mut storage = ZeroCopyGapBuffer::default();
        storage.push_line("Hello");

        // Test insert_at_grapheme.
        assert!(
            storage
                .insert_text_at_grapheme(c_row(0), c_index(5u16), " World")
                .is_ok()
        );
        assert_eq!(storage.get_line_content(c_row(0)), Some("Hello World"));

        // Test delete_at_grapheme.
        assert!(storage.delete_grapheme_at(c_row(0), c_index(5u16)).is_ok());
        assert_eq!(storage.get_line_content(c_row(0)), Some("HelloWorld"));
    }

    #[test]
    fn test_unicode_content() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Test with emoji and unicode.
        storage.push_line("Hello 👋 世界");

        assert_eq!(storage.get_line_content(c_row(0)), Some("Hello 👋 世界"));
        assert_eq!(storage.get_line_grapheme_count(c_row(0)), Some(c_len(10))); // "Hello " = 6 + emoji = 1 + space = 1 + "世界" = 2

        // Insert more unicode.
        assert!(
            storage
                .insert_text_at_grapheme(c_row(0), c_index(7u16), " 🌍")
                .is_ok()
        );
        assert_eq!(storage.get_line_content(c_row(0)), Some("Hello 👋 🌍 世界"));
    }

    #[test]
    fn test_split_and_join_lines() {
        let mut storage = ZeroCopyGapBuffer::default();
        storage.push_line("Hello World");

        // Test split_line_at_col.
        let split_content = storage.split_line_at_col(c_row(0), c_col(6));
        assert_eq!(split_content, Some("World".to_string()));
        assert_eq!(storage.get_line_content(c_row(0)), Some("Hello "));

        // Add the split content as a new line.
        storage.push_line(&split_content.expect("conversion error"));

        // Test merge_with_next_line.
        assert!(storage.merge_with_next_line(c_row(0)).is_some());
        assert_eq!(storage.get_line_content(c_row(0)), Some("Hello World"));
        assert_eq!(storage.get_line_count(), c_height(1));
    }

    #[test]
    fn test_clear() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Add some content.
        storage.push_line("Line 1");
        storage.push_line("Line 2");
        storage.push_line("Line 3");

        assert_eq!(storage.get_line_count(), c_height(3));

        // Clear all lines
        storage.clear();

        assert_eq!(storage.get_line_count(), c_height(0));
        assert!(storage.is_empty());
    }

    #[test]
    fn test_iterator() {
        let mut storage = ZeroCopyGapBuffer::default();

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
        let mut storage = ZeroCopyGapBuffer::default();

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
        assert_eq!(new_storage.get_line_count(), c_height(2));
        assert_eq!(new_storage.get_line_content(c_row(0)), Some("Line 1"));
        assert_eq!(new_storage.get_line_content(c_row(1)), Some("Line 2"));
    }

    #[test]
    fn test_delete_at_col_with_emoji() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Create line with emoji: "Hello😃World".
        storage.push_line("Hello😃World");

        // Verify initial state.
        assert_eq!(storage.get_line_content(c_row(0)), Some("Hello😃World"));
        assert_eq!(storage.get_line_display_width(c_row(0)), Some(c_width(12))); // 5 + 2 + 5

        // Delete the emoji (1 segment) at column 5
        assert_eq!(
            storage.delete_at_col(c_row(0), c_col(5), c_len(1)),
            Some(())
        );

        // Verify the emoji was deleted.
        assert_eq!(storage.get_line_content(c_row(0)), Some("HelloWorld"));
        assert_eq!(storage.get_line_display_width(c_row(0)), Some(c_width(10)));
    }

    #[test]
    fn test_delete_at_col_multiple_segments() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Create line with multiple emojis.
        storage.push_line("👋😀🎉");

        // Each emoji is 1 segment but width 2.
        assert_eq!(storage.get_line_grapheme_count(c_row(0)), Some(c_len(3)));
        assert_eq!(storage.get_line_display_width(c_row(0)), Some(c_width(6)));

        // Delete 2 segments starting at column 0.
        assert_eq!(
            storage.delete_at_col(c_row(0), c_col(0), c_len(2)),
            Some(())
        );

        // Should have deleted 👋 and 😀, leaving only 🎉.
        assert_eq!(storage.get_line_content(c_row(0)), Some("🎉"));
        assert_eq!(storage.get_line_grapheme_count(c_row(0)), Some(c_len(1)));
        assert_eq!(storage.get_line_display_width(c_row(0)), Some(c_width(2)));
    }

    #[test]
    fn test_delete_at_col_mixed_width() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Mix of ASCII and wide characters.
        storage.push_line("a😃b世界c");

        // ColWidth: a=1, 😃=2, b=1, 世=2, 界=2, c=1
        assert_eq!(storage.get_line_display_width(c_row(0)), Some(c_width(9)));

        // Delete emoji at column 1 (segment index 1)
        assert_eq!(
            storage.delete_at_col(c_row(0), c_col(1), c_len(1)),
            Some(())
        );
        assert_eq!(storage.get_line_content(c_row(0)), Some("ab世界c"));

        // Delete '世' at column 2 (after 'ab')
        assert_eq!(
            storage.delete_at_col(c_row(0), c_col(2), c_len(1)),
            Some(())
        );
        assert_eq!(storage.get_line_content(c_row(0)), Some("ab界c"));
    }

    #[test]
    fn test_delete_at_col_segment_count_parameter() {
        let mut storage = ZeroCopyGapBuffer::default();

        // Create line with text.
        storage.push_line("abcdef");

        // Delete 3 segments starting at column 1 (should delete 'bcd')
        assert_eq!(
            storage.delete_at_col(c_row(0), c_col(1), c_len(3)),
            Some(())
        );
        assert_eq!(storage.get_line_content(c_row(0)), Some("aef"));

        // Now we have "aef" (3 segments)
        // Try to delete from beginning - even with count > remaining segments.
        assert_eq!(
            storage.delete_at_col(c_row(0), c_col(0), c_len(10)),
            Some(())
        );
        assert_eq!(storage.get_line_content(c_row(0)), Some(""));
    }

    #[test]
    fn test_get_line_with_info() {
        let mut storage = ZeroCopyGapBuffer::default();
        storage.push_line("Hello 👋 World");

        // Test get_line method.
        let line = storage.get_line(c_row(0)).expect("conversion error");
        assert_eq!(line.content(), "Hello 👋 World");
        assert!(line.info().grapheme_count.as_usize() > 0);
        assert!(line.info().display_width.as_usize() > 0);

        // Test GCStringOwned-compatible methods.
        let seg_string = line
            .info()
            .get_string_at(line.content(), c_col(6))
            .expect("conversion error");
        assert_eq!(seg_string.string.as_ref(), "👋");
    }
}

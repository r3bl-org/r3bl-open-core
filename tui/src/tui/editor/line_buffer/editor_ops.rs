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

//! Editor operations for [`LineBuffer`].
//!
//! This module provides grapheme-safe operations specifically designed for:
//! - The editor component to perform CRUD operations on text
//! - The markdown parser to access content as &str with zero-copy
//!
//! All operations respect Unicode grapheme cluster boundaries and maintain
//! UTF-8 validity. Operations include insertion, deletion, and line manipulation.

use miette::{Result, miette};

use super::{LINE_PAGE_SIZE, LineBuffer};
use crate::{ByteIndex, RowIndex, SegIndex, byte_index, len,
            segment_builder::{build_segments_for_str, calculate_display_width}};

impl LineBuffer {
    /// Insert text at a specific grapheme position within a line
    ///
    /// This method inserts the given text at the specified grapheme cluster position,
    /// ensuring that we never split a grapheme cluster. The operation is Unicode-safe
    /// and will rebuild the line's segment information after insertion.
    ///
    /// # Arguments
    /// * `line_index` - The line to insert into
    /// * `seg_index` - The grapheme cluster position to insert at
    /// * `text` - The text to insert
    ///
    /// # Returns
    /// `Ok(())` if successful, `Err` with a diagnostic error if the operation fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - Text insertion fails due to capacity or encoding issues
    /// - Segment rebuilding fails
    pub fn insert_at_grapheme(
        &mut self,
        line_index: RowIndex,
        seg_index: SegIndex,
        text: &str,
    ) -> Result<()> {
        // Validate line index
        let line_info = self.get_line_info(line_index.as_usize()).ok_or_else(|| {
            miette!("Line index {} out of bounds", line_index.as_usize())
        })?;

        // Find the byte position for the grapheme index
        let byte_pos = if seg_index.as_usize() == 0 {
            // Insert at beginning
            byte_index(0)
        } else if seg_index.as_usize() >= line_info.segments.len() {
            // Insert at end
            byte_index(line_info.content_len.as_usize())
        } else {
            // Insert in middle - find the start of the target segment
            let segment = &line_info.segments[seg_index.as_usize()];
            byte_index(segment.start_byte_index.as_usize())
        };

        // Perform the actual insertion
        self.insert_text_at_byte_pos(line_index, byte_pos, text)?;

        // Rebuild segments for this line
        self.rebuild_line_segments(line_index)?;

        Ok(())
    }

    /// Insert text at a specific byte position within a line
    ///
    /// This is a lower-level helper that performs the actual buffer manipulation.
    /// It handles capacity checking, content shifting, and buffer extension if needed.
    ///
    /// # Safety
    /// The caller must ensure that `byte_pos` is at a valid UTF-8 boundary.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - The byte position exceeds the content length
    fn insert_text_at_byte_pos(
        &mut self,
        line_index: RowIndex,
        byte_pos: ByteIndex,
        text: &str,
    ) -> Result<()> {
        let text_bytes = text.as_bytes();
        let text_len = text_bytes.len();

        // Get line info and validate position
        let line_idx = line_index.as_usize();
        let line_info = self
            .get_line_info(line_idx)
            .ok_or_else(|| miette!("Line index {} out of bounds", line_idx))?;

        let byte_position = byte_pos.as_usize();
        let current_content_len = line_info.content_len.as_usize();

        // Validate byte position
        if byte_position > current_content_len {
            return Err(miette!(
                "Byte position {} exceeds content length {}",
                byte_position,
                current_content_len
            ));
        }

        // Check if we need to extend the line capacity
        let new_content_len = current_content_len + text_len;
        let required_capacity = new_content_len + 1; // +1 for newline

        if required_capacity > line_info.capacity.as_usize() {
            // Extend the line capacity
            super::LineBuffer::extend_line_capacity(self, line_index);

            // Re-check after extension (might need multiple extensions for large text)
            let line_info = self.get_line_info(line_idx).ok_or_else(|| {
                miette!("Line {} disappeared after extension", line_idx)
            })?;
            if required_capacity > line_info.capacity.as_usize() {
                // Calculate how many pages we need
                let pages_needed = (required_capacity - line_info.capacity.as_usize())
                    .div_ceil(LINE_PAGE_SIZE);
                for _ in 0..pages_needed {
                    super::LineBuffer::extend_line_capacity(self, line_index);
                }
            }
        }

        // Get updated line info after potential capacity extension
        let line_info = self.get_line_info(line_idx).ok_or_else(|| {
            miette!("Line {} not found after capacity extension", line_idx)
        })?;
        let buffer_start = line_info.buffer_offset.as_usize();
        let insert_pos = buffer_start + byte_position;

        // Shift existing content to make room
        if byte_position < current_content_len {
            // Need to move content to the right
            let move_from = insert_pos;
            let move_to = insert_pos + text_len;
            let move_len = current_content_len - byte_position;

            // Move content (including the newline)
            for i in (0..=move_len).rev() {
                self.buffer[move_to + i] = self.buffer[move_from + i];
            }
        } else {
            // Inserting at end, just move the newline
            self.buffer[insert_pos + text_len] = b'\n';
        }

        // Copy the new text into the buffer
        self.buffer[insert_pos..insert_pos + text_len].copy_from_slice(text_bytes);

        // Update line metadata
        let line_info_mut = self.get_line_info_mut(line_idx).ok_or_else(|| {
            miette!("Line {} not found when updating metadata", line_idx)
        })?;
        line_info_mut.content_len = len(new_content_len);

        Ok(())
    }

    /// Rebuild the grapheme segments for a line after modification
    ///
    /// This method recalculates all segment information including grapheme boundaries,
    /// display widths, and grapheme count. It should be called after any text
    /// modification.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - The line content contains invalid UTF-8
    fn rebuild_line_segments(&mut self, line_index: RowIndex) -> Result<()> {
        let line_idx = line_index.as_usize();

        // Get the line content as a string
        let line_info = self
            .get_line_info(line_idx)
            .ok_or_else(|| miette!("Line index {} out of bounds", line_idx))?;

        let buffer_start = line_info.buffer_offset.as_usize();
        let content_len = line_info.content_len.as_usize();
        let content_slice = &self.buffer[buffer_start..buffer_start + content_len];

        // Convert to string for segment building
        let content_str = std::str::from_utf8(content_slice)
            .map_err(|e| miette!("Invalid UTF-8 in line {}: {}", line_idx, e))?;

        // Build new segments
        let segments = build_segments_for_str(content_str);

        // Calculate display width and grapheme count
        let display_width = calculate_display_width(&segments);

        let grapheme_count = segments.len();

        // Update line info
        let line_info = self.get_line_info_mut(line_idx).ok_or_else(|| {
            miette!("Line {} not found when updating segments", line_idx)
        })?;
        line_info.segments = segments;
        line_info.display_width = display_width;
        line_info.grapheme_count = grapheme_count;

        Ok(())
    }

    /// Insert a new empty line at the specified position
    ///
    /// This shifts all subsequent lines down and inserts a new empty line.
    ///
    /// # Errors
    /// Returns an error if the line index exceeds the current line count
    pub fn insert_empty_line(&mut self, line_index: RowIndex) -> Result<()> {
        let line_idx = line_index.as_usize();

        if line_idx > self.line_count() {
            return Err(miette!(
                "Cannot insert line at index {}, only {} lines exist",
                line_idx,
                self.line_count()
            ));
        }

        // Add a new line at the end first
        let new_line_idx = self.add_line();

        // If we're inserting before the end, we need to shift lines
        if line_idx < new_line_idx {
            // Move lines down to make space
            for i in (line_idx..new_line_idx).rev() {
                // Swap line metadata
                self.swap_lines(i, i + 1);
            }

            // The actual buffer content doesn't need to be moved since
            // each line tracks its own offset
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{row, seg_index};

    #[test]
    fn test_insert_at_beginning() -> Result<()> {
        let mut buffer = LineBuffer::new();
        buffer.add_line();

        // Insert text at the beginning
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello")?;

        // Verify the content
        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello");

        // Verify segments were rebuilt
        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, 5);
        assert_eq!(line_info.content_len, len(5));

        Ok(())
    }

    #[test]
    fn test_insert_at_end() -> Result<()> {
        let mut buffer = LineBuffer::new();
        buffer.add_line();

        // First insert some text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello")?;

        // Then insert at the end
        buffer.insert_at_grapheme(row(0), seg_index(5), " World")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello World");

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, 11);

        Ok(())
    }

    #[test]
    fn test_insert_in_middle() -> Result<()> {
        let mut buffer = LineBuffer::new();
        buffer.add_line();

        // Insert initial text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Heo")?;

        // Insert in the middle
        buffer.insert_at_grapheme(row(0), seg_index(2), "ll")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello");

        Ok(())
    }

    #[test]
    fn test_insert_unicode() -> Result<()> {
        let mut buffer = LineBuffer::new();
        buffer.add_line();

        // Insert emoji
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello 😀")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello 😀");

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, 7); // "Hello " = 6 + emoji = 1

        // Insert more text after emoji
        buffer.insert_at_grapheme(row(0), seg_index(7), " World")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content after second insert"))?;
        assert_eq!(content, "Hello 😀 World");

        Ok(())
    }

    #[test]
    fn test_insert_causes_line_extension() -> Result<()> {
        let mut buffer = LineBuffer::new();
        buffer.add_line();

        // Create a string that will require line extension
        let long_text = "A".repeat(300);

        buffer.insert_at_grapheme(row(0), seg_index(0), &long_text)?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, &long_text);

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, 300);
        assert!(line_info.capacity.as_usize() >= 301); // 300 + newline

        Ok(())
    }

    #[test]
    fn test_insert_empty_line() -> Result<()> {
        let mut buffer = LineBuffer::new();
        buffer.add_line();
        buffer.add_line();

        // Add content to lines
        buffer.insert_at_grapheme(row(0), seg_index(0), "Line 1")?;
        buffer.insert_at_grapheme(row(1), seg_index(0), "Line 2")?;

        // Insert empty line in middle
        buffer.insert_empty_line(row(1))?;

        assert_eq!(buffer.line_count(), 3);

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line 0 content"))?;
        assert_eq!(content, "Line 1");

        let content = buffer
            .get_line_content(row(1))
            .ok_or_else(|| miette!("Failed to get line 1 content"))?;
        assert_eq!(content, "");

        let content = buffer
            .get_line_content(row(2))
            .ok_or_else(|| miette!("Failed to get line 2 content"))?;
        assert_eq!(content, "Line 2");

        Ok(())
    }

    #[test]
    fn test_insert_invalid_line_index() -> Result<()> {
        let mut buffer = LineBuffer::new();

        let result = buffer.insert_at_grapheme(row(0), seg_index(0), "Hello");
        assert!(result.is_err());

        let err_msg = result
            .err()
            .ok_or_else(|| miette!("Expected error but got none"))?
            .to_string();
        assert!(err_msg.contains("out of bounds"));

        Ok(())
    }

    #[test]
    fn test_insert_compound_grapheme_clusters() -> Result<()> {
        let mut buffer = LineBuffer::new();
        buffer.add_line();

        // Insert text with compound grapheme clusters
        buffer.insert_at_grapheme(row(0), seg_index(0), "👨‍👩‍👧‍👦 Family")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "👨‍👩‍👧‍👦 Family");

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        // The family emoji is 1 grapheme cluster despite being multiple code points
        assert_eq!(line_info.grapheme_count, 8); // 1 + space + 6 letters

        Ok(())
    }
}

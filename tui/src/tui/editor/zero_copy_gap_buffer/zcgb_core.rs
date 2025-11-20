// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-copy gap buffer data structure for storing editor content.
//!
//! This module contains the main [`ZeroCopyGapBuffer`] implementation with its
//! core buffer management operations including line creation, deletion, and capacity
//! management.

use super::{GapBufferLine, INITIAL_LINE_SIZE, LINE_PAGE_SIZE, LineMetadata};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ColIndex, CursorBoundsCheck,
            GraphemeDoc, GraphemeDocMut, LINE_FEED_BYTE, Length, NULL_BYTE,
            NumericValue, RowIndex, SegIndex, SegmentArray, byte_index, byte_offset,
            len, row};
use std::{borrow::Cow, fmt::Display};

/// Zero-copy gap buffer data structure for storing editor content
#[derive(Debug, Clone, PartialEq)]
pub struct ZeroCopyGapBuffer {
    /// Contiguous buffer storing all lines
    /// Each line starts at [`INITIAL_LINE_SIZE`] bytes and can grow
    pub buffer: Vec<u8>,

    /// Metadata for each line (grapheme clusters, display width, etc.)
    lines: Vec<LineMetadata>,

    /// Number of lines currently in the buffer
    line_count: Length,
}

impl ZeroCopyGapBuffer {
    /// Create a new empty [`ZeroCopyGapBuffer`]
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            lines: Vec::new(),
            line_count: len(0),
        }
    }

    /// Create a new [`ZeroCopyGapBuffer`] with pre-allocated capacity
    #[must_use]
    pub fn with_capacity(line_capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(line_capacity * INITIAL_LINE_SIZE),
            lines: Vec::with_capacity(line_capacity),
            line_count: len(0),
        }
    }

    /// Get the number of lines in the buffer
    #[must_use]
    pub fn line_count(&self) -> Length { self.line_count }

    /// Get line metadata by index
    #[must_use]
    pub fn get_line_info(
        &self,
        arg_line_index: impl Into<RowIndex>,
    ) -> Option<&LineMetadata> {
        let line_index: RowIndex = arg_line_index.into();
        self.lines.get(line_index.as_usize())
    }

    /// Get mutable line metadata by index
    pub fn get_line_info_mut(
        &mut self,
        arg_line_index: impl Into<RowIndex>,
    ) -> Option<&mut LineMetadata> {
        let line_index: RowIndex = arg_line_index.into();
        self.lines.get_mut(line_index.as_usize())
    }

    /// Swap two lines in the buffer metadata
    /// This only swaps the [`LineMetadata`] entries, not the actual buffer content
    pub fn swap_lines(&mut self, arg_i: impl Into<RowIndex>, arg_j: impl Into<RowIndex>) {
        let i: RowIndex = arg_i.into();
        let j: RowIndex = arg_j.into();
        self.lines.swap(i.as_usize(), j.as_usize());
    }

    /// Insert a new empty line at the specified position with proper buffer shifting
    ///
    /// This method properly maintains the invariant that lines are ordered by their
    /// buffer offsets by actually shifting buffer content.
    ///
    /// # Buffer Shifting Behavior
    ///
    /// - **Insertion at end**: No shifting needed, just appends a new line to the buffer
    /// - **Insertion at start/middle**: Shifts all subsequent buffer content down by
    ///   `INITIAL_LINE_SIZE` bytes to make room for the new line
    ///
    /// # Example
    ///
    /// ```text
    /// Before insertion at position 1:
    /// [Line 0: 256 bytes][Line 1: 256 bytes][Line 2: 256 bytes]
    ///
    /// After insertion at position 1:
    /// [Line 0: 256 bytes][New Line: 256 bytes][Line 1: 256 bytes][Line 2: 256 bytes]
    ///                     ↑ All content shifted →
    /// ```
    pub fn insert_line_with_buffer_shift(&mut self, arg_line_index: impl Into<RowIndex>) {
        let line_index: RowIndex = arg_line_index.into();
        let line_idx = line_index.as_usize();

        // If inserting at the end, just add a new line.
        // Use cursor position bounds checking which allows insertion at line_count (end).
        if self
            .line_count
            .check_cursor_position_bounds(line_index.into())
            == crate::CursorPositionBoundsStatus::AtEnd
        {
            self.add_line();
            return;
        }

        // Calculate where the new line should be inserted in the buffer.
        let insert_offset = if line_index.is_zero() {
            byte_index(0)
        } else {
            let prev_line = &self.lines[line_idx - 1];
            prev_line.buffer_start + byte_offset(prev_line.capacity.as_usize())
        };

        // Extend buffer by INITIAL_LINE_SIZE bytes.
        let old_buffer_len = self.buffer.len();
        self.buffer
            .resize(old_buffer_len + INITIAL_LINE_SIZE, NULL_BYTE);

        // Shift all subsequent buffer content down.
        let shift_start = *insert_offset;
        let shift_amount = INITIAL_LINE_SIZE;

        // Move content from back to front to avoid overwriting.
        for i in (shift_start..old_buffer_len).rev() {
            self.buffer[i + shift_amount] = self.buffer[i];
        }

        // Clear the newly created space.
        for i in shift_start..shift_start + INITIAL_LINE_SIZE {
            self.buffer[i] = NULL_BYTE;
        }

        // Add newline character for the empty line.
        self.buffer[shift_start] = LINE_FEED_BYTE;

        // Create line metadata for the new line.
        let new_line_info = LineMetadata {
            buffer_start: insert_offset,
            content_byte_len: len(0),
            capacity: len(INITIAL_LINE_SIZE),
            grapheme_segments: SegmentArray::new(),
            display_width: crate::width(0),
            grapheme_count: len(0),
        };

        // Insert the new line metadata at the correct position.
        self.lines.insert(line_idx, new_line_info);

        // Update buffer offsets for all subsequent lines.
        for i in (line_idx + 1)..self.lines.len() {
            self.lines[i].buffer_start += byte_offset(shift_amount);
        }

        self.line_count += len(1);
    }

    /// Add a new line to the buffer (always appends at the end)
    ///
    /// Returns the index of the newly added line.
    ///
    /// # Buffer Behavior
    ///
    /// This method always appends at the end of the buffer, so no shifting is required.
    /// The new line is allocated `INITIAL_LINE_SIZE` bytes, all initialized to `\0`
    /// except for the first byte which contains the newline character `\n`.
    ///
    /// # Example
    ///
    /// ```text
    /// Before:
    /// [Line 0: 256 bytes][Line 1: 256 bytes]
    ///
    /// After add_line():
    /// [Line 0: 256 bytes][Line 1: 256 bytes][New Line 2: 256 bytes]
    /// ```
    pub fn add_line(&mut self) -> usize {
        let line_index = self.line_count.as_usize();

        // Calculate where this line starts in the buffer.
        let buffer_pos = if self.line_count.is_zero() {
            byte_index(0)
        } else {
            let prev_line = &self.lines[line_index - 1];
            prev_line.buffer_start + byte_offset(prev_line.capacity.as_usize())
        };

        // Extend buffer by INITIAL_LINE_SIZE bytes, all initialized to '\0'
        self.buffer
            .resize(self.buffer.len() + INITIAL_LINE_SIZE, NULL_BYTE);

        // Add the newline character at the start (empty line)
        self.buffer[*buffer_pos] = LINE_FEED_BYTE;

        // Create line metadata.
        self.lines.push(LineMetadata {
            buffer_start: buffer_pos,
            content_byte_len: len(0),
            capacity: len(INITIAL_LINE_SIZE),
            grapheme_segments: SegmentArray::new(),
            display_width: crate::width(0),
            grapheme_count: len(0),
        });

        self.line_count += len(1);
        line_index
    }

    /// Remove a line from the buffer
    ///
    /// Returns true if the line was removed, false if index was out of bounds.
    ///
    /// # Buffer Shifting Behavior
    ///
    /// - **Deletion at end**: No shifting needed, just truncates the buffer
    /// - **Deletion at start/middle**: Shifts all subsequent buffer content up by the
    ///   removed line's capacity to fill the gap
    ///
    /// # Example
    ///
    /// ```text
    /// Before deletion at position 1:
    /// [Line 0: 256 bytes][Line 1: 256 bytes][Line 2: 256 bytes][Line 3: 256 bytes]
    ///
    /// After deletion at position 1:
    /// [Line 0: 256 bytes][Line 2: 256 bytes][Line 3: 256 bytes]
    ///                     ← All content shifted
    /// ```
    ///
    /// All buffer offsets for subsequent lines are updated to maintain the invariant
    /// that lines are ordered by their buffer offsets.
    pub fn remove_line(&mut self, arg_line_index: impl Into<RowIndex>) -> bool {
        let line_index: RowIndex = arg_line_index.into();
        if line_index.overflows(self.line_count) == ArrayOverflowResult::Overflowed {
            return false;
        }

        let removed_line = &self.lines[line_index.as_usize()];
        let removed_start = *removed_line.buffer_start;
        let removed_size = removed_line.capacity.as_usize();

        // Remove from metadata.
        self.lines.remove(line_index.as_usize());

        // Shift buffer contents.
        let shift_start = removed_start + removed_size;
        let buffer_len = self.buffer.len();

        // Move all subsequent bytes up.
        for i in shift_start..buffer_len {
            self.buffer[i - removed_size] = self.buffer[i];
        }

        // Truncate the buffer.
        self.buffer.truncate(buffer_len - removed_size);

        // Update buffer offsets for remaining lines.
        for line in self.lines.iter_mut().skip(line_index.as_usize()) {
            line.buffer_start = line.buffer_start - byte_offset(removed_size);
        }

        self.line_count -= len(1);
        true
    }

    /// Clear all lines from the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.lines.clear();
        self.line_count = len(0);
    }

    /// Check if a line can accommodate additional bytes without reallocation
    #[must_use]
    pub fn can_insert(
        &self,
        arg_line_index: impl Into<RowIndex>,
        additional_bytes: usize,
    ) -> bool {
        let line_index: RowIndex = arg_line_index.into();
        if let Some(line_info) = self.get_line_info(line_index) {
            // Need space for content + newline.
            line_info.content_byte_len.as_usize() + additional_bytes
                < line_info.capacity.as_usize()
        } else {
            false
        }
    }

    /// Extend the capacity of a line by `LINE_PAGE_SIZE`
    pub fn extend_line_capacity(&mut self, arg_line_index: impl Into<RowIndex>) {
        let line_index: RowIndex = arg_line_index.into();
        if line_index.overflows(self.line_count) == ArrayOverflowResult::Overflowed {
            return;
        }

        let line_info = &self.lines[line_index.as_usize()];
        let line_start = *line_info.buffer_start;
        let old_capacity = line_info.capacity.as_usize();
        let new_capacity = old_capacity + LINE_PAGE_SIZE;

        // Calculate how much to shift subsequent content.
        let shift_amount = LINE_PAGE_SIZE;
        let insert_pos = line_start + old_capacity;

        // Extend buffer to accommodate new capacity.
        self.buffer
            .resize(self.buffer.len() + shift_amount, NULL_BYTE);

        // Shift all subsequent content to the right.
        for i in (insert_pos..self.buffer.len() - shift_amount).rev() {
            self.buffer[i + shift_amount] = self.buffer[i];
        }

        // Fill the newly allocated space with nulls.
        for i in insert_pos..insert_pos + shift_amount {
            self.buffer[i] = NULL_BYTE;
        }

        // Update line capacity.
        self.lines[line_index.as_usize()].capacity = len(new_capacity);

        // Update buffer offsets for subsequent lines.
        for line in self.lines.iter_mut().skip(line_index.as_usize() + 1) {
            line.buffer_start += byte_offset(shift_amount);
        }
    }
}

impl Default for ZeroCopyGapBuffer {
    fn default() -> Self { Self::new() }
}

impl Display for ZeroCopyGapBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ZeroCopyGapBuffer {{ lines: {}, buffer_size: {} bytes }}",
            self.line_count.as_usize(),
            self.buffer.len()
        )
    }
}

impl crate::GetMemSize for ZeroCopyGapBuffer {
    fn get_mem_size(&self) -> usize {
        let buffer_size = self.buffer.len() * std::mem::size_of::<u8>();
        let lines_size = self.lines.len() * std::mem::size_of::<LineMetadata>();
        let line_metadata_size: usize = self
            .lines
            .iter()
            .map(|line| line.grapheme_segments.len() * std::mem::size_of::<crate::Seg>())
            .sum();

        buffer_size + lines_size + line_metadata_size + std::mem::size_of::<Length>()
    }
}

impl GraphemeDoc for ZeroCopyGapBuffer {
    /// Line type with lifetime tied to the buffer.
    ///
    /// The lifetime `'a` represents references to data within the `ZeroCopyGapBuffer`.
    /// The constraint `Self: 'a` ensures that the returned `GapBufferLine<'a>` cannot
    /// outlive the buffer it borrows from.
    type Line<'a> = GapBufferLine<'a>;

    /// Iterator type with lifetime tied to the buffer.
    ///
    /// The lifetime `'a` represents references to data within the `ZeroCopyGapBuffer`.
    /// The constraint `Self: 'a` ensures that the iterator cannot outlive the buffer
    /// it borrows from.
    type LineIterator<'a> = ZeroCopyLineIterator<'a>;

    fn line_count(&self) -> Length { self.line_count() }

    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>> { self.get_line(row) }

    fn total_bytes(&self) -> usize { self.buffer.len() }

    fn iter_lines(&self) -> Self::LineIterator<'_> {
        ZeroCopyLineIterator {
            buffer: self,
            current: 0,
        }
    }

    fn as_str(&self) -> Cow<'_, str> { Cow::Borrowed(self.as_str()) }

    fn as_bytes(&self) -> Cow<'_, [u8]> { Cow::Borrowed(self.as_bytes()) }
}

/// Iterator over lines in a `ZeroCopyGapBuffer`
#[derive(Debug)]
pub struct ZeroCopyLineIterator<'a> {
    buffer: &'a ZeroCopyGapBuffer,
    current: usize,
}

impl<'a> Iterator for ZeroCopyLineIterator<'a> {
    type Item = GapBufferLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_index = row(self.current);
        if current_index.overflows(self.buffer.line_count())
            == ArrayOverflowResult::Overflowed
        {
            None
        } else {
            let line = self.buffer.get_line(current_index);
            self.current += 1;
            line
        }
    }
}

impl GraphemeDocMut for ZeroCopyGapBuffer {
    type DocMutResult = ();

    fn add_line(&mut self) -> usize { self.add_line() }

    fn remove_line(&mut self, row: RowIndex) -> bool { self.remove_line(row) }

    fn insert_line_with_buffer_shift(&mut self, line_idx: usize) {
        self.insert_line_with_buffer_shift(line_idx);
    }

    fn clear(&mut self) { self.clear(); }

    fn insert_text_at_grapheme(
        &mut self,
        row: RowIndex,
        seg_index: SegIndex,
        text: &str,
    ) -> miette::Result<Self::DocMutResult> {
        self.insert_text_at_grapheme(row, seg_index, text)
            .map_err(|e| miette::miette!("{}", e))
    }

    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex,
    ) -> miette::Result<Self::DocMutResult> {
        self.delete_range(row, start_seg, end_seg)
            .map_err(|e| miette::miette!("{}", e))
    }

    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult> {
        self.insert_empty_line(row)
            .map_err(|e| miette::miette!("{}", e))
    }

    fn merge_lines(&mut self, row_idx: RowIndex) -> miette::Result<Self::DocMutResult> {
        // Since ZeroCopyGapBuffer doesn't have a built-in merge_lines method,
        // we'll implement it by copying content from the next line to current line
        // and then removing the next line.
        let next_row = row(row_idx.as_usize() + 1);
        if next_row.overflows(self.line_count()) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!(
                "Cannot merge: no line after row {}",
                row_idx.as_usize()
            ));
        }

        // Get content from the next line.
        let next_line_content = self
            .get_line(next_row)
            .map(|line| line.content().to_string())
            .ok_or_else(|| miette::miette!("Failed to get next line content"))?;

        // Append to current line.
        let current_line = self
            .get_line(row_idx)
            .ok_or_else(|| miette::miette!("Failed to get current line"))?;
        let end_seg_count = current_line.segment_count();

        // Insert the next line's content at the end of current line.
        self.insert_text_at_grapheme(row_idx, end_seg_count, &next_line_content)
            .map_err(|e| miette::miette!("{}", e))?;

        // Remove the next line.
        self.remove_line(next_row);

        Ok(())
    }

    fn split_line(
        &mut self,
        row_idx: RowIndex,
        col_idx: ColIndex,
    ) -> miette::Result<Self::DocMutResult> {
        // Use the existing split_line_at_col method.
        let right_content =
            self.split_line_at_col(row_idx, col_idx).ok_or_else(|| {
                miette::miette!("Failed to split line at column {}", col_idx.as_usize())
            })?;

        // Insert a new line after the current one.
        self.insert_empty_line(row(row_idx.as_usize() + 1))
            .map_err(|e| miette::miette!("{}", e))?;

        // Insert the right content into the new line.
        self.insert_text_at_grapheme(
            row(row_idx.as_usize() + 1),
            SegIndex::from(0),
            &right_content,
        )
        .map_err(|e| miette::miette!("{}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LengthOps;

    #[test]
    fn test_new_line_buffer() {
        let buffer = ZeroCopyGapBuffer::new();
        assert_eq!(buffer.line_count(), len(0));
        assert!(buffer.buffer.is_empty());
        assert!(buffer.lines.is_empty());
    }

    #[test]
    fn test_add_line_with_dynamic_sizing() {
        let mut buffer = ZeroCopyGapBuffer::new();

        // Add first line
        let idx1 = buffer.add_line();
        assert_eq!(idx1, 0);
        assert_eq!(buffer.line_count(), len(1));
        assert_eq!(buffer.buffer.len(), INITIAL_LINE_SIZE);

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(*line_info.buffer_start, 0);
        assert_eq!(line_info.capacity, len(INITIAL_LINE_SIZE));
        assert_eq!(line_info.content_byte_len, len(0));

        // Add second line
        let idx2 = buffer.add_line();
        assert_eq!(idx2, 1);
        assert_eq!(buffer.line_count(), len(2));
        assert_eq!(buffer.buffer.len(), 2 * INITIAL_LINE_SIZE);

        let line_info = buffer.get_line_info(1).unwrap();
        assert_eq!(*line_info.buffer_start, INITIAL_LINE_SIZE);
        assert_eq!(line_info.capacity, len(INITIAL_LINE_SIZE));
    }

    #[test]
    fn test_extend_line_capacity() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        let original_capacity = buffer.get_line_info(0).unwrap().capacity;
        assert_eq!(original_capacity, len(INITIAL_LINE_SIZE));

        // Extend the line
        buffer.extend_line_capacity(row(0));

        let new_capacity = buffer.get_line_info(0).unwrap().capacity;
        assert_eq!(new_capacity, len(INITIAL_LINE_SIZE + LINE_PAGE_SIZE));
        assert_eq!(buffer.buffer.len(), INITIAL_LINE_SIZE + LINE_PAGE_SIZE);
    }

    #[test]
    fn test_can_insert() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Should be able to insert up to capacity - 1 (for newline)
        assert!(buffer.can_insert(row(0), INITIAL_LINE_SIZE - 1));
        assert!(!buffer.can_insert(row(0), INITIAL_LINE_SIZE));

        // Out of bounds
        assert!(!buffer.can_insert(row(1), 10));
    }

    #[test]
    fn test_remove_line_with_dynamic_sizing() {
        let mut buffer = ZeroCopyGapBuffer::new();

        // Add three lines
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();

        // Extend the middle line.
        buffer.extend_line_capacity(row(1));

        let line1_offset_before = *buffer.get_line_info(2).unwrap().buffer_start;

        // Remove the extended middle line.
        assert!(buffer.remove_line(row(1)));
        assert_eq!(buffer.line_count(), len(2));

        // Check that the third line's offset was updated correctly.
        let line1_offset_after = *buffer.get_line_info(1).unwrap().buffer_start;
        assert_eq!(line1_offset_after, INITIAL_LINE_SIZE);
        // The extended line had size INITIAL_LINE_SIZE + LINE_PAGE_SIZE = 512
        assert_eq!(
            line1_offset_before - line1_offset_after,
            INITIAL_LINE_SIZE + LINE_PAGE_SIZE
        );
    }

    #[test]
    fn test_null_padding_after_line_creation() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        let line_info = buffer.get_line_info(0).unwrap();
        let buffer_start = *line_info.buffer_start;
        let capacity = line_info.capacity.as_usize();

        // Check that newline is at position 0.
        assert_eq!(buffer.buffer[buffer_start], LINE_FEED_BYTE);

        // Check that the rest of the line capacity is null-padded.
        for i in (buffer_start + 1)..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], NULL_BYTE,
                "Buffer position {} should be null-padded but found: {:?}",
                i, buffer.buffer[i]
            );
        }
    }

    #[test]
    fn test_null_padding_after_capacity_extension() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Extend the line capacity.
        buffer.extend_line_capacity(row(0));

        let line_info = buffer.get_line_info(0).unwrap();
        let buffer_start = *line_info.buffer_start;
        let capacity = line_info.capacity.as_usize();

        // Check that newline is still at position 0.
        assert_eq!(buffer.buffer[buffer_start], LINE_FEED_BYTE);

        // Check that the entire extended capacity is null-padded.
        for i in (buffer_start + 1)..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], NULL_BYTE,
                "Extended buffer position {} should be null-padded but found: {:?}",
                i, buffer.buffer[i]
            );
        }
    }

    #[test]
    fn test_insert_line_shifting_behavior() {
        let mut buffer = ZeroCopyGapBuffer::new();

        // Add three lines
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();

        // Record original offsets.
        let line0_offset = *buffer.get_line_info(0).unwrap().buffer_start;
        let line1_offset = *buffer.get_line_info(1).unwrap().buffer_start;
        let line2_offset = *buffer.get_line_info(2).unwrap().buffer_start;

        assert_eq!(line0_offset, 0);
        assert_eq!(line1_offset, INITIAL_LINE_SIZE);
        assert_eq!(line2_offset, 2 * INITIAL_LINE_SIZE);

        // Test insertion at beginning (should shift all lines)
        buffer.insert_line_with_buffer_shift(0);

        // Check that all lines were shifted.
        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_start, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_start,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(3).unwrap().buffer_start,
            3 * INITIAL_LINE_SIZE
        );

        // Test insertion in middle (should shift lines 2 and 3)
        buffer.insert_line_with_buffer_shift(2);

        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_start, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_start,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(3).unwrap().buffer_start,
            3 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(4).unwrap().buffer_start,
            4 * INITIAL_LINE_SIZE
        );

        // Test insertion at end (no shifting)
        let buffer_len_before = buffer.buffer.len();
        buffer.insert_line_with_buffer_shift(5);
        let buffer_len_after = buffer.buffer.len();

        // Only one line was added at the end.
        assert_eq!(buffer_len_after - buffer_len_before, INITIAL_LINE_SIZE);
    }

    #[test]
    fn test_remove_line_shifting_behavior() {
        let mut buffer = ZeroCopyGapBuffer::new();

        // Add five lines
        for _ in 0..5 {
            buffer.add_line();
        }

        // Test deletion at beginning (should shift all subsequent lines up)
        assert!(buffer.remove_line(row(0)));

        // Check that all lines were shifted up.
        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_start, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_start,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(3).unwrap().buffer_start,
            3 * INITIAL_LINE_SIZE
        );

        // Test deletion in middle (should shift lines 2 and 3 up)
        assert!(buffer.remove_line(row(1)));

        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_start, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_start,
            2 * INITIAL_LINE_SIZE
        );

        // Test deletion at end (no shifting)
        let last_idx = buffer.line_count.convert_to_index();
        let buffer_len_before = buffer.buffer.len();
        assert!(buffer.remove_line(last_idx));
        let buffer_len_after = buffer.buffer.len();

        // Buffer was truncated by one line.
        assert_eq!(buffer_len_before - buffer_len_after, INITIAL_LINE_SIZE);
    }
}

#[cfg(test)]
mod benches {
    use super::*;
    use std::hint::black_box;
    use test::Bencher;

    extern crate test;

    #[bench]
    fn bench_add_line(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();

        b.iter(|| {
            let idx = buffer.add_line();
            black_box(idx);
            // Reset for next iteration.
            buffer.clear();
        });
    }

    #[bench]
    fn bench_add_100_lines(b: &mut Bencher) {
        b.iter(|| {
            let mut buffer = ZeroCopyGapBuffer::new();
            for _ in 0..100 {
                buffer.add_line();
            }
            black_box(buffer.line_count());
        });
    }

    #[bench]
    fn bench_remove_line_middle(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();

        b.iter(|| {
            // Add 10 lines
            for _ in 0..10 {
                buffer.add_line();
            }
            // Remove middle line.
            buffer.remove_line(row(5));
            black_box(buffer.line_count());
            // Reset for next iteration.
            buffer.clear();
        });
    }

    #[bench]
    fn bench_extend_line_capacity(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();

        b.iter(|| {
            buffer.add_line();
            buffer.extend_line_capacity(row(0));
            black_box(buffer.get_line_info(0).unwrap().capacity);
            // Reset for next iteration.
            buffer.clear();
        });
    }

    #[bench]
    fn bench_can_insert_check(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        b.iter(|| {
            let can_insert = buffer.can_insert(row(0), black_box(100));
            black_box(can_insert);
        });
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-copy gap buffer data structure for storing editor content.
//!
//! This module contains the main [`ZeroCopyGapBuffer`] implementation with its
//! core buffer management operations including line creation, deletion, and capacity
//! management.

use super::{GapBufferLine, INITIAL_LINE_SIZE, LINE_PAGE_SIZE, LineMetadata};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CHeight, CLength, CRow,
            CursorBoundsCheck, CursorPositionBoundsStatus, DocSeg, GetMemSize,
            LINE_FEED_BYTE, NULL_BYTE, RangeExt, byte_index, byte_len, byte_offset,
            c_len, c_row, c_width};
use std::{fmt::Display, mem::size_of};

/// Zero-copy gap buffer data structure for storing editor content.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct ZeroCopyGapBuffer {
    /// Contiguous buffer storing all lines. Each line starts at [`INITIAL_LINE_SIZE`]
    /// bytes and can grow.
    ///
    /// [`INITIAL_LINE_SIZE`]: super::INITIAL_LINE_SIZE
    pub buffer: Vec<u8>,

    /// Metadata for each line (grapheme clusters, display width, etc.).
    lines: Vec<LineMetadata>,

    /// Number of lines currently in the buffer.
    line_count: CHeight,
}

impl ZeroCopyGapBuffer {
    /// Creates a new [`ZeroCopyGapBuffer`] with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `line_capacity` - Initial number of lines to allocate capacity for.
    #[must_use]
    pub fn with_capacity(line_capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(line_capacity * INITIAL_LINE_SIZE),
            lines: Vec::with_capacity(line_capacity),
            line_count: crate::c_height(0),
        }
    }

    /// Get the number of lines in the buffer.
    #[must_use]
    pub fn get_line_count(&self) -> CHeight { self.line_count }

    /// Get line metadata by index.
    ///
    /// # Arguments
    ///
    /// * `arg_line_index` - Line index converted into [`CRow`].
    ///
    /// # Returns
    ///
    /// [`Some(&LineMetadata)`] if the index is valid, [`None`] otherwise.
    ///
    /// [`Some(&LineMetadata)`]: LineMetadata
    #[must_use]
    pub fn get_line_info(
        &self,
        arg_line_index: impl Into<CRow>,
    ) -> Option<&LineMetadata> {
        let line_index: CRow = arg_line_index.into();
        self.lines.get(line_index.as_usize())
    }

    /// Get mutable line metadata by index.
    ///
    /// # Arguments
    ///
    /// * `arg_line_index` - Line index converted into [`CRow`].
    ///
    /// # Returns
    ///
    /// [`Some(&mut LineMetadata)`] if the index is valid, [`None`] otherwise.
    ///
    /// [`Some(&mut LineMetadata)`]: LineMetadata
    pub fn get_line_info_mut(
        &mut self,
        arg_line_index: impl Into<CRow>,
    ) -> Option<&mut LineMetadata> {
        let line_index: CRow = arg_line_index.into();
        self.lines.get_mut(line_index.as_usize())
    }

    /// Swap two lines in the buffer metadata.
    ///
    /// This only swaps the [`LineMetadata`] entries, not the actual buffer content.
    ///
    /// # Arguments
    ///
    /// * `arg_i` - First line index converted into [`CRow`].
    /// * `arg_j` - Second line index converted into [`CRow`].
    pub fn swap_lines(&mut self, arg_i: impl Into<CRow>, arg_j: impl Into<CRow>) {
        let i: CRow = arg_i.into();
        let j: CRow = arg_j.into();
        self.lines.swap(i.as_usize(), j.as_usize());
    }

    /// Insert a new empty line at the specified position with proper buffer shifting.
    ///
    /// This method properly maintains the invariant that lines are ordered by their
    /// buffer offsets by actually shifting buffer content.
    ///
    /// # Buffer Shifting Behavior
    ///
    /// - **Insertion at end**: No shifting needed, just appends a new line to the buffer
    /// - **Insertion at start/middle**: Shifts all subsequent buffer content down by
    ///   [`INITIAL_LINE_SIZE`] bytes to make room for the new line
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
    pub fn insert_line_with_buffer_shift(&mut self, arg_line_index: impl Into<CRow>) {
        let line_index: CRow = arg_line_index.into();
        let line_idx = line_index.as_usize();

        // If inserting at the end, just add a new line.
        // Use cursor position bounds checking which allows insertion at line_count (end).
        if self.line_count.check_cursor_position_bounds(line_index)
            == CursorPositionBoundsStatus::AtEnd
        {
            self.add_line();
            return;
        }

        self.insert_empty_line_at(line_idx);
    }

    /// Insert an empty line at the specified line index.
    ///
    /// # Arguments
    ///
    /// * `line_idx` - The line index where to insert the new line
    ///
    /// # Example
    ///
    /// ```text
    /// Before insert_empty_line_at(1):
    /// [Line 0: 256 bytes][Line 1: 256 bytes]
    ///
    /// After insert_empty_line_at(1):
    /// [Line 0: 256 bytes][Line 1 (new): 256 bytes][Line 2 (was 1): 256 bytes]
    /// ```
    pub fn insert_empty_line_at(&mut self, line_idx: usize) {
        let insert_offset = if line_idx < self.lines.len() {
            self.lines[line_idx].buffer_start
        } else {
            byte_index(self.buffer.len())
        };

        let shift_amount = INITIAL_LINE_SIZE;
        let old_buffer_len = self.buffer.len();
        self.buffer.resize(old_buffer_len + shift_amount, NULL_BYTE);

        let shift_start = insert_offset.as_usize();
        if shift_start < old_buffer_len {
            self.buffer
                .copy_within(shift_start..old_buffer_len, shift_start + shift_amount);
        }

        self.buffer[shift_start..shift_start + shift_amount].fill(NULL_BYTE);

        self.buffer[shift_start] = LINE_FEED_BYTE;

        let new_line_info = LineMetadata {
            buffer_start: insert_offset,
            content_byte_len: byte_len(0usize),
            capacity: byte_len(INITIAL_LINE_SIZE),
            grapheme_segments: Vec::new(),
            display_width: c_width(0usize),
            grapheme_count: c_len(0usize),
        };

        self.lines.insert(line_idx, new_line_info);

        for i in (line_idx + 1)..self.lines.len() {
            self.lines[i].buffer_start += byte_offset(shift_amount);
        }

        self.line_count = crate::c_height(self.line_count.as_usize() + 1);
    }

    /// Appends a new empty line to the end of the buffer.
    ///
    /// Allocates [`INITIAL_LINE_SIZE`] bytes for the new line, initializes it with a
    /// [`LINE_FEED_BYTE`], creates corresponding [`LineMetadata`], and increments the
    /// line count.
    ///
    /// # Returns
    ///
    /// The `usize` index of the newly added line.
    ///
    /// [`INITIAL_LINE_SIZE`]: super::INITIAL_LINE_SIZE
    pub fn add_line(&mut self) -> usize {
        let line_index = self.line_count.as_usize();

        let buffer_pos = if self.line_count.as_usize() == 0 {
            byte_index(0)
        } else {
            let prev_line = &self.lines[line_index - 1];
            prev_line.buffer_start + byte_offset(prev_line.capacity.as_usize())
        };

        self.buffer
            .resize(self.buffer.len() + INITIAL_LINE_SIZE, NULL_BYTE);

        self.buffer[*buffer_pos] = LINE_FEED_BYTE;

        self.lines.push(LineMetadata {
            buffer_start: buffer_pos,
            content_byte_len: byte_len(0usize),
            capacity: byte_len(INITIAL_LINE_SIZE),
            grapheme_segments: Vec::new(),
            display_width: c_width(0usize),
            grapheme_count: c_len(0usize),
        });

        self.line_count = crate::c_height(self.line_count.as_usize() + 1);
        line_index
    }

    /// Remove a line from the buffer.
    ///
    /// # Arguments
    ///
    /// * `arg_line_index` - Line index converted into [`CRow`].
    ///
    /// # Returns
    ///
    /// [`Some(LineMetadata)`] if the line was removed, or [`None`] if the
    /// line index was out of bounds.
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
    ///
    /// [`Some(LineMetadata)`]: LineMetadata
    pub fn remove_line(
        &mut self,
        arg_line_index: impl Into<CRow>,
    ) -> Option<LineMetadata> {
        let line_index: CRow = arg_line_index.into();
        if line_index.overflows(self.line_count) == ArrayOverflowResult::Overflowed {
            return None;
        }

        let removed_line = self.lines.remove(line_index.as_usize());
        let removed_start = *removed_line.buffer_start;
        let removed_size = removed_line.capacity.as_usize();

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
        let line_range = line_index..c_row(self.line_count.as_usize() - 1);
        for line_idx in line_range.as_index_iter() {
            if let Some(line) = self.lines.get_mut(line_idx.as_usize()) {
                line.buffer_start = line.buffer_start - byte_offset(removed_size);
            }
        }

        self.line_count = crate::c_height(self.line_count.as_usize() - 1);
        Some(removed_line)
    }

    /// Clear all lines from the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.lines.clear();
        self.line_count = crate::c_height(0);
    }

    /// Extend the capacity of a line by [`LINE_PAGE_SIZE`].
    ///
    /// Shifts subsequent line buffer contents to allocate an additional page of memory
    /// for the specified line, and updates buffer offsets for all following lines.
    ///
    /// # Arguments
    ///
    /// * `arg_line_index` - Line index converted into [`CRow`].
    ///
    /// [`LINE_PAGE_SIZE`]: super::LINE_PAGE_SIZE
    pub fn extend_line_capacity(&mut self, arg_line_index: impl Into<CRow>) {
        let line_index: CRow = arg_line_index.into();
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
        self.buffer[insert_pos..insert_pos + shift_amount].fill(NULL_BYTE);

        // Update line capacity.
        self.lines[line_index.as_usize()].capacity = byte_len(new_capacity);

        // Update buffer offsets for subsequent lines.
        let line_range = (line_index + 1)..c_row(self.line_count.as_usize());
        for line_idx in line_range.as_index_iter() {
            if let Some(line) = self.lines.get_mut(line_idx.as_usize()) {
                line.buffer_start += byte_offset(shift_amount);
            }
        }
    }
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

impl GetMemSize for ZeroCopyGapBuffer {
    fn get_mem_size(&self) -> usize {
        let buffer_size = self.buffer.len() * size_of::<u8>();
        let lines_size = self.lines.len() * size_of::<LineMetadata>();
        let line_metadata_size: usize = self
            .lines
            .iter()
            .map(|line| line.grapheme_segments.len() * size_of::<DocSeg>())
            .sum();

        buffer_size + lines_size + line_metadata_size + size_of::<CLength>()
    }
}

/// Iterator over lines in a [`ZeroCopyGapBuffer`].
#[derive(Debug)]
pub struct ZeroCopyLineIterator<'a> {
    buffer: &'a ZeroCopyGapBuffer,
    current: usize,
}

impl<'a> Iterator for ZeroCopyLineIterator<'a> {
    type Item = GapBufferLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_index = c_row(self.current);
        if current_index.overflows(self.buffer.get_line_count())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LengthOps;

    #[test]
    fn test_new_line_buffer() {
        let buffer = ZeroCopyGapBuffer::default();
        assert_eq!(buffer.get_line_count(), crate::c_height(0));
        assert!(buffer.buffer.is_empty());
        assert!(buffer.lines.is_empty());
    }

    #[test]
    fn test_add_line_with_dynamic_sizing() {
        let mut buffer = ZeroCopyGapBuffer::default();

        // Add first line
        let idx1 = buffer.add_line();
        assert_eq!(idx1, 0);
        assert_eq!(buffer.get_line_count(), crate::c_height(1));
        assert_eq!(buffer.buffer.len(), INITIAL_LINE_SIZE);

        let line_info = buffer.get_line_info(0).expect("conversion error");
        assert_eq!(*line_info.buffer_start, 0);
        assert_eq!(line_info.capacity, byte_len(INITIAL_LINE_SIZE));
        assert_eq!(line_info.content_byte_len, byte_len(0));

        // Add second line
        let idx2 = buffer.add_line();
        assert_eq!(idx2, 1);
        assert_eq!(buffer.get_line_count(), crate::c_height(2));
        assert_eq!(buffer.buffer.len(), 2 * INITIAL_LINE_SIZE);

        let line_info = buffer.get_line_info(1).expect("conversion error");
        assert_eq!(*line_info.buffer_start, INITIAL_LINE_SIZE);
        assert_eq!(line_info.capacity, byte_len(INITIAL_LINE_SIZE));
    }

    #[test]
    fn test_extend_line_capacity() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let original_capacity =
            buffer.get_line_info(0).expect("conversion error").capacity;
        assert_eq!(original_capacity, byte_len(INITIAL_LINE_SIZE));

        // Extend the line
        buffer.extend_line_capacity(c_row(0));

        let new_capacity = buffer.get_line_info(0).expect("conversion error").capacity;
        assert_eq!(new_capacity, byte_len(INITIAL_LINE_SIZE + LINE_PAGE_SIZE));
        assert_eq!(buffer.buffer.len(), INITIAL_LINE_SIZE + LINE_PAGE_SIZE);
    }

    #[test]
    fn test_remove_line_with_dynamic_sizing() {
        let mut buffer = ZeroCopyGapBuffer::default();

        // Add three lines
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();

        // Extend the middle line.
        buffer.extend_line_capacity(c_row(1));

        let line1_offset_before = *buffer
            .get_line_info(2)
            .expect("conversion error")
            .buffer_start;

        // Remove the extended middle line.
        assert!(buffer.remove_line(c_row(1)).is_some());
        assert_eq!(buffer.get_line_count(), crate::c_height(2));

        // Check that the third line's offset was updated correctly.
        let line1_offset_after = *buffer
            .get_line_info(1)
            .expect("conversion error")
            .buffer_start;
        assert_eq!(line1_offset_after, INITIAL_LINE_SIZE);
        // The extended line had size INITIAL_LINE_SIZE + LINE_PAGE_SIZE = 512
        assert_eq!(
            line1_offset_before - line1_offset_after,
            INITIAL_LINE_SIZE + LINE_PAGE_SIZE
        );
    }

    #[test]
    fn test_null_padding_after_line_creation() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        let line_info = buffer.get_line_info(0).expect("conversion error");
        let buffer_start = *line_info.buffer_start;
        let capacity = line_info.capacity.as_usize();

        // Check that newline is at position 0.
        assert_eq!(buffer.buffer[buffer_start], LINE_FEED_BYTE);

        // Check that the rest of the line capacity is null-padded.
        for (idx, &byte) in buffer.buffer[(buffer_start + 1)..(buffer_start + capacity)]
            .iter()
            .enumerate()
        {
            let i = buffer_start + 1 + idx;
            assert_eq!(
                byte, NULL_BYTE,
                "Buffer position {i} should be null-padded but found: {byte:?}"
            );
        }
    }

    #[test]
    fn test_null_padding_after_capacity_extension() {
        let mut buffer = ZeroCopyGapBuffer::default();
        buffer.add_line();

        // Extend the line capacity.
        buffer.extend_line_capacity(c_row(0));

        let line_info = buffer.get_line_info(0).expect("conversion error");
        let buffer_start = *line_info.buffer_start;
        let capacity = line_info.capacity.as_usize();

        // Check that newline is still at position 0.
        assert_eq!(buffer.buffer[buffer_start], LINE_FEED_BYTE);

        // Check that the entire extended capacity is null-padded.
        for (idx, &byte) in buffer.buffer[(buffer_start + 1)..(buffer_start + capacity)]
            .iter()
            .enumerate()
        {
            let i = buffer_start + 1 + idx;
            assert_eq!(
                byte, NULL_BYTE,
                "Extended buffer position {i} should be null-padded but found: {byte:?}"
            );
        }
    }

    #[test]
    fn test_insert_line_shifting_behavior() {
        let mut buffer = ZeroCopyGapBuffer::default();

        // Add three lines
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();

        // Record original offsets.
        let line0_offset = *buffer
            .get_line_info(0)
            .expect("conversion error")
            .buffer_start;
        let line1_offset = *buffer
            .get_line_info(1)
            .expect("conversion error")
            .buffer_start;
        let line2_offset = *buffer
            .get_line_info(2)
            .expect("conversion error")
            .buffer_start;

        assert_eq!(line0_offset, 0);
        assert_eq!(line1_offset, INITIAL_LINE_SIZE);
        assert_eq!(line2_offset, 2 * INITIAL_LINE_SIZE);

        // Test insertion at beginning (should shift all lines)
        buffer.insert_line_with_buffer_shift(0);

        // Check that all lines were shifted.
        assert_eq!(
            *buffer
                .get_line_info(0)
                .expect("conversion error")
                .buffer_start,
            0
        );
        assert_eq!(
            *buffer
                .get_line_info(1)
                .expect("conversion error")
                .buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(2)
                .expect("conversion error")
                .buffer_start,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(3)
                .expect("conversion error")
                .buffer_start,
            3 * INITIAL_LINE_SIZE
        );

        // Test insertion in middle (should shift lines 2 and 3)
        buffer.insert_line_with_buffer_shift(2);

        assert_eq!(
            *buffer
                .get_line_info(0)
                .expect("conversion error")
                .buffer_start,
            0
        );
        assert_eq!(
            *buffer
                .get_line_info(1)
                .expect("conversion error")
                .buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(2)
                .expect("conversion error")
                .buffer_start,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(3)
                .expect("conversion error")
                .buffer_start,
            3 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(4)
                .expect("conversion error")
                .buffer_start,
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
        let mut buffer = ZeroCopyGapBuffer::default();

        // Add five lines
        for _ in 0..5 {
            buffer.add_line();
        }

        // Test deletion at beginning (should shift all subsequent lines up)
        assert!(buffer.remove_line(c_row(0)).is_some());

        // Check that all lines were shifted up.
        assert_eq!(
            *buffer
                .get_line_info(0)
                .expect("conversion error")
                .buffer_start,
            0
        );
        assert_eq!(
            *buffer
                .get_line_info(1)
                .expect("conversion error")
                .buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(2)
                .expect("conversion error")
                .buffer_start,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(3)
                .expect("conversion error")
                .buffer_start,
            3 * INITIAL_LINE_SIZE
        );

        // Test deletion in middle (should shift lines 2 and 3 up)
        assert!(buffer.remove_line(c_row(1)).is_some());

        assert_eq!(
            *buffer
                .get_line_info(0)
                .expect("conversion error")
                .buffer_start,
            0
        );
        assert_eq!(
            *buffer
                .get_line_info(1)
                .expect("conversion error")
                .buffer_start,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer
                .get_line_info(2)
                .expect("conversion error")
                .buffer_start,
            2 * INITIAL_LINE_SIZE
        );

        // Test deletion at end (no shifting)
        let last_idx = buffer.line_count.convert_to_index();
        let buffer_len_before = buffer.buffer.len();
        assert!(buffer.remove_line(last_idx).is_some());
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
        let mut buffer = ZeroCopyGapBuffer::default();

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
            let mut buffer = ZeroCopyGapBuffer::default();
            for _ in 0..100 {
                buffer.add_line();
            }
            black_box(buffer.get_line_count());
        });
    }

    #[bench]
    fn bench_remove_line_middle(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::default();

        b.iter(|| {
            // Add 10 lines
            for _ in 0..10 {
                buffer.add_line();
            }
            // Remove middle line.
            buffer.remove_line(c_row(5));
            black_box(buffer.get_line_count());
            // Reset for next iteration.
            buffer.clear();
        });
    }

    #[bench]
    fn bench_extend_line_capacity(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::default();

        b.iter(|| {
            buffer.add_line();
            buffer.extend_line_capacity(c_row(0));
            black_box(buffer.get_line_info(0).expect("conversion error").capacity);
            // Reset for next iteration.
            buffer.clear();
        });
    }
}

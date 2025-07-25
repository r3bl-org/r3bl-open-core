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

//! Buffer storage implementation for the [`ZeroCopyGapBuffer`] data structure.
//!
//! This module contains the core storage mechanisms for the dynamically-sized line buffer
//! system. Each line starts with 256 bytes of capacity and can grow in 256-byte
//! increments as needed to accommodate larger content.
//!
//! # Storage Architecture
//!
//! ## Line Layout
//! Each line follows this storage pattern:
//! ```text
//! [content bytes][newline (\n)][null padding (\0)...]
//! ```
//!
//! ## Dynamic Growth
//! - **Initial capacity**: 256 bytes (`INITIAL_LINE_SIZE`)
//! - **Growth increment**: 256 bytes (`LINE_PAGE_SIZE`)
//! - **Growth trigger**: When content + newline exceeds current capacity
//! - **Buffer management**: Subsequent lines are shifted to accommodate growth
//!
//! ## Null-Padding Invariant
//!
//! **CRITICAL**: This module maintains a strict invariant that all unused capacity
//! in each line buffer MUST be filled with null bytes (`\0`). This invariant is
//! essential for:
//!
//! - **Security**: Prevents information leakage from uninitialized memory
//! - **Correctness**: Ensures predictable buffer state for zero-copy operations
//! - **Performance**: Enables safe slice operations without bounds checking
//!
//! All operations in this module MUST maintain this invariant by:
//! 1. Initializing new memory with `\0` (see [`add_line`][ZeroCopyGapBuffer::add_line], [`extend_line_capacity`][ZeroCopyGapBuffer::extend_line_capacity])
//! 2. Clearing gaps left by content shifts (see [`remove_line`][ZeroCopyGapBuffer::remove_line])
//! 3. Padding unused capacity after modifications
//!
//! Violation of this invariant may lead to buffer corruption, security vulnerabilities,
//! or undefined behavior in zero-copy access operations.

use crate::{ByteIndex, ColWidth, Length, RowIndex, byte_index,
            gc_string_sizing::SegmentArray, len};

/// Initial size of each line in bytes
pub const INITIAL_LINE_SIZE: usize = 256;

/// Page size for extending lines (bytes added when line overflows)
pub const LINE_PAGE_SIZE: usize = 256;

/// Zero-copy gap buffer data structure for storing editor content
#[derive(Debug, Clone)]
pub struct ZeroCopyGapBuffer {
    /// Contiguous buffer storing all lines
    /// Each line starts at `INITIAL_LINE_SIZE` bytes and can grow
    pub buffer: Vec<u8>,

    /// Metadata for each line (grapheme clusters, display width, etc.)
    lines: Vec<GapBufferLineInfo>,

    /// Number of lines currently in the buffer
    line_count: usize,
}

/// Metadata for a single line in the buffer
#[derive(Debug, Clone)]
pub struct GapBufferLineInfo {
    /// Where this line starts in the buffer
    pub buffer_offset: ByteIndex,

    /// Actual content length in bytes (before '\n')
    pub content_len: Length,

    /// Allocated capacity for this line
    pub capacity: Length,

    /// Segment array for this line (grapheme cluster information)
    pub segments: SegmentArray,

    /// Display width of the line
    pub display_width: ColWidth,

    /// Number of grapheme clusters
    pub grapheme_count: usize,
}

impl GapBufferLineInfo {
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
    pub fn content_range(&self) -> std::ops::Range<usize> {
        let start = self.buffer_offset.as_usize();
        let end = start + self.content_len.as_usize();
        start..end
    }
}

impl ZeroCopyGapBuffer {
    /// Create a new empty [`ZeroCopyGapBuffer`]
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            lines: Vec::new(),
            line_count: 0,
        }
    }

    /// Create a new [`ZeroCopyGapBuffer`] with pre-allocated capacity
    #[must_use]
    pub fn with_capacity(line_capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(line_capacity * INITIAL_LINE_SIZE),
            lines: Vec::with_capacity(line_capacity),
            line_count: 0,
        }
    }

    /// Get the number of lines in the buffer
    #[must_use]
    pub fn line_count(&self) -> usize { self.line_count }

    /// Get line metadata by index
    #[must_use]
    pub fn get_line_info(&self, line_index: usize) -> Option<&GapBufferLineInfo> {
        self.lines.get(line_index)
    }

    /// Get mutable line metadata by index
    pub fn get_line_info_mut(
        &mut self,
        line_index: usize,
    ) -> Option<&mut GapBufferLineInfo> {
        self.lines.get_mut(line_index)
    }

    /// Swap two lines in the buffer metadata
    /// This only swaps the [`GapBufferLineInfo`] entries, not the actual buffer content
    pub fn swap_lines(&mut self, i: usize, j: usize) { self.lines.swap(i, j); }

    /// Add a new line to the buffer
    /// Returns the index of the newly added line
    pub fn add_line(&mut self) -> usize {
        let line_index = self.line_count;

        // Calculate where this line starts in the buffer
        let buffer_offset = if line_index == 0 {
            byte_index(0)
        } else {
            let prev_line = &self.lines[line_index - 1];
            byte_index(*prev_line.buffer_offset + prev_line.capacity.as_usize())
        };

        // Extend buffer by INITIAL_LINE_SIZE bytes, all initialized to '\0'
        self.buffer
            .resize(self.buffer.len() + INITIAL_LINE_SIZE, b'\0');

        // Add the newline character at the start (empty line)
        self.buffer[*buffer_offset] = b'\n';

        // Create line metadata
        self.lines.push(GapBufferLineInfo {
            buffer_offset,
            content_len: len(0),
            capacity: len(INITIAL_LINE_SIZE),
            segments: SegmentArray::new(),
            display_width: crate::width(0),
            grapheme_count: 0,
        });

        self.line_count += 1;
        line_index
    }

    /// Remove a line from the buffer
    /// Returns true if the line was removed, false if index was out of bounds
    pub fn remove_line(&mut self, line_index: usize) -> bool {
        if line_index >= self.line_count {
            return false;
        }

        let removed_line = &self.lines[line_index];
        let removed_start = *removed_line.buffer_offset;
        let removed_size = removed_line.capacity.as_usize();

        // Remove from metadata
        self.lines.remove(line_index);

        // Shift buffer contents
        let shift_start = removed_start + removed_size;
        let buffer_len = self.buffer.len();

        // Move all subsequent bytes up
        for i in shift_start..buffer_len {
            self.buffer[i - removed_size] = self.buffer[i];
        }

        // Truncate the buffer
        self.buffer.truncate(buffer_len - removed_size);

        // Update buffer offsets for remaining lines
        for line in self.lines.iter_mut().skip(line_index) {
            line.buffer_offset = byte_index(*line.buffer_offset - removed_size);
        }

        self.line_count -= 1;
        true
    }

    /// Clear all lines from the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.lines.clear();
        self.line_count = 0;
    }

    /// Check if a line can accommodate additional bytes without reallocation
    #[must_use]
    pub fn can_insert(&self, line_index: RowIndex, additional_bytes: usize) -> bool {
        if let Some(line_info) = self.get_line_info(line_index.as_usize()) {
            // Need space for content + newline
            line_info.content_len.as_usize() + additional_bytes
                < line_info.capacity.as_usize()
        } else {
            false
        }
    }

    /// Extend the capacity of a line by `LINE_PAGE_SIZE`
    pub fn extend_line_capacity(&mut self, line_index: RowIndex) {
        if line_index.as_usize() >= self.line_count {
            return;
        }

        let line_info = &self.lines[line_index.as_usize()];
        let line_start = *line_info.buffer_offset;
        let old_capacity = line_info.capacity.as_usize();
        let new_capacity = old_capacity + LINE_PAGE_SIZE;

        // Calculate how much to shift subsequent content
        let shift_amount = LINE_PAGE_SIZE;
        let insert_pos = line_start + old_capacity;

        // Extend buffer to accommodate new capacity
        self.buffer.resize(self.buffer.len() + shift_amount, b'\0');

        // Shift all subsequent content to the right
        for i in (insert_pos..self.buffer.len() - shift_amount).rev() {
            self.buffer[i + shift_amount] = self.buffer[i];
        }

        // Fill the newly allocated space with nulls
        for i in insert_pos..insert_pos + shift_amount {
            self.buffer[i] = b'\0';
        }

        // Update line capacity
        self.lines[line_index.as_usize()].capacity = len(new_capacity);

        // Update buffer offsets for subsequent lines
        for line in self.lines.iter_mut().skip(line_index.as_usize() + 1) {
            line.buffer_offset = byte_index(*line.buffer_offset + shift_amount);
        }
    }
}

impl Default for ZeroCopyGapBuffer {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for ZeroCopyGapBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ZeroCopyGapBuffer {{ lines: {}, buffer_size: {} bytes }}",
            self.line_count,
            self.buffer.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row;

    #[test]
    fn test_new_line_buffer() {
        let buffer = ZeroCopyGapBuffer::new();
        assert_eq!(buffer.line_count(), 0);
        assert!(buffer.buffer.is_empty());
        assert!(buffer.lines.is_empty());
    }

    #[test]
    fn test_add_line_with_dynamic_sizing() {
        let mut buffer = ZeroCopyGapBuffer::new();

        // Add first line
        let idx1 = buffer.add_line();
        assert_eq!(idx1, 0);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.buffer.len(), INITIAL_LINE_SIZE);

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(*line_info.buffer_offset, 0);
        assert_eq!(line_info.capacity, len(INITIAL_LINE_SIZE));
        assert_eq!(line_info.content_len, len(0));

        // Add second line
        let idx2 = buffer.add_line();
        assert_eq!(idx2, 1);
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.buffer.len(), 2 * INITIAL_LINE_SIZE);

        let line_info = buffer.get_line_info(1).unwrap();
        assert_eq!(*line_info.buffer_offset, INITIAL_LINE_SIZE);
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

        // Extend the middle line
        buffer.extend_line_capacity(row(1));

        let line1_offset_before = *buffer.get_line_info(2).unwrap().buffer_offset;

        // Remove the extended middle line
        assert!(buffer.remove_line(1));
        assert_eq!(buffer.line_count(), 2);

        // Check that the third line's offset was updated correctly
        let line1_offset_after = *buffer.get_line_info(1).unwrap().buffer_offset;
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
        let buffer_start = *line_info.buffer_offset;
        let capacity = line_info.capacity.as_usize();

        // Check that newline is at position 0
        assert_eq!(buffer.buffer[buffer_start], b'\n');

        // Check that the rest of the line capacity is null-padded
        for i in (buffer_start + 1)..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], b'\0',
                "Buffer position {} should be null-padded but found: {:?}",
                i, buffer.buffer[i]
            );
        }
    }

    #[test]
    fn test_null_padding_after_capacity_extension() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Extend the line capacity
        buffer.extend_line_capacity(row(0));

        let line_info = buffer.get_line_info(0).unwrap();
        let buffer_start = *line_info.buffer_offset;
        let capacity = line_info.capacity.as_usize();

        // Check that newline is still at position 0
        assert_eq!(buffer.buffer[buffer_start], b'\n');

        // Check that the entire extended capacity is null-padded
        for i in (buffer_start + 1)..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], b'\0',
                "Extended buffer position {} should be null-padded but found: {:?}",
                i, buffer.buffer[i]
            );
        }
    }
}

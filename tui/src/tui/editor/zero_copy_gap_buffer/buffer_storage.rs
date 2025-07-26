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
//! 1. Initializing new memory with `\0` (see [`add_line`][ZeroCopyGapBuffer::add_line],
//!    [`extend_line_capacity`][ZeroCopyGapBuffer::extend_line_capacity])
//! 2. Clearing gaps left by content shifts (see
//!    [`remove_line`][ZeroCopyGapBuffer::remove_line])
//! 3. Padding unused capacity after modifications
//!
//! Violation of this invariant may lead to buffer corruption, security vulnerabilities,
//! or undefined behavior in zero-copy access operations.

use crate::{ByteIndex, ColWidth, Length, RowIndex, SegIndex, byte_index,
            gc_string_sizing::SegmentArray, len};

/// Initial size of each line in bytes
pub const INITIAL_LINE_SIZE: usize = 256;

/// Page size for extending lines (bytes added when line overflows)
pub const LINE_PAGE_SIZE: usize = 256;

/// Segment rebuild strategy based on the type of text modification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentRebuildStrategy {
    /// Full rebuild required - parse entire line content
    Full,
    /// Optimized append - only parse appended text and adjust offsets
    AppendOptimized,
    // Future: could add more strategies like PrependOptimized, SingleCharOptimized, etc.
}

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
    
    /// Get the byte position for a given segment index
    ///
    /// This method converts a grapheme cluster index (segment index) to its
    /// corresponding byte position in the line buffer. It handles three cases:
    /// - Beginning of line (`seg_index` = 0) → returns 0
    /// - End of line (`seg_index` >= `segments.len()`) → returns `content_len`
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
    /// buffer.insert_at_grapheme(row(0), seg_index(0), "Hello").unwrap();
    ///
    /// let line_info = buffer.get_line_info(0).unwrap();
    /// 
    /// // Beginning of line
    /// assert_eq!(line_info.get_byte_pos(seg_index(0)).as_usize(), 0);
    /// 
    /// // End of line
    /// assert_eq!(line_info.get_byte_pos(seg_index(5)).as_usize(), 5);
    /// ```
    #[must_use]
    pub fn get_byte_pos(&self, seg_index: SegIndex) -> ByteIndex {
        if seg_index.as_usize() == 0 {
            // Insert at beginning
            byte_index(0)
        } else if seg_index.as_usize() >= self.segments.len() {
            // Insert at end
            byte_index(self.content_len.as_usize())
        } else {
            // Insert in middle - find the start of the target segment
            let segment = &self.segments[seg_index.as_usize()];
            byte_index(segment.start_byte_index.as_usize())
        }
    }
    
    /// Get the segment index for a given byte position
    ///
    /// This method converts a byte position to its corresponding grapheme cluster
    /// index (segment index). It handles three cases:
    /// - Beginning of line (`byte_pos` = 0) → returns SegIndex(0)
    /// - End of line (`byte_pos` >= `content_len`) → returns SegIndex(segments.len())
    /// - Middle of line → returns the `seg_index` of the segment containing the byte
    ///
    /// # Arguments
    /// * `byte_pos` - The byte position to convert
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
    /// buffer.insert_at_grapheme(row(line_idx), seg_index(0), "H😀llo").unwrap();
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
    pub fn get_seg_index(&self, byte_pos: ByteIndex) -> SegIndex {
        // Handle edge cases
        if byte_pos.as_usize() == 0 {
            return crate::seg_index(0);
        }
        
        if byte_pos.as_usize() >= self.content_len.as_usize() {
            return crate::seg_index(self.segments.len());
        }
        
        // Binary search through segments to find the one containing byte_pos
        // We could optimize this with binary search, but linear is fine for now
        // since lines typically have few segments
        for segment in &self.segments {
            if byte_pos.as_usize() >= segment.start_byte_index.as_usize() 
                && byte_pos.as_usize() < segment.end_byte_index.as_usize() {
                return segment.seg_index;
            }
        }
        
        // If we get here, byte_pos is between segments (shouldn't happen with valid UTF-8)
        // Return the segment after the position
        for segment in &self.segments {
            if byte_pos.as_usize() < segment.start_byte_index.as_usize() {
                return segment.seg_index;
            }
        }
        
        // Fallback to end of line
        crate::seg_index(self.segments.len())
    }
    
    /// Determine the optimal segment rebuild strategy for a text modification
    ///
    /// This method analyzes the modification position and line state to determine
    /// whether we can use an optimized rebuild strategy or need a full rebuild.
    ///
    /// # Arguments
    ///
    /// * `modification_position` - The segment index where text is being inserted
    ///
    /// # Returns
    ///
    /// The recommended [`SegmentRebuildStrategy`] based on the current line state
    /// and modification position.
    ///
    /// # Strategy Selection Logic
    ///
    /// - **`AppendOptimized`**: When inserting at the end of a non-empty line
    /// - **Full**: For all other cases (empty line, middle insertion, etc.)
    ///
    /// # Future Extensions
    ///
    /// This method can be extended to detect more optimization opportunities:
    /// - Single character insertions (could use specialized handling)
    /// - ASCII-only content (faster segment building)
    /// - Prepend operations (inserting at beginning)
    /// - Small deletions (might not need full rebuild)
    #[must_use]
    pub fn determine_segment_rebuild_strategy(
        &self,
        modification_position: SegIndex,
    ) -> SegmentRebuildStrategy {
        // Check if this is an append at the end
        let is_end_append = modification_position.as_usize() >= self.segments.len();
        
        // Check if the line has existing content
        let has_content = !self.segments.is_empty();
        
        // Determine the strategy
        match (is_end_append, has_content) {
            (true, true) => SegmentRebuildStrategy::AppendOptimized,
            _ => SegmentRebuildStrategy::Full,
        }
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
    use crate::{row, seg_index};

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

    #[test]
    fn test_get_byte_pos_beginning() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Insert some text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello").unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // Test beginning position
        assert_eq!(line_info.get_byte_pos(seg_index(0)).as_usize(), 0);
    }

    #[test]
    fn test_get_byte_pos_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Insert some text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello").unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // Test end position (past last segment)
        assert_eq!(line_info.get_byte_pos(seg_index(5)).as_usize(), 5);
        assert_eq!(line_info.get_byte_pos(seg_index(10)).as_usize(), 5);
    }

    #[test]
    fn test_get_byte_pos_middle() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Insert text with multi-byte characters
        buffer.insert_at_grapheme(row(0), seg_index(0), "H😀llo").unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // Test various positions
        assert_eq!(line_info.get_byte_pos(seg_index(0)).as_usize(), 0); // Before 'H'
        assert_eq!(line_info.get_byte_pos(seg_index(1)).as_usize(), 1); // Before '😀'
        assert_eq!(line_info.get_byte_pos(seg_index(2)).as_usize(), 5); // Before 'l' (emoji is 4 bytes)
        assert_eq!(line_info.get_byte_pos(seg_index(3)).as_usize(), 6); // Before second 'l'
        assert_eq!(line_info.get_byte_pos(seg_index(4)).as_usize(), 7); // Before 'o'
        assert_eq!(line_info.get_byte_pos(seg_index(5)).as_usize(), 8); // End of string
    }

    #[test]
    fn test_get_byte_pos_empty_line() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // For empty line, any position should return 0
        assert_eq!(line_info.get_byte_pos(seg_index(0)).as_usize(), 0);
        assert_eq!(line_info.get_byte_pos(seg_index(1)).as_usize(), 0);
        assert_eq!(line_info.get_byte_pos(seg_index(100)).as_usize(), 0);
    }

    #[test]
    fn test_get_seg_index_beginning() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Insert some text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello").unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // Test beginning position
        assert_eq!(line_info.get_seg_index(byte_index(0)), seg_index(0));
    }

    #[test]
    fn test_get_seg_index_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Insert some text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello").unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // Test end position (at or past content length)
        assert_eq!(line_info.get_seg_index(byte_index(5)), seg_index(5));
        assert_eq!(line_info.get_seg_index(byte_index(10)), seg_index(5));
    }

    #[test]
    fn test_get_seg_index_middle() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Insert text with emoji: "H😀llo"
        buffer.insert_at_grapheme(row(0), seg_index(0), "H😀llo").unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // Test various byte positions
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
        
        // For empty line, any position should return 0
        assert_eq!(line_info.get_seg_index(byte_index(0)), seg_index(0));
        assert_eq!(line_info.get_seg_index(byte_index(1)), seg_index(0));
        assert_eq!(line_info.get_seg_index(byte_index(100)), seg_index(0));
    }

    #[test]
    fn test_get_seg_index_get_byte_pos_round_trip() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Insert text with various Unicode: "a👨‍👩‍👧‍👦b世界c"
        buffer.insert_at_grapheme(row(0), seg_index(0), "a👨‍👩‍👧‍👦b世界c").unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        
        // Test round-trip conversion for each segment
        for i in 0..line_info.segments.len() {
            let seg_idx = seg_index(i);
            let byte_pos = line_info.get_byte_pos(seg_idx);
            let seg_idx_back = line_info.get_seg_index(byte_pos);
            assert_eq!(seg_idx, seg_idx_back, 
                "Round-trip failed for segment {}: byte_pos={}", i, byte_pos.as_usize());
        }
    }
}

#[cfg(test)]
mod benches {
    use std::hint::black_box;

    use test::Bencher;

    use super::*;
    use crate::row;

    extern crate test;

    #[bench]
    fn bench_add_line(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();

        b.iter(|| {
            let idx = buffer.add_line();
            black_box(idx);
            // Reset for next iteration
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
            // Remove middle line
            buffer.remove_line(5);
            black_box(buffer.line_count());
            // Reset for next iteration
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
            // Reset for next iteration
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

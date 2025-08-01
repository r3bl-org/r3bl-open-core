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
//! ## Buffer Shifting Behavior
//!
//! The buffer maintains lines in contiguous memory, requiring shifts when lines are
//! inserted or deleted in the middle:
//!
//! ### Line Insertion
//! - **At start/middle**: Shifts all subsequent buffer content down to make room
//! - **At end**: No shifting needed, just appends to buffer
//!
//! ### Line Deletion
//! - **At start/middle**: Shifts all subsequent buffer content up to fill the gap
//! - **At end**: No shifting needed, just truncates the buffer
//!
//! This design ensures:
//! - Lines remain contiguous for zero-copy access
//! - Buffer offsets are always valid and in order
//! - No fragmentation or gaps in the buffer
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

use crate::{ByteIndex, ColIndex, ColWidth, GCStringOwned, GCStringRef, Length,
            RowIndex, Seg, SegIndex, SegStringOwned, byte_index,
            gc_string_owned_sizing::SegmentArray, len};

/// Initial size of each line in bytes
pub const INITIAL_LINE_SIZE: usize = 256;

/// Page size for extending lines (bytes added when line overflows)
pub const LINE_PAGE_SIZE: usize = 256;

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
/// let line = buffer.get_line_with_info(row(0)).unwrap();
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
    pub fn content(&self) -> &'a str {
        self.content
    }

    /// Get the line metadata.
    #[must_use]
    pub fn info(&self) -> &'a LineMetadata {
        self.info
    }
}


/// Zero-copy gap buffer data structure for storing editor content
#[derive(Debug, Clone, PartialEq)]
pub struct ZeroCopyGapBuffer {
    /// Contiguous buffer storing all lines
    /// Each line starts at `INITIAL_LINE_SIZE` bytes and can grow
    pub buffer: Vec<u8>,

    /// Metadata for each line (grapheme clusters, display width, etc.)
    lines: Vec<LineMetadata>,

    /// Number of lines currently in the buffer
    line_count: Length,
}

/// Metadata for a single line in the buffer
#[derive(Debug, Clone, PartialEq)]
pub struct LineMetadata {
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
    /// buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hello").unwrap();
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
                && byte_pos.as_usize() < segment.end_byte_index.as_usize()
            {
                return segment.seg_index;
            }
        }

        // If we get here, byte_pos is between segments (shouldn't happen with valid
        // UTF-8) Return the segment after the position
        for segment in &self.segments {
            if byte_pos.as_usize() < segment.start_byte_index.as_usize() {
                return segment.seg_index;
            }
        }

        // Fallback to end of line
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
    /// # let line = buffer.get_line_with_info(row(0)).unwrap();
    /// # let line_info = line.info();
    ///
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(0)).is_none()); // Valid
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(1)).is_none()); // Valid
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(2)).is_none()); // Valid
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(3)).is_some()); // Invalid!
    /// assert!(line_info.check_is_in_middle_of_grapheme(col(4)).is_none()); // Valid
    /// ```
    #[must_use]
    pub fn check_is_in_middle_of_grapheme(&self, col_index: ColIndex) -> Option<Seg> {
        // Find the segment that contains or would contain this column index
        for seg in &self.segments {
            let seg_start = seg.start_display_col_index;
            let seg_end = seg_start + seg.display_width;

            // Check if the column index falls within this segment
            if col_index >= seg_start && col_index < seg_end {
                // If it's not at the start of the segment, it's in the middle
                if col_index != seg_start {
                    return Some(*seg);
                }
                // If it is at the start, this is a valid cursor position
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
    /// let line = buffer.get_line_with_info(row(0)).unwrap();
    /// let seg_string = line.info().get_string_at(line.content(), col(0));
    /// ```
    #[must_use]
    pub fn get_string_at(
        &self,
        content: &str,
        col_index: ColIndex,
    ) -> Option<SegStringOwned> {
        // Find the segment at the given column index
        let target_col = col_index.as_usize();

        for segment in &self.segments {
            let seg_start_col = segment.start_display_col_index.as_usize();
            let seg_width = segment.display_width.as_usize();

            if target_col >= seg_start_col && target_col < seg_start_col + seg_width {
                // Extract the segment's string content
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
        // Find the segment after the given column index
        let target_col = col_index.as_usize();

        for segment in &self.segments {
            let seg_start_col = segment.start_display_col_index.as_usize();

            if seg_start_col > target_col {
                // This is the first segment to the right
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
        // Find the segment before the given column index
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

    /// Create a `GCStringRef` from the line content for compatibility.
    /// This is used when interfacing with non-editor code that expects a `GCString` trait
    /// object.
    ///
    /// # Arguments
    /// * `content` - The line content as a string slice
    ///
    /// # Returns
    /// A `GCStringRef` that borrows the content and metadata without copying.
    ///
    /// # Usage Pattern (for interface boundaries)
    /// ```rust
    /// # use r3bl_tui::{ZeroCopyGapBuffer, row};
    /// # let mut buffer = ZeroCopyGapBuffer::new();
    /// # buffer.add_line();
    /// let line = buffer.get_line_with_info(row(0)).unwrap();
    /// let gc_string_ref = line.info().to_gc_string_ref(line.content());
    /// // let styled_texts = color_wheel.colorize_into_styled_texts(&gc_string_ref, theme);
    /// ```
    #[must_use]
    pub fn to_gc_string_ref<'a>(&'a self, content: &'a str) -> GCStringRef<'a> {
        GCStringRef::from_gap_buffer_line(content, self)
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
        use crate::ch;

        if self.segments.is_empty() || content.is_empty() {
            return "";
        }

        // Find the starting byte index by skipping display columns
        let string_start_byte_index = {
            let mut byte_index = 0;
            let mut skip_col_count = start_col_index;

            for seg in &self.segments {
                let seg_display_width = seg.display_width;

                // If we've skipped enough columns, stop here
                if *skip_col_count == ch(0) {
                    break;
                }

                // Skip this segment's width
                skip_col_count -= seg_display_width;
                byte_index += seg.bytes_size.as_usize();
            }
            byte_index
        };

        // Find the ending byte index by consuming available column width
        let string_end_byte_index = {
            let mut byte_index = 0;
            let mut avail_col_count = max_col_width;
            let mut skip_col_count = start_col_index;

            for seg in &self.segments {
                let seg_display_width = seg.display_width;

                // Are we still skipping columns to reach the start?
                if *skip_col_count == ch(0) {
                    // We're in the content area - check if we have room for this segment
                    if avail_col_count < seg_display_width {
                        // This segment would exceed our width limit
                        break;
                    }
                    byte_index += seg.bytes_size.as_usize();
                    avail_col_count -= seg_display_width;
                } else {
                    // Still skipping to reach start position
                    skip_col_count -= seg_display_width;
                    byte_index += seg.bytes_size.as_usize();
                }
            }
            byte_index
        };

        // Ensure we don't go out of bounds
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
    pub fn get_line_info(&self, line_index: usize) -> Option<&LineMetadata> {
        self.lines.get(line_index)
    }

    /// Get mutable line metadata by index
    pub fn get_line_info_mut(
        &mut self,
        line_index: usize,
    ) -> Option<&mut LineMetadata> {
        self.lines.get_mut(line_index)
    }

    /// Swap two lines in the buffer metadata
    /// This only swaps the [`LineMetadata`] entries, not the actual buffer content
    pub fn swap_lines(&mut self, i: usize, j: usize) { self.lines.swap(i, j); }

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
    pub fn insert_line_with_buffer_shift(&mut self, line_idx: usize) {
        // If inserting at the end, just add a new line
        if line_idx == self.line_count.as_usize() {
            self.add_line();
            return;
        }

        // Calculate where the new line should be inserted in the buffer
        let insert_offset = if line_idx == 0 {
            byte_index(0)
        } else {
            let prev_line = &self.lines[line_idx - 1];
            byte_index(*prev_line.buffer_offset + prev_line.capacity.as_usize())
        };

        // Extend buffer by INITIAL_LINE_SIZE bytes
        let old_buffer_len = self.buffer.len();
        self.buffer
            .resize(old_buffer_len + INITIAL_LINE_SIZE, b'\0');

        // Shift all subsequent buffer content down
        let shift_start = *insert_offset;
        let shift_amount = INITIAL_LINE_SIZE;

        // Move content from back to front to avoid overwriting
        for i in (shift_start..old_buffer_len).rev() {
            self.buffer[i + shift_amount] = self.buffer[i];
        }

        // Clear the newly created space
        for i in shift_start..shift_start + INITIAL_LINE_SIZE {
            self.buffer[i] = b'\0';
        }

        // Add newline character for the empty line
        self.buffer[shift_start] = b'\n';

        // Create line metadata for the new line
        let new_line_info = LineMetadata {
            buffer_offset: insert_offset,
            content_len: len(0),
            capacity: len(INITIAL_LINE_SIZE),
            segments: SegmentArray::new(),
            display_width: crate::width(0),
            grapheme_count: len(0),
        };

        // Insert the new line metadata at the correct position
        self.lines.insert(line_idx, new_line_info);

        // Update buffer offsets for all subsequent lines
        for i in (line_idx + 1)..self.lines.len() {
            self.lines[i].buffer_offset =
                byte_index(*self.lines[i].buffer_offset + shift_amount);
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
        self.lines.push(LineMetadata {
            buffer_offset,
            content_len: len(0),
            capacity: len(INITIAL_LINE_SIZE),
            segments: SegmentArray::new(),
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
    pub fn remove_line(&mut self, line_index: RowIndex) -> bool {
        if line_index.as_usize() >= self.line_count.as_usize() {
            return false;
        }

        let removed_line = &self.lines[line_index.as_usize()];
        let removed_start = *removed_line.buffer_offset;
        let removed_size = removed_line.capacity.as_usize();

        // Remove from metadata
        self.lines.remove(line_index.as_usize());

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
        for line in self.lines.iter_mut().skip(line_index.as_usize()) {
            line.buffer_offset = byte_index(*line.buffer_offset - removed_size);
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
        if line_index.as_usize() >= self.line_count.as_usize() {
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
            .map(|line| line.segments.len() * std::mem::size_of::<crate::Seg>())
            .sum();

        buffer_size + lines_size + line_metadata_size + std::mem::size_of::<Length>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{row, seg_index};

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
        assert_eq!(*line_info.buffer_offset, 0);
        assert_eq!(line_info.capacity, len(INITIAL_LINE_SIZE));
        assert_eq!(line_info.content_len, len(0));

        // Add second line
        let idx2 = buffer.add_line();
        assert_eq!(idx2, 1);
        assert_eq!(buffer.line_count(), len(2));
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
        assert!(buffer.remove_line(row(1)));
        assert_eq!(buffer.line_count(), len(2));

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
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test beginning position
        assert_eq!(line_info.get_byte_pos(seg_index(0)).as_usize(), 0);
    }

    #[test]
    fn test_get_byte_pos_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert some text
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

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
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "H😀llo")
            .unwrap();

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
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test beginning position
        assert_eq!(line_info.get_seg_index(byte_index(0)), seg_index(0));
    }

    #[test]
    fn test_get_seg_index_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert some text
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

        // Insert text with emoji: "H😀llo"
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "H😀llo")
            .unwrap();

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
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "a👨‍👩‍👧‍👦b世界c")
            .unwrap();

        let line_info = buffer.get_line_info(0).unwrap();

        // Test round-trip conversion for each segment
        for i in 0..line_info.segments.len() {
            let seg_idx = seg_index(i);
            let byte_pos = line_info.get_byte_pos(seg_idx);
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
    fn test_insert_line_shifting_behavior() {
        let mut buffer = ZeroCopyGapBuffer::new();

        // Add three lines
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();

        // Record original offsets
        let line0_offset = *buffer.get_line_info(0).unwrap().buffer_offset;
        let line1_offset = *buffer.get_line_info(1).unwrap().buffer_offset;
        let line2_offset = *buffer.get_line_info(2).unwrap().buffer_offset;

        assert_eq!(line0_offset, 0);
        assert_eq!(line1_offset, INITIAL_LINE_SIZE);
        assert_eq!(line2_offset, 2 * INITIAL_LINE_SIZE);

        // Test insertion at beginning (should shift all lines)
        buffer.insert_line_with_buffer_shift(0);

        // Check that all lines were shifted
        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_offset, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_offset,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_offset,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(3).unwrap().buffer_offset,
            3 * INITIAL_LINE_SIZE
        );

        // Test insertion in middle (should shift lines 2 and 3)
        buffer.insert_line_with_buffer_shift(2);

        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_offset, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_offset,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_offset,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(3).unwrap().buffer_offset,
            3 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(4).unwrap().buffer_offset,
            4 * INITIAL_LINE_SIZE
        );

        // Test insertion at end (no shifting)
        let buffer_len_before = buffer.buffer.len();
        buffer.insert_line_with_buffer_shift(5);
        let buffer_len_after = buffer.buffer.len();

        // Only one line was added at the end
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

        // Check that all lines were shifted up
        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_offset, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_offset,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_offset,
            2 * INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(3).unwrap().buffer_offset,
            3 * INITIAL_LINE_SIZE
        );

        // Test deletion in middle (should shift lines 2 and 3 up)
        assert!(buffer.remove_line(row(1)));

        assert_eq!(*buffer.get_line_info(0).unwrap().buffer_offset, 0);
        assert_eq!(
            *buffer.get_line_info(1).unwrap().buffer_offset,
            INITIAL_LINE_SIZE
        );
        assert_eq!(
            *buffer.get_line_info(2).unwrap().buffer_offset,
            2 * INITIAL_LINE_SIZE
        );

        // Test deletion at end (no shifting)
        let last_idx = buffer.line_count.as_usize() - 1;
        let buffer_len_before = buffer.buffer.len();
        assert!(buffer.remove_line(row(last_idx)));
        let buffer_len_after = buffer.buffer.len();

        // Buffer was truncated by one line
        assert_eq!(buffer_len_before - buffer_len_after, INITIAL_LINE_SIZE);
    }

    #[test]
    fn test_gap_buffer_line_info_clip_to_range() {
        use crate::{col, row, seg_index, width};

        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert Unicode-rich content: "Hi📦XelLo🙏🏽Bye"
        // Display layout:
        // H(1) i(1) 📦(2) X(1) e(1) l(1) L(1) o(1) 🙏🏽(2) B(1) y(1) e(1) = 14 total width
        // Columns: 0    1   23     4    5   6   7   8   9A      B    C   D
        // (A=10,B=11,C=12,D=13)
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hi📦XelLo🙏🏽Bye")
            .unwrap();

        let line = buffer.get_line_with_info(row(0)).unwrap();
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
        let empty_line = empty_buffer.get_line_with_info(row(0)).unwrap();
        let result = empty_line.info().clip_to_range(empty_line.content(), col(0), width(5));
        assert_eq!(result, "");
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
            buffer.remove_line(row(5));
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

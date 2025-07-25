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

//! Text deletion operations for [`ZeroCopyGapBuffer`].
//!
//! This module implements grapheme-safe text deletion operations that maintain
//! the null-padding invariant and Unicode correctness. Unlike insertion operations,
//! deletion does not shrink line capacity - it maintains the allocated space for
//! efficient future insertions.
//!
//! # Key Features
//!
//! - **Grapheme-aware**: Respects Unicode grapheme cluster boundaries during deletion
//! - **UTF-8 safe**: Maintains UTF-8 validity throughout all operations
//! - **Capacity preservation**: Does not shrink line capacity after deletion
//! - **Null-padding restoration**: Clears freed space with null bytes
//! - **Efficient shifting**: Minimizes data movement during content removal
//!
//! # Deletion Behavior
//!
//! When text is deleted from a line:
//! 1. Validate deletion boundaries respect grapheme clusters
//! 2. Shift remaining content left to fill the gap
//! 3. Update line metadata (content length, segment info)
//! 4. **Critical**: Fill freed space at end of content with null bytes
//! 5. Preserve allocated capacity for future insertions
//!
//! # Null-Padding Restoration
//!
//! After deletion, the pattern becomes:
//! ```text
//! [remaining content][newline (\n)][null padding (\0)...]
//! ```
//!
//! The null-padding extends to fill the entire allocated capacity, ensuring:
//! - No data leakage from deleted content
//! - Consistent buffer state for zero-copy operations
//! - Safe string slice creation
//!
//! # Operations
//!
//! - [`delete_at_grapheme()`][ZeroCopyGapBuffer::delete_at_grapheme]: Delete single
//!   grapheme cluster
//! - [`delete_range()`][ZeroCopyGapBuffer::delete_range]: Delete range of grapheme
//!   clusters
//! - Internal helpers for byte-level manipulation and cleanup
//!
//! # Null-Padding Invariant
//!
//! **CRITICAL**: All deletion operations in this module MUST maintain the invariant
//! that unused capacity in each line buffer is filled with null bytes (`\0`). This
//! is essential for:
//!
//! - **Security**: Prevents information leakage from previously stored content
//! - **Correctness**: Ensures predictable buffer state for zero-copy operations
//! - **Performance**: Enables safe slice operations without bounds checking
//!
//! When deleting content, this module ensures:
//! 1. Content is shifted left to overwrite deleted portions
//! 2. The freed space at the end is immediately filled with `\0` bytes
//! 3. The newline character is properly repositioned
//! 4. All unused capacity remains null-padded
//!
//! See [`delete_bytes_at_range`][ZeroCopyGapBuffer::delete_bytes_at_range] for the core
//! null-padding logic.
//!
//! # UTF-8 Safety in Deletion Operations
//!
//! This module maintains UTF-8 validity during deletion through **boundary-aware
//! operations**:
//!
//! ## Grapheme-Level Safety
//!
//! - **[`delete_at_grapheme()`][ZeroCopyGapBuffer::delete_at_grapheme]**: Only deletes
//!   complete grapheme clusters, never splits UTF-8 sequences
//! - **[`delete_range()`][ZeroCopyGapBuffer::delete_range]**: Operates on grapheme
//!   boundaries, ensuring no mid-character cuts
//! - **Segment-based indexing**: Uses pre-computed grapheme boundaries from segment
//!   metadata
//!
//! ## Byte-Level Safety
//!
//! The low-level [`delete_bytes_at_range()`][ZeroCopyGapBuffer::delete_bytes_at_range]
//! operates on **pre-validated byte boundaries**:
//! - Callers ensure byte positions align with UTF-8 character boundaries
//! - Content shifting preserves UTF-8 validity by moving complete byte sequences
//! - No new UTF-8 validation needed since we're only removing existing valid content
//!
//! ## Why No UTF-8 Validation During Deletion
//!
//! Deletion operations **don't require UTF-8 validation** because:
//! 1. **We only remove existing valid UTF-8** (can't create invalid sequences)
//! 2. **Grapheme boundaries are pre-computed** (segment metadata ensures valid positions)
//! 3. **Byte shifting preserves encoding** (complete UTF-8 sequences moved intact)
//! 4. **Null padding is valid UTF-8** (`\0` is ASCII, thus valid UTF-8)
//!
//! This allows deletion operations to be **extremely fast** with no validation overhead.

use miette::{Result, miette};

use super::ZeroCopyGapBuffer;
use crate::{ByteIndex, RowIndex, SegIndex, ch, len};

impl ZeroCopyGapBuffer {
    /// Delete a grapheme cluster at the specified position
    ///
    /// This method removes a single grapheme cluster from the specified position,
    /// ensuring that we never split a grapheme cluster. The operation is Unicode-safe
    /// and will rebuild the line's segment information after deletion.
    ///
    /// # Arguments
    /// * `line_index` - The line containing the grapheme to delete
    /// * `seg_index` - The grapheme cluster position to delete
    ///
    /// # Returns
    /// `Ok(())` if successful, `Err` with a diagnostic error if the operation fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - The segment index is out of bounds
    /// - Segment rebuilding fails
    pub fn delete_at_grapheme(
        &mut self,
        line_index: RowIndex,
        seg_index: SegIndex,
    ) -> Result<()> {
        // Validate line index
        let line_info = self.get_line_info(line_index.as_usize()).ok_or_else(|| {
            miette!("Line index {} out of bounds", line_index.as_usize())
        })?;

        // Validate segment index
        if seg_index.as_usize() >= line_info.segments.len() {
            return Err(miette!(
                "Segment index {} out of bounds for line with {} segments",
                seg_index.as_usize(),
                line_info.segments.len()
            ));
        }

        // Get the segment to delete
        let segment = &line_info.segments[seg_index.as_usize()];
        let delete_start = segment.start_byte_index;
        let delete_end = segment.end_byte_index;

        // Perform the actual deletion
        self.delete_bytes_at_range(line_index, delete_start.into(), delete_end.into())?;

        // Rebuild segments for this line
        self.rebuild_line_segments(line_index)?;

        Ok(())
    }

    /// Delete a range of grapheme clusters
    ///
    /// This method removes multiple grapheme clusters from the specified range,
    /// ensuring Unicode safety throughout the operation.
    ///
    /// # Arguments
    /// * `line_index` - The line containing the graphemes to delete
    /// * `start_seg` - The starting grapheme cluster position (inclusive)
    /// * `end_seg` - The ending grapheme cluster position (exclusive)
    ///
    /// # Returns
    /// `Ok(())` if successful, `Err` with a diagnostic error if the operation fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - The segment indices are out of bounds
    /// - The range is invalid (start >= end)
    /// - Segment rebuilding fails
    pub fn delete_range(
        &mut self,
        line_index: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex,
    ) -> Result<()> {
        // Validate range
        if start_seg.as_usize() >= end_seg.as_usize() {
            return Err(miette!(
                "Invalid range: start {} must be less than end {}",
                start_seg.as_usize(),
                end_seg.as_usize()
            ));
        }

        // Validate line index
        let line_info = self.get_line_info(line_index.as_usize()).ok_or_else(|| {
            miette!("Line index {} out of bounds", line_index.as_usize())
        })?;

        // Validate segment indices
        if end_seg.as_usize() > line_info.segments.len() {
            return Err(miette!(
                "End segment index {} out of bounds for line with {} segments",
                end_seg.as_usize(),
                line_info.segments.len()
            ));
        }

        // Get byte range to delete
        let delete_start = if start_seg.as_usize() < line_info.segments.len() {
            line_info.segments[start_seg.as_usize()].start_byte_index
        } else {
            // Start is at end of line
            ch(line_info.content_len.as_usize())
        };

        let delete_end = if end_seg.as_usize() < line_info.segments.len() {
            line_info.segments[end_seg.as_usize()].start_byte_index
        } else {
            // End is at end of line
            ch(line_info.content_len.as_usize())
        };

        // Perform the actual deletion
        self.delete_bytes_at_range(line_index, delete_start.into(), delete_end.into())?;

        // Rebuild segments for this line
        self.rebuild_line_segments(line_index)?;

        Ok(())
    }

    /// Delete bytes within a specified range
    ///
    /// This is a lower-level helper that performs the actual buffer manipulation.
    /// It handles content shifting and null padding restoration.
    ///
    /// # Safety
    /// The caller must ensure that byte positions are at valid UTF-8 boundaries.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - The byte positions exceed the content length
    fn delete_bytes_at_range(
        &mut self,
        line_index: RowIndex,
        start_byte: ByteIndex,
        end_byte: ByteIndex,
    ) -> Result<()> {
        let line_idx = line_index.as_usize();
        let line_info = self
            .get_line_info(line_idx)
            .ok_or_else(|| miette!("Line index {} out of bounds", line_idx))?;

        let start_pos = start_byte.as_usize();
        let end_pos = end_byte.as_usize();
        let current_content_len = line_info.content_len.as_usize();

        // Validate byte positions
        if start_pos > current_content_len || end_pos > current_content_len {
            return Err(miette!(
                "Byte positions {}-{} exceed content length {}",
                start_pos,
                end_pos,
                current_content_len
            ));
        }

        if start_pos >= end_pos {
            // Nothing to delete
            return Ok(());
        }

        let delete_len = end_pos - start_pos;
        let buffer_start = line_info.buffer_offset.as_usize();
        let delete_start = buffer_start + start_pos;
        let delete_end = buffer_start + end_pos;

        // Shift content left to overwrite deleted portion
        if end_pos < current_content_len {
            // Move content after deletion point
            let move_from = delete_end;
            let move_to = delete_start;
            let move_len = current_content_len - end_pos;

            // Move content (including the newline)
            for i in 0..=move_len {
                self.buffer[move_to + i] = self.buffer[move_from + i];
            }
        }

        // New content length after deletion
        let new_content_len = current_content_len - delete_len;

        // Place newline at new end position
        self.buffer[buffer_start + new_content_len] = b'\n';

        // Fill the freed space with null bytes
        let null_start = buffer_start + new_content_len + 1;
        let null_end = buffer_start + current_content_len + 1;
        for i in null_start..null_end {
            self.buffer[i] = b'\0';
        }

        // Update line metadata
        let line_info_mut = self.get_line_info_mut(line_idx).ok_or_else(|| {
            miette!("Line {} not found when updating metadata", line_idx)
        })?;
        line_info_mut.content_len = len(new_content_len);

        Ok(())
    }

    // The [`rebuild_line_segments`][Self::rebuild_line_segments] method is now in
    // segment_construction.rs and is accessible directly on [`ZeroCopyGapBuffer`]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{row, seg_index};

    #[test]
    fn test_delete_at_grapheme() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert initial text
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello World")
            .unwrap();

        // Delete the space (index 5)
        buffer.delete_at_grapheme(row(0), seg_index(5)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "HelloWorld");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, 10);
    }

    #[test]
    fn test_delete_range() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert initial text
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello World!")
            .unwrap();

        // Delete "World" (indices 6-11)
        buffer
            .delete_range(row(0), seg_index(6), seg_index(11))
            .unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "Hello !");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, 7);
    }

    #[test]
    fn test_delete_at_beginning() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Delete first character
        buffer.delete_at_grapheme(row(0), seg_index(0)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "ello");
    }

    #[test]
    fn test_delete_at_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Delete last character
        buffer.delete_at_grapheme(row(0), seg_index(4)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "Hell");
    }

    #[test]
    fn test_delete_unicode() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with emoji
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello 😀 World")
            .unwrap();

        // Delete the emoji (index 6)
        buffer.delete_at_grapheme(row(0), seg_index(6)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "Hello  World");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, 12); // Space still there
    }

    #[test]
    fn test_delete_complex_grapheme() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with compound grapheme cluster
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "👨‍👩‍👧‍👦 Family")
            .unwrap();

        // Delete the family emoji (1 grapheme cluster)
        buffer.delete_at_grapheme(row(0), seg_index(0)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, " Family");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, 7); // Space + 6 letters
    }

    #[test]
    fn test_delete_entire_line() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Delete all characters
        buffer
            .delete_range(row(0), seg_index(0), seg_index(5))
            .unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, 0);
        assert_eq!(line_info.content_len, len(0));
    }

    #[test]
    fn test_delete_invalid_indices() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Try to delete beyond the end
        let result = buffer.delete_at_grapheme(row(0), seg_index(10));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));

        // Try to delete from invalid line
        let result = buffer.delete_at_grapheme(row(5), seg_index(0));
        assert!(result.is_err());

        // Try invalid range (start >= end)
        let result = buffer.delete_range(row(0), seg_index(3), seg_index(3));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid range"));
    }

    #[test]
    fn test_delete_preserves_null_padding() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();
        buffer.delete_at_grapheme(row(0), seg_index(2)).unwrap(); // Delete 'l'

        // Check that the buffer is properly null-padded
        let line_info = buffer.get_line_info(0).unwrap();
        let buffer_start = line_info.buffer_offset.as_usize();
        let content_len = line_info.content_len.as_usize();

        // Content should be "Helo\n"
        assert_eq!(buffer.buffer[buffer_start + content_len], b'\n');

        // Everything after newline should be null
        for i in (buffer_start + content_len + 1)
            ..(buffer_start + line_info.capacity.as_usize())
        {
            assert_eq!(buffer.buffer[i], b'\0', "Byte at {i} should be null");
        }
    }

    #[test]
    fn test_delete_range_with_unicode_boundaries() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with mixed Unicode
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "a😀b🌍c")
            .unwrap();

        // Delete range including emojis (indices 1-4, which is "😀b🌍")
        buffer
            .delete_range(row(0), seg_index(1), seg_index(4))
            .unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "ac");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, 2);
    }
}

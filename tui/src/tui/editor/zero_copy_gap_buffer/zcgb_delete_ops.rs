// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
//! - [`delete_grapheme_at()`]: Delete single grapheme cluster
//! - [`delete_range()`]: Delete range of grapheme clusters
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
//! See [`delete_bytes_at_range`] for the core
//! null-padding logic.
//!
//! # UTF-8 Safety in Deletion Operations
//!
//! This module maintains UTF-8 validity during deletion through **boundary-aware
//! operations**:
//!
//! ## Grapheme-Level Safety
//!
//! - **[`delete_grapheme_at()`]**: Only deletes complete grapheme clusters, never splits
//!   UTF-8 sequences
//! - **[`delete_range()`]**: Operates on grapheme boundaries, ensuring no mid-character
//!   cuts
//! - **Segment-based indexing**: Uses pre-computed grapheme boundaries from segment
//!   metadata
//!
//! ## Byte-Level Safety
//!
//! The low-level [`delete_bytes_at_range()`]
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
//!
//! [`delete_grapheme_at()`]: ZeroCopyGapBuffer::delete_grapheme_at
//! [`delete_range()`]: ZeroCopyGapBuffer::delete_range
//! [`delete_bytes_at_range`]: ZeroCopyGapBuffer::delete_bytes_at_range
//! [`delete_bytes_at_range()`]: ZeroCopyGapBuffer::delete_bytes_at_range

use std::ops::Range;

use miette::{Result, miette};

use super::ZeroCopyGapBuffer;
use crate::{ByteIndex, ByteOffset, IndexMarker, LINE_FEED_BYTE, LengthMarker, NULL_BYTE,
            RangeBoundary, RowIndex, SegIndex, byte_index, len, seg_length};

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
    pub fn delete_grapheme_at(
        &mut self,
        arg_line_index: impl Into<RowIndex>,
        arg_seg_index: impl Into<SegIndex>,
    ) -> Result<()> {
        let line_index: RowIndex = arg_line_index.into();
        let seg_index: SegIndex = arg_seg_index.into();
        // Validate line index.
        let line_info = self.get_line_info(line_index).ok_or_else(|| {
            miette!("Line index {} out of bounds", line_index.as_usize())
        })?;

        // Validate segment index using sophisticated bounds checking.
        let segments_count = seg_length(line_info.grapheme_segments.len());
        if seg_index.overflows(segments_count) {
            return Err(miette!(
                "Segment index {} out of bounds for line with {} segments",
                seg_index.as_usize(),
                line_info.grapheme_segments.len()
            ));
        }

        // Get the segment to delete.
        let segment = &line_info.grapheme_segments[seg_index.as_usize()];
        let delete_start = segment.start_byte_index;
        let delete_end = segment.end_byte_index;

        // Perform the actual deletion.
        self.delete_bytes_at_range(line_index, delete_start, delete_end)?;

        // Rebuild segments for this line.
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
        arg_line_index: impl Into<RowIndex>,
        arg_start_seg: impl Into<SegIndex>,
        arg_end_seg: impl Into<SegIndex>,
    ) -> Result<()> {
        let line_index: RowIndex = arg_line_index.into();
        let start_seg: SegIndex = arg_start_seg.into();
        let end_seg: SegIndex = arg_end_seg.into();
        // Validate line index.
        let line_info = self.get_line_info(line_index).ok_or_else(|| {
            miette!("Line index {} out of bounds", line_index.as_usize())
        })?;

        let delete_range: Range<SegIndex> = start_seg..end_seg;
        let segments_count = seg_length(line_info.grapheme_segments.len());

        if !delete_range.is_valid(segments_count) {
            if start_seg >= end_seg {
                return Ok(()); // Empty range - nothing to delete
            }
            return Err(miette!(
                "Invalid range: start {} must be less than end {} and within segment count {}",
                start_seg.as_usize(),
                end_seg.as_usize(),
                segments_count.as_usize()
            ));
        }

        // Get byte range to delete.
        let delete_start = if start_seg.as_usize() < line_info.grapheme_segments.len() {
            line_info.grapheme_segments[start_seg.as_usize()].start_byte_index
        } else {
            // Start is at end of line.
            byte_index(line_info.content_byte_len.as_usize())
        };

        let delete_end = if end_seg.as_usize() < line_info.grapheme_segments.len() {
            line_info.grapheme_segments[end_seg.as_usize()].start_byte_index
        } else {
            // End is at end of line.
            byte_index(line_info.content_byte_len.as_usize())
        };

        // Perform the actual deletion.
        self.delete_bytes_at_range(line_index, delete_start, delete_end)?;

        // Rebuild segments for this line.
        self.rebuild_line_segments(line_index)?;

        Ok(())
    }

    /// Delete bytes within a specified range using line-relative byte positions
    ///
    /// This is a lower-level helper that performs the actual buffer manipulation.
    /// It handles content shifting and null padding restoration.
    ///
    /// # Parameters
    /// * `arg_line_index` - The line index containing the bytes to delete
    /// * `arg_start_pos` - Line-relative byte position where deletion starts (inclusive)
    /// * `arg_end_pos` - Line-relative byte position where deletion ends (exclusive)
    ///
    /// # Content Shifting Behavior
    ///
    /// - **Deletion at end**: No shifting needed, just restore null padding
    /// - **Deletion at start/middle**: Shifts remaining content left to fill the gap
    ///
    /// After deletion, the freed space is filled with null bytes to maintain
    /// the null-padding invariant.
    ///
    /// # Safety
    /// The caller must ensure that byte positions are at valid UTF-8 boundaries.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - The byte positions exceed the content length
    pub fn delete_bytes_at_range(
        &mut self,
        arg_line_index: impl Into<RowIndex>,
        arg_start_index: impl Into<ByteIndex>,
        arg_end_index: impl Into<ByteIndex>,
    ) -> Result<()> {
        let line_index: RowIndex = arg_line_index.into();
        let start_index: ByteIndex = arg_start_index.into();
        let end_index: ByteIndex = arg_end_index.into();
        // Get line info and validate line index.
        let line_info = self.get_line_info(line_index).ok_or_else(|| {
            miette!("Line index {} out of bounds", line_index.as_usize())
        })?;

        // Validate range using type-safe bounds checking.
        if start_index >= end_index {
            // Range is empty or inverted - nothing to delete.
            return Ok(());
        }

        // Check if start position is within content bounds using type-safe overflow
        // check.
        if start_index.overflows(line_info.content_byte_len) {
            return Err(miette!(
                "Start position {} exceeds content length {}",
                start_index.as_usize(),
                line_info.content_byte_len.as_usize()
            ));
        }

        // Check if end position is within valid range.
        // For exclusive ranges, end can equal content length (e.g., 0..5 for content
        // length 5).
        if end_index.as_usize() > line_info.content_byte_len.as_usize() {
            return Err(miette!(
                "End position {} exceeds content length {}",
                end_index.as_usize(),
                line_info.content_byte_len.as_usize()
            ));
        }

        // Extract values needed for buffer operations before mutable operations.
        let num_deleted_chars = len((end_index - start_index).as_usize());
        let delete_start = line_info.buffer_start + ByteOffset::from(start_index);
        let current_content_len = line_info.content_byte_len;
        let buffer_pos = line_info.buffer_start;

        // Shift content left to overwrite deleted portion.
        if !end_index.overflows(current_content_len) {
            // Content remains after deletion - need to shift.
            let move_from = (buffer_pos + ByteOffset::from(end_index)).as_usize();
            let move_to = delete_start.as_usize();
            let remaining_content = current_content_len.remaining_from(end_index);
            let move_len = remaining_content.as_usize();

            // Move content (including the newline).
            for i in 0..=move_len {
                self.buffer[move_to + i] = self.buffer[move_from + i];
            }
        }

        // Calculate new content length after deletion.
        let new_content_len = current_content_len - num_deleted_chars;

        // Place newline at new end position.
        let newline_pos = buffer_pos.as_usize() + new_content_len.as_usize();
        self.buffer[newline_pos] = LINE_FEED_BYTE;

        // Fill the freed space with null bytes.
        let null_start = newline_pos + 1;
        let null_end = buffer_pos.as_usize() + current_content_len.as_usize() + 1;
        for i in null_start..null_end {
            self.buffer[i] = NULL_BYTE;
        }

        // Update line metadata.
        let line_info_mut = self.get_line_info_mut(line_index).ok_or_else(|| {
            miette!(
                "Line {} not found when updating metadata",
                line_index.as_usize()
            )
        })?;
        line_info_mut.content_byte_len = new_content_len;

        Ok(())
    }

    // The [`rebuild_line_segments`] method is now in
    // implementations::segment_builder and is accessible directly on.
    // [`ZeroCopyGapBuffer`].
    //
    // [`rebuild_line_segments`]: Self::rebuild_line_segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{row, seg_index};

    #[test]
    fn test_delete_at_grapheme() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert initial text.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello World")
            .unwrap();

        // Delete the space (index 5).
        buffer.delete_grapheme_at(row(0), seg_index(5)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "HelloWorld");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, len(10));
    }

    #[test]
    fn test_delete_range() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert initial text.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello World!")
            .unwrap();

        // Delete "World" (indices 6-11).
        buffer
            .delete_range(row(0), seg_index(6), seg_index(11))
            .unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "Hello !");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, len(7));
    }

    #[test]
    fn test_delete_at_beginning() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Delete first character.
        buffer.delete_grapheme_at(row(0), seg_index(0)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "ello");
    }

    #[test]
    fn test_delete_at_end() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Delete last character.
        buffer.delete_grapheme_at(row(0), seg_index(4)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "Hell");
    }

    #[test]
    fn test_delete_unicode() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with emoji.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello 😀 World")
            .unwrap();

        // Delete the emoji (index 6).
        buffer.delete_grapheme_at(row(0), seg_index(6)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "Hello  World");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, len(12)); // Space still there
    }

    #[test]
    fn test_delete_complex_grapheme() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with compound grapheme cluster.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "👨‍👩‍👧‍👦 Family")
            .unwrap();

        // Delete the family emoji (1 grapheme cluster).
        buffer.delete_grapheme_at(row(0), seg_index(0)).unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, " Family");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, len(7)); // Space + 6 letters
    }

    #[test]
    fn test_delete_entire_line() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Delete all characters.
        buffer
            .delete_range(row(0), seg_index(0), seg_index(5))
            .unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, len(0));
        assert_eq!(line_info.content_byte_len, len(0));
    }

    #[test]
    fn test_delete_invalid_indices() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();

        // Try to delete beyond the end.
        let result = buffer.delete_grapheme_at(row(0), seg_index(10));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));

        // Try to delete from invalid line.
        let result = buffer.delete_grapheme_at(row(5), seg_index(0));
        assert!(result.is_err());

        // Try empty range (start == end) - should succeed as no-op.
        let result = buffer.delete_range(row(0), seg_index(3), seg_index(3));
        assert!(result.is_ok()); // Empty range is valid and does nothing

        // Try truly invalid range (start > end).
        let result = buffer.delete_range(row(0), seg_index(4), seg_index(3));
        assert!(result.is_ok()); // This is also treated as empty range and does nothing
    }

    #[test]
    fn test_delete_preserves_null_padding() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello")
            .unwrap();
        buffer.delete_grapheme_at(row(0), seg_index(2)).unwrap(); // Delete 'l'

        // Check that the buffer is properly null-padded.
        let line_info = buffer.get_line_info(0).unwrap();
        let buffer_start = line_info.buffer_start.as_usize();
        let content_len = line_info.content_byte_len.as_usize();

        // Content should be "Helo\n".
        assert_eq!(buffer.buffer[buffer_start + content_len], LINE_FEED_BYTE);

        // Everything after newline should be null.
        for i in (buffer_start + content_len + 1)
            ..(buffer_start + line_info.capacity.as_usize())
        {
            assert_eq!(buffer.buffer[i], NULL_BYTE, "Byte at {i} should be null");
        }
    }

    #[test]
    fn test_delete_range_with_unicode_boundaries() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with mixed Unicode.
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "a😀b🌍c")
            .unwrap();

        // Delete range including emojis (indices 1-4, which is "😀b🌍").
        buffer
            .delete_range(row(0), seg_index(1), seg_index(4))
            .unwrap();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "ac");

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.grapheme_count, len(2));
    }
}

#[cfg(test)]
mod benches {
    use std::hint::black_box;

    use test::Bencher;

    use super::*;
    use crate::{row, seg_index};

    extern crate test;

    #[bench]
    fn bench_delete_single_grapheme(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let test_text = "Hello World";

        b.iter(|| {
            // Setup: insert text.
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), test_text)
                .unwrap();

            // Benchmark: delete a character.
            buffer
                .delete_grapheme_at(row(0), black_box(seg_index(5)))
                .unwrap();

            // Cleanup: clear rest of content.
            let count = buffer.get_line_info(0).unwrap().grapheme_count;
            buffer
                .delete_range(row(0), seg_index(0), seg_index(count.as_usize()))
                .unwrap();
        });
    }

    #[bench]
    fn bench_delete_range_small(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let test_text = "Hello Beautiful World";

        b.iter(|| {
            // Setup
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), test_text)
                .unwrap();

            // Benchmark: delete "Beautiful " (indices 6-16).
            buffer
                .delete_range(row(0), black_box(seg_index(6)), black_box(seg_index(16)))
                .unwrap();

            // Cleanup
            let count = buffer.get_line_info(0).unwrap().grapheme_count;
            buffer
                .delete_range(row(0), seg_index(0), seg_index(count.as_usize()))
                .unwrap();
        });
    }

    #[bench]
    fn bench_delete_unicode_grapheme(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let test_text = "Hello 😀 World";

        b.iter(|| {
            // Setup
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), test_text)
                .unwrap();

            // Benchmark: delete the emoji.
            buffer
                .delete_grapheme_at(row(0), black_box(seg_index(6)))
                .unwrap();

            // Cleanup
            let count = buffer.get_line_info(0).unwrap().grapheme_count;
            buffer
                .delete_range(row(0), seg_index(0), seg_index(count.as_usize()))
                .unwrap();
        });
    }

    #[bench]
    fn bench_delete_from_beginning(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let test_text = "Hello World";

        b.iter(|| {
            // Setup
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), test_text)
                .unwrap();

            // Benchmark: delete first 5 chars.
            buffer
                .delete_range(row(0), black_box(seg_index(0)), black_box(seg_index(5)))
                .unwrap();

            // Cleanup
            let count = buffer.get_line_info(0).unwrap().grapheme_count;
            buffer
                .delete_range(row(0), seg_index(0), seg_index(count.as_usize()))
                .unwrap();
        });
    }

    #[bench]
    fn bench_delete_entire_line_content(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let test_text = "This is a test line with some content";

        b.iter(|| {
            // Setup
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), test_text)
                .unwrap();
            let count = buffer.get_line_info(0).unwrap().grapheme_count;

            // Benchmark: delete all content.
            buffer
                .delete_range(
                    row(0),
                    black_box(seg_index(0)),
                    black_box(seg_index(count.as_usize())),
                )
                .unwrap();
        });
    }

    #[bench]
    fn bench_delete_complex_grapheme_cluster(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let test_text = "Family: 👨‍👩‍👧‍👦 is here";

        b.iter(|| {
            // Setup
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), test_text)
                .unwrap();

            // Benchmark: delete the complex family emoji.
            buffer
                .delete_grapheme_at(row(0), black_box(seg_index(8)))
                .unwrap();

            // Cleanup
            let count = buffer.get_line_info(0).unwrap().grapheme_count;
            buffer
                .delete_range(row(0), seg_index(0), seg_index(count.as_usize()))
                .unwrap();
        });
    }
}

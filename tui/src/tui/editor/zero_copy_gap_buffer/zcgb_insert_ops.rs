// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Text insertion operations for [`ZeroCopyGapBuffer`].
//!
//! This module implements grapheme-safe text insertion operations that maintain
//! the null-padding invariant and Unicode correctness while handling dynamic
//! line growth automatically.
//!
//! # Key Features
//!
//! - **Grapheme-aware**: Respects Unicode grapheme cluster boundaries
//! - **UTF-8 safe**: Maintains UTF-8 validity throughout all operations
//! - **Dynamic growth**: Automatically extends line capacity when needed
//! - **Null-padding**: Preserves null-padding in unused capacity
//! - **Zero-copy ready**: Maintains buffer state for efficient parsing
//! - **Optimized appends**: Special fast path for end-of-line insertions
//!
//! # Growth Behavior
//!
//! When text insertion would exceed current line capacity:
//! 1. Calculate required capacity (content + newline + padding)
//! 2. Extend line by one or more 256-byte pages (`LINE_PAGE_SIZE`)
//! 3. Shift subsequent lines in buffer to make room
//! 4. Initialize new capacity with null bytes
//! 5. Perform the insertion maintaining UTF-8 boundaries
//!
//! # Operations
//!
//! - [`insert_text_at_grapheme()`]: Insert text at a specific grapheme position with
//!   automatic optimization detection
//! - [`insert_empty_line()`]: Create new empty lines with proper initialization
//! - Internal helpers for byte-level manipulation and capacity management
//!
//! # Null-Padding Invariant
//!
//! **CRITICAL**: All insertion operations in this module MUST maintain the invariant
//! that unused capacity in each line buffer is filled with null bytes (`\0`). This
//! is essential for:
//!
//! - **Security**: Prevents information leakage from uninitialized memory
//! - **Correctness**: Ensures predictable buffer state for zero-copy operations
//! - **Performance**: Enables safe slice operations without bounds checking
//!
//! When inserting content, this module ensures:
//! 1. Content is shifted right to make room for new text
//! 2. The gap created by shifting is cleared with `\0` bytes
//! 3. After insertion, any remaining unused capacity is null-padded
//! 4. When extending line capacity, new memory is initialized with `\0`
//!
//! The null-padding logic is especially critical in
//! [`insert_text_at_byte_ofs`] where
//! content shifting and capacity extension occur.
//!
//! # UTF-8 Safety in Insertion Operations
//!
//! This module implements the **input validation boundary** of our UTF-8 safety
//! architecture:
//!
//! ## API-Level Validation
//!
//! UTF-8 safety is **guaranteed by Rust's type system** at the API boundary:
//!
//! - **[`insert_text_at_grapheme(text:
//!   &str)`][ZeroCopyGapBuffer::insert_text_at_grapheme]**: The `&str` parameter ensures
//!   valid UTF-8
//! - **Type system enforcement**: Impossible to pass invalid UTF-8 through safe Rust APIs
//! - **No runtime validation needed**: UTF-8 validity guaranteed by caller's type
//!   constraints
//!
//! ## Grapheme-Safe Positioning
//!
//! Insertion respects **Unicode grapheme cluster boundaries**:
//! - Uses pre-computed segment metadata to find valid insertion points
//! - Never splits multi-byte UTF-8 sequences or grapheme clusters
//! - Maintains proper Unicode text editing semantics
//!
//! ## Buffer Integrity During Insertion
//!
//! Content insertion preserves UTF-8 validity through:
//! 1. **Boundary-aware shifting**: Moves complete UTF-8 sequences during content shifting
//! 2. **Null-padding maintenance**: Fills unused capacity with valid UTF-8 null bytes
//! 3. **Capacity extension**: New memory immediately initialized with valid UTF-8 (`\0`)
//!
//! ## Why This is the Validation Point
//!
//! This module is where **all UTF-8 validation occurs** because:
//! - **Single entry point**: All content enters the buffer through these methods
//! - **Type-safe APIs**: `&str` parameters guarantee UTF-8 validity
//! - **Performance optimization**: Validate once at input, trust thereafter
//! - **Zero-copy enabling**: Subsequent operations can use `unsafe` for performance
//!
//! After insertion, all other modules can safely use [`from_utf8_unchecked()`] because
//! they operate on content that was validated at this boundary.
//!
//! [`insert_text_at_grapheme()`]: ZeroCopyGapBuffer::insert_text_at_grapheme
//! [`insert_empty_line()`]: ZeroCopyGapBuffer::insert_empty_line
//! [`insert_text_at_byte_ofs`]: ZeroCopyGapBuffer::insert_text_at_byte_ofs

use miette::{Result, miette};

use super::{LINE_PAGE_SIZE, ZeroCopyGapBuffer};
use crate::{BoundsCheck, ByteIndex, ByteOffset, CursorPositionBoundsStatus, IndexMarker,
            LINE_FEED_BYTE, LengthMarker, NULL_BYTE, RowIndex, SegIndex, UnitCompare,
            len};

impl ZeroCopyGapBuffer {
    /// Insert text at a specific grapheme position within a line.
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
    /// `Ok(())` if successful, `Err` with a diagnostic error if the operation fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - Text insertion fails due to capacity or encoding issues
    /// - Segment rebuilding fails
    pub fn insert_text_at_grapheme(
        &mut self,
        arg_line_index: impl Into<RowIndex>,
        arg_seg_index: impl Into<SegIndex>,
        text: &str,
    ) -> Result<()> {
        let line_index: RowIndex = arg_line_index.into();
        let seg_index: SegIndex = arg_seg_index.into();
        // Validate line index and get the byte position.
        let byte_pos = {
            let line_info = self.get_line_info(line_index).ok_or_else(|| {
                miette!("Line index {} out of bounds", line_index.as_usize())
            })?;
            line_info.get_byte_index(seg_index)
        };

        // Perform the actual insertion.
        self.insert_text_at_byte_pos(line_index, byte_pos, text)?;

        // Rebuild line segments after insertion.
        self.rebuild_line_segments(line_index)?;

        Ok(())
    }

    /// Insert text at a specific byte position within a line.
    ///
    /// This is a lower-level helper that performs the actual buffer manipulation.
    /// It handles capacity checking, content shifting, and buffer extension if needed.
    ///
    /// # Content Shifting Behavior
    ///
    /// - **Insertion at end**: Just appends text and moves the newline character
    /// - **Insertion at start/middle**: Shifts existing content to the right to make room
    ///
    /// The method ensures null padding is maintained after the newline character.
    ///
    /// # Safety
    /// The caller must ensure that `byte_pos` is at a valid UTF-8 boundary.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The line index is out of bounds
    /// - The byte position exceeds the content length
    pub fn insert_text_at_byte_pos(
        &mut self,
        arg_line_index: impl Into<RowIndex>,
        arg_byte_index: impl Into<ByteIndex>,
        text: &str,
    ) -> Result<()> {
        let line_index: RowIndex = arg_line_index.into();
        let byte_index: ByteIndex = arg_byte_index.into();
        let text_bytes = text.as_bytes();
        let text_len = len(text_bytes.len());

        // Get line info and validate position.
        let line_info = self.get_line_info(line_index).ok_or_else(|| {
            miette!("Line index {} out of bounds", line_index.as_usize())
        })?;

        // Validate byte position using cursor position bounds checking.
        // For insertion, we allow inserting at position equal to content length (at the
        // end of the line).
        if byte_index.check_cursor_position_bounds(line_info.content_len)
            == CursorPositionBoundsStatus::Beyond
        {
            return Err(miette!(
                "Byte position {} exceeds content length {}",
                byte_index.as_usize(),
                line_info.content_len.as_usize()
            ));
        }

        // Check if we need to extend the line capacity.
        let current_content_len = line_info.content_len;
        let new_content_len = current_content_len + text_len;
        let required_capacity = new_content_len + len(1); // +1 for newline
        if required_capacity > line_info.capacity {
            // Extend the line capacity.
            self.extend_line_capacity(line_index);

            // Re-check after extension (might need multiple extensions for large text)
            let line_info = self.get_line_info(line_index).ok_or_else(|| {
                miette!("Line {} disappeared after extension", line_index.as_usize())
            })?;
            if required_capacity > line_info.capacity {
                // Calculate how many pages we need using type-safe remaining capacity.
                let additional_capacity_needed = required_capacity - line_info.capacity;
                let pages_needed = additional_capacity_needed
                    .as_usize()
                    .div_ceil(LINE_PAGE_SIZE);
                for _ in 0..pages_needed {
                    self.extend_line_capacity(line_index);
                }
            }
        }

        // Get updated line info after potential capacity extension.
        let line_info = self.get_line_info(line_index).ok_or_else(|| {
            miette!(
                "Line {} not found after capacity extension",
                line_index.as_usize()
            )
        })?;
        let line_content_len = line_info.content_len; // Keep for type-safe operations
        let insert_pos =
            (line_info.buffer_start_byte_index + ByteOffset::from(byte_index)).as_usize();

        // Shift existing content to make room.
        if byte_index.overflows(line_content_len) {
            // Inserting at end, just move the newline.
            self.buffer[insert_pos + text_len.as_usize()] = LINE_FEED_BYTE;
        } else {
            // Need to move content to the right.
            let move_from = insert_pos;
            let move_to = insert_pos + text_len.as_usize();
            let move_len = line_content_len.remaining_from(byte_index).as_usize();

            // Move content (including the newline).
            for i in (0..=move_len).rev() {
                self.buffer[move_to + i] = self.buffer[move_from + i];
            }

            // Clear the gap left behind by the move.
            for i in move_from..move_from + text_len.as_usize() {
                self.buffer[i] = NULL_BYTE;
            }
        }

        // Copy the new text into the buffer.
        self.buffer[insert_pos..insert_pos + text_len.as_usize()]
            .copy_from_slice(text_bytes);

        // Update line metadata.
        let line_info_mut = self.get_line_info_mut(line_index).ok_or_else(|| {
            miette!(
                "Line {} not found when updating metadata",
                line_index.as_usize()
            )
        })?;
        let new_content_len = current_content_len + text_len;
        line_info_mut.content_len = new_content_len;

        // Ensure remainder of line capacity is null-padded.
        let line_end_length = new_content_len + len(1); // +1 for newline
        let remaining_capacity = line_info_mut.capacity - line_end_length;

        // null-pad if there's remaining capacity.
        if !remaining_capacity.is_zero() {
            let buffer_start = line_info_mut.buffer_start_byte_index.as_usize();
            let line_end = buffer_start + line_end_length.as_usize();
            let capacity_end = buffer_start + line_info_mut.capacity.as_usize();

            // Fill unused capacity with null bytes
            for i in line_end..capacity_end {
                self.buffer[i] = NULL_BYTE;
            }
        }

        Ok(())
    }

    /// Insert a new empty line at the specified position.
    ///
    /// This method properly maintains the invariant that lines are ordered by their
    /// buffer offsets by using the existing `insert_line_with_buffer_shift` method.
    ///
    /// # Errors
    /// Returns an error if the line index exceeds the current line count.
    pub fn insert_empty_line(
        &mut self,
        arg_line_index: impl Into<RowIndex>,
    ) -> Result<()> {
        let line_index: RowIndex = arg_line_index.into();
        // Use cursor position bounds checking for insertion operations.
        // This allows insertion at index == line_count (append at end).
        if line_index.check_cursor_position_bounds(self.line_count())
            == CursorPositionBoundsStatus::Beyond
        {
            return Err(miette!(
                "Cannot insert line at index {}, only {} lines exist",
                line_index.as_usize(),
                self.line_count().as_usize()
            ));
        }

        // Use the internal method that properly shifts buffer content.
        self.insert_line_with_buffer_shift(line_index);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{row, seg_index};

    #[test]
    fn test_insert_at_beginning() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text at the beginning.
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hello")?;

        // Verify the content.
        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello");

        // Verify segments were rebuilt.
        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, len(5));
        assert_eq!(line_info.content_len, len(5));

        Ok(())
    }

    #[test]
    fn test_insert_at_end() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // First insert some text.
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hello")?;

        // Then insert at the end.
        buffer.insert_text_at_grapheme(row(0), seg_index(5), " World")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello World");

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, len(11));

        Ok(())
    }

    #[test]
    fn test_insert_in_middle() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert initial text.
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "Heo")?;

        // Insert in the middle.
        buffer.insert_text_at_grapheme(row(0), seg_index(2), "ll")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello");

        Ok(())
    }

    #[test]
    fn test_insert_unicode() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert emoji
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hello 😀")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "Hello 😀");

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, len(7)); // "Hello " = 6 + emoji = 1

        // Insert more text after emoji.
        buffer.insert_text_at_grapheme(row(0), seg_index(7), " World")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content after second insert"))?;
        assert_eq!(content, "Hello 😀 World");

        Ok(())
    }

    #[test]
    fn test_insert_causes_line_extension() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Create a string that will require line extension.
        let long_text = "A".repeat(300);

        buffer.insert_text_at_grapheme(row(0), seg_index(0), &long_text)?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, &long_text);

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        assert_eq!(line_info.grapheme_count, len(300));
        assert!(line_info.capacity.as_usize() >= 301); // 300 + newline

        Ok(())
    }

    #[test]
    fn test_insert_empty_line() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer.add_line();

        // Add content to lines.
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "Line 1")?;
        buffer.insert_text_at_grapheme(row(1), seg_index(0), "Line 2")?;

        // Insert empty line in middle.
        buffer.insert_empty_line(row(1))?;

        assert_eq!(buffer.line_count(), len(3));

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
        let mut buffer = ZeroCopyGapBuffer::new();

        let result = buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hello");
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
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text with compound grapheme clusters.
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "👨‍👩‍👧‍👦 Family")?;

        let content = buffer
            .get_line_content(row(0))
            .ok_or_else(|| miette!("Failed to get line content"))?;
        assert_eq!(content, "👨‍👩‍👧‍👦 Family");

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        // The family emoji is 1 grapheme cluster despite being multiple code points.
        assert_eq!(line_info.grapheme_count, len(8)); // 1 + space + 6 letters

        Ok(())
    }

    #[test]
    fn test_null_padding_maintained_after_insertion() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert some text.
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "Hello")?;

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        let buffer_start = *line_info.buffer_start_byte_index;
        let content_len = line_info.content_len.as_usize();
        let capacity = line_info.capacity.as_usize();

        // Verify content and newline.
        assert_eq!(
            &buffer.buffer[buffer_start..buffer_start + content_len],
            b"Hello"
        );
        assert_eq!(buffer.buffer[buffer_start + content_len], LINE_FEED_BYTE);

        // Verify unused capacity is null-padded.
        let unused_start = buffer_start + content_len + 1; // after content + newline
        for i in unused_start..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], NULL_BYTE,
                "Unused buffer position {} should be null-padded after insertion but found: {:?}",
                i, buffer.buffer[i]
            );
        }

        Ok(())
    }

    #[test]
    fn test_null_padding_after_middle_insertion() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert initial text.
        buffer.insert_text_at_grapheme(row(0), seg_index(0), "Heo")?;

        // Insert in the middle (this will shift existing content)
        buffer.insert_text_at_grapheme(row(0), seg_index(2), "ll")?;

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        let buffer_start = *line_info.buffer_start_byte_index;
        let content_len = line_info.content_len.as_usize();
        let capacity = line_info.capacity.as_usize();

        // Verify final content.
        assert_eq!(
            &buffer.buffer[buffer_start..buffer_start + content_len],
            b"Hello"
        );
        assert_eq!(buffer.buffer[buffer_start + content_len], LINE_FEED_BYTE);

        // Verify unused capacity is still null-padded after content shifting.
        let unused_start = buffer_start + content_len + 1;
        for i in unused_start..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], NULL_BYTE,
                "Unused buffer position {} should be null-padded after middle insertion but found: {:?}",
                i, buffer.buffer[i]
            );
        }

        Ok(())
    }

    #[test]
    fn test_null_padding_after_line_extension() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Create text that will cause line extension.
        let long_text = "A".repeat(300);
        buffer.insert_text_at_grapheme(row(0), seg_index(0), &long_text)?;

        let line_info = buffer
            .get_line_info(0)
            .ok_or_else(|| miette!("Failed to get line info"))?;
        let buffer_start = *line_info.buffer_start_byte_index;
        let content_len = line_info.content_len.as_usize();
        let capacity = line_info.capacity.as_usize();

        // Verify the line was extended.
        assert!(capacity > crate::INITIAL_LINE_SIZE);

        // Verify content and newline.
        assert_eq!(
            &buffer.buffer[buffer_start..buffer_start + content_len],
            long_text.as_bytes()
        );
        assert_eq!(buffer.buffer[buffer_start + content_len], LINE_FEED_BYTE);

        // Verify unused capacity in extended line is null-padded.
        let unused_start = buffer_start + content_len + 1;
        for i in unused_start..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], NULL_BYTE,
                "Extended unused buffer position {} should be null-padded but found: {:?}",
                i, buffer.buffer[i]
            );
        }

        Ok(())
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
    fn bench_insert_small_text(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        b.iter(|| {
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), black_box("Hello"))
                .unwrap();
            // Clear content for next iteration.
            buffer
                .delete_range(row(0), seg_index(0), seg_index(5))
                .unwrap();
        });
    }

    #[bench]
    fn bench_insert_at_end(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Initial text")
            .unwrap();

        b.iter(|| {
            let end_count = buffer.get_line_info(0).unwrap().grapheme_count;
            buffer
                .insert_text_at_grapheme(
                    row(0),
                    seg_index(end_count.as_usize()),
                    black_box(" more"),
                )
                .unwrap();
            // Clear added content.
            buffer
                .delete_range(
                    row(0),
                    seg_index(end_count.as_usize()),
                    seg_index(end_count.as_usize() + 5),
                )
                .unwrap();
        });
    }

    #[bench]
    fn bench_insert_unicode(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        b.iter(|| {
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), black_box("Hello 😀 世界"))
                .unwrap();
            // Clear content
            let count = buffer.get_line_info(0).unwrap().grapheme_count;
            buffer
                .delete_range(row(0), seg_index(0), seg_index(count.as_usize()))
                .unwrap();
        });
    }

    #[bench]
    fn bench_insert_causes_extension(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        let long_text = "A".repeat(300);

        b.iter(|| {
            buffer
                .insert_text_at_grapheme(row(0), seg_index(0), black_box(&long_text))
                .unwrap();
            // Clear content
            buffer
                .delete_range(row(0), seg_index(0), seg_index(300))
                .unwrap();
        });
    }

    #[bench]
    fn bench_insert_empty_line(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();

        b.iter(|| {
            buffer.insert_empty_line(row(0)).unwrap();
            // Remove for next iteration.
            buffer.remove_line(row(0));
        });
    }

    #[bench]
    fn bench_insert_middle_of_text(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer
            .insert_text_at_grapheme(row(0), seg_index(0), "Hello World")
            .unwrap();

        b.iter(|| {
            // Insert in middle (after "Hello ")
            buffer
                .insert_text_at_grapheme(row(0), seg_index(6), black_box("Beautiful "))
                .unwrap();
            // Remove inserted text.
            buffer
                .delete_range(row(0), seg_index(6), seg_index(16))
                .unwrap();
        });
    }

    #[bench]
    fn bench_insert_at_end_with_optimization(b: &mut Bencher) {
        // This tests the real-world scenario with our optimization.
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Start with a realistic line.
        buffer
            .insert_text_at_grapheme(
                row(0),
                seg_index(0),
                "This is a typical line of text",
            )
            .unwrap();

        b.iter(|| {
            let end_count = buffer.get_line_info(0).unwrap().grapheme_count;

            // Append a single character (most common case when typing)
            buffer
                .insert_text_at_grapheme(
                    row(0),
                    seg_index(end_count.as_usize()),
                    black_box("x"),
                )
                .unwrap();

            // Delete it to reset for next iteration.
            buffer
                .delete_grapheme_at(row(0), seg_index(end_count.as_usize()))
                .unwrap();
        });
    }
}

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

//! Segment construction operations for [`ZeroCopyGapBuffer`].
//!
//! This module provides centralized segment rebuilding functionality that maintains
//! grapheme cluster metadata for lines after text modifications. It ensures:
//!
//! - **Content boundary correctness**: Only processes content up to `content_len`
//! - **UTF-8 safety**: Validates and handles UTF-8 errors appropriately
//! - **Metadata accuracy**: Updates segments, display width, and grapheme count
//! - **Performance**: Supports both single-line and batch rebuilding
//!
//! # Content Boundary Invariant
//!
//! When rebuilding segments, we MUST only read content up to the `content_len`
//! boundary. The buffer beyond this point contains null padding (`\0`) which
//! should never be included in segment calculations.
//!
//! # Usage
//!
//! This module is used internally by text insertion and deletion operations
//! to update line metadata after content changes:
//!
//! ```rust,ignore
//! // After inserting text
//! buffer.rebuild_line_segments(line_index)?;
//!
//! // For bulk operations
//! buffer.rebuild_line_segments_batch(&[0, 1, 2])?;
//! ```
//!
//! # UTF-8 Safety in Segment Construction
//!
//! This module implements **post-modification metadata rebuilding** within our UTF-8
//! safety architecture:
//!
//! ## When Segment Rebuilding Occurs
//!
//! Segment rebuilding is called **after content modifications**:
//! - After [`insert_at_grapheme()`][ZeroCopyGapBuffer::insert_at_grapheme] - content
//!   already validated at insertion
//! - After [`delete_at_grapheme()`][ZeroCopyGapBuffer::delete_at_grapheme] - removing
//!   valid UTF-8 can't create invalid sequences
//! - After bulk operations - operating on previously validated content
//!
//! ## Why `unsafe { `[`from_utf8_unchecked()`]` }` is Safe Here
//!
//! This module can safely use `unsafe` operations because:
//!
//! 1. **Controlled input**: All public insert and mutate operations use `&str`, ensuring
//!    only valid UTF-8 content is added to the buffer
//! 2. **Metadata reconstruction**: Only reading existing buffer content for analysis
//! 3. **Content boundary respect**: Only processes content up to `content_len` (validated
//!    region)
//! 4. **Bounds checking**: All buffer access is bounds checked before creating slices
//! 5. **Performance critical**: Called after every edit operation, needs maximum speed
//! 6. **Test coverage**: Comprehensive tests verify UTF-8 handling, including
//!    intentionally invalid UTF-8 scenarios that panic in debug mode
//!
//! Why unsafe is used instead of [`from_utf8_lossy()`]:
//! - [`from_utf8_lossy()`] returns `Cow<str>`, which may allocate a new `String` if
//!   invalid UTF-8 is encountered, breaking our zero-copy guarantee
//! - [`from_utf8_unchecked()`] returns `&str` directly without allocation, preserving
//!   zero-copy semantics essential for performance-critical operations
//!
//! ## Performance Justification
//!
//! Segment rebuilding is **extremely performance-sensitive**:
//! - Called after **every character typed** by the user
//! - Called after **every deletion operation** (backspace, delete key)
//! - Called during **bulk operations** (paste, find/replace, formatting)
//!
//! UTF-8 validation here would add significant overhead to basic text editing operations.
//!
//! ## Debug-Mode Safety Net
//!
//! In debug builds, we **validate UTF-8 before unsafe operations**:
//! - Catches any invariant violations during development
//! - Provides clear panic messages with line and byte position info
//! - Helps identify if content modifications broke UTF-8 boundaries
//!
//! ## Architectural Role
//!
//! This module sits in the **"trust zone"** of our UTF-8 architecture:
//! - **Input modules**
//!   ([`text_insertion`][crate::tui::editor::zero_copy_gap_buffer::text_insertion])
//!   validate UTF-8 at boundaries
//! - **This module** trusts validated content and optimizes for performance
//! - **Access modules**
//!   ([`zero_copy_access`][crate::tui::editor::zero_copy_gap_buffer::zero_copy_access])
//!   provide zero-copy string access
//!
//! The safety depends on the **architectural contract** that content entering
//! the buffer is UTF-8 validated, making subsequent operations safe.

use std::str::{from_utf8, from_utf8_unchecked};

use miette::{Result, miette};

use super::buffer_storage::ZeroCopyGapBuffer;
use crate::{RowIndex, SegIndex, len,
            segment_builder::{build_segments_for_str, calculate_display_width}};

impl ZeroCopyGapBuffer {
    /// Rebuild grapheme cluster segments for a single line.
    ///
    /// This method reconstructs the segment metadata for a line after its
    /// content has been modified. It:
    ///
    /// 1. Extracts line content up to `content_len` (excluding null padding)
    /// 2. Validates UTF-8 encoding
    /// 3. Builds new segments using the segment builder
    /// 4. Updates all metadata fields in [`crate::GapBufferLineInfo`]
    ///
    /// # Use Cases
    ///
    /// This method is called after modifying a single line's content:
    /// - **After text insertion** - When user types characters or pastes within a line
    /// - **After text deletion** - When user deletes characters (backspace, delete key)
    /// - **After in-line operations** - Like find/replace within a single line
    ///
    /// Currently used by:
    /// - [`insert_at_grapheme()`][ZeroCopyGapBuffer::insert_at_grapheme] - after
    ///   inserting text
    /// - [`delete_at_grapheme()`][ZeroCopyGapBuffer::delete_at_grapheme] - after deleting
    ///   text
    /// - [`delete_range()`][ZeroCopyGapBuffer::delete_range] - after deleting multiple
    ///   graphemes
    ///
    /// # Arguments
    ///
    /// * `line_index` - The index of the line to rebuild
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The line index is out of bounds
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the line contains invalid UTF-8. This should
    /// not happen in normal operation as the buffer maintains UTF-8 invariants.
    ///
    /// # Content Boundary
    ///
    /// This method carefully respects the `content_len` boundary to ensure
    /// null padding is never included in segment calculations.
    pub fn rebuild_line_segments(&mut self, line_index: RowIndex) -> Result<()> {
        let line_idx = line_index.as_usize();

        // Get line info for content extraction
        let line_info = self
            .get_line_info(line_idx)
            .ok_or_else(|| miette!("Line index {} out of bounds", line_idx))?;

        // Extract content slice up to content_len boundary
        // This ensures we don't read into null padding
        let content_slice = &self.buffer[line_info.content_range()];

        // Convert to string (UTF-8 invariants maintained by buffer)
        let content_str = {
            #[cfg(debug_assertions)]
            {
                if let Err(e) = from_utf8(content_slice) {
                    panic!(
                        "Line {} contains invalid UTF-8 at byte {}: {}",
                        line_idx,
                        e.valid_up_to(),
                        e
                    );
                }
            }

            // SAFETY: We maintain UTF-8 invariants via all buffer insertions using &str
            unsafe { from_utf8_unchecked(content_slice) }
        };

        // Build new segments using the segment builder
        let segments = build_segments_for_str(content_str);

        // Calculate metadata from segments
        let display_width = calculate_display_width(&segments);
        let grapheme_count = len(segments.len());

        // Update line info with new metadata
        let line_info = self.get_line_info_mut(line_idx).ok_or_else(|| {
            miette!("Line {} not found when updating segments", line_idx)
        })?;

        line_info.segments = segments;
        line_info.display_width = display_width;
        line_info.grapheme_count = grapheme_count;

        Ok(())
    }

    /// Rebuild grapheme cluster segments for multiple lines.
    ///
    /// This method efficiently rebuilds segments for a batch of lines,
    /// useful for bulk operations that modify multiple lines at once.
    ///
    /// # Use Cases
    ///
    /// This method is designed for bulk operations where multiple lines are modified:
    /// - **File Loading** - Building segments for all lines when loading a file
    /// - **Multi-line Paste** - When pasting content that spans multiple lines
    /// - **Block Operations**:
    ///   - Block indent/outdent (adding/removing spaces from multiple lines)
    ///   - Block comment/uncomment operations
    ///   - Multi-cursor edits affecting multiple lines
    /// - **Find/Replace All** - When replacing text across multiple lines
    /// - **Code Formatting** - When auto-formatting affects multiple lines
    /// - **Undo/Redo** - When undoing/redoing operations that affected multiple lines
    ///
    /// # Performance Note
    ///
    /// This is more efficient than calling
    /// [`rebuild_line_segments()`][Self::rebuild_line_segments] multiple times
    /// as it avoids repeated function call overhead for bulk operations.
    ///
    /// # Arguments
    ///
    /// * `line_indices` - Slice of line indices to rebuild
    ///
    /// # Errors
    ///
    /// Returns an error if any line fails to rebuild. The error will
    /// indicate which line failed and why.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if any line contains invalid UTF-8. This should
    /// not happen in normal operation as the buffer maintains UTF-8 invariants.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Rebuild segments for lines 0, 5, and 10
    /// buffer.rebuild_line_segments_batch(&[row(0), row(5), row(10)])?;
    /// ```
    pub fn rebuild_line_segments_batch(
        &mut self,
        line_indices: &[RowIndex],
    ) -> Result<()> {
        for &line_index in line_indices {
            self.rebuild_line_segments(line_index).map_err(|e| {
                miette!("Failed to rebuild line {}: {}", line_index.as_usize(), e)
            })?;
        }
        Ok(())
    }

    /// Optimized segment rebuilding for end-of-line append operations.
    ///
    /// This method provides a fast path for the common case of appending text at the
    /// end of a line (like normal typing). Instead of rebuilding all segments from
    /// scratch, it:
    ///
    /// 1. Parses only the newly appended text
    /// 2. Adjusts the new segments' offsets based on existing content
    /// 3. Appends them to the existing segment array
    ///
    /// # When This Optimization Applies
    ///
    /// This optimization is used when:
    /// - Text is inserted at the end of the line (`seg_index` >= segment count)
    /// - The line had content before the append (not an empty line)
    /// - We have the appended text available
    ///
    /// # Performance Benefits
    ///
    /// For typical typing scenarios:
    /// - Typing "Hello" one char at a time: ~5x faster (parse 1 char vs 5 chars)
    /// - Appending to a 100-char line: ~100x faster (parse 1 char vs 100 chars)
    ///
    /// # Arguments
    ///
    /// * `line_index` - The index of the line that was modified
    /// * `append_position` - The segment index where text was appended
    /// * `appended_text` - The text that was appended to the line
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the optimization was applied, `Ok(false)` if it wasn't applicable
    /// and a full rebuild is needed, or an error if something went wrong.
    ///
    /// # Errors
    ///
    /// Returns an error if the line index is out of bounds.
    pub fn rebuild_line_segments_append_optimized(
        &mut self,
        line_index: RowIndex,
        append_position: SegIndex,
        appended_text: &str,
    ) -> Result<bool> {
        let line_idx = line_index.as_usize();
        
        // Get line info and extract needed values
        let (existing_byte_len, existing_grapheme_count, existing_display_width) = {
            let line_info = self
                .get_line_info(line_idx)
                .ok_or_else(|| miette!("Line index {} out of bounds", line_idx))?;
            
            // Check if this is actually an append at the end
            if append_position.as_usize() < line_info.segments.len() {
                // Not an append at end, need full rebuild
                return Ok(false);
            }
            
            // If the line was empty, we need a full rebuild to establish initial state
            if line_info.segments.is_empty() {
                return Ok(false);
            }
            
            // Get the offset where the new text was appended
            let append_offset = line_info.content_len.as_usize() - appended_text.len();
            
            // Extract values we need before mutable borrow
            (append_offset, line_info.grapheme_count, line_info.display_width)
        };
        
        // Build segments only for the appended text
        let mut new_segments = build_segments_for_str(appended_text);
        
        // Adjust all the offsets in the new segments
        for seg in &mut new_segments {
            seg.start_byte_index = crate::ch(seg.start_byte_index.as_usize() + existing_byte_len);
            seg.end_byte_index = crate::ch(seg.end_byte_index.as_usize() + existing_byte_len);
            seg.seg_index = crate::seg_index(seg.seg_index.as_usize() + existing_grapheme_count.as_usize());
            seg.start_display_col_index = crate::col(
                seg.start_display_col_index.as_usize() + existing_display_width.as_usize()
            );
        }
        
        // Calculate the new display width for the appended text
        let new_display_width = calculate_display_width(&new_segments);
        let new_segment_count = new_segments.len();
        
        // Get mutable line info and append the new segments
        let line_info = self.get_line_info_mut(line_idx).ok_or_else(|| {
            miette!("Line {} not found when updating segments", line_idx)
        })?;
        
        // Append new segments to existing ones
        line_info.segments.extend(new_segments);
        
        // Update totals
        line_info.display_width = crate::width(
            line_info.display_width.as_usize() + new_display_width.as_usize()
        );
        line_info.grapheme_count += len(new_segment_count);
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{row, seg_index, width};

    #[test]
    fn test_rebuild_line_segments_empty_line() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Rebuild segments for empty line
        buffer.rebuild_line_segments(row(0))?;

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.segments.len(), 0);
        assert_eq!(line_info.grapheme_count, len(0));
        assert_eq!(line_info.display_width, width(0));

        Ok(())
    }

    #[test]
    fn test_rebuild_line_segments_ascii() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert ASCII text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello")?;

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.segments.len(), 5);
        assert_eq!(line_info.grapheme_count, len(5));
        assert_eq!(line_info.display_width, width(5));

        Ok(())
    }

    #[test]
    fn test_rebuild_line_segments_unicode() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert Unicode text with emoji
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hi 👋 😀")?;

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.segments.len(), 6); // "H" "i" " " "👋" " " "😀"
        assert_eq!(line_info.grapheme_count, len(6));
        assert_eq!(line_info.display_width, width(8)); // 1+1+1+2+1+2

        Ok(())
    }

    #[test]
    fn test_rebuild_line_segments_batch() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();

        // Create multiple lines
        for i in 0..3 {
            buffer.add_line();
            let text = format!("Line {i}");
            buffer.insert_at_grapheme(row(i), seg_index(0), &text)?;
        }

        // Rebuild all lines at once
        buffer.rebuild_line_segments_batch(&[row(0), row(1), row(2)])?;

        // Verify all lines were rebuilt correctly
        for i in 0..3 {
            let line_info = buffer.get_line_info(i).unwrap();
            assert_eq!(line_info.segments.len(), 6); // "Line X" = 6 chars
            assert_eq!(line_info.grapheme_count, len(6));
            assert_eq!(line_info.display_width, width(6));
        }

        Ok(())
    }

    #[test]
    fn test_content_boundary_correctness() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert text that's shorter than line capacity
        buffer.insert_at_grapheme(row(0), seg_index(0), "Test")?;

        let line_info = buffer.get_line_info(0).unwrap();
        let buffer_start = line_info.buffer_offset.as_usize();
        let capacity = line_info.capacity.as_usize();

        // Verify null padding exists beyond content
        for i in (buffer_start + 5)..(buffer_start + capacity) {
            assert_eq!(
                buffer.buffer[i], b'\0',
                "Expected null padding at position {i}"
            );
        }

        // Rebuild segments - should only process "Test", not null padding
        buffer.rebuild_line_segments(row(0))?;

        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.segments.len(), 4); // Only "Test"
        assert_eq!(line_info.grapheme_count, len(4));
        assert_eq!(line_info.display_width, width(4));

        Ok(())
    }

    #[test]
    fn test_rebuild_invalid_line_index() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Try to rebuild non-existent line
        let result = buffer.rebuild_line_segments(row(1));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));
    }

    #[test]
    fn test_rebuild_after_delete() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert and then delete some text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello")?;
        buffer.delete_at_grapheme(row(0), seg_index(1))?; // Delete 'e'

        // Segments should be rebuilt automatically by delete, but let's verify
        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.segments.len(), 4); // "Hllo" (after deleting 'e')
        assert_eq!(line_info.grapheme_count, len(4));
        assert_eq!(line_info.display_width, width(4));

        Ok(())
    }

    #[test]
    fn test_append_optimization() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Insert initial text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello")?;
        let _initial_segments = buffer.get_line_info(0).unwrap().segments.clone();
        
        // First, actually append the text to simulate real usage
        buffer.insert_at_grapheme(row(0), seg_index(5), " World")?;
        
        // Now test the optimization method directly with the same append
        // Reset to just "Hello" first
        buffer.clear();
        buffer.add_line();
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello")?;
        
        // Manually update content_len to simulate the text has been appended
        {
            let line_info = buffer.get_line_info_mut(0).unwrap();
            line_info.content_len = crate::len(11); // "Hello World" length
        }
        
        // Now test the optimization
        let optimized = buffer.rebuild_line_segments_append_optimized(
            row(0), 
            seg_index(5), 
            " World"
        )?;
        
        assert!(optimized);
        
        // Verify the segments are correct
        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.segments.len(), 11); // "Hello World"
        assert_eq!(line_info.grapheme_count, len(11));
        
        Ok(())
    }

    #[test]
    fn test_append_optimization_not_applicable() -> Result<()> {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Test on empty line - should not optimize
        let optimized = buffer.rebuild_line_segments_append_optimized(
            row(0), 
            seg_index(0), 
            "Hello"
        )?;
        
        assert!(!optimized); // Should return false

        // Insert some text
        buffer.insert_at_grapheme(row(0), seg_index(0), "Hello")?;
        
        // Test inserting in middle - should not optimize
        let optimized = buffer.rebuild_line_segments_append_optimized(
            row(0), 
            seg_index(2), 
            "XX"
        )?;
        
        assert!(!optimized); // Should return false
        
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
    fn bench_rebuild_single_line_ascii(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer
            .insert_at_grapheme(
                row(0),
                seg_index(0),
                "Hello, World! This is a test string.",
            )
            .unwrap();

        b.iter(|| {
            buffer.rebuild_line_segments(black_box(row(0))).unwrap();
        });
    }

    #[bench]
    fn bench_rebuild_single_line_unicode(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello 👋 World 🌍 Test 🚀")
            .unwrap();

        b.iter(|| {
            buffer.rebuild_line_segments(black_box(row(0))).unwrap();
        });
    }

    #[bench]
    fn bench_rebuild_batch_10_lines(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        let indices: Vec<RowIndex> = (0..10)
            .map(|i| {
                buffer.add_line();
                buffer
                    .insert_at_grapheme(
                        row(i),
                        seg_index(0),
                        &format!("Line {i} content"),
                    )
                    .unwrap();
                row(i)
            })
            .collect();

        b.iter(|| {
            buffer
                .rebuild_line_segments_batch(black_box(&indices))
                .unwrap();
        });
    }

    #[bench]
    fn bench_append_optimization_single_char(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Start with a line that has some content
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello World! This is a test line")
            .unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        let end_pos = seg_index(line_info.grapheme_count.as_usize());
        
        b.iter(|| {
            buffer
                .rebuild_line_segments_append_optimized(
                    black_box(row(0)),
                    black_box(end_pos),
                    black_box("x")
                )
                .unwrap();
        });
    }

    #[bench]
    fn bench_full_rebuild_after_append_single_char(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Start with the same line content
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello World! This is a test linex")
            .unwrap();
        
        b.iter(|| {
            buffer.rebuild_line_segments(black_box(row(0))).unwrap();
        });
    }

    #[bench]
    fn bench_append_optimization_word(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Start with a longer line
        buffer
            .insert_at_grapheme(row(0), seg_index(0), 
                "This is a much longer line with more content to test the optimization benefits")
            .unwrap();
        
        let line_info = buffer.get_line_info(0).unwrap();
        let end_pos = seg_index(line_info.grapheme_count.as_usize());
        
        b.iter(|| {
            buffer
                .rebuild_line_segments_append_optimized(
                    black_box(row(0)),
                    black_box(end_pos),
                    black_box(" word")
                )
                .unwrap();
        });
    }

    #[bench]
    fn bench_full_rebuild_after_append_word(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        
        // Same content plus the word
        buffer
            .insert_at_grapheme(row(0), seg_index(0), 
                "This is a much longer line with more content to test the optimization benefits word")
            .unwrap();
        
        b.iter(|| {
            buffer.rebuild_line_segments(black_box(row(0))).unwrap();
        });
    }
}

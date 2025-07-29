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

//! Zero-copy access methods for [`ZeroCopyGapBuffer`].
//!
//! This module provides methods to access the buffer contents as `&str` or `&[u8]`
//! without any copying or allocation. This is crucial for performance when passing
//! content to the markdown parser.
//!
//! # Null-Padding Invariant
//!
//! **CRITICAL**: This module relies on the null-padding invariant maintained by
//! other [`ZeroCopyGapBuffer`] modules. All unused capacity in each line MUST be filled
//! with null bytes (`\0`). This invariant enables:
//!
//! - **Safe zero-copy access**: We can create string slices without worrying about
//!   uninitialized memory
//! - **Predictable parsing**: The markdown parser can safely process content knowing that
//!   unused capacity contains only null bytes
//! - **Debug-mode validation**: We can verify UTF-8 validity without encountering random
//!   uninitialized bytes
//!
//! The null-padding invariant is essential for the safety of our `unsafe` operations:
//! - [`from_utf8_unchecked`] calls assume the buffer contains valid UTF-8
//! - The null bytes in unused capacity are valid UTF-8 (ASCII 0)
//! - This prevents undefined behavior when creating string slices
//!
//! All access methods in this module depend on this invariant being maintained by
//! the storage, insertion, and deletion operations.
//!
//! # UTF-8 Safety in Zero-Copy Access
//!
//! This module implements the **performance-optimized read path** of our UTF-8 safety
//! architecture:
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
//! ## Performance Critical Path
//!
//! Zero-copy access is **performance-critical** for:
//! - **Markdown parsing**: Requires `&str` without allocation for large documents
//! - **Text rendering**: Frequent line content access during display updates
//! - **Search operations**: String pattern matching across buffer content
//! - **Export operations**: Writing buffer contents to files or clipboard
//!
//! Using [`from_utf8_lossy()`] would break zero-copy guarantees by potentially allocating
//! `String` instances, defeating the purpose of this buffer design.
//!
//! ## Debug-Mode Validation
//!
//! In debug builds, we **validate UTF-8 before unsafe operations**:
//! - Catches any invariant violations during development
//! - Provides clear panic messages for debugging
//! - Zero overhead in production builds (assertions compiled out)
//!
//! ## Zero-Copy Guarantee
//!
//! This module maintains **true zero-copy access**:
//! - Returns `&str` slices directly into buffer memory
//! - No allocations, no copying, no UTF-8 validation overhead
//! - Enables efficient integration with parsers and external libraries
//!
//! The safety of this approach depends on the **architectural contract** that
//! UTF-8 validation occurs once at input boundaries, making subsequent unsafe
//! operations safe and performant.

use std::{ops::Range,
          str::{from_utf8, from_utf8_unchecked}};

use super::buffer_storage::ZeroCopyGapBuffer;
use crate::{ByteIndex, RowIndex, row};

impl ZeroCopyGapBuffer {
    /// Get the entire buffer as a string slice
    ///
    /// This method provides zero-copy access to the buffer contents as a `&str`. It uses
    /// `unsafe` code with [`from_utf8_unchecked()`] instead of
    /// [`String::from_utf8_lossy()`] to preserve the zero-copy guarantee that is
    /// crucial for performance when passing content to the markdown parser.
    ///
    /// # Panics
    /// Panics if the buffer contains invalid UTF-8 (should not happen in normal
    /// operation)
    #[must_use]
    pub fn as_str(&self) -> &str {
        // In debug builds, validate UTF-8
        #[cfg(debug_assertions)]
        {
            if let Err(e) = from_utf8(&self.buffer) {
                panic!(
                    "ZeroCopyGapBuffer contains invalid UTF-8 at byte {}: {}",
                    e.valid_up_to(),
                    e
                );
            }
        }

        // SAFETY: We maintain UTF-8 invariants via all buffer insertions using &str
        unsafe { from_utf8_unchecked(&self.buffer) }
    }

    /// Get the entire buffer as a byte slice
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] { &self.buffer }

    /// Get the content of a single line as a string slice
    ///
    /// This method provides zero-copy access to individual line content. Like
    /// [`Self::as_str()`], it uses `unsafe` code to preserve zero-copy semantics
    /// instead of [`String::from_utf8_lossy()`].
    ///
    /// The same safety guarantees apply: UTF-8 invariants are maintained by the buffer,
    /// debug builds validate UTF-8, and bounds are checked before slice creation.
    ///
    /// Returns None if the line index is out of bounds
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the line contains invalid UTF-8
    #[must_use]
    pub fn get_line_content(&self, line_index: RowIndex) -> Option<&str> {
        let line_info = self.get_line_info(line_index.as_usize())?;

        // In debug builds, validate UTF-8
        #[cfg(debug_assertions)]
        {
            if let Err(e) = from_utf8(&self.buffer[line_info.content_range()]) {
                panic!(
                    "Line {} contains invalid UTF-8 at byte {}: {}",
                    line_index.as_usize(),
                    e.valid_up_to(),
                    e
                );
            }
        }
        // SAFETY: We maintain UTF-8 invariants via all buffer insertions using &str
        Some(unsafe { from_utf8_unchecked(&self.buffer[line_info.content_range()]) })
    }

    /// Get a slice of lines as a string
    ///
    /// This method provides zero-copy access to multiple consecutive lines. Like other
    /// string access methods, it uses `unsafe` code to maintain zero-copy semantics
    /// with the same safety guarantees as `as_str()`.
    ///
    /// Returns None if the range is out of bounds
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the line slice contains invalid UTF-8
    #[must_use]
    pub fn get_line_slice(&self, line_range: Range<RowIndex>) -> Option<&str> {
        // Check bounds
        if line_range.start.as_usize() >= self.line_count().as_usize()
            || line_range.end.as_usize() > self.line_count().as_usize()
        {
            return None;
        }

        if line_range.is_empty() {
            return Some("");
        }

        // Calculate actual start offset from line info
        let start_info = self.get_line_info(line_range.start.as_usize())?;
        let start_offset = *start_info.buffer_offset;

        // Calculate end offset
        let end_offset = if line_range.end.as_usize() < self.line_count().as_usize() {
            let end_info = self.get_line_info(line_range.end.as_usize())?;
            *end_info.buffer_offset
        } else {
            self.buffer.len()
        };

        // In debug builds, validate UTF-8
        #[cfg(debug_assertions)]
        {
            if let Err(e) = std::str::from_utf8(&self.buffer[start_offset..end_offset]) {
                panic!(
                    "Line slice {:?} contains invalid UTF-8 at byte {}: {}",
                    line_range,
                    e.valid_up_to(),
                    e
                );
            }
        }

        // SAFETY: We maintain UTF-8 invariants via all buffer insertions using &str
        Some(unsafe { from_utf8_unchecked(&self.buffer[start_offset..end_offset]) })
    }

    /// Get the raw content of a line including null padding
    ///
    /// This is useful for debugging and testing
    #[must_use]
    pub fn get_line_raw(&self, line_index: RowIndex) -> Option<&[u8]> {
        let line_info = self.get_line_info(line_index.as_usize())?;
        let start = *line_info.buffer_offset;
        let end = start + line_info.capacity.as_usize();
        Some(&self.buffer[start..end])
    }

    /// Check if the buffer contains valid UTF-8
    ///
    /// This should always return true in normal operation
    #[must_use]
    pub fn is_valid_utf8(&self) -> bool { std::str::from_utf8(&self.buffer).is_ok() }

    /// Get the content of a line including its newline character (if present)
    ///
    /// This is useful for parsers that need to distinguish between lines with
    /// and without trailing newlines.
    ///
    /// Returns None if the line index is out of bounds
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the line with newline contains invalid UTF-8
    #[must_use]
    pub fn get_line_with_newline(&self, line_index: RowIndex) -> Option<&str> {
        let line_info = self.get_line_info(line_index.as_usize())?;
        let content_range = line_info.content_range();
        // Include the newline if there's content
        let end = if line_info.content_len.as_usize() > 0 {
            content_range.end + 1 // +1 for newline
        } else {
            content_range.start + 1 // Just the newline for empty lines
        };

        // Ensure we don't go past the line boundary
        let end = end.min(content_range.start + line_info.capacity.as_usize());
        let range = content_range.start..end;

        // In debug builds, validate UTF-8
        #[cfg(debug_assertions)]
        {
            if let Err(e) = std::str::from_utf8(&self.buffer[range.clone()]) {
                panic!(
                    "Line {} with newline contains invalid UTF-8 at byte {}: {}",
                    line_index.as_usize(),
                    e.valid_up_to(),
                    e
                );
            }
        }

        // SAFETY: We maintain UTF-8 invariants via all buffer insertions using &str
        Some(unsafe { from_utf8_unchecked(&self.buffer[range]) })
    }

    /// Find which line contains the given byte offset in the full buffer
    ///
    /// This is useful for error reporting when you have a byte position from
    /// the parser and need to know which line it corresponds to.
    ///
    /// Returns None if the byte offset is out of bounds
    #[must_use]
    pub fn find_line_containing_byte(&self, byte_offset: ByteIndex) -> Option<RowIndex> {
        if byte_offset.as_usize() >= self.buffer.len() {
            return None;
        }

        // Find the line by searching through line info
        for line_index in 0..self.line_count().as_usize() {
            let line_info = self.get_line_info(line_index)?;
            let line_start = *line_info.buffer_offset;
            let line_end = line_start + line_info.capacity.as_usize();
            if *byte_offset >= line_start && *byte_offset < line_end {
                return Some(row(line_index));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byte_index, tui::editor::zero_copy_gap_buffer::INITIAL_LINE_SIZE};

    #[test]
    fn test_as_str_empty() {
        let buffer = ZeroCopyGapBuffer::new();
        assert_eq!(buffer.as_str(), "");
    }

    #[test]
    fn test_as_str_with_lines() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer.add_line();

        let content = buffer.as_str();
        assert_eq!(content.len(), 2 * INITIAL_LINE_SIZE);
        assert!(content.starts_with('\n'));
    }

    #[test]
    fn test_as_bytes() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        let bytes = buffer.as_bytes();
        assert_eq!(bytes.len(), INITIAL_LINE_SIZE);
        assert_eq!(bytes[0], b'\n');
        assert!(bytes[1..].iter().all(|&b| b == b'\0'));
    }

    #[test]
    fn test_get_line_content_empty() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        let content = buffer.get_line_content(row(0)).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_get_line_content_out_of_bounds() {
        let buffer = ZeroCopyGapBuffer::new();
        assert!(buffer.get_line_content(row(0)).is_none());
        assert!(buffer.get_line_content(row(10)).is_none());
    }

    #[test]
    fn test_get_line_slice() {
        let mut buffer = ZeroCopyGapBuffer::new();
        for _ in 0..5 {
            buffer.add_line();
        }

        // Get middle lines
        let slice = buffer.get_line_slice(row(1)..row(4)).unwrap();
        assert_eq!(slice.len(), 3 * INITIAL_LINE_SIZE);

        // Get all lines
        let slice = buffer.get_line_slice(row(0)..row(5)).unwrap();
        assert_eq!(slice.len(), 5 * INITIAL_LINE_SIZE);

        // Empty range
        let slice = buffer.get_line_slice(row(2)..row(2)).unwrap();
        assert_eq!(slice, "");
    }

    #[test]
    fn test_get_line_slice_out_of_bounds() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer.add_line();

        assert!(buffer.get_line_slice(row(0)..row(3)).is_none()); // end > line_count
        assert!(buffer.get_line_slice(row(3)..row(4)).is_none()); // start >= line_count
    }

    #[test]
    fn test_get_line_raw() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        let raw = buffer.get_line_raw(row(0)).unwrap();
        assert_eq!(raw.len(), INITIAL_LINE_SIZE);
        assert_eq!(raw[0], b'\n');
        assert!(raw[1..].iter().all(|&b| b == b'\0'));
    }

    #[test]
    fn test_is_valid_utf8() {
        let buffer = ZeroCopyGapBuffer::new();
        assert!(buffer.is_valid_utf8());

        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        assert!(buffer.is_valid_utf8());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "ZeroCopyGapBuffer contains invalid UTF-8")]
    fn test_invalid_utf8_panic() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Get the line info first
        let offset = {
            let line_info = buffer.get_line_info(0).unwrap();
            *line_info.buffer_offset
        };

        // SAFETY: We're intentionally creating invalid UTF-8 for testing
        // This is only done in tests to verify our panic behavior
        // Insert invalid UTF-8 sequence (0xFF is never valid in UTF-8)
        buffer.buffer[offset] = 0xFF;
        buffer.buffer[offset + 1] = 0xFF;

        // Update line info
        if let Some(line_info) = buffer.get_line_info_mut(0) {
            line_info.content_len = crate::len(2);
        }

        // This should panic in debug mode
        let _ = buffer.as_str();
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Line 0 contains invalid UTF-8")]
    fn test_get_line_content_invalid_utf8() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Get the line info first
        let offset = *buffer.get_line_info(0).unwrap().buffer_offset;

        // SAFETY: We're intentionally creating invalid UTF-8 for testing
        // Insert invalid UTF-8 sequence
        // 0xC0 0x80 is an overlong encoding (invalid UTF-8)
        buffer.buffer[offset] = 0xC0;
        buffer.buffer[offset + 1] = 0x80;

        // Update line info
        if let Some(line_info) = buffer.get_line_info_mut(0) {
            line_info.content_len = crate::len(2);
        }

        // This should panic in debug mode
        let _ = buffer.get_line_content(row(0));
    }

    #[test]
    fn test_get_line_with_newline() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();

        // Empty line should just have newline
        let content = buffer.get_line_with_newline(row(0)).unwrap();
        assert_eq!(content, "\n");

        // Out of bounds
        assert!(buffer.get_line_with_newline(row(1)).is_none());
    }

    #[test]
    fn test_find_line_containing_byte() {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();

        // First line
        assert_eq!(
            buffer.find_line_containing_byte(byte_index(0)),
            Some(row(0))
        );
        assert_eq!(
            buffer.find_line_containing_byte(byte_index(INITIAL_LINE_SIZE - 1)),
            Some(row(0))
        );

        // Second line
        assert_eq!(
            buffer.find_line_containing_byte(byte_index(INITIAL_LINE_SIZE)),
            Some(row(1))
        );
        assert_eq!(
            buffer.find_line_containing_byte(byte_index(INITIAL_LINE_SIZE + 100)),
            Some(row(1))
        );

        // Third line
        assert_eq!(
            buffer.find_line_containing_byte(byte_index(2 * INITIAL_LINE_SIZE)),
            Some(row(2))
        );

        // Out of bounds
        assert!(
            buffer
                .find_line_containing_byte(byte_index(3 * INITIAL_LINE_SIZE))
                .is_none()
        );
    }
}

#[cfg(test)]
mod benches {
    use std::hint::black_box;

    use test::Bencher;

    use super::*;
    use crate::{byte_index, seg_index};

    extern crate test;

    #[bench]
    fn bench_as_str_small_buffer(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        for _ in 0..10 {
            buffer.add_line();
        }

        b.iter(|| {
            let s = buffer.as_str();
            black_box(s.len());
        });
    }

    #[bench]
    fn bench_as_str_large_buffer(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        for i in 0..100 {
            buffer.add_line();
            buffer
                .insert_at_grapheme(
                    row(i),
                    seg_index(0),
                    "This is a test line with some content",
                )
                .unwrap();
        }

        b.iter(|| {
            let s = buffer.as_str();
            black_box(s.len());
        });
    }

    #[bench]
    fn bench_get_line_content(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Hello World")
            .unwrap();

        b.iter(|| {
            let content = buffer.get_line_content(black_box(row(0))).unwrap();
            black_box(content.len());
        });
    }

    #[bench]
    fn bench_get_line_slice_10_lines(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        for i in 0..20 {
            buffer.add_line();
            buffer
                .insert_at_grapheme(row(i), seg_index(0), &format!("Line {i}"))
                .unwrap();
        }

        b.iter(|| {
            let slice = buffer
                .get_line_slice(black_box(row(5))..black_box(row(15)))
                .unwrap();
            black_box(slice.len());
        });
    }

    #[bench]
    fn bench_get_line_with_newline(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        buffer.add_line();
        buffer
            .insert_at_grapheme(row(0), seg_index(0), "Test line")
            .unwrap();

        b.iter(|| {
            let content = buffer.get_line_with_newline(black_box(row(0))).unwrap();
            black_box(content.len());
        });
    }

    #[bench]
    fn bench_find_line_containing_byte(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        for _ in 0..100 {
            buffer.add_line();
        }

        b.iter(|| {
            let line = buffer.find_line_containing_byte(black_box(byte_index(1000)));
            black_box(line);
        });
    }

    #[bench]
    fn bench_is_valid_utf8(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        for i in 0..50 {
            buffer.add_line();
            buffer
                .insert_at_grapheme(row(i), seg_index(0), "Hello 😀 World")
                .unwrap();
        }

        b.iter(|| {
            let valid = buffer.is_valid_utf8();
            black_box(valid);
        });
    }

    #[bench]
    fn bench_as_bytes(b: &mut Bencher) {
        let mut buffer = ZeroCopyGapBuffer::new();
        for _ in 0..10 {
            buffer.add_line();
        }

        b.iter(|| {
            let bytes = buffer.as_bytes();
            black_box(bytes.len());
        });
    }
}

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

//! Line buffer implementation for efficient text editing.
//!
//! This module provides a gap buffer-like data structure where each line is stored
//! as a fixed-size byte array padded with null characters. This enables zero-copy
//! access as `&str` for the markdown parser while maintaining efficient Unicode support.
//!
//! # Key Features
//!
//! - Fixed-size line buffers (256 bytes by default)
//! - Null-padded storage for efficient in-place editing
//! - Zero-copy access for parsing operations
//! - Unicode-safe text manipulation using grapheme clusters
//! - Metadata caching for performance

use crate::{ColWidth, gc_string_sizing::SegmentArray};

/// Size of each line in bytes
pub const LINE_SIZE: usize = 256;

/// Line buffer data structure for storing editor content
#[derive(Debug, Clone)]
pub struct LineBuffer {
    /// Contiguous buffer storing all lines
    /// Each line is exactly LINE_SIZE bytes
    buffer: Vec<u8>,

    /// Metadata for each line (grapheme clusters, display width, etc.)
    lines: Vec<LineInfo>,

    /// Number of lines currently in the buffer
    line_count: usize,

    /// Size of each line in bytes
    line_size: usize,
}

/// Metadata for a single line in the buffer
#[derive(Debug, Clone)]
pub struct LineInfo {
    /// Where this line starts in the buffer
    pub buffer_offset: usize,

    /// Actual content length in bytes (before '\n')
    pub content_len: usize,

    /// GCString's segment array for this line
    pub segments: SegmentArray,

    /// Display width of the line
    pub display_width: ColWidth,

    /// Number of grapheme clusters
    pub grapheme_count: usize,
}

impl LineBuffer {
    /// Create a new empty LineBuffer
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            lines: Vec::new(),
            line_count: 0,
            line_size: LINE_SIZE,
        }
    }

    /// Create a new LineBuffer with pre-allocated capacity
    #[must_use]
    pub fn with_capacity(line_capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(line_capacity * LINE_SIZE),
            lines: Vec::with_capacity(line_capacity),
            line_count: 0,
            line_size: LINE_SIZE,
        }
    }

    /// Get the number of lines in the buffer
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_count
    }

    /// Get the size of each line in bytes
    #[must_use]
    pub fn line_size(&self) -> usize {
        self.line_size
    }

    /// Get line metadata by index
    #[must_use]
    pub fn get_line_info(&self, line_index: usize) -> Option<&LineInfo> {
        self.lines.get(line_index)
    }

    /// Get mutable line metadata by index
    pub fn get_line_info_mut(&mut self, line_index: usize) -> Option<&mut LineInfo> {
        self.lines.get_mut(line_index)
    }
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LineBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LineBuffer {{ lines: {}, buffer_size: {} bytes, line_size: {} }}",
            self.line_count,
            self.buffer.len(),
            self.line_size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_line_buffer() {
        let buffer = LineBuffer::new();
        assert_eq!(buffer.line_count(), 0);
        assert_eq!(buffer.line_size(), LINE_SIZE);
        assert!(buffer.buffer.is_empty());
        assert!(buffer.lines.is_empty());
    }

    #[test]
    fn test_with_capacity() {
        let capacity = 10;
        let buffer = LineBuffer::with_capacity(capacity);
        assert_eq!(buffer.line_count(), 0);
        assert_eq!(buffer.line_size(), LINE_SIZE);
        assert_eq!(buffer.buffer.capacity(), capacity * LINE_SIZE);
        assert_eq!(buffer.lines.capacity(), capacity);
    }

    #[test]
    fn test_display_trait() {
        let buffer = LineBuffer::new();
        let display = format!("{}", buffer);
        assert_eq!(display, "LineBuffer { lines: 0, buffer_size: 0 bytes, line_size: 256 }");
    }
}
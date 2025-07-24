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
    pub fn line_count(&self) -> usize { self.line_count }

    /// Get the size of each line in bytes
    #[must_use]
    pub fn line_size(&self) -> usize { self.line_size }

    /// Get line metadata by index
    #[must_use]
    pub fn get_line_info(&self, line_index: usize) -> Option<&LineInfo> {
        self.lines.get(line_index)
    }

    /// Get mutable line metadata by index
    pub fn get_line_info_mut(&mut self, line_index: usize) -> Option<&mut LineInfo> {
        self.lines.get_mut(line_index)
    }

    /// Add a new line to the buffer
    /// Returns the index of the newly added line
    pub fn add_line(&mut self) -> usize {
        let line_index = self.line_count;
        let buffer_offset = line_index * self.line_size;

        // Extend buffer by LINE_SIZE bytes, all initialized to '\0'
        self.buffer
            .resize(self.buffer.len() + self.line_size, b'\0');

        // Add the newline character at the start (empty line)
        self.buffer[buffer_offset] = b'\n';

        // Create line metadata
        self.lines.push(LineInfo {
            buffer_offset,
            content_len: 0,
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

        // Remove from metadata
        self.lines.remove(line_index);

        // Shift buffer contents
        let start = line_index * self.line_size;
        let end = self.line_count * self.line_size;

        // Move all subsequent lines up
        for i in start..end - self.line_size {
            self.buffer[i] = self.buffer[i + self.line_size];
        }

        // Truncate the buffer
        self.buffer.truncate((self.line_count - 1) * self.line_size);

        // Update buffer offsets for remaining lines
        for (idx, line) in self.lines.iter_mut().enumerate().skip(line_index) {
            line.buffer_offset = idx * self.line_size;
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
}

impl Default for LineBuffer {
    fn default() -> Self { Self::new() }
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
        assert_eq!(
            display,
            "LineBuffer { lines: 0, buffer_size: 0 bytes, line_size: 256 }"
        );
    }

    #[test]
    fn test_add_line() {
        let mut buffer = LineBuffer::new();

        // Add first line
        let idx1 = buffer.add_line();
        assert_eq!(idx1, 0);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.buffer.len(), LINE_SIZE);
        assert_eq!(buffer.buffer[0], b'\n');

        // Add second line
        let idx2 = buffer.add_line();
        assert_eq!(idx2, 1);
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.buffer.len(), 2 * LINE_SIZE);
        assert_eq!(buffer.buffer[LINE_SIZE], b'\n');

        // Check line info
        let line_info = buffer.get_line_info(0).unwrap();
        assert_eq!(line_info.buffer_offset, 0);
        assert_eq!(line_info.content_len, 0);
        assert_eq!(line_info.grapheme_count, 0);
    }

    #[test]
    fn test_remove_line() {
        let mut buffer = LineBuffer::new();

        // Add three lines
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();
        assert_eq!(buffer.line_count(), 3);

        // Remove middle line
        assert!(buffer.remove_line(1));
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.buffer.len(), 2 * LINE_SIZE);

        // Check buffer offsets were updated
        let line_info = buffer.get_line_info(1).unwrap();
        assert_eq!(line_info.buffer_offset, LINE_SIZE);

        // Try to remove out of bounds
        assert!(!buffer.remove_line(5));
        assert_eq!(buffer.line_count(), 2);
    }

    #[test]
    fn test_clear() {
        let mut buffer = LineBuffer::new();

        // Add some lines
        buffer.add_line();
        buffer.add_line();
        buffer.add_line();

        assert_eq!(buffer.line_count(), 3);
        assert!(!buffer.buffer.is_empty());
        assert!(!buffer.lines.is_empty());

        // Clear the buffer
        buffer.clear();

        assert_eq!(buffer.line_count(), 0);
        assert!(buffer.buffer.is_empty());
        assert!(buffer.lines.is_empty());
    }

    #[test]
    fn test_get_line_count() {
        let mut buffer = LineBuffer::new();
        assert_eq!(buffer.line_count(), 0);

        for i in 1..=5 {
            buffer.add_line();
            assert_eq!(buffer.line_count(), i);
        }
    }

    #[test]
    fn test_bounds_checking() {
        let mut buffer = LineBuffer::new();

        // No lines yet
        assert!(buffer.get_line_info(0).is_none());
        assert!(buffer.get_line_info_mut(0).is_none());

        // Add a line
        buffer.add_line();

        // Valid access
        assert!(buffer.get_line_info(0).is_some());
        assert!(buffer.get_line_info_mut(0).is_some());

        // Out of bounds
        assert!(buffer.get_line_info(1).is_none());
        assert!(buffer.get_line_info_mut(1).is_none());
    }
}

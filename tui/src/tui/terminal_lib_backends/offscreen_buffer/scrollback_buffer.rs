// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Ring-buffer-backed scrollback history for terminal emulation.
//!
//! [`ScrollbackBuffer`] stores lines that have scrolled off the top of the visible
//! terminal screen. It is bounded by a fixed capacity: once full, new lines overwrite
//! the oldest entries.

use super::PixelCharLine;

/// Default maximum number of scrollback lines.
pub const DEFAULT_SCROLLBACK_CAPACITY: usize = 10_000;

/// Fixed-capacity ring buffer for scrollback history.
///
/// When the buffer reaches capacity, pushing a new line evicts the oldest line.
/// Random access and iteration are both O(1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrollbackBuffer {
    lines: Vec<PixelCharLine>,
    start: usize,
    len: usize,
    cap: usize,
}

impl Default for ScrollbackBuffer {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_SCROLLBACK_CAPACITY)
    }
}

impl ScrollbackBuffer {
    /// Create a new empty buffer with the given capacity.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            lines: Vec::with_capacity(cap),
            start: 0,
            len: 0,
            cap,
        }
    }

    /// Number of lines currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer contains no lines.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Maximum capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Append a line. When at capacity, overwrites the oldest line.
    pub fn push(&mut self, line: PixelCharLine) {
        if self.cap == 0 {
            return;
        }
        if self.len < self.cap {
            self.lines.push(line);
            self.len += 1;
        } else {
            self.lines[self.start] = line;
            self.start = (self.start + 1) % self.cap;
        }
    }

    /// Get a line by logical index (0 = oldest, len-1 = newest).
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&PixelCharLine> {
        if idx >= self.len {
            return None;
        }
        Some(&self.lines[(self.start + idx) % self.cap])
    }

    /// Iterator over all lines from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &PixelCharLine> {
        (0..self.len).filter_map(move |i| self.get(i))
    }

    /// Clear all stored lines.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.start = 0;
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PixelChar, TuiStyle, width};

    fn empty_line(w: usize) -> PixelCharLine {
        PixelCharLine::new_empty(width(w as u16))
    }

    fn char_line(w: usize, ch: char) -> PixelCharLine {
        let mut line = empty_line(w);
        for i in 0..w {
            line.pixel_chars[i] = PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            };
        }
        line
    }

    #[test]
    fn test_push_and_get() {
        let mut sb = ScrollbackBuffer::with_capacity(5);
        sb.push(char_line(3, 'A'));
        sb.push(char_line(3, 'B'));

        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get(0).unwrap().pixel_chars[0], PixelChar::PlainText {
            display_char: 'A',
            style: TuiStyle::default(),
        });
        assert_eq!(sb.get(1).unwrap().pixel_chars[0], PixelChar::PlainText {
            display_char: 'B',
            style: TuiStyle::default(),
        });
    }

    #[test]
    fn test_wrap_at_capacity() {
        let mut sb = ScrollbackBuffer::with_capacity(3);
        sb.push(char_line(2, 'A'));
        sb.push(char_line(2, 'B'));
        sb.push(char_line(2, 'C'));
        sb.push(char_line(2, 'D'));

        assert_eq!(sb.len(), 3);
        // Oldest should now be B (A was evicted)
        assert_eq!(sb.get(0).unwrap().pixel_chars[0], PixelChar::PlainText {
            display_char: 'B',
            style: TuiStyle::default(),
        });
        assert_eq!(sb.get(1).unwrap().pixel_chars[0], PixelChar::PlainText {
            display_char: 'C',
            style: TuiStyle::default(),
        });
        assert_eq!(sb.get(2).unwrap().pixel_chars[0], PixelChar::PlainText {
            display_char: 'D',
            style: TuiStyle::default(),
        });
    }

    #[test]
    fn test_get_out_of_bounds() {
        let sb = ScrollbackBuffer::with_capacity(5);
        assert!(sb.get(0).is_none());
    }

    #[test]
    fn test_clear() {
        let mut sb = ScrollbackBuffer::with_capacity(5);
        sb.push(char_line(2, 'A'));
        sb.clear();
        assert!(sb.is_empty());
        assert_eq!(sb.len(), 0);
    }

    #[test]
    fn test_zero_capacity() {
        let mut sb = ScrollbackBuffer::with_capacity(0);
        sb.push(char_line(2, 'A'));
        assert!(sb.is_empty());
    }
}

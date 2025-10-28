// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of [`PixelCharLines`] struct and its methods.
//!
//! [`PixelCharLines`] represents a collection of [`PixelCharLine`] objects,
//! used to store multiple lines of text in the offscreen buffer.
//!
//! [`PixelCharLines`]: crate::PixelCharLines
//! [`PixelCharLine`]: crate::PixelCharLine

use super::PixelCharLine;
use crate::{GetMemSize, InlineVec, Size, get_mem_size};
use smallvec::smallvec;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PixelCharLines {
    pub lines: InlineVec<PixelCharLine>,
}

impl GetMemSize for PixelCharLines {
    fn get_mem_size(&self) -> usize { get_mem_size::slice_size(self.lines.as_ref()) }
}

impl Deref for PixelCharLines {
    type Target = InlineVec<PixelCharLine>;
    fn deref(&self) -> &Self::Target { &self.lines }
}

impl DerefMut for PixelCharLines {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.lines }
}

impl PixelCharLines {
    #[must_use]
    pub fn new_empty(arg_window_size: impl Into<Size>) -> Self {
        let window_size: Size = arg_window_size.into();
        let window_height = window_size.row_height;
        let window_width = window_size.col_width;
        Self {
            lines: smallvec![
                PixelCharLine::new_empty(window_width);
                window_height.as_usize()
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PixelChar, TuiStyle, height, width};

    #[test]
    fn test_pixel_char_lines_new_empty() {
        let size = height(3) + width(4);
        let lines = PixelCharLines::new_empty(size);

        assert_eq!(lines.lines.len(), 3);

        // Check each line has correct width and is filled with spacers.
        for line in &lines.lines {
            assert_eq!(line.pixel_chars.len(), 4);
            for pixel_char in &line.pixel_chars {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }
    }

    #[test]
    fn test_pixel_char_lines_new_empty_zero_size() {
        let size = height(0) + width(0);
        let lines = PixelCharLines::new_empty(size);

        assert_eq!(lines.lines.len(), 0);
        assert!(lines.lines.is_empty());
    }

    #[test]
    fn test_pixel_char_lines_new_empty_zero_height() {
        let size = height(0) + width(5);
        let lines = PixelCharLines::new_empty(size);

        assert_eq!(lines.lines.len(), 0);
        assert!(lines.lines.is_empty());
    }

    #[test]
    fn test_pixel_char_lines_new_empty_zero_width() {
        let size = height(3) + width(0);
        let lines = PixelCharLines::new_empty(size);

        assert_eq!(lines.lines.len(), 3);

        // Each line should be empty.
        for line in &lines.lines {
            assert_eq!(line.pixel_chars.len(), 0);
            assert!(line.pixel_chars.is_empty());
        }
    }

    #[test]
    fn test_pixel_char_lines_deref() {
        let size = height(2) + width(3);
        let lines = PixelCharLines::new_empty(size);

        // Test deref functionality.
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].pixel_chars.len(), 3);
        assert_eq!(lines[1].pixel_chars.len(), 3);
    }

    #[test]
    fn test_pixel_char_lines_deref_mut() {
        let size = height(2) + width(2);
        let mut lines = PixelCharLines::new_empty(size);

        // Test deref_mut functionality.
        lines[0][0] = PixelChar::PlainText {
            display_char: 'A',
            style: TuiStyle::default(),
        };
        lines[1][1] = PixelChar::Void;

        assert!(matches!(
            lines[0][0],
            PixelChar::PlainText {
                display_char: 'A',
                ..
            }
        ));
        assert!(matches!(lines[1][1], PixelChar::Void));
        assert!(matches!(lines[0][1], PixelChar::Spacer)); // Unchanged
        assert!(matches!(lines[1][0], PixelChar::Spacer)); // Unchanged
    }

    #[test]
    fn test_pixel_char_lines_memory_size() {
        let size = height(5) + width(10);
        let lines = PixelCharLines::new_empty(size);

        let mem_size = lines.get_mem_size();
        assert!(mem_size > 0);

        // Larger buffer should have larger memory size.
        let larger_size = height(10) + width(20);
        let larger_lines = PixelCharLines::new_empty(larger_size);
        let larger_mem_size = larger_lines.get_mem_size();

        assert!(larger_mem_size > mem_size);
    }

    #[test]
    fn test_pixel_char_lines_equality() {
        let size = height(2) + width(2);
        let lines1 = PixelCharLines::new_empty(size);
        let lines2 = PixelCharLines::new_empty(size);

        assert_eq!(lines1, lines2);

        // Modify one and test inequality.
        let mut lines3 = PixelCharLines::new_empty(size);
        lines3[0][0] = PixelChar::Void;

        assert_ne!(lines1, lines3);
    }

    #[test]
    fn test_pixel_char_lines_clone() {
        let size = height(2) + width(2);
        let mut lines = PixelCharLines::new_empty(size);

        lines[0][0] = PixelChar::PlainText {
            display_char: 'X',
            style: TuiStyle::default(),
        };
        lines[1][1] = PixelChar::Void;

        let cloned = lines.clone();
        assert_eq!(lines, cloned);

        // Verify deep clone.
        assert!(matches!(
            cloned[0][0],
            PixelChar::PlainText {
                display_char: 'X',
                ..
            }
        ));
        assert!(matches!(cloned[1][1], PixelChar::Void));
    }

    #[test]
    fn test_pixel_char_lines_hash() {
        use std::collections::HashMap;

        let size = height(1) + width(1);
        let lines1 = PixelCharLines::new_empty(size);
        let lines2 = PixelCharLines::new_empty(size);

        // Equal objects should have equal hashes.
        let mut map = HashMap::new();
        map.insert(lines1.clone(), "value1");
        map.insert(lines2, "value2");

        // Since they're equal, the second insert should override the first.
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&lines1), Some(&"value2"));
    }

    #[test]
    fn test_pixel_char_lines_with_different_sizes() {
        let small_size = height(1) + width(1);
        let medium_size = height(3) + width(5);
        let large_size = height(10) + width(20);

        let small_lines = PixelCharLines::new_empty(small_size);
        let medium_lines = PixelCharLines::new_empty(medium_size);
        let large_lines = PixelCharLines::new_empty(large_size);

        assert_eq!(small_lines.len(), 1);
        assert_eq!(small_lines[0].len(), 1);

        assert_eq!(medium_lines.len(), 3);
        assert_eq!(medium_lines[0].len(), 5);
        assert_eq!(medium_lines[1].len(), 5);
        assert_eq!(medium_lines[2].len(), 5);

        assert_eq!(large_lines.len(), 10);
        for line in &large_lines.lines {
            assert_eq!(line.len(), 20);
        }
    }
}

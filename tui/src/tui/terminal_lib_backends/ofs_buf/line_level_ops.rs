// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![allow(clippy::needless_range_loop)]

//! Implementation of line-level operations for [`OfsBuf`].
//!
//! This module provides methods for manipulating entire rows in the buffer, such as
//! getting, setting, swapping, and shifting lines.

use super::{OfsBuf, PixelChar};
use crate::{CanvasStorage, Flat2DArray, VPRow, ok};

/// Line-level operations.
impl<S: CanvasStorage> OfsBuf<S> {
    /// Gets a reference to a line at the specified row.
    /// Returns None if the row is out of bounds.
    #[must_use]
    pub fn get_line(&self, row: VPRow) -> Option<&[PixelChar]> { self.get_row(row) }

    /// Set an entire line at the specified row.
    ///
    /// # Errors
    ///
    /// Returns an error if the row is out of bounds.
    pub fn set_line(&mut self, row: VPRow, line: &[PixelChar]) -> miette::Result<()> {
        let Some(target_line) = self.get_row_mut(row) else {
            return Err(miette::miette!("Row index out of bounds"));
        };
        target_line.copy_from_slice(line);
        ok!()
    }

    /// Swaps the entire content of two rows in the buffer.
    ///
    /// Depending on the underlying storage implementation, this will perform different
    /// optimizations:
    /// - For [`Flat2DArray`], this is highly optimized and delegates directly to the
    ///   underlying [`Flat1DSimdMut::swap_rows`] [SIMD] implementation, performing bulk
    ///   memory swapping rather than character-by-character iteration.
    /// - For [`GrowableBuffer`], it performs a swap of rows in the dynamic lines queue.
    ///
    /// # Errors
    ///
    /// Returns an error if either `row_1` or `row_2` is out of bounds.
    ///
    /// [`Flat1DSimdMut::swap_rows`]: crate::Flat1DSimdMut::swap_rows
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    /// [`GrowableBuffer`]: crate::tui::GrowableBuffer
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    pub fn swap_lines(&mut self, row_1: VPRow, row_2: VPRow) -> miette::Result<()> {
        (**self).swap_lines(row_1, row_2)
    }
}

impl OfsBuf<Flat2DArray<PixelChar>> {
    pub fn rotate_rows_left(&mut self, start_idx: usize, end_idx: usize, shift: usize) {
        let width = self.get_width().as_usize();
        self.data[start_idx * width..end_idx * width].rotate_left(shift * width);
    }

    pub fn rotate_rows_right(&mut self, start_idx: usize, end_idx: usize, shift: usize) {
        let width = self.get_width().as_usize();
        self.data[start_idx * width..end_idx * width].rotate_right(shift * width);
    }
}

#[cfg(test)]
mod tests_line_level_ops {
    use crate::{OfsBufVT100, PixelChar, ShiftLinesDirection, TuiStyle, vp_col,
                vp_height, vp_len, vp_row, vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(4) + vp_height(5);
        OfsBufVT100::new_empty(size)
    }

    fn create_test_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    fn create_test_line(chars: &[char]) -> Vec<PixelChar> {
        let mut line = vec![PixelChar::Spacer; 4]; // Match buffer width
        for (i, &ch) in chars.iter().enumerate().take(4) {
            line[i] = create_test_char(ch);
        }
        line
    }

    #[test]
    fn test_clear_line() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);

        // Fill the line with test characters first.
        for col_idx in 0..4 {
            let _unused =
                buffer.set_char(test_row + vp_col(col_idx), create_test_char('X'));
        }

        // Clear the line.
        let result = buffer.clear_line(test_row);
        assert!(result.is_ok());

        // Verify all characters are now spacers.
        for col_idx in 0..4 {
            let pos = test_row + vp_col(col_idx);
            let char = buffer.get_char(pos).expect("conversion error");
            assert_eq!(char, PixelChar::Spacer);
        }
    }

    #[test]
    fn test_clear_line_invalid_row() {
        let mut buffer = create_test_buffer();

        // Try to clear an invalid row.
        let result = buffer.clear_line(vp_row(10));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_line() {
        let buffer = create_test_buffer();

        // Test valid row.
        let line = buffer.get_line(vp_row(2));
        assert!(line.is_some());
        assert_eq!(line.expect("conversion error").len(), 4); // Should match buffer width

        // Test invalid row.
        let invalid_line = buffer.get_line(vp_row(10));
        assert!(invalid_line.is_none());
    }

    #[test]
    fn test_set_line() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(2);
        let test_line = create_test_line(&['A', 'B', 'C', 'D']);

        // Set the line.
        let result = buffer.set_line(test_row, &test_line);
        assert!(result.is_ok());

        // Verify the line was set correctly.
        let retrieved_line = buffer.get_line(test_row).expect("conversion error");
        assert_eq!(retrieved_line, &test_line);

        // Verify individual characters.
        assert_eq!(
            buffer
                .get_char(vp_row(2) + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(2) + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(2) + vp_col(2))
                .expect("conversion error"),
            create_test_char('C')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(2) + vp_col(3))
                .expect("conversion error"),
            create_test_char('D')
        );
    }

    #[test]
    fn test_set_line_invalid_row() {
        let mut buffer = create_test_buffer();
        let test_line = create_test_line(&['X', 'Y', 'Z']);

        // Try to set an invalid row.
        let result = buffer.set_line(vp_row(10), &test_line);
        assert!(result.is_err());
    }

    #[test]
    fn test_swap_lines() {
        let mut buffer = create_test_buffer();
        let row1 = vp_row(0);
        let row2 = vp_row(3);

        let line1 = create_test_line(&['1', '2', '3', '4']);
        let line2 = create_test_line(&['A', 'B', 'C', 'D']);

        // Set up the initial lines.
        let _unused = buffer.set_line(row1, &line1);
        let _unused = buffer.set_line(row2, &line2);

        // Swap the lines.
        let result = buffer.swap_lines(row1, row2);
        assert!(result.is_ok());

        // Verify the swap was successful.
        let swapped_line1 = buffer.get_line(row1).expect("conversion error");
        let swapped_line2 = buffer.get_line(row2).expect("conversion error");

        assert_eq!(swapped_line1, &line2); // row1 now has line2's content
        assert_eq!(swapped_line2, &line1); // row2 now has line1's content
    }

    #[test]
    fn test_swap_lines_invalid() {
        let mut buffer = create_test_buffer();

        // Try to swap with invalid rows.
        let result1 = buffer.swap_lines(vp_row(0), vp_row(10));
        assert!(result1.is_err());

        let result2 = buffer.swap_lines(vp_row(10), vp_row(0));
        assert!(result2.is_err());

        let result3 = buffer.swap_lines(vp_row(10), vp_row(11));
        assert!(result3.is_err());
    }

    #[test]
    fn test_shift_lines_up() {
        let mut buffer = create_test_buffer();

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(3), &create_test_line(&['C', 'C', 'C', 'C']));

        // Shift lines 1-3 up by 1.
        let result = buffer.shift_lines_in_range(
            ShiftLinesDirection::Up,
            vp_row(1)..vp_row(4),
            vp_len(1),
        );
        assert!(result.is_ok());

        // Verify the shift: line 2 content should now be at line 1, etc.
        let line1 = buffer.get_line(vp_row(1)).expect("conversion error");
        let line2 = buffer.get_line(vp_row(2)).expect("conversion error");
        let line3 = buffer.get_line(vp_row(3)).expect("conversion error");

        // Line 1 should now have what was line 2's content (all 'B' characters).
        for col_idx in 0..4 {
            assert_eq!(line1[col_idx], create_test_char('B'));
        }

        // Line 2 should now have what was line 3's content (all 'C' characters).
        for col_idx in 0..4 {
            assert_eq!(line2[col_idx], create_test_char('C'));
        }

        // Line 3 should be blank (all spacers).
        for col_idx in 0..4 {
            assert_eq!(line3[col_idx], PixelChar::Spacer);
        }

        // Additional verification using get_char method.
        assert_eq!(
            buffer
                .get_char(vp_row(1) + vp_col(0))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(2) + vp_col(0))
                .expect("conversion error"),
            create_test_char('C')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(3) + vp_col(0))
                .expect("conversion error"),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_shift_lines_down() {
        let mut buffer = create_test_buffer();

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(3), &create_test_line(&['C', 'C', 'C', 'C']));

        // Shift lines 1-3 down by 1.
        let result = buffer.shift_lines_in_range(
            ShiftLinesDirection::Down,
            vp_row(1)..vp_row(4),
            vp_len(1),
        );
        assert!(result.is_ok());

        // Verify the shift: line 1 content should now be at line 2, etc.
        let line1 = buffer.get_line(vp_row(1)).expect("conversion error");
        let line2 = buffer.get_line(vp_row(2)).expect("conversion error");
        let line3 = buffer.get_line(vp_row(3)).expect("conversion error");

        // Line 1 should now be blank (all spacers).
        for col_idx in 0..4 {
            assert_eq!(line1[col_idx], PixelChar::Spacer);
        }

        // Line 2 should now have what was line 1's content (all 'A' characters).
        for col_idx in 0..4 {
            assert_eq!(line2[col_idx], create_test_char('A'));
        }

        // Line 3 should now have what was line 2's content (all 'B' characters).
        for col_idx in 0..4 {
            assert_eq!(line3[col_idx], create_test_char('B'));
        }

        // Additional verification using get_char method.
        assert_eq!(
            buffer
                .get_char(vp_row(1) + vp_col(0))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(vp_row(2) + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(3) + vp_col(0))
                .expect("conversion error"),
            create_test_char('B')
        );
    }

    #[test]
    fn test_shift_lines_invalid_ranges() {
        let mut buffer = create_test_buffer();

        // Test invalid row ranges.
        let result1 = buffer.shift_lines_in_range(
            ShiftLinesDirection::Up,
            vp_row(10)..vp_row(12),
            vp_len(1),
        );
        assert!(result1.is_err());

        let result2 = buffer.shift_lines_in_range(
            ShiftLinesDirection::Down,
            vp_row(3)..vp_row(1),
            vp_len(1),
        ); // Backward range
        assert!(result2.is_err());

        let result3 = buffer.shift_lines_in_range(
            ShiftLinesDirection::Up,
            vp_row(0)..vp_row(10),
            vp_len(1),
        ); // End beyond buffer
        assert!(result3.is_err());
    }
}

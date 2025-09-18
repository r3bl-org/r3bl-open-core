// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{OffscreenBuffer, PixelCharLine};
use crate::RowIndex;

/// Line-level operations.
impl OffscreenBuffer {
    /// Get a reference to a line at the specified row.
    /// Returns None if the row is out of bounds.
    #[must_use]
    pub fn get_line(&self, row: RowIndex) -> Option<&PixelCharLine> {
        self.buffer.get(row.as_usize())
    }

    /// Set an entire line at the specified row.
    /// Returns true if the operation was successful.
    pub fn set_line(&mut self, row: RowIndex, line: PixelCharLine) -> bool {
        let row_idx = row.as_usize();
        if let Some(target_line) = self.buffer.get_mut(row_idx) {
            *target_line = line;
            true
        } else {
            false
        }
    }

    /// Swap two lines in the buffer.
    /// Returns true if both rows are valid and the swap was successful.
    pub fn swap_lines(&mut self, row_1: RowIndex, row_2: RowIndex) -> bool {
        let row_1_idx = row_1.as_usize();
        let row_2_idx = row_2.as_usize();

        if row_1_idx < self.buffer.len() && row_2_idx < self.buffer.len() {
            self.buffer.swap(row_1_idx, row_2_idx);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests_line_level_ops {
    use super::*;
    use crate::{PixelChar, TuiStyle, col, height, len, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(4) + height(5);
        OffscreenBuffer::new_empty(size)
    }

    fn create_test_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    fn create_test_line(chars: &[char]) -> PixelCharLine {
        let mut line = vec![PixelChar::Spacer; 4]; // Match buffer width
        for (i, &ch) in chars.iter().enumerate().take(4) {
            line[i] = create_test_char(ch);
        }
        PixelCharLine { pixel_chars: line }
    }

    #[test]
    fn test_clear_line() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Fill the line with test characters first.
        for col_idx in 0..4 {
            buffer.set_char(test_row + col(col_idx), create_test_char('X'));
        }

        // Clear the line.
        let result = buffer.clear_line(test_row);
        assert!(result);

        // Verify all characters are now spacers.
        for col_idx in 0..4 {
            let pos = test_row + col(col_idx);
            let char = buffer.get_char(pos).unwrap();
            assert_eq!(char, PixelChar::Spacer);
        }
    }

    #[test]
    fn test_clear_line_invalid_row() {
        let mut buffer = create_test_buffer();

        // Try to clear an invalid row.
        let result = buffer.clear_line(row(10));
        assert!(!result);
    }

    #[test]
    fn test_get_line() {
        let buffer = create_test_buffer();

        // Test valid row.
        let line = buffer.get_line(row(2));
        assert!(line.is_some());
        assert_eq!(line.unwrap().len(), 4); // Should match buffer width

        // Test invalid row.
        let invalid_line = buffer.get_line(row(10));
        assert!(invalid_line.is_none());
    }

    #[test]
    fn test_set_line() {
        let mut buffer = create_test_buffer();
        let test_row = row(2);
        let test_line = create_test_line(&['A', 'B', 'C', 'D']);

        // Set the line.
        let result = buffer.set_line(test_row, test_line.clone());
        assert!(result);

        // Verify the line was set correctly.
        let retrieved_line = buffer.get_line(test_row).unwrap();
        assert_eq!(retrieved_line, &test_line);

        // Verify individual characters.
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            create_test_char('A')
        );
        assert_eq!(
            buffer.get_char(test_row + col(1)).unwrap(),
            create_test_char('B')
        );
        assert_eq!(
            buffer.get_char(test_row + col(2)).unwrap(),
            create_test_char('C')
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            create_test_char('D')
        );
    }

    #[test]
    fn test_set_line_invalid_row() {
        let mut buffer = create_test_buffer();
        let test_line = create_test_line(&['X', 'Y', 'Z']);

        // Try to set an invalid row.
        let result = buffer.set_line(row(10), test_line);
        assert!(!result);
    }

    #[test]
    fn test_swap_lines() {
        let mut buffer = create_test_buffer();
        let row1 = row(0);
        let row2 = row(3);

        let line1 = create_test_line(&['1', '2', '3', '4']);
        let line2 = create_test_line(&['A', 'B', 'C', 'D']);

        // Set up the initial lines.
        buffer.set_line(row1, line1.clone());
        buffer.set_line(row2, line2.clone());

        // Swap the lines.
        let result = buffer.swap_lines(row1, row2);
        assert!(result);

        // Verify the swap was successful.
        let swapped_line1 = buffer.get_line(row1).unwrap();
        let swapped_line2 = buffer.get_line(row2).unwrap();

        assert_eq!(swapped_line1, &line2); // row1 now has line2's content
        assert_eq!(swapped_line2, &line1); // row2 now has line1's content
    }

    #[test]
    fn test_swap_lines_invalid() {
        let mut buffer = create_test_buffer();

        // Try to swap with invalid rows.
        let result1 = buffer.swap_lines(row(0), row(10));
        assert!(!result1);

        let result2 = buffer.swap_lines(row(10), row(0));
        assert!(!result2);

        let result3 = buffer.swap_lines(row(10), row(11));
        assert!(!result3);
    }

    #[test]
    fn test_shift_lines_up() {
        let mut buffer = create_test_buffer();

        // Set up initial lines.
        buffer.set_line(row(1), create_test_line(&['A', 'A', 'A', 'A']));
        buffer.set_line(row(2), create_test_line(&['B', 'B', 'B', 'B']));
        buffer.set_line(row(3), create_test_line(&['C', 'C', 'C', 'C']));

        // Shift lines 1-3 up by 1.
        let result = buffer.shift_lines_up(row(1)..row(4), len(1));
        assert!(result);

        // Verify the shift: line 2 content should now be at line 1, etc.
        let line1 = buffer.get_line(row(1)).unwrap();
        let line2 = buffer.get_line(row(2)).unwrap();
        let line3 = buffer.get_line(row(3)).unwrap();

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
            buffer.get_char(row(1) + col(0)).unwrap(),
            create_test_char('B')
        );
        assert_eq!(
            buffer.get_char(row(2) + col(0)).unwrap(),
            create_test_char('C')
        );
        assert_eq!(buffer.get_char(row(3) + col(0)).unwrap(), PixelChar::Spacer);
    }

    #[test]
    fn test_shift_lines_down() {
        let mut buffer = create_test_buffer();

        // Set up initial lines.
        buffer.set_line(row(1), create_test_line(&['A', 'A', 'A', 'A']));
        buffer.set_line(row(2), create_test_line(&['B', 'B', 'B', 'B']));
        buffer.set_line(row(3), create_test_line(&['C', 'C', 'C', 'C']));

        // Shift lines 1-3 down by 1.
        let result = buffer.shift_lines_down(row(1)..row(4), len(1));
        assert!(result);

        // Verify the shift: line 1 content should now be at line 2, etc.
        let line1 = buffer.get_line(row(1)).unwrap();
        let line2 = buffer.get_line(row(2)).unwrap();
        let line3 = buffer.get_line(row(3)).unwrap();

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
        assert_eq!(buffer.get_char(row(1) + col(0)).unwrap(), PixelChar::Spacer);
        assert_eq!(
            buffer.get_char(row(2) + col(0)).unwrap(),
            create_test_char('A')
        );
        assert_eq!(
            buffer.get_char(row(3) + col(0)).unwrap(),
            create_test_char('B')
        );
    }

    #[test]
    fn test_shift_lines_invalid_ranges() {
        let mut buffer = create_test_buffer();

        // Test invalid row ranges.
        let result1 = buffer.shift_lines_up(row(10)..row(12), len(1));
        assert!(!result1);

        let result2 = buffer.shift_lines_down(row(3)..row(1), len(1)); // Backward range
        assert!(!result2);

        let result3 = buffer.shift_lines_up(row(0)..row(10), len(1)); // End beyond buffer
        assert!(!result3);
    }
}

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of character shifting operations for `OffscreenBuffer`.
//!
//! This module provides methods for inserting, deleting, and erasing characters
//! at cursor positions within the buffer, handling proper shifting of existing
//! content as required by terminal emulation standards.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{Length, core::units::bounds_check::LengthMarker, height, len};

impl OffscreenBuffer {
    /// Insert blank characters at cursor position (for ICH - Insert Character).
    /// Characters at and after the cursor shift right by `how_many`.
    /// Characters that would shift beyond the line width are lost.
    /// Returns true if the operation was successful.
    ///
    /// Example - Inserting 2 blank characters at cursor position.
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ─────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After insert 2 blanks:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │   │   │ C │ D │ E │ F │ G │ H │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: 2 blanks inserted, C-D-E-F-G-H shifted right, I-J lost beyond margin.
    /// ```
    pub fn insert_chars_at_cursor(&mut self, how_many: Length) -> bool {
        let at = self.cursor_pos;
        let max_width = self.window_size.col_width;

        // Nothing to insert if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at) {
            return false;
        }

        // Calculate how many characters we can actually insert.
        let how_many_clamped = how_many.clamp_to(max_width.remaining_from(at));

        // Exit early if nothing to insert.
        if how_many_clamped == len(0) {
            return false;
        }

        let buffer_height = height(self.buffer.len());
        if buffer_height.is_overflowed_by(at) {
            return false;
        }

        let row_idx = at.row_index.as_usize();

        let cursor_pos = at.col_index.as_usize();
        let insert_amount = how_many_clamped.as_usize();
        let line_width = max_width.as_usize();

        if let Some(line) = self.buffer.get_mut(row_idx) {
            // Copy characters to the right to make room for insertion.
            let dest_start = cursor_pos + insert_amount;
            let source_end = line_width - insert_amount;

            if dest_start < line_width && cursor_pos < source_end {
                line.copy_within(cursor_pos..source_end, dest_start);
            }

            // Fill the cursor position with blanks.
            let fill_end = (cursor_pos + insert_amount).min(line_width);
            line[cursor_pos..fill_end].fill(PixelChar::Spacer);

            self.invalidate_memory_size_calc_cache();
            return true;
        }
        false
    }

    /// Delete characters at cursor position (for DCH - Delete Character).
    /// Characters at and after the deletion point shift left by `how_many`.
    /// Blank characters are inserted at the end of the line.
    /// Returns true if the operation was successful.
    ///
    /// Example - Deleting 2 characters at cursor position.
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ──────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ c │ d │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After delete 2 chars:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ E │ F │ G │ H │ I │ J │   │   │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: c and d deleted, E-F-G-H-I-J shifted left, blanks filled at end.
    /// ```
    pub fn delete_chars_at_cursor(&mut self, how_many: Length) -> bool {
        let at = self.cursor_pos;
        let max_width = self.window_size.col_width;

        // Nothing to delete if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at) {
            return false;
        }

        // Calculate how many characters we can actually delete.
        let how_many_clamped = how_many.clamp_to(max_width.remaining_from(at));

        // Exit early if nothing to delete.
        if how_many_clamped == len(0) {
            return false;
        }

        let buffer_height = height(self.buffer.len());
        if buffer_height.is_overflowed_by(at) {
            return false;
        }

        // Copy characters from the right, overwriting the characters at cursor (this IS
        // the deletion).
        self.copy_chars_within_line(
            at.row_index,
            {
                let start = at.col_index + how_many_clamped;
                let end = max_width.convert_to_col_index() + len(1);
                start..end
            },
            at.col_index,
        );

        // Clear the vacated space at the end (overwriting duplicates and filling with
        // spacers).
        self.fill_char_range(
            at.row_index,
            {
                let start = max_width.convert_to_col_index() - how_many_clamped + len(1);
                let end = max_width.convert_to_col_index() + len(1);
                start..end
            },
            PixelChar::Spacer,
        );

        self.invalidate_memory_size_calc_cache();
        true
    }

    /// Erase characters at cursor position (for ECH - Erase Character).
    /// Characters are replaced with blanks, no shifting occurs.
    /// Returns true if the operation was successful.
    ///
    /// Example - Erasing 3 characters at cursor position.
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ──────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After erase 3 chars:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │   │   │   │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: C, D, E replaced with blanks, F-G-H-I-J remain in place (no shifting)
    /// ```
    pub fn erase_chars_at_cursor(&mut self, how_many: Length) -> bool {
        let at = self.cursor_pos;
        let max_width = self.window_size.col_width;

        // Nothing to erase if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at) {
            return false;
        }

        // Calculate how many characters we can actually erase.
        let how_many_clamped = how_many.clamp_to(max_width.remaining_from(at));

        // Exit early if nothing to erase.
        if how_many_clamped == len(0) {
            return false;
        }

        let buffer_height = height(self.buffer.len());
        if buffer_height.is_overflowed_by(at) {
            return false;
        }

        let row_idx = at.row_index.as_usize();

        let cursor_pos = at.col_index.as_usize();
        let erase_amount = how_many_clamped.as_usize();

        if let Some(line) = self.buffer.get_mut(row_idx) {
            // Fill the range with blank characters.
            let fill_end = cursor_pos + erase_amount;
            line[cursor_pos..fill_end].fill(PixelChar::Spacer);

            self.invalidate_memory_size_calc_cache();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests_shifting_ops {
    use super::*;
    use crate::{RowIndex, TuiStyle, col, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(6) + height(3);
        OffscreenBuffer::new_empty(size)
    }

    fn create_test_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    fn setup_line_with_chars(
        buffer: &mut OffscreenBuffer,
        test_row: RowIndex,
        chars: &[char],
    ) {
        for (i, &ch) in chars.iter().enumerate() {
            if i < 6 {
                // Match buffer width.
                buffer.set_char(test_row + col(i), create_test_char(ch));
            }
        }
    }

    #[test]
    fn test_insert_chars_at_cursor_basic() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Insert 2 blank characters at position 2 (before 'C').
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.insert_chars_at_cursor(len(2));
        assert!(result);

        // Expected result: "AB  CD" (E and F are pushed out).
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
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(4)).unwrap(),
            create_test_char('C')
        );
        assert_eq!(
            buffer.get_char(test_row + col(5)).unwrap(),
            create_test_char('D')
        );
    }

    #[test]
    fn test_insert_chars_at_cursor_overflow() {
        let mut buffer = create_test_buffer();
        let test_row = row(0);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to insert 10 characters at position 1 (more than remaining space).
        buffer.cursor_pos = test_row + col(1);
        let result = buffer.insert_chars_at_cursor(len(10));
        assert!(result);

        // Should insert as many as possible: "A     " (5 spaces, B-F pushed out).
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            create_test_char('A')
        );
        for i in 1..6 {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }
    }

    #[test]
    fn test_insert_chars_at_end_of_line() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to insert at the last position.
        buffer.cursor_pos = test_row + col(5);
        let result = buffer.insert_chars_at_cursor(len(1));
        assert!(result);

        // Should insert one space, pushing F out: "ABCDE ".
        assert_eq!(
            buffer.get_char(test_row + col(4)).unwrap(),
            create_test_char('E')
        );
        assert_eq!(
            buffer.get_char(test_row + col(5)).unwrap(),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_insert_chars_invalid_conditions() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        buffer.cursor_pos = row(10) + col(2);
        let result1 = buffer.insert_chars_at_cursor(len(1));
        assert!(!result1);

        // Test with cursor position beyond line width.
        buffer.cursor_pos = row(0) + col(10);
        let result2 = buffer.insert_chars_at_cursor(len(1));
        assert!(!result2);

        // Test with zero insert count.
        buffer.cursor_pos = row(0) + col(2);
        let result3 = buffer.insert_chars_at_cursor(len(0));
        assert!(!result3);
    }

    #[test]
    fn test_delete_chars_at_cursor_basic() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Delete 2 characters at position 2 (delete 'C' and 'D').
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.delete_chars_at_cursor(len(2));
        assert!(result);

        // Verify: "AB" + "EF" + "  " (CD deleted, EF shifted left, blanks at end).
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
            create_test_char('E')
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            create_test_char('F')
        );
        assert_eq!(
            buffer.get_char(test_row + col(4)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(5)).unwrap(),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_delete_chars_at_cursor_overflow() {
        let mut buffer = create_test_buffer();
        let test_row = row(0);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to delete 10 characters at position 1 (more than remaining space)
        buffer.cursor_pos = test_row + col(1);
        let result = buffer.delete_chars_at_cursor(len(10));
        assert!(result);

        // Verify: "A" + "     " (BCDEF all deleted, 5 blanks at end)
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            create_test_char('A')
        );
        for i in 1..6 {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }
    }

    #[test]
    fn test_delete_chars_at_end_of_line() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to delete at the last position.
        buffer.cursor_pos = test_row + col(5);
        let result = buffer.delete_chars_at_cursor(len(1));
        assert!(result);

        // Verify: "ABCDE " (F deleted, one blank at end)
        for (i, expected_char) in ['A', 'B', 'C', 'D', 'E'].iter().enumerate() {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                create_test_char(*expected_char)
            );
        }
        assert_eq!(
            buffer.get_char(test_row + col(5)).unwrap(),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_delete_chars_invalid_conditions() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        buffer.cursor_pos = row(10) + col(2);
        let result1 = buffer.delete_chars_at_cursor(len(1));
        assert!(!result1);

        // Test with cursor position beyond line width.
        buffer.cursor_pos = row(0) + col(10);
        let result2 = buffer.delete_chars_at_cursor(len(1));
        assert!(!result2);

        // Test with zero delete count.
        buffer.cursor_pos = row(0) + col(2);
        let result3 = buffer.delete_chars_at_cursor(len(0));
        assert!(!result3);
    }

    #[test]
    fn test_erase_chars_at_cursor_basic() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Erase 3 characters at position 2 (erase 'C', 'D', 'E')
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.erase_chars_at_cursor(len(3));
        assert!(result);

        // Verify: "AB" + "   " + "F" (CDE erased with blanks, F stays in place)
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
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(4)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(5)).unwrap(),
            create_test_char('F')
        );
    }

    #[test]
    fn test_erase_chars_at_cursor_overflow() {
        let mut buffer = create_test_buffer();
        let test_row = row(0);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to erase 10 characters at position 1 (more than remaining space)
        buffer.cursor_pos = test_row + col(1);
        let result = buffer.erase_chars_at_cursor(len(10));
        assert!(result);

        // Verify: "A" + "     " (BCDEF all erased with blanks)
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            create_test_char('A')
        );
        for i in 1..6 {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }
    }

    #[test]
    fn test_erase_chars_at_end_of_line() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to erase at the last position.
        buffer.cursor_pos = test_row + col(5);
        let result = buffer.erase_chars_at_cursor(len(1));
        assert!(result);

        // Verify: "ABCDE " (F erased with blank)
        for (i, expected_char) in ['A', 'B', 'C', 'D', 'E'].iter().enumerate() {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                create_test_char(*expected_char)
            );
        }
        assert_eq!(
            buffer.get_char(test_row + col(5)).unwrap(),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_erase_chars_invalid_conditions() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        buffer.cursor_pos = row(10) + col(2);
        let result1 = buffer.erase_chars_at_cursor(len(1));
        assert!(!result1);

        // Test with cursor position beyond line width.
        buffer.cursor_pos = row(0) + col(10);
        let result2 = buffer.erase_chars_at_cursor(len(1));
        assert!(!result2);

        // Test with zero erase count.
        buffer.cursor_pos = row(0) + col(2);
        let result3 = buffer.erase_chars_at_cursor(len(0));
        assert!(!result3);
    }

    #[test]
    fn test_operations_at_line_start() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        let size = width(10) + height(3);
        let mut buffer = OffscreenBuffer::new_empty(size);
        let test_row = row(0);

        // Set up initial line with characters: "ABCDEFGHIJ".
        let chars = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'];
        for (i, &ch) in chars.iter().enumerate() {
            buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test delete at column 0 - should delete A,B and shift left.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.delete_chars_at_cursor(len(2));
        assert!(result);

        // Verify: C,D,E,F,G,H,I,J shifted left, blanks at end
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            create_test_char('C')
        );
        assert_eq!(
            buffer.get_char(test_row + col(1)).unwrap(),
            create_test_char('D')
        );
        assert_eq!(
            buffer.get_char(test_row + col(7)).unwrap(),
            create_test_char('J')
        );
        assert_eq!(
            buffer.get_char(test_row + col(8)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            PixelChar::Spacer
        );

        // Reset for insert test.
        for (i, &ch) in chars.iter().enumerate() {
            buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test insert at column 0 - should insert 2 blanks and shift right.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.insert_chars_at_cursor(len(2));
        assert!(result);

        // Verify: 2 blanks inserted at start, A-H shifted right, I,J lost
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(1)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(2)).unwrap(),
            create_test_char('A')
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            create_test_char('H')
        );

        // Reset for erase test.
        for (i, &ch) in chars.iter().enumerate() {
            buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test erase at column 0 - should erase A,B,C without shifting.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.erase_chars_at_cursor(len(3));
        assert!(result);

        // Verify: A,B,C erased (blanks), D-J remain in place
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(1)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(2)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            create_test_char('D')
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            create_test_char('J')
        );
    }

    #[test]
    fn test_single_char_operations() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        let size = width(5) + height(2);
        let mut buffer = OffscreenBuffer::new_empty(size);
        let test_row = row(0);

        // Set up initial line with characters: "ABCDE".
        let chars = ['A', 'B', 'C', 'D', 'E'];
        for (i, &ch) in chars.iter().enumerate() {
            buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test single char delete at middle position (delete C)
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.delete_chars_at_cursor(len(1));
        assert!(result);

        // Verify: A,B remain, D,E shifted left, blank at end
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
            create_test_char('D')
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            create_test_char('E')
        );
        assert_eq!(
            buffer.get_char(test_row + col(4)).unwrap(),
            PixelChar::Spacer
        );

        // Reset for insert test.
        for (i, &ch) in chars.iter().enumerate() {
            buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test single char insert at middle position (before C)
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.insert_chars_at_cursor(len(1));
        assert!(result);

        // Verify: A,B remain, blank inserted, C,D shifted right, E lost
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
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            create_test_char('C')
        );
        assert_eq!(
            buffer.get_char(test_row + col(4)).unwrap(),
            create_test_char('D')
        );

        // Reset for erase test.
        for (i, &ch) in chars.iter().enumerate() {
            buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test single char erase at middle position (erase C)
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.erase_chars_at_cursor(len(1));
        assert!(result);

        // Verify: A,B remain, C erased (blank), D,E remain in place
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
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            create_test_char('D')
        );
        assert_eq!(
            buffer.get_char(test_row + col(4)).unwrap(),
            create_test_char('E')
        );
    }

    #[test]
    fn test_operations_on_empty_line() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        let size = width(8) + height(3);
        let mut buffer = OffscreenBuffer::new_empty(size);
        let test_row = row(0);

        // Test delete on empty line (should succeed but do nothing)
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.delete_chars_at_cursor(len(3));
        assert!(result); // Should succeed on spacer-filled line
        // Verify line remains empty.
        for i in 0..8 {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }

        // Test insert on empty line at column 0.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.insert_chars_at_cursor(len(3));
        assert!(result);
        // Verify 3 blanks were inserted (line now has length but no visible chars)
        for i in 0..3 {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }

        // Reset to empty line.
        buffer.buffer[test_row.as_usize()] = PixelCharLine::new_empty(width(8));

        // Test erase on empty line (should succeed but do nothing)
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.erase_chars_at_cursor(len(2));
        assert!(result); // Should succeed on spacer-filled line
        // Verify line remains empty.
        for i in 0..8 {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }

        // Test operations beyond line length on short line.
        let chars = ['A', 'B', 'C'];
        for (i, &ch) in chars.iter().enumerate() {
            buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Try to delete at position beyond content length (but within width)
        buffer.cursor_pos = test_row + col(5);
        let result = buffer.delete_chars_at_cursor(len(1));
        assert!(result); // Should succeed - position is within buffer width
        // Verify original content unchanged.
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

        // Try to insert at valid position within width but beyond content.
        buffer.cursor_pos = test_row + col(5);
        let result = buffer.insert_chars_at_cursor(len(1));
        assert!(result);
        // Verify original content plus expanded line with blank.
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
            buffer.get_char(test_row + col(5)).unwrap(),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_delete_at_exact_boundary() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        fn setup_buffer() -> OffscreenBuffer {
            let size = width(10) + height(3);
            let mut buffer = OffscreenBuffer::new_empty(size);
            let test_row = row(0);
            let chars = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'];
            for (i, &ch) in chars.iter().enumerate() {
                buffer.set_char(test_row + col(i), create_test_char(ch));
            }
            buffer
        }

        let mut buffer = setup_buffer();
        let test_row = row(0);

        // Test delete at exact right boundary (should do nothing)
        buffer.cursor_pos = test_row + col(10);
        let result = buffer.delete_chars_at_cursor(len(1));
        assert!(!result); // Should fail - cursor is beyond valid position
        // Verify line unchanged.
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            create_test_char('J')
        );

        // Test delete at last valid position.
        buffer.cursor_pos = test_row + col(9);
        let result = buffer.delete_chars_at_cursor(len(1));
        assert!(result);
        // Verify J deleted, blank at end.
        assert_eq!(
            buffer.get_char(test_row + col(8)).unwrap(),
            create_test_char('I')
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            PixelChar::Spacer
        );

        // Test delete with count exceeding available chars.
        buffer = setup_buffer();
        buffer.cursor_pos = test_row + col(8);
        let result = buffer.delete_chars_at_cursor(len(5));
        assert!(result);
        // Verify only 2 chars deleted, others remain, blanks at end.
        assert_eq!(
            buffer.get_char(test_row + col(7)).unwrap(),
            create_test_char('H')
        );
        assert_eq!(
            buffer.get_char(test_row + col(8)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_insert_at_exact_boundary() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        fn setup_buffer() -> OffscreenBuffer {
            let size = width(10) + height(3);
            let mut buffer = OffscreenBuffer::new_empty(size);
            let test_row = row(0);
            let chars = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'];
            for (i, &ch) in chars.iter().enumerate() {
                buffer.set_char(test_row + col(i), create_test_char(ch));
            }
            buffer
        }

        let mut buffer = setup_buffer();
        let test_row = row(0);

        // Test insert at exact right boundary (should do nothing)
        buffer.cursor_pos = test_row + col(10);
        let result = buffer.insert_chars_at_cursor(len(1));
        assert!(!result); // Should fail - cursor is beyond valid position
        // Verify line unchanged.
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            create_test_char('J')
        );

        // Test insert at last valid position (should push rightmost char off)
        buffer.cursor_pos = test_row + col(9);
        let result = buffer.insert_chars_at_cursor(len(1));
        assert!(result);
        // Verify blank inserted at position 9, J pushed off.
        assert_eq!(
            buffer.get_char(test_row + col(8)).unwrap(),
            create_test_char('I')
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            PixelChar::Spacer
        );

        // Test insert with large count.
        buffer = setup_buffer();
        buffer.cursor_pos = test_row + col(8);
        let result = buffer.insert_chars_at_cursor(len(5));
        assert!(result);
        // Verify only 2 blanks inserted, I,J pushed off.
        assert_eq!(
            buffer.get_char(test_row + col(7)).unwrap(),
            create_test_char('H')
        );
        assert_eq!(
            buffer.get_char(test_row + col(8)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_erase_at_exact_boundary() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        fn setup_buffer() -> OffscreenBuffer {
            let size = width(10) + height(3);
            let mut buffer = OffscreenBuffer::new_empty(size);
            let test_row = row(0);
            let chars = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'];
            for (i, &ch) in chars.iter().enumerate() {
                buffer.set_char(test_row + col(i), create_test_char(ch));
            }
            buffer
        }

        let mut buffer = setup_buffer();
        let test_row = row(0);

        // Test erase at exact right boundary (should do nothing)
        buffer.cursor_pos = test_row + col(10);
        let result = buffer.erase_chars_at_cursor(len(1));
        assert!(!result); // Should fail - cursor is beyond valid position
        // Verify line unchanged.
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            create_test_char('J')
        );

        // Test erase at last valid position.
        buffer.cursor_pos = test_row + col(9);
        let result = buffer.erase_chars_at_cursor(len(1));
        assert!(result);
        // Verify J erased (blank), others remain
        assert_eq!(
            buffer.get_char(test_row + col(8)).unwrap(),
            create_test_char('I')
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            PixelChar::Spacer
        );

        // Test erase with large count.
        buffer = setup_buffer();
        buffer.cursor_pos = test_row + col(8);
        let result = buffer.erase_chars_at_cursor(len(5));
        assert!(result);
        // Verify only 2 chars erased.
        assert_eq!(
            buffer.get_char(test_row + col(7)).unwrap(),
            create_test_char('H')
        );
        assert_eq!(
            buffer.get_char(test_row + col(8)).unwrap(),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer.get_char(test_row + col(9)).unwrap(),
            PixelChar::Spacer
        );
    }
}

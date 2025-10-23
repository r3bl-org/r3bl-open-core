// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character operations for VT100/ANSI terminal emulation.
//!
//! This module implements character-level operations that correspond to ANSI escape
//! sequences handled by the `vt_100_ansi_parser/operations/vt_100_shim_char_ops` shim.
//! These include:
//!
//! - **ICH** (Insert Character) - [`insert_chars_at_cursor`]
//! - **DCH** (Delete Character) - [`delete_chars_at_cursor`]
//! - **ECH** (Erase Character) - [`erase_chars_at_cursor`]
//! - **Print Character** - [`print_char`] (printable character handling with VT100
//!   features)
//!
//! All operations maintain VT100 compliance and handle proper character shifting,
//! bounds checking, and cursor positioning as specified in VT100 documentation.
//!
//! This module implements the business logic for character operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See [parser module docs](crate::core::pty_mux::vt_100_ansi_parser)
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//! - **Shim**: [`char_ops`] - Parameter translation and delegation (no direct tests)
//! - **Integration Tests**: [`test_char_ops`] - Full ANSI pipeline testing
//!
//! [`insert_chars_at_cursor`]: crate::OffscreenBuffer::insert_chars_at_cursor
//! [`delete_chars_at_cursor`]: crate::OffscreenBuffer::delete_chars_at_cursor
//! [`erase_chars_at_cursor`]: crate::OffscreenBuffer::erase_chars_at_cursor
//! [`print_char`]: crate::OffscreenBuffer::print_char
//! [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::vt_100_shim_char_ops
//! [`test_char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::vt_100_test_char_ops

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ColIndex, Length, NumericValue,
            RowIndex, col,
            core::coordinates::bounds_check::{CursorBoundsCheck, LengthOps,
                                              RangeBoundsExt, RangeConvertExt},
            height, width};

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
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor position is out of bounds or if the operation
    /// fails.
    pub fn insert_chars_at_cursor(&mut self, how_many: Length) -> miette::Result<()> {
        let at = self.cursor_pos;
        let max_width = self.window_size.col_width;

        // Nothing to insert if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Calculate how many characters we can actually insert.
        let how_many_clamped = how_many.clamp_to_max(max_width.remaining_from(at));

        // Exit early if nothing to insert.
        if how_many_clamped.is_zero() {
            return Err(miette::miette!("Operation failed"));
        }

        let buffer_height = height(self.buffer.len());
        if buffer_height.is_overflowed_by(at) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        let Some(line) = self.buffer.get_mut(at.row_index.as_usize()) else {
            return Err(miette::miette!("Operation failed"));
        };

        // Copy characters to the right to make room for insertion.
        // Define inclusive range: from cursor through last position that won't overflow.
        let copy_last_position = max_width.index_from_end(how_many_clamped);
        let copy_source_range_inclusive = at.col_index..=copy_last_position;

        // Convert to exclusive range for Rust's copy_within API.
        let copy_source_range = copy_source_range_inclusive
            .to_exclusive()
            .clamp_range_to(max_width);

        // Type-safe checks:
        // 1. Destination must be within bounds.
        // 2. Source range must not be empty (clamp_range_to ensures validity).
        let copy_dest_start_col = at.col_index + how_many_clamped;
        if copy_dest_start_col.overflows(max_width) == ArrayOverflowResult::Within
            && !copy_source_range.is_empty()
        {
            // Convert to usize only when accessing the buffer.
            line.copy_within(
                copy_source_range.start.as_usize()..copy_source_range.end.as_usize(),
                copy_dest_start_col.as_usize(),
            );
        }

        // Fill the cursor position with blanks using type-safe range clamping.
        let fill_end_col = copy_dest_start_col;
        let fill_range = (at.col_index..fill_end_col).clamp_range_to(max_width);
        line[fill_range.start.as_usize()..fill_range.end.as_usize()]
            .fill(PixelChar::Spacer);

        Ok(())
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
    ///           ╭────── max_width=10 (1-based) ─────╮
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
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor position is out of bounds or if the operation
    /// fails.
    pub fn delete_chars_at_cursor(&mut self, how_many: Length) -> miette::Result<()> {
        let at = self.cursor_pos;
        let max_width = self.window_size.col_width;

        // Nothing to delete if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Calculate how many characters we can actually delete.
        let how_many_clamped = how_many.clamp_to_max(max_width.remaining_from(at));

        // Exit early if nothing to delete.
        if how_many_clamped.is_zero() {
            return Err(miette::miette!("Operation failed"));
        }

        let buffer_height = height(self.buffer.len());
        if buffer_height.is_overflowed_by(at) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Copy characters from the right, overwriting the characters at cursor (this IS
        // the deletion). Use CursorBoundsCheck for the exclusive end.
        let source_start = at.col_index + how_many_clamped;
        let source_end = max_width.eol_cursor_position();
        let copy_result = self.copy_chars_within_line(
            at.row_index,
            source_start..source_end,
            at.col_index,
        );
        debug_assert!(
            copy_result.is_ok() || source_start >= source_end,
            "Failed to copy chars within line during delete_chars_at_cursor at row {:?}, source range: {:?}..{:?}",
            at.row_index,
            source_start,
            source_end
        );

        // Clear the vacated space at the end (overwriting duplicates and filling with
        // spacers). Compute inclusive index range by converting length boundaries.
        // We need to fill from (max_width - how_many_clamped + 1) through max_width.
        // Convert to length domain for arithmetic, compute, then
        // convert back to column domain.
        let fill_start_as_length = max_width - width(how_many_clamped) + width(1);
        let fill_range_inclusive =
            width(fill_start_as_length).convert_to_index()..=max_width.convert_to_index();

        // Convert to exclusive range for fill operation.
        let fill_range = fill_range_inclusive.to_exclusive();

        let fill_result =
            self.fill_char_range(at.row_index, fill_range.clone(), PixelChar::Spacer);
        debug_assert!(
            fill_result.is_ok() || fill_range.is_empty(),
            "Failed to fill char range during delete_chars_at_cursor at row {:?}, fill range: {:?}",
            at.row_index,
            fill_range
        );

        Ok(())
    }

    /// Erase characters at cursor position (for ECH - Erase Character).
    /// Characters are replaced with blanks, no shifting occurs.
    /// Returns true if the operation was successful.
    ///
    /// Example - Erasing 3 characters at cursor position.
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
    /// After erase 3 chars:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │   │   │   │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: C, D, E replaced with blanks, F-G-H-I-J remain in place (no shifting)
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor position is out of bounds or if the operation
    /// fails.
    pub fn erase_chars_at_cursor(&mut self, how_many: Length) -> miette::Result<()> {
        let at = self.cursor_pos;
        let max_width = self.window_size.col_width;

        // Nothing to erase if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Calculate how many characters we can actually erase.
        let how_many_clamped = how_many.clamp_to_max(max_width.remaining_from(at));

        // Exit early if nothing to erase.
        if how_many_clamped.is_zero() {
            return Err(miette::miette!("Operation failed"));
        }

        let buffer_height = height(self.buffer.len());
        if buffer_height.is_overflowed_by(at) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Use type-safe range clamping for consistent patterns.
        let cursor_col = at.col_index;
        let fill_end_col = cursor_col + how_many_clamped;
        let erase_range = (cursor_col..fill_end_col).clamp_range_to(max_width);
        self.fill_char_range(at.row_index, erase_range, PixelChar::Spacer)
    }

    /// Handle printable characters with character set translation, bounds checking, and
    /// line wrapping.
    ///
    /// This method consolidates all character printing logic including:
    /// - DEC graphics character translation
    /// - Bounds checking
    /// - Character writing to buffer
    /// - DECAWM (Auto Wrap Mode) line wrap handling
    ///
    /// # Arguments
    /// * `ch` - The character to print
    ///
    /// # Behavior
    /// 1. Applies character set translation if in graphics mode
    /// 2. Writes character to buffer at current cursor position (if within bounds)
    /// 3. Advances cursor, handling line wrap based on DECAWM mode
    ///
    /// # Line Wrapping
    /// - **DECAWM enabled** (default): wraps to next line when reaching right margin
    /// - **DECAWM disabled**: cursor stays at right margin, new chars overwrite
    ///
    /// # Returns
    /// Returns true if the character was successfully processed (even if out of bounds),
    /// false if an internal operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the character cannot be processed or if the operation fails.
    pub fn print_char(&mut self, ch: char) -> miette::Result<()> {
        // Apply character set translation if in graphics mode.
        let display_char = match self.ansi_parser_support.character_set {
            CharacterSet::DECGraphics => Self::translate_dec_graphics(ch),
            CharacterSet::Ascii => ch,
        };

        let row_max = self.window_size.row_height;
        let col_max = self.window_size.col_width;
        let current_row = self.cursor_pos.row_index;
        let current_col = self.cursor_pos.col_index;

        // Only write if within bounds.
        if current_row.overflows(row_max) == ArrayOverflowResult::Within
            && current_col.overflows(col_max) == ArrayOverflowResult::Within
        {
            let result = self.set_char(
                current_row + current_col,
                PixelChar::PlainText {
                    display_char, // Use the translated character
                    style: self.ansi_parser_support.current_style,
                },
            );
            if result.is_err() {
                return Err(miette::miette!("Operation failed"));
            }

            // Move cursor forward.
            let new_col: ColIndex = current_col + 1;

            // Handle line wrap based on DECAWM (Auto Wrap Mode).
            if new_col.overflows(col_max) == ArrayOverflowResult::Overflowed {
                if self.ansi_parser_support.auto_wrap_mode {
                    // DECAWM enabled: wrap to next line (default behavior).
                    self.cursor_pos.col_index = col(0);
                    let next_row: RowIndex = current_row + 1;
                    if next_row.overflows(row_max) == ArrayOverflowResult::Within {
                        self.cursor_pos.row_index = next_row;
                    }
                } else {
                    // DECAWM disabled: stay at right margin (clamp cursor position).
                    self.cursor_pos.col_index = col_max.convert_to_index();
                }
            } else {
                self.cursor_pos.col_index = new_col;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests_shifting_ops {
    use super::*;
    use crate::{TuiStyle, len, row,
                test_fixtures_ofs_buf::{create_plain_test_char,
                                        create_test_buffer_with_size}};

    fn create_test_buffer() -> OffscreenBuffer {
        create_test_buffer_with_size(width(6), height(3))
    }

    fn create_test_char(ch: char) -> PixelChar { create_plain_test_char(ch) }

    fn setup_line_with_chars(
        buffer: &mut OffscreenBuffer,
        test_row: RowIndex,
        chars: &[char],
    ) {
        for (i, &ch) in chars.iter().enumerate() {
            if i < 6 {
                // Match buffer width.
                let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
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
        assert!(result.is_ok());

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
        assert!(result.is_ok());

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
        assert!(result.is_ok());

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
        assert!(result1.is_err());

        // Test with cursor position beyond line width.
        buffer.cursor_pos = row(0) + col(10);
        let result2 = buffer.insert_chars_at_cursor(len(1));
        assert!(result2.is_err());

        // Test with zero insert count.
        buffer.cursor_pos = row(0) + col(2);
        let result3 = buffer.insert_chars_at_cursor(len(0));
        assert!(result3.is_err());
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
        assert!(result.is_ok());

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

        // Try to delete 10 characters at position 1 (more than remaining space).
        buffer.cursor_pos = test_row + col(1);
        let result = buffer.delete_chars_at_cursor(len(10));
        assert!(result.is_ok());

        // Verify: "A" + "     " (BCDEF all deleted, 5 blanks at end).
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
        assert!(result.is_ok());

        // Verify: "ABCDE " (F deleted, one blank at end).
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
        assert!(result1.is_err());

        // Test with cursor position beyond line width.
        buffer.cursor_pos = row(0) + col(10);
        let result2 = buffer.delete_chars_at_cursor(len(1));
        assert!(result2.is_err());

        // Test with zero delete count.
        buffer.cursor_pos = row(0) + col(2);
        let result3 = buffer.delete_chars_at_cursor(len(0));
        assert!(result3.is_err());
    }

    #[test]
    fn test_erase_chars_at_cursor_basic() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Erase 3 characters at position 2 (erase 'C', 'D', 'E').
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.erase_chars_at_cursor(len(3));
        assert!(result.is_ok());

        // Verify: "AB" + "   " + "F" (CDE erased with blanks, F stays in place).
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

        // Try to erase 10 characters at position 1 (more than remaining space).
        buffer.cursor_pos = test_row + col(1);
        let result = buffer.erase_chars_at_cursor(len(10));
        assert!(result.is_ok());

        // Verify: "A" + "     " (BCDEF all erased with blanks).
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
        assert!(result.is_ok());

        // Verify: "ABCDE " (F erased with blank).
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
        assert!(result1.is_err());

        // Test with cursor position beyond line width.
        buffer.cursor_pos = row(0) + col(10);
        let result2 = buffer.erase_chars_at_cursor(len(1));
        assert!(result2.is_err());

        // Test with zero erase count.
        buffer.cursor_pos = row(0) + col(2);
        let result3 = buffer.erase_chars_at_cursor(len(0));
        assert!(result3.is_err());
    }

    // Additional comprehensive boundary tests for ICH, DCH, ECH operations.
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
            let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test delete at column 0 - should delete A,B and shift left.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.delete_chars_at_cursor(len(2));
        assert!(result.is_ok());

        // Verify: C,D,E,F,G,H,I,J shifted left, blanks at end.
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
            let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test insert at column 0 - should insert 2 blanks and shift right.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.insert_chars_at_cursor(len(2));
        assert!(result.is_ok());

        // Verify: 2 blanks inserted at start, A-H shifted right, I,J lost.
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
            let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test erase at column 0 - should erase A,B,C without shifting.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.erase_chars_at_cursor(len(3));
        assert!(result.is_ok());

        // Verify: A,B,C erased (blanks), D-J remain in place.
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
            let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test single char delete at middle position (delete C).
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.delete_chars_at_cursor(len(1));
        assert!(result.is_ok());

        // Verify: A,B remain, D,E shifted left, blank at end.
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
            let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test single char insert at middle position (before C).
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.insert_chars_at_cursor(len(1));
        assert!(result.is_ok());

        // Verify: A,B remain, blank inserted, C,D shifted right, E lost.
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
            let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Test single char erase at middle position (erase C).
        buffer.cursor_pos = test_row + col(2);
        let result = buffer.erase_chars_at_cursor(len(1));
        assert!(result.is_ok());

        // Verify: A,B remain, C erased (blank), D,E remain in place.
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
        let mut buffer = create_test_buffer();
        let test_row = row(0);

        // Test delete on empty line (should succeed but do nothing).
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.delete_chars_at_cursor(len(3));
        assert!(result.is_ok()); // Should succeed on spacer-filled line

        // Verify line remains empty.
        for i in 0..6 {
            // Match buffer width.
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }

        // Test insert on empty line at column 0.
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.insert_chars_at_cursor(len(3));
        assert!(result.is_ok());

        // Verify 3 blanks were inserted (line still appears empty).
        for i in 0..3 {
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }

        // Test erase on empty line (should succeed but do nothing).
        buffer.cursor_pos = test_row + col(0);
        let result = buffer.erase_chars_at_cursor(len(2));
        assert!(result.is_ok()); // Should succeed on spacer-filled line

        // Verify line remains empty.
        for i in 0..6 {
            // Match buffer width.
            assert_eq!(
                buffer.get_char(test_row + col(i)).unwrap(),
                PixelChar::Spacer
            );
        }

        // Test operations beyond content length on short line.
        let chars = ['A', 'B', 'C'];
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(test_row + col(i), create_test_char(ch));
        }

        // Try to delete at position beyond content length (but within width).
        buffer.cursor_pos = test_row + col(5);
        let result = buffer.delete_chars_at_cursor(len(1));
        assert!(result.is_ok()); // Should succeed - position is within buffer width

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
    }
}

#[cfg(test)]
mod tests_print_char {
    use super::*;
    use crate::{row, test_fixtures_ofs_buf::create_test_buffer_with_size};

    #[test]
    fn test_print_char_basic() {
        let mut buffer = create_test_buffer_with_size(width(10), height(5));

        // Set cursor position.
        buffer.cursor_pos = row(1) + col(2);

        // Print a character.
        let _unused = buffer.print_char('A');

        // Verify character was printed at cursor position.
        let printed_char = buffer.get_char(row(1) + col(2)).unwrap();
        match printed_char {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, 'A'),
            _ => panic!("Expected PlainText with 'A'"),
        }

        // Verify cursor advanced by one column.
        assert_eq!(buffer.cursor_pos, row(1) + col(3));
    }

    #[test]
    fn test_print_char_dec_graphics_mode() {
        let mut buffer = create_test_buffer_with_size(width(10), height(5));

        // Set DEC graphics character set.
        buffer.ansi_parser_support.character_set = CharacterSet::DECGraphics;

        buffer.cursor_pos = row(0) + col(0);

        // Print DEC graphics characters that should be translated.
        let _unused = buffer.print_char('q'); // Should become '─' (horizontal line)

        // Verify translation occurred.
        let printed_char = buffer.get_char(row(0) + col(0)).unwrap();
        match printed_char {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, '─'),
            _ => panic!("Expected PlainText with '─'"),
        }
    }

    #[test]
    fn test_print_char_line_wrap() {
        let mut buffer = create_test_buffer_with_size(width(5), height(3));

        // Ensure DECAWM is enabled (default).
        buffer.ansi_parser_support.auto_wrap_mode = true;

        // Position cursor at end of line (column 4 in 5-width buffer).
        buffer.cursor_pos = row(1) + col(4);

        // Print a character - should wrap to next line.
        let _unused = buffer.print_char('X');

        // Verify character was printed at end of current line.
        let printed_char = buffer.get_char(row(1) + col(4)).unwrap();
        match printed_char {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, 'X'),
            _ => panic!("Expected PlainText with 'X'"),
        }

        // Verify cursor wrapped to beginning of next line.
        assert_eq!(buffer.cursor_pos, row(2) + col(0));
    }
}

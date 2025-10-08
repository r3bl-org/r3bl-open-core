// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{OffscreenBuffer, PixelChar};
use crate::{ArrayOverflowResult, ColIndex, LengthOps, Pos, RowIndex, row};
use std::ops::Range;

/// Buffer manipulation methods - provides encapsulated access to buffer data.
impl OffscreenBuffer {
    /// Get character at position, returns None if position is out of bounds.
    #[must_use]
    pub fn get_char(&self, pos: Pos) -> Option<PixelChar> {
        // Use type-safe bounds checking before converting to usize.
        let buffer_height = crate::height(self.buffer.len());
        if buffer_height.is_overflowed_by(pos) == ArrayOverflowResult::Overflowed {
            return None;
        }

        // Convert to usize only at Vec access boundary.
        let row_idx = pos.row_index.as_usize();
        let col_idx = pos.col_index.as_usize();

        self.buffer.get(row_idx)?.get(col_idx).copied()
    }

    /// Set character at position. Automatically handles cache invalidation.
    /// Returns true if the position was valid and the character was set.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
    pub fn set_char(&mut self, pos: Pos, char: PixelChar) -> miette::Result<()> {
        // Use type-safe row validation via validation helpers
        let row_range = pos.row_index..row(pos.row_index.as_usize() + 1);
        let Some((_, _, lines)) = self.validate_row_range_mut(row_range) else {
            miette::bail!("Position out of bounds");
        };

        // Validate column within the selected line using type-safe bounds checking.
        let line_width = crate::width(lines[0].len());
        if line_width.is_overflowed_by(pos.col_index) == ArrayOverflowResult::Overflowed {
            miette::bail!("Position out of bounds");
        }

        // Safe assignment - both row and column have been validated.
        let col_idx = pos.col_index.as_usize();
        lines[0][col_idx] = char;

        // Debug assertion to verify the character was actually set.
        debug_assert_eq!(
            lines[0][col_idx], char,
            "Character assignment failed at position {pos:?}"
        );

        Ok(())
    }

    /// Fill a range of characters in a line with the specified character.
    /// Returns true if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the row or column range is out of bounds.
    pub fn fill_char_range(
        &mut self,
        row: RowIndex,
        col_range: Range<ColIndex>,
        char: PixelChar,
    ) -> miette::Result<()> {
        // Use type-safe range validation for both row and column bounds.
        let Some((start_col, end_col, line)) =
            self.validate_col_range_mut(row, col_range)
        else {
            miette::bail!("Position out of bounds");
        };

        line[start_col..end_col].fill(char);
        Ok(())
    }

    /// Copy characters within a line from source range to destination position.
    /// Returns true if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the row or any column position is out of bounds.
    pub fn copy_chars_within_line(
        &mut self,
        row: RowIndex,
        source_range: Range<ColIndex>,
        dest_start: ColIndex,
    ) -> miette::Result<()> {
        // Use type-safe range validation for both row and column bounds.
        let Some((source_start, source_end, line)) =
            self.validate_col_range_mut(row, source_range)
        else {
            miette::bail!("Position out of bounds");
        };

        // Validate destination position is within line bounds using type-safe bounds
        // checking.
        let line_width = crate::width(line.len());
        if line_width.is_overflowed_by(dest_start) == ArrayOverflowResult::Overflowed {
            miette::bail!("Position out of bounds");
        }

        // Perform the copy operation.
        line.copy_within(source_start..source_end, dest_start.as_usize());
        Ok(())
    }
}

#[cfg(test)]
mod tests_char_ops {
    use super::*;
    use crate::{TuiStyle, col, height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(5) + height(3);
        OffscreenBuffer::new_empty(size)
    }

    fn create_test_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_get_char_valid_position() {
        let mut buffer = create_test_buffer();
        let pos = row(1) + col(2);
        let test_char = create_test_char('A');

        // Set a character first.
        let _unused = buffer.set_char(pos, test_char);

        // Then get it back.
        let retrieved = buffer.get_char(pos);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_char);
    }

    #[test]
    fn test_get_char_out_of_bounds() {
        let buffer = create_test_buffer();

        // Test row out of bounds.
        let invalid_pos1 = row(10) + col(2);
        assert!(buffer.get_char(invalid_pos1).is_none());

        // Test column out of bounds.
        let invalid_pos2 = row(1) + col(10);
        assert!(buffer.get_char(invalid_pos2).is_none());

        // Test both out of bounds.
        let invalid_pos3 = row(10) + col(10);
        assert!(buffer.get_char(invalid_pos3).is_none());
    }

    #[test]
    fn test_set_char_with_cache_invalidation() {
        let mut buffer = create_test_buffer();
        let pos = row(0) + col(1);
        let test_char = create_test_char('B');

        // Verify the character was set successfully.
        let result = buffer.set_char(pos, test_char);
        assert!(result.is_ok());

        // Verify we can retrieve it.
        let retrieved = buffer.get_char(pos);
        assert_eq!(retrieved.unwrap(), test_char);
    }

    #[test]
    fn test_set_char_out_of_bounds() {
        let mut buffer = create_test_buffer();
        let test_char = create_test_char('C');

        // Test row out of bounds.
        let invalid_pos1 = row(10) + col(2);
        let result1 = buffer.set_char(invalid_pos1, test_char);
        assert!(result1.is_err());

        // Test column out of bounds.
        let invalid_pos2 = row(1) + col(10);
        let result2 = buffer.set_char(invalid_pos2, test_char);
        assert!(result2.is_err());
    }

    #[test]
    fn test_fill_char_range() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);
        let col_range = col(1)..col(4);
        let fill_char = create_test_char('X');

        // Fill the range.
        let result = buffer.fill_char_range(test_row, col_range.clone(), fill_char);
        assert!(result.is_ok());

        // Verify all characters in range were filled.
        for col_idx in 1..4 {
            let pos = test_row + col(col_idx);
            let retrieved = buffer.get_char(pos);
            assert_eq!(retrieved.unwrap(), fill_char);
        }

        // Verify characters outside range were not affected.
        let outside_pos = test_row + col(0);
        let outside_char = buffer.get_char(outside_pos);
        assert_ne!(outside_char.unwrap(), fill_char);
    }

    #[test]
    fn test_fill_char_range_invalid() {
        let mut buffer = create_test_buffer();
        let fill_char = create_test_char('Y');

        // Test with invalid row.
        let result1 = buffer.fill_char_range(row(10), col(0)..col(2), fill_char);
        assert!(result1.is_err());

        // Test with invalid column range.
        let result2 = buffer.fill_char_range(row(0), col(3)..col(10), fill_char);
        assert!(result2.is_err());

        // Test with backward range.
        let result3 = buffer.fill_char_range(row(0), col(3)..col(1), fill_char);
        assert!(result3.is_err());
    }

    #[test]
    fn test_copy_chars_within_line() {
        let mut buffer = create_test_buffer();
        let test_row = row(0);

        // Set up source characters.
        let _unused = buffer.set_char(test_row + col(1), create_test_char('A'));
        let _unused = buffer.set_char(test_row + col(2), create_test_char('B'));
        let _unused = buffer.set_char(test_row + col(3), create_test_char('C'));

        // Copy from columns 1-3 to column 0.
        let result = buffer.copy_chars_within_line(test_row, col(1)..col(3), col(0));
        assert!(result.is_ok());

        // Verify the copy was successful.
        assert_eq!(
            buffer.get_char(test_row + col(0)).unwrap(),
            create_test_char('A')
        );
        assert_eq!(
            buffer.get_char(test_row + col(1)).unwrap(),
            create_test_char('B')
        );

        // Original positions should still have their values (since we didn't overwrite
        // them).
        assert_eq!(
            buffer.get_char(test_row + col(2)).unwrap(),
            create_test_char('B')
        );
        assert_eq!(
            buffer.get_char(test_row + col(3)).unwrap(),
            create_test_char('C')
        );
    }

    #[test]
    fn test_copy_chars_within_line_invalid() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        let result1 = buffer.copy_chars_within_line(row(10), col(0)..col(2), col(3));
        assert!(result1.is_err());

        // Test with invalid source range.
        let result2 = buffer.copy_chars_within_line(row(0), col(3)..col(10), col(0));
        assert!(result2.is_err());

        // Test with invalid destination.
        let result3 = buffer.copy_chars_within_line(row(0), col(0)..col(2), col(10));
        assert!(result3.is_err());
    }
}

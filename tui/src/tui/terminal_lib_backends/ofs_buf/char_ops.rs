// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of character-level operations for [`OfsBuf`].
//!
//! This module provides methods for reading, writing, filling, and copying individual
//! characters or ranges of characters within the buffer.

use super::{CanvasStorage, OfsBuf, PixelChar};
use crate::{NarrowingCastToU16, RangeBoundsExt, RangeExclusive, RangeExt,
            RangeValidityStatus, VPCol, VPPos, VPRow, ok, vp_len};

/// Buffer manipulation methods - provides encapsulated access to buffer data.
impl<S: CanvasStorage> OfsBuf<S> {
    /// Gets character at position, returns [`None`] if position is out of bounds.
    #[must_use]
    pub fn get_char(&self, pos: VPPos) -> Option<PixelChar> {
        let row = self.get_row(pos.row_index)?;
        let cell = row.get(pos.col_index.as_usize())?;
        Some(*cell)
    }

    /// Set character at position.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the character assignment fails despite being in bounds.
    pub fn set_char(&mut self, pos: VPPos, char: PixelChar) -> miette::Result<()> {
        let Some(row_slice) = self.get_row_mut(pos.row_index) else {
            return Err(miette::miette!("Position out of bounds (row)"));
        };

        let Some(cell) = row_slice.get_mut(pos.col_index.as_usize()) else {
            return Err(miette::miette!("Position out of bounds (col)"));
        };

        *cell = char;

        ok!()
    }

    /// Fill a column range within a line with specified character.
    ///
    /// # Errors
    ///
    /// Returns an error if the row or column range is out of bounds.
    pub fn fill_char_range(
        &mut self,
        row: VPRow,
        col_range: RangeExclusive<VPCol>,
        char: PixelChar,
    ) -> miette::Result<()> {
        let Some(row_slice) = self.get_row_mut(row) else {
            return Err(miette::miette!("Row position out of bounds"));
        };

        let row_len = vp_len((row_slice.len()).as_u16_narrowing());

        if col_range.check_range_is_valid_for_length(row_len)
            != RangeValidityStatus::Valid
        {
            return Err(miette::miette!("Column position out of bounds"));
        }

        row_slice[col_range.as_usize_range()].fill(char);

        ok!()
    }

    /// Copy characters within a line from source range to destination position.
    ///
    /// # Errors
    ///
    /// Returns an error if the row or any column position is out of bounds.
    pub fn copy_chars_within_line(
        &mut self,
        row: VPRow,
        col_range: RangeExclusive<VPCol>,
        new_col_start: VPCol,
    ) -> miette::Result<()> {
        if col_range.is_empty() {
            return ok!();
        }

        let Some(row_slice) = self.get_row_mut(row) else {
            return Err(miette::miette!("Row index out of bounds"));
        };

        let row_len = vp_len((row_slice.len()).as_u16_narrowing());

        if col_range.check_range_is_valid_for_length(row_len)
            != RangeValidityStatus::Valid
        {
            return Err(miette::miette!("Column range out of bounds"));
        }

        let col_offset = col_range.end - col_range.start;
        let dest_range = new_col_start..new_col_start + col_offset;

        if dest_range.check_range_is_valid_for_length(row_len)
            != RangeValidityStatus::Valid
        {
            return Err(miette::miette!("Column range out of bounds"));
        }

        row_slice.copy_within(col_range.as_usize_range(), new_col_start.as_usize());

        ok!()
    }
}

#[cfg(test)]
mod tests_char_ops {
    use super::*;
    use crate::{OfsBufVT100, TuiStyle, vp_col, vp_height, vp_row, vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(5) + vp_height(3);
        OfsBufVT100::new_empty(size)
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
        let pos = vp_row(1) + vp_col(2);
        let test_char = create_test_char('A');

        // Set a character first.
        let _unused = buffer.set_char(pos, test_char);

        // Then get it back.
        assert_eq!(buffer.get_char(pos), Some(test_char));
    }

    #[test]
    fn test_get_char_out_of_bounds() {
        let buffer = create_test_buffer();

        // Test row out of bounds.
        let invalid_pos1 = vp_row(10) + vp_col(2);
        assert!(buffer.get_char(invalid_pos1).is_none());

        // Test column out of bounds.
        let invalid_pos2 = vp_row(1) + vp_col(10);
        assert!(buffer.get_char(invalid_pos2).is_none());

        // Test both out of bounds.
        let invalid_pos3 = vp_row(10) + vp_col(10);
        assert!(buffer.get_char(invalid_pos3).is_none());
    }

    #[test]
    fn test_set_char_valid_position() {
        let mut buffer = create_test_buffer();
        let pos = vp_row(0) + vp_col(1);
        let test_char = create_test_char('B');

        // Verify the character was set successfully.
        let result = buffer.set_char(pos, test_char);
        assert!(result.is_ok());

        // Verify we can retrieve it.
        assert_eq!(buffer.get_char(pos), Some(test_char));
    }

    #[test]
    fn test_set_char_out_of_bounds() {
        let mut buffer = create_test_buffer();
        let test_char = create_test_char('C');

        // Test row out of bounds.
        let invalid_pos1 = vp_row(10) + vp_col(2);
        let result1 = buffer.set_char(invalid_pos1, test_char);
        assert!(result1.is_err());

        // Test column out of bounds.
        let invalid_pos2 = vp_row(1) + vp_col(10);
        let result2 = buffer.set_char(invalid_pos2, test_char);
        assert!(result2.is_err());
    }

    #[test]
    fn test_fill_char_range() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);
        let col_range = vp_col(1)..vp_col(4);
        let fill_char = create_test_char('X');

        // Fill the range.
        let result = buffer.fill_char_range(test_row, col_range.clone(), fill_char);
        assert!(result.is_ok());

        // Verify all characters in range were filled.
        for col_idx in 1..4 {
            let pos = test_row + vp_col(col_idx);
            assert_eq!(buffer.get_char(pos), Some(fill_char));
        }

        // Verify characters outside range were not affected.
        let outside_pos = test_row + vp_col(0);
        assert_ne!(buffer.get_char(outside_pos), Some(fill_char));
    }

    #[test]
    fn test_fill_char_range_invalid() {
        let mut buffer = create_test_buffer();
        let fill_char = create_test_char('Y');

        // Test with invalid row.
        let result1 = buffer.fill_char_range(vp_row(10), vp_col(0)..vp_col(2), fill_char);
        assert!(result1.is_err());

        // Test with invalid column range.
        let result2 = buffer.fill_char_range(vp_row(0), vp_col(3)..vp_col(10), fill_char);
        assert!(result2.is_err());

        // Test with backward range.
        let result3 = buffer.fill_char_range(vp_row(0), vp_col(3)..vp_col(1), fill_char);
        assert!(result3.is_err());
    }

    #[test]
    fn test_copy_chars_within_line() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(0);

        // Set up source characters.
        let _unused = buffer.set_char(test_row + vp_col(1), create_test_char('A'));
        let _unused = buffer.set_char(test_row + vp_col(2), create_test_char('B'));
        let _unused = buffer.set_char(test_row + vp_col(3), create_test_char('C'));

        // Copy from columns 1-3 to column 0.
        let result =
            buffer.copy_chars_within_line(test_row, vp_col(1)..vp_col(3), vp_col(0));
        assert!(result.is_ok());

        // Verify the copy was successful.
        assert_eq!(
            buffer.get_char(test_row + vp_col(0)),
            Some(create_test_char('A'))
        );
        assert_eq!(
            buffer.get_char(test_row + vp_col(1)),
            Some(create_test_char('B'))
        );

        // Original positions should still have their values (since we didn't overwrite
        // them).
        assert_eq!(
            buffer.get_char(test_row + vp_col(2)),
            Some(create_test_char('B'))
        );
        assert_eq!(
            buffer.get_char(test_row + vp_col(3)),
            Some(create_test_char('C'))
        );
    }

    #[test]
    fn test_copy_chars_within_line_invalid() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        let result1 =
            buffer.copy_chars_within_line(vp_row(10), vp_col(0)..vp_col(2), vp_col(3));
        assert!(result1.is_err());

        // Test with invalid source range.
        let result2 =
            buffer.copy_chars_within_line(vp_row(0), vp_col(3)..vp_col(10), vp_col(0));
        assert!(result2.is_err());

        // Test with invalid destination.
        let result3 =
            buffer.copy_chars_within_line(vp_row(0), vp_col(0)..vp_col(2), vp_col(10));
        assert!(result3.is_err());
    }
}

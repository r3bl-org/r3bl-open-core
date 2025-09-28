// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line manipulation operations for VT100/ANSI terminal emulation.
//!
//! This module implements line-level operations that correspond to ANSI line
//! sequences handled by the `vt_100_ansi_parser::operations::line_ops` module. These
//! include:
//!
//! - **IL** (Insert Lines) - [`shift_lines_down`]
//! - **DL** (Delete Lines) - [`shift_lines_up`]
//! - **EL** (Erase Line) - [`clear_line`]
//!
//! [`shift_lines_down`]: crate::OffscreenBuffer::shift_lines_down
//! [`shift_lines_up`]: crate::OffscreenBuffer::shift_lines_up
//! [`clear_line`]: crate::OffscreenBuffer::clear_line
//!
//! All operations maintain VT100 compliance and handle proper line manipulation
//! within scroll regions as specified in VT100 documentation.
//!
//! This module implements the business logic for line operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See [parser module docs](crate::core::pty_mux::vt_100_ansi_parser)
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//! - **Shim**: [`line_ops`] - Parameter translation and delegation (no direct tests)
//! - **Integration Tests**: [`test_line_ops`] - Full ANSI pipeline testing
//!
//! [`line_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::line_ops
//! [`test_line_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::test_line_ops

use std::ops::Range;

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{Length, RowIndex};

impl OffscreenBuffer {
    /// Clear an entire line by filling it with blank characters.
    /// Returns true if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the row is out of bounds.
    pub fn clear_line(&mut self, row: RowIndex) -> miette::Result<()> {
        // Use type-safe row validation via validation helpers.
        let next_row = RowIndex::from(row.as_usize() + 1);
        let row_range = row..next_row;
        let Some((_, _, lines)) = self.validate_row_range_mut(row_range) else {
            miette::bail!("Operation failed");
        };

        // Safe to clear the validated line.
        lines[0].fill(PixelChar::Spacer);

        // Debug assertion to verify the line was actually cleared.
        debug_assert!(
            lines[0].iter().all(|ch| *ch == PixelChar::Spacer),
            "Line clear operation failed at row {row:?}"
        );

        Ok(())
    }

    /// Shift lines up within a range by the specified amount.
    /// Lines at the bottom of the range are filled with blank lines.
    /// Returns true if the operation was successful.
    ///
    /// Used by ANSI DL (Delete Line) and SU (Scroll Up) operations.
    ///
    /// [`is_row_range_valid()`]: crate::OffscreenBuffer::is_row_range_valid
    /// [`validate_row_range_mut()`]: crate::OffscreenBuffer::validate_row_range_mut
    ///
    /// # Errors
    ///
    /// Returns an error if the row range is invalid or out of bounds.
    pub fn shift_lines_up(
        &mut self,
        row_range: Range<RowIndex>,
        shift_by: Length,
    ) -> miette::Result<()> {
        // Use lightweight validation-only method without creating unused slice
        if !self.is_row_range_valid(row_range.clone()) {
            miette::bail!("Operation failed");
        }

        let start_idx = row_range.start.as_usize();
        let end_idx = row_range.end.as_usize();

        // Shift lines up using rotate_left for better performance
        for _ in 0..shift_by.as_usize() {
            // Use rotate_left to shift lines up efficiently
            let range_len = end_idx - start_idx;
            if range_len > 1 {
                self.buffer[start_idx..end_idx].rotate_left(1);
            }

            // Clear the bottom line (which is now at the end after rotation).
            self.buffer[end_idx.saturating_sub(1)].fill(PixelChar::Spacer);
        }

        Ok(())
    }

    /// Shift lines down within a range by the specified amount.
    /// Lines at the top of the range are filled with blank lines.
    /// Returns true if the operation was successful.
    ///
    /// Used by ANSI IL (Insert Line) and SD (Scroll Down) operations.
    ///
    /// For scrolling operations, this is also used to scroll buffer content down.
    /// The bottom line is lost, and a new empty line appears at top.
    ///
    /// [`is_row_range_valid()`]: crate::OffscreenBuffer::is_row_range_valid
    /// [`validate_row_range_mut()`]: crate::OffscreenBuffer::validate_row_range_mut
    ///
    /// # Errors
    ///
    /// Returns an error if the row range is invalid or out of bounds.
    pub fn shift_lines_down(
        &mut self,
        row_range: Range<RowIndex>,
        shift_by: Length,
    ) -> miette::Result<()> {
        // Use lightweight validation-only method without creating unused slice
        if !self.is_row_range_valid(row_range.clone()) {
            miette::bail!("Invalid row range");
        }

        let start_idx = row_range.start.as_usize();
        let end_idx = row_range.end.as_usize();

        // Shift lines down using rotate_right for better performance
        for _ in 0..shift_by.as_usize() {
            // Use rotate_right to shift lines down efficiently
            let range_len = end_idx - start_idx;
            if range_len > 1 {
                self.buffer[start_idx..end_idx].rotate_right(1);
            }

            // Clear the top line (which is now at the beginning after rotation).
            self.buffer[start_idx].fill(PixelChar::Spacer);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests_line_ops {
    use super::*;
    use crate::{col, height, len, row,
                test_fixtures_ofs_buf::{create_plain_test_char,
                                        create_test_buffer_with_size,
                                        create_test_line_with_chars},
                width};

    fn create_test_buffer() -> OffscreenBuffer {
        create_test_buffer_with_size(width(4), height(5))
    }

    fn create_test_char(ch: char) -> PixelChar { create_plain_test_char(ch) }

    fn create_test_line(chars: &[char]) -> PixelCharLine {
        create_test_line_with_chars(width(4), chars)
    }

    #[test]
    fn test_clear_line() {
        let mut buffer = create_test_buffer();
        let test_row = row(1);

        // Fill the line with test characters first.
        for col_idx in 0..4 {
            let _unused = buffer.set_char(test_row + col(col_idx), create_test_char('X'));
        }

        // Clear the line.
        let result = buffer.clear_line(test_row);
        assert!(result.is_ok());

        // Verify all characters are now spacers.
        for col_idx in 0..4 {
            let pos = test_row + col(col_idx);
            let char = buffer.get_char(pos).unwrap();
            assert_eq!(char, PixelChar::Spacer);
        }
    }

    #[test]
    fn test_clear_line_out_of_bounds() {
        let mut buffer = create_test_buffer();
        let result = buffer.clear_line(row(10)); // Out of bounds
        assert!(result.is_err());
    }

    #[test]
    fn test_shift_lines_up() {
        let mut buffer = create_test_buffer();

        // Set up initial lines.
        let _unused = buffer.set_line(row(1), create_test_line(&['A', 'A', 'A', 'A']));
        let _unused = buffer.set_line(row(2), create_test_line(&['B', 'B', 'B', 'B']));
        let _unused = buffer.set_line(row(3), create_test_line(&['C', 'C', 'C', 'C']));

        // Shift lines 1-3 up by 1.
        let result = buffer.shift_lines_up(row(1)..row(4), len(1));
        assert!(result.is_ok());

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
        let _unused = buffer.set_line(row(1), create_test_line(&['A', 'A', 'A', 'A']));
        let _unused = buffer.set_line(row(2), create_test_line(&['B', 'B', 'B', 'B']));
        let _unused = buffer.set_line(row(3), create_test_line(&['C', 'C', 'C', 'C']));

        // Shift lines 1-3 down by 1.
        let result = buffer.shift_lines_down(row(1)..row(4), len(1));
        assert!(result.is_ok());

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
}

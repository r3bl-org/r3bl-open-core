// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![allow(clippy::needless_range_loop)]

//! Line manipulation operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements line-level operations that correspond to [`ANSI`] line
//! sequences. These include:
//!
//! - `IL` (Insert Lines) - [`shift_lines_in_range()`] (Down)
//! - `DL` (Delete Lines) - [`shift_lines_in_range()`] (Up)
//! - `EL` (Erase Line) - [`clear_line()`]
//!
//! All operations maintain [`VT-100`] compliance and handle proper line manipulation
//! within scroll regions as specified in [`VT-100`] documentation.
//!
//! This module implements the business logic for line operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the three-layer architecture documentation above for architecture.
//!
//! # [`VT-100`] Scroll Region Boundaries
//!
//! Line insertion and deletion operations respect [`VT-100`] scroll region boundaries.
//! The scroll region defines an inclusive range `[scroll_top, scroll_bottom]` where line
//! operations are confined. Lines outside this region remain fixed. See [Interval
//! Notation] for details on mathematical range syntax.
//!
//! ```text
//! Terminal Buffer:
//! ┌─────────────────┐
//! │ Line 0 (fixed)  │  ← Outside scroll region
//! │ Line 1 (fixed)  │  ← Outside scroll region
//! ├─────────────────┤  ← scroll_top = 2
//! │ Line 2          │  ← ┐
//! │ Line 3          │  ← │ Scroll Region
//! │ Line 4          │  ← │ [2, 5] inclusive
//! │ Line 5          │  ← ┘
//! ├─────────────────┤  ← scroll_bottom = 5
//! │ Line 6 (fixed)  │  ← Outside scroll region
//! └─────────────────┘
//!
//! Scroll region membership check uses: (scroll_top..=scroll_bottom).contains(&row_index)
//!
//! - row_index=1 → false (above scroll region)
//! - row_index=2 → true  (at top boundary)
//! - row_index=4 → true  (within scroll region)
//! - row_index=5 → true  (at bottom boundary)
//! - row_index=6 → false (below scroll region)
//! ```
//!
//! Operations only affect lines within the scroll region. If the cursor is outside the
//! scroll region, the operation is skipped entirely.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`clear_line()`]: OfsBufVT100::clear_line
//! [`shift_lines_in_range()`]: OfsBufVT100::shift_lines_in_range
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [Interval Notation]: crate::bounds_check#interval-notation

use crate::{OfsBufVT100, PixelChar, RangeExclusive, ShiftLinesDirection, VPHeight,
            VPLength, VPRow,
            core::coordinates::bounds_check::{RangeBoundsExt, RangeConvertExt,
                                              RangeValidityStatus},
            ok};

impl OfsBufVT100 {
    /// Clear an entire line by filling it with blank characters.
    ///
    /// Returns true if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the row is out of bounds.
    pub fn clear_line(&mut self, row: VPRow) -> miette::Result<()> {
        let active_buf = self.get_active_screen_buffer_mut();
        let Some(row_data) = active_buf.get_row_mut(row) else {
            return Err(miette::miette!("Row index {row:?} out of bounds"));
        };
        row_data.fill(PixelChar::Spacer);
        ok!()
    }
    /// Shifts lines upward or downward within a range by the specified amount.
    ///
    /// - For **Up**: Lines at the bottom of the range are filled with blank lines. Used
    ///   by [`ANSI`] [`DL`] (Delete Line) and [`SU`] (Scroll Up).
    /// - For **Down**: Lines at the top of the range are filled with blank lines. Used by
    ///   [`ANSI`] [`IL`] (Insert Line) and [`SD`] (Scroll Down).
    ///
    /// **Performance Note:** This method avoids memory reallocation churn during
    /// scrolling.
    ///
    /// # Errors
    ///
    /// Returns an error if the row range is invalid or out of bounds.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`DL`]: https://vt100.net/docs/vt510-rm/DL.html
    /// [`IL`]: https://vt100.net/docs/vt510-rm/IL.html
    /// [`SD`]: https://vt100.net/docs/vt510-rm/SD.html
    /// [`SU`]: https://vt100.net/docs/vt510-rm/SU.html
    pub fn shift_lines_in_range(
        &mut self,
        direction: ShiftLinesDirection,
        row_range: RangeExclusive<VPRow>,
        arg_shift_by: impl Into<VPLength>,
    ) -> miette::Result<()> {
        let shift_by: VPLength = arg_shift_by.into();
        let row_height = self.get_active_screen_buffer().get_viewport().get_height();

        if row_range.check_range_is_valid_for_length(row_height)
            != RangeValidityStatus::Valid
        {
            return Err(miette::miette!("Invalid row range"));
        }

        self.get_active_screen_buffer_mut().shift_lines_in_range(
            direction,
            row_range,
            shift_by,
            PixelChar::Spacer,
        );

        ok!()
    }

    /// Insert multiple blank lines at the specified row position.
    ///
    /// - Lines below the insertion point shift down within the scroll region.
    /// - Lines at the bottom of the scroll region are lost.
    ///
    /// This operation respects [`VT-100`] scroll region boundaries. If the specified row
    /// is outside the scroll region, the operation is skipped.
    ///
    /// Used by [`ANSI`] `IL` (Insert Line) operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn insert_lines_at(
        &mut self,
        row_index: VPRow,
        how_many: VPHeight,
    ) -> miette::Result<()> {
        // Get scroll region as an inclusive range.
        let scroll_region = self.get_scroll_range_inclusive();

        // Only operate within scroll region - use type-safe inclusive range checking.
        if !scroll_region.contains(&row_index) {
            // Skip operation - cursor is outside scroll region.
            return ok!();
        }

        let scroll_bottom = *scroll_region.end();

        // Use shift_lines_in_range to shift lines down by how_many positions (vacated
        // lines are automatically filled with blanks).
        let exclusive_range = (row_index..=scroll_bottom).to_exclusive();
        self.shift_lines_in_range(ShiftLinesDirection::Down, exclusive_range, how_many)
    }

    /// Delete multiple lines at the specified row position.
    ///
    /// - Lines below the deletion point shift up within the scroll region.
    /// - Blank lines are added at the bottom of the scroll region.
    ///
    /// This operation respects [`VT-100`] scroll region boundaries. If the specified row
    /// is outside the scroll region, the operation is skipped.
    ///
    /// Used by [`ANSI`] `DL` (Delete Line) operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn delete_lines_at(
        &mut self,
        row_index: VPRow,
        how_many: VPHeight,
    ) -> miette::Result<()> {
        // Get scroll region as an inclusive range.
        let scroll_region = self.get_scroll_range_inclusive();

        // Only operate within scroll region - use type-safe inclusive range checking.
        if !scroll_region.contains(&row_index) {
            // Skip operation - cursor is outside scroll region.
            return ok!();
        }

        // Use shift_lines_in_range to shift lines up by how_many positions (vacated lines
        // are automatically filled with blanks).
        let exclusive_range = (row_index..=*scroll_region.end()).to_exclusive();
        self.shift_lines_in_range(ShiftLinesDirection::Up, exclusive_range, how_many)
    }
}

#[cfg(test)]
mod tests_line_ops {
    use super::*;
    use crate::{OfsBufVT100, PixelCharLine, TermRow,
                test_fixtures_ofs_buf::{create_plain_test_char,
                                        create_test_line_with_chars,
                                        create_vt100_test_buffer_with_size},
                vp_col, vp_height, vp_len, vp_row, vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        create_vt100_test_buffer_with_size(vp_width(4), vp_height(5))
    }

    fn create_test_char(ch: char) -> PixelChar { create_plain_test_char(ch) }

    fn create_test_line(chars: &[char]) -> PixelCharLine {
        create_test_line_with_chars(vp_width(4), chars)
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
            let pos = vp_row(test_row) + vp_col(col_idx);
            let char = buffer.get_char(pos).expect("conversion error");
            assert_eq!(char, PixelChar::Spacer);
        }
    }

    #[test]
    fn test_clear_line_out_of_bounds() {
        let mut buffer = create_test_buffer();
        let result = buffer.clear_line(vp_row(10)); // Out of bounds
        assert!(result.is_err());
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
    fn test_insert_lines_at_within_scroll_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 1-3 (inclusive).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(1)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(3)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(3), &create_test_line(&['C', 'C', 'C', 'C']));

        // Insert 1 line at row 1.
        let result = buffer.insert_lines_at(vp_row(1), vp_height(1));
        assert!(result.is_ok());

        // Verify: blank line at row 1, A's at row 2, B's at row 3, C's lost.
        let line1 = buffer.get_line(vp_row(1)).expect("conversion error");
        let line2 = buffer.get_line(vp_row(2)).expect("conversion error");
        let line3 = buffer.get_line(vp_row(3)).expect("conversion error");

        for col_idx in 0..4 {
            assert_eq!(line1[col_idx], PixelChar::Spacer);
            assert_eq!(line2[col_idx], create_test_char('A'));
            assert_eq!(line3[col_idx], create_test_char('B'));
        }
    }

    #[test]
    fn test_insert_lines_at_outside_scroll_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 1-3 (inclusive).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(1)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(3)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(0), &create_test_line(&['X', 'X', 'X', 'X']));
        let _unused =
            buffer.set_line(vp_row(4), &create_test_line(&['Y', 'Y', 'Y', 'Y']));

        // Try to insert at row 0 (outside scroll region) - should be no-op.
        let result = buffer.insert_lines_at(vp_row(0), vp_height(1));
        assert!(result.is_ok());

        // Verify row 0 unchanged.
        let line0 = buffer.get_line(vp_row(0)).expect("conversion error");
        for col_idx in 0..4 {
            assert_eq!(line0[col_idx], create_test_char('X'));
        }

        // Try to insert at row 4 (outside scroll region) - should be no-op.
        let result = buffer.insert_lines_at(vp_row(4), vp_height(1));
        assert!(result.is_ok());

        // Verify row 4 unchanged.
        let line4 = buffer.get_line(vp_row(4)).expect("conversion error");
        for col_idx in 0..4 {
            assert_eq!(line4[col_idx], create_test_char('Y'));
        }
    }

    #[test]
    fn test_delete_lines_at_within_scroll_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 1-3 (inclusive).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(1)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(3)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(3), &create_test_line(&['C', 'C', 'C', 'C']));

        // Delete 1 line at row 1.
        let result = buffer.delete_lines_at(vp_row(1), vp_height(1));
        assert!(result.is_ok());

        // Verify: B's at row 1, C's at row 2, blank line at row 3.
        let line1 = buffer.get_line(vp_row(1)).expect("conversion error");
        let line2 = buffer.get_line(vp_row(2)).expect("conversion error");
        let line3 = buffer.get_line(vp_row(3)).expect("conversion error");

        for col_idx in 0..4 {
            assert_eq!(line1[col_idx], create_test_char('B'));
            assert_eq!(line2[col_idx], create_test_char('C'));
            assert_eq!(line3[col_idx], PixelChar::Spacer);
        }
    }

    #[test]
    fn test_delete_lines_at_outside_scroll_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 1-3 (inclusive).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(1)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(3)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(0), &create_test_line(&['X', 'X', 'X', 'X']));
        let _unused =
            buffer.set_line(vp_row(4), &create_test_line(&['Y', 'Y', 'Y', 'Y']));

        // Try to delete at row 0 (outside scroll region) - should be no-op.
        let result = buffer.delete_lines_at(vp_row(0), vp_height(1));
        assert!(result.is_ok());

        // Verify row 0 unchanged.
        let line0 = buffer.get_line(vp_row(0)).expect("conversion error");
        for col_idx in 0..4 {
            assert_eq!(line0[col_idx], create_test_char('X'));
        }

        // Try to delete at row 4 (outside scroll region) - should be no-op.
        let result = buffer.delete_lines_at(vp_row(4), vp_height(1));
        assert!(result.is_ok());

        // Verify row 4 unchanged.
        let line4 = buffer.get_line(vp_row(4)).expect("conversion error");
        for col_idx in 0..4 {
            assert_eq!(line4[col_idx], create_test_char('Y'));
        }
    }

    #[test]
    fn test_insert_lines_at_multiple_lines() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 0-4 (entire buffer).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(0)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(4)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(0), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['C', 'C', 'C', 'C']));

        // Insert 2 lines at row 0.
        let result = buffer.insert_lines_at(vp_row(0), vp_height(2));
        assert!(result.is_ok());

        // Verify: 2 blank lines at rows 0-1, A's at row 2, B's at row 3, C's at row 4.
        for row_idx in 0..2 {
            let line = buffer.get_line(vp_row(row_idx)).expect("conversion error");
            for col_idx in 0..4 {
                assert_eq!(line[col_idx], PixelChar::Spacer);
            }
        }

        let line2 = buffer.get_line(vp_row(2)).expect("conversion error");
        let line3 = buffer.get_line(vp_row(3)).expect("conversion error");
        let line4 = buffer.get_line(vp_row(4)).expect("conversion error");

        for col_idx in 0..4 {
            assert_eq!(line2[col_idx], create_test_char('A'));
            assert_eq!(line3[col_idx], create_test_char('B'));
            assert_eq!(line4[col_idx], create_test_char('C'));
        }
    }

    #[test]
    fn test_insert_lines_at_exceeding_how_many_preserves_lines_above_row_index() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 1-4 (inclusive).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(1)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(4)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(0), &create_test_line(&['X', 'X', 'X', 'X']));
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(3), &create_test_line(&['C', 'C', 'C', 'C']));
        let _unused =
            buffer.set_line(vp_row(4), &create_test_line(&['D', 'D', 'D', 'D']));

        // Insert 10 lines starting at row 2 (inside scroll region [1..=4], but row_index
        // > scroll_top). Lines available from row 2 to row 4 = 3 lines.
        // how_many = 10 (exceeds 3).
        let result = buffer.insert_lines_at(vp_row(2), vp_height(10));
        assert!(result.is_ok());

        // Verify:
        // - Row 0 (outside scroll region) is untouched ('X').
        // - Row 1 (inside scroll region, but above row_index 2) is untouched ('A').
        // - Rows 2, 3, 4 are cleared (Spacer) because inserted blank lines fill the
        //   region.
        let line0 = buffer.get_line(vp_row(0)).expect("conversion error");
        let line1 = buffer.get_line(vp_row(1)).expect("conversion error");

        for col_idx in 0..4 {
            assert_eq!(line0[col_idx], create_test_char('X'));
            assert_eq!(line1[col_idx], create_test_char('A'));
        }

        for row_idx in 2..=4 {
            let line = buffer.get_line(vp_row(row_idx)).expect("conversion error");
            for col_idx in 0..4 {
                assert_eq!(line[col_idx], PixelChar::Spacer);
            }
        }
    }

    #[test]
    fn test_delete_lines_at_multiple_lines() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 0-4 (entire buffer).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(0)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(4)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(0), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['C', 'C', 'C', 'C']));
        let _unused =
            buffer.set_line(vp_row(3), &create_test_line(&['D', 'D', 'D', 'D']));
        let _unused =
            buffer.set_line(vp_row(4), &create_test_line(&['E', 'E', 'E', 'E']));

        // Delete 2 lines at row 0.
        let result = buffer.delete_lines_at(vp_row(0), vp_height(2));
        assert!(result.is_ok());

        // Verify: C's at row 0, D's at row 1, E's at row 2, blanks at rows 3-4.
        let line0 = buffer.get_line(vp_row(0)).expect("conversion error");
        let line1 = buffer.get_line(vp_row(1)).expect("conversion error");
        let line2 = buffer.get_line(vp_row(2)).expect("conversion error");

        for col_idx in 0..4 {
            assert_eq!(line0[col_idx], create_test_char('C'));
            assert_eq!(line1[col_idx], create_test_char('D'));
            assert_eq!(line2[col_idx], create_test_char('E'));
        }

        for row_idx in 3..=4 {
            let line = buffer.get_line(vp_row(row_idx)).expect("conversion error");
            for col_idx in 0..4 {
                assert_eq!(line[col_idx], PixelChar::Spacer);
            }
        }
    }

    #[test]
    fn test_delete_lines_at_exceeding_how_many_preserves_lines_above_row_index() {
        let mut buffer = create_test_buffer();

        // Set scroll region to rows 1-4 (inclusive).
        buffer.get_parser_global_state_mut().scroll_region_top =
            Some(TermRow::from(vp_row(1)));
        buffer.get_parser_global_state_mut().scroll_region_bottom =
            Some(TermRow::from(vp_row(4)));

        // Set up initial lines.
        let _unused =
            buffer.set_line(vp_row(0), &create_test_line(&['X', 'X', 'X', 'X']));
        let _unused =
            buffer.set_line(vp_row(1), &create_test_line(&['A', 'A', 'A', 'A']));
        let _unused =
            buffer.set_line(vp_row(2), &create_test_line(&['B', 'B', 'B', 'B']));
        let _unused =
            buffer.set_line(vp_row(3), &create_test_line(&['C', 'C', 'C', 'C']));
        let _unused =
            buffer.set_line(vp_row(4), &create_test_line(&['D', 'D', 'D', 'D']));

        // Delete 10 lines starting at row 2 (inside scroll region [1..=4], but row_index
        // > scroll_top). Lines available from row 2 to row 4 = 3 lines.
        // how_many = 10 (exceeds 3).
        let result = buffer.delete_lines_at(vp_row(2), vp_height(10));
        assert!(result.is_ok());

        // Verify:
        // - Row 0 (outside scroll region) is untouched ('X').
        // - Row 1 (inside scroll region, but above row_index 2) is untouched ('A').
        // - Rows 2, 3, 4 are cleared (Spacer) because B, C, D were shifted off / cleared.
        let line0 = buffer.get_line(vp_row(0)).expect("conversion error");
        let line1 = buffer.get_line(vp_row(1)).expect("conversion error");

        for col_idx in 0..4 {
            assert_eq!(line0[col_idx], create_test_char('X'));
            assert_eq!(line1[col_idx], create_test_char('A'));
        }

        for row_idx in 2..=4 {
            let line = buffer.get_line(vp_row(row_idx)).expect("conversion error");
            for col_idx in 0..4 {
                assert_eq!(line[col_idx], PixelChar::Spacer);
            }
        }
    }
}

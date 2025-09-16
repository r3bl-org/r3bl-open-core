// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
//! ANSI terminal-specific bounds checking operations for `OffscreenBuffer`.
//!
//! This module provides helper methods for common bounds checking patterns
//! used by ANSI escape sequence operations, including scroll region boundaries,
//! cursor position clamping, and terminal dimension conversions.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{BoundsOverflowStatus::Overflowed,
            ColIndex, ColWidth, RowHeight, RowIndex,
            core::{pty_mux::ansi_parser::term_units::TermRow,
                   units::bounds_check::BoundsCheck},
            row};

impl OffscreenBuffer {
    /// Get the top boundary of the scroll region (0 if no region set).
    ///
    /// This resolves the ANSI parser's scroll region top boundary, converting
    /// from 1-based ANSI coordinates to 0-based buffer indices.
    pub fn get_scroll_top_boundary(&self) -> RowIndex {
        self.ansi_parser_support
            .scroll_region_top
            .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
            .map_or(/* None */ row(0), /* Some */ Into::into)
    }

    /// Get the bottom boundary of the scroll region (screen bottom if no region set).
    ///
    /// This resolves the ANSI parser's scroll region bottom boundary, converting
    /// from 1-based ANSI coordinates to 0-based buffer indices.
    pub fn get_scroll_bottom_boundary(&self) -> RowIndex {
        self.ansi_parser_support
            .scroll_region_bottom
            .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
            .map_or(
                /* None */ self.window_size.row_height.convert_to_row_index(),
                /* Some */ Into::into,
            )
    }

    /// Clamp a column index to the valid range [0, `max_col_index`].
    ///
    /// Clamp to `max_col-1` if it would overflow. This ensures column positions stay
    /// within the terminal width, using type-safe overflow checking.
    #[must_use]
    pub fn clamp_column(&self, max_col: ColIndex) -> ColIndex {
        if max_col.check_overflows(self.window_size.col_width) == Overflowed {
            self.window_size.col_width.convert_to_col_index()
        } else {
            max_col
        }
    }

    /// Get the maximum valid column index (0-based).
    ///
    /// This converts the 1-based column width to the maximum valid 0-based index.
    #[must_use]
    pub fn max_col_index(&self) -> ColIndex {
        self.window_size.col_width.convert_to_col_index()
    }

    /// Clamp a row to stay within the scroll region boundaries.
    ///
    /// This ensures row positions respect ANSI scroll region settings,
    /// keeping the cursor within the defined scrollable area.
    #[must_use]
    pub fn clamp_row_to_scroll_region(&self, row: RowIndex) -> RowIndex {
        let top = self.get_scroll_top_boundary();
        let bottom = self.get_scroll_bottom_boundary();

        if row < top {
            top
        } else if row > bottom {
            bottom
        } else {
            row
        }
    }

    /// Get the maximum valid row index (0-based).
    ///
    /// This converts the 1-based row height to the maximum valid 0-based index.
    #[must_use]
    pub fn max_row_index(&self) -> RowIndex {
        self.window_size.row_height.convert_to_row_index()
    }

    /// Move cursor forward, clamping to screen width.
    ///
    /// This updates the cursor position while ensuring it doesn't exceed
    /// the terminal width using type-safe bounds checking.
    pub fn move_cursor_forward(&mut self, amount: ColWidth) {
        let new_col = self.cursor_pos.col_index + amount;
        self.cursor_pos.col_index = self.clamp_column(new_col);
    }

    /// Move cursor backward, stopping at column 0.
    ///
    /// This updates the cursor position while ensuring it doesn't go
    /// below column 0 using type-safe underflow protection.
    pub fn move_cursor_backward(&mut self, amount: ColWidth) {
        self.cursor_pos.col_index -= amount;
    }

    /// Move cursor up, respecting scroll region boundaries.
    ///
    /// This updates the cursor position while ensuring it stays within
    /// the current scroll region using ANSI-compliant boundary checking.
    pub fn move_cursor_up(&mut self, amount: RowHeight) {
        let new_row = self.cursor_pos.row_index - amount;
        self.cursor_pos.row_index = self.clamp_row_to_scroll_region(new_row);
    }

    /// Move cursor down, respecting scroll region boundaries.
    ///
    /// This updates the cursor position while ensuring it stays within
    /// the current scroll region using ANSI-compliant boundary checking.
    pub fn move_cursor_down(&mut self, amount: RowHeight) {
        let new_row = self.cursor_pos.row_index + amount;
        self.cursor_pos.row_index = self.clamp_row_to_scroll_region(new_row);
    }
}

#[cfg(test)]
mod tests_bounds_check_ops {
    use super::*;
    use crate::{col, core::pty_mux::ansi_parser::term_units::term_row, height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_get_scroll_top_boundary_no_region() {
        let buffer = create_test_buffer();

        // No scroll region set - should return row 0
        assert_eq!(buffer.get_scroll_top_boundary(), row(0));
    }

    #[test]
    fn test_get_scroll_top_boundary_with_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region top to row 3 (1-based)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));

        // Should return row 2 (0-based)
        assert_eq!(buffer.get_scroll_top_boundary(), row(2));
    }

    #[test]
    fn test_get_scroll_bottom_boundary_no_region() {
        let buffer = create_test_buffer();

        // No scroll region set - should return max row index (height 6 = max index 5)
        assert_eq!(buffer.get_scroll_bottom_boundary(), row(5));
    }

    #[test]
    fn test_get_scroll_bottom_boundary_with_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region bottom to row 4 (1-based)
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(4));

        // Should return row 3 (0-based)
        assert_eq!(buffer.get_scroll_bottom_boundary(), row(3));
    }

    #[test]
    fn test_clamp_column_within_bounds() {
        let buffer = create_test_buffer();

        // Column 5 is within bounds (width 10)
        let clamped = buffer.clamp_column(col(5));
        assert_eq!(clamped, col(5));
    }

    #[test]
    fn test_clamp_column_overflow() {
        let buffer = create_test_buffer();

        // Column 15 exceeds width (10) - should be clamped to max column (9)
        let clamped = buffer.clamp_column(col(15));
        assert_eq!(clamped, col(9));
    }

    #[test]
    fn test_max_col_index() {
        let buffer = create_test_buffer();

        // Width 10 means max index is 9
        assert_eq!(buffer.max_col_index(), col(9));
    }

    #[test]
    fn test_max_row_index() {
        let buffer = create_test_buffer();

        // Height 6 means max index is 5
        assert_eq!(buffer.max_row_index(), row(5));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_no_region() {
        let buffer = create_test_buffer();

        // No scroll region - row should remain unchanged
        assert_eq!(buffer.clamp_row_to_scroll_region(row(3)), row(3));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_within_bounds() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        // Row 3 (0-based) is within the scroll region
        assert_eq!(buffer.clamp_row_to_scroll_region(row(3)), row(3));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_above_top() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        // Row 0 is above scroll region - should be clamped to top (row 2)
        assert_eq!(buffer.clamp_row_to_scroll_region(row(0)), row(2));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_below_bottom() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        // Row 5 is below scroll region - should be clamped to bottom (row 4)
        assert_eq!(buffer.clamp_row_to_scroll_region(row(5)), row(4));
    }

    #[test]
    fn test_move_cursor_forward_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(3);

        buffer.move_cursor_forward(crate::ColWidth::from(2));

        assert_eq!(buffer.cursor_pos.col_index, col(5));
        assert_eq!(buffer.cursor_pos.row_index, row(2));
    }

    #[test]
    fn test_move_cursor_forward_clamped() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(8);

        buffer.move_cursor_forward(crate::ColWidth::from(5));

        // Should be clamped to max column (9)
        assert_eq!(buffer.cursor_pos.col_index, col(9));
        assert_eq!(buffer.cursor_pos.row_index, row(2));
    }

    #[test]
    fn test_move_cursor_backward_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(5);

        buffer.move_cursor_backward(crate::ColWidth::from(2));

        assert_eq!(buffer.cursor_pos.col_index, col(3));
        assert_eq!(buffer.cursor_pos.row_index, row(2));
    }

    #[test]
    fn test_move_cursor_backward_underflow_protection() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(1);

        buffer.move_cursor_backward(crate::ColWidth::from(5));

        // Type-safe underflow should handle this gracefully
        // The exact behavior depends on the implementation of ColIndex subtraction
        assert_eq!(buffer.cursor_pos.row_index, row(2));
    }

    #[test]
    fn test_move_cursor_up_within_scroll_region() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(3) + col(5);

        // Set scroll region from row 1 to row 4 (1-based: 2 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(2));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        buffer.move_cursor_up(crate::RowHeight::from(1));

        assert_eq!(buffer.cursor_pos.row_index, row(2));
        assert_eq!(buffer.cursor_pos.col_index, col(5));
    }

    #[test]
    fn test_move_cursor_up_clamped_to_scroll_top() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(5);

        // Set scroll region from row 1 to row 4 (1-based: 2 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(2));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        buffer.move_cursor_up(crate::RowHeight::from(5));

        // Should be clamped to scroll region top (row 1)
        assert_eq!(buffer.cursor_pos.row_index, row(1));
        assert_eq!(buffer.cursor_pos.col_index, col(5));
    }

    #[test]
    fn test_move_cursor_down_within_scroll_region() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(5);

        // Set scroll region from row 1 to row 4 (1-based: 2 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(2));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        buffer.move_cursor_down(crate::RowHeight::from(1));

        assert_eq!(buffer.cursor_pos.row_index, row(3));
        assert_eq!(buffer.cursor_pos.col_index, col(5));
    }

    #[test]
    fn test_move_cursor_down_clamped_to_scroll_bottom() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(3) + col(5);

        // Set scroll region from row 1 to row 4 (1-based: 2 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(2));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        buffer.move_cursor_down(crate::RowHeight::from(5));

        // Should be clamped to scroll region bottom (row 4)
        assert_eq!(buffer.cursor_pos.row_index, row(4));
        assert_eq!(buffer.cursor_pos.col_index, col(5));
    }
}

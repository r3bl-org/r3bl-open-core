// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control character operations for VT100/ANSI terminal emulation.
//!
//! This module implements control character handling that corresponds to ANSI control
//! sequences handled by the `vt_100_ansi_parser::operations::control_ops` module. These
//! include:
//!
//! - **BS** (Backspace) - `handle_backspace`
//! - **TAB** (Tab) - `handle_tab`
//! - **LF** (Line Feed) - `handle_line_feed`
//! - **CR** (Carriage Return) - `handle_carriage_return`
//!
//! All operations maintain VT100 compliance and handle proper cursor positioning
//! and scrolling as specified in VT100 documentation.

#[allow(clippy::wildcard_imports)]
use super::super::*;
use super::TAB_STOP_WIDTH;
use crate::{BoundsCheck, RowIndex, col,
            core::units::bounds_check::{IndexMarker, LengthMarker}};

impl OffscreenBuffer {
    /// Handle backspace control character (0x08).
    /// Moves cursor left one position if not at leftmost column.
    pub fn handle_backspace(&mut self) {
        let current_col = self.cursor_pos.col_index;
        if current_col > col(0) {
            self.cursor_pos.col_index = current_col - 1;
        }
    }

    /// Handle tab control character (0x09).
    /// Moves cursor to next 8-column tab stop boundary.
    pub fn handle_tab(&mut self) {
        let current_col = self.cursor_pos.col_index;
        let current_tab_zone = current_col.as_usize() / TAB_STOP_WIDTH;
        let next_tab_zone = current_tab_zone + 1;
        let next_tab_col = next_tab_zone * TAB_STOP_WIDTH;
        let max_col = self.window_size.col_width;

        // Clamp to max valid column index if it would overflow.
        let next_col_index = col(next_tab_col);
        self.cursor_pos.col_index = if next_col_index.overflows(max_col) {
            max_col.convert_to_index()
        } else {
            next_col_index
        };
    }

    /// Handle line feed control character (0x0A).
    /// Moves cursor down one line if not at bottom boundary.
    pub fn handle_line_feed(&mut self) {
        let max_row = self.window_size.row_height;
        let next_row: RowIndex = self.cursor_pos.row_index + 1;
        if next_row.check_overflows(max_row) == crate::BoundsOverflowStatus::Within {
            self.cursor_pos.row_index = next_row;
        }
    }

    /// Handle carriage return control character (0x0D).
    /// Moves cursor to start of current line (column 0).
    pub fn handle_carriage_return(&mut self) { self.cursor_pos.col_index = col(0); }
}

#[cfg(test)]
mod tests_control_ops {
    use super::*;
    use crate::{height, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_handle_backspace_within_line() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(5);

        buffer.handle_backspace();

        assert_eq!(buffer.cursor_pos.row_index, row(2));
        assert_eq!(buffer.cursor_pos.col_index, col(4));
    }

    #[test]
    fn test_handle_backspace_at_start_of_line() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(0);

        buffer.handle_backspace();

        // Should not move when already at leftmost column
        assert_eq!(buffer.cursor_pos.row_index, row(2));
        assert_eq!(buffer.cursor_pos.col_index, col(0));
    }

    #[test]
    fn test_handle_tab_to_next_stop() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(1) + col(3);

        buffer.handle_tab();

        // Should move to next 8-column tab stop (column 8)
        assert_eq!(buffer.cursor_pos.row_index, row(1));
        assert_eq!(buffer.cursor_pos.col_index, col(8));
    }

    #[test]
    fn test_handle_tab_at_tab_stop() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(1) + col(8);

        buffer.handle_tab();

        // Should move to next tab stop, but clamp to window width (10 cols = index 9 max)
        assert_eq!(buffer.cursor_pos.row_index, row(1));
        assert_eq!(buffer.cursor_pos.col_index, col(9)); // max index for width 10
    }

    #[test]
    fn test_handle_tab_near_right_edge() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(1) + col(9); // at right edge

        buffer.handle_tab();

        // Should clamp to window boundary
        assert_eq!(buffer.cursor_pos.row_index, row(1));
        assert_eq!(buffer.cursor_pos.col_index, col(9)); // stays at max valid index
    }

    #[test]
    fn test_handle_line_feed_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(2) + col(5);

        buffer.handle_line_feed();

        assert_eq!(buffer.cursor_pos.row_index, row(3));
        assert_eq!(buffer.cursor_pos.col_index, col(5)); // column preserved
    }

    #[test]
    fn test_handle_line_feed_at_bottom() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(5) + col(3); // bottom row for height 6

        buffer.handle_line_feed();

        // Should not move when at bottom
        assert_eq!(buffer.cursor_pos.row_index, row(5));
        assert_eq!(buffer.cursor_pos.col_index, col(3));
    }

    #[test]
    fn test_handle_carriage_return() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(3) + col(7);

        buffer.handle_carriage_return();

        assert_eq!(buffer.cursor_pos.row_index, row(3)); // row preserved
        assert_eq!(buffer.cursor_pos.col_index, col(0)); // moved to start of line
    }

    #[test]
    fn test_handle_carriage_return_already_at_start() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(3) + col(0);

        buffer.handle_carriage_return();

        // Should work correctly when already at start
        assert_eq!(buffer.cursor_pos.row_index, row(3));
        assert_eq!(buffer.cursor_pos.col_index, col(0));
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI cursor movement operations for `OffscreenBuffer`.
//!
//! This module provides methods for moving the cursor position within the buffer,
//! handling boundary conditions, scroll regions, and cursor state management
//! as required by ANSI terminal emulation standards.
//!
//! This module implements the business logic for cursor operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See [parser module docs](crate::core::pty_mux::vt_100_ansi_parser)
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//! - **Shim**: [`cursor_ops`] - Parameter translation and delegation (no direct tests)
//! - **Integration Tests**: [`test_cursor_ops`] - Full ANSI pipeline testing
//!
//! [`cursor_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::vt_100_shim_cursor_ops
//! [`test_cursor_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::vt_100_test_cursor_ops

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{ColIndex, ColWidth, Pos, RowHeight, RowIndex, col,
            core::coordinates::bounds_check::IndexOps};

impl OffscreenBuffer {
    /// Move cursor up by n lines.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Moving cursor up by 2 lines with scroll region
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A                              │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D  ← cursor (row 4, 0-based)  │ ← Move up 2 lines
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After CUU 2:
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ Line A                              │
    ///              │  2  │ Line B  ← cursor moved here         │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: Cursor moved up 2 lines, stops at scroll region boundaries
    /// ```
    pub fn cursor_up(&mut self, how_many: RowHeight) {
        let current_row = self.cursor_pos.row_index;
        let scroll_top_boundary = *self.get_scroll_range_inclusive().start();

        // Move cursor up but don't go above scroll region boundary.
        let potential_new_row = current_row - how_many;
        let new_row = potential_new_row.clamp(scroll_top_boundary, current_row);
        self.cursor_pos.row_index = new_row;
    }

    /// Move cursor down by n lines.
    /// Respects DECSTBM scroll region margins.
    pub fn cursor_down(&mut self, how_many: RowHeight) {
        let current_row = self.cursor_pos.row_index;
        let scroll_bottom_boundary = *self.get_scroll_range_inclusive().end();

        // Move cursor down but don't go below scroll region boundary.
        let potential_new_row = current_row + how_many;
        let new_row = potential_new_row.clamp(current_row, scroll_bottom_boundary);
        self.cursor_pos.row_index = new_row;
    }

    /// Move cursor forward by n columns.
    ///
    /// Example - Moving cursor forward by 3 columns
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ─────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based) → Move forward 3
    ///
    /// After CUF 3:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴───┴───┴───┴─▲─┴───┴───┴───┴───┘
    ///                               ╰ cursor moved here (col 5, 0-based)
    ///
    /// Result: Cursor moved forward 3 columns, clamped to screen width
    /// ```
    pub fn cursor_forward(&mut self, how_many: ColWidth) {
        let new_col = self.cursor_pos.col_index + how_many;
        self.cursor_pos.col_index =
            new_col.clamp_to_max_length(self.window_size.col_width);
    }

    /// Move cursor backward by n columns.
    pub fn cursor_backward(&mut self, how_many: ColWidth) {
        let current_col = self.cursor_pos.col_index;
        self.cursor_pos.col_index = current_col - how_many;
    }

    /// Set cursor position to specific row and column coordinates.
    /// Coordinates are clamped to valid screen boundaries and scroll regions.
    pub fn cursor_to_position(&mut self, row: RowIndex, col: ColIndex) {
        let scroll_region = self.get_scroll_range_inclusive();

        // Clamp row to scroll region boundaries.
        let clamped_row = row.clamp_to_range(scroll_region);

        // Clamp column to screen width.
        let new_col = col.clamp_to_max_length(self.window_size.col_width);

        self.cursor_pos = Pos {
            row_index: clamped_row,
            col_index: new_col,
        };
    }

    /// Move cursor to beginning of current line.
    pub fn cursor_to_line_start(&mut self) { self.cursor_pos.col_index = col(0); }

    /// Move cursor to beginning of next line.
    pub fn cursor_to_next_line_start(&mut self) {
        self.cursor_pos.col_index = col(0);
        self.cursor_down(crate::RowHeight::from(1));
    }

    /// Move cursor to specific column on current line.
    pub fn cursor_to_column(&mut self, target_col: ColIndex) {
        // Convert from 1-based to 0-based, clamp to buffer width.
        self.cursor_pos.col_index =
            target_col.clamp_to_max_length(self.window_size.col_width);
    }

    /// Save current cursor position for later restoration.
    pub fn save_cursor_position(&mut self) {
        self.ansi_parser_support.cursor_pos_for_esc_save_and_restore =
            Some(self.cursor_pos);
    }

    /// Restore previously saved cursor position.
    pub fn restore_cursor_position(&mut self) {
        if let Some(saved_pos) =
            self.ansi_parser_support.cursor_pos_for_esc_save_and_restore
        {
            self.cursor_pos = saved_pos;
        }
    }

    /// Move cursor to specific row on current column.
    pub fn cursor_to_row(&mut self, target_row: RowIndex) {
        // Clamp to valid range (conversion from 1-based to 0-based already done).
        let new_row = target_row.clamp_to_max_length(self.window_size.row_height);
        // Update only the row, preserve column.
        self.cursor_pos.row_index = new_row;
    }
}

#[cfg(test)]
mod tests_cursor_ops {
    use super::*;
    use crate::{height, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_cursor_up_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(3) + col(2);

        buffer.cursor_up(crate::RowHeight::from(2));

        assert_eq!(buffer.cursor_pos.row_index, row(1));
        assert_eq!(buffer.cursor_pos.col_index, col(2));
    }

    #[test]
    fn test_cursor_up_clamped_at_top() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(1) + col(2);

        buffer.cursor_up(crate::RowHeight::from(5));

        assert_eq!(buffer.cursor_pos.row_index, row(0));
    }

    #[test]
    fn test_cursor_down_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(1) + col(2);

        buffer.cursor_down(crate::RowHeight::from(2));

        assert_eq!(buffer.cursor_pos.row_index, row(3));
        assert_eq!(buffer.cursor_pos.col_index, col(2));
    }

    #[test]
    fn test_cursor_forward_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(1) + col(2);

        buffer.cursor_forward(crate::ColWidth::from(3));

        assert_eq!(buffer.cursor_pos.col_index, col(5));
        assert_eq!(buffer.cursor_pos.row_index, row(1));
    }

    #[test]
    fn test_cursor_forward_clamped_at_right() {
        let mut buffer = create_test_buffer();
        buffer.cursor_pos = row(1) + col(8);

        buffer.cursor_forward(crate::ColWidth::from(5));

        // Should be clamped to max column (9 for 0-based, width 10).
        assert_eq!(buffer.cursor_pos.col_index, col(9));
    }

    #[test]
    fn test_cursor_to_position() {
        let mut buffer = create_test_buffer();

        buffer.cursor_to_position(row(2), col(5));

        assert_eq!(buffer.cursor_pos.row_index, row(2));
        assert_eq!(buffer.cursor_pos.col_index, col(5));
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut buffer = create_test_buffer();
        let initial_pos = row(2) + col(5);
        buffer.cursor_pos = initial_pos;

        buffer.save_cursor_position();
        buffer.cursor_pos = row(4) + col(8);

        buffer.restore_cursor_position();

        assert_eq!(buffer.cursor_pos, initial_pos);
    }
}

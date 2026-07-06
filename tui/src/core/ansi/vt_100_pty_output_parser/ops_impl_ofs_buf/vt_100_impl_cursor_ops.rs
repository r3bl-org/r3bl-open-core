// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`] cursor movement operations for [`OfsBufVT100`].
//!
//! This module provides methods for moving the cursor position within the buffer,
//! handling boundary conditions, scroll regions, and cursor state management as required
//! by [`ANSI`] terminal emulation standards.
//!
//! This module implements the business logic for cursor operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code

use crate::{ColIndex, ColWidth, OfsBufVT100, Pos, RowHeight, RowIndex, col,
            core::coordinates::bounds_check::IndexOps};

impl OfsBufVT100 {
    /// Move cursor up by n lines.
    ///
    /// Respects [`DECSTBM`] scroll region margins.
    ///
    /// Example - Moving cursor up by 2 lines with scroll region
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ↓  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top
    ///              │  1  │ Line A                              │   (row 1, 0-based)
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D  ← cursor (row 4, 0-based)   │ ← Move up 2 lines
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom
    ///              ╰  5  │ Footer line (outside scroll region) │   (row 4, 0-based)
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
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    pub fn move_cursor_up(&mut self, how_many: RowHeight) {
        let current_row = self.get_cursor_pos().row_index;
        let scroll_top_boundary = *self.get_scroll_range_inclusive().start();

        // Move cursor up but don't go above scroll region boundary.
        let potential_new_row = current_row - how_many;
        let new_row = potential_new_row.clamp(scroll_top_boundary, current_row);
        self.update_cursor_pos(|pos| pos.row_index = new_row);
        self.parser_global_state.clear_pending_wrap();
    }

    /// Move cursor down by n lines.
    ///
    /// Respects [`DECSTBM`] scroll region margins.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    pub fn move_cursor_down(&mut self, how_many: RowHeight) {
        let current_row = self.get_cursor_pos().row_index;
        let scroll_bottom_boundary = *self.get_scroll_range_inclusive().end();

        // Move cursor down but don't go below scroll region boundary.
        let potential_new_row = current_row + how_many;
        let new_row = potential_new_row.clamp(current_row, scroll_bottom_boundary);
        self.update_cursor_pos(|pos| pos.row_index = new_row);
        self.parser_global_state.clear_pending_wrap();
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
    pub fn move_cursor_right(&mut self, how_many: ColWidth) {
        let new_col = self.get_cursor_pos().col_index + how_many;
        let max_col = self.ofs_buf.get_window_size().col_width;
        let clamped = new_col.clamp_to_max_length(max_col);
        self.update_cursor_pos(|pos| pos.col_index = clamped);
        self.parser_global_state.clear_pending_wrap();
    }

    /// Move cursor backward by n columns.
    pub fn move_cursor_left(&mut self, how_many: ColWidth) {
        let current_col = self.get_cursor_pos().col_index;
        self.update_cursor_pos(|pos| pos.col_index = current_col - how_many);
        self.parser_global_state.clear_pending_wrap();
    }

    /// Set cursor position to specific row and column coordinates.
    ///
    /// Coordinates are clamped to valid screen boundaries and scroll regions.
    pub fn cursor_to_position(&mut self, row: RowIndex, col: ColIndex) {
        let scroll_region = self.get_scroll_range_inclusive();

        // Clamp row to scroll region boundaries.
        let clamped_row = row.clamp_to_range(scroll_region);

        // Clamp column to screen width.
        let new_col = col.clamp_to_max_length(self.ofs_buf.get_window_size().col_width);

        self.set_cursor_pos(Pos {
            row_index: clamped_row,
            col_index: new_col,
        });
        self.parser_global_state.clear_pending_wrap();
    }

    /// Move cursor to beginning of current line.
    pub fn cursor_to_line_start(&mut self) {
        self.update_cursor_pos(|pos| pos.col_index = col(0));
        self.parser_global_state.clear_pending_wrap();
    }

    /// Move cursor to beginning of next line.
    pub fn cursor_to_next_line_start(&mut self) {
        self.update_cursor_pos(|pos| pos.col_index = col(0));
        self.move_cursor_down(crate::RowHeight::from(1));
        self.parser_global_state.clear_pending_wrap();
    }

    /// Move cursor to specific column on current line.
    pub fn cursor_to_column(&mut self, target_col: ColIndex) {
        // Convert from 1-based to 0-based, clamp to buffer width.
        let max_col = self.ofs_buf.get_window_size().col_width;
        let clamped = target_col.clamp_to_max_length(max_col);
        self.update_cursor_pos(|pos| pos.col_index = clamped);
        self.parser_global_state.clear_pending_wrap();
    }

    /// Save current cursor position for later restoration.
    pub fn save_cursor_position(&mut self) {
        self.parser_global_state.cursor_pos_for_esc_save_and_restore =
            Some(self.get_cursor_pos());
    }

    /// Restore previously saved cursor position.
    pub fn restore_cursor_position(&mut self) {
        if let Some(saved_pos) =
            self.parser_global_state.cursor_pos_for_esc_save_and_restore
        {
            self.set_cursor_pos(saved_pos);
        }
    }

    /// Move cursor to specific row on current column.
    pub fn cursor_to_row(&mut self, target_row: RowIndex) {
        let row_height = self.ofs_buf.get_window_size().row_height;
        // Clamp to valid range (conversion from 1-based to 0-based already done).
        let clamped = target_row.clamp_to_max_length(row_height);
        // Update only the row, preserve column.
        self.update_cursor_pos(|pos| pos.row_index = clamped);
        self.parser_global_state.clear_pending_wrap();
    }
}

#[cfg(test)]
mod tests_cursor_ops {
    use super::*;
    use crate::{OfsBufVT100, height, row, width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = width(10) + height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_move_cursor_up_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.set_cursor_pos(row(3) + col(2));

        buffer.move_cursor_up(crate::RowHeight::from(2));

        assert_eq!(buffer.get_cursor_pos().row_index, row(1));
        assert_eq!(buffer.get_cursor_pos().col_index, col(2));
    }

    #[test]
    fn test_move_cursor_up_clamped_at_top() {
        let mut buffer = create_test_buffer();
        buffer.set_cursor_pos(row(1) + col(2));

        buffer.move_cursor_up(crate::RowHeight::from(5));

        assert_eq!(buffer.get_cursor_pos().row_index, row(0));
    }

    #[test]
    fn test_move_cursor_down_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.set_cursor_pos(row(1) + col(2));

        buffer.move_cursor_down(crate::RowHeight::from(2));

        assert_eq!(buffer.get_cursor_pos().row_index, row(3));
        assert_eq!(buffer.get_cursor_pos().col_index, col(2));
    }

    #[test]
    fn test_move_cursor_right_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer.set_cursor_pos(row(1) + col(2));

        buffer.move_cursor_right(crate::ColWidth::from(3));

        assert_eq!(buffer.get_cursor_pos().col_index, col(5));
        assert_eq!(buffer.get_cursor_pos().row_index, row(1));
    }

    #[test]
    fn test_move_cursor_right_clamped_at_right() {
        let mut buffer = create_test_buffer();
        buffer.set_cursor_pos(row(1) + col(8));

        buffer.move_cursor_right(crate::ColWidth::from(5));

        // Should be clamped to max column (9 for 0-based, width 10).
        assert_eq!(buffer.get_cursor_pos().col_index, col(9));
    }

    #[test]
    fn test_cursor_to_position() {
        let mut buffer = create_test_buffer();

        buffer.cursor_to_position(row(2), col(5));

        assert_eq!(buffer.get_cursor_pos().row_index, row(2));
        assert_eq!(buffer.get_cursor_pos().col_index, col(5));
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut buffer = create_test_buffer();
        let initial_pos = row(2) + col(5);
        buffer.set_cursor_pos(initial_pos);

        buffer.save_cursor_position();
        buffer.set_cursor_pos(row(4) + col(8));

        buffer.restore_cursor_position();

        assert_eq!(buffer.get_cursor_pos(), initial_pos);
    }
}

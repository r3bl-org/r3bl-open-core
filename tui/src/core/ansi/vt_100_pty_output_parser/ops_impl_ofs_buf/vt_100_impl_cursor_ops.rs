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

use crate::{OfsBufVT100, VPCol, VPHeight, VPPos, VPRow, VPWidth,
            core::coordinates::bounds_check::IndexOps, vp_col, vp_height};

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
    /// After move_cursor_up(2):
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
    /// Clamping behavior at top:
    /// - If cursor is inside scroll region: clamps at `scroll_top`
    /// - If cursor is outside (above) scroll region: clamps at row 0
    /// - If cursor is outside (below) scroll region: clamps at `scroll_bottom + 1`
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    pub fn move_cursor_up(&mut self, how_many: VPHeight) {
        let current_row = self.get_active_screen_buffer().get_cursor_pos().row_index;
        let scroll_top_boundary = *self.get_scroll_range_inclusive().start();

        // Move cursor up but don't go above scroll region boundary.
        let potential_new_row = current_row - how_many;
        let new_row = potential_new_row.clamp(scroll_top_boundary, current_row);
        let mut pos = self.get_active_screen_buffer().get_cursor_pos();
        pos.row_index = new_row;
        self.get_active_screen_buffer_mut().set_cursor_pos(pos);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Move cursor down by n lines.
    ///
    /// Respects [`DECSTBM`] scroll region margins.
    ///
    /// Example - Moving cursor down by 2 lines with scroll region
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ↓  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top
    ///              │  1  │ Line A                              │   (row 1, 0-based)
    ///              │  2  │ Line B  ← cursor (row 2, 0-based)   │ ← Move down 2 lines
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom
    ///              ╰  5  │ Footer line (outside scroll region) │   (row 4, 0-based)
    ///                    └─────────────────────────────────────┘
    ///
    /// After move_cursor_down(2):
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ Line A                              │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D  ← cursor (row 4, 0-based)   │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: Cursor moved down 2 lines within scroll region
    /// ```
    ///
    /// Clamping behavior at bottom:
    /// - If cursor is inside scroll region: clamps at `scroll_bottom`
    /// - If cursor is outside (above) scroll region: clamps at `scroll_top - 1`
    /// - If cursor is outside (below) scroll region: clamps at `max_height - 1`
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    pub fn move_cursor_down(&mut self, how_many: VPHeight) {
        let current_row = self.get_active_screen_buffer().get_cursor_pos().row_index;
        let scroll_bottom_boundary = *self.get_scroll_range_inclusive().end();

        // Move cursor down but don't go below scroll region boundary.
        let potential_new_row = current_row + how_many;
        let new_row = potential_new_row.clamp(current_row, scroll_bottom_boundary);
        let mut pos = self.get_active_screen_buffer().get_cursor_pos();
        pos.row_index = new_row;
        self.get_active_screen_buffer_mut().set_cursor_pos(pos);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Move cursor forward (right) by n columns.
    ///
    /// Clamps at right margin.
    ///
    /// Example - Moving cursor forward 3 columns
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
    /// After move_cursor_right(3):
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴───┴───┴───┴─▲─┴───┴───┴───┴───┘
    ///                               ╰ cursor moved here (col 5, 0-based)
    ///
    /// Result: Cursor moved forward 3 columns, clamped to screen width
    /// ```
    pub fn move_cursor_right(&mut self, how_many: VPWidth) {
        let new_col =
            self.get_active_screen_buffer().get_cursor_pos().col_index + how_many;
        let max_col = self.get_active_screen_buffer().get_viewport().get_width();
        let clamped = new_col.clamp_to_max_length(max_col);
        let mut pos_copy = self.get_active_screen_buffer().get_cursor_pos();
        pos_copy.col_index = clamped;
        self.get_active_screen_buffer_mut().set_cursor_pos(pos_copy);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Move cursor backward (left) by n columns.
    ///
    /// Clamps at column 0.
    pub fn move_cursor_left(&mut self, how_many: VPWidth) {
        let current_col = self.get_active_screen_buffer().get_cursor_pos().col_index;
        let mut pos_copy = self.get_active_screen_buffer().get_cursor_pos();
        pos_copy.col_index = current_col - how_many;
        self.get_active_screen_buffer_mut().set_cursor_pos(pos_copy);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Set cursor position to specific row and column coordinates.
    ///
    /// Coordinates are clamped to valid screen boundaries and scroll regions.
    pub fn cursor_to_position(&mut self, row: VPRow, col: VPCol) {
        let scroll_region = self.get_scroll_range_inclusive();
        // Clamp row to scroll region boundaries.
        let clamped_row = row.clamp_to_range(scroll_region);
        // Clamp column to screen width.
        let new_col = col.clamp_to_max_length(
            self.get_active_screen_buffer().get_viewport().get_width(),
        );

        self.get_active_screen_buffer_mut().set_cursor_pos(VPPos {
            row_index: clamped_row,
            col_index: new_col,
        });
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Move cursor to beginning of current line.
    pub fn cursor_to_line_start(&mut self) {
        let mut pos_copy = self.get_active_screen_buffer().get_cursor_pos();
        pos_copy.col_index = vp_col(0);
        self.get_active_screen_buffer_mut().set_cursor_pos(pos_copy);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Move cursor to beginning of next line.
    pub fn cursor_to_next_line_start(&mut self) {
        let mut pos_copy = self.get_active_screen_buffer().get_cursor_pos();
        pos_copy.col_index = vp_col(0);
        self.get_active_screen_buffer_mut().set_cursor_pos(pos_copy);
        self.move_cursor_down(vp_height(1));
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Move cursor to specific column on current line.
    pub fn cursor_to_column(&mut self, target_col: VPCol) {
        // Convert from 1-based to 0-based, clamp to buffer width.
        let max_col = self.get_active_screen_buffer().get_viewport().get_width();
        let clamped = target_col.clamp_to_max_length(max_col);
        let mut pos_copy = self.get_active_screen_buffer().get_cursor_pos();
        pos_copy.col_index = clamped;
        self.get_active_screen_buffer_mut().set_cursor_pos(pos_copy);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Save current cursor position for later restoration.
    pub fn save_cursor_position(&mut self) {
        self.get_parser_global_state_mut()
            .cursor_pos_for_esc_save_and_restore =
            Some(self.get_active_screen_buffer().get_cursor_pos());
    }

    /// Restore previously saved cursor position.
    pub fn restore_cursor_position(&mut self) {
        if let Some(saved_pos) = self
            .get_parser_global_state_mut()
            .cursor_pos_for_esc_save_and_restore
        {
            self.get_active_screen_buffer_mut()
                .set_cursor_pos(saved_pos);
        }
    }

    /// Move cursor to specific row on current column.
    pub fn cursor_to_row(&mut self, target_row: VPRow) {
        let row_height = self.get_active_screen_buffer().get_viewport().get_height();
        // Clamp to valid range (conversion from 1-based to 0-based already done).
        let clamped = target_row.clamp_to_max_length(row_height);
        // Update only the row, preserve column.
        let mut pos_copy = self.get_active_screen_buffer().get_cursor_pos();
        pos_copy.row_index = clamped;
        self.get_active_screen_buffer_mut().set_cursor_pos(pos_copy);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }
}

#[cfg(test)]
mod tests_cursor_ops {
    use crate::{OfsBufVT100, VPWidth, vp_col, vp_height, vp_row, vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(10) + vp_height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_move_cursor_up_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_row(3) + vp_col(2));

        buffer.move_cursor_up(vp_height(2));

        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().row_index,
            vp_row(1)
        );
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().col_index,
            vp_col(2)
        );
    }

    #[test]
    fn test_move_cursor_up_clamped_at_top() {
        let mut buffer = create_test_buffer();
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_row(1) + vp_col(2));

        buffer.move_cursor_up(vp_height(5));

        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().row_index,
            vp_row(0)
        );
    }

    #[test]
    fn test_move_cursor_down_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_row(1) + vp_col(2));

        buffer.move_cursor_down(vp_height(2));

        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().row_index,
            vp_row(3)
        );
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().col_index,
            vp_col(2)
        );
    }

    #[test]
    fn test_move_cursor_right_within_bounds() {
        let mut buffer = create_test_buffer();
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_row(1) + vp_col(2));

        buffer.move_cursor_right(VPWidth::from(3));

        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().col_index,
            vp_col(5)
        );
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().row_index,
            vp_row(1)
        );
    }

    #[test]
    fn test_move_cursor_right_clamped_at_right() {
        let mut buffer = create_test_buffer();
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_row(1) + vp_col(8));

        buffer.move_cursor_right(VPWidth::from(5));

        // Should be clamped to max column (9 for 0-based, width 10).
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().col_index,
            vp_col(9)
        );
    }

    #[test]
    fn test_cursor_to_position() {
        let mut buffer = create_test_buffer();

        buffer.cursor_to_position(vp_row(2), vp_col(5));

        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().row_index,
            vp_row(2)
        );
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos().col_index,
            vp_col(5)
        );
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut buffer = create_test_buffer();
        let initial_pos = vp_row(2) + vp_col(5);
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(initial_pos);

        buffer.save_cursor_position();
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_row(4) + vp_col(8));

        buffer.restore_cursor_position();

        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos(),
            initial_pos
        );
    }
}

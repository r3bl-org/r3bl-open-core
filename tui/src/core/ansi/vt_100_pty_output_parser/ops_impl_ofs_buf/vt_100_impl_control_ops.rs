// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control character operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements control character handling that corresponds to [`ANSI`] control
//! sequences handled by the [`control_ops`] module. These include:
//!
//! - `BS` (Backspace) - [`handle_backspace`]
//! - `TAB` (Tab) - [`handle_tab`]
//! - `LF` (Line Feed) - [`handle_line_feed`]
//! - `CR` (Carriage Return) - [`handle_carriage_return`]
//!
//! All operations maintain [`VT-100`] compliance and handle proper cursor positioning and
//! scrolling as specified in [`VT-100`] documentation.
//!
//! This module implements the business logic for control operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`control_ops`]:
//!     crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_control_ops
//! [`handle_backspace`]: crate::OfsBufVT100::handle_backspace
//! [`handle_carriage_return`]: crate::OfsBufVT100::handle_carriage_return
//! [`handle_line_feed`]: crate::OfsBufVT100::handle_line_feed
//! [`handle_tab`]: crate::OfsBufVT100::handle_tab
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

use super::TAB_STOP_WIDTH;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, LengthOps, NumericValue, OfsBufVT100,
            col};

impl OfsBufVT100 {
    /// Handles backspace control character (`8` dec, `0x08` hex).
    ///
    /// Moves cursor left one position if not at leftmost column.
    pub fn handle_backspace(&mut self) {
        let current_col = self.get_cursor_pos().col_index;
        if !current_col.is_zero() {
            self.update_cursor_pos(|pos| pos.col_index = current_col - 1);
        }
        self.parser_global_state.clear_pending_wrap();
    }

    /// Handles tab control character (`9` dec, `0x09` hex).
    ///
    /// Moves cursor to next 8-column tab stop boundary.
    pub fn handle_tab(&mut self) {
        let current_col = self.get_cursor_pos().col_index;
        let max_col = self.ofs_buf.get_window_size().col_width;

        // Calculate next tab stop using type-safe operations
        let current_col_usize = current_col.as_usize(); // Convert only for division
        let current_tab_zone = current_col_usize / TAB_STOP_WIDTH;
        let next_tab_zone = current_tab_zone + 1;
        let next_tab_col_usize = next_tab_zone * TAB_STOP_WIDTH;

        // Convert back to type-safe column index
        let next_col_index = col(next_tab_col_usize);

        // Use type-safe overflow checking and clamping
        self.update_cursor_pos(|pos| {
            pos.col_index = {
                if next_col_index.overflows(max_col) == ArrayOverflowResult::Overflowed {
                    max_col.convert_to_index()
                } else {
                    next_col_index
                }
            }
        });
        self.parser_global_state.clear_pending_wrap();
    }

    /// Handles line feed control character (`10` dec, `0x0A` hex).
    ///
    /// Moves cursor down one line. If at the bottom of the scroll region, it scrolls the
    /// region up by one line.
    pub fn handle_line_feed(&mut self) {
        let _unused = self.index_down();
        self.parser_global_state.clear_pending_wrap();
    }

    /// Handles carriage return control character (`13` dec, `0x0D` hex).
    ///
    /// Moves cursor to start of current line (column 0).
    pub fn handle_carriage_return(&mut self) {
        self.update_cursor_pos(|pos| pos.col_index = col(0));
        self.parser_global_state.clear_pending_wrap();
    }
}

#[cfg(test)]
mod tests_control_ops {
    use super::*;
    use crate::{OfsBufVT100, PixelChar, TuiStyle, height, row, width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = width(10) + height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_handle_backspace_within_line() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(5));

        ofs_buf_vt_100.handle_backspace();

        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(2));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(4));
    }

    #[test]
    fn test_handle_backspace_at_start_of_line() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(0));

        ofs_buf_vt_100.handle_backspace();

        // Should not move when already at leftmost column.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(2));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(0));
    }

    #[test]
    fn test_handle_tab_to_next_stop() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(1) + col(3));

        ofs_buf_vt_100.handle_tab();

        // Should move to next 8-column tab stop (column 8).
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(1));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(8));
    }

    #[test]
    fn test_handle_tab_at_tab_stop() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(1) + col(8));

        ofs_buf_vt_100.handle_tab();

        // Should move to next tab stop, but clamp to window width (10 cols = index 9
        // max).
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(1));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(9)); // max index for width 10
    }

    #[test]
    fn test_handle_tab_near_right_edge() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(1) + col(9)); // at right edge

        ofs_buf_vt_100.handle_tab();

        // Should clamp to window boundary.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(1));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(9)); // stays at max valid index
    }

    #[test]
    fn test_handle_line_feed_within_bounds() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(2) + col(5));

        ofs_buf_vt_100.handle_line_feed();

        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(3));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(5)); // column preserved
    }

    #[test]
    fn test_handle_line_feed_at_bottom() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        let bottom = row(5); // bottom row for height 6
        let row_above = row(4);

        // Place a marker char at row above bottom.
        let _unused = ofs_buf_vt_100.set_char(
            row_above + col(0),
            PixelChar::PlainText {
                display_char: 'A',
                style: TuiStyle::default(),
            },
        );

        ofs_buf_vt_100.set_cursor_pos(bottom + col(3));

        ofs_buf_vt_100.handle_line_feed();

        // Buffer scrolled up: char from row 4 moved to row 3.
        let scrolled_up = row(3) + col(0);
        let ch = ofs_buf_vt_100.get_char(scrolled_up).unwrap();
        match ch {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, 'A'),
            _ => panic!("Expected PlainText with 'A'"),
        }

        // Cursor stays at bottom row, column preserved.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, bottom);
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(3));
    }

    #[test]
    fn test_handle_carriage_return() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(3) + col(7));

        ofs_buf_vt_100.handle_carriage_return();

        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(3)); // row preserved
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(0)); // moved to start of line
    }

    #[test]
    fn test_handle_carriage_return_already_at_start() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(row(3) + col(0));

        ofs_buf_vt_100.handle_carriage_return();

        // Should work correctly when already at start.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, row(3));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, col(0));
    }
}

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
//! [`handle_backspace`]: OfsBufVT100::handle_backspace
//! [`handle_carriage_return`]: OfsBufVT100::handle_carriage_return
//! [`handle_line_feed`]: OfsBufVT100::handle_line_feed
//! [`handle_tab`]: OfsBufVT100::handle_tab
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

use super::TAB_STOP_WIDTH;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, LengthOps, NarrowingCastToU16,
            NumericValue, OfsBufVT100, vp_col, vp_width};

impl OfsBufVT100 {
    /// Handles backspace control character (`8` dec, `0x08` hex).
    ///
    /// Moves cursor left one position if not at leftmost column.
    pub fn handle_backspace(&mut self) {
        let current_col = self.get_active_screen_buffer().get_cursor_pos().col_index;
        if !current_col.is_zero() {
            let mut pos = self.get_active_screen_buffer().get_cursor_pos();
            pos.col_index = current_col - vp_col(1);
            self.get_active_screen_buffer_mut().set_cursor_pos(pos);
        }
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Handles tab control character (`9` dec, `0x09` hex).
    ///
    /// Moves cursor to next 8-column tab stop boundary.
    pub fn handle_tab(&mut self) {
        let current_col = self.get_active_screen_buffer().get_cursor_pos().col_index;
        let max_col = vp_width(
            self.get_active_screen_buffer_mut()
                .get_viewport()
                .get_width(),
        );

        // Calculate next tab stop using type-safe operations
        let current_col_usize = current_col.as_usize(); // Convert only for division
        let current_tab_zone = current_col_usize / TAB_STOP_WIDTH;
        let next_tab_zone = current_tab_zone + 1;
        let next_tab_col_usize = next_tab_zone * TAB_STOP_WIDTH;

        // Convert back to type-safe column index
        let next_col_index = vp_col((next_tab_col_usize).as_u16_narrowing());

        let new_col =
            if next_col_index.overflows(max_col) == ArrayOverflowResult::Overflowed {
                max_col.convert_to_index()
            } else {
                next_col_index
            };

        let mut pos = self.get_active_screen_buffer().get_cursor_pos();
        pos.col_index = new_col;
        self.get_active_screen_buffer_mut().set_cursor_pos(pos);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Handles line feed control character (`10` dec, `0x0A` hex).
    ///
    /// Moves cursor down one line. If at the bottom of the scroll region, it scrolls the
    /// region up by one line.
    pub fn handle_line_feed(&mut self) {
        let _unused = self.index_down();
        self.get_parser_global_state_mut().clear_pending_wrap();
    }

    /// Handles carriage return control character (`13` dec, `0x0D` hex).
    ///
    /// Moves cursor to start of current line (column 0).
    pub fn handle_carriage_return(&mut self) {
        let mut pos = self.get_active_screen_buffer().get_cursor_pos();
        pos.col_index = vp_col(0);
        self.get_active_screen_buffer_mut().set_cursor_pos(pos);
        self.get_parser_global_state_mut().clear_pending_wrap();
    }
}

#[cfg(test)]
mod tests_control_ops {
    use crate::{OfsBufVT100, PixelChar, TuiStyle, vp_col, vp_height, vp_row, vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(10) + vp_height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_handle_backspace_within_line() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(5, 2));

        ofs_buf_vt_100.handle_backspace();

        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(2));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(4));
    }

    #[test]
    fn test_handle_backspace_at_start_of_line() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(0, 2));

        ofs_buf_vt_100.handle_backspace();

        // Should not move when already at leftmost column.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(2));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(0));
    }

    #[test]
    fn test_handle_tab_to_next_stop() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(3, 1));

        ofs_buf_vt_100.handle_tab();

        // Should move to next 8-column tab stop (column 8).
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(1));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(8));
    }

    #[test]
    fn test_handle_tab_at_tab_stop() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(8, 1));

        ofs_buf_vt_100.handle_tab();

        // Should move to next tab stop, but clamp to window width (10 cols = index 9
        // max).
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(1));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(9)); // max index for width 10
    }

    #[test]
    fn test_handle_tab_near_right_edge() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(9, 1)); // at right edge

        ofs_buf_vt_100.handle_tab();

        // Should clamp to window boundary.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(1));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(9)); // stays at max valid index
    }

    #[test]
    fn test_handle_line_feed_within_bounds() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(5, 2));

        ofs_buf_vt_100.handle_line_feed();

        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(3));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(5)); // column preserved
    }

    #[test]
    fn test_handle_line_feed_at_bottom() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        let bottom = vp_row(5); // bottom row for height 6
        let row_above = vp_row(4);

        // Place a marker char at row above bottom.
        let _unused = ofs_buf_vt_100.set_char(
            row_above + vp_col(0),
            PixelChar::PlainText {
                display_char: 'A',
                style: TuiStyle::default(),
            },
        );

        ofs_buf_vt_100.set_cursor_pos(bottom + vp_col(3));

        ofs_buf_vt_100.handle_line_feed();

        // Buffer scrolled up: char from row 4 moved to row 3.
        let scrolled_up = crate::vp_pos(0, 3);
        let ch = ofs_buf_vt_100
            .get_char(scrolled_up)
            .expect("conversion error");
        match ch {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, 'A'),
            _ => panic!("Expected PlainText with 'A'"),
        }

        // Cursor stays at bottom row, column preserved.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, bottom);
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(3));
    }

    #[test]
    fn test_handle_carriage_return() {
        let mut ofs_buf_vt_100 = create_test_buffer();
        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(7, 3));

        ofs_buf_vt_100.handle_carriage_return();

        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(3)); // row preserved
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(0)); // moved to start of line

        ofs_buf_vt_100.set_cursor_pos(crate::vp_pos(0, 3));

        ofs_buf_vt_100.handle_carriage_return();

        // Should work correctly when already at start.
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().row_index, vp_row(3));
        assert_eq!(ofs_buf_vt_100.get_cursor_pos().col_index, vp_col(0));
    }
}

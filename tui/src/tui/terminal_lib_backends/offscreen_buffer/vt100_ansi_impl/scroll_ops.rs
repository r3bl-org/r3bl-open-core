// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI vertical scrolling operations for `OffscreenBuffer`.
//!
//! This module provides methods for vertical line-based scrolling operations,
//! including index operations (IND/RI) and scroll operations (SU/SD).
//! These operations respect DECSTBM scroll region margins and handle
//! cursor positioning as required by ANSI terminal emulation standards.

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{RowHeight, len};

impl OffscreenBuffer {
    /// Move cursor down one line, scrolling the buffer if at bottom.
    /// Implements the ESC D (IND) escape sequence.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Index down at scroll region bottom triggers scroll
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A                              │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D  ← cursor at scroll_bottom   │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After IND:
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ Line B (moved up)                   │
    ///              │  2  │ Line C (moved up)                   │
    ///              │  3  │ Line D (moved up)                   │
    ///              │  4  │ (blank line)  ← cursor stays here   │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: Buffer scrolled up, cursor stays at scroll region bottom
    /// ```
    pub fn index_down(&mut self) {
        let current_row = /* 0-based */ self.cursor_pos.row_index;

        // Get bottom boundary of scroll region using helper method.
        let scroll_bottom_boundary = self.get_scroll_bottom_boundary();

        // Check if we're at the bottom of the scroll region.
        if current_row >= scroll_bottom_boundary {
            // At scroll region bottom - scroll buffer content up by one line.
            self.scroll_buffer_up();
        } else {
            // Not at scroll region bottom - just move cursor down.
            self.cursor_down(RowHeight::from(1));
        }
    }

    /// Move cursor up one line, scrolling the buffer if at top.
    /// Implements the ESC M (RI) escape sequence.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Reverse index up at scroll region top triggers scroll
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A  ← cursor at scroll_top       │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After RI:
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ (blank line)  ← cursor stays here   │
    ///              │  2  │ Line A (moved down)                 │
    ///              │  3  │ Line B (moved down)                 │
    ///              │  4  │ Line C (moved down)                 │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: Buffer scrolled down, cursor stays at scroll region top
    /// ```
    pub fn reverse_index_up(&mut self) {
        let current_row = /* 0-based */ self.cursor_pos.row_index;

        // Get top boundary of scroll region using helper method.
        let scroll_top_boundary = self.get_scroll_top_boundary();

        // Check if we're at the top of the scroll region.
        if current_row <= scroll_top_boundary {
            // At scroll region top - scroll buffer content down by one line.
            self.scroll_buffer_down();
        } else {
            // Not at scroll region top - just move cursor up.
            self.cursor_up(RowHeight::from(1));
        }
    }

    /// Scroll buffer content up by one line (for ESC D at bottom).
    /// The top line is lost, and a new empty line appears at bottom.
    /// Respects DECSTBM scroll region margins.
    /// See [`crate::OffscreenBuffer::shift_lines_up`] for detailed behavior and examples.
    pub fn scroll_buffer_up(&mut self) {
        // Get scroll region boundaries using helper methods.
        let scroll_top = self.get_scroll_top_boundary();
        let scroll_bottom = self.get_scroll_bottom_boundary();

        // Use shift_lines_up to shift lines up within the scroll region.
        self.shift_lines_up(
            {
                let start = scroll_top;
                let end = scroll_bottom + 1;
                start..end
            },
            len(1),
        );
    }

    /// Scroll buffer content down by one line (for ESC M at top).
    /// The bottom line is lost, and a new empty line appears at top.
    /// Respects DECSTBM scroll region margins.
    /// See [`crate::OffscreenBuffer::shift_lines_down`] for detailed behavior and
    /// examples.
    pub fn scroll_buffer_down(&mut self) {
        // Get scroll region boundaries using helper methods.
        let scroll_top = self.get_scroll_top_boundary();
        let scroll_bottom = self.get_scroll_bottom_boundary();

        // Use shift_lines_down to shift lines down within the scroll region.
        self.shift_lines_down(
            {
                let start = scroll_top;
                let end = scroll_bottom + 1;
                start..end
            },
            len(1),
        );
    }

    /// Handle SU (Scroll Up) - scroll display up by n lines.
    /// Multiple lines at the top are lost, new empty lines appear at bottom.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Scrolling up by 2 lines
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A (will be lost)               │
    ///              │  2  │ Line B (will be lost)               │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After scroll_up(2):
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ Line C (moved up 2)                 │
    ///              │  2  │ Line D (moved up 2)                 │
    ///              │  3  │ (blank line)                        │
    ///              │  4  │ (blank line)                        │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: 2 lines scrolled up, Lines A and B lost, 2 blank lines added at bottom
    /// ```
    pub fn scroll_up(&mut self, how_many: RowHeight) {
        for _ in 0..how_many.as_u16() {
            self.scroll_buffer_up();
        }
    }

    /// Handle SD (Scroll Down) - scroll display down by n lines.
    /// Multiple lines at the bottom are lost, new empty lines appear at top.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Scrolling down by 2 lines
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A                              │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C (will be lost)               │
    ///              │  4  │ Line D (will be lost)               │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After scroll_down(2):
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ (blank line)                        │
    ///              │  2  │ (blank line)                        │
    ///              │  3  │ Line A (moved down 2)               │
    ///              │  4  │ Line B (moved down 2)               │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: 2 lines scrolled down, Lines C and D lost, 2 blank lines added at top
    /// ```
    pub fn scroll_down(&mut self, how_many: RowHeight) {
        for _ in 0..how_many.as_u16() {
            self.scroll_buffer_down();
        }
    }
}

#[cfg(test)]
mod tests_scroll_vert_ops {
    use super::*;
    use crate::{col, core::pty_mux::vt100_ansi_parser::term_units::term_row, height,
                idx, ofs_buf_test_fixtures::assert_plain_char_at, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    fn fill_buffer_with_test_content(buffer: &mut OffscreenBuffer) {
        // Fill buffer with identifiable content:
        // Row 0: "0000000000"
        // Row 1: "1111111111"
        // Row 2: "2222222222"
        // Row 3: "3333333333"
        // Row 4: "4444444444"
        // Row 5: "5555555555"
        for row_idx in 0..6 {
            for col_idx in 0..10 {
                buffer.cursor_pos = row(row_idx) + col(col_idx);
                let index = idx(row_idx);
                buffer.print_char(char::from_digit(index.as_u32(), 10).unwrap());
            }
        }
        buffer.cursor_pos = row(0) + col(0);
    }

    #[test]
    fn test_index_down_within_scroll_region() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Position cursor at row 2, col 5
        buffer.cursor_pos = row(2) + col(5);

        buffer.index_down();

        // Cursor should move down one row
        assert_eq!(buffer.cursor_pos.row_index, row(3));
        assert_eq!(buffer.cursor_pos.col_index, col(5));

        // Content should remain unchanged
        assert_plain_char_at(&buffer, 2, 0, '2');
        assert_plain_char_at(&buffer, 3, 0, '3');
    }

    #[test]
    fn test_index_down_at_scroll_bottom_triggers_scroll() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Position cursor at bottom row (row 5)
        buffer.cursor_pos = row(5) + col(3);

        buffer.index_down();

        // Cursor should stay at bottom row
        assert_eq!(buffer.cursor_pos.row_index, row(5));
        assert_eq!(buffer.cursor_pos.col_index, col(3));

        // Buffer should have scrolled up - top line lost, new blank line at bottom
        assert_plain_char_at(&buffer, 0, 0, '1'); // Row 1 moved to row 0
        assert_plain_char_at(&buffer, 1, 0, '2'); // Row 2 moved to row 1
        assert_plain_char_at(&buffer, 4, 0, '5'); // Row 5 moved to row 4

        // New blank line at bottom (row 5) should be empty
        let bottom_char = buffer.get_char(row(5) + col(0));
        assert!(
            bottom_char.is_none()
                || matches!(bottom_char, Some(crate::PixelChar::Spacer))
        );
    }

    #[test]
    fn test_reverse_index_up_within_scroll_region() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Position cursor at row 3, col 2
        buffer.cursor_pos = row(3) + col(2);

        buffer.reverse_index_up();

        // Cursor should move up one row
        assert_eq!(buffer.cursor_pos.row_index, row(2));
        assert_eq!(buffer.cursor_pos.col_index, col(2));

        // Content should remain unchanged
        assert_plain_char_at(&buffer, 2, 0, '2');
        assert_plain_char_at(&buffer, 3, 0, '3');
    }

    #[test]
    fn test_reverse_index_up_at_scroll_top_triggers_scroll() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Position cursor at top row (row 0)
        buffer.cursor_pos = row(0) + col(7);

        buffer.reverse_index_up();

        // Cursor should stay at top row
        assert_eq!(buffer.cursor_pos.row_index, row(0));
        assert_eq!(buffer.cursor_pos.col_index, col(7));

        // Buffer should have scrolled down - bottom line lost, new blank line at top
        // New blank line at top (row 0) should be empty
        let top_char = buffer.get_char(row(0) + col(0));
        assert!(top_char.is_none() || matches!(top_char, Some(crate::PixelChar::Spacer)));

        assert_plain_char_at(&buffer, 1, 0, '0'); // Row 0 moved to row 1
        assert_plain_char_at(&buffer, 2, 0, '1'); // Row 1 moved to row 2
        assert_plain_char_at(&buffer, 5, 0, '4'); // Row 4 moved to row 5
    }

    #[test]
    fn test_scroll_up_multiple_lines() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        buffer.scroll_up(RowHeight::from(2));

        // Top 2 lines should be lost, content shifted up
        assert_plain_char_at(&buffer, 0, 0, '2'); // Row 2 moved to row 0
        assert_plain_char_at(&buffer, 1, 0, '3'); // Row 3 moved to row 1
        assert_plain_char_at(&buffer, 2, 0, '4'); // Row 4 moved to row 2
        assert_plain_char_at(&buffer, 3, 0, '5'); // Row 5 moved to row 3

        // Bottom 2 lines should be blank
        let blank_line_1 = buffer.get_char(row(4) + col(0));
        let blank_line_2 = buffer.get_char(row(5) + col(0));
        assert!(
            blank_line_1.is_none()
                || matches!(blank_line_1, Some(crate::PixelChar::Spacer))
        );
        assert!(
            blank_line_2.is_none()
                || matches!(blank_line_2, Some(crate::PixelChar::Spacer))
        );
    }

    #[test]
    fn test_scroll_down_multiple_lines() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        buffer.scroll_down(RowHeight::from(2));

        // Top 2 lines should be blank
        let blank_line_1 = buffer.get_char(row(0) + col(0));
        let blank_line_2 = buffer.get_char(row(1) + col(0));
        assert!(
            blank_line_1.is_none()
                || matches!(blank_line_1, Some(crate::PixelChar::Spacer))
        );
        assert!(
            blank_line_2.is_none()
                || matches!(blank_line_2, Some(crate::PixelChar::Spacer))
        );

        // Content should be shifted down
        assert_plain_char_at(&buffer, 2, 0, '0'); // Row 0 moved to row 2
        assert_plain_char_at(&buffer, 3, 0, '1'); // Row 1 moved to row 3
        assert_plain_char_at(&buffer, 4, 0, '2'); // Row 2 moved to row 4
        assert_plain_char_at(&buffer, 5, 0, '3'); // Row 3 moved to row 5
    }

    #[test]
    fn test_scroll_region_boundaries() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Set up scroll region from row 1 to row 4
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(2));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        // Position cursor at scroll region bottom
        buffer.cursor_pos = row(4) + col(0);

        buffer.index_down();

        // Only content within scroll region should have moved
        assert_plain_char_at(&buffer, 0, 0, '0'); // Row 0 unchanged (outside region)
        assert_plain_char_at(&buffer, 1, 0, '2'); // Row 2 moved to row 1
        assert_plain_char_at(&buffer, 2, 0, '3'); // Row 3 moved to row 2
        assert_plain_char_at(&buffer, 3, 0, '4'); // Row 4 moved to row 3
        assert_plain_char_at(&buffer, 5, 0, '5'); // Row 5 unchanged (outside region)

        // New blank line should appear at row 4
        let blank_char = buffer.get_char(row(4) + col(0));
        assert!(
            blank_char.is_none() || matches!(blank_char, Some(crate::PixelChar::Spacer))
        );
    }
}

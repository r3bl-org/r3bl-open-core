// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck as _, ArrayOverflowResult, CursorBoundsCheck as _,
            OfsBufVT100, PixelChar, RangeBoundsExt as _, RangeExt as _,
            glyphs::SPACER_GLYPH_CHAR, height, ok, row, width};
use std::cmp::min;

impl OfsBufVT100 {
    /// Creates a pixel character configured for erasing, correctly implementing
    /// Background Color Erase ([`BCE`]) according to the [`VT-100`]/[`xterm`]
    /// specifications.
    ///
    /// When terminal clear/erase commands (like `CSI 2 J` to clear the screen, or `EL 0`
    /// to clear a line) are executed, the erased areas are filled with space characters.
    /// According to the [`BCE`] specification, these spaces must inherit the **currently
    /// active background color**, but they must **not** inherit text attributes like
    /// underline, bold, italic, or foreground color.
    ///
    /// For example, if a shell (like [`fish`]) happens to leave the terminal in an
    /// underlined state right before issuing a [`clear`] command, the cleared screen must
    /// be filled with plain spaces (with the correct background color), not underlined
    /// spaces. This method guarantees that behavior by returning a [`PixelChar`] with a
    /// clean text style that retains only the active [`color_bg`].
    ///
    /// [`BCE`]: https://en.wikipedia.org/wiki/ANSI_escape_code#Colors
    /// [`clear`]: https://en.wikipedia.org/wiki/Clear_(Unix)
    /// [`color_bg`]: crate::TuiStyle::color_bg
    /// [`fish`]: https://fishshell.com/
    /// [`PixelChar`]: crate::PixelChar
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    /// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
    #[must_use]
    pub fn create_empty_pixel_char(&self) -> PixelChar {
        PixelChar::PlainText {
            display_char: SPACER_GLYPH_CHAR,
            style: self
                .parser_global_state
                .current_style
                .retain_bg_color_only(),
        }
    }

    /// Clears the line from the cursor to the end of the line (for `EL 0` - Erase in
    /// Line).
    ///
    /// Characters from the cursor position to the right margin are replaced with blanks.
    ///
    /// Example - Erasing from cursor (col 2) to end of line.
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ─────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ c │ d │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After erase line from cursor to end:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │   │   │   │   │   │   │   │   │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: c through J replaced with blanks.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails (though bounded safely).
    pub fn erase_line_from_cursor_to_end(&mut self) -> miette::Result<()> {
        let cursor_row = self.get_cursor_pos().row_index;
        let cursor_col = self.get_cursor_pos().col_index;
        let empty_char = self.create_empty_pixel_char();

        let row_idx_usize = cursor_row.as_usize();
        if let Some(row) = self.ofs_buf.get_row_mut(row_idx_usize) {
            let row_width = width(row.len() /* 1-based */);
            if cursor_col.overflows(row_width) == ArrayOverflowResult::Within {
                row[(cursor_col..).as_usize_range()].fill(empty_char);
            }
        }
        ok!()
    }

    /// Clears the line from the beginning of the line to the cursor (for `EL 1` - Erase
    /// in Line). Characters from the left margin up to and including the cursor position
    /// are replaced with blanks.
    ///
    /// Example - Erasing from start to cursor (col 2).
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ─────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ c │ d │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After erase line from start to cursor:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │   │   │   │ d │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: A through c replaced with blanks.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails (though bounded safely).
    pub fn erase_line_from_start_to_cursor(&mut self) -> miette::Result<()> {
        let cursor_row = self.get_cursor_pos().row_index;
        let cursor_col = self.get_cursor_pos().col_index;
        let empty_char = self.create_empty_pixel_char();

        let row_idx_usize = cursor_row.as_usize();
        if let Some(row) = self.ofs_buf.get_row_mut(row_idx_usize) {
            let row_width = width(row.len() /* 1-based */);
            let end_col = min(cursor_col.convert_to_length(), row_width);
            row[(..end_col).as_usize_range()].fill(empty_char);
        }
        ok!()
    }

    /// Clears the entire current line (for `EL 2` - Erase in Line). All characters on the
    /// current line are replaced with blanks.
    ///
    /// Example - Erasing entire line.
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ─────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ c │ d │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After erase line entire:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │   │   │   │   │   │   │   │   │   │   │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: Entire line replaced with blanks.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails (though bounded safely).
    pub fn erase_line_entire(&mut self) -> miette::Result<()> {
        let cursor_row = self.get_cursor_pos().row_index;
        let empty_char = self.create_empty_pixel_char();

        let row_idx_usize = cursor_row.as_usize();
        if let Some(row) = self.ofs_buf.get_row_mut(row_idx_usize) {
            row.fill(empty_char);
        }

        ok!()
    }

    /// Clears the display from the cursor to the end of the screen (for `ED 0` - Erase in
    /// Display). Clears from the cursor to the end of the line, and all lines below.
    ///
    /// Example - Erasing display from cursor (row 1, col 2) to end.
    ///
    /// ```text
    /// Before:
    ///           ╭─ max_width=5 ─╮
    ///           │   (1-based)   │
    /// Column:   0   1   2   3   4
    ///         ┌───┬───┬───┬───┬───┐
    /// Row:  0 │ A │ B │ C │ D │ E │
    ///         ├───┼───┼───┼───┼───┤
    /// Row:  1 │ F │ G │ h │ i │ J │
    ///         ├───┴───┴─▲─┴───┴───┤
    ///                   ╰ cursor (row 1, col 2)
    /// Row:  2 │ K │ L │ M │ N │ O │
    ///         └───┴───┴───┴───┴───┘
    ///
    /// After erase display from cursor to end:
    /// Column:   0   1   2   3   4
    ///         ┌───┬───┬───┬───┬───┐
    /// Row:  0 │ A │ B │ C │ D │ E │
    ///         ├───┼───┼───┼───┼───┤
    /// Row:  1 │ F │ G │   │   │   │
    ///         ├───┴───┴─▲─┴───┴───┤
    ///                   ╰ cursor (row 1, col 2)
    /// Row:  2 │   │   │   │   │   │
    ///         └───┴───┴───┴───┴───┘
    ///
    /// Result: Cursor to end of line 1 cleared, all of line 2 cleared.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn erase_display_from_cursor_to_end(&mut self) -> miette::Result<()> {
        self.erase_line_from_cursor_to_end()?;

        let cursor_row = self.get_cursor_pos().row_index;
        let empty_char = self.create_empty_pixel_char();

        let buffer_height =
            height(self.ofs_buf.get_height().as_usize() /* 1-based */);
        let start_row = cursor_row + 1;
        let end_row = buffer_height.eol_cursor_position();
        let range_to_clear = (start_row..end_row).clamp_range_to(buffer_height);

        for row_idx in range_to_clear.as_usize_range() {
            if let Some(row) = self.ofs_buf.get_row_mut(row_idx) {
                row.fill(empty_char);
            }
        }

        ok!()
    }

    /// Clears the display from the beginning of the screen to the cursor (for `ED 1` -
    /// Erase in Display). Clears all lines above the cursor, and from the start of the
    /// line to the cursor.
    ///
    /// Example - Erasing display from start to cursor (row 1, col 2).
    ///
    /// ```text
    /// Before:
    ///           ╭─ max_width=5 ─╮
    ///           │   (1-based)   │
    /// Column:   0   1   2   3   4
    ///         ┌───┬───┬───┬───┬───┐
    /// Row:  0 │ A │ B │ C │ D │ E │
    ///         ├───┼───┼───┼───┼───┤
    /// Row:  1 │ F │ G │ h │ i │ J │
    ///         ├───┴───┴─▲─┴───┴───┤
    ///                   ╰ cursor (row 1, col 2)
    /// Row:  2 │ K │ L │ M │ N │ O │
    ///         └───┴───┴───┴───┴───┘
    ///
    /// After erase display from start to cursor:
    /// Column:   0   1   2   3   4
    ///         ┌───┬───┬───┬───┬───┐
    /// Row:  0 │   │   │   │   │   │
    ///         ├───┼───┼───┼───┼───┤
    /// Row:  1 │   │   │   │ i │ J │
    ///         ├───┴───┴─▲─┴───┴───┤
    ///                   ╰ cursor (row 1, col 2)
    /// Row:  2 │ K │ L │ M │ N │ O │
    ///         └───┴───┴───┴───┴───┘
    ///
    /// Result: All of line 0 cleared, start to cursor of line 1 cleared.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn erase_display_from_start_to_cursor(&mut self) -> miette::Result<()> {
        let cursor_row = self.get_cursor_pos().row_index;
        let empty_char = self.create_empty_pixel_char();

        let buffer_height =
            height(self.ofs_buf.get_height().as_usize() /* 1-based */);
        let range_to_clear = (row(0)..cursor_row).clamp_range_to(buffer_height);

        for row_idx in range_to_clear.as_usize_range() {
            if let Some(row) = self.ofs_buf.get_row_mut(row_idx) {
                row.fill(empty_char);
            }
        }

        self.erase_line_from_start_to_cursor()?;

        ok!()
    }

    /// Clears the entire screen display (for `ED 2` - Erase in Display). All lines are
    /// replaced with blanks.
    ///
    /// Example - Erasing entire display.
    ///
    /// ```text
    /// Before:
    ///           ╭─ max_width=5 ─╮
    ///           │   (1-based)   │
    /// Column:   0   1   2   3   4
    ///         ┌───┬───┬───┬───┬───┐
    /// Row:  0 │ A │ B │ C │ D │ E │
    ///         ├───┼───┼───┼───┼───┤
    /// Row:  1 │ F │ G │ h │ i │ J │
    ///         ├───┴───┴─▲─┴───┴───┤
    ///                   ╰ cursor (row 1, col 2)
    /// Row:  2 │ K │ L │ M │ N │ O │
    ///         └───┴───┴───┴───┴───┘
    ///
    /// After erase display entire:
    /// Column:   0   1   2   3   4
    ///         ┌───┬───┬───┬───┬───┐
    /// Row:  0 │   │   │   │   │   │
    ///         ├───┼───┼───┼───┼───┤
    /// Row:  1 │   │   │   │   │   │
    ///         ├───┴───┴─▲─┴───┴───┤
    ///                   ╰ cursor (row 1, col 2)
    /// Row:  2 │   │   │   │   │   │
    ///         └───┴───┴───┴───┴───┘
    ///
    /// Result: Entire buffer replaced with blanks.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn erase_display_entire(&mut self) -> miette::Result<()> {
        let empty_char = self.create_empty_pixel_char();
        let height = self.ofs_buf.get_height().as_usize();
        for row_idx in 0..height {
            if let Some(row) = self.ofs_buf.get_row_mut(row_idx) {
                row.fill(empty_char);
            }
        }

        ok!()
    }

    /// Clears the entire scrollback buffer (for `ED 3` - Erase in Display).
    ///
    /// This is typically used by `clear` or `reset` commands in terminal emulators
    /// to delete the history of lines that have scrolled off the top of the screen.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn erase_display_scrollback(&mut self) -> miette::Result<()> {
        self.scrollback_buffer.clear();

        ok!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{OfsBufVT100, PixelChar, TuiStyle, col, height, row, width};

    fn create_test_buffer() -> OfsBufVT100 {
        let mut buf = OfsBufVT100::new_empty(height(3) + width(4));
        let style = TuiStyle {
            id: None,
            ..Default::default()
        };
        buf.parser_global_state.current_style = style;

        let char_x = PixelChar::PlainText {
            display_char: 'x',
            style,
        };

        // Fill buffer with 'x'
        for r in 0..3 {
            for c in 0..4 {
                buf.ofs_buf.get_row_mut(r).unwrap()[c] = char_x;
            }
        }

        // Set cursor to middle
        buf.ofs_buf.set_cursor_pos(row(1) + col(2));
        buf
    }

    fn assert_char_eq(pixel: &PixelChar, expected: char) {
        match pixel {
            PixelChar::PlainText { display_char, .. } => {
                assert_eq!(*display_char, expected);
            }
            PixelChar::Spacer if expected == ' ' => (),
            _ => panic!("Expected {expected} but got {pixel:?}"),
        }
    }

    #[test]
    fn test_erase_line_from_cursor_to_end() {
        let mut buf = create_test_buffer();
        buf.erase_line_from_cursor_to_end().unwrap();

        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[1], 'x');
        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[2], ' ');
        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[3], ' ');
    }

    #[test]
    fn test_erase_line_from_start_to_cursor() {
        let mut buf = create_test_buffer();
        buf.erase_line_from_start_to_cursor().unwrap();

        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[0], ' ');
        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[1], ' ');
        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[2], ' ');
        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[3], 'x');
    }

    #[test]
    fn test_erase_display_from_cursor_to_end() {
        let mut buf = create_test_buffer();
        buf.erase_display_from_cursor_to_end().unwrap();

        assert_char_eq(&buf.ofs_buf.get_row_mut(0).unwrap()[3], 'x');
        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[1], 'x');
        assert_char_eq(&buf.ofs_buf.get_row_mut(1).unwrap()[2], ' ');
        assert_char_eq(&buf.ofs_buf.get_row_mut(2).unwrap()[0], ' ');
        assert_char_eq(&buf.ofs_buf.get_row_mut(2).unwrap()[3], ' ');
    }

    #[test]
    fn test_bce_strips_attributes_but_keeps_bg_color() {
        use crate::{tui_color, tui_style_attrib};

        let mut buf = create_test_buffer();

        // Simulate a state where the terminal has both background color and text
        // attributes (like bold/underline).
        let active_style = TuiStyle {
            color_bg: Some(tui_color!(red)),
            color_fg: Some(tui_color!(blue)),
            attribs: crate::TuiStyleAttribs {
                bold: Some(tui_style_attrib::Bold),
                underline: Some(tui_style_attrib::Underline),
                ..Default::default()
            },
            ..Default::default()
        };
        buf.parser_global_state.current_style = active_style;

        let empty_char = buf.create_empty_pixel_char();

        if let PixelChar::PlainText {
            display_char,
            style,
        } = empty_char
        {
            // Must be a blank space
            assert_eq!(display_char, ' ');

            // BCE MANDATE: Must retain the background color
            assert_eq!(style.color_bg, Some(tui_color!(red)));

            // BCE MANDATE: Must strip all foreground colors and text attributes
            assert_eq!(style.color_fg, None);
            assert!(style.attribs.bold.is_none());
            assert!(style.attribs.underline.is_none());
        } else {
            panic!("Expected PlainText");
        }
    }

    #[test]
    fn test_erase_display_scrollback() {
        use crate::{GetMemSize, PixelCharLine, width};

        let mut buf = create_test_buffer();

        // Add some lines to the scrollback
        buf.scrollback_buffer
            .push_and_enforce_limit(PixelCharLine::new_empty(width(10)));
        buf.scrollback_buffer
            .push_and_enforce_limit(PixelCharLine::new_empty(width(10)));
        assert_eq!(buf.scrollback_buffer.lines.len(), 2);
        assert!(buf.scrollback_buffer.get_mem_size() > 0);

        // Clear the scrollback
        buf.erase_display_scrollback().unwrap();

        assert_eq!(buf.scrollback_buffer.lines.len(), 0);
        assert_eq!(buf.scrollback_buffer.cached_mem_size, 0);
    }
}

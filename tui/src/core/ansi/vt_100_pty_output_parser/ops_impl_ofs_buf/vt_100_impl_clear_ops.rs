// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck as _, ArrayOverflowResult, CursorBoundsCheck as _,
            NarrowingCastToU16, OfsBufVT100, PixelChar, RangeBoundsExt as _,
            RangeExt as _, VPHeight, VPRow, glyphs::SPACER_GLYPH_CHAR, ok, vp_height,
            vp_width};
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
    /// underlined or bold state, clear operations should not create long stretches of
    /// underlined spaces.
    ///
    /// [`BCE`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Functions-using-BCE
    /// [`fish`]: https://fishshell.com/
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    /// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
    #[must_use]
    pub fn create_empty_pixel_char(&self) -> PixelChar {
        PixelChar::PlainText {
            display_char: SPACER_GLYPH_CHAR,
            style: self
                .get_parser_global_state()
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
        let cursor_row = self.get_active_screen_buffer().get_cursor_pos().row_index;
        let cursor_col = self.get_active_screen_buffer().get_cursor_pos().col_index;
        let empty_char = self.create_empty_pixel_char();

        if let Some(row) = self.get_active_screen_buffer_mut().get_row_mut(cursor_row) {
            let row_width = vp_width((row.len()).as_u16_narrowing() /* 1-based */);
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
        let cursor_row = self.get_active_screen_buffer().get_cursor_pos().row_index;
        let cursor_col = self.get_active_screen_buffer().get_cursor_pos().col_index;
        let empty_char = self.create_empty_pixel_char();

        if let Some(row) = self.get_active_screen_buffer_mut().get_row_mut(cursor_row) {
            let row_width = vp_width((row.len()).as_u16_narrowing() /* 1-based */);
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
        let cursor_row = self.get_active_screen_buffer().get_cursor_pos().row_index;
        let empty_char = self.create_empty_pixel_char();

        if let Some(row) = self.get_active_screen_buffer_mut().get_row_mut(cursor_row) {
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

        let cursor_row = self.get_active_screen_buffer().get_cursor_pos().row_index;
        let empty_char = self.create_empty_pixel_char();

        let active_buf = self.get_active_screen_buffer_mut();
        let buffer_height: VPHeight = active_buf.get_viewport().get_height();
        let start_row = cursor_row + 1;
        let end_row = buffer_height.eol_cursor_position();
        let range_to_clear = (start_row..end_row).clamp_range_to(buffer_height);

        active_buf.fill_row_range(range_to_clear, empty_char);

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
        self.erase_line_from_start_to_cursor()?;

        let cursor_row = self.get_active_screen_buffer().get_cursor_pos().row_index;
        let empty_char = self.create_empty_pixel_char();

        let active_buf = self.get_active_screen_buffer_mut();
        let buffer_height = *active_buf.get_viewport().get_height();
        let clear_range =
            (VPRow::from(0)..cursor_row).clamp_range_to(vp_height(buffer_height));

        active_buf.fill_row_range(clear_range, empty_char);

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
        self.get_active_screen_buffer_mut()
            .clear_viewport_with(empty_char);
        ok!()
    }

    /// Clears the entire scrollback buffer (for `ED 3` - Erase in Display).
    ///
    /// This is typically used by `clear` or `reset` commands in terminal emulators to
    /// delete the history of lines that have scrolled off the top of the screen.
    ///
    /// # Design Decision for compatibility
    ///
    /// This blindly clears the *primary buffer's* scrollback, even if the terminal is
    /// currently in the alternate screen mode. As per standard [`xterm`] / `VTE` /
    /// `WezTerm` behavior, `ED 3` is a global operation that always clears the primary
    /// screen's scrollback buffer, even if the alternate screen is currently active.
    ///
    /// For more context on this behavior:
    /// - [Search `CSI Ps J` -> `Ps = 3` Xterm Control Sequences][1]
    /// - [WezTerm `ClearScrollback` documentation][2]
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    ///
    /// [1]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
    /// [2]: https://wezterm.org/config/lua/keyassignment/ClearScrollback.html
    /// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
    pub fn erase_display_scrollback(&mut self) -> miette::Result<()> {
        self.primary_buffer_mut().clear_scrollback();
        ok!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{OfsBufVT100, PixelChar, TuiStyle, TuiStyleAttribs, vp_height, vp_pos,
                vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        let mut buf = OfsBufVT100::new_empty(vp_height(3) + vp_width(4));
        let style = TuiStyle {
            id: None,
            ..Default::default()
        };
        buf.get_parser_global_state_mut().current_style = style;

        let char_x = PixelChar::PlainText {
            display_char: 'x',
            style,
        };

        // Fill buffer with 'x'
        for r in 0..3 {
            for c in 0..4 {
                buf.primary_buffer_mut()
                    .get_row_mut(r.into())
                    .expect("conversion error")[c] = char_x;
            }
        }

        // Set cursor to middle
        buf.primary_buffer_mut().set_cursor_pos(vp_pos(2, 1));
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
        buf.erase_line_from_cursor_to_end()
            .expect("conversion error");

        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[1],
            'x',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[2],
            ' ',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[3],
            ' ',
        );
    }

    #[test]
    fn test_erase_line_from_start_to_cursor() {
        let mut buf = create_test_buffer();
        buf.erase_line_from_start_to_cursor()
            .expect("conversion error");

        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[0],
            ' ',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[1],
            ' ',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[2],
            ' ',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[3],
            'x',
        );
    }

    #[test]
    fn test_erase_display_from_cursor_to_end() {
        let mut buf = create_test_buffer();
        buf.erase_display_from_cursor_to_end()
            .expect("conversion error");

        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(0u16.into())
                .expect("conversion error")[3],
            'x',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[1],
            'x',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(1u16.into())
                .expect("conversion error")[2],
            ' ',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(2u16.into())
                .expect("conversion error")[0],
            ' ',
        );
        assert_char_eq(
            &buf.primary_buffer_mut()
                .get_row_mut(2u16.into())
                .expect("conversion error")[3],
            ' ',
        );
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
            attribs: TuiStyleAttribs {
                bold: Some(tui_style_attrib::Bold),
                underline: Some(tui_style_attrib::Underline),
                ..Default::default()
            },
            ..Default::default()
        };
        buf.get_parser_global_state_mut().current_style = active_style;

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
        use crate::{GetMemSize, PixelCharLine,
                    test_fixture_growable_buffer_for_conformance_tests::TestGrowableBufferExt,
                    test_fixture_viewport::TestViewportExt, vp_width};

        let mut buf = create_test_buffer();
        let initial_len = buf.primary_buffer_mut().get_lines().len();

        // Add some lines to the scrollback
        buf.primary_buffer_mut()
            .get_lines_mut()
            .push_front(PixelCharLine::new_empty(vp_width(10), PixelChar::Spacer));
        buf.primary_buffer_mut()
            .get_lines_mut()
            .push_front(PixelCharLine::new_empty(vp_width(10), PixelChar::Spacer));
        buf.primary_buffer_mut()
            .get_viewport_mut()
            .set_history_len(2);

        assert_eq!(buf.primary_buffer_mut().get_lines().len(), initial_len + 2);
        assert!(buf.primary_buffer_mut().get_mem_size() > 0);

        // Clear the scrollback
        buf.erase_display_scrollback().expect("conversion error");

        assert_eq!(buf.primary_buffer_mut().get_lines().len(), initial_len);
    }

    #[test]
    fn test_erase_line_from_cursor_to_end_and_start_to_cursor() -> miette::Result<()> {
        use crate::{vp_col, vp_row};

        let mut buf = create_test_buffer();
        buf.set_cursor_pos(vp_col(2) + vp_row(0));

        // Fill line 0 with 'X'
        if let Some(r) = buf.get_active_screen_buffer_mut().get_row_mut(vp_row(0)) {
            r.fill(PixelChar::PlainText {
                display_char: 'X',
                style: TuiStyle::default(),
            });
        }

        // Erase from cursor (col 2) to end of line
        buf.erase_line_from_cursor_to_end()?;
        let r0 = buf.get_active_screen_buffer().get_row(vp_row(0)).unwrap();
        assert_eq!(
            r0[0],
            PixelChar::PlainText {
                display_char: 'X',
                style: TuiStyle::default()
            }
        );
        assert_eq!(
            r0[1],
            PixelChar::PlainText {
                display_char: 'X',
                style: TuiStyle::default()
            }
        );
        assert_eq!(r0[2], buf.create_empty_pixel_char());

        // Fill line 1 with 'Y' and erase from start to cursor (col 2)
        buf.set_cursor_pos(vp_col(2) + vp_row(1));
        if let Some(r) = buf.get_active_screen_buffer_mut().get_row_mut(vp_row(1)) {
            r.fill(PixelChar::PlainText {
                display_char: 'Y',
                style: TuiStyle::default(),
            });
        }
        buf.erase_line_from_start_to_cursor()?;
        let r1 = buf.get_active_screen_buffer().get_row(vp_row(1)).unwrap();
        assert_eq!(r1[0], buf.create_empty_pixel_char());
        assert_eq!(r1[1], buf.create_empty_pixel_char());
        assert_eq!(r1[2], buf.create_empty_pixel_char());
        assert_eq!(
            r1[3],
            PixelChar::PlainText {
                display_char: 'Y',
                style: TuiStyle::default()
            }
        );

        Ok(())
    }
}

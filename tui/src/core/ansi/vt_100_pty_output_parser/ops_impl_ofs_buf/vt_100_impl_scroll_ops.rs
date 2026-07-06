// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`] vertical scrolling operations for [`OfsBufVT100`].
//!
//! This module provides methods for vertical line-based scrolling operations, including
//! index operations ([`IND`]/[`RI`]) and scroll operations ([`SU`]/[`SD`]). These
//! operations respect [`DECSTBM`] scroll region margins and handle cursor positioning as
//! required by [`ANSI`] terminal emulation standards.
//!
//! This module implements the business logic for scroll operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
//! [`IND`]: https://vt100.net/docs/vt510-rm/IND.html
//! [`RI`]: https://vt100.net/docs/vt510-rm/RI.html
//! [`SD`]: https://vt100.net/docs/vt510-rm/SD.html
//! [`SU`]: https://vt100.net/docs/vt510-rm/SU.html

use crate::{ArrayBoundsCheck, ArrayUnderflowResult, LengthOps, OfsBufVT100,
            PixelCharLine, RowHeight, core::coordinates::bounds_check::RangeConvertExt,
            ok, row};

impl OfsBufVT100 {
    /// Move cursor down one line, scrolling the buffer if at bottom.
    ///
    /// Implements the `ESC D` ([`IND`]) escape sequence. Respects [`DECSTBM`] scroll
    /// region margins.
    ///
    /// Example - Index down at scroll region bottom triggers scroll
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ↓  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top
    ///              │  1  │ Line A                              │   (row 1, 0-based)
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D  ← cursor at scroll_bottom   │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom
    ///              ╰  5  │ Footer line (outside scroll region) │   (row 4, 0-based)
    ///                    └─────────────────────────────────────┘
    ///
    /// After `IND`:
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
    ///
    /// # Errors
    ///
    /// Returns an error if the scroll operation fails.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`IND`]: https://vt100.net/docs/vt510-rm/IND.html
    pub fn index_down(&mut self) -> miette::Result<()> {
        let current_row = self.get_cursor_pos().row_index;

        // Get bottom boundary of scroll region from inclusive range.
        let scroll_bottom_boundary = *self.get_scroll_range_inclusive().end();

        // Check if we're at the bottom of the scroll region.
        if current_row.underflows(scroll_bottom_boundary)
            == ArrayUnderflowResult::Underflowed
        {
            // Not at scroll region bottom - just move cursor down.
            self.move_cursor_down(RowHeight::from(1));
            ok!()
        } else {
            // At scroll region bottom - scroll buffer content up by one line.
            self.scroll_buffer_up()
        }
    }

    /// Move cursor up one line, scrolling the buffer if at top.
    ///
    /// Implements the `ESC M` ([`RI`]) escape sequence. Respects [`DECSTBM`] scroll
    /// region margins.
    ///
    /// Example - Reverse index up at scroll region top triggers scroll
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ↓  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top
    ///              │  1  │ Line A  ← cursor at scroll_top      │ (row 1, 0-based)
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom
    ///              ╰  5  │ Footer line (outside scroll region) │ (row 4, 0-based)
    ///                    └─────────────────────────────────────┘
    ///
    /// After `RI`:
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
    ///
    /// # Errors
    ///
    /// Returns an error if the scroll operation fails.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`RI`]: https://vt100.net/docs/vt510-rm/RI.html
    pub fn reverse_index_up(&mut self) -> miette::Result<()> {
        let current_row = self.get_cursor_pos().row_index;

        // Get top boundary of scroll region from inclusive range.
        let scroll_top_boundary = *self.get_scroll_range_inclusive().start();

        // Check if we're at the top of the scroll region.
        match scroll_top_boundary.underflows(current_row) {
            ArrayUnderflowResult::Underflowed => {
                // Not at scroll region top - just move cursor up.
                self.move_cursor_up(RowHeight::from(1));
                ok!()
            }
            ArrayUnderflowResult::Within => {
                // At scroll region top - scroll buffer content down by one line.
                self.scroll_buffer_down()
            }
        }
    }

    /// Scroll buffer content up by one line (for `ESC D` at bottom).
    ///
    /// The top line is lost (or saved to scrollback), and a new empty line appears at
    /// bottom.
    ///
    /// Respects [`DECSTBM`] scroll region margins.
    ///
    /// # Use Case
    ///
    /// This handles the "normal" scrolling behavior seen in daily terminal usage.
    ///
    /// ## 1. The Standard Terminal Emulator Story
    ///
    /// Imagine a user is running a standard terminal emulator (like `Wezterm`) and
    /// running a shell like `fish` or `bash` inside it. They type `ls -la /etc`,
    /// producing hundreds of lines of output. As `fish` or `bash` prints each new line
    /// (via `\n` or the [`IND`] sequence), the cursor eventually hits the bottom of the
    /// screen.
    ///
    /// The original hardware [`VT-100`] specification (1978) had no concept of a
    /// "scrollback buffer". When a line scrolled off the top of the physical screen, it
    /// was deleted from RAM forever. Shells like `bash` or `fish` still operate under
    /// this assumption: they print text blindly, expecting the terminal to discard the
    /// top line when the screen fills up.
    ///
    /// However, modern terminal emulators invented the scrollback buffer as a
    /// quality-of-life UI feature. When the screen fills up, the terminal emulator shifts
    /// all text up by one line, intercepting the evicted top line and saving it to a
    /// private history buffer so the user can scroll up later.
    ///
    /// ## 2. How [`pty_mux`] Emulates This (the code here)
    ///
    /// When someone builds a TUI app using our [`pty_mux`] module, they are essentially
    /// running our "headless" terminal emulator inside their app.
    ///
    /// When the child [`PTY`] process (like `fish` or `bash`) hits the bottom of the
    /// virtual screen and requests a new line, it is **this output parser** that acts as
    /// the terminal emulator. It shifts all text up by one line in its virtual memory
    /// canvas to make room at the bottom. The parser then captures the line that was just
    /// pushed off the top of the virtual screen and saves it into its own private
    /// scrollback history, perfectly mimicking the behavior of a standard terminal
    /// emulator.
    ///
    /// See [`shift_lines_up()`] for detailed behavior and examples.
    ///
    /// # Errors
    ///
    /// Returns an error if the scroll operation fails.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`IND`]: https://vt100.net/docs/vt510-rm/IND.html
    /// [`pty_mux`]: crate::core::pty::pty_mux
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`shift_lines_up()`]: crate::OfsBufVT100::shift_lines_up
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn scroll_buffer_up(&mut self) -> miette::Result<()> {
        // Get scroll region as an inclusive range and convert to
        // exclusive for iteration.
        let scroll_region = self.get_scroll_range_inclusive();

        // Restricted Scrolling (Not Full Screen): Terminals support a feature called
        // `DECSTBM` (Set Top and Bottom Margins), which lets a program restrict scrolling
        // to a specific "window" or sub-region of the screen (e.g., only scrolling lines
        // 5 through 15).
        // - Complex TUI apps like vim, tmux, or htop use this heavily. They might keep a
        //   status bar locked at the bottom and a file header locked at the top, while
        //   only the text in the middle scrolls.
        // - When a restricted region scrolls, a line is still pushed out of that region,
        //   but we do not want to save it to your scrollback history. If we did, your
        //   history would become polluted with random fragments of vim's UI!
        let is_unrestricted_scroll = {
            // 1) The scroll region must start at the absolute top of the terminal.
            let region_starts_at_top = *scroll_region.start() == row(0);

            // 2) The scroll region must end at the absolute bottom of the terminal.
            let region_ends_at_bottom = *scroll_region.end()
                == self.ofs_buf.get_window_size().row_height.convert_to_index();

            region_starts_at_top && region_ends_at_bottom
        };

        if is_unrestricted_scroll {
            // Get the line that is about to be evicted from the top.
            let row_index_at_top_pending_eviction = scroll_region.start().as_usize();

            // Clone the evicted line because `shift_lines_up()` uses a zero-allocation
            // `Slice::rotate_left()` optimization and wipes the original line memory to
            // recycle it.
            if let Some(row) = self.ofs_buf.get_row(row_index_at_top_pending_eviction) {
                let evicted_line = row.to_vec();

                // Push it to our scrollback history, maintaining capacity limit.
                self.scrollback_buffer
                    .push_and_enforce_limit(PixelCharLine {
                        pixel_chars: evicted_line,
                    });
            }
        }

        // Use shift_lines_up to shift lines up within the scroll region.
        self.shift_lines_up(scroll_region.to_exclusive(), 1)
    }

    /// Scroll buffer content down by one line (for `ESC M` at top).
    ///
    /// The bottom line is lost, and a new empty line appears at top.
    ///
    /// Respects [`DECSTBM`] scroll region margins.
    ///
    /// # Use Case
    ///
    /// This handles "reverse" scrolling, most commonly used by full-screen TUI
    /// applications.
    ///
    /// ## 1. The Standard Terminal Emulator Story
    ///
    /// Imagine a user is running a standard terminal emulator (like `Wezterm`) and
    /// running a full-screen TUI app like `vim` or `less` inside it. If the user presses
    /// the "Up Arrow" key to scroll up in a document while their cursor is already at the
    /// very top of the screen, `vim` or `less` needs to make room at the top to draw the
    /// new line.
    ///
    /// Instead of redrawing the entire screen, `vim` or `less` sends the `ESC M` ([`RI`]
    /// (Reverse Index)) sequence. This tells the terminal emulator to efficiently shift
    /// the entire screen down by one line. The line at the very bottom is pushed off the
    /// screen and permanently lost (terminals do not have a "scroll-forward" history
    /// buffer), leaving a fresh blank line at the top where `vim` or `less` can draw the
    /// new text.
    ///
    /// ## 2. How [`pty_mux`] Emulates This (the code here)
    ///
    /// When someone builds a TUI app using our [`pty_mux`] module, they are essentially
    /// running our "headless" terminal emulator inside their app.
    ///
    /// When the child [`PTY`] process (like `vim` or `less`) sends the `ESC M` sequence,
    /// it is **this output parser** that acts as the terminal emulator. It shifts all
    /// text down by one line in its virtual memory canvas. The bottom line is dropped
    /// completely, and a blank line is made available at the top of the virtual screen
    /// for the child process to use.
    ///
    /// See [`shift_lines_down()`] for detailed behavior and examples.
    ///
    /// # Errors
    ///
    /// Returns an error if the scroll operation fails.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`pty_mux`]: crate::core::pty::pty_mux
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`RI`]: https://vt100.net/docs/vt510-rm/RI.html
    /// [`shift_lines_down()`]: crate::OfsBufVT100::shift_lines_down
    pub fn scroll_buffer_down(&mut self) -> miette::Result<()> {
        // Get scroll region as an inclusive range and convert to exclusive for iteration.
        let scroll_region = self.get_scroll_range_inclusive();

        // Use shift_lines_down to shift lines down within the scroll region.
        self.shift_lines_down(scroll_region.to_exclusive(), 1)
    }

    /// Handle [`SU`] (Scroll Up) - scroll display up by n lines.
    ///
    /// Multiple lines at the top are lost, new empty lines appear at bottom. Respects
    /// [`DECSTBM`] scroll region margins.
    ///
    /// Example - Scrolling up by 2 lines
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ↓  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top
    ///              │  1  │ Line A (will be lost)               │   (row 1, 0-based)
    ///              │  2  │ Line B (will be lost)               │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom
    ///              ╰  5  │ Footer line (outside scroll region) │   (row 4, 0-based)
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
    ///
    /// # Errors
    ///
    /// Returns an error if the scroll operation fails.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`SU`]: https://vt100.net/docs/vt510-rm/SU.html
    pub fn scroll_up(&mut self, how_many: RowHeight) -> miette::Result<()> {
        for _ in 0..how_many.as_u16() {
            self.scroll_buffer_up()?;
        }
        ok!()
    }

    /// Handle [`SD`] (Scroll Down) - scroll display down by n lines.
    ///
    /// Multiple lines at the bottom are lost, new empty lines appear at top. Respects
    /// [`DECSTBM`] scroll region margins.
    ///
    /// Example - Scrolling down by 2 lines
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ↓  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top
    ///              │  1  │ Line A                              │   (row 1, 0-based)
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C (will be lost)               │
    ///              │  4  │ Line D (will be lost)               │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom
    ///              ╰  5  │ Footer line (outside scroll region) │   (row 4, 0-based)
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
    ///
    /// # Errors
    ///
    /// Returns an error if the scroll operation fails.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`SD`]: https://vt100.net/docs/vt510-rm/SD.html
    pub fn scroll_down(&mut self, how_many: RowHeight) -> miette::Result<()> {
        for _ in 0..how_many.as_u16() {
            self.scroll_buffer_down()?;
        }
        ok!()
    }
}

#[cfg(test)]
mod tests_scroll_vert_ops {
    use super::*;
    use crate::{OfsBufVT100, col, height, idx, row, term_row,
                test_fixtures_ofs_buf::{assert_plain_char_at,
                                        create_vt100_test_buffer_with_size},
                vt_100_pty_output_conformance_tests::nz,
                width};

    fn create_test_buffer() -> OfsBufVT100 {
        create_vt100_test_buffer_with_size(width(10), height(6))
    }

    fn fill_buffer_with_test_content(buffer: &mut OfsBufVT100) {
        // Fill buffer with identifiable content:
        // Row 0: "0000000000"
        // Row 1: "1111111111"
        // Row 2: "2222222222"
        // Row 3: "3333333333"
        // Row 4: "4444444444"
        // Row 5: "5555555555"
        for row_idx in 0..6 {
            for col_idx in 0..10 {
                buffer.cursor_to_position(row(row_idx), col(col_idx));
                let index = idx(row_idx);
                let _unused =
                    buffer.print_char(char::from_digit(index.as_u32(), 10).unwrap());
            }
        }
        buffer.cursor_to_position(row(0), col(0));
    }

    #[test]
    fn test_index_down_within_scroll_region() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Position cursor at row 2, col 5.
        buffer.set_cursor_pos(row(2) + col(5));

        let _unused = buffer.index_down();

        // Cursor should move down one row.
        assert_eq!(buffer.get_cursor_pos().row_index, row(3));
        assert_eq!(buffer.get_cursor_pos().col_index, col(5));

        // Content should remain unchanged.
        assert_plain_char_at(&buffer, 2, 0, '2');
        assert_plain_char_at(&buffer, 3, 0, '3');
    }

    #[test]
    fn test_index_down_at_scroll_bottom_triggers_scroll() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Position cursor at bottom row (row 5).
        buffer.set_cursor_pos(row(5) + col(3));

        let _unused = buffer.index_down();

        // Cursor should stay at bottom row.
        assert_eq!(buffer.get_cursor_pos().row_index, row(5));
        assert_eq!(buffer.get_cursor_pos().col_index, col(3));

        // Buffer should have scrolled up - top line lost, new blank line at bottom.
        assert_plain_char_at(&buffer, 0, 0, '1'); // Row 1 moved to row 0
        assert_plain_char_at(&buffer, 1, 0, '2'); // Row 2 moved to row 1
        assert_plain_char_at(&buffer, 4, 0, '5'); // Row 5 moved to row 4

        // New blank line at bottom (row 5) should be empty.
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

        // Position cursor at row 3, col 2.
        buffer.set_cursor_pos(row(3) + col(2));

        let _unused = buffer.reverse_index_up();

        // Cursor should move up one row.
        assert_eq!(buffer.get_cursor_pos().row_index, row(2));
        assert_eq!(buffer.get_cursor_pos().col_index, col(2));

        // Content should remain unchanged.
        assert_plain_char_at(&buffer, 2, 0, '2');
        assert_plain_char_at(&buffer, 3, 0, '3');
    }

    #[test]
    fn test_reverse_index_up_at_scroll_top_triggers_scroll() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Position cursor at top row (row 0).
        buffer.set_cursor_pos(row(0) + col(7));

        let _unused = buffer.reverse_index_up();

        // Cursor should stay at top row.
        assert_eq!(buffer.get_cursor_pos().row_index, row(0));
        assert_eq!(buffer.get_cursor_pos().col_index, col(7));

        // Buffer should have scrolled down - bottom line lost, new blank line at top.
        // New blank line at top (row 0) should be empty.
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

        let _unused = buffer.scroll_up(RowHeight::from(2));

        // Top 2 lines should be lost, content shifted up.
        assert_plain_char_at(&buffer, 0, 0, '2'); // Row 2 moved to row 0
        assert_plain_char_at(&buffer, 1, 0, '3'); // Row 3 moved to row 1
        assert_plain_char_at(&buffer, 2, 0, '4'); // Row 4 moved to row 2
        assert_plain_char_at(&buffer, 3, 0, '5'); // Row 5 moved to row 3

        // Bottom 2 lines should be blank.
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

        let _unused = buffer.scroll_down(RowHeight::from(2));

        // Top 2 lines should be blank.
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

        // Content should be shifted down.
        assert_plain_char_at(&buffer, 2, 0, '0'); // Row 0 moved to row 2
        assert_plain_char_at(&buffer, 3, 0, '1'); // Row 1 moved to row 3
        assert_plain_char_at(&buffer, 4, 0, '2'); // Row 2 moved to row 4
        assert_plain_char_at(&buffer, 5, 0, '3'); // Row 3 moved to row 5
    }

    #[test]
    fn test_scroll_region_boundaries() {
        let mut buffer = create_test_buffer();
        fill_buffer_with_test_content(&mut buffer);

        // Set up scroll region from row 1 to row 4.
        buffer.parser_global_state.scroll_region_top = Some(term_row(nz(2)));
        buffer.parser_global_state.scroll_region_bottom = Some(term_row(nz(5)));

        // Position cursor at scroll region bottom.
        buffer.set_cursor_pos(row(4) + col(0));

        let _unused = buffer.index_down();

        // Only content within scroll region should have moved.
        assert_plain_char_at(&buffer, 0, 0, '0'); // Row 0 unchanged (outside region)
        assert_plain_char_at(&buffer, 1, 0, '2'); // Row 2 moved to row 1
        assert_plain_char_at(&buffer, 2, 0, '3'); // Row 3 moved to row 2
        assert_plain_char_at(&buffer, 3, 0, '4'); // Row 4 moved to row 3
        assert_plain_char_at(&buffer, 5, 0, '5'); // Row 5 unchanged (outside region)

        // New blank line should appear at row 4.
        let blank_char = buffer.get_char(row(4) + col(0));
        assert!(
            blank_char.is_none() || matches!(blank_char, Some(crate::PixelChar::Spacer))
        );
    }
}

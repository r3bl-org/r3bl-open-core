// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words BCDEF

//! Character operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements character-level operations that correspond to [`ANSI`] escape
//! sequences handled by the [`char_ops`] shim. These include:
//!
//! - `ICH` (Insert Character) - [`insert_chars`]
//! - `DCH` (Delete Character) - [`delete_chars`]
//! - `ECH` (Erase Character) - [`clear_chars`]
//! - `Print Character` - [`print_char`] (printable character handling with [`VT-100`]
//!   features)
//!
//! All operations maintain [`VT-100`] compliance and handle proper character shifting,
//! bounds checking, and cursor positioning as specified in [`VT-100`] documentation.
//!
//! This module implements the business logic for character operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`char_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_char_ops
//! [`clear_chars`]: OfsBufVT100::clear_chars
//! [`delete_chars`]: OfsBufVT100::delete_chars
//! [`insert_chars`]: OfsBufVT100::insert_chars
//! [`print_char`]: OfsBufVT100::print_char
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, AutoWrapMode, OfsBufVT100, PixelChar,
            VPLength,
            core::coordinates::bounds_check::{CursorBoundsCheck, LengthOps,
                                              RangeBoundsExt, RangeConvertExt, RangeExt},
            ok, vp_col, vp_width};

impl OfsBufVT100 {
    /// Insert blank characters at cursor position (for `ICH` - Insert Character).
    ///
    /// - Characters at and after the cursor shift right by `how_many`.
    /// - Characters that would shift beyond the line width are lost.
    ///
    /// Example - Inserting 2 blank characters at cursor position.
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ─────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After insert 2 blanks:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │   │   │ C │ D │ E │ F │ G │ H │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: 2 blanks inserted, C-D-E-F-G-H shifted right, I-J lost beyond margin.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor position is out of bounds or if the operation
    /// fails.
    pub fn insert_chars(&mut self, how_many: VPLength) -> miette::Result<()> {
        let at = self.get_active_screen_buffer().get_cursor_pos();
        let active_buf = self.get_active_screen_buffer_mut();
        let max_width = active_buf.get_viewport().get_width();

        // Nothing to insert if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at.col_index) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Calculate how many characters we can actually insert.
        let how_many_clamped =
            how_many.clamp_to_max(max_width.remaining_from(at.col_index));

        // Exit early if nothing to insert.
        if how_many_clamped.is_empty() {
            return Err(miette::miette!("Operation failed"));
        }

        let buffer_height = active_buf.get_viewport().get_height();
        if buffer_height.is_overflowed_by(at.row_index) == ArrayOverflowResult::Overflowed
        {
            return Err(miette::miette!("Operation failed"));
        }

        let Some(line) = active_buf.get_row_mut(at.row_index) else {
            return Err(miette::miette!("Operation failed"));
        };

        // Copy characters to the right to make room for insertion. Define inclusive
        // range: from cursor through last position that won't overflow.
        let copy_last_position = max_width.index_from_end(how_many_clamped);
        let copy_source_range_inclusive = at.col_index..=copy_last_position;

        // Convert to exclusive range for Rust's copy_within API.
        let copy_source_range = copy_source_range_inclusive
            .to_exclusive()
            .clamp_range_to(max_width);

        // Type-safe checks:
        // 1. Destination must be within bounds.
        // 2. Source range must not be empty (clamp_range_to ensures validity).
        let copy_dest_start_col = at.col_index + how_many_clamped;
        if copy_dest_start_col.overflows(max_width) == ArrayOverflowResult::Within
            && !copy_source_range.is_empty()
        {
            // Convert to usize only when accessing the buffer.
            line.copy_within(
                copy_source_range.as_usize_range(),
                copy_dest_start_col.as_usize(),
            );
        }

        // Fill the cursor position with blanks using type-safe range clamping.
        let fill_end_col = copy_dest_start_col;
        let fill_range = (at.col_index..fill_end_col).clamp_range_to(max_width);
        line[fill_range.as_usize_range()].fill(PixelChar::Spacer);

        ok!()
    }

    /// Delete characters at cursor position (for `DCH` - Delete Character).
    ///
    /// - Characters at and after the deletion point shift left by `how_many`.
    /// - Blank characters are inserted at the end of the line.
    ///
    /// Example - Deleting 2 characters at cursor position.
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
    /// After delete 2 chars:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ E │ F │ G │ H │ I │ J │   │   │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: c and d deleted, E-F-G-H-I-J shifted left, blanks filled at end.
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor position is out of bounds or if the operation
    /// fails.
    pub fn delete_chars(&mut self, how_many: VPLength) -> miette::Result<()> {
        let at = self.get_active_screen_buffer().get_cursor_pos();
        let active_buf = self.get_active_screen_buffer_mut();
        let max_width = active_buf.get_viewport().get_width();

        // Nothing to delete if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at.col_index) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Calculate how many characters we can actually delete.
        let how_many_clamped =
            how_many.clamp_to_max(max_width.remaining_from(at.col_index));

        // Exit early if nothing to delete.
        if how_many_clamped.is_empty() {
            return Err(miette::miette!("Operation failed"));
        }

        let buffer_height = active_buf.get_viewport().get_height();
        if buffer_height.is_overflowed_by(at.row_index) == ArrayOverflowResult::Overflowed
        {
            return Err(miette::miette!("Operation failed"));
        }

        // Copy characters from the right, overwriting the characters at cursor (this IS
        // the deletion). Use CursorBoundsCheck for the exclusive end.
        let source_start = at.col_index + how_many_clamped;
        let source_end = max_width.eol_cursor_position();
        active_buf.copy_chars_within_line(
            at.row_index,
            source_start..source_end,
            at.col_index,
        )?;

        // Clear the vacated space at the end (overwriting duplicates and filling with
        // spacers). Compute inclusive index range by converting length boundaries. We
        // need to fill from (max_width - how_many_clamped + 1) through max_width. Convert
        // to length domain for arithmetic, compute, then convert back to column domain.
        let fill_start_as_length = max_width - vp_width(how_many_clamped) + vp_width(1);
        let fill_range_inclusive =
            fill_start_as_length.convert_to_index()..=max_width.convert_to_index();

        // Convert to exclusive range for fill operation.
        let fill_range = fill_range_inclusive.to_exclusive();
        let fill_range_vp = fill_range.start..fill_range.end;
        active_buf.fill_char_range(at.row_index, fill_range_vp, PixelChar::Spacer)?;

        ok!()
    }

    /// Erase characters at cursor position (for `ECH` - Erase Character).
    ///
    /// Characters are replaced with blanks, no shifting occurs.
    ///
    /// Example - Erasing 3 characters at cursor position.
    ///
    /// ```text
    /// Before:
    ///           ╭────── max_width=10 (1-based) ─────╮
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// After erase 3 chars:
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Row:    │ A │ B │   │   │   │ F │ G │ H │ I │ J │
    ///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
    ///                   ╰ cursor (col 2, 0-based)
    ///
    /// Result: C, D, E replaced with blanks, F-G-H-I-J remain in place (no shifting)
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor position is out of bounds or if the operation
    /// fails.
    pub fn clear_chars(&mut self, how_many: VPLength) -> miette::Result<()> {
        let at = self.get_active_screen_buffer().get_cursor_pos();
        let active_buf = self.get_active_screen_buffer_mut();
        let max_width = active_buf.get_viewport().get_width();

        // Nothing to erase if cursor is at or beyond right margin.
        if max_width.is_overflowed_by(at.col_index) == ArrayOverflowResult::Overflowed {
            return Err(miette::miette!("Operation failed"));
        }

        // Calculate how many characters we can actually erase.
        let how_many_clamped =
            how_many.clamp_to_max(max_width.remaining_from(at.col_index));

        // Exit early if nothing to erase.
        if how_many_clamped.is_empty() {
            return Err(miette::miette!("Operation failed"));
        }

        let buffer_height = active_buf.get_viewport().get_height();
        if buffer_height.is_overflowed_by(at.row_index) == ArrayOverflowResult::Overflowed
        {
            return Err(miette::miette!("Operation failed"));
        }

        // Use type-safe range clamping for consistent patterns.
        let cursor_col = at.col_index;
        let fill_end_col = cursor_col + how_many_clamped;
        let erase_range = (cursor_col..fill_end_col).clamp_range_to(max_width);
        let erase_range_vp = erase_range.start..erase_range.end;
        active_buf.fill_char_range(at.row_index, erase_range_vp, PixelChar::Spacer)
    }

    /// Applies the deferred pending wrap by performing a carriage return and line feed.
    ///
    /// This is called when the terminal is in a pending wrap state and receives the next
    /// printable character. It delegates to [`handle_carriage_return`] and [`index_down`]
    /// which automatically handles [`DECSTBM`] scrolling regions correctly.
    ///
    /// # Errors
    /// Returns an error if the scrolling operation fails.
    ///
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    pub fn apply_pending_wrap(&mut self) -> miette::Result<()> {
        self.handle_carriage_return();
        self.index_down()?;
        self.get_parser_global_state_mut().clear_pending_wrap();
        ok!()
    }

    /// Handles printable characters with character set translation, bounds checking, and
    /// line wrapping.
    ///
    /// This method consolidates all character printing logic including:
    /// - [`DEC`] graphics character translation
    /// - Bounds checking
    /// - Character writing to buffer
    /// - [`DECAWM`] (Auto Wrap Mode) line wrap handling
    ///
    /// # Arguments
    /// * `ch` - The character to print
    ///
    /// # Behavior
    /// 1. Applies character set translation if in graphics mode
    /// 2. Writes character to buffer at current cursor position (if within bounds)
    /// 3. Advances cursor, handling line wrap based on [`DECAWM`] mode
    ///
    /// # Line Wrapping
    /// - [`DECAWM`] enabled (default): wraps to next line when reaching right margin
    /// - [`DECAWM`] disabled: cursor stays at right margin, new chars overwrite
    ///
    /// # Errors
    ///
    /// Returns an error if the character cannot be processed or if the operation fails.
    ///
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
    pub fn print_char(&mut self, ch: char) -> miette::Result<()> {
        // If there's a pending wrap, apply it before printing this new character. Handle
        // pending line wrap based on DECAWM (Auto Wrap Mode).
        if self.get_parser_global_state_mut().get_pending_wrap() == PendingWrap::Yes {
            self.apply_pending_wrap()?;
        }

        // Apply character set translation if in graphics mode.
        let display_char = match self.get_parser_global_state_mut().character_set {
            CharacterSet::DECGraphics => Self::translate_dec_graphics(ch),
            CharacterSet::Ascii => ch,
        };

        let current_style = self.get_parser_global_state_mut().current_style;
        let cursor_pos = self.get_active_screen_buffer().get_cursor_pos();
        let current_row = cursor_pos.row_index;
        let current_col = cursor_pos.col_index;

        let active_buf = self.get_active_screen_buffer_mut();
        let row_max = active_buf.get_viewport().get_height();
        let col_max = active_buf.get_viewport().get_width();
        // Only write if within bounds.
        if current_row.overflows(row_max) == ArrayOverflowResult::Within
            && current_col.overflows(col_max) == ArrayOverflowResult::Within
        {
            active_buf.set_char(
                cursor_pos,
                PixelChar::PlainText {
                    display_char, // Use the translated character
                    style: current_style,
                },
            )?;

            // Move cursor forward.
            let next_col_index = current_col + vp_col(1);
            let new_col = next_col_index;

            // Handle line wrap based on DECAWM (Auto Wrap Mode).
            //
            // This logic only triggers when `print_char()` places a character into the
            // very last column on the screen (when `new_col` overflows the maximum
            // column). When that happens, the terminal does not jump to the next line
            // yet. It just parks the cursor directly on top of the character it just
            // printed at the right edge and flags itself with `PendingWrap::Yes`.
            if new_col.overflows(col_max) == ArrayOverflowResult::Overflowed {
                if self.get_parser_global_state_mut().auto_wrap_mode
                    == AutoWrapMode::Enabled
                {
                    // DECAWM enabled: enter pending wrap state.
                    self.get_parser_global_state_mut().set_pending_wrap();
                }

                let mut pos = self.get_active_screen_buffer().get_cursor_pos();
                pos.col_index = col_max.convert_to_index();
                self.get_active_screen_buffer_mut().set_cursor_pos(pos);
            } else {
                let mut pos = self.get_active_screen_buffer().get_cursor_pos();
                pos.col_index = new_col;
                self.get_active_screen_buffer_mut().set_cursor_pos(pos);
            }
        }

        ok!()
    }
}

#[cfg(test)]
#[allow(clippy::too_many_lines)]
mod tests_shifting_ops {
    use crate::{NarrowingCastToU16, OfsBufVT100, PixelChar, TuiStyle, VPRow, VPSize,
                test_fixtures_ofs_buf::{create_plain_test_char,
                                        create_vt100_test_buffer_with_size},
                vp_col, vp_height, vp_len, vp_pos, vp_row, vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        create_vt100_test_buffer_with_size(vp_width(6), vp_height(3))
    }

    fn create_test_char(ch: char) -> PixelChar { create_plain_test_char(ch) }

    fn setup_line_with_chars(buffer: &mut OfsBufVT100, test_row: VPRow, chars: &[char]) {
        for (i, &ch) in chars.iter().enumerate() {
            if i < 6 {
                // Match buffer width.
                let _unused = buffer.set_char(
                    test_row + vp_col(i.as_u16_narrowing()),
                    create_test_char(ch),
                );
            }
        }
    }

    #[test]
    fn test_insert_chars_basic() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Insert 2 blank characters at position 2 (before 'C').
        buffer.set_cursor_pos(test_row + vp_col(2));
        let result = buffer.insert_chars(vp_len(2));
        assert!(result.is_ok());

        // Expected result: "AB  CD" (E and F are pushed out).
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(3))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(4))
                .expect("conversion error"),
            create_test_char('C')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(5))
                .expect("conversion error"),
            create_test_char('D')
        );
    }

    #[test]
    fn test_insert_chars_overflow() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(0);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to insert 10 characters at position 1 (more than remaining space).
        buffer.set_cursor_pos(test_row + vp_col(1));
        let result = buffer.insert_chars(vp_len(10));
        assert!(result.is_ok());

        // Should insert as many as possible: "A     " (5 spaces, B-F pushed out).
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        for i in 1..6 {
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                PixelChar::Spacer
            );
        }
    }

    #[test]
    fn test_insert_chars_at_end_of_line() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to insert at the last position.
        buffer.set_cursor_pos(test_row + vp_col(5));
        let result = buffer.insert_chars(vp_len(1));
        assert!(result.is_ok());

        // Should insert one space, pushing F out: "ABCDE ".
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(4))
                .expect("conversion error"),
            create_test_char('E')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(5))
                .expect("conversion error"),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_insert_chars_invalid_conditions() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        buffer.set_cursor_pos(vp_pos(2, 10));
        let result1 = buffer.insert_chars(vp_len(1));
        assert!(result1.is_err());

        // Test with cursor position beyond line width.
        buffer.set_cursor_pos(vp_pos(10, 0));
        let result2 = buffer.insert_chars(vp_len(1));
        assert!(result2.is_err());

        // Test with zero insert count.
        buffer.set_cursor_pos(vp_pos(2, 0));
        let result3 = buffer.insert_chars(vp_len(0));
        assert!(result3.is_err());
    }

    #[test]
    fn test_delete_chars_basic() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Delete 2 characters at position 2 (delete 'C' and 'D').
        buffer.set_cursor_pos(test_row + vp_col(2));
        let result = buffer.delete_chars(vp_len(2));
        assert!(result.is_ok());

        // Verify: "AB" + "EF" + "  " (CD deleted, EF shifted left, blanks at end).
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            create_test_char('E')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(3))
                .expect("conversion error"),
            create_test_char('F')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(4))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(5))
                .expect("conversion error"),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_delete_chars_overflow() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(0);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to delete 10 characters at position 1 (more than remaining space).
        buffer.set_cursor_pos(test_row + vp_col(1));
        let result = buffer.delete_chars(vp_len(10));
        assert!(result.is_ok());

        // Verify: "A" + "     " (BCDEF all deleted, 5 blanks at end).
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        for i in 1..6 {
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                PixelChar::Spacer
            );
        }
    }

    #[test]
    fn test_delete_chars_at_end_of_line() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to delete at the last position.
        buffer.set_cursor_pos(test_row + vp_col(5));
        let result = buffer.delete_chars(vp_len(1));
        assert!(result.is_ok());

        // Verify: "ABCDE " (F deleted, one blank at end).
        for (i, expected_char) in ['A', 'B', 'C', 'D', 'E'].iter().enumerate() {
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                create_test_char(*expected_char)
            );
        }
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(5))
                .expect("conversion error"),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_delete_chars_invalid_conditions() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        buffer.set_cursor_pos(vp_pos(2, 10));
        let result1 = buffer.delete_chars(vp_len(1));
        assert!(result1.is_err());

        // Test with cursor position beyond line width.
        buffer.set_cursor_pos(vp_pos(10, 0));
        let result2 = buffer.delete_chars(vp_len(1));
        assert!(result2.is_err());

        // Test with zero delete count.
        buffer.set_cursor_pos(vp_pos(2, 0));
        let result3 = buffer.delete_chars(vp_len(0));
        assert!(result3.is_err());
    }

    #[test]
    fn test_clear_chars_basic() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Erase 3 characters at position 2 (erase 'C', 'D', 'E').
        buffer.set_cursor_pos(test_row + vp_col(2));
        let result = buffer.clear_chars(vp_len(3));
        assert!(result.is_ok());

        // Verify: "AB" + "   " + "F" (CDE erased with blanks, F stays in place).
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(3))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(4))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(5))
                .expect("conversion error"),
            create_test_char('F')
        );
    }

    #[test]
    fn test_clear_chars_overflow() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(0);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to erase 10 characters at position 1 (more than remaining space).
        buffer.set_cursor_pos(test_row + vp_col(1));
        let result = buffer.clear_chars(vp_len(10));
        assert!(result.is_ok());

        // Verify: "A" + "     " (BCDEF all erased with blanks).
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        for i in 1..6 {
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                PixelChar::Spacer
            );
        }
    }

    #[test]
    fn test_erase_chars_at_end_of_line() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(1);

        // Set up initial line: "ABCDEF".
        setup_line_with_chars(&mut buffer, test_row, &['A', 'B', 'C', 'D', 'E', 'F']);

        // Try to erase at the last position.
        buffer.set_cursor_pos(test_row + vp_col(5));
        let result = buffer.clear_chars(vp_len(1));
        assert!(result.is_ok());

        // Verify: "ABCDE " (F erased with blank).
        for (i, expected_char) in ['A', 'B', 'C', 'D', 'E'].iter().enumerate() {
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                create_test_char(*expected_char)
            );
        }
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(5))
                .expect("conversion error"),
            PixelChar::Spacer
        );
    }

    #[test]
    fn test_erase_chars_invalid_conditions() {
        let mut buffer = create_test_buffer();

        // Test with invalid row.
        buffer.set_cursor_pos(vp_pos(2, 10));
        let result1 = buffer.clear_chars(vp_len(1));
        assert!(result1.is_err());

        // Test with cursor position beyond line width.
        buffer.set_cursor_pos(vp_pos(10, 0));
        let result2 = buffer.clear_chars(vp_len(1));
        assert!(result2.is_err());

        // Test with zero erase count.
        buffer.set_cursor_pos(vp_pos(2, 0));
        let result3 = buffer.clear_chars(vp_len(0));
        assert!(result3.is_err());
    }

    // Additional comprehensive boundary tests for ICH, DCH, ECH operations.
    #[test]
    fn test_operations_at_line_start() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        let size = VPSize {
            col_width: vp_width(10),
            row_height: vp_height(3),
        };
        let mut buffer = OfsBufVT100::new_empty(size);
        let test_row = vp_row(0);

        // Set up initial line with characters: "ABCDEFGHIJ".
        let chars = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'];
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(
                test_row + vp_col(i.as_u16_narrowing()),
                create_test_char(ch),
            );
        }

        // Test delete at column 0 - should delete A,B and shift left.
        buffer.set_cursor_pos(test_row + vp_col(0));
        let result = buffer.delete_chars(vp_len(2));
        assert!(result.is_ok());

        // Verify: C,D,E,F,G,H,I,J shifted left, blanks at end.
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('C')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('D')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(7))
                .expect("conversion error"),
            create_test_char('J')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(8))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(9))
                .expect("conversion error"),
            PixelChar::Spacer
        );

        // Reset for insert test.
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(
                test_row + vp_col(i.as_u16_narrowing()),
                create_test_char(ch),
            );
        }

        // Test insert at column 0 - should insert 2 blanks and shift right.
        buffer.set_cursor_pos(test_row + vp_col(0));
        let result = buffer.insert_chars(vp_len(2));
        assert!(result.is_ok());

        // Verify: 2 blanks inserted at start, A-H shifted right, I,J lost.
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(9))
                .expect("conversion error"),
            create_test_char('H')
        );

        // Reset for erase test.
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(
                test_row + vp_col(i.as_u16_narrowing()),
                create_test_char(ch),
            );
        }

        // Test erase at column 0 - should erase A,B,C without shifting.
        buffer.set_cursor_pos(test_row + vp_col(0));
        let result = buffer.clear_chars(vp_len(3));
        assert!(result.is_ok());

        // Verify: A,B,C erased (blanks), D-J remain in place.
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(3))
                .expect("conversion error"),
            create_test_char('D')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(9))
                .expect("conversion error"),
            create_test_char('J')
        );
    }

    #[test]
    fn test_single_char_operations() {
        // Helper function to create test characters.
        fn create_test_char(ch: char) -> PixelChar {
            PixelChar::PlainText {
                display_char: ch,
                style: TuiStyle::default(),
            }
        }

        let size = VPSize {
            col_width: vp_width(5),
            row_height: vp_height(2),
        };
        let mut buffer = OfsBufVT100::new_empty(size);
        let test_row = vp_row(0);

        // Set up initial line with characters: "ABCDE".
        let chars = ['A', 'B', 'C', 'D', 'E'];
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(
                test_row + vp_col(i.as_u16_narrowing()),
                create_test_char(ch),
            );
        }

        // Test single char delete at middle position (delete C).
        buffer.set_cursor_pos(test_row + vp_col(2));
        let result = buffer.delete_chars(vp_len(1));
        assert!(result.is_ok());

        // Verify: A,B remain, D,E shifted left, blank at end.
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            create_test_char('D')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(3))
                .expect("conversion error"),
            create_test_char('E')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(4))
                .expect("conversion error"),
            PixelChar::Spacer
        );

        // Reset for insert test.
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(
                test_row + vp_col(i.as_u16_narrowing()),
                create_test_char(ch),
            );
        }

        // Test single char insert at middle position (before C).
        buffer.set_cursor_pos(test_row + vp_col(2));
        let result = buffer.insert_chars(vp_len(1));
        assert!(result.is_ok());

        // Verify: A,B remain, blank inserted, C,D shifted right, E lost.
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(3))
                .expect("conversion error"),
            create_test_char('C')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(4))
                .expect("conversion error"),
            create_test_char('D')
        );

        // Reset for erase test.
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(
                test_row + vp_col(i.as_u16_narrowing()),
                create_test_char(ch),
            );
        }

        // Test single char erase at middle position (erase C).
        buffer.set_cursor_pos(test_row + vp_col(2));
        let result = buffer.clear_chars(vp_len(1));
        assert!(result.is_ok());

        // Verify: A,B remain, C erased (blank), D,E remain in place.
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            PixelChar::Spacer
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(3))
                .expect("conversion error"),
            create_test_char('D')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(4))
                .expect("conversion error"),
            create_test_char('E')
        );
    }

    #[test]
    fn test_operations_on_empty_line() {
        let mut buffer = create_test_buffer();
        let test_row = vp_row(0);

        // Test delete on empty line (should succeed but do nothing).
        buffer.set_cursor_pos(test_row + vp_col(0));
        let result = buffer.delete_chars(vp_len(3));
        assert!(result.is_ok()); // Should succeed on spacer-filled line

        // Verify line remains empty.
        for i in 0..6 {
            // Match buffer width.
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                PixelChar::Spacer
            );
        }

        // Test insert on empty line at column 0.
        buffer.set_cursor_pos(test_row + vp_col(0));
        let result = buffer.insert_chars(vp_len(3));
        assert!(result.is_ok());

        // Verify 3 blanks were inserted (line still appears empty).
        for i in 0..3 {
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                PixelChar::Spacer
            );
        }

        // Test erase on empty line (should succeed but do nothing).
        buffer.set_cursor_pos(test_row + vp_col(0));
        let result = buffer.clear_chars(vp_len(2));
        assert!(result.is_ok()); // Should succeed on spacer-filled line

        // Verify line remains empty.
        for i in 0..6 {
            // Match buffer width.
            assert_eq!(
                buffer
                    .get_char(test_row + vp_col(i.as_u16_narrowing()))
                    .expect("conversion error"),
                PixelChar::Spacer
            );
        }

        // Test operations beyond content length on short line.
        let chars = ['A', 'B', 'C'];
        for (i, &ch) in chars.iter().enumerate() {
            let _unused = buffer.set_char(
                test_row + vp_col(i.as_u16_narrowing()),
                create_test_char(ch),
            );
        }

        // Try to delete at position beyond content length (but within width).
        buffer.set_cursor_pos(test_row + vp_col(5));
        let result = buffer.delete_chars(vp_len(1));
        assert!(result.is_ok()); // Should succeed - position is within buffer width

        // Verify original content unchanged.
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(test_row + vp_col(2))
                .expect("conversion error"),
            create_test_char('C')
        );
    }
}

#[cfg(test)]
#[allow(clippy::too_many_lines)]
mod tests_print_char {
    use super::*;
    use crate::{test_fixtures_ofs_buf::create_vt100_test_buffer_with_size, vp_col,
                vp_height, vp_pos, vp_row, vp_width};

    #[test]
    fn test_print_char_basic() {
        let mut buffer = create_vt100_test_buffer_with_size(vp_width(10), vp_height(5));

        // Set cursor position.
        buffer.set_cursor_pos(vp_pos(2, 1));

        // Print a character.
        let _unused = buffer.print_char('A');

        // Verify character was printed at cursor position.
        let printed_char = buffer.get_char(vp_pos(2, 1)).expect("conversion error");
        match printed_char {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, 'A'),
            _ => panic!("Expected PlainText with 'A'"),
        }

        // Verify cursor advanced by one column.
        assert_eq!(buffer.get_cursor_pos(), vp_row(1) + vp_col(3));
    }

    #[test]
    fn test_print_char_dec_graphics_mode() {
        let mut buffer = create_vt100_test_buffer_with_size(vp_width(10), vp_height(5));

        // Set DEC graphics character set.
        buffer.get_parser_global_state_mut().character_set = CharacterSet::DECGraphics;

        buffer.set_cursor_pos(vp_pos(0, 0));

        // Print DEC graphics characters that should be translated.
        let _unused = buffer.print_char('q'); // Should become '─' (horizontal line)

        // Verify translation occurred.
        let printed_char = buffer.get_char(vp_pos(0, 0)).expect("conversion error");
        match printed_char {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, '─'),
            _ => panic!("Expected PlainText with '─'"),
        }
    }

    #[test]
    fn test_print_char_line_wrap() {
        let mut buffer = create_vt100_test_buffer_with_size(vp_width(5), vp_height(3));

        // Ensure DECAWM is enabled (default).
        buffer.get_parser_global_state_mut().auto_wrap_mode = AutoWrapMode::Enabled;

        // Position cursor at end of line (column 4 in 5-width buffer).
        buffer.set_cursor_pos(vp_pos(4, 1));

        // Print a character - should wrap to next line.
        let _unused = buffer.print_char('X');

        // Verify character was printed at end of current line.
        let printed_char = buffer.get_char(vp_pos(4, 1)).expect("conversion error");
        match printed_char {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, 'X'),
            _ => panic!("Expected PlainText with 'X'"),
        }

        // Verify cursor stays at right margin.
        assert_eq!(buffer.get_cursor_pos(), vp_row(1) + vp_col(4));
        // Verify pending wrap is true.
        assert_eq!(
            buffer.get_parser_global_state_mut().get_pending_wrap(),
            PendingWrap::Yes
        );

        // Print another character - should wrap to next line, and print at column 0.
        let _unused = buffer.print_char('Y');
        let printed_char = buffer.get_char(vp_pos(0, 2)).expect("conversion error");
        match printed_char {
            PixelChar::PlainText { display_char, .. } => assert_eq!(display_char, 'Y'),
            _ => panic!("Expected PlainText with 'Y'"),
        }

        // Verify cursor advanced to next column on the new line.
        assert_eq!(buffer.get_cursor_pos(), vp_row(2) + vp_col(1));
        assert_eq!(
            buffer.get_parser_global_state_mut().get_pending_wrap(),
            PendingWrap::No
        );
    }
}

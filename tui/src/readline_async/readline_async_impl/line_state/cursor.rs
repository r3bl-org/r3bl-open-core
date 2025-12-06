// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::LineState;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ColWidth, CsiSequence, CursorBoundsCheck,
            NumericValue, Seg, StringLength, col, ok, seg_index, width};
use std::io::{self, Write};

impl LineState {
    /// Gets the number of lines wrapped (how many rows the text spans).
    ///
    /// The `pos` parameter is a display offset (column width) from the start of the line.
    pub(crate) fn line_height(&self, pos: ColWidth) -> u16 {
        pos.as_u16() / self.term_size.col_width.as_u16()
    }

    /// Move from a position on the line to the start.
    ///
    /// The `from` parameter is a display offset (column width) from the start of the line.
    pub(crate) fn move_to_beginning(&self, term: &mut dyn Write, from: ColWidth) -> io::Result<()> {
        // Calculate row directly from position.
        // Position 80 on 80-col terminal is Row 1, Col 0: 80/80 = 1 row.
        let move_up = self.line_height(from);

        // Move to column 0 (CHA = Cursor Horizontal Absolute, 1-based).
        term.write_all(CsiSequence::CursorHorizontalAbsolute(1).to_string().as_bytes())?;

        // Move up the calculated number of rows (CUU = Cursor Up).
        if move_up != 0 {
            term.write_all(CsiSequence::CursorUp(move_up).to_string().as_bytes())?;
        }

        ok!()
    }

    /// Move from the start of the line to some position.
    ///
    /// The `to` parameter is a display offset (column width) from the start of the line.
    pub(crate) fn move_from_beginning(&self, term: &mut dyn Write, to: ColWidth) -> io::Result<()> {
        // Calculate row directly from position.
        // Position 80 on 80-col terminal is Row 1, Col 0: 80/80 = 1 row.
        let line_height = self.line_height(to);
        let line_remaining_len = to.as_u16() % self.term_size.col_width.as_u16(); // Column position

        // Move down the calculated number of rows (CUD = Cursor Down).
        if line_height != 0 {
            term.write_all(CsiSequence::CursorDown(line_height).to_string().as_bytes())?;
        }

        // Move right to the column position (CUF = Cursor Forward).
        // Guard: CursorForward(0) is interpreted as CursorForward(1) by most terminals
        // due to ANSI/CSI cursor movement commands treating 0 as 1.
        if line_remaining_len != 0 {
            term.write_all(
                CsiSequence::CursorForward(line_remaining_len)
                    .to_string()
                    .as_bytes(),
            )?;
        }

        ok!()
    }

    /// Move cursor by one unicode grapheme either left (negative) or right (positive).
    ///
    /// # Errors
    ///
    /// Returns an error if I/O operations fail.
    pub fn move_cursor(&mut self, change: isize) -> io::Result<()> {
        if change > 0 {
            let count = self.line.segment_count();

            // We know that change is positive, so we can safely cast it to usize.
            #[allow(clippy::cast_sign_loss)]
            let change_usize = change as usize;

            let new_position = self.line_cursor_grapheme + seg_index(change_usize);
            // Use CursorBoundsCheck for text cursor positioning (allows position == length).
            self.line_cursor_grapheme = count.clamp_cursor_position(new_position);
        } else {
            // Use unsigned_abs() to convert negative change to
            // positive amount to subtract.
            let change_seg_idx = seg_index(change.unsigned_abs());
            self.line_cursor_grapheme = if change_seg_idx
                .overflows(self.line_cursor_grapheme.convert_to_seg_length())
                == ArrayOverflowResult::Overflowed
            {
                seg_index(0)
            } else {
                self.line_cursor_grapheme - change_seg_idx
            };
        }

        // Calculate display width up to cursor position using segment metadata.
        let line_display_width = self.calculate_display_width_up_to_cursor();

        let prompt_len =
            StringLength::StripAnsi.calculate(&self.prompt, &mut self.memoized_len_map);

        self.current_column = col(prompt_len + line_display_width.as_u16());

        ok!()
    }

    /// Calculate the display width of the line up to the current cursor position.
    ///
    /// Uses pre-computed segment metadata for O(n) where n is segments up to cursor,
    /// rather than re-parsing the entire string.
    fn calculate_display_width_up_to_cursor(&self) -> ColWidth {
        let mut total_width = width(0);
        for i in 0..self.line_cursor_grapheme.as_usize() {
            if let Some(seg) = self.line.get(seg_index(i)) {
                total_width += seg.display_width;
            }
        }
        total_width
    }

    /// Returns the grapheme cluster segment immediately before the cursor position.
    ///
    /// Returns `None` if the cursor is at the beginning of the line (position 0).
    ///
    /// # Returns
    ///
    /// A [`Seg`] containing byte offset, display width, and other segment metadata.
    /// Use `seg.get_str(&self.line)` to get the actual grapheme string.
    #[must_use]
    pub fn current_grapheme(&self) -> Option<Seg> {
        if self.line_cursor_grapheme.is_zero() {
            return None;
        }
        self.line.get(self.line_cursor_grapheme - seg_index(1))
    }

    /// Returns the grapheme cluster segment at the cursor position (to be deleted by Delete key).
    ///
    /// Returns `None` if the cursor is at the end of the line.
    ///
    /// # Returns
    ///
    /// A [`Seg`] containing byte offset, display width, and other segment metadata.
    /// Use `seg.get_str(&self.line)` to get the actual grapheme string.
    #[must_use]
    pub fn next_grapheme(&self) -> Option<Seg> {
        let total = self.line.segment_count();
        if self.line_cursor_grapheme.as_usize() >= total.as_usize() {
            return None;
        }
        self.line.get(self.line_cursor_grapheme)
    }

    /// Moves the terminal cursor to the beginning of the input line.
    ///
    /// This is used before re-rendering or when the cursor position needs to be
    /// recalculated from the start.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn reset_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
        // Column index value equals distance from start (col 5 = 5 chars from start).
        self.move_to_beginning(term, width(self.current_column.as_u16()))?;

        ok!()
    }

    /// Moves the terminal cursor from the beginning to the current cursor position.
    ///
    /// This is typically called after [`reset_cursor`](Self::reset_cursor) to restore
    /// the cursor to its logical position within the line.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn set_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
        // Column index value equals distance from start (col 5 = 5 chars from start).
        self.move_from_beginning(term, width(self.current_column.as_u16()))?;

        ok!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ANSI_CSI_BRACKET, CSI_START, CUD_CURSOR_DOWN, CUF_CURSOR_FORWARD, ESC_START,
        core::test_fixtures::StdoutMock,
    };

    /// Check if the string contains a `CursorForward` sequence (CSI <n> C).
    fn contains_cursor_forward(s: &str) -> bool {
        // CSI sequences start with ESC [ and CursorForward ends with 'C'.
        // We look for patterns like "\x1b[5C" or "\x1b[10C".
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == ESC_START && chars.next() == Some(ANSI_CSI_BRACKET as char) {
                // Read digits.
                let mut has_digits = false;
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() {
                        has_digits = true;
                        chars.next();
                    } else {
                        break;
                    }
                }
                // Check if it ends with 'C' (CursorForward).
                if has_digits && chars.next() == Some(CUF_CURSOR_FORWARD) {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn test_move_from_beginning_no_spurious_move_right_regression() {
        let line_state = LineState::new(String::new(), (80, 100));

        // Test case 1: move_from_beginning with to=0 should NOT emit any movement.
        {
            let mut stdout_mock = StdoutMock::default();
            line_state
                .move_from_beginning(&mut stdout_mock, width(0))
                .unwrap();
            let output_str = stdout_mock.get_copy_of_buffer_as_string();

            // Should NOT contain any ANSI sequences.
            assert!(
                !output_str.contains(CSI_START),
                "Expected no ANSI sequences for to=0, but got: {output_str:?}"
            );
        }

        // Test case 2: move_from_beginning with to=240 (exactly 3 rows on 80-col term)
        // should emit MoveDown(3) and NO MoveRight.
        // Position 240 means: 240/80 = 3 rows down, 240%80 = 0 cols right.
        {
            let mut stdout_mock = StdoutMock::default();
            line_state
                .move_from_beginning(&mut stdout_mock, width(240))
                .unwrap();
            let output_str = stdout_mock.get_copy_of_buffer_as_string();

            // Should contain CSI 3 B (CursorDown(3)).
            let expected_cursor_down_3 = format!("{CSI_START}3{CUD_CURSOR_DOWN}");
            assert!(
                output_str.contains(&expected_cursor_down_3),
                "Expected CursorDown(3) for to=240, but got: {output_str:?}"
            );

            // Should NOT contain CursorForward (CSI <n> C).
            assert!(
                !contains_cursor_forward(&output_str),
                "Expected NO CursorForward for to=240, but got: {output_str:?}"
            );
        }

        // Test case 3: move_from_beginning with to=5 SHOULD emit MoveRight(5).
        {
            let mut stdout_mock = StdoutMock::default();
            line_state
                .move_from_beginning(&mut stdout_mock, width(5))
                .unwrap();
            let output_str = stdout_mock.get_copy_of_buffer_as_string();

            // Should contain CSI 5 C (CursorForward(5)).
            let expected_cursor_forward_5 = format!("{CSI_START}5{CUF_CURSOR_FORWARD}");
            assert!(
                output_str.contains(&expected_cursor_forward_5),
                "Expected CursorForward(5) for to=5, but got: {output_str:?}"
            );
        }

        // Test case 4: move_from_beginning with to=80 (exactly 1 row on 80-col term)
        // should emit MoveDown(1) and NO MoveRight.
        {
            let mut stdout_mock = StdoutMock::default();
            line_state
                .move_from_beginning(&mut stdout_mock, width(80))
                .unwrap();
            let output_str = stdout_mock.get_copy_of_buffer_as_string();

            // Should contain CSI 1 B (CursorDown(1)).
            let expected_cursor_down_1 = format!("{CSI_START}1{CUD_CURSOR_DOWN}");
            assert!(
                output_str.contains(&expected_cursor_down_1),
                "Expected CursorDown(1) for to=80, but got: {output_str:?}"
            );

            // Should NOT contain CursorForward.
            assert!(
                !contains_cursor_forward(&output_str),
                "Expected NO CursorForward for to=80, but got: {output_str:?}"
            );
        }
    }
}

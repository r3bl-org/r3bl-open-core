// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::LineState;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ColWidth, CsiSequence, CursorBoundsCheck,
            NumericValue, Seg, StringLength, TermCol, TermColDelta, TermRowDelta, col, ok,
            seg_index, term_col_delta, term_row_delta, width};
use std::io::{self, Write};

impl LineState {
    /// Gets the number of lines wrapped (how many rows the text spans).
    ///
    /// The `pos` parameter is a display offset (column width) from the start of the line.
    /// Returns a [`TermRowDelta`] representing how many rows down the position is.
    /// Returns `None` if the calculated delta is zero (position is on the first line).
    #[must_use]
    pub fn line_height(&self, pos: ColWidth) -> Option<TermRowDelta> {
        term_row_delta(pos / self.term_size.col_width)
    }

    /// Gets the column offset within the current row.
    ///
    /// The `pos` parameter is a display offset (column width) from the start of the line.
    /// Returns a [`TermColDelta`] representing the horizontal position within the row.
    /// Returns `None` if the calculated delta is zero (position is at the start of a row).
    #[must_use]
    pub fn line_column_offset(&self, pos: ColWidth) -> Option<TermColDelta> {
        term_col_delta(pos % self.term_size.col_width)
    }

    /// Move from a position on the line to the start.
    ///
    /// The `from` parameter is a display offset (column width) from the start of the line.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn move_to_beginning(&self, term: &mut dyn Write, from: ColWidth) -> io::Result<()> {
        // Calculate row delta from position.
        // Position 80 on 80-col terminal is Row 1, Col 0: 80/80 = 1 row.
        let move_up = self.line_height(from);

        // Move to column 1 (CHA = Cursor Horizontal Absolute, 1-based).
        term.write_all(CsiSequence::CursorHorizontalAbsolute(TermCol::ONE).to_string().as_bytes())?;

        // Move up the calculated number of rows (CUU = Cursor Up).
        // Only emit if Some (non-zero) - guards against CSI zero bug.
        if let Some(delta) = move_up {
            term.write_all(CsiSequence::CursorUp(delta).to_string().as_bytes())?;
        }

        ok!()
    }

    /// Move from the start of the line to some position.
    ///
    /// The `to` parameter is a display offset (column width) from the start of the line.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn move_from_beginning(&self, term: &mut dyn Write, to: ColWidth) -> io::Result<()> {
        // Calculate deltas from position.
        // Position 80 on 80-col terminal is Row 1, Col 0: 80/80 = 1 row, 80%80 = 0 cols.
        let rows_down = self.line_height(to);
        let cols_right = self.line_column_offset(to);

        // Move down the calculated number of rows (CUD = Cursor Down).
        // Only emit if Some (non-zero) - guards against CSI zero bug.
        if let Some(delta) = rows_down {
            term.write_all(CsiSequence::CursorDown(delta).to_string().as_bytes())?;
        }

        // Move right to the column position (CUF = Cursor Forward).
        // Only emit if Some (non-zero) - guards against CSI zero bug where
        // CursorForward(0) is interpreted as CursorForward(1) by terminals.
        if let Some(delta) = cols_right {
            term.write_all(CsiSequence::CursorForward(delta).to_string().as_bytes())?;
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
    use test_case::test_case;

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

    /// Check if the string contains a `CursorDown` sequence (CSI <n> B).
    fn contains_cursor_down(s: &str) -> bool {
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == ESC_START && chars.next() == Some(ANSI_CSI_BRACKET as char) {
                let mut has_digits = false;
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() {
                        has_digits = true;
                        chars.next();
                    } else {
                        break;
                    }
                }
                if has_digits && chars.next() == Some(CUD_CURSOR_DOWN) {
                    return true;
                }
            }
        }
        false
    }

    // ========================================================================
    // Terminal boundary regression tests for `move_from_beginning`.
    //
    // On an 80-column terminal, positions that are exact multiples of 80
    // (80, 160, 240, 320) sit at column 0 of their respective rows.
    // These boundary cases are critical for detecting off-by-one errors:
    //
    // | Position | Rows (pos/80) | Column (pos%80) | Expected Output         |
    // |----------|---------------|-----------------|-------------------------|
    // | 0        | 0             | 0               | No movement             |
    // | 5        | 0             | 5               | CursorForward(5)        |
    // | 80       | 1             | 0               | CursorDown(1) only      |
    // | 120      | 1             | 40              | CursorDown(1) + Fwd(40) |
    // | 160      | 2             | 0               | CursorDown(2) only      |
    // | 240      | 3             | 0               | CursorDown(3) only      |
    // | 320      | 4             | 0               | CursorDown(4) only      |
    //
    // The key regression this catches: emitting `CursorForward(0)` when column
    // is 0. ANSI terminals interpret `CSI 0 C` as `CSI 1 C` (move 1 right),
    // causing a spurious 1-column offset.
    // ========================================================================

    /// Test positions at exact terminal width boundaries (column = 0).
    ///
    /// These MUST emit `CursorDown(n)` only, with NO `CursorForward`.
    #[test_case(80, 1  ; "80 cols = 1 row boundary")]
    #[test_case(160, 2 ; "160 cols = 2 row boundary")]
    #[test_case(240, 3 ; "240 cols = 3 row boundary")]
    #[test_case(320, 4 ; "320 cols = 4 row boundary")]
    fn test_move_from_beginning_at_row_boundary(position: u16, expected_rows: u16) {
        let line_state = LineState::new(String::new(), (80, 100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .move_from_beginning(&mut stdout_mock, width(position))
            .unwrap();

        let output_str = stdout_mock.get_copy_of_buffer_as_string();

        // Must emit CursorDown with correct row count.
        let expected_cursor_down = format!("{CSI_START}{expected_rows}{CUD_CURSOR_DOWN}");
        assert!(
            output_str.contains(&expected_cursor_down),
            "position={position}: expected CursorDown({expected_rows}), got: {output_str:?}"
        );

        // Must NOT emit CursorForward (regression guard).
        assert!(
            !contains_cursor_forward(&output_str),
            "position={position}: spurious CursorForward detected (off-by-one bug), got: {output_str:?}"
        );
    }

    /// Test position 0: should emit NO movement at all.
    #[test]
    fn test_move_from_beginning_at_zero() {
        let line_state = LineState::new(String::new(), (80, 100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .move_from_beginning(&mut stdout_mock, width(0))
            .unwrap();

        let output_str = stdout_mock.get_copy_of_buffer_as_string();

        // No ANSI sequences should be emitted.
        assert!(
            !output_str.contains(CSI_START),
            "position=0: expected no ANSI sequences, got: {output_str:?}"
        );
    }

    /// Test positions within a single row (no row crossing).
    ///
    /// These MUST emit `CursorForward(n)` only, with NO `CursorDown`.
    #[test_case(5, 5   ; "5 cols = just column movement")]
    #[test_case(40, 40 ; "40 cols = half row")]
    #[test_case(79, 79 ; "79 cols = last column before wrap")]
    fn test_move_from_beginning_within_first_row(position: u16, expected_cols: u16) {
        let line_state = LineState::new(String::new(), (80, 100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .move_from_beginning(&mut stdout_mock, width(position))
            .unwrap();

        let output_str = stdout_mock.get_copy_of_buffer_as_string();

        // Must emit CursorForward with correct column count.
        let expected_cursor_forward = format!("{CSI_START}{expected_cols}{CUF_CURSOR_FORWARD}");
        assert!(
            output_str.contains(&expected_cursor_forward),
            "position={position}: expected CursorForward({expected_cols}), got: {output_str:?}"
        );

        // Must NOT emit CursorDown.
        assert!(
            !contains_cursor_down(&output_str),
            "position={position}: unexpected CursorDown, got: {output_str:?}"
        );
    }

    /// Test positions that cross rows AND have non-zero column offset.
    ///
    /// These MUST emit both `CursorDown(n)` AND `CursorForward(m)`.
    #[test_case(120, 1, 40 ; "120 = 1 row + 40 cols")]
    #[test_case(200, 2, 40 ; "200 = 2 rows + 40 cols")]
    #[test_case(81, 1, 1   ; "81 = 1 row + 1 col (just past boundary)")]
    fn test_move_from_beginning_row_and_column(
        position: u16,
        expected_rows: u16,
        expected_cols: u16,
    ) {
        let line_state = LineState::new(String::new(), (80, 100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .move_from_beginning(&mut stdout_mock, width(position))
            .unwrap();

        let output_str = stdout_mock.get_copy_of_buffer_as_string();

        // Must emit both CursorDown and CursorForward.
        let expected_cursor_down = format!("{CSI_START}{expected_rows}{CUD_CURSOR_DOWN}");
        let expected_cursor_forward = format!("{CSI_START}{expected_cols}{CUF_CURSOR_FORWARD}");

        assert!(
            output_str.contains(&expected_cursor_down),
            "position={position}: expected CursorDown({expected_rows}), got: {output_str:?}"
        );
        assert!(
            output_str.contains(&expected_cursor_forward),
            "position={position}: expected CursorForward({expected_cols}), got: {output_str:?}"
        );
    }
}

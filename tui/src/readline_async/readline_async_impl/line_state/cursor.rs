// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::LineState;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CsiSequence, CursorBoundsCheck,
            NarrowingCastToU16, NumericValue, Seg, TermCol, TermColDelta, TermRowDelta,
            VPCol, VPWidth, inline_string, ok, seg_index, seg_length, term_col_delta,
            term_row_delta, vp_col};
use std::io::{self, Write};

impl LineState {
    /// Moves the terminal cursor to the beginning of the input line.
    ///
    /// This is used before re-rendering or when the cursor position needs to be
    /// recalculated from the start.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn paint_cursor_rewind_to_start(&self, term: &mut dyn Write) -> io::Result<()> {
        let cursor_distance_from_start =
            self.calc_current_column().distance_from(vp_col(0));
        self.paint_cursor_to_start_from(term, cursor_distance_from_start)?;
        ok!()
    }

    /// Moves the terminal cursor from the beginning to the current cursor position.
    ///
    /// This is typically called after [`paint_cursor_rewind_to_start`] to restore the
    /// cursor to its logical position within the line.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    ///
    /// [`paint_cursor_rewind_to_start`]: Self::paint_cursor_rewind_to_start
    pub fn paint_cursor_at_current_column(&self, term: &mut dyn Write) -> io::Result<()> {
        let cursor_distance_from_start =
            self.calc_current_column().distance_from(vp_col(0));
        self.paint_cursor_from_start_to(term, cursor_distance_from_start)?;
        ok!()
    }

    /// Gets the row delta (how many wrapped rows down) from the start of the line.
    ///
    /// The `pos` parameter is a display offset (column width) from the start of the line.
    ///
    /// # Returns
    ///
    /// A [`TermRowDelta`] representing how many rows down the position is. Returns `None`
    /// if the calculated delta is zero (position is on the first line).
    #[must_use]
    pub fn calc_row_delta_from_start_to(&self, pos: VPWidth) -> Option<TermRowDelta> {
        term_row_delta(pos / self.term_size.col_width)
    }

    /// Gets the column offset within the current row from the start of the line.
    ///
    /// The `pos` parameter is a display offset (column width) from the start of the line.
    ///
    /// # Returns
    ///
    /// A [`TermColDelta`] representing the horizontal position within the row.
    /// Returns `None` if the calculated delta is zero (position is at the start of a
    /// row).
    #[must_use]
    pub fn calc_col_delta_from_start_to(&self, pos: VPWidth) -> Option<TermColDelta> {
        term_col_delta(pos % self.term_size.col_width)
    }

    /// Move from a position on the line to the start.
    ///
    /// The `from` parameter is a display offset (column width) from the start of the
    /// line.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn paint_cursor_to_start_from(
        &self,
        term: &mut dyn Write,
        from: VPWidth,
    ) -> io::Result<()> {
        // Calculate row delta from position.
        // Position 80 on 80-col terminal is Row 1, Col 0: 80/80 = 1 row.
        let move_up = self.calc_row_delta_from_start_to(from);

        // Move to column 1 (CHA = Cursor Horizontal Absolute, 1-based).
        term.write_all(
            inline_string!("{}", CsiSequence::CursorHorizontalAbsolute(TermCol::ONE))
                .as_bytes(),
        )?;

        // Move up the calculated number of rows (CUU = Cursor Up).
        // Only emit if Some (non-zero) - guards against CSI zero bug.
        if let Some(delta) = move_up {
            term.write_all(
                inline_string!("{}", CsiSequence::CursorUp(delta)).as_bytes(),
            )?;
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
    pub fn paint_cursor_from_start_to(
        &self,
        term: &mut dyn Write,
        to: VPWidth,
    ) -> io::Result<()> {
        // Calculate deltas from position.
        // Position 80 on 80-col terminal is Row 1, Col 0: 80/80 = 1 row, 80%80 = 0 cols.
        let rows_down = self.calc_row_delta_from_start_to(to);
        let cols_right = self.calc_col_delta_from_start_to(to);

        // Move down the calculated number of rows (CUD = Cursor Down).
        // Only emit if Some (non-zero) - guards against CSI zero bug.
        if let Some(delta) = rows_down {
            term.write_all(
                inline_string!("{}", CsiSequence::CursorDown(delta)).as_bytes(),
            )?;
        }

        // Move right to the column position (CUF = Cursor Forward).
        // Only emit if Some (non-zero) - guards against CSI zero bug where
        // CursorForward(0) is interpreted as CursorForward(1) by terminals.
        if let Some(delta) = cols_right {
            term.write_all(
                inline_string!("{}", CsiSequence::CursorForward(delta)).as_bytes(),
            )?;
        }

        ok!()
    }

    /// Shifts the logical cursor by the given number of unicode grapheme segments either
    /// left (negative) or right (positive).
    ///
    /// This is an infallible, in-memory state update that does not perform any terminal
    /// I/O. To update the physical terminal cursor after shifting, invoke
    /// [`paint_cursor_at_current_column`].
    ///
    /// [`paint_cursor_at_current_column`]: Self::paint_cursor_at_current_column
    pub fn shift_logical_cursor_by(&mut self, seg_delta: isize) {
        if seg_delta > 0 {
            let count = self.line.segment_count();
            let seg_delta_u16 = seg_delta.as_u16_narrowing();
            let new_position = self.cursor_position + seg_index(seg_delta_u16);
            // Use CursorBoundsCheck for text cursor positioning (allows position ==
            // length).
            self.cursor_position = count.clamp_cursor_position(new_position);
        } else {
            // Use unsigned_abs() to convert negative seg_delta to positive amount to
            // subtract.
            let seg_delta_idx = seg_index(seg_delta.unsigned_abs().as_u16_narrowing());
            self.cursor_position = if seg_delta_idx
                .overflows(self.cursor_position.convert_to_seg_length())
                == ArrayOverflowResult::Overflowed
            {
                seg_index(0)
            } else {
                self.cursor_position - seg_delta_idx
            };
        }
    }

    /// Moves the logical cursor to the beginning of the line buffer (position 0).
    pub fn move_logical_cursor_to_start(&mut self) {
        self.cursor_position = seg_index(0);
    }

    /// Moves the logical cursor to the end of the line buffer (after the last grapheme).
    pub fn move_logical_cursor_to_end(&mut self) {
        self.cursor_position = self.line.segment_count().eol_cursor_position();
    }

    /// Calculates the 0-based terminal column position for the physical cursor.
    ///
    /// The physical cursor column is derived by starting at the origin ([`vp_col(0)`])
    /// and adding both the prompt width and the display width of the buffer content
    /// preceding the logical cursor:
    ///
    /// ```text
    /// [ prompt.width() ] [ calc_display_width_up_to_cursor() ]
    /// 0 -----------------------------------------------------> current_column
    /// ```
    ///
    /// Because grapheme clusters vary in display width (e.g. [`ASCII`] is 1 column, wide
    /// characters and emojis are 2 columns, zero-width joiners are 0 columns), the
    /// column position is computed in display cells rather than byte or segment indices.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`vp_col(0)`]: crate::vp_col
    #[must_use]
    pub fn calc_current_column(&self) -> VPCol {
        let line_display_width = self.calc_display_width_up_to_cursor();
        vp_col(0) + self.prompt.width() + line_display_width
    }

    /// Calculates the total display width of buffer content up to the current logical
    /// cursor position.
    ///
    /// Uses pre-computed grapheme cluster segment metadata from [`GCStringOwned`] to
    /// achieve an `O(1)` direct array lookup, rather than iterating or re-parsing the
    /// underlying text.
    ///
    /// [`GCStringOwned`]: crate::GCStringOwned
    fn calc_display_width_up_to_cursor(&self) -> VPWidth {
        match self.line.get(self.cursor_position) {
            // Cursor is positioned in front of an existing segment. The display width up
            // to the cursor is the start column of this segment.
            Some(seg) => seg.start_display_col_index.distance_from(vp_col(0)),

            // Cursor is at or past the end of the line (or the line is empty). There is
            // no segment at this index, so the display width is the full line width.
            None => self.line.display_width(),
        }
    }

    /// Returns the grapheme cluster segment immediately before the cursor position.
    ///
    /// Returns `None` if the cursor is at the beginning of the line (position 0).
    ///
    /// # Returns
    ///
    /// A [`Seg`] containing byte offset, display width, and other segment metadata.
    /// Use [`seg.get_str(&self.line)`] to get the actual grapheme string.
    ///
    /// [`seg.get_str(&self.line)`]: crate::Seg::get_str
    #[must_use]
    pub fn grapheme_before_cursor(&self) -> Option<Seg> {
        if self.cursor_position.is_zero() {
            return None;
        }
        self.line.get(self.cursor_position - seg_length(1))
    }

    /// Returns the grapheme cluster segment at the cursor position (to be deleted by
    /// Delete key).
    ///
    /// Returns `None` if the cursor is at the end of the line.
    ///
    /// # Returns
    ///
    /// A [`Seg`] containing byte offset, display width, and other segment metadata. Use
    /// [`seg.get_str(&self.line)`] to get the actual grapheme string.
    ///
    /// [`seg.get_str(&self.line)`]: crate::Seg::get_str
    #[must_use]
    pub fn grapheme_at_cursor(&self) -> Option<Seg> {
        let total = self.line.segment_count();
        if self.cursor_position.overflows(total) == ArrayOverflowResult::Overflowed {
            return None;
        }
        self.line.get(self.cursor_position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ANSI_CSI_BRACKET, CSI_START, CUD_CURSOR_DOWN, CUF_CURSOR_FORWARD,
                ESC_START, core::test_fixtures::StdoutMock, vp_height, vp_width};
    use test_case::test_case;

    /// Checks if the string contains a `CursorForward` sequence ([`CSI`] <n> C).
    ///
    /// [`CSI`]: crate::CsiSequence
    fn contains_move_cursor_right(s: &str) -> bool {
        // CSI sequences start with ESC [ and CursorForward ends with 'C'.
        // We look for patterns like "\x1b[5C" or "\x1b[10C".
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == ESC_START && chars.next() == Some(char::from(ANSI_CSI_BRACKET)) {
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

    /// Checks if the string contains a `CursorDown` sequence ([`CSI`] <n> B).
    ///
    /// [`CSI`]: crate::CsiSequence
    fn contains_move_cursor_down(s: &str) -> bool {
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == ESC_START && chars.next() == Some(char::from(ANSI_CSI_BRACKET)) {
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
    // Terminal boundary regression tests for `paint_cursor_from_start_to`.
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
    fn test_paint_cursor_from_start_to_at_row_boundary(
        position: u16,
        expected_rows: u16,
    ) {
        let line_state = LineState::new(String::new(), vp_width(80) + vp_height(100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .paint_cursor_from_start_to(&mut stdout_mock, vp_width(position))
            .unwrap_or_default();

        let output_str = stdout_mock.get_copy_of_buffer_as_string();

        // Must emit CursorDown with correct row count.
        let expected_move_cursor_down =
            format!("{CSI_START}{expected_rows}{CUD_CURSOR_DOWN}");
        assert!(
            output_str.contains(&expected_move_cursor_down),
            "position={position}: expected CursorDown({expected_rows}), got: {output_str:?}"
        );

        // Must NOT emit CursorForward (regression guard).
        assert!(
            !contains_move_cursor_right(&output_str),
            "position={position}: spurious CursorForward detected (off-by-one bug), got: {output_str:?}"
        );
    }

    /// Test position 0: should emit NO movement at all.
    #[test]
    fn test_paint_cursor_from_start_to_at_zero() {
        let line_state = LineState::new(String::new(), vp_width(80) + vp_height(100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .paint_cursor_from_start_to(&mut stdout_mock, vp_width(0))
            .unwrap_or_default();

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
    fn test_paint_cursor_from_start_to_within_first_row(
        position: u16,
        expected_cols: u16,
    ) {
        let line_state = LineState::new(String::new(), vp_width(80) + vp_height(100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .paint_cursor_from_start_to(&mut stdout_mock, vp_width(position))
            .unwrap_or_default();

        let output_str = stdout_mock.get_copy_of_buffer_as_string();

        // Must emit CursorForward with correct column count.
        let expected_move_cursor_right =
            format!("{CSI_START}{expected_cols}{CUF_CURSOR_FORWARD}");
        assert!(
            output_str.contains(&expected_move_cursor_right),
            "position={position}: expected CursorForward({expected_cols}), got: {output_str:?}"
        );

        // Must NOT emit CursorDown.
        assert!(
            !contains_move_cursor_down(&output_str),
            "position={position}: unexpected CursorDown, got: {output_str:?}"
        );
    }

    /// Test positions that cross rows AND have non-zero column offset.
    ///
    /// These MUST emit both `CursorDown(n)` AND `CursorForward(m)`.
    #[test_case(120, 1, 40 ; "120 = 1 row + 40 cols")]
    #[test_case(200, 2, 40 ; "200 = 2 rows + 40 cols")]
    #[test_case(81, 1, 1   ; "81 = 1 row + 1 col (just past boundary)")]
    fn test_paint_cursor_from_start_to_row_and_column(
        position: u16,
        expected_rows: u16,
        expected_cols: u16,
    ) {
        let line_state = LineState::new(String::new(), vp_width(80) + vp_height(100));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .paint_cursor_from_start_to(&mut stdout_mock, vp_width(position))
            .unwrap_or_default();

        let output_str = stdout_mock.get_copy_of_buffer_as_string();

        // Must emit both CursorDown and CursorForward.
        let expected_move_cursor_down =
            format!("{CSI_START}{expected_rows}{CUD_CURSOR_DOWN}");
        let expected_move_cursor_right =
            format!("{CSI_START}{expected_cols}{CUF_CURSOR_FORWARD}");

        assert!(
            output_str.contains(&expected_move_cursor_down),
            "position={position}: expected CursorDown({expected_rows}), got: {output_str:?}"
        );
        assert!(
            output_str.contains(&expected_move_cursor_right),
            "position={position}: expected CursorForward({expected_cols}), got: {output_str:?}"
        );
    }

    #[test]
    fn test_calc_display_width_up_to_cursor() {
        let mut line_state = LineState::new(String::new(), vp_width(80) + vp_height(100));

        // Empty line.
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(0));

        // ASCII line: "hello" (5 chars, each 1 col).
        line_state.line = "hello".into();
        line_state.cursor_position = seg_index(0);
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(0));
        line_state.cursor_position = seg_index(2);
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(2));
        line_state.cursor_position = seg_index(5);
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(5));
        line_state.cursor_position = seg_index(10); // Beyond end.
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(5));

        // Unicode line with wide characters: "📦🙏🏽" (each 2 cols).
        line_state.line = "📦🙏🏽".into();
        line_state.cursor_position = seg_index(0);
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(0));
        line_state.cursor_position = seg_index(1);
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(2));
        line_state.cursor_position = seg_index(2);
        assert_eq!(line_state.calc_display_width_up_to_cursor(), vp_width(4));
    }

    #[test]
    fn test_logical_cursor_movement() {
        let mut line_state =
            LineState::new("prompt> ".into(), vp_width(80) + vp_height(100));
        line_state.line = "hello 🌍".into();

        // Prompt is 8 display columns.
        assert_eq!(line_state.calc_current_column(), vp_col(8));
        assert_eq!(line_state.cursor_position, seg_index(0));

        // Shift forward by 2 grapheme segments.
        line_state.shift_logical_cursor_by(2);
        assert_eq!(line_state.cursor_position, seg_index(2));
        assert_eq!(line_state.calc_current_column(), vp_col(10));

        // Shift beyond end clamps to end (7 segments: 'h','e','l','l','o',' ','🌍').
        line_state.shift_logical_cursor_by(100);
        assert_eq!(line_state.cursor_position, seg_index(7));
        // "hello " is 6 cols + "🌍" is 2 cols = 8 cols + prompt (8) = 16.
        assert_eq!(line_state.calc_current_column(), vp_col(16));

        // Move to start.
        line_state.move_logical_cursor_to_start();
        assert_eq!(line_state.cursor_position, seg_index(0));
        assert_eq!(line_state.calc_current_column(), vp_col(8));

        // Move to end.
        line_state.move_logical_cursor_to_end();
        assert_eq!(line_state.cursor_position, seg_index(7));
        assert_eq!(line_state.calc_current_column(), vp_col(16));

        // Shift backward by 1.
        line_state.shift_logical_cursor_by(-1);
        assert_eq!(line_state.cursor_position, seg_index(6));
        assert_eq!(line_state.calc_current_column(), vp_col(14));

        // Shift backward past start clamps to 0.
        line_state.shift_logical_cursor_by(-100);
        assert_eq!(line_state.cursor_position, seg_index(0));
        assert_eq!(line_state.calc_current_column(), vp_col(8));
    }
}

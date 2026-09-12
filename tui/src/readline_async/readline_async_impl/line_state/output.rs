// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::{EndsWithNewline, LineState};
use crate::{CsiSequence, GCStringOwned, LINE_FEED_BYTE, NarrowingCastToU16,
            ReadlineError, TermCol, TermColDelta, TermRowDelta, early_return_if_paused,
            inline_string, ok, vp_width};
use std::io::Write;

impl LineState {
    /// Prints raw byte data to the terminal and re-renders the prompt.
    ///
    /// This method handles the complex task of printing output from concurrent tasks
    /// (via [`SharedWriter`]) while maintaining the readline prompt display. It:
    ///
    /// 1. Clears the current line
    /// 2. Restores cursor position if previous output didn't end with newline
    /// 3. Writes the data with proper newline handling
    /// 4. Re-renders the prompt and input line
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    ///
    /// [`SharedWriter`]: crate::SharedWriter
    pub fn print_data_and_flush(
        &mut self,
        data: &[u8],
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        self.clear(term)?;

        // If last written data was not newline, restore the cursor.
        if self.ends_with_newline == EndsWithNewline::No {
            // Move up 1 row, to column 0, then right to the last position.
            term.write_all(
                inline_string!("{}", CsiSequence::CursorUp(TermRowDelta::ONE)).as_bytes(),
            )?;
            term.write_all(
                inline_string!("{}", CsiSequence::CursorHorizontalAbsolute(TermCol::ONE))
                    .as_bytes(),
            )?;
            // Only emit CursorForward if the delta is non-zero (illegal states
            // unrepresentable).
            if let Some(cols_right) = TermColDelta::new(self.last_line_length.as_u16()) {
                term.write_all(
                    inline_string!("{}", CsiSequence::CursorForward(cols_right))
                        .as_bytes(),
                )?;
            }
        }

        // In raw mode, a Line Feed (LF, '\n') only moves the cursor down one row without
        // performing an automatic Carriage Return (CR). To prevent a "staircase" effect
        // where subsequent lines remain indented, we explicitly move the cursor back to
        // column 1 after each newline using Cursor Horizontal Absolute: CHA(1) stored in
        // variable `cha_1`.
        //
        // Deduping the final CHA(1): If the final segment ends with a newline, emitting
        // CHA(1) here would be followed immediately by `render_and_flush()` emitting its
        // own CHA(1) when redrawing the prompt. That back-to-back duplicate sequence: LF
        // -> CHA(1) -> CHA(1), causes visual artifacts (such as an extra blank line) on
        // some terminal emulators. Therefore, we skip emitting CHA(1) here on the last
        // segment if it ends with a newline.
        let cha_1 =
            inline_string!("{}", CsiSequence::CursorHorizontalAbsolute(TermCol::ONE));
        let segments: Vec<_> = data.split_inclusive(|b| *b == LINE_FEED_BYTE).collect();
        let last_idx = segments.len().saturating_sub(1);
        for (idx, line) in segments.into_iter().enumerate() {
            term.write_all(line)?;
            // Emit CHA(1) after each segment, unless this is the final newline-terminated
            // segment (which is positioned by the upcoming `render_and_flush()`).
            let is_last = idx == last_idx;
            let ends_with_newline = line.ends_with(&[LINE_FEED_BYTE]);
            if !(is_last && ends_with_newline) {
                term.write_all(cha_1.as_bytes())?;
            }
        }

        // Set whether data ends with newline.
        self.ends_with_newline = if data.ends_with(&[LINE_FEED_BYTE]) {
            EndsWithNewline::Yes
        } else {
            EndsWithNewline::No
        };

        // If data does not end with newline, save the cursor and write newline for
        // prompt. Usually data does end in newline due to the buffering of
        // SharedWriter, but sometimes it may not (i.e. if .flush() is called).
        match self.ends_with_newline {
            EndsWithNewline::Yes => {
                self.last_line_length = vp_width(0);
            }
            EndsWithNewline::No => {
                // Add data length to last_line_length.
                let new_len = self.last_line_length.as_usize() + data.len();
                let term_width = self.term_size.col_width.as_usize();
                // Make sure that last_line_length wraps around when doing multiple
                // writes.
                if new_len >= term_width {
                    self.last_line_length =
                        vp_width((new_len % term_width).as_u16_narrowing());
                    term.write_all(&[LINE_FEED_BYTE])?;
                } else {
                    self.last_line_length = vp_width((new_len).as_u16_narrowing());
                }
                term.write_all(&[LINE_FEED_BYTE])?; // Move to beginning of line and make new line
            }
        }

        term.write_all(
            inline_string!("{}", CsiSequence::CursorHorizontalAbsolute(TermCol::ONE))
                .as_bytes(),
        )?;
        self.render_and_flush(term)?;

        ok!()
    }

    /// Prints a string to the terminal and re-renders the prompt.
    ///
    /// This is a convenience wrapper around
    /// [`print_data_and_flush`] that accepts a string
    /// slice. Respects pause state - does nothing if paused.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    ///
    /// [`print_data_and_flush`]: Self::print_data_and_flush
    pub fn print_and_flush(
        &mut self,
        string: &str,
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        early_return_if_paused!(self @Unit);

        self.print_data_and_flush(string.as_bytes(), term)?;

        ok!()
    }

    /// Updates the prompt string and re-renders the line.
    ///
    /// Use this to dynamically change the prompt (e.g., to show current directory
    /// or command status).
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn update_prompt(
        &mut self,
        prompt: &str,
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        self.clear(term)?;
        self.prompt.set(prompt);
        self.render_and_flush(term)?;

        ok!()
    }

    /// Clears the line state and prepares the terminal for exit.
    ///
    /// Called when the user presses Ctrl+C or Ctrl+D. Clears the current line
    /// content and moves the cursor to column 0.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn exit(&mut self, term: &mut dyn Write) -> Result<(), ReadlineError> {
        self.line = GCStringOwned::new("");
        self.clear(term)?;

        term.write_all(
            inline_string!("{}", CsiSequence::CursorHorizontalAbsolute(TermCol::ONE))
                .as_bytes(),
        )?;
        term.flush()?;

        ok!()
    }

    /// Moves cursor to the beginning and re-renders the line.
    ///
    /// Used after submitting a line (pressing Enter) to start fresh on a new line.
    /// Respects pause state - does nothing if paused.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn render_new_line_from_beginning_and_flush(
        &mut self,
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        early_return_if_paused!(self @Unit);

        self.move_logical_cursor_to_start();
        self.clear_and_render_and_flush(term)?;

        ok!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::test_fixtures::StdoutMock, vp_height, vp_width};
    use smallvec::SmallVec;

    /// Helper to decode [`ANSI`] escape sequences in output for debugging.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    fn describe_ansi_output(output: &[u8]) -> String {
        use std::fmt::Write;

        let mut result = String::new();
        let mut i = 0;
        while i < output.len() {
            if output[i] == 0x1b && i + 1 < output.len() && output[i + 1] == b'[' {
                // Parse CSI sequence.
                let start = i;
                i += 2;
                let mut params = String::new();
                while i < output.len()
                    && (output[i].is_ascii_digit() || output[i] == b';')
                {
                    params.push(char::from(output[i]));
                    i += 1;
                }
                if i < output.len() {
                    let cmd = char::from(output[i]);
                    let desc = match cmd {
                        'A' => format!("CursorUp({params})"),
                        'B' => format!("CursorDown({params})"),
                        'C' => format!("CursorForward({params})"),
                        'D' => format!("CursorBackward({params})"),
                        'G' => format!("CHA({params})"),
                        'H' => format!("CUP({params})"),
                        'J' => format!("EraseDisplay({params})"),
                        'K' => format!("EraseLine({params})"),
                        _ => format!("CSI[{params}{cmd}]"),
                    };
                    write!(result, "[{desc}]").expect("conversion error");
                    i += 1;
                } else {
                    write!(result, "[CSI:incomplete@{start}]").expect("conversion error");
                }
            } else if output[i] == b'\n' {
                result.push_str("[LF]");
                i += 1;
            } else if output[i] == b'\r' {
                result.push_str("[CR]");
                i += 1;
            } else if output[i].is_ascii_graphic() || output[i] == b' ' {
                result.push(char::from(output[i]));
                i += 1;
            } else {
                write!(result, "[0x{:02x}]", output[i]).expect("conversion error");
                i += 1;
            }
        }
        result
    }

    /// Regression test for issue #442: extra blank line before prompt.
    ///
    /// Verifies that `print_data_and_flush` with newline-terminated data produces
    /// exactly one LF in the output, preventing extra blank lines before the prompt.
    #[test]
    fn test_print_data_no_extra_newlines_issue_442() {
        let mut line_state = LineState::new("> ".into(), vp_width(80) + vp_height(24));
        let mut stdout_mock = StdoutMock::default();

        // Simulate initial state: prompt has been rendered.
        line_state
            .render_and_flush(&mut stdout_mock)
            .expect("conversion error");
        stdout_mock.buffer.write(SmallVec::clear);

        // First log line (ends with newline).
        line_state
            .print_data_and_flush(b"line 1\n", &mut stdout_mock)
            .expect("conversion error");

        // Verify ends_with_newline is Yes.
        assert_eq!(
            line_state.ends_with_newline,
            EndsWithNewline::Yes,
            "ends_with_newline should be Yes after newline-terminated data"
        );

        // Clear buffer for second call.
        stdout_mock.buffer.write(SmallVec::clear);

        // Second log line (ends with newline).
        line_state
            .print_data_and_flush(b"line 2\n", &mut stdout_mock)
            .expect("conversion error");

        // Verify the stripped output has exactly 1 newline.
        let stripped = stdout_mock.get_copy_of_buffer_as_string_strip_ansi();
        let newline_count = stripped.matches('\n').count();
        assert_eq!(
            newline_count, 1,
            "Expected exactly 1 newline in output, got {newline_count}. Stripped: {stripped:?}"
        );

        // Verify the escape sequence pattern doesn't have redundant CHA(1) after LF.
        let decoded = describe_ansi_output(&stdout_mock.get_copy_of_buffer());
        // After fix: should be [LF][CHA(1)] not [LF][CHA(1)][CHA(1)].
        assert!(
            !decoded.contains("[LF][CHA(1)][CHA(1)]"),
            "Redundant CHA(1) after LF detected. Decoded: {decoded}"
        );
    }

    /// Regression test: verify partial line writes still work correctly.
    ///
    /// When data doesn't end with newline (e.g., manual `.flush()` call), the code
    /// should still emit [`CHA(1)`] to ensure proper cursor positioning.
    ///
    /// [`CHA(1)`]: crate::CsiSequence::CursorHorizontalAbsolute
    #[test]
    fn test_print_data_partial_line_emits_cha() {
        let mut line_state = LineState::new("> ".into(), vp_width(80) + vp_height(24));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .render_and_flush(&mut stdout_mock)
            .expect("conversion error");
        stdout_mock.buffer.write(SmallVec::clear);

        // Partial line (no newline at end).
        line_state
            .print_data_and_flush(b"partial", &mut stdout_mock)
            .expect("conversion error");

        // Verify ends_with_newline is No.
        assert_eq!(
            line_state.ends_with_newline,
            EndsWithNewline::No,
            "ends_with_newline should be No after non-newline data"
        );

        // Verify CHA(1) is emitted after the data for partial lines.
        let decoded = describe_ansi_output(&stdout_mock.get_copy_of_buffer());
        // For partial lines, we should see: data + CHA(1) + LF + CHA(1) + prompt.
        assert!(
            decoded.contains("partial[CHA(1)]"),
            "CHA(1) should be emitted after partial line data. Decoded: {decoded}"
        );
    }

    /// Test multiple segments in a single write (e.g., "line1\nline2\n").
    ///
    /// In raw terminal mode, LF only moves the cursor down without returning to
    /// column 1. Therefore, we need [`CHA(1)`] after each line segment to ensure
    /// subsequent lines start at column 1. The only exception is the final segment
    /// when it ends with newline - we skip [`CHA(1)`] there to avoid double [`CHA(1)`]
    /// with the one emitted before `render_and_flush`.
    ///
    /// [`CHA(1)`]: crate::CsiSequence::CursorHorizontalAbsolute
    #[test]
    fn test_print_data_multiple_segments() {
        let mut line_state = LineState::new("> ".into(), vp_width(80) + vp_height(24));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .render_and_flush(&mut stdout_mock)
            .expect("conversion error");
        stdout_mock.buffer.write(SmallVec::clear);

        // Multiple lines in single call.
        line_state
            .print_data_and_flush(b"line1\nline2\n", &mut stdout_mock)
            .expect("conversion error");

        let decoded = describe_ansi_output(&stdout_mock.get_copy_of_buffer());
        // After first LF, we need CHA(1) so line2 starts at column 1.
        // After second LF, we skip CHA(1) since render_and_flush handles it.
        // Expected pattern: line1[LF][CHA(1)]line2[LF][CHA(1)]> .
        assert!(
            decoded.contains("line1[LF][CHA(1)]line2[LF]"),
            "First line should have [LF][CHA(1)] to return cursor to column 1. Decoded: {decoded}"
        );
        // Should NOT have double CHA(1) after the final LF.
        assert!(
            !decoded.contains("line2[LF][CHA(1)][CHA(1)]"),
            "Should not have redundant CHA(1) after final LF. Decoded: {decoded}"
        );
    }

    #[test]
    fn test_exit_clears_line() {
        let mut line_state = LineState::new("$ ".into(), vp_width(80) + vp_height(24));
        line_state.line = GCStringOwned::new("some content");
        let mut stdout_mock = StdoutMock::default();

        line_state.exit(&mut stdout_mock).expect("conversion error");

        // Line should be cleared.
        assert!(line_state.line.is_empty());
    }

    #[test]
    fn test_update_prompt_changes_prompt() {
        let mut line_state = LineState::new("old> ".into(), vp_width(80) + vp_height(24));
        let mut stdout_mock = StdoutMock::default();

        line_state
            .update_prompt("new> ", &mut stdout_mock)
            .expect("conversion error");

        assert_eq!(line_state.prompt.as_str(), "new> ");
        assert_eq!(line_state.prompt.width(), vp_width(5));
    }

    #[test]
    fn test_print_data_sets_ends_with_newline() {
        let mut line_state = LineState::new("$ ".into(), vp_width(80) + vp_height(24));
        let mut stdout_mock = StdoutMock::default();

        // Data ending with newline.
        line_state
            .print_data_and_flush(b"hello\n", &mut stdout_mock)
            .expect("conversion error");
        assert_eq!(line_state.ends_with_newline, EndsWithNewline::Yes);

        // Data not ending with newline.
        line_state
            .print_data_and_flush(b"world", &mut stdout_mock)
            .expect("conversion error");
        assert_eq!(line_state.ends_with_newline, EndsWithNewline::No);
    }
}

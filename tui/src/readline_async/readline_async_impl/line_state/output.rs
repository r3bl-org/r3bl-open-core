// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::LineState;
use crate::{CsiSequence, GCStringOwned, LINE_FEED_BYTE, LineStateLiveness, ReadlineError,
            TermCol, TermColDelta, TermRowDelta, early_return_if_paused, ok, width};
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
        if !self.last_line_completed {
            // Move up 1 row, to column 0, then right to the last position.
            term.write_all(
                CsiSequence::CursorUp(TermRowDelta::ONE)
                    .to_string()
                    .as_bytes(),
            )?;
            term.write_all(
                CsiSequence::CursorHorizontalAbsolute(TermCol::ONE)
                    .to_string()
                    .as_bytes(),
            )?;
            // Only emit CursorForward if the delta is non-zero (illegal states unrepresentable).
            if let Some(cols_right) = TermColDelta::new(self.last_line_length.as_u16()) {
                term.write_all(CsiSequence::CursorForward(cols_right).to_string().as_bytes())?;
            }
        }

        // Write data in a way that newlines also act as carriage returns.
        // In raw mode, LF doesn't auto-CR, so we explicitly move to column 1 after
        // every segment. This ensures multi-line output displays correctly.
        //
        // For the last segment only: if it ends with newline, we skip the CHA(1)
        // here since the subsequent CHA(1) before render_and_flush handles it.
        // This avoids a redundant [LF][CHA(1)][CHA(1)] pattern that can cause
        // visual artifacts (extra blank line) on some terminal emulators.
        let col_0 = CsiSequence::CursorHorizontalAbsolute(TermCol::ONE).to_string();
        let segments: Vec<_> = data.split_inclusive(|b| *b == LINE_FEED_BYTE).collect();
        let last_idx = segments.len().saturating_sub(1);
        for (idx, line) in segments.into_iter().enumerate() {
            term.write_all(line)?;
            // Emit CHA(1) after every segment EXCEPT the last one if it ends with
            // newline. The final CHA(1) before render_and_flush handles that case.
            let is_last = idx == last_idx;
            let ends_with_newline = line.ends_with(&[LINE_FEED_BYTE]);
            if !(is_last && ends_with_newline) {
                term.write_all(col_0.as_bytes())?;
            }
        }

        self.last_line_completed = data.ends_with(&[LINE_FEED_BYTE]); // Set whether data ends with newline

        // If data does not end with newline, save the cursor and write newline for
        // prompt. Usually data does end in newline due to the buffering of
        // SharedWriter, but sometimes it may not (i.e. if .flush() is called).
        if self.last_line_completed {
            self.last_line_length = width(0);
        } else {
            // Add data length to last_line_length.
            let new_len = self.last_line_length.as_usize() + data.len();
            let term_width = self.term_size.col_width.as_usize();
            // Make sure that last_line_length wraps around when doing multiple writes.
            if new_len >= term_width {
                self.last_line_length = width(new_len % term_width);
                writeln!(term)?;
            } else {
                self.last_line_length = width(new_len);
            }
            writeln!(term)?; // Move to beginning of line and make new line
        }

        term.write_all(
            CsiSequence::CursorHorizontalAbsolute(TermCol::ONE)
                .to_string()
                .as_bytes(),
        )?;
        self.render_and_flush(term)?;

        ok!()
    }

    /// Prints a string to the terminal and re-renders the prompt.
    ///
    /// This is a convenience wrapper around
    /// [`print_data_and_flush`](Self::print_data_and_flush) that accepts a string
    /// slice. Respects pause state - does nothing if paused.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
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
        self.prompt.clear();
        self.prompt.push_str(prompt);

        // Recalculates column.
        self.move_cursor(0)?;
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
            CsiSequence::CursorHorizontalAbsolute(TermCol::ONE)
                .to_string()
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

        self.move_cursor(-100_000)?;
        self.clear_and_render_and_flush(term)?;

        ok!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_fixtures::StdoutMock;

    /// Helper to decode ANSI escape sequences in output for debugging.
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
                while i < output.len() && (output[i].is_ascii_digit() || output[i] == b';') {
                    params.push(output[i] as char);
                    i += 1;
                }
                if i < output.len() {
                    let cmd = output[i] as char;
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
                    write!(result, "[{desc}]").unwrap();
                    i += 1;
                } else {
                    write!(result, "[CSI:incomplete@{start}]").unwrap();
                }
            } else if output[i] == b'\n' {
                result.push_str("[LF]");
                i += 1;
            } else if output[i] == b'\r' {
                result.push_str("[CR]");
                i += 1;
            } else if output[i].is_ascii_graphic() || output[i] == b' ' {
                result.push(output[i] as char);
                i += 1;
            } else {
                write!(result, "[0x{:02x}]", output[i]).unwrap();
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
        let mut line_state = LineState::new("> ".into(), (80, 24));
        let mut stdout_mock = StdoutMock::default();

        // Simulate initial state: prompt has been rendered.
        line_state.render_and_flush(&mut stdout_mock).unwrap();
        stdout_mock.buffer.lock().unwrap().clear();

        // First log line (ends with newline).
        line_state
            .print_data_and_flush(b"line 1\n", &mut stdout_mock)
            .unwrap();

        // Verify last_line_completed is true.
        assert!(
            line_state.last_line_completed,
            "last_line_completed should be true after newline-terminated data"
        );

        // Clear buffer for second call.
        stdout_mock.buffer.lock().unwrap().clear();

        // Second log line (ends with newline).
        line_state
            .print_data_and_flush(b"line 2\n", &mut stdout_mock)
            .unwrap();

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
    /// should still emit CHA(1) to ensure proper cursor positioning.
    #[test]
    fn test_print_data_partial_line_emits_cha() {
        let mut line_state = LineState::new("> ".into(), (80, 24));
        let mut stdout_mock = StdoutMock::default();

        line_state.render_and_flush(&mut stdout_mock).unwrap();
        stdout_mock.buffer.lock().unwrap().clear();

        // Partial line (no newline at end).
        line_state
            .print_data_and_flush(b"partial", &mut stdout_mock)
            .unwrap();

        // Verify last_line_completed is false.
        assert!(
            !line_state.last_line_completed,
            "last_line_completed should be false after non-newline data"
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
    /// column 1. Therefore, we need `CHA(1)` after each line segment to ensure
    /// subsequent lines start at column 1. The only exception is the final segment
    /// when it ends with newline - we skip `CHA(1)` there to avoid double `CHA(1)`
    /// with the one emitted before `render_and_flush`.
    #[test]
    fn test_print_data_multiple_segments() {
        let mut line_state = LineState::new("> ".into(), (80, 24));
        let mut stdout_mock = StdoutMock::default();

        line_state.render_and_flush(&mut stdout_mock).unwrap();
        stdout_mock.buffer.lock().unwrap().clear();

        // Multiple lines in single call.
        line_state
            .print_data_and_flush(b"line1\nline2\n", &mut stdout_mock)
            .unwrap();

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
        let mut line_state = LineState::new("$ ".into(), (80, 24));
        line_state.line = GCStringOwned::new("some content");
        let mut stdout_mock = StdoutMock::default();

        line_state.exit(&mut stdout_mock).unwrap();

        // Line should be cleared.
        assert!(line_state.line.is_empty());
    }

    #[test]
    fn test_update_prompt_changes_prompt() {
        let mut line_state = LineState::new("old> ".into(), (80, 24));
        let mut stdout_mock = StdoutMock::default();

        line_state.update_prompt("new> ", &mut stdout_mock).unwrap();

        assert_eq!(line_state.prompt, "new> ");
    }

    #[test]
    fn test_print_data_sets_last_line_completed() {
        let mut line_state = LineState::new("$ ".into(), (80, 24));
        let mut stdout_mock = StdoutMock::default();

        // Data ending with newline.
        line_state
            .print_data_and_flush(b"hello\n", &mut stdout_mock)
            .unwrap();
        assert!(line_state.last_line_completed);

        // Data not ending with newline.
        line_state
            .print_data_and_flush(b"world", &mut stdout_mock)
            .unwrap();
        assert!(!line_state.last_line_completed);
    }
}

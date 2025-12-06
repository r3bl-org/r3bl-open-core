// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::LineState;
use crate::{CsiSequence, GCStringOwned, LINE_FEED_BYTE, LineStateLiveness, ReadlineError,
            TermColDelta, early_return_if_paused, ok, width};
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
            term.write_all(CsiSequence::CursorUp(1).to_string().as_bytes())?;
            term.write_all(
                CsiSequence::CursorHorizontalAbsolute(1)
                    .to_string()
                    .as_bytes(),
            )?;
            // Use TermColDelta to guard against CSI zero bug.
            let cols_right: TermColDelta = self.last_line_length.into();
            if let Some(n) = cols_right.as_nonzero_u16() {
                term.write_all(CsiSequence::CursorForward(n).to_string().as_bytes())?;
            }
        }

        // Write data in a way that newlines also act as carriage returns.
        let col_0 = CsiSequence::CursorHorizontalAbsolute(1).to_string();
        for line in data.split_inclusive(|b| *b == LINE_FEED_BYTE) {
            term.write_all(line)?;
            term.write_all(col_0.as_bytes())?;
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
            CsiSequence::CursorHorizontalAbsolute(1)
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
            CsiSequence::CursorHorizontalAbsolute(1)
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

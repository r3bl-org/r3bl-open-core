// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::{LineState, LineStateLiveness, early_return_if_paused};
use crate::{CsiSequence, StringLength, ok, width};
use std::io::{self, Write};

impl LineState {
    /// Clear current line.
    ///
    /// # Errors
    ///
    /// Returns an error if clearing the terminal line fails.
    pub fn clear(&self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        // Column index value equals distance from start (col 5 = 5 chars from start).
        self.move_to_beginning(term, width(self.current_column.as_u16()))?;
        // ED 0 = Erase from cursor to end of screen (CSI 0J).
        term.write_all(CsiSequence::EraseDisplay(0).to_string().as_bytes())?;

        ok!()
    }

    /// Render line (prompt + line) and flush.
    ///
    /// # Errors
    ///
    /// Returns an error if rendering or flushing the terminal fails.
    pub fn render_and_flush(&mut self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        let output = format!("{}{}", self.prompt, self.line);
        write!(term, "{output}")?;

        let prompt_len =
            StringLength::StripAnsi.calculate(&self.prompt, &mut self.memoized_len_map);

        let line_len =
            StringLength::Unicode.calculate(&self.line, &mut self.memoized_len_map);

        let total_line_len = width(prompt_len + line_len);

        self.move_to_beginning(term, total_line_len)?;
        // Column index value equals distance from start (col 5 = 5 chars from start).
        self.move_from_beginning(term, width(self.current_column.as_u16()))?;

        term.flush()?;

        ok!()
    }

    /// Clear line and render.
    ///
    /// # Errors
    ///
    /// Returns an error if clearing, rendering, or flushing the terminal fails.
    pub fn clear_and_render_and_flush(&mut self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        self.clear(term)?;
        self.render_and_flush(term)?;

        ok!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_fixtures::StdoutMock;

    #[test]
    fn test_clear_writes_escape_sequences() {
        let line_state = LineState::new("prompt> ".into(), (80, 24));
        let mut stdout_mock = StdoutMock::default();

        line_state.clear(&mut stdout_mock).unwrap();
        let output = stdout_mock.get_copy_of_buffer_as_string();

        // Should contain CursorHorizontalAbsolute (move to column 1).
        assert!(
            output.contains("\x1b[1G"),
            "Expected CHA sequence, got: {output:?}"
        );

        // Should contain EraseDisplay (clear from cursor to end).
        assert!(
            output.contains("\x1b[0J"),
            "Expected ED sequence, got: {output:?}"
        );
    }

    #[test]
    fn test_render_and_flush_outputs_prompt_and_line() {
        let mut line_state = LineState::new("$ ".into(), (80, 24));
        line_state.line = "hello".to_string();
        let mut stdout_mock = StdoutMock::default();

        line_state.render_and_flush(&mut stdout_mock).unwrap();
        let output = stdout_mock.get_copy_of_buffer_as_string();

        // Should contain the prompt and line content.
        assert!(
            output.contains("$ hello"),
            "Expected '$ hello' in output, got: {output:?}"
        );
    }
}

/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::io::{self, Write};

use crossterm::{cursor,
                event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
                terminal::{Clear,
                           ClearType::{All, FromCursorDown}},
                QueueableCommand};
use unicode_segmentation::UnicodeSegmentation;

use crate::{ok,
            MemoizedLenMap,
            ReadlineError,
            ReadlineEvent,
            SafeHistory,
            StringLength};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LineStateLiveness {
    Paused,
    NotPaused,
}

impl LineStateLiveness {
    pub fn is_paused(&self) -> bool { matches!(self, LineStateLiveness::Paused) }
}

/// This struct actually handles the line editing, and rendering. This works hand in hand
/// with the [crate::Readline] to make sure that the line is rendered correctly, with
/// pause and resume support.
pub struct LineState {
    /// Unicode line.
    pub line: String,

    /// Index of grapheme in line.
    pub line_cursor_grapheme: usize,

    /// Column of grapheme in line.
    pub current_column: u16,

    /// buffer for holding partial grapheme clusters as they come in
    pub cluster_buffer: String,

    pub prompt: String,

    /// After pressing enter, should we print the line just submitted?
    pub should_print_line_on_enter: bool,

    /// After pressing control_c should we print the line just cancelled?
    pub should_print_line_on_control_c: bool,

    pub last_line_length: usize,
    pub last_line_completed: bool,

    pub term_size: (u16, u16),

    /// This is the only place where this information is stored. Since pause and resume
    /// ultimately only affect this struct.
    pub is_paused: LineStateLiveness,

    /// Use to memoize the length of strings.
    pub memoized_len_map: MemoizedLenMap,
}

macro_rules! early_return_if_paused {
    ($self:ident @None) => {
        if matches!($self.is_paused, LineStateLiveness::Paused) {
            return Ok(None);
        }
    };

    ($self:ident @Unit) => {
        if matches!($self.is_paused, LineStateLiveness::Paused) {
            return Ok(());
        }
    };
}

impl LineState {
    pub fn new(prompt: String, term_size: (u16, u16)) -> Self {
        let mut memoized_len_map = MemoizedLenMap::new();
        let current_column =
            StringLength::StripAnsi.calculate(prompt.as_str(), &mut memoized_len_map);
        Self {
            prompt,
            last_line_completed: true,
            term_size,
            current_column,
            should_print_line_on_enter: true,
            should_print_line_on_control_c: false,
            line: String::new(),
            line_cursor_grapheme: 0,
            cluster_buffer: String::new(),
            last_line_length: 0,
            is_paused: LineStateLiveness::NotPaused,
            memoized_len_map,
        }
    }

    /// Update the paused state, which affects the following:
    /// - Rendering the output from multiple [crate::SharedWriter]s. When paused nothing
    ///   is rendered from them, and things like the [crate::Spinner] can be active.
    /// - Handling user input while the [crate::Readline::readline] is awaiting user input
    ///   (which is equivalent to awaiting [crate::ReadlineAsyncContext::read_line]).
    ///
    /// This should not be called directly. Instead, use the mechanism provided by the
    /// following:
    /// - [crate::SharedWriter::line_state_control_channel_sender]
    /// - [tokio::sync::mpsc::channel]
    pub fn set_paused(
        &mut self,
        is_paused: LineStateLiveness,
        term: &mut dyn Write,
    ) -> io::Result<()> {
        // Set the current value.
        self.is_paused = is_paused;

        // When going from paused → unpaused, we need to clear and render the line.
        if !is_paused.is_paused() {
            self.clear_and_render_and_flush(term)?
        }

        ok!()
    }

    /// Gets the number of lines wrapped
    fn line_height(&self, pos: u16) -> u16 { pos / self.term_size.0 }

    /// Move from a position on the line to the start.
    fn move_to_beginning(&self, term: &mut dyn Write, from: u16) -> io::Result<()> {
        let move_up = self.line_height(from.saturating_sub(1));
        term.queue(cursor::MoveToColumn(0))?;
        if move_up != 0 {
            term.queue(cursor::MoveUp(move_up))?;
        }

        ok!()
    }

    /// Move from the start of the line to some position.
    fn move_from_beginning(&self, term: &mut dyn Write, to: u16) -> io::Result<()> {
        let line_height = self.line_height(to.saturating_sub(1));
        let line_remaining_len = to % self.term_size.0; // Get the remaining length
        if line_height != 0 {
            term.queue(cursor::MoveDown(line_height))?;
        }
        term.queue(cursor::MoveRight(line_remaining_len))?;

        ok!()
    }

    /// Move cursor by one unicode grapheme either left (negative) or right (positive).
    fn move_cursor(&mut self, change: isize) -> io::Result<()> {
        if change > 0 {
            let count = self.line.graphemes(true).count();
            self.line_cursor_grapheme =
                usize::min(self.line_cursor_grapheme + change as usize, count);
        } else {
            self.line_cursor_grapheme =
                self.line_cursor_grapheme.saturating_sub((-change) as usize);
        }
        let (pos, str) = self.current_grapheme().unwrap_or((0, ""));
        let pos = pos + str.len();

        let prompt_len =
            StringLength::StripAnsi.calculate(&self.prompt, &mut self.memoized_len_map);

        let line_len = StringLength::Unicode
            .calculate(&self.line[0..pos], &mut self.memoized_len_map);

        self.current_column = prompt_len + line_len;

        ok!()
    }

    fn current_grapheme(&self) -> Option<(usize, &str)> {
        self.line
            .grapheme_indices(true)
            .take(self.line_cursor_grapheme)
            .last()
    }

    fn next_grapheme(&self) -> Option<(usize, &str)> {
        let total = self.line.grapheme_indices(true).count();
        if self.line_cursor_grapheme == total {
            return None;
        }
        self.line
            .grapheme_indices(true)
            .take(self.line_cursor_grapheme + 1)
            .last()
    }

    fn reset_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
        self.move_to_beginning(term, self.current_column)?;

        ok!()
    }

    fn set_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
        self.move_from_beginning(term, self.current_column)?;

        ok!()
    }

    /// Clear current line.
    pub fn clear(&self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        self.move_to_beginning(term, self.current_column)?;
        term.queue(Clear(FromCursorDown))?;

        ok!()
    }

    /// Render line (prompt + line) and flush.
    pub fn render_and_flush(&mut self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        let output = format!("{}{}", self.prompt, self.line);
        write!(term, "{output}")?;

        let prompt_len =
            StringLength::StripAnsi.calculate(&self.prompt, &mut self.memoized_len_map);

        let line_len =
            StringLength::Unicode.calculate(&self.line, &mut self.memoized_len_map);

        let total_line_len = prompt_len + line_len;

        self.move_to_beginning(term, total_line_len)?;
        self.move_from_beginning(term, self.current_column)?;

        term.flush()?;

        ok!()
    }

    /// Clear line and render.
    pub fn clear_and_render_and_flush(&mut self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        self.clear(term)?;
        self.render_and_flush(term)?;

        ok!()
    }

    pub fn print_data_and_flush(
        &mut self,
        data: &[u8],
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        self.clear(term)?;

        // If last written data was not newline, restore the cursor
        if !self.last_line_completed {
            term.queue(cursor::MoveUp(1))?
                .queue(cursor::MoveToColumn(0))?
                .queue(cursor::MoveRight(self.last_line_length as u16))?;
        }

        // Write data in a way that newlines also act as carriage returns
        for line in data.split_inclusive(|b| *b == b'\n') {
            term.write_all(line)?;
            term.queue(cursor::MoveToColumn(0))?;
        }

        self.last_line_completed = data.ends_with(b"\n"); // Set whether data ends with newline

        // If data does not end with newline, save the cursor and write newline for prompt
        // Usually data does end in newline due to the buffering of SharedWriter, but
        // sometimes it may not (i.e. if .flush() is called)
        if !self.last_line_completed {
            self.last_line_length += data.len();
            // Make sure that last_line_length wraps around when doing multiple writes
            if self.last_line_length >= self.term_size.0 as usize {
                self.last_line_length %= self.term_size.0 as usize;
                writeln!(term)?;
            }
            writeln!(term)?; // Move to beginning of line and make new line
        } else {
            self.last_line_length = 0;
        }

        term.queue(cursor::MoveToColumn(0))?;
        self.render_and_flush(term)?;

        ok!()
    }

    pub fn print_and_flush(
        &mut self,
        string: &str,
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        early_return_if_paused!(self @Unit);

        self.print_data_and_flush(string.as_bytes(), term)?;

        ok!()
    }

    pub fn update_prompt(
        &mut self,
        prompt: &str,
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        self.clear(term)?;
        self.prompt.clear();
        self.prompt.push_str(prompt);

        // recalculates column
        self.move_cursor(0)?;
        self.render_and_flush(term)?;

        ok!()
    }

    pub fn exit(&mut self, term: &mut dyn Write) -> Result<(), ReadlineError> {
        self.line.clear();
        self.clear(term)?;

        term.queue(cursor::MoveToColumn(0))?;
        term.flush()?;

        ok!()
    }

    pub fn render_new_line_from_beginning_and_flush(
        &mut self,
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        early_return_if_paused!(self @Unit);

        self.move_cursor(-100000)?;
        self.clear_and_render_and_flush(term)?;

        ok!()
    }

    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    pub fn apply_event_and_render(
        &mut self,
        event: Event,
        term: &mut dyn Write,
        safe_history: SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        match event {
            // Control Keys
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                ..
            }) => match code {
                // End of transmission (Ctrl+D)
                KeyCode::Char('d') => {
                    self.exit(term)?;
                    return Ok(Some(ReadlineEvent::Eof));
                }
                // End of text (Ctrl+C)
                KeyCode::Char('c') => {
                    if self.should_print_line_on_control_c && !self.is_paused.is_paused()
                    {
                        self.print_and_flush(
                            &format!("{}{}", self.prompt, self.line),
                            term,
                        )?;
                    }
                    self.exit(term)?;
                    return Ok(Some(ReadlineEvent::Interrupted));
                }
                // Clear all
                KeyCode::Char('l') => {
                    early_return_if_paused!(self @None);

                    term.queue(Clear(All))?.queue(cursor::MoveTo(0, 0))?;
                    self.clear_and_render_and_flush(term)?;
                }
                // Clear to start
                KeyCode::Char('u') => {
                    early_return_if_paused!(self @None);

                    if let Some((pos, str)) = self.current_grapheme() {
                        let pos = pos + str.len();
                        self.line.drain(0..pos);
                        self.move_cursor(-100000)?;
                        self.clear_and_render_and_flush(term)?;
                    }
                }
                // Clear last word
                KeyCode::Char('w') => {
                    early_return_if_paused!(self @None);

                    let count = self.line.graphemes(true).count();
                    let skip_count = count - self.line_cursor_grapheme;
                    let start = self
                        .line
                        .grapheme_indices(true)
                        .rev()
                        .skip(skip_count)
                        .skip_while(|(_, str)| *str == " ")
                        .find_map(
                            |(pos, str)| if str == " " { Some(pos + 1) } else { None },
                        )
                        .unwrap_or(0);
                    let end = self
                        .line
                        .grapheme_indices(true)
                        .nth(self.line_cursor_grapheme)
                        .map(|(end, _)| end);
                    let change = start as isize - self.line_cursor_grapheme as isize;
                    self.move_cursor(change)?;
                    if let Some(end) = end {
                        self.line.drain(start..end);
                    } else {
                        self.line.drain(start..);
                    }

                    self.clear_and_render_and_flush(term)?;
                }
                // Move to beginning
                #[cfg(feature = "emacs")]
                KeyCode::Char('a') => {
                    early_return_if_paused!(self @None);

                    self.reset_cursor(term)?;
                    self.move_cursor(-100000)?;
                    self.set_cursor(term)?;

                    term.flush()?;
                }
                // Move to end
                #[cfg(feature = "emacs")]
                KeyCode::Char('e') => {
                    early_return_if_paused!(self @None);

                    self.reset_cursor(term)?;
                    self.move_cursor(100000)?;
                    self.set_cursor(term)?;

                    term.flush()?;
                }
                // Move cursor left to previous word
                KeyCode::Left => {
                    early_return_if_paused!(self @None);

                    self.reset_cursor(term)?;
                    let count = self.line.graphemes(true).count();
                    let skip_count = count - self.line_cursor_grapheme;
                    if let Some((pos, _)) = self
                        .line
                        .grapheme_indices(true)
                        .rev()
                        .skip(skip_count)
                        .skip_while(|(_, str)| *str == " ")
                        .find(|(_, str)| *str == " ")
                    {
                        let change = pos as isize - self.line_cursor_grapheme as isize;
                        self.move_cursor(change + 1)?;
                    } else {
                        self.move_cursor(-100000)?
                    }
                    self.set_cursor(term)?;

                    term.flush()?;
                }
                // Move cursor right to next word
                KeyCode::Right => {
                    early_return_if_paused!(self @None);

                    self.reset_cursor(term)?;
                    if let Some((pos, _)) = self
                        .line
                        .grapheme_indices(true)
                        .skip(self.line_cursor_grapheme)
                        .skip_while(|(_, c)| *c == " ")
                        .find(|(_, c)| *c == " ")
                    {
                        let change = pos as isize - self.line_cursor_grapheme as isize;
                        self.move_cursor(change)?;
                    } else {
                        self.move_cursor(10000)?;
                    };
                    self.set_cursor(term)?;

                    term.flush()?;
                }
                _ => {}
            },
            // Other Modifiers (None, Shift, Control+Alt)
            // All other modifiers must be considered because the match expression cannot
            // match combined KeyModifiers. Control+Alt is used to reach
            // certain special symbols on a lot of international keyboard
            // layouts.
            Event::Key(KeyEvent {
                code,
                modifiers: _,
                kind: KeyEventKind::Press,
                ..
            }) => {
                early_return_if_paused!(self @None);

                match code {
                    KeyCode::Enter => {
                        // Print line so you can see what commands you've typed.
                        if self.should_print_line_on_enter && !self.is_paused.is_paused()
                        {
                            self.print_and_flush(
                                &format!("{}{}\n", self.prompt, self.line),
                                term,
                            )?;
                        }

                        // Take line
                        let line = std::mem::take(&mut self.line);
                        self.render_new_line_from_beginning_and_flush(term)?;

                        // Return line
                        return Ok(Some(ReadlineEvent::Line(line)));
                    }
                    // Delete character from line
                    KeyCode::Backspace => {
                        if let Some((pos, str)) = self.current_grapheme() {
                            self.clear(term)?;
                            let len = pos + str.len();
                            self.line.replace_range(pos..len, "");
                            self.move_cursor(-1)?;

                            self.render_and_flush(term)?;
                        }
                    }
                    KeyCode::Delete => {
                        if let Some((pos, str)) = self.next_grapheme() {
                            self.clear(term)?;
                            let len = pos + str.len();
                            self.line.replace_range(pos..len, "");

                            self.render_and_flush(term)?;
                        }
                    }
                    KeyCode::Left => {
                        self.reset_cursor(term)?;
                        self.move_cursor(-1)?;
                        self.set_cursor(term)?;
                        term.flush()?;
                    }
                    KeyCode::Right => {
                        self.reset_cursor(term)?;
                        self.move_cursor(1)?;
                        self.set_cursor(term)?;
                        term.flush()?;
                    }
                    KeyCode::Home => {
                        self.reset_cursor(term)?;
                        self.move_cursor(-100000)?;
                        self.set_cursor(term)?;
                        term.flush()?;
                    }
                    KeyCode::End => {
                        self.reset_cursor(term)?;
                        self.move_cursor(100000)?;
                        self.set_cursor(term)?;
                        term.flush()?;
                    }
                    KeyCode::Up => {
                        // search for next history item, replace line if found.
                        if let Some(line) = safe_history.lock().unwrap().search_next() {
                            self.line.clear();
                            self.line += line;
                            self.clear(term)?;
                            self.move_cursor(100000)?;
                            self.render_and_flush(term)?;
                        }
                    }
                    KeyCode::Down => {
                        // search for next history item, replace line if found.
                        if let Some(line) = safe_history.lock().unwrap().search_previous()
                        {
                            self.line.clear();
                            self.line += line;
                            self.clear(term)?;
                            self.move_cursor(100000)?;
                            self.render_and_flush(term)?;
                        }
                    }
                    // Add character to line and output
                    KeyCode::Char(c) => {
                        self.clear(term)?;
                        let prev_len = self.cluster_buffer.graphemes(true).count();
                        self.cluster_buffer.push(c);
                        let new_len = self.cluster_buffer.graphemes(true).count();

                        let (g_pos, g_str) = self.current_grapheme().unwrap_or((0, ""));
                        let pos = g_pos + g_str.len();

                        self.line.insert(pos, c);

                        if prev_len != new_len {
                            self.move_cursor(1)?;
                            if prev_len > 0 {
                                if let Some((pos, str)) =
                                    self.cluster_buffer.grapheme_indices(true).next()
                                {
                                    let len = str.len();
                                    self.cluster_buffer.replace_range(pos..len, "");
                                }
                            }
                        }

                        self.render_and_flush(term)?;
                    }
                    _ => {}
                }
            }
            Event::Resize(x, y) => {
                early_return_if_paused!(self @None);

                self.term_size = (x, y);
                self.clear_and_render_and_flush(term)?;

                return Ok(Some(ReadlineEvent::Resized));
            }
            _ => {}
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{core::test_fixtures::StdoutMock, History, StdMutex};

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_add_char() {
        let mut line = LineState::new("foo".into(), (100, 100));

        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        let it = line.apply_event_and_render(
            event,
            &mut *safe_output_terminal.lock().unwrap(),
            safe_history,
        );

        assert!(matches!(it, Ok(None)));

        assert_eq!(line.line, "a");
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_move_cursor() {
        let mut line = LineState::new("foo".into(), (100, 100));

        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

        let it = line.apply_event_and_render(
            event,
            &mut *safe_output_terminal.lock().unwrap(),
            safe_history,
        );

        assert!(matches!(it, Ok(None)));

        assert_eq!(line.current_column, 3);
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_search_next() {
        let mut line = LineState::new("foo".into(), (100, 100));

        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        let it = line.apply_event_and_render(
            event,
            &mut *safe_output_terminal.lock().unwrap(),
            safe_history,
        );

        assert!(matches!(it, Ok(None)));

        assert_eq!(line.line, "");
    }
}

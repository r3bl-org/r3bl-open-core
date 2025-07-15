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

use crate::{ok, MemoizedLenMap, ReadlineError, ReadlineEvent, SafeHistory, StringLength};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LineStateLiveness {
    Paused,
    NotPaused,
}

impl LineStateLiveness {
    #[must_use]
    pub fn is_paused(&self) -> bool { matches!(self, LineStateLiveness::Paused) }
}

/// This struct actually handles the line editing, and rendering. This works hand in hand
/// with the [`crate::Readline`] to make sure that the line is rendered correctly, with
/// pause and resume support.
#[derive(Debug)]
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

    /// After pressing `control_c` should we print the line just cancelled?
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
    #[must_use]
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
    /// - Rendering the output from multiple [`crate::SharedWriter`]s. When paused nothing
    ///   is rendered from them, and things like the [`crate::Spinner`] can be active.
    /// - Handling user input while the [`crate::Readline::readline`] is awaiting user
    ///   input (which is equivalent to awaiting
    ///   [`crate::ReadlineAsyncContext::read_line`]).
    ///
    /// This should not be called directly. Instead, use the mechanism provided by the
    /// following:
    /// - [`crate::SharedWriter::line_state_control_channel_sender`]
    /// - [`tokio::sync::mpsc::channel`]
    pub fn set_paused(
        &mut self,
        is_paused: LineStateLiveness,
        term: &mut dyn Write,
    ) -> io::Result<()> {
        // Set the current value.
        self.is_paused = is_paused;

        // When going from paused → unpaused, we need to clear and render the line.
        if !is_paused.is_paused() {
            self.clear_and_render_and_flush(term)?;
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
    pub fn move_cursor(&mut self, change: isize) -> io::Result<()> {
        if change > 0 {
            let count = self.line.graphemes(true).count();

            // We know that change is positive, so we can safely cast it to usize.
            #[allow(clippy::cast_sign_loss)]
            let change_usize = change as usize;

            self.line_cursor_grapheme =
                usize::min(self.line_cursor_grapheme + change_usize, count);
        } else {
            // Use unsigned_abs() to convert negative change to positive amount to
            // subtract.
            self.line_cursor_grapheme = self
                .line_cursor_grapheme
                .saturating_sub(change.unsigned_abs());
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

    #[must_use]
    pub fn current_grapheme(&self) -> Option<(usize, &str)> {
        self.line
            .grapheme_indices(true)
            .take(self.line_cursor_grapheme)
            .last()
    }

    #[must_use]
    pub fn next_grapheme(&self) -> Option<(usize, &str)> {
        let total = self.line.grapheme_indices(true).count();
        if self.line_cursor_grapheme == total {
            return None;
        }
        self.line
            .grapheme_indices(true)
            .take(self.line_cursor_grapheme + 1)
            .last()
    }

    pub fn reset_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
        self.move_to_beginning(term, self.current_column)?;

        ok!()
    }

    pub fn set_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
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
            // This cast is intentional to ensure that the last line length is
            // represented as a u16, which is the type expected by crossterm's
            // `MoveRight` command.
            #[allow(clippy::cast_possible_truncation)]
            let last_line_length_u16 = self.last_line_length as u16;
            term.queue(cursor::MoveUp(1))?
                .queue(cursor::MoveToColumn(0))?
                .queue(cursor::MoveRight(last_line_length_u16))?;
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
        if self.last_line_completed {
            self.last_line_length = 0;
        } else {
            self.last_line_length += data.len();
            // Make sure that last_line_length wraps around when doing multiple writes
            if self.last_line_length >= self.term_size.0 as usize {
                self.last_line_length %= self.term_size.0 as usize;
                writeln!(term)?;
            }
            writeln!(term)?; // Move to beginning of line and make new line
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

        self.move_cursor(-100_000)?;
        self.clear_and_render_and_flush(term)?;

        ok!()
    }

    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    pub fn apply_event_and_render(
        &mut self,
        event: Event,
        term: &mut dyn Write,
        safe_history: SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        use apply_event_and_render_helper::{handle_control_key, handle_regular_key,
                                            handle_resize};

        match event {
            // Control Keys
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                ..
            }) => handle_control_key(self, code, term, safe_history),
            // Other Modifiers (None, Shift, Control+Alt)
            Event::Key(KeyEvent {
                code,
                modifiers: _,
                kind: KeyEventKind::Press,
                ..
            }) => handle_regular_key(self, code, term, safe_history),
            Event::Resize(x, y) => handle_resize(self, x, y, term),
            _ => Ok(None),
        }
    }
}

mod apply_event_and_render_helper {
    use super::{cursor, All, Clear, KeyCode, LineState, LineStateLiveness,
                QueueableCommand, ReadlineError, ReadlineEvent, SafeHistory,
                UnicodeSegmentation, Write};

    /// Handle control key events (Ctrl+key combinations)
    pub fn handle_control_key(
        line_state: &mut LineState,
        code: KeyCode,
        term: &mut dyn Write,
        _safe_history: SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        match code {
            KeyCode::Char('d') => handle_ctrl_d(line_state, term),
            KeyCode::Char('c') => handle_ctrl_c(line_state, term),
            KeyCode::Char('l') => handle_ctrl_l(line_state, term),
            KeyCode::Char('u') => handle_ctrl_u(line_state, term),
            KeyCode::Char('w') => handle_ctrl_w(line_state, term),
            #[cfg(feature = "emacs")]
            KeyCode::Char('a') => handle_ctrl_a(line_state, term),
            #[cfg(feature = "emacs")]
            KeyCode::Char('e') => handle_ctrl_e(line_state, term),
            KeyCode::Left => handle_ctrl_left(line_state, term),
            KeyCode::Right => handle_ctrl_right(line_state, term),
            _ => Ok(None),
        }
    }

    /// Handle regular key events (no modifiers or non-Control modifiers)
    pub fn handle_regular_key(
        line_state: &mut LineState,
        code: KeyCode,
        term: &mut dyn Write,
        safe_history: SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        match code {
            KeyCode::Enter => handle_enter(line_state, term),
            KeyCode::Backspace => handle_backspace(line_state, term),
            KeyCode::Delete => handle_delete(line_state, term),
            KeyCode::Left => handle_left(line_state, term),
            KeyCode::Right => handle_right(line_state, term),
            KeyCode::Home => handle_home(line_state, term),
            KeyCode::End => handle_end(line_state, term),
            KeyCode::Up => handle_up(line_state, term, safe_history),
            KeyCode::Down => handle_down(line_state, term, safe_history),
            KeyCode::Char(c) => handle_char(line_state, term, c),
            _ => Ok(None),
        }
    }

    /// Handle terminal resize events
    pub fn handle_resize(
        line_state: &mut LineState,
        x: u16,
        y: u16,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.term_size = (x, y);
        line_state.clear_and_render_and_flush(term)?;
        Ok(Some(ReadlineEvent::Resized))
    }

    // Control key handlers
    fn handle_ctrl_d(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        // End of transmission (Ctrl+D)
        line_state.exit(term)?;
        Ok(Some(ReadlineEvent::Eof))
    }

    // End of text (Ctrl+C)
    fn handle_ctrl_c(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        if line_state.should_print_line_on_control_c && !line_state.is_paused.is_paused()
        {
            line_state.print_and_flush(
                &format!("{}{}", line_state.prompt, line_state.line),
                term,
            )?;
        }
        line_state.exit(term)?;
        Ok(Some(ReadlineEvent::Interrupted))
    }

    // Clear all
    fn handle_ctrl_l(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        term.queue(Clear(All))?.queue(cursor::MoveTo(0, 0))?;
        line_state.clear_and_render_and_flush(term)?;
        Ok(None)
    }

    // Clear to start
    fn handle_ctrl_u(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        if let Some((pos, str)) = line_state.current_grapheme() {
            let pos = pos + str.len();
            line_state.line.drain(0..pos);
            line_state.move_cursor(-100_000)?;
            line_state.clear_and_render_and_flush(term)?;
        }
        Ok(None)
    }

    // Clear last word
    fn handle_ctrl_w(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        let count = line_state.line.graphemes(true).count();
        let skip_count = count - line_state.line_cursor_grapheme;
        let start = line_state
            .line
            .grapheme_indices(true)
            .rev()
            .skip(skip_count)
            .skip_while(|(_, str)| *str == " ")
            .find_map(|(pos, str)| if str == " " { Some(pos + 1) } else { None })
            .unwrap_or(0);
        let end = line_state
            .line
            .grapheme_indices(true)
            .nth(line_state.line_cursor_grapheme)
            .map(|(end, _)| end);
        // Calculate cursor movement to beginning of word being deleted.
        // Handles potential overflow by using checked conversion to isize.
        // Calculate the change in cursor position, handling potential overflow
        let change = if start >= line_state.line_cursor_grapheme {
            // Moving forward (positive change)
            let diff = start - line_state.line_cursor_grapheme;
            isize::try_from(diff).unwrap_or(isize::MAX)
        } else {
            // Moving backward (negative change)
            let diff = line_state.line_cursor_grapheme - start;
            -(isize::try_from(diff).unwrap_or(isize::MAX))
        };
        line_state.move_cursor(change)?;
        if let Some(end) = end {
            line_state.line.drain(start..end);
        } else {
            line_state.line.drain(start..);
        }

        line_state.clear_and_render_and_flush(term)?;
        Ok(None)
    }

    // Move to beginning
    #[cfg(feature = "emacs")]
    fn handle_ctrl_a(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;
        line_state.move_cursor(-100_000)?;
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Move to end
    #[cfg(feature = "emacs")]
    fn handle_ctrl_e(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;
        line_state.move_cursor(100_000)?;
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Move cursor left to previous word
    fn handle_ctrl_left(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;
        let count = line_state.line.graphemes(true).count();
        let skip_count = count - line_state.line_cursor_grapheme;
        if let Some((pos, _)) = line_state
            .line
            .grapheme_indices(true)
            .rev()
            .skip(skip_count)
            .skip_while(|(_, str)| *str == " ")
            .find(|(_, str)| *str == " ")
        {
            // Calculate cursor movement to beginning of word being deleted.
            // Handles potential overflow by using checked conversion to isize.
            // Calculate the change in cursor position, handling potential overflow
            let change = if pos >= line_state.line_cursor_grapheme {
                let diff = pos - line_state.line_cursor_grapheme;
                isize::try_from(diff).unwrap_or(isize::MAX)
            } else {
                let diff = line_state.line_cursor_grapheme - pos;
                -(isize::try_from(diff).unwrap_or(isize::MAX))
            };
            line_state.move_cursor(change + 1)?;
        } else {
            line_state.move_cursor(-100_000)?;
        }
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Move cursor right to next word
    fn handle_ctrl_right(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;
        if let Some((pos, _)) = line_state
            .line
            .grapheme_indices(true)
            .skip(line_state.line_cursor_grapheme)
            .skip_while(|(_, c)| *c == " ")
            .find(|(_, c)| *c == " ")
        {
            // Calculate cursor movement to beginning of word being deleted.
            // Handles potential overflow by using checked conversion to isize.
            // Calculate the change in cursor position, handling potential overflow
            let change = if pos >= line_state.line_cursor_grapheme {
                let diff = pos - line_state.line_cursor_grapheme;
                isize::try_from(diff).unwrap_or(isize::MAX)
            } else {
                let diff = line_state.line_cursor_grapheme - pos;
                -(isize::try_from(diff).unwrap_or(isize::MAX))
            };
            line_state.move_cursor(change)?;
        } else {
            line_state.move_cursor(100_000)?;
        }
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Regular key handlers
    fn handle_enter(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        // Print line so you can see what commands you've typed.
        if line_state.should_print_line_on_enter && !line_state.is_paused.is_paused() {
            line_state.print_and_flush(
                &format!("{}{}\n", line_state.prompt, line_state.line),
                term,
            )?;
        }

        // Take line
        let line = std::mem::take(&mut line_state.line);
        line_state.render_new_line_from_beginning_and_flush(term)?;

        // Return line
        Ok(Some(ReadlineEvent::Line(line)))
    }

    // Delete (backspace) character from line
    fn handle_backspace(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        if let Some((pos, str)) = line_state.current_grapheme() {
            line_state.clear(term)?;
            let len = pos + str.len();
            line_state.line.replace_range(pos..len, "");
            line_state.move_cursor(-1)?;
            line_state.render_and_flush(term)?;
        }
        Ok(None)
    }

    // Delete character from line
    fn handle_delete(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        if let Some((pos, str)) = line_state.next_grapheme() {
            line_state.clear(term)?;
            let len = pos + str.len();
            line_state.line.replace_range(pos..len, "");
            line_state.render_and_flush(term)?;
        }
        Ok(None)
    }

    // Move cursor left
    fn handle_left(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        line_state.reset_cursor(term)?;
        line_state.move_cursor(-1)?;
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Move cursor right
    fn handle_right(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        line_state.reset_cursor(term)?;
        line_state.move_cursor(1)?;
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Move cursor home
    fn handle_home(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        line_state.reset_cursor(term)?;
        line_state.move_cursor(-100_000)?;
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Move cursor end
    fn handle_end(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        line_state.reset_cursor(term)?;
        line_state.move_cursor(100_000)?;
        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Search for next history item, replace line if found
    fn handle_up(
        line_state: &mut LineState,
        term: &mut dyn Write,
        safe_history: SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        if let Some(line) = safe_history.lock().unwrap().search_next() {
            line_state.line.clear();
            line_state.line += line;
            line_state.clear(term)?;
            line_state.move_cursor(100_000)?;
            line_state.render_and_flush(term)?;
        }
        Ok(None)
    }

    // Search for next history item, replace line if found
    fn handle_down(
        line_state: &mut LineState,
        term: &mut dyn Write,
        safe_history: SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        if let Some(line) = safe_history.lock().unwrap().search_previous() {
            line_state.line.clear();
            line_state.line += line;
            line_state.clear(term)?;
            line_state.move_cursor(100_000)?;
            line_state.render_and_flush(term)?;
        }
        Ok(None)
    }

    // Add character to line and output
    fn handle_char(
        line_state: &mut LineState,
        term: &mut dyn Write,
        c: char,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        line_state.clear(term)?;
        let prev_len = line_state.cluster_buffer.graphemes(true).count();
        line_state.cluster_buffer.push(c);
        let new_len = line_state.cluster_buffer.graphemes(true).count();

        let (g_pos, g_str) = line_state.current_grapheme().unwrap_or((0, ""));
        let pos = g_pos + g_str.len();

        line_state.line.insert(pos, c);

        if prev_len != new_len {
            line_state.move_cursor(1)?;
            if prev_len > 0
                && let Some((pos, str)) =
                    line_state.cluster_buffer.grapheme_indices(true).next()
                {
                    let len = str.len();
                    line_state.cluster_buffer.replace_range(pos..len, "");
                }
        }

        line_state.render_and_flush(term)?;
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

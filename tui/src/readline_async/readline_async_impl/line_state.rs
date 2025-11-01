// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, IndexOps, InputEvent,
            Key, KeyPress, KeyState, LINE_FEED_BYTE, MemoizedLenMap, ReadlineError,
            ReadlineEvent, SafeHistory, SpecialKey, StringLength,
            core::coordinates::idx, find_next_word_end, find_prev_word_start, ok};
#[cfg(test)]
use crate::ModifierKeysMask;
use crossterm::{QueueableCommand, cursor,
                terminal::{Clear,
                           ClearType::{All, FromCursorDown}}};
use std::io::{self, Write};
use unicode_segmentation::UnicodeSegmentation;

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
    ///
    /// # Errors
    ///
    /// Returns an error if clearing or rendering the line fails.
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
        let from_index = idx(usize::from(from));
        let one_idx = idx(1);
        let prev_pos = if one_idx.overflows(from_index.convert_to_length())
            == ArrayOverflowResult::Overflowed
        {
            0
        } else {
            from - 1
        };
        let move_up = self.line_height(prev_pos);
        term.queue(cursor::MoveToColumn(0))?;
        if move_up != 0 {
            term.queue(cursor::MoveUp(move_up))?;
        }

        ok!()
    }

    /// Move from the start of the line to some position.
    fn move_from_beginning(&self, term: &mut dyn Write, to: u16) -> io::Result<()> {
        let to_index = idx(usize::from(to));
        let one_idx = idx(1);
        let prev_pos = if one_idx.overflows(to_index.convert_to_length())
            == ArrayOverflowResult::Overflowed
        {
            0
        } else {
            to - 1
        };
        let line_height = self.line_height(prev_pos);
        let line_remaining_len = to % self.term_size.0; // Get the remaining length
        if line_height != 0 {
            term.queue(cursor::MoveDown(line_height))?;
        }
        term.queue(cursor::MoveRight(line_remaining_len))?;

        ok!()
    }

    /// Move cursor by one unicode grapheme either left (negative) or right (positive).
    ///
    /// # Errors
    ///
    /// Returns an error if I/O operations fail.
    pub fn move_cursor(&mut self, change: isize) -> io::Result<()> {
        if change > 0 {
            let count = self.line.graphemes(true).count();

            // We know that change is positive, so we can safely cast it to usize.
            #[allow(clippy::cast_sign_loss)]
            let change_usize = change as usize;

            let new_position = idx(self.line_cursor_grapheme + change_usize);
            let count_length = idx(count).convert_to_length();
            self.line_cursor_grapheme =
                new_position.clamp_to_max_length(count_length).as_usize();
        } else {
            // Use unsigned_abs() to convert negative change to
            // positive amount to subtract.
            let change_idx = idx(change.unsigned_abs());
            let current_idx = idx(self.line_cursor_grapheme);
            self.line_cursor_grapheme = if change_idx
                .overflows(current_idx.convert_to_length())
                == ArrayOverflowResult::Overflowed
            {
                0
            } else {
                self.line_cursor_grapheme - change.unsigned_abs()
            };
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

    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn reset_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
        self.move_to_beginning(term, self.current_column)?;

        ok!()
    }

    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn set_cursor(&self, term: &mut dyn Write) -> io::Result<()> {
        self.move_from_beginning(term, self.current_column)?;

        ok!()
    }

    /// Clear current line.
    /// # Errors
    ///
    /// Returns an error if clearing the terminal line fails.
    pub fn clear(&self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        self.move_to_beginning(term, self.current_column)?;
        term.queue(Clear(FromCursorDown))?;

        ok!()
    }

    /// Render line (prompt + line) and flush.
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

        let total_line_len = prompt_len + line_len;

        self.move_to_beginning(term, total_line_len)?;
        self.move_from_beginning(term, self.current_column)?;

        term.flush()?;

        ok!()
    }

    /// Clear line and render.
    /// # Errors
    ///
    /// Returns an error if clearing, rendering, or flushing the terminal fails.
    pub fn clear_and_render_and_flush(&mut self, term: &mut dyn Write) -> io::Result<()> {
        early_return_if_paused!(self @Unit);

        self.clear(term)?;
        self.render_and_flush(term)?;

        ok!()
    }

    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn print_data_and_flush(
        &mut self,
        data: &[u8],
        term: &mut dyn Write,
    ) -> Result<(), ReadlineError> {
        self.clear(term)?;

        // If last written data was not newline, restore the cursor.
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

        // Write data in a way that newlines also act as carriage returns.
        for line in data.split_inclusive(|b| *b == LINE_FEED_BYTE) {
            term.write_all(line)?;
            term.queue(cursor::MoveToColumn(0))?;
        }

        self.last_line_completed = data.ends_with(b"\n"); // Set whether data ends with newline

        // If data does not end with newline, save the cursor and write newline for
        // prompt. Usually data does end in newline due to the buffering of
        // SharedWriter, but sometimes it may not (i.e. if .flush() is called).
        if self.last_line_completed {
            self.last_line_length = 0;
        } else {
            self.last_line_length += data.len();
            // Make sure that last_line_length wraps around when doing multiple writes.
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

    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    pub fn exit(&mut self, term: &mut dyn Write) -> Result<(), ReadlineError> {
        self.line.clear();
        self.clear(term)?;

        term.queue(cursor::MoveToColumn(0))?;
        term.flush()?;

        ok!()
    }

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

    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails or if the event cannot be
    /// processed.
    pub fn apply_event_and_render(
        &mut self,
        event: &InputEvent,
        term: &mut dyn Write,
        safe_history: &SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        use apply_event_and_render_helper::{handle_alt_key, handle_control_key,
                                            handle_regular_key, handle_resize};

        match event {
            InputEvent::Keyboard(keypress) => match keypress {
                KeyPress::Plain { key } => {
                    handle_regular_key(self, key, term, safe_history)
                }
                KeyPress::WithModifiers { key, mask } => {
                    // Determine if ONLY Ctrl is pressed (no Shift or Alt)
                    let is_ctrl_only = mask.ctrl_key_state == KeyState::Pressed
                        && mask.shift_key_state == KeyState::NotPressed
                        && mask.alt_key_state == KeyState::NotPressed;

                    // Determine if ONLY Alt is pressed (no Shift or Ctrl)
                    let is_alt_only = mask.alt_key_state == KeyState::Pressed
                        && mask.shift_key_state == KeyState::NotPressed
                        && mask.ctrl_key_state == KeyState::NotPressed;

                    if is_ctrl_only {
                        handle_control_key(self, key, term, safe_history)
                    } else if is_alt_only {
                        handle_alt_key(self, key, term, safe_history)
                    } else {
                        handle_regular_key(self, key, term, safe_history)
                    }
                }
            }
            InputEvent::Resize(size) => {
                let width = size.col_width.0.value;
                let height = size.row_height.0.value;
                handle_resize(self, width, height, term)
            }
            _ => Ok(None),
        }
    }
}

mod apply_event_and_render_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Handle control key events (Ctrl+key combinations)
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn handle_control_key(
        line_state: &mut LineState,
        key: &Key,
        term: &mut dyn Write,
        _safe_history: &SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        match key {
            Key::Character('d') => handle_ctrl_d(line_state, term),
            Key::Character('c') => handle_ctrl_c(line_state, term),
            Key::Character('l') => handle_ctrl_l(line_state, term),
            Key::Character('u') => handle_ctrl_u(line_state, term),
            Key::Character('w') => handle_ctrl_w(line_state, term),
            #[cfg(feature = "emacs")]
            Key::Character('a') => handle_ctrl_a(line_state, term),
            #[cfg(feature = "emacs")]
            Key::Character('e') => handle_ctrl_e(line_state, term),
            Key::SpecialKey(SpecialKey::Left) => handle_ctrl_left(line_state, term),
            Key::SpecialKey(SpecialKey::Right) => handle_ctrl_right(line_state, term),
            _ => Ok(None),
        }
    }

    /// Handle Alt key events (Alt+key combinations)
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn handle_alt_key(
        line_state: &mut LineState,
        key: &Key,
        term: &mut dyn Write,
        _safe_history: &SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        match key {
            Key::Character('b') => handle_alt_b(line_state, term),
            Key::Character('f') => handle_alt_f(line_state, term),
            Key::Character('d') => handle_alt_d(line_state, term),
            Key::SpecialKey(SpecialKey::Backspace) => handle_alt_backspace(line_state, term),
            _ => Ok(None),
        }
    }

    /// Handle regular key events (no modifiers or non-Control modifiers)
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn handle_regular_key(
        line_state: &mut LineState,
        key: &Key,
        term: &mut dyn Write,
        safe_history: &SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        match key {
            Key::SpecialKey(SpecialKey::Enter) => handle_enter(line_state, term),
            Key::SpecialKey(SpecialKey::Backspace) => handle_backspace(line_state, term),
            Key::SpecialKey(SpecialKey::Delete) => handle_delete(line_state, term),
            Key::SpecialKey(SpecialKey::Left) => handle_left(line_state, term),
            Key::SpecialKey(SpecialKey::Right) => handle_right(line_state, term),
            Key::SpecialKey(SpecialKey::Home) => handle_home(line_state, term),
            Key::SpecialKey(SpecialKey::End) => handle_end(line_state, term),
            Key::SpecialKey(SpecialKey::Up) => handle_up(line_state, term, safe_history),
            Key::SpecialKey(SpecialKey::Down) => handle_down(line_state, term, safe_history),
            Key::Character(c) => handle_char(line_state, term, *c),
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

    // Control key handlers.
    fn handle_ctrl_d(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        // Bash-standard Ctrl+D behavior:
        // - If line is empty: exit (EOF)
        // - If line is not empty: delete character at cursor (like Delete key)
        if line_state.line.is_empty() {
            line_state.exit(term)?;
            Ok(Some(ReadlineEvent::Eof))
        } else {
            handle_delete(line_state, term)
        }
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

    // Clear last word (delete word backward)
    fn handle_ctrl_w(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        let cursor_pos = line_state.line_cursor_grapheme;
        if cursor_pos == 0 {
            return Ok(None); // Nothing to delete
        }

        // Find start of previous word using word_boundaries module
        let word_start = find_prev_word_start(&line_state.line, cursor_pos);

        if word_start < cursor_pos {
            // Get byte indices for string manipulation
            let graphemes: Vec<_> = line_state.line.grapheme_indices(true).collect();
            let start_byte = graphemes.get(word_start).map_or(0, |(i, _)| *i);
            let end_byte = graphemes
                .get(cursor_pos)
                .map_or(line_state.line.len(), |(i, _)| *i);

            // Delete the word
            line_state.line.drain(start_byte..end_byte);

            // Move cursor to deletion point
            #[allow(clippy::cast_possible_wrap)]
            let movement = word_start as isize - cursor_pos as isize;
            line_state.move_cursor(movement)?;

            line_state.clear_and_render_and_flush(term)?;
        }

        Ok(None)
    }

    // Move to beginning.
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

    // Move cursor left to previous word (backward-word navigation).
    fn handle_ctrl_left(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;

        let cursor_pos = line_state.line_cursor_grapheme;
        if cursor_pos > 0 {
            // Find start of previous word using word_boundaries module
            let word_start = find_prev_word_start(&line_state.line, cursor_pos);
            #[allow(clippy::cast_possible_wrap)]
            let movement = word_start as isize - cursor_pos as isize;
            line_state.move_cursor(movement)?;
        }

        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Move cursor right to next word (forward-word navigation).
    fn handle_ctrl_right(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;

        let cursor_pos = line_state.line_cursor_grapheme;
        let line_len = line_state.line.graphemes(true).count();

        if cursor_pos < line_len {
            // Find end of next word using word_boundaries module
            let word_end = find_next_word_end(&line_state.line, cursor_pos);
            #[allow(clippy::cast_possible_wrap)]
            let movement = word_end as isize - cursor_pos as isize;
            line_state.move_cursor(movement)?;
        }

        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Alt+key handlers.

    // Alt+B: backward-word (move cursor to start of previous word)
    fn handle_alt_b(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;

        let cursor_pos = line_state.line_cursor_grapheme;
        if cursor_pos > 0 {
            // Find start of previous word
            let word_start = find_prev_word_start(&line_state.line, cursor_pos);
            #[allow(clippy::cast_possible_wrap)]
            let movement = word_start as isize - cursor_pos as isize;
            line_state.move_cursor(movement)?;
        }

        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Alt+F: forward-word (move cursor to start of next word)
    fn handle_alt_f(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        line_state.reset_cursor(term)?;

        let cursor_pos = line_state.line_cursor_grapheme;
        let line_len = line_state.line.graphemes(true).count();

        if cursor_pos < line_len {
            // Find end of next word
            let word_end = find_next_word_end(&line_state.line, cursor_pos);
            #[allow(clippy::cast_possible_wrap)]
            let movement = word_end as isize - cursor_pos as isize;
            line_state.move_cursor(movement)?;
        }

        line_state.set_cursor(term)?;
        term.flush()?;
        Ok(None)
    }

    // Alt+D: kill-word (delete from cursor to end of word)
    fn handle_alt_d(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        let cursor_pos = line_state.line_cursor_grapheme;
        let line_len = line_state.line.graphemes(true).count();

        if cursor_pos < line_len {
            // Find end of current/next word
            let word_end = find_next_word_end(&line_state.line, cursor_pos);

            if word_end > cursor_pos {
                // Get byte indices for string manipulation
                let graphemes: Vec<_> = line_state.line.grapheme_indices(true).collect();
                let start_byte = graphemes.get(cursor_pos).map_or(0, |(i, _)| *i);
                let end_byte = graphemes
                    .get(word_end)
                    .map_or(line_state.line.len(), |(i, _)| *i);

                line_state.line.drain(start_byte..end_byte);
                line_state.clear_and_render_and_flush(term)?;
            }
        }

        Ok(None)
    }

    // Alt+Backspace: backward-kill-word (delete from start of word to cursor)
    fn handle_alt_backspace(
        line_state: &mut LineState,
        term: &mut dyn Write,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        early_return_if_paused!(line_state @None);

        let cursor_pos = line_state.line_cursor_grapheme;
        if cursor_pos == 0 {
            return Ok(None);
        }

        // Find start of previous word
        let word_start = find_prev_word_start(&line_state.line, cursor_pos);

        if word_start < cursor_pos {
            // Get byte indices for string manipulation
            let graphemes: Vec<_> = line_state.line.grapheme_indices(true).collect();
            let start_byte = graphemes.get(word_start).map_or(0, |(i, _)| *i);
            let end_byte = graphemes
                .get(cursor_pos)
                .map_or(line_state.line.len(), |(i, _)| *i);

            line_state.line.drain(start_byte..end_byte);
            #[allow(clippy::cast_possible_wrap)]
            let movement = word_start as isize - cursor_pos as isize;
            line_state.move_cursor(movement)?;
            line_state.clear_and_render_and_flush(term)?;
        }

        Ok(None)
    }

    // Regular key handlers.
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

    // Delete character from line.
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

    // Move cursor left.
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

    // Move cursor right.
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

    // Move cursor home.
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

    // Search for next history item, replace line if found.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    fn handle_up(
        line_state: &mut LineState,
        term: &mut dyn Write,
        safe_history: &SafeHistory,
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

    // Search for next history item, replace line if found.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    fn handle_down(
        line_state: &mut LineState,
        term: &mut dyn Write,
        safe_history: &SafeHistory,
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

    // Add character to line and output.
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
    use super::*;
    use crate::{History, StdMutex, core::test_fixtures::StdoutMock};
    use std::sync::Arc;

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_add_char() {
        let mut line = LineState::new("foo".into(), (100, 100));

        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('a'),
        });

        let it = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
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

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Right),
        });

        let it = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
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

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        });

        let it = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(it, Ok(None)));

        assert_eq!(line.line, "");
    }

    // Phase 1.1: Tests for recent bug fixes

    #[tokio::test]
    async fn test_ctrl_d_empty_line_eof() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('d'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Ctrl+D on empty line should return EOF
        assert!(matches!(result, Ok(Some(ReadlineEvent::Eof))));
    }

    #[tokio::test]
    async fn test_ctrl_d_non_empty_deletes_char() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello".to_string();
        line.line_cursor_grapheme = 0; // Cursor at start
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('d'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Ctrl+D on non-empty line should delete character at cursor
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line, "ello");
    }

    #[tokio::test]
    async fn test_ctrl_w_word_boundaries() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "foo bar-baz qux".to_string();
        line.line_cursor_grapheme = 15; // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('w'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+W should delete "qux"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, "foo bar-baz ");

        // Second Ctrl+W should delete "baz"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, "foo bar-");

        // Third Ctrl+W should delete "bar"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, "foo ");
    }

    #[tokio::test]
    async fn test_ctrl_left_word_navigation() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello-world foo".to_string();
        line.line_cursor_grapheme = 15; // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+Left should move to start of "foo"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 12);

        // Second Ctrl+Left should move to start of "world"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 6);

        // Third Ctrl+Left should move to start of "hello"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 0);
    }

    #[tokio::test]
    async fn test_ctrl_right_word_navigation() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello-world foo".to_string();
        line.line_cursor_grapheme = 0; // Start of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Right),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+Right should move to end of "hello"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 5);

        // Second Ctrl+Right should move to end of "world"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 11);

        // Third Ctrl+Right should move to end of "foo"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 15);
    }

    // Phase 1.2: Tests for new Alt+key handlers

    #[tokio::test]
    async fn test_alt_b_backward_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "one two three".to_string();
        line.line_cursor_grapheme = 13; // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('b'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+B should move to start of "three"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 8);

        // Second Alt+B should move to start of "two"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 4);

        // Third Alt+B should move to start of "one"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 0);
    }

    #[tokio::test]
    async fn test_alt_f_forward_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "one two three".to_string();
        line.line_cursor_grapheme = 0; // Start of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('f'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+F should move to end of "one"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 3);

        // Second Alt+F should move to end of "two"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 7);

        // Third Alt+F should move to end of "three"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, 13);
    }

    #[tokio::test]
    async fn test_alt_d_kill_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "foo bar baz".to_string();
        line.line_cursor_grapheme = 0; // Start of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('d'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+D should delete "foo"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, " bar baz");

        // Second Alt+D should delete the space and "bar"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, " baz");

        // Third Alt+D should delete the space and "baz"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, "");
    }

    #[tokio::test]
    async fn test_alt_backspace_backward_kill_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "foo bar baz".to_string();
        line.line_cursor_grapheme = 11; // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Backspace),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+Backspace should delete "baz"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, "foo bar ");

        // Second Alt+Backspace should delete "bar"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, "foo ");

        // Third Alt+Backspace should delete "foo"
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line, "");
    }

    // Phase 2: Tests for remaining handlers

    // Phase 2.1: Remaining Ctrl+key handlers

    #[tokio::test]
    async fn test_ctrl_c_interrupt() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "some input".to_string();
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('c'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Ctrl+C should signal interrupt
        assert!(matches!(result, Ok(Some(ReadlineEvent::Interrupted))));
    }

    #[tokio::test]
    async fn test_ctrl_l_clear_screen() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "test".to_string();
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('l'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Ctrl+L should clear screen and re-render
        assert!(matches!(result, Ok(None)));
        // Line content should be preserved
        assert_eq!(line.line, "test");
    }

    #[tokio::test]
    async fn test_ctrl_u_delete_to_start() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello world".to_string();
        line.line_cursor_grapheme = 11; // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('u'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Ctrl+U should delete from cursor to line start
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line, "");
        assert_eq!(line.line_cursor_grapheme, 0);
    }

    #[cfg(feature = "emacs")]
    #[tokio::test]
    async fn test_ctrl_a_move_to_start() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello world".to_string();
        line.line_cursor_grapheme = 11; // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('a'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Ctrl+A should move cursor to line start
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 0);
    }

    #[cfg(feature = "emacs")]
    #[tokio::test]
    async fn test_ctrl_e_move_to_end() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello world".to_string();
        line.line_cursor_grapheme = 0; // Start of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('e'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Ctrl+E should move cursor to line end
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 11);
    }

    // Phase 2.2: Regular key handlers

    #[tokio::test]
    async fn test_enter_submit_line() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "test command".to_string();
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Enter should submit the line
        assert!(matches!(result, Ok(Some(ReadlineEvent::Line(_)))));
    }

    #[tokio::test]
    async fn test_backspace_delete_before() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello".to_string();
        line.line_cursor_grapheme = 5; // After "hello"
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Backspace),
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Backspace should delete character before cursor
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line, "hell");
        assert_eq!(line.line_cursor_grapheme, 4);
    }

    #[tokio::test]
    async fn test_delete_key_delete_at_cursor() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello".to_string();
        line.line_cursor_grapheme = 0; // At start
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Delete),
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Delete should delete character at cursor
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line, "ello");
        assert_eq!(line.line_cursor_grapheme, 0);
    }

    #[tokio::test]
    async fn test_left_arrow_move_left() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello".to_string();
        line.line_cursor_grapheme = 5; // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Left),
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Left arrow should move cursor left one position
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 4);
    }

    #[tokio::test]
    async fn test_home_key_move_to_start() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello world".to_string();
        line.line_cursor_grapheme = 11; // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Home),
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Home should move cursor to start of line
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 0);
    }

    #[tokio::test]
    async fn test_end_key_move_to_end() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = "hello world".to_string();
        line.line_cursor_grapheme = 0; // At start
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::End),
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // End should move cursor to end of line
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 11);
    }

    #[tokio::test]
    async fn test_down_arrow_history_next() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        // Create history with some entries
        let (mut history, _) = History::new();
        history.update(Some("command1".to_string()));
        history.update(Some("command2".to_string()));
        let safe_history = Arc::new(StdMutex::new(history));

        // Navigate up first to position in history
        let up_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        });
        line.apply_event_and_render(
            &up_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();

        // Now test down arrow
        let down_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Down),
        });

        let result = line.apply_event_and_render(
            &down_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Down should navigate forward in history
        assert!(matches!(result, Ok(None)));
    }

    // Edge case tests

    #[tokio::test]
    async fn test_unicode_emoji_word_operations() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Setup: "hello 世界 🌍 test"
        //         012345678901234567890
        //               ^cursor at end (20)
        line.line = "hello 世界 🌍 test".to_string();
        line.line_cursor_grapheme = 20;

        // Test Ctrl+W (delete word backward) with Unicode/emoji
        let ctrl_w_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('w'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &ctrl_w_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Should delete "test", leaving "hello 世界 🌍 "
        assert_eq!(line.line, "hello 世界 🌍 ");

        // Test Alt+Backspace with emoji
        let alt_backspace_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Backspace),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        let result = line.apply_event_and_render(
            &alt_backspace_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Should delete "🌍 ", leaving "hello 世界 "
        assert_eq!(line.line, "hello 世界 ");
    }

    #[tokio::test]
    async fn test_ctrl_left_unicode() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Setup: "hello 世界 test"
        line.line = "hello 世界 test".to_string();
        line.line_cursor_grapheme = 16; // At end

        let ctrl_left_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+Left: should move to start of "test"
        let result = line.apply_event_and_render(
            &ctrl_left_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 9); // Start of "test"

        // Second Ctrl+Left: should move to start of "世界"
        let result = line.apply_event_and_render(
            &ctrl_left_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 6); // Start of "世界"

        // Third Ctrl+Left: should move to start of "hello"
        let result = line.apply_event_and_render(
            &ctrl_left_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, 0); // Start of line
    }

    #[tokio::test]
    async fn test_pause_resume_state() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        line.line = "test".to_string();
        line.line_cursor_grapheme = 4;

        // Pause the line state
        line.set_paused(
            LineStateLiveness::Paused,
            &mut *safe_output_terminal.lock().unwrap(),
        )
        .unwrap();

        // Try to send input while paused - should be ignored
        let char_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('x'),
        });

        let result = line.apply_event_and_render(
            &char_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Line should be unchanged because it's paused
        assert_eq!(line.line, "test");
        assert_eq!(line.line_cursor_grapheme, 4);

        // Resume the line state
        line.set_paused(
            LineStateLiveness::NotPaused,
            &mut *safe_output_terminal.lock().unwrap(),
        )
        .unwrap();

        // Now input should work
        let result = line.apply_event_and_render(
            &char_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Line should now have the character appended
        assert_eq!(line.line, "testx");
        assert_eq!(line.line_cursor_grapheme, 5);
    }

    #[tokio::test]
    async fn test_ctrl_w_empty_line() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Empty line
        line.line = String::new();
        line.line_cursor_grapheme = 0;

        let ctrl_w_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('w'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &ctrl_w_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Should not crash, should return Ok(None)
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line, String::new());
        assert_eq!(line.line_cursor_grapheme, 0);
    }

    #[tokio::test]
    async fn test_word_boundaries_with_only_punctuation() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Line with only punctuation: "...---..."
        line.line = "...---...".to_string();
        line.line_cursor_grapheme = 9; // At end

        // Test Ctrl+W on punctuation-only line
        let ctrl_w_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('w'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &ctrl_w_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Should delete to beginning since all are boundaries
        assert_eq!(line.line, String::new());
        assert_eq!(line.line_cursor_grapheme, 0);

        // Test Ctrl+Left on mixed punctuation
        line.line = "hello...world".to_string();
        line.line_cursor_grapheme = 13;

        let ctrl_left_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = line.apply_event_and_render(
            &ctrl_left_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Should move to start of "world" at position 8
        assert_eq!(line.line_cursor_grapheme, 8);
    }
}

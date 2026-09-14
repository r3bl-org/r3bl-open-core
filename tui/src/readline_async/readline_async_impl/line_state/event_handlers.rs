// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::{LineState, PauseState, PrintLineOnControlC, PrintLineOnEnter};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ByteIndex, CSI_ERASE_DISPLAY_ALL,
            CsiSequence, CursorBoundsCheck, FunctionKey, GCStringOwned, InputEvent, Key,
            KeyPress, KeyState, NarrowingCastToIsize, NumericValue, ReadlineError,
            ReadlineEvent, SafeHistory, SegIndex, SpecialKey, VPSize,
            early_return_if_paused, find_next_word_end, find_next_word_start,
            find_prev_word_start, inline_string, tui::NEW_LINE, vp_col, vp_row};
use std::{io::Write, num::NonZeroU8};

impl LineState {
    /// Processes an input event, updates line state, and renders changes to the terminal.
    ///
    /// This is the **core event processing method** for the readline event loop. It:
    /// 1. Receives an input event (keyboard, resize, mouse, etc.)
    /// 2. Updates the internal line state (text, cursor position, history)
    /// 3. Renders the updated state to the terminal
    /// 4. Returns any significant events that the caller needs to handle
    ///
    /// # Returns
    ///
    /// - `Ok(Some(ReadlineEvent))` when a **significant event** occurs that the caller
    ///   should handle:
    ///   - [`ReadlineEvent::Line`] - User pressed Enter, line is complete
    ///   - [`ReadlineEvent::Eof`] - User pressed Ctrl+D on empty line
    ///   - [`ReadlineEvent::Resized`] - Terminal was resized
    /// - `Ok(None)` for **normal editing operations** that don't require caller action:
    ///   - Character insertion/deletion
    ///   - Cursor movement (arrow keys, Home, End, Ctrl+Left/Right, Alt+B/F)
    ///   - Word deletion (Ctrl+W, Alt+D, Alt+Backspace)
    ///   - Line editing (Ctrl+A, Ctrl+E, Ctrl+K, Ctrl+U)
    ///   - History navigation (Up/Down arrows)
    ///
    /// # Examples
    ///
    /// ## Basic Usage (Simulated Events)
    ///
    /// ```rust
    /// use r3bl_tui::{InputEvent, KeyPress, LineState, ReadlineEvent, StdMutex,
    ///               StdoutMock, VPSize, seg_index, vp_height, vp_width, SpecialKey};
    /// use std::sync::Arc;
    ///
    /// // Setup
    /// let mut line_state = LineState::new(String::new(), vp_width(80) + vp_height(24));
    /// let mut stdout = StdoutMock::default();
    /// let history = r3bl_tui::readline_async::readline_async_impl::History::new();
    /// let safe_history = Arc::new(StdMutex::new(history));
    ///
    /// // Simulate typing "hello"
    /// for ch in "hello".chars() {
    ///     let event = InputEvent::Keyboard(KeyPress::Plain {
    ///         key: r3bl_tui::Key::Character(ch)
    ///     });
    ///
    ///     let result = line_state.apply_event_and_render(
    ///         &event,
    ///         &mut stdout,
    ///         &safe_history
    ///     ).expect("conversion error");
    ///
    ///     // Normal character input returns None
    ///     assert!(result.is_none());
    /// }
    ///
    /// assert_eq!(line_state.line.as_str(), "hello");
    /// assert_eq!(line_state.cursor_position, seg_index(5));
    ///
    /// // Simulate pressing Enter
    /// let enter_event = InputEvent::Keyboard(KeyPress::Plain {
    ///     key: r3bl_tui::Key::SpecialKey(SpecialKey::Enter)
    /// });
    ///
    /// let result = line_state.apply_event_and_render(
    ///     &enter_event,
    ///     &mut stdout,
    ///     &safe_history
    /// ).expect("conversion error");
    ///
    /// // Enter returns Some(ReadlineEvent::Line)
    /// match result {
    ///     Some(ReadlineEvent::Line(text)) => {
    ///         assert_eq!(text, "hello");
    ///     }
    ///     _ => panic!("Expected ReadlineEvent::Line"),
    /// }
    /// ```
    ///
    /// ## Real-World Usage
    ///
    /// For complete async event loop implementations, see:
    /// - [`pty_ctrl_navigation_test`] - Shows full [`PTY`] test pattern with debouncing
    /// - [`pty_ctrl_d_eof_test`] - Shows handling of Ctrl+D as [`EOF`]
    /// - [`pty_ctrl_d_delete_test`] - Shows handling of Ctrl+D as delete
    ///
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread panics while
    /// holding the lock. To avoid panics, ensure that the code that locks the mutex does
    /// not panic while holding the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails or if the event cannot be
    /// processed.
    ///
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`pty_ctrl_d_delete_test`]:
    ///     crate::readline_async::readline_async_impl::readline_async_integration_tests::pty_ctrl_d_delete_test
    /// [`pty_ctrl_d_eof_test`]:
    ///     crate::readline_async::readline_async_impl::readline_async_integration_tests::pty_ctrl_d_eof_test
    /// [`pty_ctrl_navigation_test`]:
    ///     crate::readline_async::readline_async_impl::readline_async_integration_tests::pty_ctrl_navigation_test
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[allow(clippy::unwrap_in_result)] /* This is for lock.expect("conversion error") */
    pub fn apply_event_and_render(
        &mut self,
        event: &InputEvent,
        term: &mut dyn Write,
        safe_history: &SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
        match event {
            InputEvent::Keyboard(keypress) => match keypress {
                KeyPress::Plain { key } => {
                    handle_regular_key(self, *key, term, safe_history)
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
                        handle_control_key(self, *key, term, safe_history)
                    } else if is_alt_only {
                        handle_alt_key(self, *key, term, safe_history)
                    } else {
                        handle_regular_key(self, *key, term, safe_history)
                    }
                }
            },
            InputEvent::Resize(size) => handle_resize(self, *size, term),
            _ => Ok(None),
        }
    }
}

/// Handles control key events (Ctrl+key combinations)
pub fn handle_control_key(
    line_state: &mut LineState,
    key: Key,
    term: &mut dyn Write,
    _safe_history: &SafeHistory,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    match key {
        Key::Character('d') => handle_ctrl_d(line_state, term),
        Key::Character('c') => handle_ctrl_c(line_state, term),
        Key::Character('l') => handle_ctrl_l(line_state, term),
        Key::Character('u') => handle_ctrl_u(line_state, term),
        Key::Character('w') => handle_ctrl_w(line_state, term),
        Key::Character('a') => handle_ctrl_a(line_state, term),
        Key::Character('e') => handle_ctrl_e(line_state, term),
        Key::SpecialKey(SpecialKey::Left) => handle_ctrl_left(line_state, term),
        Key::SpecialKey(SpecialKey::Right) => handle_ctrl_right(line_state, term),
        _ => Ok(None),
    }
}

/// Handle Alt key events (Alt+key combinations)
pub fn handle_alt_key(
    line_state: &mut LineState,
    key: Key,
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

/// Handles regular key events (no modifiers or non-Control modifiers)
pub fn handle_regular_key(
    line_state: &mut LineState,
    key: Key,
    term: &mut dyn Write,
    safe_history: &SafeHistory,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    match key {
        // Internal handlers (modify state, return Ok(None)).
        Key::SpecialKey(SpecialKey::Enter) => handle_enter(line_state, term),
        Key::SpecialKey(SpecialKey::Backspace) => handle_backspace(line_state, term),
        Key::SpecialKey(SpecialKey::Delete) => handle_delete(line_state, term),
        Key::SpecialKey(SpecialKey::Left) => handle_left(line_state, term),
        Key::SpecialKey(SpecialKey::Right) => handle_right(line_state, term),
        Key::SpecialKey(SpecialKey::Home) => handle_home(line_state, term),
        Key::SpecialKey(SpecialKey::End) => handle_end(line_state, term),
        Key::SpecialKey(SpecialKey::Up) => handle_up(line_state, term, safe_history),
        Key::SpecialKey(SpecialKey::Down) => handle_down(line_state, term, safe_history),
        Key::Character(c) => handle_char(line_state, term, c),

        // Pass-through keys (return event for caller to handle).
        Key::SpecialKey(SpecialKey::Tab) => Ok(Some(ReadlineEvent::Tab)),
        Key::SpecialKey(SpecialKey::BackTab) => Ok(Some(ReadlineEvent::BackTab)),
        Key::SpecialKey(SpecialKey::PageUp) => Ok(Some(ReadlineEvent::PageUp)),
        Key::SpecialKey(SpecialKey::PageDown) => Ok(Some(ReadlineEvent::PageDown)),
        Key::SpecialKey(SpecialKey::Insert) => Ok(Some(ReadlineEvent::Insert)),

        // Function keys F1-F12.
        // unwrap() is safe: literal values 1-12 are guaranteed non-zero.
        #[allow(
            clippy::unwrap_in_result,
            clippy::unwrap_used,
            reason = "Hardcoded non-zero values"
        )]
        Key::FunctionKey(fn_key) => Ok(Some(ReadlineEvent::FnKey(match fn_key {
            FunctionKey::F1 => NonZeroU8::new(1).expect("conversion error"),
            FunctionKey::F2 => NonZeroU8::new(2).expect("conversion error"),
            FunctionKey::F3 => NonZeroU8::new(3).expect("conversion error"),
            FunctionKey::F4 => NonZeroU8::new(4).expect("conversion error"),
            FunctionKey::F5 => NonZeroU8::new(5).expect("conversion error"),
            FunctionKey::F6 => NonZeroU8::new(6).expect("conversion error"),
            FunctionKey::F7 => NonZeroU8::new(7).expect("conversion error"),
            FunctionKey::F8 => NonZeroU8::new(8).expect("conversion error"),
            FunctionKey::F9 => NonZeroU8::new(9).expect("conversion error"),
            FunctionKey::F10 => NonZeroU8::new(10).expect("conversion error"),
            FunctionKey::F11 => NonZeroU8::new(11).expect("conversion error"),
            FunctionKey::F12 => NonZeroU8::new(12).expect("conversion error"),
        }))),

        // Catch-all for unhandled keys.
        _ => Ok(Some(ReadlineEvent::UnhandledKey(KeyPress::Plain { key }))),
    }
}

/// Handles terminal resize events.
pub fn handle_resize(
    line_state: &mut LineState,
    size: VPSize,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);
    line_state.term_size = size;
    line_state.clear_and_render_and_flush(term)?;
    Ok(Some(ReadlineEvent::Resized(size)))
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
    if line_state.print_line_on_control_c == PrintLineOnControlC::Print
        && line_state.pause_state == PauseState::NotPaused
    {
        line_state.print_and_flush(
            &[line_state.prompt.as_str(), line_state.line.as_str()].join(""),
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

    // ED 2 = Erase entire screen (CSI 2J), then move cursor to home (row 0, col 0).
    term.write_all(CSI_ERASE_DISPLAY_ALL.as_bytes())?;
    term.write_all(
        inline_string!(
            "{}",
            CsiSequence::CursorPosition {
                row: vp_row(0).into(),
                col: vp_col(0).into(),
            }
        )
        .as_bytes(),
    )?;

    line_state.clear_and_render_and_flush(term)?;

    Ok(None)
}

// Clear to start
fn handle_ctrl_u(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    // Delete from start of line (position 0) to cursor position.
    // If cursor is at position 0, this deletes nothing.
    // If cursor is in middle or end, deletes from start to cursor.
    if !line_state.cursor_position.is_zero() {
        let cursor_pos = line_state
            .line
            .segment_count()
            .clamp_cursor_position(line_state.cursor_position);

        // Get byte offset at cursor using segment metadata.
        let cursor_byte_pos = get_byte_index_or_end(&line_state.line, cursor_pos);

        // Create new string without the deleted portion.
        let remaining = &line_state.line.as_str()[*cursor_byte_pos..];
        line_state.line = GCStringOwned::new(remaining);
        line_state.move_logical_cursor_to_start();
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

    // Early return if cursor is at start of line.
    if line_state.cursor_position.is_zero() {
        return Ok(None); // Nothing to delete
    }

    // Get cursor position in terms of grapheme segments.
    let cursor_pos = line_state
        .line
        .segment_count()
        .clamp_cursor_position(line_state.cursor_position);

    // Find start of previous word using word_boundaries module.
    let word_start = find_prev_word_start(&line_state.line, cursor_pos);

    if word_start < cursor_pos {
        // Get byte indices using segment metadata.
        let start_byte = get_byte_index_or_end(&line_state.line, word_start);
        let end_byte = get_byte_index_or_end(&line_state.line, cursor_pos);

        // Create new string with the word deleted.
        let left = &line_state.line.as_str()[..*start_byte];
        let right = &line_state.line.as_str()[*end_byte..];
        line_state.line = GCStringOwned::new([left, right].join(""));

        // Move cursor to deletion point.
        let backward_count = cursor_pos.distance_from(word_start);
        let movement = -backward_count.as_usize().as_isize_narrowing();
        line_state.shift_logical_cursor_by(movement);
        line_state.clear_and_render_and_flush(term)?;
    }

    Ok(None)
}

/// Move to beginning.
fn handle_ctrl_a(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    line_state.paint_cursor_rewind_to_start(term)?;
    line_state.move_logical_cursor_to_start();
    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

/// Move to end.
fn handle_ctrl_e(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    line_state.paint_cursor_rewind_to_start(term)?;
    line_state.move_logical_cursor_to_end();
    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

/// Move cursor left to previous word (backward-word navigation).
fn handle_ctrl_left(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    line_state.paint_cursor_rewind_to_start(term)?;

    if !line_state.cursor_position.is_zero() {
        let cursor_pos = line_state.cursor_position;
        // Find start of previous word using word_boundaries module.
        let word_start = find_prev_word_start(&line_state.line, cursor_pos);
        let backward_count = cursor_pos.distance_from(word_start);
        let movement = -backward_count.as_usize().as_isize_narrowing();
        line_state.shift_logical_cursor_by(movement);
    }

    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

// Move cursor right to next word (forward-word navigation).
fn handle_ctrl_right(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    line_state.paint_cursor_rewind_to_start(term)?;

    let cursor_pos = line_state.cursor_position;
    let total_segs = line_state.line.segment_count();

    if cursor_pos.overflows(total_segs) == ArrayOverflowResult::Within {
        // Find start of next word using word_boundaries module.
        let word_start = find_next_word_start(&line_state.line, cursor_pos);
        let forward_count = word_start.distance_from(cursor_pos);
        let movement = forward_count.as_usize().as_isize_narrowing();
        line_state.shift_logical_cursor_by(movement);
    }

    line_state.paint_cursor_at_current_column(term)?;
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

    line_state.paint_cursor_rewind_to_start(term)?;

    if !line_state.cursor_position.is_zero() {
        let cursor_pos = line_state.cursor_position;
        // Find start of previous word.
        let word_start = find_prev_word_start(&line_state.line, cursor_pos);
        let backward_count = cursor_pos.distance_from(word_start);
        let movement = -backward_count.as_usize().as_isize_narrowing();
        line_state.shift_logical_cursor_by(movement);
    }

    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

// Alt+F: forward-word (move cursor to start of next word)
fn handle_alt_f(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    line_state.paint_cursor_rewind_to_start(term)?;

    let cursor_pos = line_state.cursor_position;
    let total_segs = line_state.line.segment_count();

    if cursor_pos.overflows(total_segs) == ArrayOverflowResult::Within {
        // Find start of next word.
        let word_start = find_next_word_start(&line_state.line, cursor_pos);
        let forward_count = word_start.distance_from(cursor_pos);
        let movement = forward_count.as_usize().as_isize_narrowing();
        line_state.shift_logical_cursor_by(movement);
    }

    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

// Alt+D: kill-word (delete from cursor to end of word)
fn handle_alt_d(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    early_return_if_paused!(line_state @None);

    let cursor_pos = line_state.cursor_position;
    let total_segs = line_state.line.segment_count();

    if cursor_pos.overflows(total_segs) == ArrayOverflowResult::Within {
        // Find end of current/next word.
        let word_end = find_next_word_end(&line_state.line, cursor_pos);

        if word_end > cursor_pos {
            // Get byte indices using segment metadata.
            let start_byte = get_byte_index_or_end(&line_state.line, cursor_pos);
            let end_byte = get_byte_index_or_end(&line_state.line, word_end);

            // Create new string with the word deleted.
            let left = &line_state.line.as_str()[..*start_byte];
            let right = &line_state.line.as_str()[*end_byte..];
            line_state.line = GCStringOwned::new([left, right].join(""));
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

    // Early return if cursor is at start of line.
    if line_state.cursor_position.is_zero() {
        return Ok(None);
    }

    // Clamp cursor position to be within bounds of line.
    let cursor_pos = line_state
        .line
        .segment_count()
        .clamp_cursor_position(line_state.cursor_position);

    // Find start of previous word.
    let word_start = find_prev_word_start(&line_state.line, cursor_pos);

    if word_start < cursor_pos {
        // Get byte indices using segment metadata.
        let start_byte = get_byte_index_or_end(&line_state.line, word_start);
        let end_byte = get_byte_index_or_end(&line_state.line, cursor_pos);

        // Create new string with the word deleted.
        let left = &line_state.line.as_str()[..*start_byte];
        let right = &line_state.line.as_str()[*end_byte..];
        line_state.line = GCStringOwned::new([left, right].join(""));

        let backward_count = cursor_pos.distance_from(word_start);
        let movement = -backward_count.as_usize().as_isize_narrowing();
        line_state.shift_logical_cursor_by(movement);
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
    if line_state.print_line_on_enter == PrintLineOnEnter::Print
        && line_state.pause_state == PauseState::NotPaused
    {
        line_state.print_and_flush(
            &[
                line_state.prompt.as_str(),
                line_state.line.as_str(),
                NEW_LINE,
            ]
            .join(""),
            term,
        )?;
    }

    // Take line content and reset to empty.
    let line_string = line_state.line.as_str().to_string();
    line_state.line = GCStringOwned::new("");
    line_state.render_new_line_from_beginning_and_flush(term)?;

    // Return line.
    Ok(Some(ReadlineEvent::Line(line_string)))
}

// Delete (backspace) character from line.
fn handle_backspace(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    if let Some(seg) = line_state.grapheme_before_cursor() {
        line_state.clear(term)?;

        // Create new string without the deleted character.
        let left = {
            let start_idx = seg.start_byte_index.as_usize();
            &line_state.line.as_str()[..start_idx]
        };
        let right = {
            let end_idx = seg.end_byte_index.as_usize();
            &line_state.line.as_str()[end_idx..]
        };
        line_state.line = GCStringOwned::new([left, right].join(""));

        line_state.shift_logical_cursor_by(-1);
        line_state.render_and_flush(term)?;
    }
    Ok(None)
}

// Delete character from line.
fn handle_delete(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    if let Some(seg) = line_state.grapheme_at_cursor() {
        line_state.clear(term)?;

        // Create new string without the deleted character.
        let left = {
            let start = seg.start_byte_index.as_usize();
            &line_state.line.as_str()[..start]
        };
        let right = {
            let end = seg.end_byte_index.as_usize();
            &line_state.line.as_str()[end..]
        };
        line_state.line = GCStringOwned::new([left, right].join(""));

        line_state.render_and_flush(term)?;
    }
    Ok(None)
}

// Move cursor left.
fn handle_left(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    line_state.paint_cursor_rewind_to_start(term)?;
    line_state.shift_logical_cursor_by(-1);
    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

// Move cursor right.
fn handle_right(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    line_state.paint_cursor_rewind_to_start(term)?;
    line_state.shift_logical_cursor_by(1);
    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

// Move cursor home.
fn handle_home(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    line_state.paint_cursor_rewind_to_start(term)?;
    line_state.move_logical_cursor_to_start();
    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

// Move cursor to end.
fn handle_end(
    line_state: &mut LineState,
    term: &mut dyn Write,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    line_state.paint_cursor_rewind_to_start(term)?;
    line_state.move_logical_cursor_to_end();
    line_state.paint_cursor_at_current_column(term)?;
    term.flush()?;

    Ok(None)
}

// Navigate to older history entry.
#[allow(clippy::unwrap_in_result)] /* This is for lock.expect("conversion error") */
fn handle_up(
    line_state: &mut LineState,
    term: &mut dyn Write,
    safe_history: &SafeHistory,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    if let Some(line) =
        safe_history.write(|history| history.search_next().map(String::from))
    {
        line_state.line = GCStringOwned::new(line);
        line_state.clear(term)?;
        line_state.move_logical_cursor_to_end();
        line_state.render_and_flush(term)?;
    }

    Ok(None)
}

// Navigate to newer history entry.
#[allow(clippy::unwrap_in_result)] /* This is for lock.expect("conversion error") */
fn handle_down(
    line_state: &mut LineState,
    term: &mut dyn Write,
    safe_history: &SafeHistory,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    if let Some(line) =
        safe_history.write(|history| history.search_previous().map(String::from))
    {
        line_state.line = GCStringOwned::new(line);
        line_state.clear(term)?;
        line_state.move_logical_cursor_to_end();
        line_state.render_and_flush(term)?;
    }

    Ok(None)
}

// Add character to line and output.
fn handle_char(
    line_state: &mut LineState,
    term: &mut dyn Write,
    char_to_insert: char,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    // Clear the line (so we can mutate and re-render subsequently).
    line_state.clear(term)?;

    // Track segment count before insertion.
    let seg_count_before_insert = line_state.line.segment_count();

    // Get byte position after grapheme before cursor (insertion point).
    let insert_at = if let Some(seg) = line_state.grapheme_before_cursor() {
        seg.end_byte_index.as_usize()
    } else {
        0
    };

    // Insert character by rebuilding the string with exact capacity.
    line_state.line = GCStringOwned::new({
        let left_of = &line_state.line.as_str()[..insert_at];
        let right_of = &line_state.line.as_str()[insert_at..];

        let mut new_str = String::with_capacity(
            line_state.line.bytes_size().as_usize() + char_to_insert.len_utf8(),
        );

        new_str.push_str(left_of);
        new_str.push(char_to_insert);
        new_str.push_str(right_of);

        new_str
    });

    // Advance cursor only if a new grapheme cluster was created. If a combining character
    // attached to the previous grapheme, the segment count remains unchanged and the
    // cursor does not move.
    if line_state.line.segment_count() > seg_count_before_insert {
        // Inserting a single char (char_to_insert) cannot create more than 1 new grapheme
        // cluster segment.
        line_state.shift_logical_cursor_by(1);
    }

    // Render the modified line and flush to terminal.
    line_state.render_and_flush(term)?;

    Ok(None)
}

/// Returns the byte position of `seg_idx` in `line`.
///
/// If `seg_idx` is at or past the end of the line, this silently clamps the return value
/// to the total byte length of the line. This clamping is an intentional UI fail-safe to
/// prevent terminal crashes in the event of an out-of-bounds cursor calculation.
fn get_byte_index_or_end(line: &GCStringOwned, seg_idx: SegIndex) -> ByteIndex {
    line.get_byte_index(seg_idx)
        .unwrap_or_else(|| line.bytes_size().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{History, ModifierKeysMask, StdMutex, core::test_fixtures::StdoutMock,
                seg_index, vp_col, vp_height, vp_width};
    use std::sync::Arc;

    // cspell:words ello testx

    #[test]
    #[allow(clippy::needless_return)]
    fn test_add_char() {
        let mut line = LineState::new("foo".into(), vp_width(100) + vp_height(100));

        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('a'),
        });

        let it = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(matches!(it, Ok(None)));

        assert_eq!(line.line.as_str(), "a");
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_move_cursor() {
        let mut line = LineState::new("foo".into(), vp_width(100) + vp_height(100));

        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Right),
        });

        let it = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(matches!(it, Ok(None)));

        assert_eq!(line.calc_current_column(), vp_col(3));
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_search_next() {
        let mut line = LineState::new("foo".into(), vp_width(100) + vp_height(100));

        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        });

        let it = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(matches!(it, Ok(None)));

        assert_eq!(line.line.as_str(), "");
    }

    // Phase 1.1: Tests for recent bug fixes.

    #[test]
    fn test_ctrl_d_empty_line_eof() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('d'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Ctrl+D on empty line should return EOF.
        assert!(matches!(result, Ok(Some(ReadlineEvent::Eof))));
    }

    #[test]
    fn test_ctrl_d_non_empty_deletes_char() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("abc");
        line.cursor_position = seg_index(1); // Cursor after 'a'
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('d'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Ctrl+D on non-empty line should delete char at cursor.
        assert!(matches!(result, Ok(None)));
        // 'b' should be deleted (char at cursor position).
        assert_eq!(line.line.as_str(), "ac");
    }

    #[test]
    fn test_ctrl_w_word_boundaries() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello world");
        line.cursor_position = seg_index(11); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('w'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(matches!(result, Ok(None)));
        // "world" should be deleted, leaving "hello ".
        assert_eq!(line.line.as_str(), "hello ");
        assert_eq!(line.cursor_position, seg_index(6));
    }

    #[test]
    fn test_ctrl_left_word_navigation() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello-world foo");
        line.cursor_position = seg_index(15); // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+Left should move to start of "foo".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(12));

        // Second Ctrl+Left should move to start of "world".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(6));

        // Third Ctrl+Left should move to start of "hello".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(0));
    }

    #[test]
    fn test_ctrl_right_word_navigation() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello-world foo");
        line.cursor_position = seg_index(0); // Start of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Right),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+Right should move to start of "world" (after hyphen).
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(6));

        // Second Ctrl+Right should move to start of "foo".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(12));

        // Third Ctrl+Right should move to end (no next word).
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(15));
    }

    // Phase 1.2: Tests for new Alt+key handlers.

    #[test]
    fn test_alt_b_backward_word() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("one two three");
        line.cursor_position = seg_index(13); // End of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('b'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+B should move to start of "three".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(8));

        // Second Alt+B should move to start of "two".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(4));

        // Third Alt+B should move to start of "one".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(0));
    }

    #[test]
    fn test_alt_f_forward_word() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("one two three");
        line.cursor_position = seg_index(0); // Start of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('f'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+F should move to start of "two".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(4));

        // Second Alt+F should move to start of "three".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(8));

        // Third Alt+F should move to end (no next word).
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.cursor_position, seg_index(13));
    }

    #[test]
    fn test_alt_d_kill_word() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("foo bar baz");
        line.cursor_position = seg_index(0); // Start of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('d'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+D should delete "foo".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.line.as_str(), " bar baz");

        // Second Alt+D should delete " bar".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.line.as_str(), " baz");
    }

    #[test]
    fn test_alt_backspace_backward_kill_word() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("one two three");
        line.cursor_position = seg_index(13); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Backspace),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });

        // First Alt+Backspace should delete "three".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.line.as_str(), "one two ");

        // Second Alt+Backspace should delete "two ".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.line.as_str(), "one ");
    }

    // Phase 1.3: Tests for interrupt handling.

    #[test]
    fn test_ctrl_c_interrupt() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("some input");
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('c'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Ctrl+C should signal interrupt.
        assert!(matches!(result, Ok(Some(ReadlineEvent::Interrupted))));
    }

    #[test]
    fn test_ctrl_l_clear_screen() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("test");
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('l'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Ctrl+L should clear screen and re-render.
        assert!(matches!(result, Ok(None)));
        // Line content should be preserved.
        assert_eq!(line.line.as_str(), "test");
    }

    #[test]
    fn test_ctrl_u_delete_to_start() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello world");
        line.cursor_position = seg_index(6); // After "hello "
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('u'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Ctrl+U should delete from cursor to start.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "world");
        assert_eq!(line.cursor_position, seg_index(0));
    }

    #[test]
    fn test_ctrl_a_move_to_start() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        line.cursor_position = seg_index(5); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('a'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Ctrl+A should move cursor to start.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(0));
    }

    #[test]
    fn test_ctrl_e_move_to_end() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        line.cursor_position = seg_index(0); // At start
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('e'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Ctrl+E should move cursor to end.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(5));
    }

    #[test]
    fn test_enter_submit_line() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Enter should return the line.
        assert!(matches!(result, Ok(Some(ReadlineEvent::Line(ref s))) if s == "hello"));
        // Line should be cleared after submission.
        assert_eq!(line.line.as_str(), "");
    }

    #[test]
    fn test_backspace_delete_before() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        line.cursor_position = seg_index(5); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Backspace),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Backspace should delete character before cursor.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "hell");
        assert_eq!(line.cursor_position, seg_index(4));
    }

    #[test]
    fn test_delete_key_delete_at_cursor() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        line.cursor_position = seg_index(0); // At start
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Delete),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Delete should delete character at cursor.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "ello");
    }

    #[test]
    fn test_left_arrow_move_left() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        line.cursor_position = seg_index(5); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Left),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Left arrow should move cursor left one position.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(4));
    }

    #[test]
    fn test_home_key_move_to_start() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello world");
        line.cursor_position = seg_index(11); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Home),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Home should move cursor to start of line.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(0));
    }

    #[test]
    fn test_end_key_move_to_end() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello world");
        line.cursor_position = seg_index(0); // At start
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::End),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // End should move cursor to end of line.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(11));
    }

    #[test]
    fn test_down_arrow_history_next() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Add some history entries.
        safe_history.write(|history| history.update(Some("first".to_string())));
        safe_history.write(|history| history.update(Some("second".to_string())));

        // Navigate up first to get into history.
        let up_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        });

        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&up_event, stdout, &safe_history))
            .expect("conversion error");
        assert_eq!(line.line.as_str(), "second");

        // Now test down arrow.
        let down_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Down),
        });

        let result = safe_output_terminal
            .write(|term| line.apply_event_and_render(&down_event, term, &safe_history));

        assert!(matches!(result, Ok(None)));
    }

    // Phase 3: Edge case and Unicode tests.

    #[test]
    fn test_unicode_emoji_word_operations() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello 🎉 world");
        line.cursor_position = seg_index(14); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('w'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // Ctrl+W should delete "world".
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");

        // Should have "hello 🎉 " remaining.
        assert_eq!(line.line.as_str(), "hello 🎉 ");
    }

    #[test]
    fn test_ctrl_w_empty_line() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('w'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        // Should not panic or error on empty line.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "");
    }

    #[test]
    fn test_word_boundaries_with_only_punctuation() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("...---===");
        line.cursor_position = seg_index(9); // At end
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // Ctrl+Left on punctuation-only string.
        safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history))
            .expect("conversion error");

        // Punctuation-only strings are treated as one "word", so jumps to start.
        assert_eq!(line.cursor_position, seg_index(0));
    }

    #[test]
    fn test_ctrl_left_unicode() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Setup: "hello 世界 test"
        line.line = GCStringOwned::new("hello 世界 test");
        line.cursor_position = seg_index(16); // At end

        let ctrl_left_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+Left: should move to start of "test".
        let result = safe_output_terminal.write(|term| {
            line.apply_event_and_render(&ctrl_left_event, term, &safe_history)
        });

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(9)); // Start of "test"

        // Second Ctrl+Left: should move to start of "世界".
        let result = safe_output_terminal.write(|term| {
            line.apply_event_and_render(&ctrl_left_event, term, &safe_history)
        });

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(6)); // Start of "世界"

        // Third Ctrl+Left: should move to start of "hello".
        let result = safe_output_terminal.write(|term| {
            line.apply_event_and_render(&ctrl_left_event, term, &safe_history)
        });

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(0)); // Start of line
    }

    // ===================================================================================
    // Phase 4: Tests for new ReadlineEvent variants (Tab, PageUp/Down, FnKey, etc.)
    // ===================================================================================

    /// Test that F1-F12 keys are correctly converted to FnKey(1)-FnKey(12).
    #[test]
    fn test_fnkey_f1_through_f12() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Test all function keys F1-F12.
        let test_cases = [
            (FunctionKey::F1, 1),
            (FunctionKey::F2, 2),
            (FunctionKey::F3, 3),
            (FunctionKey::F4, 4),
            (FunctionKey::F5, 5),
            (FunctionKey::F6, 6),
            (FunctionKey::F7, 7),
            (FunctionKey::F8, 8),
            (FunctionKey::F9, 9),
            (FunctionKey::F10, 10),
            (FunctionKey::F11, 11),
            (FunctionKey::F12, 12),
        ];

        for (fn_key, expected_num) in test_cases {
            let event = InputEvent::Keyboard(KeyPress::Plain {
                key: Key::FunctionKey(fn_key),
            });

            let result = safe_output_terminal.write(|stdout| {
                line.apply_event_and_render(&event, stdout, &safe_history)
            });
            assert!(
                matches!(result, Ok(Some(ReadlineEvent::FnKey(n))) if n.get() == expected_num),
                "Expected FnKey({expected_num}) for {fn_key:?}, got {result:?}"
            );
        }
    }

    /// Test comprehensive `SpecialKey` -> `ReadlineEvent` mapping for pass-through keys.
    /// These are keys that readline doesn't handle internally - they're passed to caller.
    #[test]
    #[allow(clippy::type_complexity)]
    fn test_passthrough_special_keys() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Test pass-through keys: Tab, BackTab, PageUp, PageDown, Insert.
        let test_cases: &[(SpecialKey, fn(&ReadlineEvent) -> bool)] = &[
            (SpecialKey::Tab, |e| matches!(e, ReadlineEvent::Tab)),
            (SpecialKey::BackTab, |e| matches!(e, ReadlineEvent::BackTab)),
            (SpecialKey::PageUp, |e| matches!(e, ReadlineEvent::PageUp)),
            (SpecialKey::PageDown, |e| {
                matches!(e, ReadlineEvent::PageDown)
            }),
            (SpecialKey::Insert, |e| matches!(e, ReadlineEvent::Insert)),
        ];

        for (special_key, matcher) in test_cases {
            let event = InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(*special_key),
            });

            let result = safe_output_terminal.write(|stdout| {
                line.apply_event_and_render(&event, stdout, &safe_history)
            });
            match result {
                Ok(Some(ref readline_event)) => {
                    assert!(
                        matcher(readline_event),
                        "Unexpected event for {special_key:?}: got {readline_event:?}"
                    );
                }
                other => {
                    panic!("Expected Ok(Some(_)) for {special_key:?}, got {other:?}")
                }
            }
        }
    }

    /// Test that internally-handled keys return Ok(None) (they modify state, not return
    /// events).
    #[test]
    fn test_internal_special_keys_return_none() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("test");
        line.cursor_position = seg_index(2); // Middle of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // These keys modify state and return Ok(None).
        let internal_keys = [
            SpecialKey::Left,
            SpecialKey::Right,
            SpecialKey::Home,
            SpecialKey::End,
            SpecialKey::Up,
            SpecialKey::Down,
            // Note: Backspace/Delete need specific cursor positions to work
        ];

        for special_key in internal_keys {
            // Reset line state for each test.
            line.line = GCStringOwned::new("test");
            line.cursor_position = seg_index(2);

            let event = InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(special_key),
            });

            let result = safe_output_terminal.write(|stdout| {
                line.apply_event_and_render(&event, stdout, &safe_history)
            });
            assert!(
                matches!(result, Ok(None)),
                "Expected Ok(None) for internal key {special_key:?}, got {result:?}"
            );
        }
    }

    /// Test that Esc key (and other unhandled `SpecialKey`s) return `UnhandledKey`.
    #[test]
    fn test_unhandled_special_key_returns_unhandled_event() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Esc is not explicitly handled, so it should return UnhandledKey.
        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Esc),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(
            matches!(result, Ok(Some(ReadlineEvent::UnhandledKey(_)))),
            "Expected UnhandledKey for Esc, got {result:?}"
        );
    }

    #[test]
    fn test_handle_char_combining_characters() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Type 'e'.
        let event_e = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('e'),
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event_e, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "e");
        assert_eq!(line.line.segment_count().as_usize(), 1);
        assert_eq!(line.cursor_position, seg_index(1));

        // Type combining acute accent '\u{0301}'.
        let event_accent = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('\u{0301}'),
        });
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&event_accent, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "e\u{0301}");
        // Segment count should still be 1 (one grapheme cluster 'é').
        assert_eq!(line.line.segment_count().as_usize(), 1);
        // Cursor grapheme index should NOT advance.
        assert_eq!(line.cursor_position, seg_index(1));

        // Type 'b'.
        let event_b = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('b'),
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event_b, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "e\u{0301}b");
        assert_eq!(line.line.segment_count().as_usize(), 2);
        assert_eq!(line.cursor_position, seg_index(2));

        // Move cursor back to middle (after "e\u{0301}").
        line.cursor_position = seg_index(1);
        // Type another character 'x' in the middle.
        let event_x = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('x'),
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event_x, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "e\u{0301}xb");
        assert_eq!(line.line.segment_count().as_usize(), 3);
        assert_eq!(line.cursor_position, seg_index(2));
    }

    #[test]
    fn test_resize_event() {
        let mut line = LineState::new(String::new(), vp_width(80) + vp_height(24));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let new_size = vp_width(120) + vp_height(40);
        let event = InputEvent::Resize(new_size);

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(
            matches!(result, Ok(Some(ReadlineEvent::Resized(size))) if size == new_size)
        );
        assert_eq!(line.term_size, new_size);
    }

    #[test]
    fn test_unhandled_modifier_keys_and_wildcard_events() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Unhandled Ctrl key (e.g. Ctrl+K) returns Ok(None).
        let ctrl_k = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('k'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&ctrl_k, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));

        // Unhandled Alt key (e.g. Alt+Z) returns Ok(None).
        let alt_z = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('z'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&alt_z, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));

        // Multi-modifier fallback branch (Shift+Char('A')) calls handle_regular_key.
        let shift_a = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('A'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::NotPressed,
            },
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&shift_a, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "A");

        // Non-keyboard, non-resize event (e.g. BracketedPaste) returns Ok(None).
        let paste_event = InputEvent::BracketedPaste("pasted text".to_string());
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&paste_event, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
    }

    #[test]
    fn test_boundary_cursor_no_ops() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        line.cursor_position = seg_index(0);
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Backspace at index 0 is a no-op.
        let backspace = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Backspace),
        });
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&backspace, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "hello");
        assert_eq!(line.cursor_position, seg_index(0));

        // Alt+Backspace at index 0 is a no-op.
        let alt_backspace = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Backspace),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&alt_backspace, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "hello");

        // Ctrl+U at index 0 is a no-op.
        let ctrl_u = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('u'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&ctrl_u, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "hello");

        // Move cursor to end (index 5).
        line.cursor_position = seg_index(5);

        // Delete key at end of line is a no-op.
        let delete_key = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Delete),
        });
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&delete_key, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "hello");
        assert_eq!(line.cursor_position, seg_index(5));

        // Alt+D at end of line is a no-op.
        let alt_d = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('d'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&alt_d, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "hello");
    }

    #[test]
    fn test_paused_line_state_suppresses_events() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello");
        line.pause_state = PauseState::PausedByModal;
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Character input while paused does not mutate line.
        let char_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('x'),
        });
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&char_event, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.line.as_str(), "hello");

        // Resize while paused does not mutate term_size.
        let resize_event = InputEvent::Resize(vp_width(200) + vp_height(50));
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&resize_event, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.term_size, vp_width(100) + vp_height(100));
    }

    #[test]
    fn test_print_line_on_control_c() {
        let mut line =
            LineState::new("prompt> ".to_string(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("cancelled input");
        line.print_line_on_control_c = PrintLineOnControlC::Print;

        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('c'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(matches!(result, Ok(Some(ReadlineEvent::Interrupted))));
        let output = stdout_mock.get_copy_of_buffer_as_string();
        assert!(output.contains("prompt> cancelled input"));
    }

    #[test]
    fn test_do_not_print_line_on_enter() {
        let mut line =
            LineState::new("prompt> ".to_string(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("secret password");
        line.print_line_on_enter = PrintLineOnEnter::DoNotPrint;

        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        });

        let result = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&event, stdout, &safe_history));

        assert!(
            matches!(result, Ok(Some(ReadlineEvent::Line(ref s))) if s == "secret password")
        );
        let output = stdout_mock.get_copy_of_buffer_as_string();
        assert!(!output.contains("secret password\n"));
    }

    #[test]
    fn test_word_navigation_at_boundaries_no_ops() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        line.line = GCStringOwned::new("hello world");
        line.cursor_position = seg_index(0);

        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Ctrl+Left at index 0 is a no-op.
        let ctrl_left = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&ctrl_left, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(0));

        // Alt+B at index 0 is a no-op.
        let alt_b = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('b'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&alt_b, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(0));

        // Move cursor to end of string (index 11).
        line.cursor_position = seg_index(11);

        // Ctrl+Right at end of string is a no-op.
        let ctrl_right = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Right),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });
        let res = safe_output_terminal.write(|stdout| {
            line.apply_event_and_render(&ctrl_right, stdout, &safe_history)
        });
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(11));

        // Alt+F at end of string is a no-op.
        let alt_f = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('f'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        });
        let res = safe_output_terminal
            .write(|stdout| line.apply_event_and_render(&alt_f, stdout, &safe_history));
        assert!(matches!(res, Ok(None)));
        assert_eq!(line.cursor_position, seg_index(11));
    }
}

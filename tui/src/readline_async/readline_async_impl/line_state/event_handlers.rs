// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::core::LineState;
use crate::{AnsiSequenceGenerator, CsiSequence, EraseDisplayMode, FunctionKey,
            GCStringOwned, InputEvent, Key, KeyPress, KeyState, LineStateLiveness,
            NumericValue, ReadlineError, ReadlineEvent, SafeHistory, Size, SpecialKey,
            col, early_return_if_paused, find_next_word_end, find_next_word_start,
            find_prev_word_start, row, seg_index};
use std::{io::Write, num::NonZeroU8};
use unicode_segmentation::UnicodeSegmentation;

/// Get the byte offset at a given segment index.
///
/// Returns the byte position where the segment at `seg_idx` starts.
/// If `seg_idx` is beyond the end, returns the total byte length.
fn get_byte_offset_at_seg_index(line: &GCStringOwned, seg_idx: usize) -> usize {
    if seg_idx >= line.segment_count().as_usize() {
        return line.bytes_size().as_usize();
    }
    if let Some(seg) = line.get(seg_index(seg_idx)) {
        seg.start_byte_index.as_usize()
    } else {
        line.bytes_size().as_usize()
    }
}

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
        Key::Character(c) => handle_char(line_state, term, *c),

        // Pass-through keys (return event for caller to handle).
        Key::SpecialKey(SpecialKey::Tab) => Ok(Some(ReadlineEvent::Tab)),
        Key::SpecialKey(SpecialKey::BackTab) => Ok(Some(ReadlineEvent::BackTab)),
        Key::SpecialKey(SpecialKey::PageUp) => Ok(Some(ReadlineEvent::PageUp)),
        Key::SpecialKey(SpecialKey::PageDown) => Ok(Some(ReadlineEvent::PageDown)),
        Key::SpecialKey(SpecialKey::Insert) => Ok(Some(ReadlineEvent::Insert)),

        // Function keys F1-F12.
        // unwrap() is safe: literal values 1-12 are guaranteed non-zero.
        #[allow(clippy::unwrap_in_result)]
        Key::FunctionKey(fn_key) => Ok(Some(ReadlineEvent::FnKey(match fn_key {
            FunctionKey::F1 => NonZeroU8::new(1).unwrap(),
            FunctionKey::F2 => NonZeroU8::new(2).unwrap(),
            FunctionKey::F3 => NonZeroU8::new(3).unwrap(),
            FunctionKey::F4 => NonZeroU8::new(4).unwrap(),
            FunctionKey::F5 => NonZeroU8::new(5).unwrap(),
            FunctionKey::F6 => NonZeroU8::new(6).unwrap(),
            FunctionKey::F7 => NonZeroU8::new(7).unwrap(),
            FunctionKey::F8 => NonZeroU8::new(8).unwrap(),
            FunctionKey::F9 => NonZeroU8::new(9).unwrap(),
            FunctionKey::F10 => NonZeroU8::new(10).unwrap(),
            FunctionKey::F11 => NonZeroU8::new(11).unwrap(),
            FunctionKey::F12 => NonZeroU8::new(12).unwrap(),
        }))),

        // Catch-all for unhandled keys.
        _ => Ok(Some(ReadlineEvent::UnhandledKey(KeyPress::Plain { key: *key }))),
    }
}

/// Handle terminal resize events.
pub fn handle_resize(
    line_state: &mut LineState,
    size: Size,
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
    if line_state.should_print_line_on_control_c && !line_state.is_paused.is_paused() {
        line_state.print_and_flush(
            &format!("{}{}", line_state.prompt, line_state.line.as_str()),
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
    term.write_all(
        CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen)
            .to_string()
            .as_bytes(),
    )?;
    term.write_all(AnsiSequenceGenerator::cursor_position(row(0), col(0)).as_bytes())?;
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
    if !line_state.line_cursor_grapheme.is_zero() {
        // Get byte offset at cursor using segment metadata.
        let cursor_byte_pos = get_byte_offset_at_seg_index(
            &line_state.line,
            line_state.line_cursor_grapheme.as_usize(),
        );

        // Create new string without the deleted portion.
        let remaining = &line_state.line.as_str()[cursor_byte_pos..];
        line_state.line = GCStringOwned::new(remaining);
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

    if line_state.line_cursor_grapheme.is_zero() {
        return Ok(None); // Nothing to delete
    }
    let cursor_pos = line_state.line_cursor_grapheme.as_usize();

    // Find start of previous word using word_boundaries module.
    let word_start = find_prev_word_start(line_state.line.as_str(), cursor_pos);

    if word_start < cursor_pos {
        // Get byte indices using segment metadata.
        let start_byte = get_byte_offset_at_seg_index(&line_state.line, word_start);
        let end_byte = get_byte_offset_at_seg_index(&line_state.line, cursor_pos);

        // Create new string with the word deleted.
        let left = &line_state.line.as_str()[..start_byte];
        let right = &line_state.line.as_str()[end_byte..];
        line_state.line = GCStringOwned::new(format!("{left}{right}"));

        // Move cursor to deletion point.
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

    if !line_state.line_cursor_grapheme.is_zero() {
        let cursor_pos = line_state.line_cursor_grapheme.as_usize();
        // Find start of previous word using word_boundaries module.
        let word_start = find_prev_word_start(line_state.line.as_str(), cursor_pos);
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

    let cursor_pos = line_state.line_cursor_grapheme.as_usize();
    let line_len = line_state.line.segment_count().as_usize();

    if cursor_pos < line_len {
        // Find start of next word using word_boundaries module.
        let word_start = find_next_word_start(line_state.line.as_str(), cursor_pos);
        #[allow(clippy::cast_possible_wrap)]
        let movement = word_start as isize - cursor_pos as isize;
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

    if !line_state.line_cursor_grapheme.is_zero() {
        let cursor_pos = line_state.line_cursor_grapheme.as_usize();
        // Find start of previous word.
        let word_start = find_prev_word_start(line_state.line.as_str(), cursor_pos);
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

    let cursor_pos = line_state.line_cursor_grapheme.as_usize();
    let line_len = line_state.line.segment_count().as_usize();

    if cursor_pos < line_len {
        // Find start of next word.
        let word_start = find_next_word_start(line_state.line.as_str(), cursor_pos);
        #[allow(clippy::cast_possible_wrap)]
        let movement = word_start as isize - cursor_pos as isize;
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

    let cursor_pos = line_state.line_cursor_grapheme.as_usize();
    let line_len = line_state.line.segment_count().as_usize();

    if cursor_pos < line_len {
        // Find end of current/next word.
        let word_end = find_next_word_end(line_state.line.as_str(), cursor_pos);

        if word_end > cursor_pos {
            // Get byte indices using segment metadata.
            let start_byte = get_byte_offset_at_seg_index(&line_state.line, cursor_pos);
            let end_byte = get_byte_offset_at_seg_index(&line_state.line, word_end);

            // Create new string with the word deleted.
            let left = &line_state.line.as_str()[..start_byte];
            let right = &line_state.line.as_str()[end_byte..];
            line_state.line = GCStringOwned::new(format!("{left}{right}"));
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

    if line_state.line_cursor_grapheme.is_zero() {
        return Ok(None);
    }
    let cursor_pos = line_state.line_cursor_grapheme.as_usize();

    // Find start of previous word.
    let word_start = find_prev_word_start(line_state.line.as_str(), cursor_pos);

    if word_start < cursor_pos {
        // Get byte indices using segment metadata.
        let start_byte = get_byte_offset_at_seg_index(&line_state.line, word_start);
        let end_byte = get_byte_offset_at_seg_index(&line_state.line, cursor_pos);

        // Create new string with the word deleted.
        let left = &line_state.line.as_str()[..start_byte];
        let right = &line_state.line.as_str()[end_byte..];
        line_state.line = GCStringOwned::new(format!("{left}{right}"));

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
            &format!("{}{}\n", line_state.prompt, line_state.line.as_str()),
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
    if let Some(seg) = line_state.current_grapheme() {
        line_state.clear(term)?;
        let start = seg.start_byte_index.as_usize();
        let end = start + seg.bytes_size.as_usize();

        // Create new string without the deleted character.
        let left = &line_state.line.as_str()[..start];
        let right = &line_state.line.as_str()[end..];
        line_state.line = GCStringOwned::new(format!("{left}{right}"));

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
    if let Some(seg) = line_state.next_grapheme() {
        line_state.clear(term)?;
        let start = seg.start_byte_index.as_usize();
        let end = start + seg.bytes_size.as_usize();

        // Create new string without the deleted character.
        let left = &line_state.line.as_str()[..start];
        let right = &line_state.line.as_str()[end..];
        line_state.line = GCStringOwned::new(format!("{left}{right}"));

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

// Move cursor to end.
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

// Navigate to older history entry.
#[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
fn handle_up(
    line_state: &mut LineState,
    term: &mut dyn Write,
    safe_history: &SafeHistory,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    if let Some(line) = safe_history.lock().unwrap().search_next() {
        line_state.line = GCStringOwned::new(line);
        line_state.clear(term)?;
        line_state.move_cursor(100_000)?;
        line_state.render_and_flush(term)?;
    }
    Ok(None)
}

// Navigate to newer history entry.
#[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
fn handle_down(
    line_state: &mut LineState,
    term: &mut dyn Write,
    safe_history: &SafeHistory,
) -> Result<Option<ReadlineEvent>, ReadlineError> {
    if let Some(line) = safe_history.lock().unwrap().search_previous() {
        line_state.line = GCStringOwned::new(line);
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

    // Get byte position after current grapheme (insertion point).
    let insert_byte_pos = if let Some(seg) = line_state.current_grapheme() {
        seg.start_byte_index.as_usize() + seg.bytes_size.as_usize()
    } else {
        0
    };

    // Insert character by rebuilding the string.
    let left = &line_state.line.as_str()[..insert_byte_pos];
    let right = &line_state.line.as_str()[insert_byte_pos..];
    line_state.line = GCStringOwned::new(format!("{left}{c}{right}"));

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

impl LineState {
    /// Processes an input event, updates line state, and renders changes to the terminal.
    ///
    /// This is the **core event processing method** for the readline event loop. It:
    /// 1. Receives an input event (keyboard, resize, mouse, etc.)
    /// 2. Updates the internal line state (text, cursor position, history)
    /// 3. Renders the updated state to the terminal
    /// 4. Returns any significant events that the caller needs to handle
    ///
    /// # Return Value
    ///
    /// Returns `Ok(Some(ReadlineEvent))` when a **significant event** occurs that the
    /// caller should handle:
    /// - [`ReadlineEvent::Line`] - User pressed Enter, line is complete
    /// - [`ReadlineEvent::Eof`] - User pressed Ctrl+D on empty line
    /// - [`ReadlineEvent::Resized`] - Terminal was resized
    ///
    /// Returns `Ok(None)` for **normal editing operations** that don't require caller
    /// action:
    /// - Character insertion/deletion
    /// - Cursor movement (arrow keys, Home, End, Ctrl+Left/Right, Alt+B/F)
    /// - Word deletion (Ctrl+W, Alt+D, Alt+Backspace)
    /// - Line editing (Ctrl+A, Ctrl+E, Ctrl+K, Ctrl+U)
    /// - History navigation (Up/Down arrows)
    ///
    /// # Examples
    ///
    /// ## Basic Usage (Simulated Events)
    ///
    /// ```rust
    /// use r3bl_tui::{InputEvent, KeyPress, SpecialKey, LineState, StdoutMock, ReadlineEvent,
    ///               seg_index};
    /// use std::sync::{Arc, Mutex};
    ///
    /// // Setup
    /// let mut line_state = LineState::new(String::new(), (80, 24));
    /// let mut stdout = StdoutMock::default();
    /// let (history, _) = r3bl_tui::readline_async::readline_async_impl::History::new();
    /// let safe_history = Arc::new(Mutex::new(history));
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
    ///     ).unwrap();
    ///
    ///     // Normal character input returns None
    ///     assert!(result.is_none());
    /// }
    ///
    /// assert_eq!(line_state.line.as_str(), "hello");
    /// assert_eq!(line_state.line_cursor_grapheme, seg_index(5));
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
    /// ).unwrap();
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
    /// - [`pty_ctrl_navigation_test`] - Shows full PTY test pattern with debouncing
    /// - [`pty_ctrl_d_eof_test`] - Shows handling of Ctrl+D as EOF
    /// - [`pty_ctrl_d_delete_test`] - Shows handling of Ctrl+D as delete
    ///
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails or if the event cannot be
    /// processed.
    ///
    /// [`pty_ctrl_d_delete_test`]: crate::readline_async::readline_async_impl::integration_tests::pty_ctrl_d_delete_test
    /// [`pty_ctrl_d_eof_test`]: crate::readline_async::readline_async_impl::integration_tests::pty_ctrl_d_eof_test
    /// [`pty_ctrl_navigation_test`]: crate::readline_async::readline_async_impl::integration_tests::pty_ctrl_navigation_test
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    pub fn apply_event_and_render(
        &mut self,
        event: &InputEvent,
        term: &mut dyn Write,
        safe_history: &SafeHistory,
    ) -> Result<Option<ReadlineEvent>, ReadlineError> {
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
            },
            InputEvent::Resize(size) => handle_resize(self, *size, term),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{History, ModifierKeysMask, StdMutex, core::test_fixtures::StdoutMock};
    use std::sync::Arc;

    // cspell:words ello testx

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

        assert_eq!(line.line.as_str(), "a");
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

        assert_eq!(line.current_column, col(3));
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

        assert_eq!(line.line.as_str(), "");
    }

    // Phase 1.1: Tests for recent bug fixes.

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

        // Ctrl+D on empty line should return EOF.
        assert!(matches!(result, Ok(Some(ReadlineEvent::Eof))));
    }

    #[tokio::test]
    async fn test_ctrl_d_non_empty_deletes_char() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("abc");
        line.line_cursor_grapheme = seg_index(1); // Cursor after 'a'
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

        // Ctrl+D on non-empty line should delete char at cursor.
        assert!(matches!(result, Ok(None)));
        // 'b' should be deleted (char at cursor position).
        assert_eq!(line.line.as_str(), "ac");
    }

    #[tokio::test]
    async fn test_ctrl_w_word_boundaries() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello world");
        line.line_cursor_grapheme = seg_index(11); // At end
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

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // "world" should be deleted, leaving "hello ".
        assert_eq!(line.line.as_str(), "hello ");
    }

    #[tokio::test]
    async fn test_ctrl_left_word_navigation() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello-world foo");
        line.line_cursor_grapheme = seg_index(15); // End of line
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

        // First Ctrl+Left should move to start of "foo".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(12));

        // Second Ctrl+Left should move to start of "world".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(6));

        // Third Ctrl+Left should move to start of "hello".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(0));
    }

    #[tokio::test]
    async fn test_ctrl_right_word_navigation() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello-world foo");
        line.line_cursor_grapheme = seg_index(0); // Start of line
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

        // First Ctrl+Right should move to start of "world" (after hyphen).
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(6));

        // Second Ctrl+Right should move to start of "foo".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(12));

        // Third Ctrl+Right should move to end (no next word).
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(15));
    }

    // Phase 1.2: Tests for new Alt+key handlers.

    #[tokio::test]
    async fn test_alt_b_backward_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("one two three");
        line.line_cursor_grapheme = seg_index(13); // End of line
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

        // First Alt+B should move to start of "three".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(8));

        // Second Alt+B should move to start of "two".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(4));

        // Third Alt+B should move to start of "one".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(0));
    }

    #[tokio::test]
    async fn test_alt_f_forward_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("one two three");
        line.line_cursor_grapheme = seg_index(0); // Start of line
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

        // First Alt+F should move to start of "two".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(4));

        // Second Alt+F should move to start of "three".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(8));

        // Third Alt+F should move to end (no next word).
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line_cursor_grapheme, seg_index(13));
    }

    #[tokio::test]
    async fn test_alt_d_kill_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("foo bar baz");
        line.line_cursor_grapheme = seg_index(0); // Start of line
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

        // First Alt+D should delete "foo".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line.as_str(), " bar baz");

        // Second Alt+D should delete " bar".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line.as_str(), " baz");
    }

    #[tokio::test]
    async fn test_alt_backspace_backward_kill_word() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("one two three");
        line.line_cursor_grapheme = seg_index(13); // At end
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

        // First Alt+Backspace should delete "three".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line.as_str(), "one two ");

        // Second Alt+Backspace should delete "two ".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line.as_str(), "one ");
    }

    // Phase 1.3: Tests for interrupt handling.

    #[tokio::test]
    async fn test_ctrl_c_interrupt() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("some input");
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

        // Ctrl+C should signal interrupt.
        assert!(matches!(result, Ok(Some(ReadlineEvent::Interrupted))));
    }

    #[tokio::test]
    async fn test_ctrl_l_clear_screen() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("test");
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

        // Ctrl+L should clear screen and re-render.
        assert!(matches!(result, Ok(None)));
        // Line content should be preserved.
        assert_eq!(line.line.as_str(), "test");
    }

    #[tokio::test]
    async fn test_ctrl_u_delete_to_start() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello world");
        line.line_cursor_grapheme = seg_index(6); // After "hello "
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

        // Ctrl+U should delete from cursor to start.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "world");
        assert_eq!(line.line_cursor_grapheme, seg_index(0));
    }

    #[tokio::test]
    #[cfg(feature = "emacs")]
    async fn test_ctrl_a_move_to_start() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello");
        line.line_cursor_grapheme = seg_index(5); // At end
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

        // Ctrl+A should move cursor to start.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(0));
    }

    #[tokio::test]
    #[cfg(feature = "emacs")]
    async fn test_ctrl_e_move_to_end() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello");
        line.line_cursor_grapheme = seg_index(0); // At start
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

        // Ctrl+E should move cursor to end.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(5));
    }

    #[tokio::test]
    async fn test_enter_submit_line() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello");
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

        // Enter should return the line.
        assert!(matches!(result, Ok(Some(ReadlineEvent::Line(ref s))) if s == "hello"));
        // Line should be cleared after submission.
        assert_eq!(line.line.as_str(), "");
    }

    #[tokio::test]
    async fn test_backspace_delete_before() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello");
        line.line_cursor_grapheme = seg_index(5); // At end
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

        // Backspace should delete character before cursor.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "hell");
        assert_eq!(line.line_cursor_grapheme, seg_index(4));
    }

    #[tokio::test]
    async fn test_delete_key_delete_at_cursor() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello");
        line.line_cursor_grapheme = seg_index(0); // At start
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

        // Delete should delete character at cursor.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "ello");
    }

    #[tokio::test]
    async fn test_left_arrow_move_left() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello");
        line.line_cursor_grapheme = seg_index(5); // At end
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

        // Left arrow should move cursor left one position.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(4));
    }

    #[tokio::test]
    async fn test_home_key_move_to_start() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello world");
        line.line_cursor_grapheme = seg_index(11); // At end
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

        // Home should move cursor to start of line.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(0));
    }

    #[tokio::test]
    async fn test_end_key_move_to_end() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello world");
        line.line_cursor_grapheme = seg_index(0); // At start
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

        // End should move cursor to end of line.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(11));
    }

    #[tokio::test]
    async fn test_down_arrow_history_next() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Add some history entries.
        safe_history
            .lock()
            .unwrap()
            .update(Some("first".to_string()));
        safe_history
            .lock()
            .unwrap()
            .update(Some("second".to_string()));

        // Navigate up first to get into history.
        let up_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        });

        line.apply_event_and_render(
            &up_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();
        assert_eq!(line.line.as_str(), "second");

        // Now test down arrow.
        let down_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Down),
        });

        let result = line.apply_event_and_render(
            &down_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
    }

    // Phase 3: Edge case and Unicode tests.

    #[tokio::test]
    async fn test_unicode_emoji_word_operations() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("hello 🎉 world");
        line.line_cursor_grapheme = seg_index(14); // At end
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

        // Ctrl+W should delete "world".
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();

        // Should have "hello 🎉 " remaining.
        assert_eq!(line.line.as_str(), "hello 🎉 ");
    }

    #[tokio::test]
    async fn test_ctrl_w_empty_line() {
        let mut line = LineState::new(String::new(), (100, 100));
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

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        // Should not panic or error on empty line.
        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line.as_str(), "");
    }

    #[tokio::test]
    async fn test_word_boundaries_with_only_punctuation() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("...---===");
        line.line_cursor_grapheme = seg_index(9); // At end
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

        // Ctrl+Left on punctuation-only string.
        line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        )
        .unwrap();

        // Punctuation-only strings are treated as one "word", so jumps to start.
        assert_eq!(line.line_cursor_grapheme, seg_index(0));
    }

    #[tokio::test]
    async fn test_ctrl_left_unicode() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Setup: "hello 世界 test"
        line.line = GCStringOwned::new("hello 世界 test");
        line.line_cursor_grapheme = seg_index(16); // At end

        let ctrl_left_event = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Left),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        });

        // First Ctrl+Left: should move to start of "test".
        let result = line.apply_event_and_render(
            &ctrl_left_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(9)); // Start of "test"

        // Second Ctrl+Left: should move to start of "世界".
        let result = line.apply_event_and_render(
            &ctrl_left_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(6)); // Start of "世界"

        // Third Ctrl+Left: should move to start of "hello".
        let result = line.apply_event_and_render(
            &ctrl_left_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        assert_eq!(line.line_cursor_grapheme, seg_index(0)); // Start of line
    }

    // ===================================================================================
    // Phase 4: Tests for new ReadlineEvent variants (Tab, PageUp/Down, FnKey, etc.)
    // ===================================================================================

    /// Test that F1-F12 keys are correctly converted to FnKey(1)-FnKey(12).
    #[tokio::test]
    async fn test_fnkey_f1_through_f12() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
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

            let result = line.apply_event_and_render(
                &event,
                &mut *safe_output_terminal.lock().unwrap(),
                &safe_history,
            );

            assert!(
                matches!(result, Ok(Some(ReadlineEvent::FnKey(n))) if n.get() == expected_num),
                "Expected FnKey({expected_num}) for {fn_key:?}, got {result:?}"
            );
        }
    }

    /// Test comprehensive `SpecialKey` -> `ReadlineEvent` mapping for pass-through keys.
    /// These are keys that readline doesn't handle internally - they're passed to caller.
    #[tokio::test]
    #[allow(clippy::type_complexity)]
    async fn test_passthrough_special_keys() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
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

            let result = line.apply_event_and_render(
                &event,
                &mut *safe_output_terminal.lock().unwrap(),
                &safe_history,
            );

            match result {
                Ok(Some(ref readline_event)) => {
                    assert!(
                        matcher(readline_event),
                        "Unexpected event for {special_key:?}: got {readline_event:?}"
                    );
                }
                other => panic!("Expected Ok(Some(_)) for {special_key:?}, got {other:?}"),
            }
        }
    }

    /// Test that internally-handled keys return Ok(None) (they modify state, not return
    /// events).
    #[tokio::test]
    async fn test_internal_special_keys_return_none() {
        let mut line = LineState::new(String::new(), (100, 100));
        line.line = GCStringOwned::new("test");
        line.line_cursor_grapheme = seg_index(2); // Middle of line
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
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
            line.line_cursor_grapheme = seg_index(2);

            let event = InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(special_key),
            });

            let result = line.apply_event_and_render(
                &event,
                &mut *safe_output_terminal.lock().unwrap(),
                &safe_history,
            );

            assert!(
                matches!(result, Ok(None)),
                "Expected Ok(None) for internal key {special_key:?}, got {result:?}"
            );
        }
    }

    /// Test that Esc key (and other unhandled `SpecialKey`s) return `UnhandledKey`.
    #[tokio::test]
    async fn test_unhandled_special_key_returns_unhandled_event() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Esc is not explicitly handled, so it should return UnhandledKey.
        let event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Esc),
        });

        let result = line.apply_event_and_render(
            &event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(
            matches!(result, Ok(Some(ReadlineEvent::UnhandledKey(_)))),
            "Expected UnhandledKey for Esc, got {result:?}"
        );
    }
}

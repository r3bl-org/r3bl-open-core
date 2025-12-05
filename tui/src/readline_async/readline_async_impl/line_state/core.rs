// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words testx

use crate::{ColIndex, ColWidth, MemoizedLenMap, SegIndex, Size, StringLength, height, ok, width};
use std::io::{self, Write};

/// Controls whether [`LineState`] processes input and renders output.
///
/// When paused, the line state ignores keyboard events and suppresses terminal
/// rendering. This allows other UI elements (like [`Spinner`]) to temporarily
/// take control of the terminal display.
///
/// # States
///
/// - [`Paused`]: Input ignored, rendering suppressed
/// - [`NotPaused`]: Normal operation, processes input and renders
///
/// # Usage
///
/// Use [`LineState::set_paused`] to change the liveness state. When transitioning
/// from `Paused` to `NotPaused`, the line is automatically re-rendered.
///
/// [`Spinner`]: crate::Spinner
/// [`Paused`]: LineStateLiveness::Paused
/// [`NotPaused`]: LineStateLiveness::NotPaused
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LineStateLiveness {
    /// Input is ignored and rendering is suppressed.
    Paused,
    /// Normal operation - input is processed and output is rendered.
    NotPaused,
}

impl LineStateLiveness {
    /// Returns `true` if the state is [`Paused`](LineStateLiveness::Paused).
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

    /// Index of grapheme in line (0-based position within grapheme array).
    pub line_cursor_grapheme: SegIndex,

    /// Column of grapheme in line (0-based terminal column).
    pub current_column: ColIndex,

    /// buffer for holding partial grapheme clusters as they come in
    pub cluster_buffer: String,

    pub prompt: String,

    /// After pressing enter, should we print the line just submitted?
    pub should_print_line_on_enter: bool,

    /// After pressing `control_c` should we print the line just cancelled?
    pub should_print_line_on_control_c: bool,

    /// Length of last incomplete line (for cursor restoration).
    pub last_line_length: ColWidth,
    pub last_line_completed: bool,

    /// Terminal dimensions: `col_width` (columns) and `row_height` (rows).
    pub term_size: Size,

    /// This is the only place where this information is stored. Since pause and resume
    /// ultimately only affect this struct.
    pub is_paused: LineStateLiveness,

    /// Use to memoize the length of strings.
    pub memoized_len_map: MemoizedLenMap,
}

/// Early return from a function if [`LineState`] is paused.
///
/// This macro provides a consistent pattern for checking pause state at the start
/// of methods that should be skipped when the line state is paused.
///
/// # Variants
///
/// - `@None`: Returns `Ok(None)` - for methods returning `Result<Option<T>, E>`
/// - `@Unit`: Returns `Ok(())` - for methods returning `Result<(), E>`
///
/// # Example
///
/// ```rust,ignore
/// pub fn handle_input(&mut self) -> Result<Option<Event>, Error> {
///     early_return_if_paused!(self @None);
///     // ... process input ...
/// }
///
/// pub fn render(&mut self) -> io::Result<()> {
///     early_return_if_paused!(self @Unit);
///     // ... render output ...
/// }
/// ```
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

pub(crate) use early_return_if_paused;

impl LineState {
    /// Create a new `LineState` with the given prompt and terminal size.
    ///
    /// The `term_size` parameter accepts a `(u16, u16)` tuple: `(width_cols, height_rows)`.
    #[must_use]
    pub fn new(prompt: String, term_size: (u16, u16)) -> Self {
        let mut memoized_len_map = MemoizedLenMap::new();
        let current_column =
            StringLength::StripAnsi.calculate(prompt.as_str(), &mut memoized_len_map);
        // Convert (width, height) tuple to Size struct.
        let term_size = Size::new((width(term_size.0), height(term_size.1)));
        Self {
            prompt,
            last_line_completed: true,
            term_size,
            current_column: crate::col(current_column),
            should_print_line_on_enter: true,
            should_print_line_on_control_c: false,
            line: String::new(),
            line_cursor_grapheme: 0.into(),
            cluster_buffer: String::new(),
            last_line_length: width(0),
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
}

#[cfg(test)]
mod tests {
    use super::{LineState, LineStateLiveness};
    use crate::{History, InputEvent, Key, KeyPress, StdMutex, core::test_fixtures::StdoutMock,
                seg_index};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pause_resume_state() {
        let mut line = LineState::new(String::new(), (100, 100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let (history, _) = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        line.line = "test".to_string();
        line.line_cursor_grapheme = seg_index(4);

        // Pause the line state.
        line.set_paused(
            LineStateLiveness::Paused,
            &mut *safe_output_terminal.lock().unwrap(),
        )
        .unwrap();

        // Try to send input while paused - should be ignored.
        let char_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('x'),
        });

        let result = line.apply_event_and_render(
            &char_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Line should be unchanged because it's paused.
        assert_eq!(line.line, "test");
        assert_eq!(line.line_cursor_grapheme, seg_index(4));

        // Resume the line state.
        line.set_paused(
            LineStateLiveness::NotPaused,
            &mut *safe_output_terminal.lock().unwrap(),
        )
        .unwrap();

        // Now input should work.
        let result = line.apply_event_and_render(
            &char_event,
            &mut *safe_output_terminal.lock().unwrap(),
            &safe_history,
        );

        assert!(matches!(result, Ok(None)));
        // Line should now have the character appended.
        assert_eq!(line.line, "testx");
    }
}

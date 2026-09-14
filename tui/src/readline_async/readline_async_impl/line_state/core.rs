// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::Prompt;
use crate::{GCStringOwned, SegIndex, VPSize, VPWidth, seg_index, vp_width};

/// This struct actually handles the line editing, and rendering. This works hand in hand
/// with the [`crate::Readline`] to make sure that the line is rendered correctly, with
/// pause and resume support.
#[derive(Debug)]
pub struct LineState {
    /// The user's input line with pre-computed grapheme cluster metadata.
    ///
    /// Uses [`GCStringOwned`] for efficient Unicode handling: O(1) segment count,
    /// display width, and direct segment access by index. Mutations rebuild the
    /// segment array (acceptable for typical readline input lengths).
    ///
    /// [`GCStringOwned`]: crate::GCStringOwned
    pub line: GCStringOwned,

    /// Index of grapheme in line (0-based position within grapheme array).
    pub cursor_position: SegIndex,

    /// The prompt string and its pre-computed display width.
    ///
    /// See [`Prompt`] for details on [`ANSI`] handling and width calculation.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    pub prompt: Prompt,

    /// After pressing enter, should we print the line just submitted?
    pub print_line_on_enter: PrintLineOnEnter,

    /// After pressing `control_c` should we print the line just cancelled?
    pub print_line_on_control_c: PrintLineOnControlC,

    /// Length of last incomplete line (for cursor restoration).
    pub last_line_length: VPWidth,

    /// Whether the last written data chunk ended with a newline character.
    pub ends_with_newline: EndsWithNewline,

    /// Terminal dimensions: `col_width` (columns) and `row_height` (rows).
    pub term_size: VPSize,

    /// This is the only place where this information is stored. Since pause and resume
    /// ultimately only affect this struct.
    pub pause_state: PauseState,
}

impl LineState {
    /// Creates a new [`LineState`] with the given prompt and terminal size.
    #[must_use]
    pub fn new(prompt: String, term_size: VPSize) -> Self {
        let prompt = Prompt::new(prompt);
        Self {
            prompt,
            ends_with_newline: EndsWithNewline::Yes,
            term_size,
            print_line_on_enter: PrintLineOnEnter::Print,
            print_line_on_control_c: PrintLineOnControlC::DoNotPrint,
            line: GCStringOwned::new(""),
            cursor_position: seg_index(0),
            last_line_length: vp_width(0),
            pause_state: PauseState::NotPaused,
        }
    }

    /// Calculates the display width of a prompt string, excluding [`ANSI`] escape
    /// sequences.
    ///
    /// Delegated to [`Prompt::calculate_width`].
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    #[must_use]
    pub fn calculate_prompt_width(prompt: &str) -> VPWidth {
        Prompt::calculate_width(prompt)
    }
}

/// Controls whether [`LineState`] prints the prompt and line when <kbd>Enter</kbd> is
/// pressed.
///
/// When set to [`Print`], pressing <kbd>Enter</kbd> will print the prompt and submitted
/// text, moving the cursor to the next line. When set to [`DoNotPrint`], the prompt and
/// input are cleared instead.
///
/// [`DoNotPrint`]: PrintLineOnEnter::DoNotPrint
/// [`LineState`]: LineState
/// [`Print`]: PrintLineOnEnter::Print
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrintLineOnEnter {
    /// Print the prompt and entered line on submit.
    #[default]
    Print,

    /// Erase the prompt and input on submit without printing.
    DoNotPrint,
}

/// Controls whether [`LineState`] prints the prompt and line when <kbd>Ctrl+C</kbd> is
/// pressed.
///
/// When set to [`Print`], pressing <kbd>Ctrl+C</kbd> will print the prompt and cancelled
/// text. When set to [`DoNotPrint`], the prompt and input are erased instead.
///
/// [`DoNotPrint`]: PrintLineOnControlC::DoNotPrint
/// [`LineState`]: LineState
/// [`Print`]: PrintLineOnControlC::Print
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrintLineOnControlC {
    /// Print the prompt and entered line on cancellation.
    Print,

    /// Erase the prompt and input on cancellation without printing.
    #[default]
    DoNotPrint,
}

/// Tracks whether the last written data chunk ended with a newline character.
///
/// Used by [`LineState`] to determine whether the cursor needs to be restored when
/// printing subsequent data chunks.
///
/// [`LineState`]: LineState
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndsWithNewline {
    /// The last written data chunk ended with a newline character.
    #[default]
    Yes,

    /// The last written data chunk did not end with a newline character.
    No,
}

/// Controls whether [`LineState`] processes input and renders output.
///
/// When paused, the line state ignores keyboard events and suppresses terminal rendering.
/// This allows other UI elements (like [`Spinner`]) to temporarily take control of the
/// terminal display.
///
/// # Usage
///
/// Pausing is controlled asynchronously via [`LineStateControlSignal`] messages sent
/// through [`SharedWriter::line_state_control_channel_sender`]. When transitioning from
/// a paused state to [`NotPaused`], the line is automatically re-rendered.
///
/// [`choose()`]: crate::choose
/// [`LineStateControlSignal`]: crate::LineStateControlSignal
/// [`NotPaused`]: PauseState::NotPaused
/// [`PausedByBoth`]: PauseState::PausedByBoth
/// [`PausedByModal`]: PauseState::PausedByModal
/// [`PausedBySpinner`]: PauseState::PausedBySpinner
/// [`SharedWriter::line_state_control_channel_sender`]: crate::SharedWriter::line_state_control_channel_sender
/// [`Spinner`]: crate::readline_async::Spinner
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PauseState {
    /// Normal operation: input is processed and output is rendered.
    NotPaused,

    /// Suspended by an active background spinner (see [`Spinner`]).
    ///
    /// [`Spinner`]: crate::readline_async::Spinner
    PausedBySpinner,

    /// Suspended by an active modal interface (see [`choose()`]).
    ///
    /// [`choose()`]: crate::choose
    PausedByModal,

    /// Suspended by both a spinner and a modal interface.
    PausedByBoth,
}

impl PauseState {
    /// Pauses rendering for a background spinner.
    ///
    /// Returns [`PauseStateTransition::Paused`] if rendering was newly suspended.
    pub fn pause_spinner(&mut self) -> PauseStateTransition {
        match *self {
            PauseState::NotPaused => {
                *self = PauseState::PausedBySpinner;
                PauseStateTransition::Paused
            }
            PauseState::PausedByModal => {
                *self = PauseState::PausedByBoth;
                PauseStateTransition::Unchanged
            }
            PauseState::PausedBySpinner | PauseState::PausedByBoth => {
                PauseStateTransition::Unchanged
            }
        }
    }

    /// Resumes rendering after a background spinner completes.
    ///
    /// Returns [`PauseStateTransition::Resumed`] if rendering was fully restored.
    pub fn resume_spinner(&mut self) -> PauseStateTransition {
        match *self {
            PauseState::PausedBySpinner => {
                *self = PauseState::NotPaused;
                PauseStateTransition::Resumed
            }
            PauseState::PausedByBoth => {
                *self = PauseState::PausedByModal;
                PauseStateTransition::Unchanged
            }
            PauseState::NotPaused | PauseState::PausedByModal => {
                PauseStateTransition::Unchanged
            }
        }
    }

    /// Pauses rendering while a modal interface is active.
    ///
    /// Returns [`PauseStateTransition::Paused`] if rendering was newly suspended.
    pub fn pause_modal(&mut self) -> PauseStateTransition {
        match *self {
            PauseState::NotPaused => {
                *self = PauseState::PausedByModal;
                PauseStateTransition::Paused
            }
            PauseState::PausedBySpinner => {
                *self = PauseState::PausedByBoth;
                PauseStateTransition::Unchanged
            }
            PauseState::PausedByModal | PauseState::PausedByBoth => {
                PauseStateTransition::Unchanged
            }
        }
    }

    /// Resumes rendering after a modal interface closes.
    ///
    /// Returns [`PauseStateTransition::Resumed`] if rendering was fully restored.
    pub fn resume_modal(&mut self) -> PauseStateTransition {
        match *self {
            PauseState::PausedByModal => {
                *self = PauseState::NotPaused;
                PauseStateTransition::Resumed
            }
            PauseState::PausedByBoth => {
                *self = PauseState::PausedBySpinner;
                PauseStateTransition::Unchanged
            }
            PauseState::NotPaused | PauseState::PausedBySpinner => {
                PauseStateTransition::Unchanged
            }
        }
    }
}

/// Describes the visibility boundary transition of the terminal line when pausing or
/// resuming.
///
/// Used by [`PauseState`] transition methods to indicate whether screen operations, such
/// as clearing or redrawing the line, are required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseStateTransition {
    /// State transitioned from unpaused to paused.
    ///
    /// Terminal prompt needs clearing.
    Paused,

    /// State transitioned from paused back to unpaused.
    ///
    /// Buffered output should be flushed and the prompt redrawn.
    Resumed,

    /// State transitioned between suspended sub-variants or remained unchanged.
    ///
    /// No terminal clear or redraw is needed.
    Unchanged,
}

/// Early return from a function if [`LineState`] is paused.
///
/// This macro provides a consistent pattern for checking pause state at the start
/// of methods that should be skipped when the line state is paused.
///
/// # Variants
///
/// - `@None`: Returns `Ok(None)` for methods returning `Result<Option<T>, E>`
/// - `@Unit`: Returns `Ok(())` for methods returning `Result<(), E>`
#[macro_export]
macro_rules! early_return_if_paused {
    ($self:ident @None) => {
        if $self.pause_state != $crate::PauseState::NotPaused {
            return Ok(None);
        }
    };

    ($self:ident @Unit) => {
        if $self.pause_state != $crate::PauseState::NotPaused {
            return Ok(());
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{EndsWithNewline, LineState, PauseState, PauseStateTransition,
                PrintLineOnControlC, PrintLineOnEnter};
    use crate::{GCStringOwned, History, InputEvent, Key, KeyPress, StdMutex,
                core::test_fixtures::StdoutMock, seg_index, vp_col, vp_height, vp_width};
    use std::sync::Arc;

    #[test]
    fn test_line_state_new_full_initialization() {
        let line = LineState::new("test> ".into(), vp_width(80) + vp_height(24));
        assert_eq!(line.prompt.as_str(), "test> ");
        assert_eq!(line.calc_current_column(), vp_col(6));
        assert_eq!(line.cursor_position, seg_index(0));
        assert_eq!(line.last_line_length, vp_width(0));
        assert_eq!(line.line.as_str(), "");
        assert_eq!(line.term_size, vp_width(80) + vp_height(24));
        assert_eq!(line.print_line_on_enter, PrintLineOnEnter::Print);
        assert_eq!(
            line.print_line_on_control_c,
            PrintLineOnControlC::DoNotPrint
        );
        assert_eq!(line.ends_with_newline, EndsWithNewline::Yes);
        assert_eq!(line.pause_state, PauseState::NotPaused);
    }

    #[test]
    fn test_pause_state_transitions() {
        let mut state = PauseState::NotPaused;

        // Spinner pause from NotPaused: becomes paused.
        assert_eq!(state.pause_spinner(), PauseStateTransition::Paused);
        assert_eq!(state, PauseState::PausedBySpinner);

        // Spinner pause while already PausedBySpinner: unchanged.
        assert_eq!(state.pause_spinner(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedBySpinner);

        // Modal pause while PausedBySpinner: unchanged, transitions to PausedByBoth.
        assert_eq!(state.pause_modal(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedByBoth);

        // Spinner resume while PausedByBoth: unchanged, transitions to PausedByModal.
        assert_eq!(state.resume_spinner(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedByModal);

        // Modal resume while PausedByModal: becomes active, transitions to NotPaused.
        assert_eq!(state.resume_modal(), PauseStateTransition::Resumed);
        assert_eq!(state, PauseState::NotPaused);

        // Modal pause from NotPaused: becomes paused.
        assert_eq!(state.pause_modal(), PauseStateTransition::Paused);
        assert_eq!(state, PauseState::PausedByModal);

        // Resume from PausedBySpinner when already NotPaused: unchanged.
        let mut unpaused = PauseState::NotPaused;
        assert_eq!(unpaused.resume_spinner(), PauseStateTransition::Unchanged);
        assert_eq!(unpaused, PauseState::NotPaused);
    }

    #[test]
    fn test_pause_resume_state() {
        let mut line = LineState::new(String::new(), vp_width(100) + vp_height(100));
        let stdout_mock = StdoutMock::default();
        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        line.line = GCStringOwned::new("test");
        line.cursor_position = seg_index(4);

        // Pause the line state.
        line.pause_state = PauseState::PausedBySpinner;

        // Try to send input while paused, should be ignored.
        let char_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('x'),
        });

        safe_output_terminal.write(|term| {
            let result = line.apply_event_and_render(&char_event, term, &safe_history);
            assert!(matches!(result, Ok(None)));
        });

        // Line should be unchanged because it's paused.
        assert_eq!(line.line.as_str(), "test");
        assert_eq!(line.cursor_position, seg_index(4));

        // Resume the line state.
        line.pause_state = PauseState::NotPaused;

        // Now input should work.
        safe_output_terminal.write(|term| {
            let result = line.apply_event_and_render(&char_event, term, &safe_history);
            assert!(matches!(result, Ok(None)));
        });

        // Line should now have the character appended.
        assert_eq!(line.line.as_str(), "testx");
    }

    #[test]
    fn test_spinner_standalone_lifecycle() {
        let mut state = PauseState::NotPaused;

        // Spinner pauses from unpaused.
        assert_eq!(state.pause_spinner(), PauseStateTransition::Paused);
        assert_eq!(state, PauseState::PausedBySpinner);

        // Spinner resumes back to unpaused.
        assert_eq!(state.resume_spinner(), PauseStateTransition::Resumed);
        assert_eq!(state, PauseState::NotPaused);
    }

    #[test]
    fn test_interleaved_modal_then_spinner_lifecycle() {
        let mut state = PauseState::NotPaused;

        // 1. Modal opens first.
        assert_eq!(state.pause_modal(), PauseStateTransition::Paused);
        assert_eq!(state, PauseState::PausedByModal);

        // 2. Spinner starts while modal is open.
        assert_eq!(state.pause_spinner(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedByBoth);

        // 3. Modal closes while spinner is still active.
        assert_eq!(state.resume_modal(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedBySpinner);

        // 4. Spinner completes, fully restoring rendering.
        assert_eq!(state.resume_spinner(), PauseStateTransition::Resumed);
        assert_eq!(state, PauseState::NotPaused);
    }

    #[test]
    fn test_calculate_prompt_width_with_ansi() {
        // Plain prompt.
        assert_eq!(LineState::calculate_prompt_width("> "), vp_width(2));

        // Prompt with 8-color ANSI escapes.
        assert_eq!(
            LineState::calculate_prompt_width("\x1b[32m>\x1b[0m "),
            vp_width(2)
        );

        // Prompt with 24-bit truecolor ANSI escapes.
        assert_eq!(
            LineState::calculate_prompt_width("\x1b[38;2;255;100;0mprompt>\x1b[0m "),
            vp_width(8)
        );
    }

    #[test]
    fn test_early_return_if_paused_macro_unit() {
        struct MockContext {
            pause_state: PauseState,
        }

        impl MockContext {
            fn operation(&self) -> Result<(), ()> {
                crate::early_return_if_paused!(self @Unit);
                Err(())
            }
        }

        let paused_ctx = MockContext {
            pause_state: PauseState::PausedBySpinner,
        };
        assert_eq!(paused_ctx.operation(), Ok(()));

        let unpaused_ctx = MockContext {
            pause_state: PauseState::NotPaused,
        };
        assert_eq!(unpaused_ctx.operation(), Err(()));
    }

    #[test]
    fn test_early_return_if_paused_macro_none() {
        struct MockContext {
            pause_state: PauseState,
        }

        impl MockContext {
            fn operation(&self) -> Result<Option<()>, ()> {
                crate::early_return_if_paused!(self @None);
                Ok(Some(()))
            }
        }

        let paused_ctx = MockContext {
            pause_state: PauseState::PausedBySpinner,
        };
        assert_eq!(paused_ctx.operation(), Ok(None));

        let unpaused_ctx = MockContext {
            pause_state: PauseState::NotPaused,
        };
        assert_eq!(unpaused_ctx.operation(), Ok(Some(())));
    }

    #[test]
    fn test_pause_state_idempotent_and_noop_transitions() {
        // pause_spinner on PausedByBoth is unchanged.
        let mut state = PauseState::PausedByBoth;
        assert_eq!(state.pause_spinner(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedByBoth);

        // resume_spinner on PausedByModal is unchanged.
        let mut state = PauseState::PausedByModal;
        assert_eq!(state.resume_spinner(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedByModal);

        // pause_modal on PausedByModal is unchanged.
        let mut state = PauseState::PausedByModal;
        assert_eq!(state.pause_modal(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedByModal);

        // pause_modal on PausedByBoth is unchanged.
        let mut state = PauseState::PausedByBoth;
        assert_eq!(state.pause_modal(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedByBoth);

        // resume_modal on NotPaused is unchanged.
        let mut state = PauseState::NotPaused;
        assert_eq!(state.resume_modal(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::NotPaused);

        // resume_modal on PausedBySpinner is unchanged.
        let mut state = PauseState::PausedBySpinner;
        assert_eq!(state.resume_modal(), PauseStateTransition::Unchanged);
        assert_eq!(state, PauseState::PausedBySpinner);
    }
}

// cspell:words testx mprompt

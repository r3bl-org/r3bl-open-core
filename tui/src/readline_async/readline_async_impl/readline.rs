// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use crate::{ChannelCapacity, CommonResultWithError, History, InputDevice, InputEvent,
            LineState, LineStateControlSignal, LineStateLiveness, ModifierKeysMask,
            OutputDevice, PauseBuffer, SafeHistory, SafeLineState, SafePauseBuffer,
            SendRawTerminal, SharedWriter, StdMutex, execute_commands_no_lock, join,
            key_press, lock_output_device_as_mut};
use crossterm::{ExecutableCommand, QueueableCommand, cursor,
                terminal::{self, Clear, disable_raw_mode}};
use miette::Report as ErrorReport;
use std::{io::{self, Write},
          sync::Arc,
          time::Duration};
use thiserror::Error;
use tokio::{select, spawn,
            sync::{broadcast,
                   mpsc::{self, UnboundedReceiver, UnboundedSender}},
            task::JoinHandle,
            time::sleep};

/// This is an artificial delay amount that is added to hide the jank of displaying the
/// cursor to the terminal when the prompt is first printed, after the terminal is put
/// into raw mode.
pub const READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY: Duration =
    Duration::from_millis(66);

/// # Mental model and overview
///
/// This is a replacement for a [`std::io::BufRead::read_line`] function. It is async. It
/// supports other tasks concurrently writing to the terminal output (via
/// [`SharedWriter`]s). It also supports being paused so that [`crate::Spinner`] can
/// display an indeterminate progress spinner. Then it can be resumed so that the user can
/// type in the terminal. Upon resumption, any queued output from the [`SharedWriter`]s is
/// printed out.
///
/// When you call [`Self::readline()`] it enters an infinite loop. During which you can
/// type things into the multiline editor, which also displays the prompt. You can press
/// up, down, left, right, etc. While in this loop other tasks can send messages to the
/// `Readline` task via the `line` channel, using the
/// [`SharedWriter::line_state_control_channel_sender`].
///
/// When you create a new [`Readline`] instance, a task, is started via
/// [`manage_shared_writer_output::spawn_task_to_monitor_line_control_channel()`]. This
/// task monitors the `line` channel, and processes any messages that are sent to it. This
/// allows the task to be paused, and resumed, and to flush the output from the
/// [`SharedWriter`]s.
///
/// # How or when to terminate the session
///
/// There is no `close()` function on [`Readline`]. You simply drop it. This will cause
/// the terminal to come out of raw mode. And all the buffers will be flushed. However,
/// there are 2 ways to use this [`Readline::readline()`] in a loop or just as a one-off.
/// Each time this function is called, you have to `await` it to return the user input or
/// `Interrupted` or `Eof` signal.
///
/// When creating a new [`crate::ReadlineAsyncContext`] instance, you can use this
/// repeatedly before dropping it. This is because the [`crate::SharedWriter`] is cloned,
/// and the terminal is kept in `raw mode` until the associated [`crate::Readline`] is
/// dropped.
///
/// To fully terminate the session, you can call
/// [`crate::ReadlineAsyncContext::request_shutdown()`] on it's "enclosing context". Then
/// wait for that to complete by calling
/// [`crate::ReadlineAsyncContext::await_shutdown()`]. If a `readline()` function is
/// currently running, it will stop and be dropped as well! This is the beauty of
/// non-blocking terminal input support!
///
/// # Inputs and dependency injection
///
/// There are 2 main resources that must be passed into [`Self::try_new()`]:
/// 1. [`InputDevice`] which contains a resource that implements
///    [`crate::PinnedInputStream`]. This trait represents an async stream of events. It
///    is typically implemented by
///    [`crossterm::event::EventStream`](https://docs.rs/crossterm/latest/crossterm/event/struct.EventStream.html).
///    This is used to get input from the user. However, for testing you can provide your
///    own implementation of this trait.
/// 2. [`OutputDevice`] which contains a resource that implements
///    [`crate::SafeRawTerminal`]. This trait represents a raw terminal. It is typically
///    implemented by [`std::io::Stdout`]. This is used to write to the terminal. However,
///    for testing you can provide your own implementation of this trait.
///
/// Other structs are passed in as well, and these are:
/// 1. `prompt` - This prompt will be displayed to the user.
/// 2. `shutdown_complete_sender` - This is a shutdown channel that is used to signal that
///    the shutdown process is complete.
///
/// # Support for testing
///
/// Almost all the fields of this struct contain `Safe` in their names. This is because
/// they are wrapped in a `Mutex` and `Arc`, so that they can be shared between tasks.
/// This makes it easier to test this struct, because you can mock the terminal output,
/// and the input stream. You can also mock the history, and the pause buffer. This is all
/// possible because of the dependency injection that this struct uses. See the tests for
/// how this is used. If there are some fields that seem a bit uneconomic, in where they
/// come from, it is probably due to the requirement for every part of this system to be
/// testable (easily).
///
/// # Pause and resume
///
/// If the terminal is paused, then any output from the [`SharedWriter`]s will not be
/// printed to the terminal. This is useful when you want to display a spinner, or some
/// other indeterminate progress indicator. The user input from the terminal is not going
/// to be accepted either. Only `Ctrl+C`, and `Ctrl+D` are accepted while paused. This
/// ensures that the user can't enter any input while the terminal is paused. And output
/// from a [`crate::Spinner`] won't clobber the output from the [`SharedWriter`]s or from
/// the user input prompt while [`crate::Readline::readline()`] (or
/// [`crate::ReadlineAsyncContext::read_line`]) is being awaited.
///
/// When the terminal is resumed, then the output from the [`SharedWriter`]s will be
/// printed to the terminal by the [`manage_shared_writer_output::flush_internal()`]
/// method, which drains a buffer that holds any output that was generated while paused,
/// of type [`PauseBuffer`]. The user input prompt will be displayed again, and the user
/// can enter input.
///
/// This is possible, because while paused, the
/// [`manage_shared_writer_output::process_line_control_signal()`] method doesn't actually
/// print anything to the display. When resumed, the
/// [`manage_shared_writer_output::flush_internal()`] method is called, which drains the
/// [`PauseBuffer`] (if there are any messages in it, and prints them out) so nothing is
/// lost!
///
/// References:
/// 1. Review the [`crate::LineState`] struct for more information on exactly how the
///    terminal is paused and resumed, when it comes to accepting or rejecting user input,
///    and rendering output or not.
/// 2. Review the [`crate::ReadlineAsyncContext`] module docs for more information on the
///    mental mode and architecture of this.
///
/// # Usage details
///
/// `Readline` struct allows reading lines of input from a terminal, without blocking the
/// calling thread, while lines are output to the terminal concurrently.
///
/// Terminal input is retrieved by calling [`Readline::readline()`], which returns each
/// complete line of input once the user presses `Enter`.
///
/// Each `Readline` instance is associated with one or more [`SharedWriter`] instances.
///
/// Lines written to an associated `SharedWriter` are output:
/// 1. While retrieving input with [`readline()`][Readline::readline].
/// 2. By calling [`manage_shared_writer_output::flush_internal()`].
///
/// You can provide your own implementation of [`crate::SafeRawTerminal`], like
/// [`OutputDevice`], via [dependency injection](https://developerlife.com/category/DI/),
/// so that you can mock terminal output for testing. You can also extend this struct to
/// adapt your own terminal output using this mechanism. Essentially anything that
/// compiles with `dyn std::io::Write + Send` trait bounds can be used.
#[allow(missing_debug_implementations)]
pub struct Readline {
    /// Device used to write rendered display output to (usually `stdout`).
    pub output_device: OutputDevice,

    /// Device used to get stream of events from user (usually `stdin`).
    pub input_device: InputDevice,

    /// Current line.
    pub safe_line_state: SafeLineState,

    /// Use to send history updates.
    pub history_sender: UnboundedSender<String>,
    /// Use to receive history updates.
    pub history_receiver: UnboundedReceiver<String>,
    /// Manages the history.
    pub safe_history: SafeHistory,

    /// Collects lines that are written to the terminal while the terminal is paused.
    pub safe_is_paused_buffer: SafePauseBuffer,

    /// - Is [Some] if a [`crate::Spinner`] is currently active. This works with the
    ///   signal [`LineStateControlSignal::SpinnerActive`]; this is used to set the
    ///   [`crate::Spinner::shutdown_sender`]. Also works with the
    ///   [`LineStateControlSignal::Pause`] signal.
    /// - Is [None] if no [`crate::Spinner`] is active. Also works with the
    ///   [`LineStateControlSignal::Resume`] signal.
    pub safe_spinner_is_active: Arc<StdMutex<Option<broadcast::Sender<()>>>>,

    /// Shutdown channel.
    shutdown_complete_sender: broadcast::Sender<()>,
}

/// Error returned from [`readline()`][Readline::readline]. Such errors generally require
/// specific procedures to recover from.
#[derive(Debug, Error)]
pub enum ReadlineError {
    /// An internal I/O error occurred.
    #[error(transparent)]
    IO(#[from] io::Error),

    /// `readline()` was called after the [`SharedWriter`] was dropped and everything
    /// written to the `SharedWriter` was already output.
    #[error("line writers closed")]
    Closed,
}

/// For convenience, convert [`ErrorReport`] to [`ReadlineError`],
/// so that `into_diagnostic()` works.
impl From<ErrorReport> for ReadlineError {
    fn from(report: ErrorReport) -> Self {
        ReadlineError::IO(io::Error::other(format!("{report}")))
    }
}

/// Events emitted by [`Readline::readline()`].
#[derive(Debug, PartialEq, Clone)]
pub enum ReadlineEvent {
    /// The user entered a line of text.
    Line(String),

    /// The user pressed `Ctrl+D`.
    Eof,

    /// The user pressed `Ctrl+C`.
    Interrupted,

    /// The terminal was resized.
    Resized,
}

/// Internal control flow for the `readline` method. This is used primarily to make
/// testing easier.
#[derive(Debug, PartialEq, Clone)]
pub enum ControlFlowExtended<T, E> {
    ReturnOk(T),
    ReturnError(E),
    Continue,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ControlFlowLimited<E> {
    ReturnError(E),
    Continue,
}

/// # Task creation, shutdown and cleanup
///
/// The task spawned by
/// [`manage_shared_writer_output::spawn_task_to_monitor_line_control_channel()`] doesn't
/// need to be shutdown, since it will simply `request_shutdown` when the [`Readline`]
/// instance is dropped. The loop awaits on the channel, and when the [`Readline`]
/// instance is dropped, the channel is dropped as well, since the
/// [`tokio::sync::mpsc::channel()`]'s [`tokio::sync::mpsc::Sender`] is dropped when the
/// [`SharedWriter`] associated with the [`Readline`] is dropped.
///
/// # Support for buffering & writing output from [`SharedWriter`]s
///
/// - This module contains the logic for managing the `line_state_control_channel` that's
///   created in [`Readline::try_new()`].
/// - This channel is used to send signals *from* [`SharedWriter`]s *to*
///   [`Readline::readline()`], to control the [`LineState`] of the terminal.
/// - Note that [`Readline::readline()`] must be called in a loop while the user is
///   interacting with the terminal, so that these signals can be processed.
///
/// # Buffering and output
///
/// When the terminal is paused, the output from the [`SharedWriter`]s is buffered in a
/// [`PauseBuffer`]. When the terminal is resumed, the buffer is drained and the output is
/// written to the terminal.
pub mod manage_shared_writer_output {
    use super::{Arc, CommonResultWithError, ControlFlowLimited, JoinHandle, LineState,
                LineStateControlSignal, LineStateLiveness, OutputDevice, PauseBuffer,
                ReadlineError, SafeLineState, SafePauseBuffer, SendRawTerminal,
                StdMutex, broadcast, io, join, lock_output_device_as_mut, mpsc, spawn};

    /// - Receiver end of the channel, which does the actual writing to the terminal.
    /// - The sender end of the channel is in [`crate::SharedWriter`].
    pub fn spawn_task_to_monitor_line_control_channel(
        /* Move */
        mut line_control_channel_receiver: mpsc::Receiver<LineStateControlSignal>,
        safe_line_state: SafeLineState,
        output_device: OutputDevice,
        safe_is_paused_buffer: SafePauseBuffer,
        safe_spinner_is_active: Arc<StdMutex<Option<broadcast::Sender<()>>>>,
        shutdown_complete_sender: broadcast::Sender<()>,
    ) -> JoinHandle<()> {
        spawn(async move {
            loop {
                // Poll line channel for events.
                // This branch is cancel safe because recv is cancel safe.
                let maybe_line_control_signal = line_control_channel_receiver.recv();

                // Channel is open.
                // if-let scope has changed in Rust 2024, so use match here and not
                // if-let.
                #[allow(clippy::single_match_else)]
                match maybe_line_control_signal.await {
                    Some(maybe_line_control_signal) => {
                        let control_flow = process_line_control_signal(
                            maybe_line_control_signal,
                            safe_is_paused_buffer.clone(),
                            safe_line_state.clone(),
                            output_device.clone(),
                            safe_spinner_is_active.clone(),
                        );
                        match control_flow {
                            ControlFlowLimited::ReturnError(_) => {
                                // Initiate shutdown.
                                // We don't care about the result of this operation.
                                shutdown_complete_sender.send(()).ok();
                                break;
                            }
                            ControlFlowLimited::Continue => {
                                // continue.
                            }
                        }
                    }
                    _ => {
                        // Initiate shutdown.
                        // We don't care about the result of this operation.
                        shutdown_complete_sender.send(()).ok();
                        break;
                    }
                }
            }
        })
    }

    /// Process a line control signal. And actually write the line or buffered lines to
    /// the terminal.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    #[allow(clippy::needless_pass_by_value)]
    pub fn process_line_control_signal(
        line_control_signal: LineStateControlSignal,
        self_safe_is_paused_buffer: SafePauseBuffer,
        self_safe_line_state: SafeLineState,
        output_device: OutputDevice,
        self_safe_spinner_is_active: Arc<StdMutex<Option<broadcast::Sender<()>>>>,
    ) -> ControlFlowLimited<ReadlineError> {
        match line_control_signal {
            LineStateControlSignal::ExitReadlineLoop => {
                // This causes the readline loop to request_shutdown by using
                // `Readline::shutdown_sender`.
                return ControlFlowLimited::ReturnError(ReadlineError::Closed);
            }

            // Handle a line of text from user input w/ support for pause & resume.
            LineStateControlSignal::Line(buf) => {
                // Early return if paused. Push the line to pause_buffer, don't render
                // anything, and return!
                let mut line_state = self_safe_line_state.lock().unwrap();
                if line_state.is_paused.is_paused() {
                    let pause_buffer = &mut *self_safe_is_paused_buffer.lock().unwrap();
                    pause_buffer.push(buf);
                    return ControlFlowLimited::Continue;
                }

                // Print the line to the terminal.
                let term = lock_output_device_as_mut!(output_device);
                if let Err(err) = line_state.print_data_and_flush(buf.as_ref(), term) {
                    return ControlFlowLimited::ReturnError(err);
                }
                if let Err(err) = term.flush() {
                    return ControlFlowLimited::ReturnError(err.into());
                }
            }

            // Handle a flush signal.
            LineStateControlSignal::Flush => {
                let is_paused = self_safe_line_state.lock().unwrap().is_paused;
                let term = lock_output_device_as_mut!(output_device);
                let line_state = self_safe_line_state.lock().unwrap();
                // We don't care about the result of this operation.
                flush_internal(&self_safe_is_paused_buffer, is_paused, line_state, term)
                    .ok();
            }

            // Pause the terminal.
            LineStateControlSignal::Pause => {
                let new_value = LineStateLiveness::Paused;
                let term = lock_output_device_as_mut!(output_device);
                let mut line_state = self_safe_line_state.lock().unwrap();
                if line_state.set_paused(new_value, term).is_err() {
                    return ControlFlowLimited::ReturnError(ReadlineError::IO(
                        io::Error::other("failed to pause terminal"),
                    ));
                }
            }

            // Resume the terminal.
            LineStateControlSignal::Resume => {
                let new_value = LineStateLiveness::NotPaused;
                let mut line_state = self_safe_line_state.lock().unwrap();
                let term = lock_output_device_as_mut!(output_device);
                // Resume the terminal.
                if line_state.set_paused(new_value, term).is_err() {
                    return ControlFlowLimited::ReturnError(ReadlineError::IO(
                        io::Error::other("failed to resume terminal"),
                    ));
                }
                // We don't care about the result of this operation.
                flush_internal(&self_safe_is_paused_buffer, new_value, line_state, term)
                    .ok();
            }
            LineStateControlSignal::SpinnerActive(spinner_shutdown_sender) => {
                // Handle spinner active signal & register the spinner shutdown sender.
                let mut spinner_is_active = self_safe_spinner_is_active.lock().unwrap();
                *spinner_is_active = Some(spinner_shutdown_sender);
            }
            LineStateControlSignal::SpinnerInactive => {
                // Handle spinner inactive signal & remove the spinner shutdown sender.
                let mut spinner_is_active = self_safe_spinner_is_active.lock().unwrap();
                let _unused: Option<_> = spinner_is_active.take();
            }
        }

        ControlFlowLimited::Continue
    }

    /// Flush all writers to terminal and erase the prompt string.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    pub fn flush_internal(
        self_safe_is_paused_buffer: &SafePauseBuffer,
        is_paused: LineStateLiveness,
        mut line_state: std::sync::MutexGuard<'_, LineState>,
        term: &mut SendRawTerminal,
    ) -> CommonResultWithError<(), ReadlineError> {
        // If paused, then return!
        if is_paused.is_paused() {
            return Ok(());
        }

        let is_paused_buffer = {
            let paused_text_buffer: PauseBuffer = self_safe_is_paused_buffer
                .lock()
                .unwrap()
                .drain(..)
                .collect();
            join!(
                from: paused_text_buffer,
                each: text,
                delim: "",
                format: "{text}"
            )
        };

        line_state.print_data_and_flush(is_paused_buffer.as_bytes(), term)?;
        line_state.clear_and_render_and_flush(term)?;

        Ok(())
    }
}

impl Drop for Readline {
    fn drop(&mut self) {
        let term = lock_output_device_as_mut!(self.output_device);
        // We don't care about the result of this operation.
        self.safe_line_state.lock().unwrap().exit(term).ok();
        // We don't care about the result of this operation.
        disable_raw_mode().ok();
    }
}

impl Readline {
    /// Create a new instance with an associated [`SharedWriter`]. To customize the
    /// behavior of this instance, you can use the following methods:
    /// - [`Self::should_print_line_on`]
    /// - [`Self::set_max_history`]
    ///
    /// # Smooth cursor display
    ///
    /// There is a delay of
    /// [`READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY`] added before
    /// the cursor is displayed. This is to ensure that the initial display of the cursor
    /// does not appear janky.
    ///
    /// This delay happens in a spawned background task and does not block the caller.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal operations fail.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    #[allow(clippy::needless_pass_by_value)]
    pub fn try_new(
        prompt: String,
        output_device: OutputDevice,
        /* move */ input_device: InputDevice,
        /* move */ shutdown_complete_sender: broadcast::Sender<()>,
        channel_capacity: ChannelCapacity,
    ) -> CommonResultWithError<(Self, SharedWriter), ReadlineError> {
        // Immediately hide the cursor. Then wait for
        // `READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY` to display the cursor
        // (try to eliminate jank). It makes it appear as if the cursor is animated into
        // place.
        {
            let writer = lock_output_device_as_mut!(output_device);
            execute_commands_no_lock!(writer, cursor::Hide);
            execute_commands_no_lock!(writer, terminal::EnableLineWrap);
        } // This drops the writer lock.

        // Enable raw mode. Drop will disable raw mode.
        terminal::enable_raw_mode()?;

        // Line control channel - signals are send to this channel to control `LineState`.
        // A task is spawned to monitor this channel.
        let line_state_control_channel =
            mpsc::channel::<LineStateControlSignal>(channel_capacity.capacity());
        let (line_control_channel_sender, line_state_control_channel_receiver) =
            line_state_control_channel;

        // History setup.
        let (history, history_receiver) = History::new();
        let history_sender = history.sender.clone();
        let safe_history = Arc::new(StdMutex::new(history));

        // Line state.
        let line_state = LineState::new(prompt, terminal::size()?);
        let safe_line_state = Arc::new(StdMutex::new(line_state));

        // Pause buffer.
        let is_paused_buffer = PauseBuffer::new();
        let safe_is_paused_buffer = Arc::new(StdMutex::new(is_paused_buffer));

        // Start task to process line_receiver.
        let safe_spinner_is_active = Arc::new(StdMutex::new(None));
        manage_shared_writer_output::spawn_task_to_monitor_line_control_channel(
            line_state_control_channel_receiver,
            safe_line_state.clone(),
            output_device.clone(),
            safe_is_paused_buffer.clone(),
            safe_spinner_is_active.clone(),
            shutdown_complete_sender.clone(),
        );

        // Create the instance with all the supplied components.
        let readline = Readline {
            output_device: output_device.clone(),
            input_device,
            safe_line_state: safe_line_state.clone(),
            history_sender,
            history_receiver,
            safe_history,
            safe_is_paused_buffer,
            safe_spinner_is_active,
            shutdown_complete_sender,
        };

        // Print the prompt.
        {
            let term = lock_output_device_as_mut!(output_device);
            readline
                .safe_line_state
                .lock()
                .unwrap()
                .render_and_flush(term)?;
        } // Drop the term lock.

        let output_device_clone = output_device.clone();
        spawn({
            async move {
                // In a background task, wait for
                // `READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY` to
                // display the cursor (try to eliminate jank). This does not make
                // caller wait.
                sleep(READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY).await;
                let term = lock_output_device_as_mut!(output_device_clone);
                // We don't care about the result of this operation.
                term.execute(cursor::Show).ok();
            }
        });

        // Create the shared writer.
        let shared_writer = SharedWriter::new(line_control_channel_sender);

        // Return the instance and the shared writer.
        Ok((readline, shared_writer))
    }

    /// Change the prompt.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if updating the prompt fails.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    pub fn update_prompt(
        &mut self,
        prompt: &str,
    ) -> CommonResultWithError<(), ReadlineError> {
        let term = lock_output_device_as_mut!(self.output_device);
        self.safe_line_state
            .lock()
            .unwrap()
            .update_prompt(prompt, term)?;
        Ok(())
    }

    /// Clear the screen.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if clearing the screen fails.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.unwrap() */
    pub fn clear(&mut self) -> CommonResultWithError<(), ReadlineError> {
        let term = lock_output_device_as_mut!(self.output_device);
        term.queue(Clear(terminal::ClearType::All))?;
        self.safe_line_state
            .lock()
            .unwrap()
            .clear_and_render_and_flush(term)?;
        term.flush()?;
        Ok(())
    }

    /// Set maximum history length. The default length is [`crate::HISTORY_SIZE_MAX`].
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    pub fn set_max_history(&mut self, max_size: usize) {
        let mut history = self.safe_history.lock().unwrap();
        history.max_size = max_size;
        history.entries.truncate(max_size);
    }

    /// Set whether the input line should remain on the screen after events.
    ///
    /// If `enter` is true, then when the user presses "Enter", the prompt and the text
    /// they entered will remain on the screen, and the cursor will move to the next line.
    /// If `enter` is false, the prompt & input will be erased instead.
    /// The default value for this is `true`.
    ///
    /// `control_c` similarly controls the behavior for when the user presses `Ctrl+C`.
    /// The default value for this is `false`.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    pub fn should_print_line_on(&mut self, enter: bool, control_c: bool) {
        let mut line_state = self.safe_line_state.lock().unwrap();
        line_state.should_print_line_on_enter = enter;
        line_state.should_print_line_on_control_c = control_c;
    }

    /// This function returns when <kbd>Ctrl+D</kbd>, <kbd>Ctrl+C</kbd>, or
    /// <kbd>Enter</kbd> is pressed with some user input.
    ///
    /// Note that this function can be called repeatedly in a loop. It will return each
    /// line of input as it is entered (and return / `request_shutdown`). The
    /// [`crate::ReadlineAsyncContext`] can be re-used, since the [`crate::SharedWriter`]
    /// is cloned, and the terminal is kept in `raw mode` until the associated
    /// [`crate::Readline`] is dropped.
    ///
    /// Polling function for [`Self::readline`], manages all input and output. Returns
    /// either an [`ReadlineEvent`] or an [`ReadlineError`].
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if reading input fails.
    pub async fn readline(
        &mut self,
    ) -> CommonResultWithError<ReadlineEvent, ReadlineError> {
        let mut shutdown_complete_receiver = self.shutdown_complete_sender.subscribe();

        loop {
            select! {
                // Poll for events.
                // This branch is cancel safe because no state is declared inside the
                // future in the following block.
                // - All the state comes from other variables (self.*).
                // - So if this future is dropped, then the item in the
                //   pinned_input_stream isn't used, and the state isn't modified.
                maybe_input_event = self.input_device.next_input_event() => {
                    if let Some(input_event) = maybe_input_event {
                        match readline_internal::apply_event_to_line_state_and_render(
                            input_event,
                            &self.safe_line_state,
                            lock_output_device_as_mut!(self.output_device),
                            &self.safe_history,
                            &self.safe_spinner_is_active,
                        ) {
                            ControlFlowExtended::ReturnOk(ok_value) => {
                                return Ok(ok_value);
                            },
                            ControlFlowExtended::ReturnError(err_value) => {
                                return Err(err_value);
                            },
                            ControlFlowExtended::Continue => {}
                        }
                    }
                },

                // Poll for history updates.
                // This branch is cancel safe because recv is cancel safe.
                maybe_line = self.history_receiver.recv() => {
                    self.safe_history.lock().unwrap().update(maybe_line);
                },

                // Poll for shutdown signal.
                _ = shutdown_complete_receiver.recv() => {
                    return Err(ReadlineError::Closed);
                }
            }
        }
    }

    /// Add a line to the input history.
    pub fn add_history_entry(&mut self, entry: String) -> Option<()> {
        self.history_sender.send(entry).ok()
    }
}

pub mod readline_internal {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    pub fn apply_event_to_line_state_and_render(
        input_event: InputEvent,
        self_line_state: &SafeLineState,
        term: &mut dyn Write,
        self_safe_history: &SafeHistory,
        self_safe_is_spinner_active: &Arc<StdMutex<Option<broadcast::Sender<()>>>>,
    ) -> ControlFlowExtended<ReadlineEvent, ReadlineError> {
        // Check if this is Ctrl+C or Ctrl+D
        let is_ctrl_c_or_d = input_event.matches_any_of_these_keypresses(&[
            key_press!(@char ModifierKeysMask::new().with_ctrl(), 'c'),
            key_press!(@char ModifierKeysMask::new().with_ctrl(), 'd'),
        ]);

        let mut line_state = self_line_state.lock().unwrap();

        // Intercept Ctrl+C or Ctrl+D here and send a signal to spinner (if it is
        // active). And early return!
        let is_spinner_active = self_safe_is_spinner_active.lock().unwrap().take();
        if is_ctrl_c_or_d && let Some(spinner_shutdown_sender) = is_spinner_active {
            // Send signal to SharedWriter spinner shutdown channel.
            // We don't care about the result of this operation.
            spinner_shutdown_sender.send(()).ok();
            return ControlFlowExtended::Continue;
        }

        // Regular readline event handling - use the canonical InputEvent directly
        let result_maybe_readline_event =
            line_state.apply_event_and_render(&input_event, term, self_safe_history);

        match result_maybe_readline_event {
            Ok(maybe_readline_event) => {
                if let Some(readline_event) = maybe_readline_event {
                    return ControlFlowExtended::ReturnOk(readline_event);
                }
            }
            Err(e) => return ControlFlowExtended::ReturnError(e),
        }

        ControlFlowExtended::Continue
    }

    /// Convert `crossterm::event::Event` to canonical `InputEvent`
    #[must_use] 
    pub fn convert_crossterm_event_to_input_event(
        event: crossterm::event::Event,
    ) -> Option<InputEvent> {
        use crate::{Button, FunctionKey, Key, KeyPress, KeyState, MouseInputKind,
                    SpecialKey};
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent,
                               MouseEventKind};

        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                let key = match code {
                    KeyCode::Char(c) => Key::Character(c),
                    KeyCode::F(n) => {
                        let fn_key = match n {
                            1 => FunctionKey::F1,
                            2 => FunctionKey::F2,
                            3 => FunctionKey::F3,
                            4 => FunctionKey::F4,
                            5 => FunctionKey::F5,
                            6 => FunctionKey::F6,
                            7 => FunctionKey::F7,
                            8 => FunctionKey::F8,
                            9 => FunctionKey::F9,
                            10 => FunctionKey::F10,
                            11 => FunctionKey::F11,
                            12 => FunctionKey::F12,
                            _ => return None,
                        };
                        Key::FunctionKey(fn_key)
                    }
                    KeyCode::Up => Key::SpecialKey(SpecialKey::Up),
                    KeyCode::Down => Key::SpecialKey(SpecialKey::Down),
                    KeyCode::Left => Key::SpecialKey(SpecialKey::Left),
                    KeyCode::Right => Key::SpecialKey(SpecialKey::Right),
                    KeyCode::Home => Key::SpecialKey(SpecialKey::Home),
                    KeyCode::End => Key::SpecialKey(SpecialKey::End),
                    KeyCode::PageUp => Key::SpecialKey(SpecialKey::PageUp),
                    KeyCode::PageDown => Key::SpecialKey(SpecialKey::PageDown),
                    KeyCode::Tab => Key::SpecialKey(SpecialKey::Tab),
                    KeyCode::BackTab => Key::SpecialKey(SpecialKey::BackTab),
                    KeyCode::Delete => Key::SpecialKey(SpecialKey::Delete),
                    KeyCode::Insert => Key::SpecialKey(SpecialKey::Insert),
                    KeyCode::Enter => Key::SpecialKey(SpecialKey::Enter),
                    KeyCode::Backspace => Key::SpecialKey(SpecialKey::Backspace),
                    KeyCode::Esc => Key::SpecialKey(SpecialKey::Esc),
                    _ => return None,
                };

                let mask = crate::ModifierKeysMask {
                    shift_key_state: if modifiers.contains(KeyModifiers::SHIFT) {
                        KeyState::Pressed
                    } else {
                        KeyState::NotPressed
                    },
                    ctrl_key_state: if modifiers.contains(KeyModifiers::CONTROL) {
                        KeyState::Pressed
                    } else {
                        KeyState::NotPressed
                    },
                    alt_key_state: if modifiers.contains(KeyModifiers::ALT) {
                        KeyState::Pressed
                    } else {
                        KeyState::NotPressed
                    },
                };

                let keypress = if mask.shift_key_state == KeyState::NotPressed
                    && mask.ctrl_key_state == KeyState::NotPressed
                    && mask.alt_key_state == KeyState::NotPressed
                {
                    KeyPress::Plain { key }
                } else {
                    KeyPress::WithModifiers { key, mask }
                };

                Some(InputEvent::Keyboard(keypress))
            }
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers,
            }) => {
                let mouse_input = crate::MouseInput {
                    pos: crate::Pos {
                        col_index: crate::ColIndex::from(i32::from(column)),
                        row_index: crate::RowIndex::from(i32::from(row)),
                    },
                    kind: match kind {
                        MouseEventKind::Down(button) => {
                            let btn = match button {
                                crossterm::event::MouseButton::Left => Button::Left,
                                crossterm::event::MouseButton::Right => Button::Right,
                                crossterm::event::MouseButton::Middle => Button::Middle,
                            };
                            MouseInputKind::MouseDown(btn)
                        }
                        MouseEventKind::Up(button) => {
                            let btn = match button {
                                crossterm::event::MouseButton::Left => Button::Left,
                                crossterm::event::MouseButton::Right => Button::Right,
                                crossterm::event::MouseButton::Middle => Button::Middle,
                            };
                            MouseInputKind::MouseUp(btn)
                        }
                        MouseEventKind::Drag(button) => {
                            let btn = match button {
                                crossterm::event::MouseButton::Left => Button::Left,
                                crossterm::event::MouseButton::Right => Button::Right,
                                crossterm::event::MouseButton::Middle => Button::Middle,
                            };
                            MouseInputKind::MouseDrag(btn)
                        }
                        MouseEventKind::Moved => MouseInputKind::MouseMove,
                        MouseEventKind::ScrollUp => MouseInputKind::ScrollUp,
                        MouseEventKind::ScrollDown => MouseInputKind::ScrollDown,
                        MouseEventKind::ScrollLeft => MouseInputKind::ScrollLeft,
                        MouseEventKind::ScrollRight => MouseInputKind::ScrollRight,
                    },
                    maybe_modifier_keys: if modifiers.contains(KeyModifiers::SHIFT)
                        || modifiers.contains(KeyModifiers::CONTROL)
                        || modifiers.contains(KeyModifiers::ALT)
                    {
                        Some(crate::ModifierKeysMask {
                            shift_key_state: if modifiers.contains(KeyModifiers::SHIFT) {
                                KeyState::Pressed
                            } else {
                                KeyState::NotPressed
                            },
                            ctrl_key_state: if modifiers.contains(KeyModifiers::CONTROL) {
                                KeyState::Pressed
                            } else {
                                KeyState::NotPressed
                            },
                            alt_key_state: if modifiers.contains(KeyModifiers::ALT) {
                                KeyState::Pressed
                            } else {
                                KeyState::NotPressed
                            },
                        })
                    } else {
                        None
                    },
                };
                Some(InputEvent::Mouse(mouse_input))
            }
            Event::Resize(width, height) => Some(InputEvent::Resize(crate::Size {
                col_width: crate::ColWidth::from(width),
                row_height: crate::RowHeight::from(height),
            })),
            _ => None,
        }
    }
}

#[cfg(test)]
pub mod readline_test_fixtures {
    use crate::{CrosstermEventResult, InlineVec};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use smallvec::smallvec;

    pub(super) fn get_input_vec() -> InlineVec<CrosstermEventResult> {
        smallvec![
            // a
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
            ))),
            // b
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('b'),
                KeyModifiers::NONE,
            ))),
            // c
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::NONE,
            ))),
            // enter
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
        ]
    }
}

#[cfg(test)]
mod test_readline {
    use super::{Arc, ChannelCapacity, ControlFlowExtended, Duration, History,
                InputDevice, LineStateControlSignal, LineStateLiveness, OutputDevice,
                Readline, ReadlineEvent, StdMutex, broadcast, lock_output_device_as_mut,
                readline_internal, readline_test_fixtures::get_input_vec, sleep};
    use crate::{OutputDeviceExt, TTYResult, is_partially_uninteractive_terminal};

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_readline_internal_process_event_and_terminal_output() {
        let vec = get_input_vec();
        let mut iter = vec.iter();

        let prompt_str = "> ";

        if let TTYResult::IsNotInteractive = is_partially_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let input_device = InputDevice::new_mock(get_input_vec());
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let (readline, _) = Readline::try_new(
            prompt_str.into(),
            output_device.clone(),
            /* move */ input_device,
            /* move */ shutdown_sender,
            ChannelCapacity::Minimal, // Test uses minimal capacity
        )
        .unwrap();

        let safe_is_spinner_active = Arc::new(StdMutex::new(None));

        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history.0));

        // Simulate 'a'.
        let Some(Ok(event)) = iter.next() else {
            panic!();
        };
        let Some(input_event) =
            readline_internal::convert_crossterm_event_to_input_event(event.clone())
        else {
            panic!("Failed to convert event");
        };
        let control_flow = readline_internal::apply_event_to_line_state_and_render(
            input_event,
            &readline.safe_line_state,
            lock_output_device_as_mut!(output_device),
            &safe_history,
            &safe_is_spinner_active,
        );

        assert!(matches!(control_flow, ControlFlowExtended::Continue));
        assert_eq!(readline.safe_line_state.lock().unwrap().line, "a");

        let output_buffer_data = stdout_mock.get_copy_of_buffer_as_string_strip_ansi();
        // println!("\n`{}`\n", output_buffer_data);
        assert!(output_buffer_data.contains("> a"));
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_readline() {
        let prompt_str = "> ";

        if let TTYResult::IsNotInteractive = is_partially_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let input_device = InputDevice::new_mock(get_input_vec());
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let (mut readline, _) = Readline::try_new(
            prompt_str.into(),
            output_device.clone(),
            /* move */ input_device,
            shutdown_sender,
            ChannelCapacity::Minimal, // Test uses minimal capacity
        )
        .unwrap();

        let result = readline.readline().await;
        assert!(matches!(result, Ok(ReadlineEvent::Line(_))));
        pretty_assertions::assert_eq!(
            result.unwrap(),
            ReadlineEvent::Line("abc".to_string())
        );
        pretty_assertions::assert_eq!(readline.safe_line_state.lock().unwrap().line, "");

        let output_buffer_data = stdout_mock.get_copy_of_buffer_as_string_strip_ansi();
        // println!("\n`{}`\n", output_buffer_data);
        assert!(output_buffer_data.contains("> abc"));
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_pause_resume() {
        let prompt_str = "> ";

        if let TTYResult::IsNotInteractive = is_partially_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (output_device, _) = OutputDevice::new_mock();
        let input_device = InputDevice::new_mock(get_input_vec());
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let (readline, shared_writer) = Readline::try_new(
            prompt_str.into(),
            output_device.clone(),
            /* move */ input_device,
            shutdown_sender,
            ChannelCapacity::Minimal, // Test uses minimal capacity
        )
        .unwrap();

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await
            .unwrap();
        sleep(Duration::from_millis(1)).await;

        assert_eq!(
            readline.safe_line_state.lock().unwrap().is_paused,
            LineStateLiveness::Paused
        );

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await
            .unwrap();
        sleep(Duration::from_millis(1)).await;

        assert_eq!(
            readline.safe_line_state.lock().unwrap().is_paused,
            LineStateLiveness::NotPaused
        );
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_pause_resume_with_output() {
        let prompt_str = "> ";

        if let TTYResult::IsNotInteractive = is_partially_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (output_device, _) = OutputDevice::new_mock();
        let input_device = InputDevice::new_mock(get_input_vec());
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let (readline, shared_writer) = Readline::try_new(
            prompt_str.into(),
            output_device.clone(),
            /* move */ input_device,
            shutdown_sender,
            ChannelCapacity::Minimal, // Test uses minimal capacity
        )
        .unwrap();

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await
            .unwrap();
        sleep(Duration::from_millis(1)).await;

        assert_eq!(
            readline.safe_line_state.lock().unwrap().is_paused,
            LineStateLiveness::Paused
        );

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Line("abc".into()))
            .await
            .unwrap();
        sleep(Duration::from_millis(1)).await;

        let pause_buffer = readline.safe_is_paused_buffer.lock().unwrap().clone();
        assert_eq!(pause_buffer.len(), 1);
        assert_eq!(pause_buffer[0], "abc");

        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await
            .unwrap();
        sleep(Duration::from_millis(1)).await;

        assert_eq!(
            readline.safe_line_state.lock().unwrap().is_paused,
            LineStateLiveness::NotPaused
        );
    }
}

#[cfg(test)]
mod test_streams {
    use super::*;
    use crate::core::test_fixtures::gen_input_stream;
    use test_streams::readline_test_fixtures::get_input_vec;

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_generate_event_stream_pinned() {
        use futures_util::StreamExt;

        let mut count = 0;
        let mut it = gen_input_stream(get_input_vec());
        while let Some(event) = it.next().await {
            let lhs = event.unwrap();
            let rhs = get_input_vec()[count].as_ref().unwrap().clone();
            assert_eq!(lhs, rhs);
            count += 1;
        }
    }
}

#[cfg(test)]
mod test_pause_and_resume_support {
    use super::*;
    use crate::core::test_fixtures::StdoutMock;
    use manage_shared_writer_output::flush_internal;
    use std::sync::Mutex;

    #[test]
    fn test_flush_internal_paused() {
        // Create a mock `LineState` with initial data.
        let safe_line_state =
            Arc::new(Mutex::new(LineState::new("> ".to_string(), (100, 100))));

        // Create a mock `SafePauseBuffer` with some paused lines.
        let mut pause_buffer = PauseBuffer::new();
        pause_buffer.push("Paused line 1".into());
        pause_buffer.push("Paused line 2".into());

        // Create a mock `SafeIsPausedBuffer` with the pause buffer.
        let safe_is_paused_buffer = Arc::new(Mutex::new(pause_buffer));

        let mut stdout_mock = StdoutMock::default();

        let line_state = safe_line_state.lock().unwrap();

        // Call the `flush_internal` function.
        let result = flush_internal(
            &safe_is_paused_buffer,
            LineStateLiveness::Paused,
            line_state,
            &mut stdout_mock,
        );

        // Assert that the function returns Ok(())
        assert!(result.is_ok());

        // Assert that the mock terminal received the expected output.
        assert_eq!(stdout_mock.get_copy_of_buffer_as_string_strip_ansi(), "");
    }

    #[test]
    fn test_flush_internal_not_paused() {
        // Create a mock `LineState` with initial data.
        let safe_line_state =
            Arc::new(Mutex::new(LineState::new("> ".to_string(), (100, 100))));

        // Create a mock `SafePauseBuffer` with some paused lines.
        let mut pause_buffer = PauseBuffer::new();
        pause_buffer.push("Paused line 1".into());
        pause_buffer.push("Paused line 2".into());

        // Create a mock `SafeIsPausedBuffer` with the pause buffer.
        let safe_is_paused_buffer = Arc::new(Mutex::new(pause_buffer));

        let mut stdout_mock = StdoutMock::default();

        let line_state = safe_line_state.lock().unwrap();

        // Call the `flush_internal` function.
        let result = flush_internal(
            &safe_is_paused_buffer,
            LineStateLiveness::NotPaused,
            line_state,
            &mut stdout_mock,
        );

        // Assert that the function returns Ok(())
        assert!(result.is_ok());

        // Assert that the mock terminal received the expected output.
        assert_eq!(
            stdout_mock.get_copy_of_buffer_as_string_strip_ansi(),
            "Paused line 1Paused line 2\n> > "
        );
    }
}

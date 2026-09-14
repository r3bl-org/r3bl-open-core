// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Button, ChannelCapacity, CommonResultWithError, Continuation,
            CursorBoundsCheck, CursorPositionBoundsStatus, GCStringOwned, History,
            InputDevice, InputEvent, Key, KeyPress, KeyState, LineState,
            LineStateControlSignal, ModifierKeysMask, MouseInput, OutputDevice,
            PaintMode, PauseBuffer, PauseState, PauseStateTransition,
            PrintLineOnControlC, PrintLineOnEnter, ReadlineLockManager, SafeHistory,
            SafeLineState, SafePauseBuffer, SegIndex, SendRawTerminal, SharedWriter,
            StdMutex, VPHeight, VPSize, VPWidth, execute_commands_no_lock, join,
            key_press, ok, vp_col, vp_row};
use crossterm::{ExecutableCommand, QueueableCommand, cursor,
                terminal::{self, Clear}};
use miette::Report as ErrorReport;
use std::{io::{self, Write},
          num::NonZeroU8,
          sync::Arc,
          time::Duration};
use thiserror::Error;
use tokio::{select, spawn,
            sync::{broadcast,
                   mpsc::{self, UnboundedReceiver, UnboundedSender}},
            task::JoinHandle,
            time::sleep};

/// # Mental model and overview
///
/// This is a replacement for a [`std::io::BufRead::read_line`] function. It is async. It
/// supports other tasks concurrently writing to the terminal output (via
/// [`SharedWriter`]s). It also supports being paused so that [`Spinner`] can display an
/// indeterminate progress spinner. Then it can be resumed so that the user can type in
/// the terminal. Upon resumption, any queued output from the [`SharedWriter`]s is printed
/// out.
///
/// For details on the underlying async orchestration, including [`Pin`] and [`Unpin`]
/// requirements for [`tokio::select!`], see [Core Async Concepts].
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
/// When creating a new [`ReadlineAsyncContext`] instance, you can use this repeatedly
/// before dropping it. This is because the [`SharedWriter`] is cloned, and the terminal
/// is kept in `raw mode` until the associated [`Readline`] is dropped.
///
/// To fully terminate the session, you can call
/// [`ReadlineAsyncContext::request_shutdown`] on it's "enclosing context". Then wait for
/// that to complete by calling [`ReadlineAsyncContext::await_shutdown`]. If a
/// `readline()` function is currently running, it will stop and be dropped as well! This
/// is the beauty of non-blocking terminal input support!
///
/// # Inputs and dependency injection
///
/// There are 2 main resources that must be passed into [`Self::try_new()`]:
/// - [`InputDevice`] which contains a resource that implements [`PinnedInputStream`].
///   This trait represents an async stream of events. It is typically implemented by
///   [`crossterm::event::EventStream`]. This is used to get input from the user. However,
///   for testing you can provide your own implementation of this trait.
/// - [`OutputDevice`] which contains a resource that implements [`SafeRawTerminal`]. This
///   trait represents a raw terminal. It is typically implemented by [`std::io::Stdout`].
///   This is used to write to the terminal. However, for testing you can provide your own
///   implementation of this trait.
///
/// Other structs are passed in as well, and these are:
/// - `prompt` - This prompt will be displayed to the user.
/// - `shutdown_complete_sender` - This is a shutdown channel that is used to signal that
///   the shutdown process is complete.
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
/// # Pause, resume, and modal architecture
///
/// When an interactive terminal application is running, background tasks may concurrently
/// write output via [`SharedWriter`] instances while a user is typing at the readline
/// prompt, or while a specialized sub-UI (like a [`Spinner`] or a modal dialog such as
/// [`choose()`]) is active.
///
/// To prevent display corruption, screen clobbering, and input confusion, the readline
/// subsystem provides an integrated suspension architecture.
///
/// ## Core suspension mechanism
///
/// While the terminal is paused:
/// 1. Any output written to [`SharedWriter`] instances is not printed to the display
///    immediately. Instead, it is routed to an internal [`PauseBuffer`] where messages
///    are queued safely.
/// 2. User input from the terminal keyboard is suppressed. Only `Ctrl+C` (cancellation)
///    and `Ctrl+D` (`Eof`) signals are processed.
///
/// When the terminal is resumed:
/// 1. The [`manage_shared_writer_output::flush_internal()`] method is called.
/// 2. It drains all queued messages from the [`PauseBuffer`] and flushes them to the
///    display so no background output is lost.
/// 3. The prompt is re-displayed and normal keyboard input handling resumes.
///
/// ## Two modes of suspension
///
/// There are two distinct ways to suspend the readline engine, tailored for different use
/// cases:
///
/// 1. **Lightweight pause** ([`ReadlineAsyncContext::pause()`] / [`Spinner`]):
///    - Background [`SharedWriter`] output is buffered and user typing is suppressed, but
///      the prompt remains on the display.
///    - Does not grant exclusive mutable access to the underlying I/O devices.
///    - Ideal for non-interactive background operations, such as displaying an
///      indeterminate progress [`Spinner`].
///
/// 2. **Modal pause** ([`ModalTerminalGuard`] /
///    [`ReadlineAsyncContext::acquire_modal_terminal()`]):
///    - Clears the current prompt from the display, pauses background writes into the
///      [`PauseBuffer`], and takes exclusive control over the terminal.
///    - Grants exclusive mutable access to `(InputDevice, OutputDevice)` via
///      [`ModalTerminalGuard`].
///    - Ideal for interactive full-terminal sub-applications (like [`choose()`]) that
///      require dedicated ownership of keyboard input and screen rendering.
///
/// ## State machine ([`PauseState`])
///
/// Suspension state is tracked by the [`PauseState`] enum on [`LineState`]:
/// - [`PauseState::NotPaused`]: Normal operation; input and output are active.
/// - [`PauseState::PausedBySpinner`]: Suspended by an active [`Spinner`].
/// - [`PauseState::PausedByModal`]: Suspended by an active [`ModalTerminalGuard`].
/// - [`PauseState::PausedByBoth`]: Suspended concurrently by both a spinner and a modal
///   guard.
///
/// The state machine guarantees that if a modal dialog opens while a spinner is active,
/// closing the modal dialog returns the state safely to [`PauseState::PausedBySpinner`]
/// rather than prematurely resuming background output or redrawing the prompt.
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
/// - While retrieving input with [`readline()`][Readline::readline].
/// - By calling [`manage_shared_writer_output::flush_internal()`].
///
/// You can provide your own implementation of [`SafeRawTerminal`], like [`OutputDevice`],
/// via [dependency injection], so that you can mock terminal output for testing. You can
/// also extend this struct to adapt your own terminal output using this mechanism.
/// Essentially anything that compiles with `dyn std::io::Write + Send` trait bounds can
/// be used.
///
/// # Poison Safety
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in the
/// crate root documentation for why this is designed to be poison-safe.
///
/// [`choose()`]: crate::choose
/// [`crossterm::event::EventStream`]:
///     https://docs.rs/crossterm/latest/crossterm/event/struct.EventStream.html
/// [`LineState`]: crate::readline_async::LineState
/// [`ModalTerminalGuard`]: crate::ModalTerminalGuard
/// [`PauseBuffer`]: crate::PauseBuffer
/// [`PauseState::NotPaused`]: crate::PauseState::NotPaused
/// [`PauseState::PausedByBoth`]: crate::PauseState::PausedByBoth
/// [`PauseState::PausedByModal`]: crate::PauseState::PausedByModal
/// [`PauseState::PausedBySpinner`]: crate::PauseState::PausedBySpinner
/// [`PauseState`]: crate::PauseState
/// [`Pin`]: std::pin::Pin
/// [`PinnedInputStream`]: crate::core::PinnedInputStream
/// [`ReadlineAsyncContext::acquire_modal_terminal()`]:
///     crate::ReadlineAsyncContext::acquire_modal_terminal
/// [`ReadlineAsyncContext::await_shutdown`]:
///     crate::readline_async::ReadlineAsyncContext::await_shutdown
/// [`ReadlineAsyncContext::pause()`]: crate::ReadlineAsyncContext::pause
/// [`ReadlineAsyncContext::read_line`]:
///     crate::readline_async::ReadlineAsyncContext::read_line
/// [`ReadlineAsyncContext::request_shutdown`]:
///     crate::readline_async::ReadlineAsyncContext::request_shutdown
/// [`ReadlineAsyncContext`]: crate::readline_async::ReadlineAsyncContext
/// [`SafeRawTerminal`]: crate::core::SafeRawTerminal
/// [`SharedWriter`]: crate::SharedWriter
/// [`Spinner`]: crate::readline_async::Spinner
/// [Core Async Concepts]: crate::main_event_loop_impl#core-async-concepts-pin-and-unpin
/// [dependency injection]: https://developerlife.com/category/DI/
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
#[allow(missing_debug_implementations)]
pub struct Readline {
    /// Manages hierarchical locking between line state and output device.
    pub(in crate::readline_async) lock_manager: ReadlineLockManager,

    /// Device used to get stream of events from user (usually `stdin`).
    pub(in crate::readline_async) input_device: InputDevice,

    /// Sender to the line state control channel (for signals like
    /// [`LineStateControlSignal::Flush`]).
    pub(in crate::readline_async) line_control_sender:
        Option<tokio::sync::mpsc::Sender<LineStateControlSignal>>,

    /// Use to send history updates.
    pub(in crate::readline_async) history_sender: UnboundedSender<String>,

    /// Use to receive history updates.
    pub(in crate::readline_async) history_receiver: UnboundedReceiver<String>,

    /// Manages the history.
    pub(in crate::readline_async) safe_history: SafeHistory,

    /// Collects lines that are written to the terminal while the terminal is paused.
    #[allow(dead_code)]
    pub(in crate::readline_async) safe_is_paused_buffer: SafePauseBuffer,

    /// Thread-safe tracker and shutdown mechanism for an active [`Spinner`].
    ///
    /// This field coordinates terminal exclusivity between the readline input loop,
    /// background loggers, and an active progress spinner. It prevents concurrent
    /// spinners and provides a mechanism to cancel an active spinner via `Ctrl+C` or
    /// `Ctrl+D`.
    ///
    /// - `Some(sender)`: A spinner is actively rendering. The `sender` can be used to
    ///   broadcast a shutdown signal to instantly kill the spinner task. This state
    ///   corresponds with the [`LineStateControlSignal::SpinnerActive`] and
    ///   [`LineStateControlSignal::Pause`] signals, as normal terminal output must be
    ///   buffered while the spinner renders.
    /// - `None`: No spinner is active. Normal terminal output and input editing proceed
    ///   as usual (corresponding to [`LineStateControlSignal::Resume`]).
    ///
    /// [`LineStateControlSignal::Pause`]: crate::LineStateControlSignal::Pause
    /// [`LineStateControlSignal::Resume`]: crate::LineStateControlSignal::Resume
    /// [`LineStateControlSignal::SpinnerActive`]:
    ///     crate::LineStateControlSignal::SpinnerActive
    /// [`Spinner`]: crate::Spinner
    pub(in crate::readline_async) safe_spinner_is_active:
        Arc<StdMutex<Option<broadcast::Sender<()>>>>,

    /// Shutdown channel.
    shutdown_complete_sender: broadcast::Sender<()>,
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

    /// The user pressed `Tab`.
    Tab,

    /// The user pressed `Shift+Tab` (`BackTab`).
    BackTab,

    /// The user pressed `Page Up`.
    PageUp,

    /// The user pressed `Page Down`.
    PageDown,

    /// The user pressed `Insert`.
    Insert,

    /// The user pressed a function key (`F1`–`F12`).
    ///
    /// The value is 1–12 (not 0–11), matching the key labels.
    FnKey(NonZeroU8),

    /// A key that readline doesn't handle internally.
    ///
    /// This allows consumers to handle application-specific keys without
    /// requiring changes to the readline library.
    UnhandledKey(KeyPress),

    /// The terminal was resized.
    Resized(VPSize),
}

/// This is an artificial delay amount that is added to hide the jank of displaying the
/// cursor to the terminal when the prompt is first printed, after the terminal is put
/// into raw mode.
pub const READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY: Duration =
    Duration::from_millis(66);

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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// - Receiver end of the channel, which does the actual writing to the terminal.
    /// - The sender end of the channel is in [`SharedWriter`].
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
                            Continuation::ReturnError(_) => {
                                // Signal that this task is exiting and break the loop.
                                // We don't care about the result of this operation.
                                shutdown_complete_sender.send(()).ok();
                                break;
                            }
                            Continuation::Continue => {
                                // Do nothing and continue the loop.
                            }
                            Continuation::Stop | Continuation::Restart => {
                                unreachable!(
                                    "process_line_control_signal never returns Stop or Restart"
                                )
                            }
                        }
                    }
                    _ => {
                        // Signal that this task is exiting and break the loop.
                        // We don't care about the result of this operation.
                        shutdown_complete_sender.send(()).ok();
                        break;
                    }
                }
            }
        })
    }

    /// Processes a line control signal. And actually write the line or buffered lines to
    /// the terminal.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    #[allow(clippy::needless_pass_by_value)]
    pub fn process_line_control_signal(
        line_control_signal: LineStateControlSignal,
        self_safe_is_paused_buffer: SafePauseBuffer,
        self_safe_line_state: SafeLineState,
        output_device: OutputDevice,
        self_safe_spinner_is_active: Arc<StdMutex<Option<broadcast::Sender<()>>>>,
    ) -> Continuation<ReadlineError> {
        match line_control_signal {
            // XMARK: Clever use of block, containing ? call, with .into() for err
            // conversion

            // Handle a line of text from user input w/ support for pause & resume.
            LineStateControlSignal::Line(buf) => {
                self_safe_line_state.write(|line_state| {
                    // Early return if paused. Push the line to pause_buffer, don't print
                    // it.
                    if line_state.pause_state != PauseState::NotPaused {
                        self_safe_is_paused_buffer.write(|pause_buffer| {
                            pause_buffer.push(buf);
                        });
                        return Continuation::Continue;
                    }

                    // Try to immediately print the incoming output and flush.
                    output_device
                        .write(|term| {
                            line_state.print_data_and_flush(buf.as_ref(), term)?;
                            term.flush()?;
                            ok!()
                        })
                        .into()
                })
            }

            // Pause the terminal.
            LineStateControlSignal::Pause => self_safe_line_state.write(|line_state| {
                if line_state.pause_state.pause_spinner() == PauseStateTransition::Paused
                {
                    output_device
                        .write(|term| {
                            line_state.clear_and_render_and_flush(term).map_err(
                                |_| {
                                    ReadlineError::IO(io::Error::other(
                                        "failed to pause terminal",
                                    ))
                                },
                            )?;
                            ok!()
                        })
                        .into()
                } else {
                    Continuation::Continue
                }
            }),

            // Resume the terminal.
            LineStateControlSignal::Resume => self_safe_line_state.write(|line_state| {
                if line_state.pause_state.resume_spinner()
                    == PauseStateTransition::Resumed
                {
                    output_device
                        .write(|term| {
                            // We don't care about the result of this operation.
                            drop(flush_internal(
                                &self_safe_is_paused_buffer,
                                line_state.pause_state,
                                line_state,
                                term,
                            ));

                            ok!()
                        })
                        .into()
                } else {
                    Continuation::Continue
                }
            }),

            LineStateControlSignal::ExitReadlineLoop => {
                // Signal the background task to stop and trigger a broadcast shutdown. By
                // returning an error to the caller, the caller will then send a signal to
                // the channel using `shutdown_complete_sender.send(())` and break the
                // loop.
                Err(ReadlineError::Closed).into()
            }

            // Handle a flush signal.
            LineStateControlSignal::Flush => self_safe_line_state.write(|line_state| {
                if line_state.pause_state != PauseState::NotPaused {
                    return Continuation::Continue;
                }
                output_device
                    .write(|term| {
                        // We don't care about the result of this operation.
                        drop(flush_internal(
                            &self_safe_is_paused_buffer,
                            line_state.pause_state,
                            line_state,
                            term,
                        ));
                        ok!()
                    })
                    .into()
            }),

            LineStateControlSignal::SpinnerActive(spinner_shutdown_sender) => {
                // Handle spinner active signal & register the spinner shutdown sender.
                self_safe_spinner_is_active.write(|spinner_is_active| {
                    *spinner_is_active = Some(spinner_shutdown_sender);
                });
                Continuation::Continue
            }

            LineStateControlSignal::SpinnerInactive => {
                // Handle spinner inactive signal & remove the spinner shutdown sender.
                self_safe_spinner_is_active.write(|spinner_is_active| {
                    drop(spinner_is_active.take());
                });
                Continuation::Continue
            }
        }
    }

    /// Flushes all writers to terminal and erase the prompt string.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the terminal fails.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.expect("conversion error") */
    pub fn flush_internal(
        self_safe_is_paused_buffer: &SafePauseBuffer,
        pause_state: PauseState,
        line_state: &mut LineState,
        term: &mut SendRawTerminal,
    ) -> CommonResultWithError<(), ReadlineError> {
        // If paused, then return!
        if pause_state != PauseState::NotPaused {
            return ok!();
        }

        let is_paused_buffer = {
            self_safe_is_paused_buffer.write(|paused_text_buffer_guard| {
                let paused_text_buffer: PauseBuffer =
                    paused_text_buffer_guard.drain(..).collect();
                join!(
                    from: paused_text_buffer,
                    each: text,
                    delim: "",
                    format: "{text}"
                )
            })
        };

        line_state.print_data_and_flush(is_paused_buffer.as_bytes(), term)?;
        line_state.clear_and_render_and_flush(term)?;

        ok!()
    }
}

impl Drop for Readline {
    /// Performs terminal-output related cleanup.
    ///
    /// # Poison Safety
    ///
    /// This implementation is designed to be **infallible and poison-safe** to prevent a
    /// [Double Panic Abort] (which would **brick the user's terminal**). It uses
    /// poison-safe locking to ensure that even if the input buffer or line state is
    /// corrupted, it can still attempt to restore the terminal to a usable state before
    /// the process exits. We prioritize **Resilience over Integrity** here.
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [Double Panic Abort]: crate#the-double-panic-abort-risk
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    fn drop(&mut self) { self.lock_manager.poison_safe_terminal_restore_on_drop(); }
}

impl Readline {
    #[cfg(test)]
    pub(crate) fn lock_manager_for_testing(&self) -> &ReadlineLockManager {
        &self.lock_manager
    }

    /// Checks if a spinner is currently active.
    #[must_use]
    pub fn is_spinner_active(&self) -> bool {
        self.safe_spinner_is_active.read(Option::is_some)
    }

    /// Creates a new instance with an associated [`SharedWriter`]. To customize the
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
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal operations fail.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.expect("conversion error") */
    #[allow(clippy::needless_pass_by_value)]
    pub fn try_new(
        prompt: String,
        output_device: OutputDevice,
        /* move */ input_device: InputDevice,
        /* move */ shutdown_complete_sender: broadcast::Sender<()>,
        channel_capacity: ChannelCapacity,
        size: VPSize,
    ) -> CommonResultWithError<(Self, SharedWriter), ReadlineError> {
        // Immediately hide the cursor. Then wait for
        // `READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY` to display the cursor
        // (try to eliminate jank). It makes it appear as if the cursor is animated into
        // place.
        output_device.write(|writer| {
            execute_commands_no_lock!(writer, cursor::Hide);
            execute_commands_no_lock!(writer, terminal::EnableLineWrap);
            Ok::<(), miette::Report>(())
        })?;

        // Enable raw mode (unless using a mock output device for testing). Drop will
        // disable raw mode.
        if output_device.paint_mode != PaintMode::Mock {
            crate::enable_raw_mode()?;
        }

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
        let line_state = LineState::new(prompt, size);
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

        let lock_manager =
            ReadlineLockManager::new(safe_line_state.clone(), output_device.clone());

        // Create the instance with all the supplied components.
        let readline = Readline {
            lock_manager,
            input_device,
            history_sender,
            history_receiver,
            safe_history,
            safe_is_paused_buffer,
            safe_spinner_is_active,
            shutdown_complete_sender,
            line_control_sender: Some(line_control_channel_sender.clone()),
        };

        // Print the prompt.
        readline
            .lock_manager
            .lock_both(|line_state, term| line_state.render_and_flush(term))?;

        spawn({
            let output_device_clone = output_device.clone();
            async move {
                // In a background task, wait for
                // `READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY` to
                // display the cursor (try to eliminate jank). This does not make
                // caller wait.
                sleep(READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY).await;
                output_device_clone.write(|term| {
                    // We don't care about the result of this operation.
                    drop(term.execute(cursor::Show));
                });
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
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// # Errors
    ///
    /// Returns an error if updating the prompt fails.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.expect("conversion error") */
    pub fn update_prompt(
        &mut self,
        prompt: &str,
    ) -> CommonResultWithError<(), ReadlineError> {
        self.lock_manager
            .lock_both(|line_state, term| line_state.update_prompt(prompt, term))?;
        ok!()
    }

    /// Clears the screen.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// # Errors
    ///
    /// Returns an error if clearing the screen fails.
    #[allow(clippy::unwrap_in_result)] /* This is for lock.expect("conversion error") */
    pub fn clear(&mut self) -> CommonResultWithError<(), ReadlineError> {
        self.lock_manager.lock_both(|line_state, term| {
            term.queue(Clear(terminal::ClearType::All))?;
            line_state.clear_and_render_and_flush(term)?;
            term.flush()?;
            Ok::<(), ReadlineError>(())
        })?;
        ok!()
    }

    /// Sets maximum history length. The default length is [`HISTORY_SIZE_MAX`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// [`HISTORY_SIZE_MAX`]: crate::readline_async::HISTORY_SIZE_MAX
    pub fn set_max_history(&mut self, max_size: usize) {
        self.safe_history.write(|history| {
            history.max_size = max_size;
            history.entries.truncate(max_size);
        });
    }

    /// Sets whether the input line should remain on the screen after events.
    ///
    /// # Arguments
    ///
    /// - `enter`:
    ///     - [`PrintLineOnEnter::Print`]: when the user presses `Enter`, the prompt and
    ///       the text they entered will remain on the screen, and the cursor will move to
    ///       the next line.
    ///     - [`PrintLineOnEnter::DoNotPrint`]: the prompt & input will be erased instead.
    ///     - The default value for `enter` is [`PrintLineOnEnter::Print`].
    ///
    /// - `control_c`:
    ///     - [`PrintLineOnControlC::Print`]: when the user presses `Ctrl+C`, the prompt
    ///       and the text will remain on the screen.
    ///     - [`PrintLineOnControlC::DoNotPrint`]: the prompt & input will be erased
    ///       instead.
    ///     - The default value for `control_c` is [`PrintLineOnControlC::DoNotPrint`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    pub fn should_print_line_on(
        &mut self,
        enter: PrintLineOnEnter,
        control_c: PrintLineOnControlC,
    ) {
        self.lock_manager.lock_line_state(|line_state| {
            line_state.print_line_on_enter = enter;
            line_state.print_line_on_control_c = control_c;
        });
    }

    /// This function returns when `Ctrl+D`, `Ctrl+C`, or `Enter` is pressed with some
    /// user input.
    ///
    /// Note that this function can be called repeatedly in a loop. It will return each
    /// line of input as it is entered (and return / `request_shutdown`). The
    /// [`ReadlineAsyncContext`] can be re-used, since the [`SharedWriter`] is cloned, and
    /// the terminal is kept in `raw mode` until the associated [`Readline`] is dropped.
    ///
    /// Polling function for [`Self::readline`], manages all input and output. Returns
    /// either an [`ReadlineEvent`] or an [`ReadlineError`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// # Errors
    ///
    /// Returns an error if reading input fails.
    ///
    /// [`ReadlineAsyncContext`]: crate::readline_async::ReadlineAsyncContext
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
                maybe_input_event = self.input_device.next() => {
                    if let Some(input_event) = maybe_input_event {
                        let result = self.lock_manager.lock_both(|line_state, term| {
                            readline_internal::apply_event_to_line_state_and_render(
                                input_event,
                                line_state,
                                term,
                                &self.safe_history,
                                &self.safe_spinner_is_active,
                            )
                        });
                        match result {
                            ReadlineControlFlow::ReturnOk(ok_value) => {
                                return Ok(ok_value);
                            }
                            ReadlineControlFlow::ReturnError(err_value) => {
                                return Err(err_value);
                            }
                            ReadlineControlFlow::Continue => {}
                        }
                    }
                },

                // Poll for history updates.
                // This branch is cancel safe because recv is cancel safe.
                maybe_line = self.history_receiver.recv() => {
                    self.safe_history.write(|history| {
                        history.update(maybe_line);
                    });
                },

                // Poll for shutdown signal.
                _ = shutdown_complete_receiver.recv() => {
                    return Err(ReadlineError::Closed);
                }
            }
        }
    }

    /// Adds a line to the input history.
    pub fn add_history_entry(&mut self, entry: String) -> Option<()> {
        self.history_sender.send(entry).ok()
    }

    /// Returns a clone of the current buffer content with grapheme metadata.
    ///
    /// The returned [`GCStringOwned`] includes pre-computed grapheme cluster
    /// information such as [`segment_count()`] and [`display_width`], useful for
    /// cursor positioning and completion UI rendering.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// [`display_width`]: GCStringOwned::display_width
    /// [`segment_count()`]: GCStringOwned::segment_count
    #[must_use]
    pub fn get_buffer(&self) -> GCStringOwned {
        self.lock_manager
            .lock_line_state(|line_state| line_state.line.clone())
    }

    /// Returns the cursor position as a type-safe grapheme segment index (0-based).
    ///
    /// The returned [`SegIndex`] can be used with [`ArrayBoundsCheck`] for safe
    /// comparisons against buffer length.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// [`ArrayBoundsCheck`]: crate::core::ArrayBoundsCheck
    #[must_use]
    pub fn get_cursor_position(&self) -> SegIndex {
        self.lock_manager
            .lock_line_state(|line_state| line_state.cursor_position)
    }

    /// Returns the cursor position status relative to the buffer content.
    ///
    /// This method uses the type-safe [`CursorBoundsCheck`] trait to determine
    /// where the cursor is positioned within the buffer.
    ///
    /// # Returns
    ///
    /// - [`AtStart`] - cursor at position 0 (beginning of buffer)
    /// - [`Within`] - cursor in middle of text
    /// - [`AtEnd`] - cursor at end of buffer (after last character)
    /// - [`Beyond`] - invalid state (should not occur with valid cursor)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use r3bl_tui::CursorPositionBoundsStatus;
    ///
    /// fn handle_tab_completion(status: CursorPositionBoundsStatus) {
    ///     match status {
    ///         CursorPositionBoundsStatus::AtStart => {
    ///             // Show full completion menu
    ///         }
    ///         CursorPositionBoundsStatus::AtEnd => {
    ///             // Show inline completion
    ///         }
    ///         CursorPositionBoundsStatus::Within => {
    ///             // Editing mid-line, maybe no completion
    ///         }
    ///         CursorPositionBoundsStatus::Beyond => unreachable!(),
    ///     }
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// [`AtEnd`]: CursorPositionBoundsStatus::AtEnd
    /// [`AtStart`]: CursorPositionBoundsStatus::AtStart
    /// [`Beyond`]: CursorPositionBoundsStatus::Beyond
    /// [`CursorBoundsCheck`]: CursorBoundsCheck
    /// [`Within`]: CursorPositionBoundsStatus::Within
    #[must_use]
    pub fn get_cursor_position_status(&self) -> CursorPositionBoundsStatus {
        self.lock_manager.lock_line_state(|line_state| {
            line_state
                .line
                .segment_count()
                .check_cursor_position_bounds(line_state.cursor_position)
        })
    }
}

pub mod readline_internal {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    pub fn apply_event_to_line_state_and_render(
        input_event: InputEvent,
        line_state: &mut LineState,
        term: &mut dyn Write,
        self_safe_history: &SafeHistory,
        self_safe_is_spinner_active: &Arc<StdMutex<Option<broadcast::Sender<()>>>>,
    ) -> ReadlineControlFlow<ReadlineEvent, ReadlineError> {
        // Check if this is Ctrl+C or Ctrl+D
        let is_ctrl_c_or_d = input_event.matches_any_of_these_keypresses(&[
            key_press!(@char ModifierKeysMask::new().with_ctrl(), 'c'),
            key_press!(@char ModifierKeysMask::new().with_ctrl(), 'd'),
        ]);

        // Intercept Ctrl+C or Ctrl+D here and send a signal to spinner (if it is
        // active). And early return!
        let is_spinner_active = self_safe_is_spinner_active.write(Option::take);

        if is_ctrl_c_or_d && let Some(spinner_shutdown_sender) = is_spinner_active {
            // Send signal to SharedWriter spinner shutdown channel.
            // We don't care about the result of this operation.
            spinner_shutdown_sender.send(()).ok();
            return ReadlineControlFlow::Continue;
        }

        // Regular readline event handling - use the canonical InputEvent directly
        line_state
            .apply_event_and_render(&input_event, term, self_safe_history)
            .into()
    }

    /// Converts crossterm `KeyCode` to canonical `Key`
    #[must_use]
    fn convert_key_code_to_key(code: crossterm::event::KeyCode) -> Option<Key> {
        use crate::{FunctionKey, Key, SpecialKey};
        use crossterm::event::KeyCode;

        match code {
            KeyCode::Char(c) => Some(Key::Character(c)),
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
                Some(Key::FunctionKey(fn_key))
            }
            KeyCode::Up => Some(Key::SpecialKey(SpecialKey::Up)),
            KeyCode::Down => Some(Key::SpecialKey(SpecialKey::Down)),
            KeyCode::Left => Some(Key::SpecialKey(SpecialKey::Left)),
            KeyCode::Right => Some(Key::SpecialKey(SpecialKey::Right)),
            KeyCode::Home => Some(Key::SpecialKey(SpecialKey::Home)),
            KeyCode::End => Some(Key::SpecialKey(SpecialKey::End)),
            KeyCode::PageUp => Some(Key::SpecialKey(SpecialKey::PageUp)),
            KeyCode::PageDown => Some(Key::SpecialKey(SpecialKey::PageDown)),
            KeyCode::Tab => Some(Key::SpecialKey(SpecialKey::Tab)),
            KeyCode::BackTab => Some(Key::SpecialKey(SpecialKey::BackTab)),
            KeyCode::Delete => Some(Key::SpecialKey(SpecialKey::Delete)),
            KeyCode::Insert => Some(Key::SpecialKey(SpecialKey::Insert)),
            KeyCode::Enter => Some(Key::SpecialKey(SpecialKey::Enter)),
            KeyCode::Backspace => Some(Key::SpecialKey(SpecialKey::Backspace)),
            KeyCode::Esc => Some(Key::SpecialKey(SpecialKey::Esc)),
            _ => None,
        }
    }

    /// Converts crossterm modifiers to canonical modifier mask
    #[must_use]
    fn convert_modifier_keys(
        modifiers: crossterm::event::KeyModifiers,
    ) -> ModifierKeysMask {
        use KeyState;
        use crossterm::event::KeyModifiers;

        ModifierKeysMask {
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
        }
    }

    /// Converts crossterm mouse button to canonical button
    #[must_use]
    fn convert_mouse_button(button: crossterm::event::MouseButton) -> Button {
        use Button;
        use crossterm::event::MouseButton;

        match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        }
    }

    /// Converts `crossterm::event::Event` to canonical `InputEvent`
    #[must_use]
    pub fn convert_crossterm_event_to_input_event(
        event: crossterm::event::Event,
    ) -> Option<InputEvent> {
        use crate::{KeyState, MouseInputKind};
        use crossterm::event::{Event, KeyEvent, MouseEvent, MouseEventKind};

        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                let key = convert_key_code_to_key(code)?;

                let mask = convert_modifier_keys(modifiers);
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
                let modifiers_mask = convert_modifier_keys(modifiers);
                let mouse_input = MouseInput {
                    pos: vp_col(column) + vp_row(row),
                    kind: match kind {
                        MouseEventKind::Down(btn) => {
                            MouseInputKind::MouseDown(convert_mouse_button(btn))
                        }
                        MouseEventKind::Up(btn) => {
                            MouseInputKind::MouseUp(convert_mouse_button(btn))
                        }
                        MouseEventKind::Drag(btn) => {
                            MouseInputKind::MouseDrag(convert_mouse_button(btn))
                        }
                        MouseEventKind::Moved => MouseInputKind::MouseMove,
                        MouseEventKind::ScrollUp => MouseInputKind::ScrollUp,
                        MouseEventKind::ScrollDown => MouseInputKind::ScrollDown,
                        MouseEventKind::ScrollLeft => MouseInputKind::ScrollLeft,
                        MouseEventKind::ScrollRight => MouseInputKind::ScrollRight,
                    },
                    maybe_modifier_keys: if modifiers_mask.shift_key_state
                        == KeyState::NotPressed
                        && modifiers_mask.ctrl_key_state == KeyState::NotPressed
                        && modifiers_mask.alt_key_state == KeyState::NotPressed
                    {
                        None
                    } else {
                        Some(modifiers_mask)
                    },
                };
                Some(InputEvent::Mouse(mouse_input))
            }
            Event::Resize(width, height) => Some(InputEvent::Resize(VPSize {
                col_width: VPWidth::from(width),
                row_height: VPHeight::from(height),
            })),
            _ => None,
        }
    }
}

/// Internal control flow for the [`readline()`] method. This is used primarily to make
/// testing easier.
///
/// # Result Conversion
///
/// This type supports implicit conversion from [`Result<Option<T>, E>`] via [`.into()`],
/// allowing for a fluid functional style when working with locks and loops.
///
/// # Usage Guidance
///
/// To maintain high readability and low cognitive load, follow these conventions:
/// 1. **Errors**: Prefer `ReturnError(E)` directly for early exits with errors.
/// 2. **Success**: Prefer `ReturnOk(T)` directly for successful completion.
/// 3. **Early Returns**: Use [`Self::Continue`] directly for early returns in a state
///    machine loop.
///
/// [`.into()`]: Into::into
/// [`readline()`]: Readline::readline
#[derive(Debug, PartialEq, Clone)]
pub enum ReadlineControlFlow<T, E> {
    ReturnOk(T),
    ReturnError(E),
    Continue,
}

impl<T, E> From<Result<Option<T>, E>> for ReadlineControlFlow<T, E> {
    fn from(result: Result<Option<T>, E>) -> ReadlineControlFlow<T, E> {
        match result {
            Ok(Some(val)) => Self::ReturnOk(val),
            Ok(None) => Self::Continue,
            Err(err) => Self::ReturnError(err),
        }
    }
}

/// Error returned from [`readline()`]. Such errors generally require specific procedures
/// to recover from.
///
/// # High-Fidelity Diagnostics
///
/// This type implements [`miette::Diagnostic`], which allows for high-fidelity error
/// reporting with help text and error codes. Use [`.into_diagnostic()`] to convert this
/// into a [`miette::Report`].
///
/// # Implicit Conversions
///
/// - **From Result**: Supports implicit conversion into the core [`Continuation`] type
///   via [`.into()`] (useful for loop control flow).
/// - **From Report**: Supports implicit conversion from [`miette::Report`] (via
///   [`From<ErrorReport>`]).
///
/// [`.into()`]: Into::into
/// [`.into_diagnostic()`]: miette::IntoDiagnostic::into_diagnostic
/// [`readline()`]: Readline::readline
#[derive(Debug, Error, miette::Diagnostic)]
pub enum ReadlineError {
    /// An internal I/O error occurred.
    #[error(transparent)]
    IO(#[from] io::Error),

    /// `readline()` was called after the [`SharedWriter`] was dropped and everything
    /// written to the `SharedWriter` was already output.
    #[error("line writers closed")]
    Closed,
}

/// For convenience, convert [`ErrorReport`] to [`ReadlineError`], so that
/// [`into_diagnostic()`] works.
///
/// [`into_diagnostic()`]: miette::IntoDiagnostic::into_diagnostic
impl From<ErrorReport> for ReadlineError {
    fn from(report: ErrorReport) -> ReadlineError {
        ReadlineError::IO(io::Error::other(format!("{report}")))
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
            let lhs = event.expect("conversion error");
            let rhs = get_input_vec()[count]
                .as_ref()
                .expect("conversion error")
                .clone();
            assert_eq!(lhs, rhs);
            count += 1;
        }
    }
}

#[cfg(test)]
mod test_pause_and_resume_support {
    use super::*;
    use crate::{core::test_fixtures::StdoutMock, vp_height, vp_width};
    use manage_shared_writer_output::flush_internal;

    #[test]
    fn test_flush_internal_paused() {
        // Create a mock `LineState` with initial data.
        let test_size = vp_width(100) + vp_height(100);
        let safe_line_state = Arc::new(crate::scoped_mutex!(
            SPECIFIC,
            LineState::new("> ".to_string(), test_size)
        ));

        // Create a mock `SafePauseBuffer` with some paused lines.
        let mut pause_buffer = PauseBuffer::new();
        pause_buffer.push("Paused line 1".into());
        pause_buffer.push("Paused line 2".into());

        // Create a mock `SafeIsPausedBuffer` with the pause buffer.
        let safe_is_paused_buffer = Arc::new(StdMutex::new(pause_buffer));

        let mut stdout_mock = StdoutMock::default();

        safe_line_state.write(|line_state| {
            // Call the `flush_internal` function.
            let result = flush_internal(
                &safe_is_paused_buffer,
                PauseState::PausedBySpinner,
                line_state,
                &mut stdout_mock,
            );

            // Assert that the function returns Ok(())
            assert!(result.is_ok());
        });

        // Assert that the mock terminal received the expected output.
        assert_eq!(stdout_mock.get_copy_of_buffer_as_string_strip_ansi(), "");
    }

    #[test]
    fn test_flush_internal_not_paused() {
        // Create a mock `LineState` with initial data.
        let test_size = vp_width(100) + vp_height(100);
        let safe_line_state = Arc::new(crate::scoped_mutex!(
            SPECIFIC,
            LineState::new("> ".to_string(), test_size)
        ));

        // Create a mock `SafePauseBuffer` with some paused lines.
        let mut pause_buffer = PauseBuffer::new();
        pause_buffer.push("Paused line 1".into());
        pause_buffer.push("Paused line 2".into());

        // Create a mock `SafeIsPausedBuffer` with the pause buffer.
        let safe_is_paused_buffer = Arc::new(StdMutex::new(pause_buffer));

        let mut stdout_mock = StdoutMock::default();

        safe_line_state.write(|line_state| {
            // Call the `flush_internal` function.
            let result = flush_internal(
                &safe_is_paused_buffer,
                PauseState::NotPaused,
                line_state,
                &mut stdout_mock,
            );

            // Assert that the function returns Ok(())
            assert!(result.is_ok());
        });

        // Assert that the mock terminal received the expected output.
        assert_eq!(
            stdout_mock.get_copy_of_buffer_as_string_strip_ansi(),
            "Paused line 1Paused line 2\n> > "
        );
    }
}

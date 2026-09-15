// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Async readline core struct and event loop implementation.

use super::{apply_event_to_line_state_and_render,
            spawn_task_to_monitor_line_control_channel};
use crate::{ChannelCapacity, CommonResultWithError, CursorBoundsCheck,
            CursorPositionBoundsStatus, GCStringOwned, History, InputDevice, LineState,
            LineStateControlSignal, OutputDevice, PaintMode, PauseBuffer,
            PrintLineOnControlC, PrintLineOnEnter,
            READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY,
            ReadlineControlFlow, ReadlineError, ReadlineEvent, ReadlineLockManager,
            SafeHistory, SafePauseBuffer, SegIndex, SharedWriter, StdMutex, VPSize,
            execute_commands_no_lock, ok};
use crossterm::{ExecutableCommand, QueueableCommand, cursor,
                terminal::{self, Clear}};
use std::sync::Arc;
use tokio::{select, spawn,
            sync::{broadcast, mpsc},
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
/// [`spawn_task_to_monitor_line_control_channel()`]. This task monitors the `line`
/// channel, and processes any messages that are sent to it. This allows the task to be
/// paused, and resumed, and to flush the output from the [`SharedWriter`]s.
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
/// 1. The [`flush_internal()`] method is called.
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
/// - By calling [`flush_internal()`].
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
/// [`flush_internal()`]: crate::flush_internal
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
        let history = History::new();
        let safe_history = Arc::new(StdMutex::new(history));

        // Line state.
        let line_state = LineState::new(prompt, size);
        let safe_line_state = Arc::new(StdMutex::new(line_state));

        // Pause buffer.
        let is_paused_buffer = PauseBuffer::new();
        let safe_is_paused_buffer = Arc::new(StdMutex::new(is_paused_buffer));

        // Start task to process line_receiver.
        let safe_spinner_is_active = Arc::new(StdMutex::new(None));
        spawn_task_to_monitor_line_control_channel(
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
                            apply_event_to_line_state_and_render(
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

                // Poll for shutdown signal.
                _ = shutdown_complete_receiver.recv() => {
                    return Err(ReadlineError::Closed);
                }
            }
        }
    }

    /// Adds a line to the input history.
    pub fn add_history_entry(&mut self, entry: String) -> Option<()> {
        self.safe_history.write(|history| {
            history.update(Some(entry));
        });
        Some(())
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

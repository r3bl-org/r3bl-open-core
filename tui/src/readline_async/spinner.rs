// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{InlineString, LineStateControlSignal, OutputDevice, SafeBool,
            SafeInlineString, SharedWriter, SpinnerStyle, StdMutex,
            TerminalInteractiveStatus, TuiAvailability, check_is_terminal_interactive,
            contains_ansi_escape_sequence, emit_stderr_redirection_disclaimer,
            get_terminal_width, ok, spinner_print, spinner_render};
use std::{sync::Arc, time::Duration};
use tokio::{sync::broadcast, time::interval};

/// [`Spinner`] is an [interactive terminal application entry point] that displays an
/// indeterminate spinner for long-running tasks. It only checks [`stdout`] interactivity
/// (not [`stdin`] or [`stderr`]), so it works with piped [`stdin`] or [`stderr`].
///
/// # Two modes
///
/// ## Standalone mode
///
/// Pass [`None`] for [`SharedWriter`] and use [`OutputDevice::default()`]. No
/// [`ReadlineAsyncContext`] needed. This is useful when you just need visual feedback
/// during a long operation (e.g., the upgrade check in `giti` or `edi` binaries in the
/// [`r3bl-cmdr`] crate). In this mode, the [`Spinner`] goes into and out of [raw mode] on
/// its own.
///
/// ## Embedded mode (with [`ReadlineAsyncContext`])
///
/// Pass a [`SharedWriter`] to coordinate output, so nothing gets clobbered. While the
/// spinner is active, async terminal output is paused. Ctrl+C and Ctrl+D cancellation is
/// supported when **both** the readline and spinner are active:
/// - The readline is active when [`read_line()`] is called.
/// - The spinner is active when [`Spinner::try_start()`] is called.
///
/// **Embedded mode internals**
///
/// This behavior is handled by [`ReadlineAsyncContext`], with coordination from
///
/// - In [`Self::try_start_task()`], the [`Spinner`] sends a [`LineStateControlSignal`]
///   containing a `shutdown_sender` of type [`tokio::sync::broadcast::Sender`<()>] to the
///   [`SharedWriter`] instance of the [`ReadlineAsyncContext`].
///   - This tells the [`ReadlineAsyncContext`] that a spinner is active.
///   - It also gives a way to stop the spinner via the `shutdown_sender`.
///
/// - When `Ctrl+C` or `Ctrl+D` is intercepted by [`ReadlineAsyncContext`] in
///   [`apply_event_to_line_state_and_render()`], a `()` is sent to
///   [`safe_spinner_is_active`], which shuts the spinner down.
///
/// # Usage Example
///
/// To properly stop a spinner and ensure it has completely shutdown:
///
/// ```no_run
/// // This example requires terminal output for the spinner animation
/// # use std::time::Duration;
/// # use r3bl_tui::{ok, SpinnerStyle, OutputDevice, Spinner, IntoErr, TuiAvailability};
/// # async fn example() -> miette::Result<()> {
///     let mut spinner = match Spinner::try_start(
///         "Loading...",
///         "Done!",
///         Duration::from_millis(100),
///         SpinnerStyle::default(),
///         OutputDevice::default(),
///         None,
///     )
///     .await {
///         TuiAvailability::Available(spinner) => spinner,
///         it => return it.into_err(),
///     };
///
///     // Some work happens here...
///
///     // Stop the spinner (sends the shutdown signal)
///     spinner.request_shutdown();
///     // Wait for the spinner to completely shutdown
///     spinner.await_shutdown().await;
/// # ok!()
/// # }
/// ```
///
/// [`apply_event_to_line_state_and_render()`]:
///     super::readline_internal::apply_event_to_line_state_and_render()
/// [`r3bl-cmdr`]: https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr
/// [`read_line()`]: crate::ReadlineAsyncContext::read_line()
/// [`ReadlineAsyncContext`]: crate::ReadlineAsyncContext
/// [`safe_spinner_is_active`]: crate::Readline::safe_spinner_is_active
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [interactive terminal application entry point]:
///     crate#interactive-terminal-application-entry-points
/// [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
#[allow(missing_debug_implementations)]
pub struct Spinner {
    pub tick_delay: Duration,
    /// [`ANSI`] escape sequences are stripped from this before being assigned.
    /// Thread-safe message that can be updated during spinner animation.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    pub interval_message: SafeInlineString,
    pub final_message: InlineString,
    pub style: SpinnerStyle,
    pub output_device: OutputDevice,
    pub maybe_shared_writer: Option<SharedWriter>,
    pub shutdown_sender: broadcast::Sender<()>,
    safe_is_shutdown: SafeBool,
    /// This is used to signal when the task has completely shutdown. Use the
    /// [`Self::wait_for_shutdown()`].
    maybe_shutdown_complete_rx: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl Spinner {
    /// Creates a new instance of [Spinner]. If the `arg_spinner_message` contains
    /// [`ANSI`] escape sequences then these will be stripped.
    ///
    /// # Returns
    /// 1. This will return an error if the task is already running.
    /// 2. If the terminal is not interactive then it will return
    ///    [`TuiAvailability::NotAvailable`], and won't start the task.
    /// 3. Otherwise, it will start the task and return a [`TuiAvailability::Available`]
    ///    containing the [`Spinner`] instance.
    ///
    /// More info on terminal piping:
    /// - <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
    ///
    /// # Note on [`stderr`] redirection
    ///
    /// This function calls [`emit_stderr_redirection_disclaimer()`] to ensure that if
    /// [`stderr`] is redirected, the user is notified that application logs are handled
    /// internally.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The spinner task cannot be started
    /// - The communication channels fail to initialize
    ///
    /// # Other entry points for interactive terminal apps
    ///
    /// See [interactive terminal application entry points].
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`emit_stderr_redirection_disclaimer()`]: crate::emit_stderr_redirection_disclaimer
    /// [`stderr`]: std::io::stderr
    /// [interactive terminal application entry points]: crate#interactive-terminal-application-entry-points
    pub async fn try_start(
        arg_interval_msg: impl AsRef<str>,
        arg_final_msg: impl AsRef<str>,
        tick_delay: Duration,
        style: SpinnerStyle,
        output_device: OutputDevice,
        maybe_shared_writer: Option<SharedWriter>,
    ) -> TuiAvailability<Spinner> {
        match check_is_terminal_interactive() {
            TerminalInteractiveStatus::NotAvailable(reason) => {
                TuiAvailability::NotAvailable(reason)
            }
            TerminalInteractiveStatus::Available => {
                let init = async || {
                    emit_stderr_redirection_disclaimer();

                    // Make sure no ANSI escape sequences are in the message.
                    let interval_msg = {
                        let msg = arg_interval_msg.as_ref();
                        if contains_ansi_escape_sequence(msg) {
                            strip_ansi_escapes::strip_str(msg)
                        } else {
                            msg.to_string()
                        }
                    };

                    // Make sure no ANSI escape sequences are in the final_message.
                    let final_msg = {
                        let msg = arg_final_msg.as_ref();
                        if contains_ansi_escape_sequence(msg) {
                            strip_ansi_escapes::strip_str(msg)
                        } else {
                            msg.to_string()
                        }
                    };

                    // Shutdown broadcast channel.
                    let (shutdown_sender, _) = broadcast::channel::<()>(1);

                    // Only start the task if the terminal is fully interactive.
                    let mut spinner = Spinner {
                        interval_message: Arc::new(StdMutex::new(interval_msg.into())),
                        final_message: final_msg.into(),
                        tick_delay,
                        style,
                        output_device,
                        maybe_shared_writer,
                        shutdown_sender,
                        safe_is_shutdown: Arc::new(StdMutex::new(false)),
                        maybe_shutdown_complete_rx: None,
                    };

                    // Start task.
                    spinner.try_start_task().await?;

                    Ok(spinner)
                };
                match init().await {
                    Ok(spinner) => TuiAvailability::Available(spinner),
                    Err(e) => TuiAvailability::Broken(e),
                }
            }
        }
    }

    /// This is meant for the task that spawned this [Spinner] to check if it should
    /// shutdown, due to:
    /// 1. The user pressing `Ctrl+C` or `Ctrl+D`.
    /// 2. Or the [`Spinner::request_shutdown`] got called.
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
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    #[must_use]
    pub fn is_shutdown(&self) -> bool { *self.safe_is_shutdown.lock().unwrap() }

    /// Starts and manages a task that will run in the background. This is where the
    /// spinner is started and the task is spawned. This will also pause the terminal
    /// output while the spinner is active. This will continue running until
    /// [`Self::request_shutdown()`] is called, which simply sends a message to the
    /// shutdown channel, so that this task can shut itself down.
    ///
    /// This method also sets up a [`tokio::sync::oneshot::channel`] to allow
    /// [`Self::await_shutdown()`] to know when the task has completely finished.
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
    /// Returns an error if:
    /// - The spinner task cannot be spawned
    /// - The communication channels fail
    ///
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    pub async fn try_start_task(&mut self) -> miette::Result<()> {
        // Tell readline that spinner is active & register the spinner shutdown sender.

        if let Some(shared_writer) = self.maybe_shared_writer.as_ref() {
            // We don't care about the result of this operation.
            shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::SpinnerActive(
                    self.shutdown_sender.clone(),
                ))
                .await
                .ok();

            // Pause the terminal.
            // We don't care about the result of this operation.
            shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Pause)
                .await
                .ok();
        }

        let mut shutdown_receiver = self.shutdown_sender.subscribe();

        let self_safe_is_shutdown = self.safe_is_shutdown.clone();

        // This does nothing if this is used in a `ReadlineAsyncContext` context.
        spinner_print::print_start_if_standalone(
            self.output_device.clone(),
            &self.maybe_shared_writer,
        )?;

        // Create a oneshot channel to signal when the task is complete.
        let (shutdown_complete_sender, shutdown_complete_receiver) =
            tokio::sync::oneshot::channel::<()>();
        self.maybe_shutdown_complete_rx = Some(shutdown_complete_receiver);

        tokio::spawn({
            let output_device_clone = self.output_device.clone();
            let interval_message_clone = self.interval_message.clone();
            let final_message_clone = self.final_message.clone();
            let maybe_shared_writer_clone = self.maybe_shared_writer.clone();
            let mut style_clone = self.style.clone();
            let tick_delay_clone = self.tick_delay;
            async move {
                let mut interval = interval(tick_delay_clone);

                // Count is used to determine the output.
                let mut count = 0;

                loop {
                    tokio::select! {
                        // Poll shutdown channel.
                        // This branch is cancel safe because recv is cancel safe.
                        _ = shutdown_receiver.recv() => {
                            // Cancel the interval.
                            drop(interval);

                            // Tell readline that spinner is inactive.
                            if let Some(shared_writer) = maybe_shared_writer_clone.as_ref() {
                                // We don't care about the result of this operation.
                                shared_writer
                                    .line_state_control_channel_sender
                                    .send(LineStateControlSignal::SpinnerInactive)
                                    .await.ok();
                            }

                            // Print the final message.
                            let final_output = spinner_render::render_final_tick(
                                &style_clone,
                                &final_message_clone,
                                get_terminal_width(),
                            );
                            // We don't care about the result of this operation.
                            spinner_print::print_tick_final_msg(
                                &style_clone,
                                &final_output,
                                output_device_clone.clone(),
                                &maybe_shared_writer_clone,
                            ).ok();

                            // Resume the terminal.
                            if let Some(shared_writer) = maybe_shared_writer_clone.as_ref() {
                                // We don't care about the result of this operation.
                                shared_writer
                                    .line_state_control_channel_sender
                                    .send(LineStateControlSignal::Resume)
                                    .await.ok();
                            }

                            // This spinner is now shutdown, so other task(s) using it will
                            // know that this spinner has been shutdown by user interaction or
                            // other means.
                            *self_safe_is_shutdown.lock().unwrap() = true;

                            // Signal that the task has completely shutdown. It's okay if this
                            // fails - it just means the receiver was dropped.
                            // We don't care about the result of this operation.
                            shutdown_complete_sender.send(()).ok();

                            break;
                        }

                        // Poll interval.
                        // This branch is cancel safe because tick is cancel safe.
                        // https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/#example-1-right-and-wrong-way-to-sleep-and-interval
                        _ = interval.tick() => {
                            // Early return if the spinner is shutdown.
                            if *self_safe_is_shutdown.lock().unwrap() {
                                break;
                            }

                            // Render and print the interval message, based on style.
                            let current_message = interval_message_clone.lock().unwrap().clone();
                            let output = spinner_render::render_tick(
                                &mut style_clone,
                                &current_message,
                                count,
                                get_terminal_width(),
                            );
                            // We don't care about the result of this operation.
                            spinner_print::print_tick_interval_msg(
                                &style_clone,
                                &output,
                                output_device_clone.clone()
                            ).ok();

                            // Increment count to affect the output in the next iteration of this loop.
                            count += 1;
                        },
                    }
                }
            }
        });

        ok!()
    }

    /// Shuts down the task started by [`Self::try_start_task()`]. This method only sends
    /// the shutdown signal and returns immediately without waiting for the spinner
    /// task to completely shutdown. To wait for the task to actually finish shutting
    /// down, call [`Self::await_shutdown()`] after this method.
    pub fn request_shutdown(&mut self) {
        // We don't care about the result of this operation.
        self.shutdown_sender.send(()).ok();
    }

    /// Waits for the spinner task to completely shutdown. This can be used after calling
    /// [`Self::request_shutdown()`] to ensure the task has fully completed. This consumes
    /// self, and ensures this instance is dropped after the task has completed and
    /// can't be used again.
    pub async fn await_shutdown(mut self) {
        if let Some(receiver) = self.maybe_shutdown_complete_rx.take() {
            // Wait for the task to signal completion. Ignore the error if the sender is
            // dropped without sending (rare case).
            // We don't care about the result of this operation.
            receiver.await.ok();
        }
    }

    /// Updates the interval message that's displayed during spinner animation. This can
    /// be called from another task/thread to update progress.
    ///
    /// [`ANSI`] escape sequences are stripped from the message if present.
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
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    pub fn update_message(&self, new_message: impl Into<InlineString>) {
        let msg = new_message.into();
        // Strip ANSI codes if present.
        let clean_msg = if contains_ansi_escape_sequence(&msg) {
            strip_ansi_escapes::strip_str(&msg).into()
        } else {
            msg
        };
        *self.interval_message.lock().unwrap() = clean_msg;
    }
}

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ChannelCapacity, CommonResult, CursorPositionBoundsStatus, GCStringOwned,
            InputDevice, LineStateControlSignal, OutputDevice,
            READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY, Readline,
            ReadlineEvent, SegIndex, SharedWriter, TerminalInteractiveStatus,
            TuiAvailability, check_is_terminal_interactive,
            emit_stderr_redirection_disclaimer, get_size, inline_string, ok};
use futures_util::FutureExt;
use miette::IntoDiagnostic;
use tokio::sync::broadcast;

/// This is the context for the readline async API. It contains the [Readline] instance,
/// the shared writer, and the shutdown completion channel.
///
/// The mental model for this is that you create a readline async context and then use it
/// to read lines from the terminal. You can re-use the `Readline` to read as many lines
/// as you want. The `SharedWriter` is used to write to the terminal. This context can be
/// paused and resumed.
///
/// When you are done with the context, you should call
/// [`ReadlineAsyncContext::request_shutdown()`] to request a shutdown. This will cause
/// the readline loop to exit and the context to be dropped. You should also call
/// [`ReadlineAsyncContext::await_shutdown()`] to wait for the shutdown to complete. This
/// is important because there is a lot of machinery that needs to be cleaned up and
/// shutdown. This is done in a non-blocking way, so you can continue to use the context
/// until the shutdown is complete.
///
/// Finally, another benefit of having a non-blocking readline, is that if you call
/// [`ReadlineAsyncContext::request_shutdown()`] it will exit a readline loop that might
/// currently be running!
///
/// # Example
///
/// Here's an example of how to use this method:
/// ```no_run
/// // This example requires an interactive terminal for user input
/// # async fn foo() -> miette::Result<()> {
///     # use r3bl_tui::readline_async::ReadlineAsyncContext;
///     # use r3bl_tui::ChannelCapacity;
///     # use r3bl_tui::TuiAvailability;
///     # use r3bl_tui::IntoErr;
///     # use r3bl_tui::ok;
///     let mut rl_ctx = match ReadlineAsyncContext::try_new(
///         Some("> "),
///         Some(ChannelCapacity::VeryLarge),
///     ).await {
///         TuiAvailability::Available(rl_ctx) => rl_ctx,
///         it => return it.into_err(),
///     };
///
///     let ReadlineAsyncContext { readline: ref mut rl, .. } = rl_ctx;
///     let user_input = rl.readline().await;
///     rl_ctx.request_shutdown(Some("Shutting down...")).await?;
///     rl_ctx.await_shutdown().await;
///     ok!()
/// # }
/// ```
#[allow(missing_debug_implementations)]
pub struct ReadlineAsyncContext {
    pub readline: Readline,
    pub shared_writer: SharedWriter,
    /// Shutdown completion channel. Requesting a shutdown is a complex process, and this
    /// channel is used to signal when that process has completed.
    pub shutdown_complete_sender: broadcast::Sender<()>,
}

/// Don't change the `content`. Print it as is. And it is compatible w/ the
/// [`ReadlineAsyncContext::read_line`] method.
#[macro_export]
macro_rules! rla_println {
    (
        $rla:ident,
        $($format:tt)*
    ) => {{
        use std::io::Write;
        // We don't care about the result of this operation.
        writeln!($rla.shared_writer, $($format)*).ok();
    }};
}

#[macro_export]
macro_rules! rla_print {
    (
        $rla:ident,
        $($format:tt)*
    ) => {{
        use std::io::Write;
        // We don't care about the result of this operation.
        write!($rla.shared_writer, $($format)*).ok();
    }};
}

/// Prefix the `content` with a color and special characters, then print it.
#[macro_export]
macro_rules! rla_println_prefixed {
    (
        $rla:ident,
        $($format:tt)*
    ) => {{
        use std::io::Write;
        use $crate::fg_pink;
        // We don't care about the result of this operation.
        write!($rla.shared_writer, "{}", fg_pink(" > ").bold().bg_moonlight_blue()).ok();
        // We don't care about the result of this operation.
        writeln!($rla.shared_writer, $($format)*).ok();
    }};
}

impl ReadlineAsyncContext {
    /// Creates a new instance of [`ReadlineAsyncContext`]. Example of `prompt` is `"> "`.
    /// It is safe to have [`ANSI`] escape sequences inside the `prompt` as this is taken
    /// into account when calculating the width of the terminal when displaying it in the
    /// "line editor".
    ///
    /// # Arguments
    ///
    /// - `read_line_prompt`: Optional prompt string (defaults to `"> "`).
    /// - `channel_capacity`: Optional channel capacity (defaults to
    ///   [`ChannelCapacity::VeryLarge`]). Choose based on expected burst traffic - see
    ///   [`ChannelCapacity`] documentation for detailed guidance.
    ///
    /// # Returns
    ///
    /// Returns a [`TuiAvailability`] containing the [`ReadlineAsyncContext`] if the
    /// terminal is interactive.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The terminal cannot be put into [raw mode]
    /// - The readline instance cannot be created
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
    /// # Other entry points for interactive terminal apps
    ///
    /// See [interactive terminal application entry points].
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`check_is_terminal_interactive()`]: crate::check_is_terminal_interactive
    /// [`emit_stderr_redirection_disclaimer()`]:
    ///     crate::emit_stderr_redirection_disclaimer
    /// [`stderr`]: std::io::stderr
    /// [interactive terminal application entry points]: crate#interactive-terminal-application-entry-points
    /// [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
    pub async fn try_new(
        read_line_prompt: Option<impl AsRef<str>>,
        channel_capacity: Option<ChannelCapacity>,
    ) -> TuiAvailability<ReadlineAsyncContext> {
        match check_is_terminal_interactive() {
            TerminalInteractiveStatus::NotAvailable(reason) => {
                TuiAvailability::NotAvailable(reason)
            }

            TerminalInteractiveStatus::Available => {
                let init = async || {
                    emit_stderr_redirection_disclaimer();

                    let initial_size = get_size()?;
                    let output_device = OutputDevice::new_stdout();
                    let input_device = InputDevice::default();

                    let prompt = match read_line_prompt {
                        Some(ref p) => p.as_ref().to_string(),
                        None => "> ".to_owned(),
                    };

                    // Use the provided channel capacity or default to VeryLarge.
                    let capacity = channel_capacity.unwrap_or_default();

                    // Create a channel to signal when shutdown is complete.
                    let shutdown_complete_channel = broadcast::channel::<()>(1);
                    let (shutdown_complete_sender, _) = shutdown_complete_channel;

                    let (readline, stdout) = Readline::try_new(
                        prompt.clone(),
                        output_device,
                        input_device,
                        shutdown_complete_sender.clone(),
                        capacity,
                        initial_size,
                    )
                    .into_diagnostic()?;

                    // Sleep for READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY.
                    tokio::time::sleep(
                        READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY,
                    )
                    .await;

                    Ok(ReadlineAsyncContext {
                        readline,
                        shared_writer: stdout,
                        shutdown_complete_sender,
                    })
                };
                match init().await {
                    Ok(ctx) => TuiAvailability::Available(ctx),
                    Err(e) => TuiAvailability::Broken(e),
                }
            }
        }
    }

    #[must_use]
    pub fn clone_shared_writer(&self) -> SharedWriter { self.shared_writer.clone() }

    pub fn mut_input_device(&mut self) -> &mut InputDevice {
        &mut self.readline.input_device
    }

    pub fn clone_output_device(&mut self) -> OutputDevice {
        self.readline.output_device.clone()
    }

    /// Replacement for [`std::io::Stdin::read_line()`] (this is async and non-blocking).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The readline operation fails due to I/O errors
    /// - The terminal has been closed or disconnected
    /// - The readline loop has been shut down
    pub async fn read_line(&mut self) -> miette::Result<ReadlineEvent> {
        self.readline.readline().fuse().await.into_diagnostic()
    }

    /// Simply flush the buffer. If there's a newline in the buffer, it will be printed.
    /// Otherwise, it won't.
    pub async fn flush(&mut self) {
        // We don't care about the result of this operation.
        self.shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Flush)
            .await
            .ok();
    }

    pub async fn pause(&mut self) {
        // We don't care about the result of this operation.
        self.shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await
            .ok();
    }

    pub async fn resume(&mut self) {
        // We don't care about the result of this operation.
        self.shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await
            .ok();
    }

    /// Returns a clone of the current buffer content with grapheme metadata.
    #[must_use]
    pub fn get_buffer(&self) -> GCStringOwned { self.readline.get_buffer() }

    /// Returns the cursor position as a type-safe grapheme segment index (0-based).
    #[must_use]
    pub fn get_cursor_position(&self) -> SegIndex { self.readline.get_cursor_position() }

    /// Returns the cursor position status relative to the buffer content.
    #[must_use]
    pub fn get_cursor_position_status(&self) -> CursorPositionBoundsStatus {
        self.readline.get_cursor_position_status()
    }

    /// Make sure to call this method when you are done with the [`ReadlineAsyncContext`]
    /// instance. It will flush the buffer and print the message if provided. This
    /// also consumes the [`ReadlineAsyncContext`] instance, so it can't be used after
    /// this method is called.
    ///
    /// This method performs an important task - it exits the readline loop gracefully.
    /// Here are the details of how it does this:
    ///
    /// 1. it sends a [`LineStateControlSignal::ExitReadlineLoop`] signal to the [Readline]
    ///    instance's
    ///    [`crate::readline_async_impl::manage_shared_writer_output::spawn_task_to_monitor_line_control_channel`]
    ///    task (aka "actor"). This makes the task shut itself down,
    /// 2. which then causes a message to be sent to the
    ///    [`ReadlineAsyncContext::shutdown_complete_sender`],
    /// 3. which also causes the [`Readline::readline()`] method to shutdown (if it is
    ///    currently running). It doesn't block, since it is `readline_async` after all.
    ///    This is a very powerful feature that is not available in synchronous blocking
    ///    `readline`.
    ///
    /// If you don't call this method, when the underlying [Readline] instance is dropped,
    /// it's [Drop] implementation will perform terminal-output related cleanup, but it
    /// won't print any `request_shutdown` message or stop the readline loop.
    ///
    /// Make sure to call [`Self::await_shutdown()`], to ensure that the
    /// mechanism is cleanly shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The shutdown signal cannot be sent to the readline loop
    /// - The final message cannot be written to the terminal
    pub async fn request_shutdown(&self, message: Option<&str>) -> CommonResult {
        // Process the request_shutdown message (if some).
        if let Some(message) = message {
            // Prefix the message with `\r`.
            let message = inline_string!("\r{message}");

            self.shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Line(message))
                .await
                .map_err(std::io::Error::other)
                .into_diagnostic()?;

            self.shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Flush)
                .await
                .map_err(std::io::Error::other)
                .into_diagnostic()?;
        }

        // Send the `LineStateControlSignal::ExitReadlineLoop` signal, which will cause
        // the `spawn_task_to_monitor_line_control_channel()` to **initiate**
        // shutdown. Shutdown might take some time to complete, which is why
        // `await_shutdown().await` should be called to wait for this shutdown
        // process to complete.
        self.shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::ExitReadlineLoop)
            .await
            .map_err(std::io::Error::other)
            .into_diagnostic()?;

        ok!()
    }

    /// Waits for the tasks to completely shutdown. This can be used after calling
    /// [`Self::request_shutdown()`] to ensure the task has fully completed. This consumes
    /// self, and ensures this instance is dropped after the task has completed and
    /// can't be used again.
    pub async fn await_shutdown(self) {
        let mut shutdown_complete_receiver = self.shutdown_complete_sender.subscribe();
        // We don't care about the result of this operation.
        shutdown_complete_receiver.recv().await.ok();
    }
}

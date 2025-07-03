/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use futures_util::FutureExt as _;
use miette::IntoDiagnostic as _;
use tokio::sync::broadcast;

use crate::{inline_string,
            is_fully_uninteractive_terminal,
            is_stdin_piped,
            is_stdout_piped,
            ok,
            CommonResult,
            InputDevice,
            LineStateControlSignal,
            OutputDevice,
            Readline,
            ReadlineEvent,
            SharedWriter,
            StdinIsPipedResult,
            StdoutIsPipedResult,
            TTYResult,
            READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY};

/// This is the context for the readline async API. It contains the
/// [Readline] instance, the shared writer, and the shutdown completion
/// channel.
///
/// The mental model for this is that you create a readline async context
/// and then use it to read lines from the terminal. You can re-use the
/// `Readline` to read as many lines as you want. The `SharedWriter` is used to
/// write to the terminal. This context can be paused and resumed.
///
/// When you are done with the context, you should call
/// [`ReadlineAsyncContext::request_shutdown()`] to request a shutdown. This will
/// cause the readline loop to exit and the context to be dropped. You should
/// also call [`ReadlineAsyncContext::await_shutdown()`] to wait for the shutdown
/// to complete. This is important because there is a lot of machinery that needs
/// to be cleaned up and shutdown. This is done in a non-blocking way, so you
/// can continue to use the context until the shutdown is complete.
///
/// Finally, another benefit of having a non-blocking readline, is that if you
/// call [`ReadlineAsyncContext::request_shutdown()`] it will exit a readline loop
/// that might currently be running!
///
/// # Example
///
/// Here's an example of how to use this method:
/// ```
/// # async fn foo() -> miette::Result<()> {
///     # use r3bl_tui::readline_async::ReadlineAsyncContext;
///     # use r3bl_tui::ok;
///     let Some(mut rl_ctx) = ReadlineAsyncContext::try_new(Some("> ")).await?
///     else {
///         return Err(miette::miette!("Failed to create terminal"));
///     };
///     let ReadlineAsyncContext { readline: ref mut rl, .. } = rl_ctx;
///     let user_input = rl.readline().await;
///     rl_ctx.request_shutdown(Some("Shutting down...")).await;
///     rl_ctx.await_shutdown().await;
///     ok!()
/// # }
/// ```
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
        use std::io::Write as _;
        _ = writeln!($rla.shared_writer, $($format)*);
    }};
}

#[macro_export]
macro_rules! rla_print {
    (
        $rla:ident,
        $($format:tt)*
    ) => {{
        use std::io::Write as _;
        _ = write!($rla.shared_writer, $($format)*);
    }};
}

/// Prefix the `content` with a color and special characters, then print it.
#[macro_export]
macro_rules! rla_println_prefixed {
    (
        $rla:ident,
        $($format:tt)*
    ) => {{
        use std::io::Write as _;
        use $crate::fg_pink;
        _ = write!($rla.shared_writer, "{}", fg_pink(" > ").bold().bg_moonlight_blue());
        _ = writeln!($rla.shared_writer, $($format)*);
    }};
}

impl ReadlineAsyncContext {
    /// Create a new instance of [`ReadlineAsyncContext`]. Example of `prompt` is `"> "`.
    /// It is safe to have ANSI escape sequences inside the `prompt` as this is taken
    /// into account when calculating the width of the terminal when displaying it in
    /// the "line editor".
    ///
    /// # Returns
    /// 1. If the terminal is not fully interactive, then it will return [None], and won't
    ///    create the [Readline]. This is when the terminal is not considered fully
    ///    interactive:
    ///    - `stdout` is piped, e.g., `echo "foo" | cargo run --example spinner`.
    ///    - or all three `stdin`, `stdout`, `stderr` are not `is_tty`, e.g., when running
    ///      in `cargo test`.
    /// 2. Otherwise, it will return a [`ReadlineAsyncContext`] instance.
    /// 3. If any issues arise when putting the terminal into raw mode, or getting the
    ///    terminal size, it will return an error.
    ///
    /// More info on terminal piping:
    /// - <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
    pub async fn try_new(
        read_line_prompt: Option<impl AsRef<str>>,
    ) -> miette::Result<Option<ReadlineAsyncContext>> {
        if let StdinIsPipedResult::StdinIsPiped = is_stdin_piped() {
            return Ok(None);
        }
        if let StdoutIsPipedResult::StdoutIsPiped = is_stdout_piped() {
            return Ok(None);
        }
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return Ok(None);
        }

        let output_device = OutputDevice::new_stdout();
        let input_device = InputDevice::new_event_stream();

        let prompt =
            read_line_prompt.map_or_else(|| "> ".to_owned(), |p| p.as_ref().to_string());

        // Create a channel to signal when shutdown is complete.
        let shutdown_complete_channel = broadcast::channel::<()>(1);
        let (shutdown_complete_sender, _) = shutdown_complete_channel;

        let (readline, stdout) = Readline::try_new(
            prompt.clone(),
            output_device,
            input_device,
            shutdown_complete_sender.clone(),
        )
        .await
        .into_diagnostic()?;

        // Sleep for READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY.
        tokio::time::sleep(READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY).await;

        Ok(Some(ReadlineAsyncContext {
            readline,
            shared_writer: stdout,
            shutdown_complete_sender,
        }))
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
    pub async fn read_line(&mut self) -> miette::Result<ReadlineEvent> {
        self.readline.readline().fuse().await.into_diagnostic()
    }

    /// Simply flush the buffer. If there's a newline in the buffer, it will be printed.
    /// Otherwise, it won't.
    pub async fn flush(&mut self) {
        drop(
            self.shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Flush)
                .await,
        );
    }

    pub async fn pause(&mut self) {
        drop(
            self.shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Pause)
                .await,
        );
    }

    pub async fn resume(&mut self) {
        drop(
            self.shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Resume)
                .await,
        );
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
    pub async fn request_shutdown(&self, message: Option<&str>) -> CommonResult<()> {
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
        _ = shutdown_complete_receiver.recv().await;
    }
}

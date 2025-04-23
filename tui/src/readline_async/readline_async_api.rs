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

use crate::{is_fully_uninteractive_terminal,
            is_stdin_piped,
            is_stdout_piped,
            InputDevice,
            LineStateControlSignal,
            OutputDevice,
            Readline,
            ReadlineEvent,
            SharedWriter,
            StdinIsPipedResult,
            StdoutIsPipedResult,
            TTYResult};

pub struct ReadlineAsync {
    pub readline: Readline,
    pub shared_writer: SharedWriter,
}

/// Don't change the `content`. Print it as is. And it is compatible w/ the
/// [ReadlineAsync::read_line] method.
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

impl ReadlineAsync {
    /// Create a new instance of [ReadlineAsync]. Example of `prompt` is `"> "`.
    ///
    /// # Example
    ///
    /// Here's an example of how to use this method:
    /// ```rust
    /// async fn foo() -> miette::Result<()> {
    ///     # use r3bl_tui::readline_async::ReadlineAsync;
    ///     # use r3bl_tui::ok;
    ///     let readline_async = ReadlineAsync::try_new(None::<String>)?
    ///         .ok_or_else(|| miette::miette!("Failed to create terminal"))?;
    ///     ok!()
    /// }
    /// ```
    ///
    /// Another example:
    /// ```rust
    /// async fn foo() -> miette::Result<()> {
    ///     # use r3bl_tui::readline_async::ReadlineAsync;
    ///     # use r3bl_tui::ok;
    ///     let Some(mut readline_async) = ReadlineAsync::try_new(Some("> "))? else {
    ///         return Err(miette::miette!("Failed to create terminal"));
    ///     };
    ///     ok!()
    /// }
    /// ```
    ///
    /// # Returns
    /// 1. If the terminal is not fully interactive, then it will return [None], and won't
    ///    create the [Readline]. This is when the terminal is not considered fully
    ///    interactive:
    ///    - `stdout` is piped, e.g., `echo "foo" | cargo run --example spinner`.
    ///    - or all three `stdin`, `stdout`, `stderr` are not `is_tty`, e.g., when running in
    ///      `cargo test`.
    /// 2. Otherwise, it will return a [ReadlineAsync] instance.
    /// 3. If any issues arise when putting the terminal into raw mode, or getting the
    ///    terminal size, it will return an error.
    ///
    /// More info on terminal piping:
    /// - <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
    pub fn try_new(
        read_line_prompt: Option<impl AsRef<str>>,
    ) -> miette::Result<Option<ReadlineAsync>> {
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

        let prompt = read_line_prompt
            .map(|p| p.as_ref().to_string())
            .unwrap_or_else(|| "> ".to_owned());

        let (readline, stdout) =
            Readline::new(prompt.to_owned(), output_device, input_device)
                .into_diagnostic()?;

        Ok(Some(ReadlineAsync {
            readline,
            shared_writer: stdout,
        }))
    }

    pub fn clone_shared_writer(&self) -> SharedWriter { self.shared_writer.clone() }

    pub fn mut_input_device(&mut self) -> &mut InputDevice {
        &mut self.readline.input_device
    }

    pub fn clone_output_device(&mut self) -> OutputDevice {
        self.readline.output_device.clone()
    }

    /// Replacement for [std::io::Stdin::read_line()] (this is async and non-blocking).
    pub async fn read_line(&mut self) -> miette::Result<ReadlineEvent> {
        self.readline.readline().fuse().await.into_diagnostic()
    }

    /// Simply flush the buffer. If there's a newline in the buffer, it will be printed.
    /// Otherwise, it won't.
    pub async fn flush(&mut self) {
        let _ = self
            .shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Flush)
            .await;
    }

    pub async fn pause(&mut self) {
        let _ = self
            .shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await;
    }

    pub async fn resume(&mut self) {
        let _ = self
            .shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await;
    }

    /// Make sure to call this method when you are done with the [ReadlineAsync] instance.
    /// It will flush the buffer and print the message if provided. This also consumes the
    /// [ReadlineAsync] instance, so it can't be used after this method is called.
    ///
    /// This method performs an important task - it exits the readline loop gracefully.
    /// Here are the details of how it does this:
    ///
    /// 1. it sends a [LineStateControlSignal::ExitReadlineLoop] signal to the [Readline]
    ///    instance's
    ///    [crate::readline_async_impl::manage_shared_writer_output::spawn_task_to_monitor_line_state_signals]
    ///    task (aka "actor"),
    /// 2. which causes a message to be sent to the [Readline::shutdown_receiver],
    /// 3. which causes the [Readline::readline()] method to exit (if it is
    ///    currently running). It doesn't block, since it is `readline_async` after all.
    ///    This is a very powerful feature that is not available in synchronous
    ///    blocking `readline`.
    ///
    /// If you don't call this method, when the underlying [Readline] instance is dropped,
    /// it's [Drop] implementation will perform (most of the) cleanup, but it won't print
    /// any exit message or stop the readline loop.
    pub async fn exit(self, message: Option<&str>) -> std::io::Result<()> {
        if let Some(message) = message {
            self.shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Line(message.into()))
                .await
                .map_err(std::io::Error::other)?;

            self.shared_writer
                .line_state_control_channel_sender
                .send(LineStateControlSignal::Flush)
                .await
                .map_err(std::io::Error::other)?;
        }

        self.shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::ExitReadlineLoop)
            .await
            .map_err(std::io::Error::other)?;

        // Pause to allow all the messages to be printed.
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        drop(self);

        Ok(())
    }
}

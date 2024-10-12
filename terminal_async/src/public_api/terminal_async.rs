/*
 *   Copyright (c) 2024 R3BL LLC
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

use std::io::{stdout, Write};

use crossterm::{cursor::MoveToColumn,
                style::{Print, ResetColor, Stylize},
                terminal::{Clear, ClearType}};
use futures_util::FutureExt as _;
use miette::IntoDiagnostic as _;
use r3bl_ansi_color::{is_fully_uninteractive_terminal,
                      is_stdin_piped,
                      is_stdout_piped,
                      StdinIsPipedResult,
                      StdoutIsPipedResult,
                      TTYResult};
use r3bl_core::{InputDevice, LineStateControlSignal, OutputDevice, SharedWriter};

use crate::{Readline, ReadlineEvent};

pub struct TerminalAsync {
    pub readline: Readline,
    pub shared_writer: SharedWriter,
}

impl TerminalAsync {
    /// Create a new instance of [TerminalAsync]. Example of `prompt` is `"> "`.
    ///
    /// # Example
    ///
    /// Here's an example of how to use this method:
    ///
    /// ```
    /// async fn foo() -> miette::Result<()> {
    ///     use r3bl_terminal_async::TerminalAsync;
    ///     let terminal_async = TerminalAsync::try_new("> ")
    ///         .await?
    ///         .ok_or_else(|| miette::miette!("Failed to create terminal"))?;
    ///     r3bl_core::ok!()
    /// }
    /// ```
    ///
    /// Another example:
    ///
    /// ```
    /// async fn foo() -> miette::Result<()> {
    ///     use r3bl_terminal_async::TerminalAsync;
    ///     let Some(mut terminal_async) = TerminalAsync::try_new("> ").await? else {
    ///         return Err(miette::miette!("Failed to create terminal"));
    ///     };
    ///     r3bl_core::ok!()
    /// }
    /// ```
    ///
    /// # Returns
    /// 1. If the terminal is not fully interactive then it will return [None], and won't
    ///    create the [Readline]. This is when the terminal is not considered fully
    ///    interactive:
    ///    - `stdout` is piped, eg: `echo "foo" | cargo run --example spinner`.
    ///    - or all three `stdin`, `stdout`, `stderr` are not `is_tty`, eg when running in
    ///      `cargo test`.
    /// 2. Otherwise, it will return a [TerminalAsync] instance.
    /// 3. In case there are any issues putting the terminal into raw mode, or getting the
    ///    terminal size, it will return an error.
    ///
    /// More info on terminal piping:
    /// - <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
    pub async fn try_new(prompt: &str) -> miette::Result<Option<TerminalAsync>> {
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

        let (readline, stdout) =
            Readline::new(prompt.to_owned(), output_device, input_device)
                .into_diagnostic()?;

        Ok(Some(TerminalAsync {
            readline,
            shared_writer: stdout,
        }))
    }

    pub fn clone_shared_writer(&self) -> SharedWriter { self.shared_writer.clone() }

    /// Replacement for [std::io::Stdin::read_line()] (this is async and non blocking).
    pub async fn get_readline_event(&mut self) -> miette::Result<ReadlineEvent> {
        self.readline.readline().fuse().await.into_diagnostic()
    }

    /// Don't change the `content`. Print it as is. This works concurrently and is async
    /// and non blocking. And it is compatible w/ the
    /// [get_readline_event](TerminalAsync::get_readline_event) method.
    pub async fn println<T>(&mut self, content: T)
    where
        T: std::fmt::Display,
    {
        let _ = writeln!(self.shared_writer, "{}", content);
    }

    /// Prefix the `content` with a color and special characters, then print it.
    pub async fn println_prefixed<T>(&mut self, content: T)
    where
        T: std::fmt::Display,
    {
        let _ = writeln!(
            self.shared_writer,
            "{} {}",
            " > ".red().bold().on_dark_grey(),
            content
        );
    }

    /// Simply flush the buffer. If there's a newline in the buffer, it will be printed.
    /// Otherwise it won't.
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

    pub fn print_exit_message(message: &str) -> miette::Result<()> {
        crossterm::queue!(
            stdout(),
            MoveToColumn(0),
            ResetColor,
            Clear(ClearType::CurrentLine),
            Print(message),
            Print("\n"),
        )
        .into_diagnostic()?;
        Ok(())
    }
}

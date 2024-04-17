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

use crate::{
    History, LineState, PauseBuffer, PinnedInputStream, SafeBool, SafeHistory, SafeLineState,
    SafePauseBuffer, SafeRawTerminal, SharedWriter, Text, TokioMutex, CHANNEL_CAPACITY,
};
use crossterm::{
    event::Event,
    terminal::{self, disable_raw_mode, Clear},
    QueueableCommand,
};
use futures_util::StreamExt;
use std::{
    io::{self, Error, Write},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender};

/// ### Mental model and overview
///
/// This is a replacement for a [std::io::BufRead::read_line] function. It is async. It
/// supports other tasks concurrently writing to the terminal output (via
/// [SharedWriter]s). It also supports being paused so that [crate::Spinner] can display
/// an indeterminate progress spinner. Then it can be resumed so that the user can type in
/// the terminal. Upon resumption, any queued output from the [SharedWriter]s is printed
/// out.
///
/// When you call [Self::readline()] it enters an infinite loop. During which you can type
/// things into the multiline editor, which also displays the prompt. You can press up,
/// down, left, right, etc. While in this loop other tasks can send messages to the
/// `Readline` task via the `line` channel, using the [`SharedWriter::line_sender`].
///
/// When you create a new [`Readline`] instance, a task,
/// [`Readline::monitor_line_receiver_task_join_handle`]], is started via
/// [`pause_and_resume_support::spawn_task_to_monitor_line_receiver()`]. This task
/// monitors the `line` channel, and processes any messages that are sent to it. This
/// allows the task to be paused, and resumed, and to flush the output from the
/// [`SharedWriter`]s. When you [`Readline::close()`] the instance or drop it, this task
/// is aborted.
///
/// ### Inputs and dependency injection
///
/// There are 2 main resources that must be passed into [`Self::new()`]:
/// 1. [`PinnedInputStream`] - This trait represents an async stream of events. It is
///    typically implemented by
///    [`crossterm::event::EventStream`](https://docs.rs/crossterm/latest/crossterm/event/struct.EventStream.html).
///    This is used to get input from the user. However for testing you can provide your
///    own implementation of this trait.
/// 2. [`SafeRawTerminal`] - This trait represents a raw terminal. It is typically
///    implemented by [`std::io::Stdout`]. This is used to write to the terminal. However
///    for testing you can provide your own implementation of this trait.
///
/// ### Support for testing
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
/// ### Pause and resume
///
/// When the terminal is paused, then any output from the [`SharedWriter`]s will not be
/// printed to the terminal. This is useful when you want to display a spinner, or some
/// other indeterminate progress indicator.
///
/// When the terminal is resumed, then the output from the [`SharedWriter`]s will be
/// printed to the terminal by the [`pause_and_resume_support::flush_internal()`] method,
/// which drains a buffer that holds any output that was generated while paused, of type
/// [`PauseBuffer`].
///
/// This is possible, because while paused, the
/// [`pause_and_resume_support::poll_for_shared_writer_output()`] method doesn't actually
/// print anything to the display. When resumed, the
/// [`pause_and_resume_support::flush_internal()`] method is called, which drains the
/// [`PauseBuffer`] (if there are any messages in it, and prints them out) so nothing is
/// lost!
///
/// ### Usage details
///
/// Struct for reading lines of input from a terminal while lines are output to the
/// terminal concurrently.
///
/// Terminal input is retrieved by calling [`Readline::readline()`], which returns each
/// complete line of input once the user presses Enter.
///
/// Each `Readline` instance is associated with one or more [`SharedWriter`] instances.
///
/// Lines written to an associated `SharedWriter` are output:
/// 1. While retrieving input with [`readline()`][Readline::readline].
/// 2. By calling [`pause_and_resume_support::flush_internal()`].
///
/// You can provide your own implementation of [SafeRawTerminal], via [dependency
/// injection](https://developerlife.com/category/DI/), so that you can mock terminal
/// output for testing. You can also extend this struct to adapt your own terminal output
/// using this mechanism. Essentially anything that complies with `dyn std::io::Write +
/// Send` trait bounds can be used.
pub struct Readline {
    /// Raw terminal implementation, you can supply this via dependency injection.
    pub safe_raw_terminal: SafeRawTerminal,

    /// Stream of events.
    pub pinned_input_stream: PinnedInputStream,

    /// Current line.
    pub safe_line_state: SafeLineState,

    /// Use to send history updates.
    pub history_sender: UnboundedSender<String>,
    /// Use to receive history updates.
    pub history_receiver: UnboundedReceiver<String>,
    /// Manages the history.
    pub safe_history: SafeHistory,

    /// Determines whether terminal is paused or not. When paused, concurrent output
    /// via [`SharedWriter`]s is not printed to the terminal.
    pub safe_is_paused: SafeBool,

    /// Collects lines that are written to the terminal while the terminal is paused.
    pub safe_is_paused_buffer: SafePauseBuffer,

    /// Shutdown broadcast channel that is used to stop both:
    /// 1. [`pause_and_resume_support::spawn_task_to_monitor_line_channel`].
    /// 2. [`Readline::readline`] if it is currently running.
    /// 3. Also see: [`Readline::close`].
    pub shutdown_sender: tokio::sync::broadcast::Sender<bool>,
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

/// Events emitted by [`Readline::readline()`].
#[derive(Debug, PartialEq, Clone)]
pub enum ReadlineEvent {
    /// The user entered a line of text.
    Line(String),

    /// The user pressed Ctrl-D.
    Eof,

    /// The user pressed Ctrl-C.
    Interrupted,
}

/// Signals that can be sent to the `line` channel, which is monitored by the
/// [`Readline::monitor_line_receiver_task_join_handle`] task.
#[derive(Debug, PartialEq, Clone)]
pub enum LineControlSignal {
    Line(Text),
    Flush,
    Pause,
    Resume,
}

/// Internal control flow for the `readline` method. This is used primarily to make testing
/// easier.
#[derive(Debug, PartialEq, Clone)]
pub enum InternalControlFlow<T, E> {
    ReturnOk(T),
    ReturnError(E),
    Continue,
}

pub mod pause_and_resume_support {
    use super::*;

    /// Receiver end of the channel, the sender end is in [`SharedWriter`], which does the
    /// actual writing to the terminal.
    pub fn spawn_task_to_monitor_line_channel(
        shutdown_sender: tokio::sync::broadcast::Sender<bool>,
        /* move */ mut line_receiver: Receiver<LineControlSignal>,
        safe_is_paused: SafeBool,
        safe_line_state: SafeLineState,
        safe_raw_terminal: SafeRawTerminal,
        safe_is_paused_buffer: SafePauseBuffer,
    ) -> tokio::task::JoinHandle<()> {
        let mut shutdown_receiver = shutdown_sender.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Poll line channel for events.
                    maybe_line_control_signal = line_receiver.recv() => {
                        let control_flow = poll_for_shared_writer_output(
                            maybe_line_control_signal,
                            safe_is_paused_buffer.clone(),
                            safe_line_state.clone(),
                            safe_raw_terminal.clone(),
                            safe_is_paused.clone(),
                        )
                        .await;

                        match control_flow {
                            InternalControlFlow::ReturnError(_) => {
                                line_receiver.close();
                                break;
                            }
                            InternalControlFlow::Continue => {
                                // continue.
                            }
                            _ => {
                                unreachable!();
                            }
                        }
                    }

                    // Poll shutdown channel.
                    _ = shutdown_receiver.recv() => {
                        break;
                    }
                }
            }
        })
    }

    /// Flush all writers to terminal and erase the prompt string.
    pub async fn flush_internal(
        self_safe_is_paused_buffer: SafePauseBuffer,
        safe_is_paused: SafeBool,
        safe_line_state: SafeLineState,
        safe_raw_terminal: SafeRawTerminal,
    ) -> Result<(), ReadlineError> {
        // If paused, then return!
        if *safe_is_paused.lock().await {
            return Ok(());
        }

        let is_paused_buffer = &mut *self_safe_is_paused_buffer.lock().await;

        while let Some(buf) = is_paused_buffer.pop_front() {
            safe_line_state
                .lock()
                .await
                .print_data(&buf, &mut *safe_raw_terminal.lock().await)?;
        }

        safe_line_state
            .lock()
            .await
            .clear_and_render(&mut *safe_raw_terminal.lock().await)?;
        safe_raw_terminal.lock().await.flush()?;

        Ok(())
    }

    /// Returns only the following:
    /// - [InternalControlFlow::Continue]
    /// - [InternalControlFlow::ReturnError]
    pub async fn poll_for_shared_writer_output(
        maybe_line_control_signal: Option<LineControlSignal>,
        self_safe_is_paused_buffer: SafePauseBuffer,
        self_safe_line_state: SafeLineState,
        self_safe_raw_terminal: SafeRawTerminal,
        self_safe_is_paused: SafeBool,
    ) -> InternalControlFlow<(), ReadlineError> {
        match maybe_line_control_signal {
            Some(line_control_signal) => match line_control_signal {
                LineControlSignal::Line(buf) => {
                    // If paused, then return!
                    if *self_safe_is_paused.lock().await {
                        let pause_buffer = &mut *self_safe_is_paused_buffer.lock().await;
                        pause_buffer.push_back(buf);
                        return InternalControlFlow::Continue;
                    }

                    if let Err(err) = self_safe_line_state
                        .lock()
                        .await
                        .print_data(&buf, &mut *self_safe_raw_terminal.lock().await)
                    {
                        return InternalControlFlow::ReturnError(err);
                    }
                    if let Err(err) = self_safe_raw_terminal.lock().await.flush() {
                        return InternalControlFlow::ReturnError(err.into());
                    }
                }

                LineControlSignal::Flush => {
                    let _ = flush_internal(
                        self_safe_is_paused_buffer,
                        self_safe_is_paused,
                        self_safe_line_state,
                        self_safe_raw_terminal,
                    )
                    .await;
                }

                LineControlSignal::Pause => {
                    *self_safe_is_paused.lock().await = true;
                }

                LineControlSignal::Resume => {
                    *self_safe_is_paused.lock().await = false;
                    let _ = flush_internal(
                        self_safe_is_paused_buffer,
                        self_safe_is_paused,
                        self_safe_line_state,
                        self_safe_raw_terminal,
                    )
                    .await;
                }
            },
            None => {
                return InternalControlFlow::ReturnError(ReadlineError::Closed);
            }
        }

        InternalControlFlow::Continue
    }
}

impl Readline {
    /// Create a new instance with an associated [`SharedWriter`]. To customize the
    /// behavior of this instance, you can use the following methods:
    /// - [Self::should_print_line_on]
    /// - [Self::set_max_history]
    pub async fn new(
        prompt: String,
        safe_raw_terminal: SafeRawTerminal,
        /* move */ pinned_input_stream: PinnedInputStream,
    ) -> Result<(Self, SharedWriter), ReadlineError> {
        // Line channel.
        let line_channel = tokio::sync::mpsc::channel::<LineControlSignal>(CHANNEL_CAPACITY);
        let (line_sender, line_receiver) = line_channel;

        // Shutdown channel.
        let shutdown_channel = tokio::sync::broadcast::channel::<bool>(1);
        let (shutdown_sender, _) = shutdown_channel;

        // Paused state.
        let safe_is_paused = Arc::new(TokioMutex::new(false));

        // Enable raw mode. Drop will disable raw mode.
        terminal::enable_raw_mode()?;

        // History setup.
        let (history, history_receiver) = History::new();
        let history_sender = history.sender.clone();
        let safe_history = Arc::new(TokioMutex::new(history));

        // Line state.
        let line_state = LineState::new(prompt, terminal::size()?);
        let safe_line_state = Arc::new(TokioMutex::new(line_state));

        // Pause buffer.
        let is_paused_buffer = PauseBuffer::new();
        let safe_is_paused_buffer = Arc::new(TokioMutex::new(is_paused_buffer));

        // Start task to process line_receiver.
        pause_and_resume_support::spawn_task_to_monitor_line_channel(
            shutdown_sender.clone(),
            line_receiver,
            safe_is_paused.clone(),
            safe_line_state.clone(),
            safe_raw_terminal.clone(),
            safe_is_paused_buffer.clone(),
        );

        // Create the instance with all the supplied components.
        let readline = Readline {
            safe_raw_terminal: safe_raw_terminal.clone(),
            pinned_input_stream,
            safe_line_state: safe_line_state.clone(),
            history_sender,
            safe_is_paused: safe_is_paused.clone(),
            history_receiver,
            safe_history,
            shutdown_sender,
            safe_is_paused_buffer,
        };

        // Print the prompt.
        readline
            .safe_line_state
            .lock()
            .await
            .render(&mut *readline.safe_raw_terminal.lock().await)?;
        readline
            .safe_raw_terminal
            .lock()
            .await
            .queue(terminal::EnableLineWrap)?;
        readline.safe_raw_terminal.lock().await.flush()?;

        // Create the shared writer.
        let shared_writer = SharedWriter {
            line_sender,
            buffer: Vec::new(),
        };

        // Return the instance and the shared writer.
        Ok((readline, shared_writer))
    }

    /// Change the prompt.
    pub async fn update_prompt(&mut self, prompt: &str) -> Result<(), ReadlineError> {
        self.safe_line_state
            .lock()
            .await
            .update_prompt(prompt, &mut *self.safe_raw_terminal.lock().await)?;
        Ok(())
    }

    /// Clear the screen.
    pub async fn clear(&mut self) -> Result<(), ReadlineError> {
        self.safe_raw_terminal
            .lock()
            .await
            .queue(Clear(terminal::ClearType::All))?;
        self.safe_line_state
            .lock()
            .await
            .clear_and_render(&mut *self.safe_raw_terminal.lock().await)?;
        self.safe_raw_terminal.lock().await.flush()?;
        Ok(())
    }

    /// Set maximum history length. The default length is [crate::HISTORY_SIZE_MAX].
    pub async fn set_max_history(&mut self, max_size: usize) {
        let mut history = self.safe_history.lock().await;
        history.max_size = max_size;
        history.entries.truncate(max_size);
    }

    /// Set whether the input line should remain on the screen after events.
    ///
    /// If `enter` is true, then when the user presses "Enter", the prompt and the text
    /// they entered will remain on the screen, and the cursor will move to the next line.
    /// If `enter` is false, the prompt & input will be erased instead.
    ///
    /// `control_c` similarly controls the behavior for when the user presses Ctrl-C.
    ///
    /// The default value for both settings is `true`.
    pub async fn should_print_line_on(&mut self, enter: bool, control_c: bool) {
        let mut line_state = self.safe_line_state.lock().await;
        line_state.should_print_line_on_enter = enter;
        line_state.should_print_line_on_control_c = control_c;
    }

    /// Polling function for `readline`, manages all input and output. Returns either an
    /// [ReadlineEvent] or an [ReadlineError].
    pub async fn readline(&mut self) -> miette::Result<ReadlineEvent, ReadlineError> {
        let mut shutdown_receiver = self.shutdown_sender.subscribe();
        loop {
            tokio::select! {
                // Poll for events.
                maybe_result_crossterm_event = self.pinned_input_stream.next() => {
                    match readline_internal::process_event(
                        maybe_result_crossterm_event,
                        self.safe_line_state.clone(),
                        &mut *self.safe_raw_terminal.lock().await,
                        self.safe_history.clone()
                    ).await {
                        InternalControlFlow::ReturnOk(ok_value) => {return Ok(ok_value);},
                        InternalControlFlow::ReturnError(err_value) => {return Err(err_value);},
                        InternalControlFlow::Continue => {}
                    }
                },

                // Poll for history updates.
                maybe_line = self.history_receiver.recv() => {
                    self.safe_history.lock().await.update(maybe_line).await;
                }

                // Poll shutdown channel.
                _ = shutdown_receiver.recv() => {
                    break Err(ReadlineError::Closed);
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
    use super::*;

    pub async fn process_event(
        maybe_result_crossterm_event: Option<Result<Event, Error>>,
        self_line_state: SafeLineState,
        self_raw_terminal: &mut dyn Write,
        self_safe_history: SafeHistory,
    ) -> InternalControlFlow<ReadlineEvent, ReadlineError> {
        if let Some(result_crossterm_event) = maybe_result_crossterm_event {
            match result_crossterm_event {
                Ok(crossterm_event) => {
                    let mut it = self_line_state.lock().await;
                    let result_maybe_readline_event = it
                        .handle_event(crossterm_event, self_raw_terminal, self_safe_history)
                        .await;
                    match result_maybe_readline_event {
                        Ok(maybe_readline_event) => {
                            if let Err(e) = self_raw_terminal.flush() {
                                return InternalControlFlow::ReturnError(e.into());
                            }
                            if let Some(readline_event) = maybe_readline_event {
                                return InternalControlFlow::ReturnOk(readline_event);
                            }
                        }
                        Err(e) => return InternalControlFlow::ReturnError(e),
                    }
                }
                Err(e) => return InternalControlFlow::ReturnError(e.into()),
            }
        }
        InternalControlFlow::Continue
    }
}

/// Exit raw mode when the instance is dropped.
impl Drop for Readline {
    /// There is no need to explicitly call [Readline::close()] if the instance is
    /// dropped, since it will close the shutdown channel and the task
    /// 1. [`pause_and_resume_support::spawn_task_to_monitor_line_channel`].
    /// 2. [`Readline::readline`] if it is currently running.
    /// 3. See also: [`Readline::close`].
    fn drop(&mut self) {
        let _ = self.shutdown_sender.send(true);
        let _ = disable_raw_mode();
    }
}

impl Readline {
    /// Call this to shutdown:
    /// 1. [`pause_and_resume_support::spawn_task_to_monitor_line_channel()`].
    /// 2. [`Readline::readline()`] only if it is currently running.
    ///
    /// Typically this happens when your CLI wants to exit, due to some user input
    /// requesting this. This will result in any awaiting tasks in various places to error
    /// out, which is the desired behavior, rather than just hanging, waiting on events
    /// that will never happen.
    pub async fn close(&mut self) {
        let _ = self.shutdown_sender.send(true);
    }
}

#[cfg(test)]
pub mod test_fixtures {
    use crate::StdMutex;

    use super::*;
    use async_stream::stream;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    pub(super) fn gen_input_stream() -> PinnedInputStream {
        let it = stream! {
            for event in get_input_vec() {
                yield Ok(event);
            }
        };
        Box::pin(it)
    }

    pub(super) fn get_input_vec() -> Vec<Event> {
        vec![
            // a
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            // b
            Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)),
            // c
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)),
            // enter
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ]
    }

    #[derive(Clone)]
    pub struct StdoutMock {
        pub buffer: Arc<StdMutex<Vec<u8>>>,
    }

    impl Write for StdoutMock {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::StdMutex;

    use super::*;
    use r3bl_tuify::{is_fully_uninteractive_terminal, TTYResult};
    use strip_ansi_escapes::strip;
    use tests::test_fixtures::{gen_input_stream, get_input_vec, StdoutMock};

    #[tokio::test]
    async fn test_readline_internal_process_event_and_terminal_output() {
        let vec = get_input_vec();
        let mut iter = vec.iter();

        let prompt_str = "> ";

        let output_buffer = Vec::new();
        let stdout_mock = StdoutMock {
            buffer: Arc::new(StdMutex::new(output_buffer)),
        };

        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (readline, _) = Readline::new(
            prompt_str.into(),
            Arc::new(TokioMutex::new(stdout_mock.clone())),
            gen_input_stream(),
        )
        .await
        .unwrap();

        let history = History::new();
        let safe_history = Arc::new(TokioMutex::new(history.0));

        // Simulate 'a'.
        let event = iter.next().unwrap();
        let control_flow = readline_internal::process_event(
            Some(Ok(event.clone())),
            readline.safe_line_state.clone(),
            &mut *readline.safe_raw_terminal.lock().await,
            safe_history.clone(),
        )
        .await;

        assert!(matches!(control_flow, InternalControlFlow::Continue));
        assert_eq!(readline.safe_line_state.lock().await.line, "a");

        let output_buffer_data = stdout_mock.buffer.lock().unwrap();
        let output_buffer_data = strip(output_buffer_data.to_vec());
        let output_buffer_data = String::from_utf8(output_buffer_data).expect("utf8");
        // println!("\n`{}`\n", output_buffer_data);
        assert!(output_buffer_data.contains("> a"));
    }

    #[tokio::test]
    async fn test_readline() {
        let prompt_str = "> ";

        let output_buffer = Vec::new();
        let stdout_mock = StdoutMock {
            buffer: Arc::new(StdMutex::new(output_buffer)),
        };

        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (mut readline, _) = Readline::new(
            prompt_str.into(),
            Arc::new(TokioMutex::new(stdout_mock.clone())),
            gen_input_stream(),
        )
        .await
        .unwrap();

        let result = readline.readline().await;
        assert!(matches!(result, Ok(ReadlineEvent::Line(_))));
        pretty_assertions::assert_eq!(result.unwrap(), ReadlineEvent::Line("abc".to_string()));
        pretty_assertions::assert_eq!(readline.safe_line_state.lock().await.line, "");

        let output_buffer_data = stdout_mock.buffer.lock().unwrap();
        let output_buffer_data = strip(output_buffer_data.to_vec());
        let output_buffer_data = String::from_utf8(output_buffer_data).expect("utf8");
        // println!("\n`{}`\n", output_buffer_data);
        assert!(output_buffer_data.contains("> abc"));
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let prompt_str = "> ";

        let output_buffer = Vec::new();
        let stdout_mock = StdoutMock {
            buffer: Arc::new(StdMutex::new(output_buffer)),
        };

        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (readline, shared_writer) = Readline::new(
            prompt_str.into(),
            Arc::new(TokioMutex::new(stdout_mock.clone())),
            gen_input_stream(),
        )
        .await
        .unwrap();

        shared_writer
            .line_sender
            .send(LineControlSignal::Pause)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        assert!(*readline.safe_is_paused.lock().await);

        shared_writer
            .line_sender
            .send(LineControlSignal::Resume)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        assert!(!(*readline.safe_is_paused.lock().await));
    }

    #[tokio::test]
    async fn test_pause_resume_with_output() {
        let prompt_str = "> ";

        let output_buffer = Vec::new();
        let stdout_mock = StdoutMock {
            buffer: Arc::new(StdMutex::new(output_buffer)),
        };

        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }

        // We will get the `line_state` out of this to test.
        let (readline, shared_writer) = Readline::new(
            prompt_str.into(),
            Arc::new(TokioMutex::new(stdout_mock.clone())),
            gen_input_stream(),
        )
        .await
        .unwrap();

        shared_writer
            .line_sender
            .send(LineControlSignal::Pause)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        assert!(*readline.safe_is_paused.lock().await);

        shared_writer
            .line_sender
            .send(LineControlSignal::Line("abc".into()))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        let pause_buffer = readline.safe_is_paused_buffer.lock().await;
        assert_eq!(pause_buffer.len(), 1);
        assert_eq!(String::from_utf8_lossy(&pause_buffer[0]), "abc".to_string());

        shared_writer
            .line_sender
            .send(LineControlSignal::Resume)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        assert!(!(*readline.safe_is_paused.lock().await));
    }
}

#[cfg(test)]
mod test_streams {
    use super::*;
    use test_streams::test_fixtures::{gen_input_stream, get_input_vec};

    #[tokio::test]
    async fn test_generate_event_stream_pinned() {
        let mut count = 0;
        let mut it = gen_input_stream();
        while let Some(event) = it.next().await {
            let lhs = event.unwrap();
            let rhs = get_input_vec()[count].clone();
            assert_eq!(lhs, rhs);
            count += 1;
        }
    }
}

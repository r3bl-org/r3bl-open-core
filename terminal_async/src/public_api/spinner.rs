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

use std::{sync::Arc, time::Duration};

use r3bl_ansi_color::{StdoutIsPipedResult,
                      TTYResult,
                      is_fully_uninteractive_terminal,
                      is_stdout_piped};
use r3bl_core::{LineStateControlSignal, SharedWriter, get_terminal_width};
use r3bl_tui::{SpinnerStyle, spinner_render};
use tokio::time::interval;

use crate::{SafeBool, SafeRawTerminal, StdMutex};

pub struct Spinner {
    pub tick_delay: Duration,
    pub message: String,
    pub style: SpinnerStyle,
    pub safe_output_terminal: SafeRawTerminal,
    pub shared_writer: SharedWriter,
    pub shutdown_sender: tokio::sync::broadcast::Sender<()>,
    safe_is_shutdown: SafeBool,
}

impl Spinner {
    /// Create a new instance of [Spinner].
    ///
    /// # Returns
    /// 1. This will return an error if the task is already running.
    /// 2. If the terminal is not fully interactive then it will return [None], and won't
    ///    start the task. This is when the terminal is not considered fully interactive:
    ///    - `stdout` is piped, eg: `echo "foo" | cargo run --example spinner`.
    ///    - or all three `stdin`, `stdout`, `stderr` are not `is_tty`, eg when running in
    ///      `cargo test`.
    /// 3. Otherwise, it will start the task and return a [Spinner] instance.
    ///
    /// More info on terminal piping:
    /// - <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
    pub async fn try_start(
        spinner_message: String,
        tick_delay: Duration,
        style: SpinnerStyle,
        safe_output_terminal: SafeRawTerminal,
        shared_writer: SharedWriter,
    ) -> miette::Result<Option<Spinner>> {
        if let StdoutIsPipedResult::StdoutIsPiped = is_stdout_piped() {
            return Ok(None);
        }
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return Ok(None);
        }

        // Shutdown broadcast channel.
        let (shutdown_sender, _) = tokio::sync::broadcast::channel::<()>(1);

        // Only start the task if the terminal is fully interactive.
        let mut spinner = Spinner {
            message: spinner_message,
            tick_delay,
            style,
            safe_output_terminal,
            shared_writer,
            shutdown_sender,
            safe_is_shutdown: Arc::new(StdMutex::new(false)),
        };

        // Start task.
        spinner.try_start_task().await?;

        Ok(Some(spinner))
    }

    /// This is meant for the task that spawned this [Spinner] to check if it should
    /// shutdown, due to:
    /// 1. The user pressing `Ctrl-C` or `Ctrl-D`.
    /// 2. Or the [Spinner::stop] got called.
    pub fn is_shutdown(&self) -> bool { *self.safe_is_shutdown.lock().unwrap() }

    async fn try_start_task(&mut self) -> miette::Result<()> {
        // Tell readline that spinner is active & register the spinner shutdown sender.
        _ = self
            .shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::SpinnerActive(
                self.shutdown_sender.clone(),
            ))
            .await;

        // Pause the terminal.
        let _ = self
            .shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await;

        let message = self.message.clone();
        let tick_delay = self.tick_delay;
        let mut style = self.style.clone();
        let safe_output_terminal = self.safe_output_terminal.clone();

        let mut shutdown_receiver = self.shutdown_sender.subscribe();

        let self_safe_is_shutdown = self.safe_is_shutdown.clone();

        tokio::spawn(async move {
            let mut interval = interval(tick_delay);

            // Count is used to determine the output.
            let mut count = 0;
            let message_clone = message.clone();

            loop {
                tokio::select! {
                    // Poll interval.
                    // This branch is cancel safe because tick is cancel safe.
                    _ = interval.tick() => {
                        // Render and paint the output, based on style.
                        let output = spinner_render::render_tick(
                            &mut style,
                            &message_clone,
                            count,
                            get_terminal_width(),
                        );
                        let _ = spinner_render::print_tick(
                            &style,
                            &output,
                            &mut (*safe_output_terminal.lock().unwrap())
                        );
                        // Increment count to affect the output in the next iteration of this loop.
                        count += 1;
                    },

                    // Poll shutdown channel.
                    // This branch is cancel safe because recv is cancel safe.
                    _ = shutdown_receiver.recv() => {
                        // This spinner is now shutdown, so other task(s) using it will
                        // know that this spinner has been shutdown by user interaction or
                        // other means.
                        *self_safe_is_shutdown.lock().unwrap() = true;
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self, final_message: &str) -> miette::Result<()> {
        // Tell readline that spinner is inactive.
        _ = self
            .shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::SpinnerInactive)
            .await;

        // Shutdown the task (if it hasn't already been shutdown).
        if !*self.safe_is_shutdown.lock().unwrap() {
            // Produces an error if the spinner is already shutdown.
            _ = self.shutdown_sender.send(());
        }

        // Print the final message.
        let final_output = spinner_render::render_final_tick(
            &self.style,
            final_message,
            get_terminal_width(),
        );
        spinner_render::print_final_tick(
            &self.style,
            &final_output,
            &mut *self.safe_output_terminal.clone().lock().unwrap(),
        )?;

        // Resume the terminal.
        let _ = self
            .shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use r3bl_core::StdMutex;
    use r3bl_test_fixtures::StdoutMock;
    use r3bl_tui::{SpinnerColor, SpinnerTemplate};
    use smallvec::SmallVec;

    use super::{Duration,
                LineStateControlSignal,
                SharedWriter,
                Spinner,
                SpinnerStyle,
                TTYResult,
                is_fully_uninteractive_terminal};

    type ArrayVec = SmallVec<[LineStateControlSignal; FACTOR as usize]>;
    const FACTOR: u32 = 5;
    const QUANTUM: Duration = Duration::from_millis(100);

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_spinner_color() {
        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let shared_writer = SharedWriter::new(line_sender);

        let spinner = Spinner::try_start(
            "message".to_string(),
            QUANTUM,
            SpinnerStyle {
                template: SpinnerTemplate::Braille,
                color: SpinnerColor::None,
            },
            safe_output_terminal,
            shared_writer,
        )
        .await;

        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }

        let mut spinner = spinner.unwrap().unwrap();

        tokio::time::sleep(QUANTUM * FACTOR).await;

        spinner.stop("final message").await.unwrap();

        let output_buffer_data = stdout_mock.get_copy_of_buffer_as_string_strip_ansi();
        // println!("{:?}", output_buffer_data);

        assert!(output_buffer_data.contains("final message"));
        assert_eq!(
            output_buffer_data,
            "⠁ message\n⠃ message\n⡇ message\n⠇ message\n⡎ message\nfinal message\n"
        );

        let line_control_signal_sink = {
            let mut acc = ArrayVec::new();
            loop {
                let it = line_receiver.try_recv();
                match it {
                    Ok(signal) => {
                        acc.push(signal);
                    }
                    Err(error) => match error {
                        tokio::sync::mpsc::error::TryRecvError::Empty => {
                            break;
                        }
                        tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                            break;
                        }
                    },
                }
            }
            acc
        };
        // println!("{:?}", line_control_signal_sink);

        assert_eq!(line_control_signal_sink.len(), 4);
        matches!(
            line_control_signal_sink[0],
            LineStateControlSignal::SpinnerActive(_)
        );
        matches!(line_control_signal_sink[1], LineStateControlSignal::Pause);
        matches!(
            line_control_signal_sink[2],
            LineStateControlSignal::SpinnerInactive
        );
        matches!(line_control_signal_sink[3], LineStateControlSignal::Resume);

        drop(line_receiver);
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_spinner_no_color() {
        let stdout_mock = StdoutMock::default();

        let safe_output_terminal = Arc::new(StdMutex::new(stdout_mock.clone()));

        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let shared_writer = SharedWriter::new(line_sender);

        let quantum = QUANTUM;

        let spinner = Spinner::try_start(
            "message".to_string(),
            quantum,
            SpinnerStyle::default(),
            safe_output_terminal,
            shared_writer,
        )
        .await;

        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }

        let mut spinner = spinner.unwrap().unwrap();

        tokio::time::sleep(quantum * FACTOR).await;

        spinner.stop("final message").await.unwrap();

        // spell-checker:disable
        let output_buffer_data = stdout_mock.get_copy_of_buffer_as_string();
        // println!("{:?}", output_buffer_data);
        assert!(output_buffer_data.contains("final message"));
        assert_ne!(
            output_buffer_data,
            "⠁ message\n⠃ message\n⡇ message\n⠇ message\n⡎ message\nfinal message\n"
        );
        assert!(output_buffer_data.contains(
            "\u{1b}[1G\u{1b}[2K\u{1b}[38;2;18;194;233m⠁\u{1b}[39m \u{1b}[38;2;18;194;233mmessage"
        ));
        assert!(
            output_buffer_data
                .contains("\u{1b}[39m\n\u{1b}[1A\u{1b}[1G\u{1b}[2Kfinal message\n")
        );
        // spell-checker:enable

        let line_control_signal_sink = {
            let mut acc = ArrayVec::new();
            loop {
                let it = line_receiver.try_recv();
                match it {
                    Ok(signal) => {
                        acc.push(signal);
                    }
                    Err(error) => match error {
                        tokio::sync::mpsc::error::TryRecvError::Empty => {
                            break;
                        }
                        tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                            break;
                        }
                    },
                }
            }
            acc
        };
        // println!("{:?}", line_control_signal_sink);

        assert_eq!(line_control_signal_sink.len(), 4);
        matches!(
            line_control_signal_sink[0],
            LineStateControlSignal::SpinnerActive(_)
        );
        matches!(line_control_signal_sink[1], LineStateControlSignal::Pause);
        matches!(
            line_control_signal_sink[2],
            LineStateControlSignal::SpinnerInactive
        );
        matches!(line_control_signal_sink[3], LineStateControlSignal::Resume);

        drop(line_receiver);
    }
}

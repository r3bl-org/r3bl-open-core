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

use tokio::{sync::broadcast, time::interval};

use crate::{contains_ansi_escape_sequence, get_terminal_width,
            is_fully_uninteractive_terminal, is_stdout_piped, ok, spinner_print,
            spinner_render, InlineString, LineStateControlSignal, OutputDevice,
            SafeBool, SharedWriter, SpinnerStyle, StdMutex, StdoutIsPipedResult,
            TTYResult};

/// `Spinner` works in conjunction with [`crate::ReadlineAsyncContext`] to provide a
/// spinner in the terminal for long running tasks.
///
/// While the spinner is active, the async terminal output is paused. Also, when `Ctrl+C`
/// or `Ctrl+D` is pressed, while both the readline **is active**, and a spinner **is
/// active**, the spinner will be stopped, but the readline will continue to run. This
/// behavior will not work unless **both** are active:
/// - The readline is active, when [`crate::ReadlineAsyncContext::read_line()`] is called.
/// - The spinner is active, when [`Spinner::try_start()`] is called.
///
/// This behavior is handled by [`crate::ReadlineAsyncContext`], with some coordination
/// with `Spinner`. The spinner has to tell the [`crate::ReadlineAsyncContext`] before it
/// starts, and provide a way to stop the spinner when `Ctrl+C` or `Ctrl+D` is pressed.
/// Here are the details:
///
/// - In [`Self::try_start_task()`], the `Spinner` will send a [`LineStateControlSignal`],
///   containing a `shutdown_sender` of type [`tokio::sync::broadcast::Sender`<()>],
///   signal to the [`SharedWriter`] instance of the [`crate::ReadlineAsyncContext`].
///   - This tells the [`crate::ReadlineAsyncContext`] that a spinner is active.
///   - It also gives a way to stop the spinner via the `shutdown_sender`.
///
/// - With this teed up, when `Ctrl+C` or `Ctrl+D` is intercepted by
///   [`crate::ReadlineAsyncContext`] in
///   [`crate::readline_internal::apply_event_to_line_state_and_render()`], this will
///   result in a `()` to be sent to [`crate::Readline::safe_spinner_is_active`], which
///   shuts the spinner down.
///
/// # Usage Example
///
/// To properly stop a spinner and ensure it has completely shutdown:
/// ```
/// # use std::time::Duration;
/// # use r3bl_tui::{SpinnerStyle, OutputDevice, Spinner};
/// # async fn example() -> miette::Result<()> {
///     let mut spinner = Spinner::try_start(
///         "Loading...",
///         "Done!",
///         Duration::from_millis(100),
///         SpinnerStyle::default(),
///         OutputDevice::default(),
///         None,
///     )
///     .await?
///     .unwrap();
///
///     // Some work happens here...
///
///     // Stop the spinner (sends the shutdown signal)
///     spinner.request_shutdown();
///     // Wait for the spinner to completely shutdown
///     spinner.await_shutdown().await;
/// # Ok(())
/// # }
/// ```
#[allow(missing_debug_implementations)]
pub struct Spinner {
    pub tick_delay: Duration,
    /// ANSI escape sequences are stripped from this before being assigned.
    pub interval_message: InlineString,
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
    /// Create a new instance of [Spinner]. If the `arg_spinner_message` contains ANSI
    /// escape sequences then these will be stripped.
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
        arg_interval_msg: impl AsRef<str>,
        arg_final_msg: impl AsRef<str>,
        tick_delay: Duration,
        style: SpinnerStyle,
        output_device: OutputDevice,
        maybe_shared_writer: Option<SharedWriter>,
    ) -> miette::Result<Option<Spinner>> {
        // Early return if the terminal is not fully interactive.
        if let StdoutIsPipedResult::StdoutIsPiped = is_stdout_piped() {
            return Ok(None);
        }
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return Ok(None);
        }

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
            interval_message: interval_msg.into(),
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

        Ok(Some(spinner))
    }

    /// This is meant for the task that spawned this [Spinner] to check if it should
    /// shutdown, due to:
    /// 1. The user pressing `Ctrl+C` or `Ctrl+D`.
    /// 2. Or the [`Spinner::request_shutdown`] got called.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
    #[must_use]
    pub fn is_shutdown(&self) -> bool { *self.safe_is_shutdown.lock().unwrap() }

    /// Start and manage a task that will run in the background. This is where the spinner
    /// is started and the task is spawned. This will also pause the terminal output while
    /// the spinner is active. This will continue running until
    /// [`Self::request_shutdown()`] is called, which simply sends a message to the
    /// shutdown channel, so that this task can shut itself down.
    ///
    /// This method also sets up a [`tokio::sync::oneshot::channel`] to allow
    /// [`Self::await_shutdown()`] to know when the task has completely finished.
    ///
    /// # Panics
    ///
    /// This will panic if the lock is poisoned, which can happen if a thread
    /// panics while holding the lock. To avoid panics, ensure that the code that
    /// locks the mutex does not panic while holding the lock.
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

        // This does nothing if this is used in a `ReadlineAsync` context.
        spinner_print::print_start_if_standalone(
            self.output_device.clone(),
            self.maybe_shared_writer.clone(),
        )?;

        // Create a oneshot channel to signal when the task is complete
        let (shutdown_complete_sender, shutdown_complete_receiver) =
            tokio::sync::oneshot::channel::<()>();
        self.maybe_shutdown_complete_rx = Some(shutdown_complete_receiver);

        // These are all moved into the spawn block.
        let output_device_clone = self.output_device.clone();
        let interval_message_clone = self.interval_message.clone();
        let final_message_clone = self.final_message.clone();
        let maybe_shared_writer_clone = self.maybe_shared_writer.clone();
        let mut style_clone = self.style.clone();
        let tick_delay_clone = self.tick_delay;

        tokio::spawn(async move {
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
                            maybe_shared_writer_clone.clone(),
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
                    _ = interval.tick() => {
                        // Early return if the spinner is shutdown.
                        if *self_safe_is_shutdown.lock().unwrap() {
                            break;
                        }

                        // Render and print the interval message, based on style.
                        let output = spinner_render::render_tick(
                            &mut style_clone,
                            &interval_message_clone,
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
        });

        ok!()
    }

    /// Shutdown the task started by [`Self::try_start_task()`]. This method only sends
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
}

#[cfg(test)]
mod tests {
    use smallvec::SmallVec;

    use super::{Duration, LineStateControlSignal, SharedWriter, Spinner, SpinnerStyle,
                TTYResult};
    use crate::{return_if_not_interactive_terminal, OutputDevice, OutputDeviceExt,
                SpinnerColor, SpinnerTemplate};

    type ArrayVec = SmallVec<[LineStateControlSignal; FACTOR as usize]>;
    const FACTOR: u32 = 5;
    const QUANTUM: Duration = Duration::from_millis(100);

    #[serial_test::serial]
    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_spinner_color() {
        let (output_device_mock, stdout_mock) = OutputDevice::new_mock();

        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let shared_writer = SharedWriter::new(line_sender);

        let res_maybe_spinner = Spinner::try_start(
            "message",
            "final message",
            QUANTUM,
            SpinnerStyle {
                template: SpinnerTemplate::Braille,
                color: SpinnerColor::None,
            },
            output_device_mock,
            Some(shared_writer),
        )
        .await;

        return_if_not_interactive_terminal!();

        let mut spinner = res_maybe_spinner.unwrap().unwrap();

        // Let the spinner run for a while.
        tokio::time::sleep(QUANTUM * FACTOR).await;

        // This might take some time to finish, so we need to wait for it.
        spinner.request_shutdown();
        spinner.await_shutdown().await;

        let output_buffer_data = stdout_mock.get_copy_of_buffer_as_string_strip_ansi();
        println!("{output_buffer_data:?}");
        assert!(output_buffer_data.contains("⠁ message\n"));
        assert!(output_buffer_data.contains("final message\n"));

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

    #[serial_test::serial]
    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_spinner_no_color() {
        let (output_device_mock, stdout_mock) = OutputDevice::new_mock();

        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let shared_writer = SharedWriter::new(line_sender);

        let res_maybe_spinner = Spinner::try_start(
            "message",
            "final message",
            QUANTUM,
            SpinnerStyle::default(),
            output_device_mock,
            Some(shared_writer),
        )
        .await;

        return_if_not_interactive_terminal!();

        let mut spinner = res_maybe_spinner.unwrap().unwrap();

        // Let the spinner run for a while.
        tokio::time::sleep(QUANTUM * FACTOR).await;

        // This might take some time to finish, so we need to wait for it.
        spinner.request_shutdown();
        spinner.await_shutdown().await;

        // spell-checker:disable
        let output_buffer_data = stdout_mock.get_copy_of_buffer_as_string();
        println!("{output_buffer_data:?}");
        let stripped_output_buffer_data =
            strip_ansi_escapes::strip_str(&output_buffer_data);
        assert!(stripped_output_buffer_data.contains("⠁ message\n"));
        assert!(stripped_output_buffer_data.contains("final message\n"));
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

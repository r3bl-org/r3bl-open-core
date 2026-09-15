// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Channel monitoring and output management for async readline.
//!
//! # Task creation, shutdown and cleanup
//!
//! The task spawned by [`spawn_task_to_monitor_line_control_channel()`] is designed to
//! exit cleanly.
//! - The preferred way to shut it down is by sending a
//!   [`LineStateControlSignal::ExitReadlineLoop`] signal (which is done automatically by
//!   [`ReadlineAsyncContext::request_shutdown()`]).
//! - Alternatively, the task will also exit when the [`Readline`] instance is dropped,
//!   because the [`tokio::sync::mpsc::channel()`]'s [`tokio::sync::mpsc::Sender`] (held
//!   by [`SharedWriter`]) is dropped, closing the channel.
//!
//! # Support for buffering & writing output from [`SharedWriter`]s
//!
//! - This module contains the logic for processing messages from the line control channel
//!   created in [`Readline::try_new()`].
//! - The sender end of this channel is exposed publicly via
//!   [`SharedWriter::line_state_control_channel_sender`].
//! - This channel is used to send signals *from* [`SharedWriter`]s *to* this background
//!   task, to control the [`LineState`] of the terminal.
//! - Note that [`Readline::readline()`] must be called in a loop while the user is
//!   interacting with the terminal, so that these signals can be processed.
//!
//! # Buffering and output
//!
//! When the terminal is paused, the output from the [`SharedWriter`]s is buffered in a
//! [`PauseBuffer`]. When the terminal is resumed, the buffer is drained and the output is
//! written to the terminal.
//!
//! [`LineState`]: crate::LineState
//! [`LineStateControlSignal::ExitReadlineLoop`]:
//!     crate::LineStateControlSignal::ExitReadlineLoop
//! [`PauseBuffer`]: crate::PauseBuffer
//! [`Readline::readline()`]: crate::Readline::readline
//! [`Readline::try_new()`]: crate::Readline::try_new
//! [`Readline`]: crate::Readline
//! [`ReadlineAsyncContext::request_shutdown()`]:
//!     crate::ReadlineAsyncContext::request_shutdown
//! [`SharedWriter::line_state_control_channel_sender`]:
//!     crate::SharedWriter::line_state_control_channel_sender
//! [`SharedWriter`]: crate::SharedWriter
//! [`tokio::sync::mpsc::channel()`]: tokio::sync::mpsc::channel
//! [`tokio::sync::mpsc::Sender`]: tokio::sync::mpsc::Sender

use crate::{CommonResultWithError, Continuation, LineState, LineStateControlSignal,
            OutputDevice, PauseBuffer, PauseState, PauseStateTransition, ReadlineError,
            SafeLineState, SafePauseBuffer, SendRawTerminal, StdMutex, join, ok};
use std::{io, sync::Arc};
use tokio::{spawn,
            sync::{broadcast, mpsc},
            task::JoinHandle};

/// Spawns a background task that monitors the line control channel for incoming signals.
///
/// This task receives signals from the [`SharedWriter`]s (which hold the sender end of
/// the channel) and handles the actual writing to the terminal. It also manages pause
/// states and spinner states.
///
/// [`SharedWriter`]: crate::SharedWriter
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

/// Processes a line control signal, and actually writes the line or buffered lines to
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
///
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
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
            if line_state.pause_state.pause_spinner() == PauseStateTransition::Paused {
                output_device
                    .write(|term| {
                        line_state.clear_and_render_and_flush(term).map_err(|_| {
                            ReadlineError::IO(io::Error::other(
                                "failed to pause terminal",
                            ))
                        })?;
                        ok!()
                    })
                    .into()
            } else {
                Continuation::Continue
            }
        }),

        // Resume the terminal.
        LineStateControlSignal::Resume => self_safe_line_state.write(|line_state| {
            if line_state.pause_state.resume_spinner() == PauseStateTransition::Resumed {
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

/// Flushes all writers to the terminal and re-renders the prompt string.
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
///
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
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

#[cfg(test)]
mod test_pause_and_resume_support {
    use super::*;
    use crate::{OutputDeviceExt, core::test_fixtures::StdoutMock, vp_height, vp_width};
    use std::sync::Arc;
    use tokio::sync::{broadcast, mpsc};

    type TestFixtures = (
        SafeLineState,
        SafePauseBuffer,
        OutputDevice,
        StdoutMock,
        Arc<StdMutex<Option<broadcast::Sender<()>>>>,
    );

    fn create_test_fixtures() -> TestFixtures {
        let test_size = vp_width(100) + vp_height(100);
        let safe_line_state = Arc::new(crate::scoped_mutex!(
            SPECIFIC,
            LineState::new("> ".to_string(), test_size)
        ));
        let safe_pause_buffer = Arc::new(StdMutex::new(PauseBuffer::new()));
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let safe_spinner = Arc::new(StdMutex::new(None));
        (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            stdout_mock,
            safe_spinner,
        )
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_flush_internal_paused() {
        let (
            safe_line_state,
            safe_is_paused_buffer,
            _output_device,
            mut stdout_mock,
            _safe_spinner,
        ) = create_test_fixtures();

        safe_is_paused_buffer.write(|pb| {
            pb.push("Paused line 1".into());
            pb.push("Paused line 2".into());
        });

        safe_line_state.write(|line_state| {
            let result = flush_internal(
                &safe_is_paused_buffer,
                PauseState::PausedBySpinner,
                line_state,
                &mut stdout_mock,
            );
            assert!(result.is_ok());
        });

        assert_eq!(stdout_mock.get_copy_of_buffer_as_string_strip_ansi(), "");
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_flush_internal_not_paused() {
        let (
            safe_line_state,
            safe_is_paused_buffer,
            _output_device,
            mut stdout_mock,
            _safe_spinner,
        ) = create_test_fixtures();

        safe_is_paused_buffer.write(|pb| {
            pb.push("Paused line 1".into());
            pb.push("Paused line 2".into());
        });

        safe_line_state.write(|line_state| {
            let result = flush_internal(
                &safe_is_paused_buffer,
                PauseState::NotPaused,
                line_state,
                &mut stdout_mock,
            );
            assert!(result.is_ok());
        });

        assert_eq!(
            stdout_mock.get_copy_of_buffer_as_string_strip_ansi(),
            "Paused line 1Paused line 2\n> > "
        );
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_process_signal_line_unpaused_prints_immediately() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();

        let cont = process_line_control_signal(
            LineStateControlSignal::Line("Hello world\n".to_string().into()),
            safe_pause_buffer.clone(),
            safe_line_state,
            output_device,
            safe_spinner,
        );

        assert!(matches!(cont, Continuation::Continue));
        assert_eq!(safe_pause_buffer.read(PauseBuffer::len), 0);
        assert!(
            stdout_mock
                .get_copy_of_buffer_as_string_strip_ansi()
                .contains("Hello world")
        );
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_process_signal_line_paused_buffers_text() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();

        // Put into paused state first.
        safe_line_state.write(|ls| {
            ls.pause_state.pause_spinner();
        });

        let cont = process_line_control_signal(
            LineStateControlSignal::Line("Buffered line\n".to_string().into()),
            safe_pause_buffer.clone(),
            safe_line_state,
            output_device,
            safe_spinner,
        );

        assert!(matches!(cont, Continuation::Continue));
        assert_eq!(safe_pause_buffer.read(PauseBuffer::len), 1);
        assert_eq!(stdout_mock.get_copy_of_buffer_as_string_strip_ansi(), "");
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_process_signal_pause_and_resume() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();

        // 1. Initial pause signal transitions NotPaused -> Paused.
        let cont_pause = process_line_control_signal(
            LineStateControlSignal::Pause,
            safe_pause_buffer.clone(),
            safe_line_state.clone(),
            output_device.clone(),
            safe_spinner.clone(),
        );
        assert!(matches!(cont_pause, Continuation::Continue));
        assert_eq!(
            safe_line_state.read(|ls| ls.pause_state),
            PauseState::PausedBySpinner
        );

        // 2. Duplicate pause signal (AlreadyPaused) is a no-op.
        let cont_pause_dup = process_line_control_signal(
            LineStateControlSignal::Pause,
            safe_pause_buffer.clone(),
            safe_line_state.clone(),
            output_device.clone(),
            safe_spinner.clone(),
        );
        assert!(matches!(cont_pause_dup, Continuation::Continue));

        // 3. Buffer a line while paused.
        process_line_control_signal(
            LineStateControlSignal::Line("Buffered content\n".to_string().into()),
            safe_pause_buffer.clone(),
            safe_line_state.clone(),
            output_device.clone(),
            safe_spinner.clone(),
        );
        assert_eq!(safe_pause_buffer.read(PauseBuffer::len), 1);

        // 4. Resume signal transitions Paused -> Resumed and flushes buffer.
        let cont_resume = process_line_control_signal(
            LineStateControlSignal::Resume,
            safe_pause_buffer.clone(),
            safe_line_state.clone(),
            output_device.clone(),
            safe_spinner.clone(),
        );
        assert!(matches!(cont_resume, Continuation::Continue));
        assert_eq!(
            safe_line_state.read(|ls| ls.pause_state),
            PauseState::NotPaused
        );
        assert_eq!(safe_pause_buffer.read(PauseBuffer::len), 0);
        assert!(
            stdout_mock
                .get_copy_of_buffer_as_string_strip_ansi()
                .contains("Buffered content")
        );

        // 5. Duplicate resume signal (AlreadyResumed) is a no-op.
        let cont_resume_dup = process_line_control_signal(
            LineStateControlSignal::Resume,
            safe_pause_buffer,
            safe_line_state,
            output_device,
            safe_spinner,
        );
        assert!(matches!(cont_resume_dup, Continuation::Continue));
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_process_signal_flush() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();

        // 1. Flush when unpaused.
        let cont = process_line_control_signal(
            LineStateControlSignal::Flush,
            safe_pause_buffer.clone(),
            safe_line_state.clone(),
            output_device.clone(),
            safe_spinner.clone(),
        );
        assert!(matches!(cont, Continuation::Continue));
        assert_eq!(
            stdout_mock.get_copy_of_buffer_as_string_strip_ansi(),
            "\n> > "
        );

        // 2. Flush when paused returns Continue without flushing.
        safe_line_state.write(|ls| {
            ls.pause_state.pause_spinner();
        });
        safe_pause_buffer.write(|pb| {
            pb.push("Paused data\n".into());
        });

        let cont_paused = process_line_control_signal(
            LineStateControlSignal::Flush,
            safe_pause_buffer.clone(),
            safe_line_state,
            output_device,
            safe_spinner,
        );
        assert!(matches!(cont_paused, Continuation::Continue));
        // Data stays buffered in pause buffer.
        assert_eq!(safe_pause_buffer.read(PauseBuffer::len), 1);
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_process_signal_exit_returns_error() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            _stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();

        let cont = process_line_control_signal(
            LineStateControlSignal::ExitReadlineLoop,
            safe_pause_buffer,
            safe_line_state,
            output_device,
            safe_spinner,
        );

        assert!(matches!(
            cont,
            Continuation::ReturnError(ReadlineError::Closed)
        ));
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_process_signal_spinner_active_and_inactive() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            _stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();
        let (tx, _rx) = broadcast::channel(1);

        // Activate spinner.
        let cont1 = process_line_control_signal(
            LineStateControlSignal::SpinnerActive(tx),
            safe_pause_buffer.clone(),
            safe_line_state.clone(),
            output_device.clone(),
            safe_spinner.clone(),
        );
        assert!(matches!(cont1, Continuation::Continue));
        assert!(safe_spinner.read(Option::is_some));

        // Deactivate spinner.
        let cont2 = process_line_control_signal(
            LineStateControlSignal::SpinnerInactive,
            safe_pause_buffer,
            safe_line_state,
            output_device,
            safe_spinner.clone(),
        );
        assert!(matches!(cont2, Continuation::Continue));
        assert!(safe_spinner.read(Option::is_none));
    }

    #[tokio::test]
    async fn test_spawn_task_shutdown_on_exit_signal() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            _stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();
        let (line_tx, line_rx) = mpsc::channel(10);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

        let handle = spawn_task_to_monitor_line_control_channel(
            line_rx,
            safe_line_state,
            output_device,
            safe_pause_buffer,
            safe_spinner,
            shutdown_tx,
        );

        // Send ExitReadlineLoop signal.
        line_tx
            .send(LineStateControlSignal::ExitReadlineLoop)
            .await
            .unwrap();

        // Await shutdown notification broadcast.
        assert!(shutdown_rx.recv().await.is_ok());
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_spawn_task_shutdown_on_channel_drop() {
        let (
            safe_line_state,
            safe_pause_buffer,
            output_device,
            _stdout_mock,
            safe_spinner,
        ) = create_test_fixtures();
        let (line_tx, line_rx) = mpsc::channel(10);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

        let handle = spawn_task_to_monitor_line_control_channel(
            line_rx,
            safe_line_state,
            output_device,
            safe_pause_buffer,
            safe_spinner,
            shutdown_tx,
        );

        // Drop sender to close the channel.
        drop(line_tx);

        // Await shutdown notification broadcast.
        assert!(shutdown_rx.recv().await.is_ok());
        handle.await.unwrap();
    }
}

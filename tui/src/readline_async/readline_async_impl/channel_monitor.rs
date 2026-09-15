// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Channel monitoring and output management for async readline.
//!
//! # Task creation, shutdown and cleanup
//!
//! The task spawned by [`spawn_task_to_monitor_line_control_channel()`] doesn't need to
//! be shutdown, since it will simply exit when the [`Readline`] instance is dropped. The
//! loop awaits on the channel, and when the [`Readline`] instance is dropped, the channel
//! is dropped as well, since the [`tokio::sync::mpsc::channel()`]'s
//! [`tokio::sync::mpsc::Sender`] is dropped when the [`SharedWriter`] associated with the
//! [`Readline`] is dropped.
//!
//! # Support for buffering & writing output from [`SharedWriter`]s
//!
//! - This module contains the logic for managing the `line_state_control_channel` that's
//!   created in [`Readline::try_new()`].
//! - This channel is used to send signals *from* [`SharedWriter`]s *to*
//!   [`Readline::readline()`], to control the [`LineState`] of the terminal.
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
//! [`PauseBuffer`]: crate::PauseBuffer
//! [`Readline::readline()`]: crate::Readline::readline
//! [`Readline::try_new()`]: crate::Readline::try_new
//! [`Readline`]: crate::Readline
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

/// - Receiver end of the channel, which does the actual writing to the terminal.
/// - The sender end of the channel is in [`SharedWriter`].
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

/// Processes a line control signal. And actually write the line or buffered lines to
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

/// Flushes all writers to terminal and erase the prompt string.
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
    use crate::{core::test_fixtures::StdoutMock, vp_height, vp_width};

    #[test]
    fn test_flush_internal_paused() {
        // Create a mock `LineState` with initial data.
        let test_size = vp_width(100) + vp_height(100);
        let safe_line_state = Arc::new(crate::scoped_mutex!(
            SPECIFIC,
            LineState::new("> ".to_string(), test_size)
        ));

        // Create a mock `SafePauseBuffer` with some paused lines.
        let mut pause_buffer = PauseBuffer::new();
        pause_buffer.push("Paused line 1".into());
        pause_buffer.push("Paused line 2".into());

        // Create a mock `SafeIsPausedBuffer` with the pause buffer.
        let safe_is_paused_buffer = Arc::new(StdMutex::new(pause_buffer));

        let mut stdout_mock = StdoutMock::default();

        safe_line_state.write(|line_state| {
            // Call the `flush_internal` function.
            let result = flush_internal(
                &safe_is_paused_buffer,
                PauseState::PausedBySpinner,
                line_state,
                &mut stdout_mock,
            );

            // Assert that the function returns Ok(())
            assert!(result.is_ok());
        });

        // Assert that the mock terminal received the expected output.
        assert_eq!(stdout_mock.get_copy_of_buffer_as_string_strip_ansi(), "");
    }

    #[test]
    fn test_flush_internal_not_paused() {
        // Create a mock `LineState` with initial data.
        let test_size = vp_width(100) + vp_height(100);
        let safe_line_state = Arc::new(crate::scoped_mutex!(
            SPECIFIC,
            LineState::new("> ".to_string(), test_size)
        ));

        // Create a mock `SafePauseBuffer` with some paused lines.
        let mut pause_buffer = PauseBuffer::new();
        pause_buffer.push("Paused line 1".into());
        pause_buffer.push("Paused line 2".into());

        // Create a mock `SafeIsPausedBuffer` with the pause buffer.
        let safe_is_paused_buffer = Arc::new(StdMutex::new(pause_buffer));

        let mut stdout_mock = StdoutMock::default();

        safe_line_state.write(|line_state| {
            // Call the `flush_internal` function.
            let result = flush_internal(
                &safe_is_paused_buffer,
                PauseState::NotPaused,
                line_state,
                &mut stdout_mock,
            );

            // Assert that the function returns Ok(())
            assert!(result.is_ok());
        });

        // Assert that the mock terminal received the expected output.
        assert_eq!(
            stdout_mock.get_copy_of_buffer_as_string_strip_ansi(),
            "Paused line 1Paused line 2\n> > "
        );
    }
}

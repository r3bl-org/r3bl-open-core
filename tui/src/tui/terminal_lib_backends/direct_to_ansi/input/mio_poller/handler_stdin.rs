// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR wakeup

//! Event handlers for stdin input processing.

use super::{super::{channel_types::{PollerEvent, StdinEvent},
                    paste_state_machine::{PasteStateResult, apply_paste_state_machine}},
            MioPollWorker};
use crate::{Continuation, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use std::io::{ErrorKind, Read as _};
use tokio::sync::broadcast::Sender;

/// Read buffer size for stdin reads (`1_024` bytes).
///
/// When `read_count == STDIN_READ_BUFFER_SIZE`, more data is likely waiting in the
/// kernel buffer—this is the `more` flag used for ESC disambiguation.
pub const STDIN_READ_BUFFER_SIZE: usize = 1_024;

/// Handles [`stdin`] becoming readable, using explicit `tx` parameter.
///
/// Reads bytes from [`stdin`], parses them into [`VT100InputEventIR`] events, applies
/// the paste state machine, and sends final events to the channel. See [EINTR Handling]
/// for how interrupted syscalls are handled.
///
/// This variant is used by [`MioPollWorker`] which implements the generic
/// [`ThreadWorker`] trait and receives `tx` as a parameter.
///
/// # Returns
///
/// - [`Continuation::Continue`]: Successfully processed or recoverable error.
/// - [`Continuation::Stop`]: [`EOF`], fatal error, or receiver dropped.
///
/// [EINTR Handling]: super#eintr-handling
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`MioPollWorker`]: super::MioPollWorker
/// [`ThreadWorker`]: crate::core::resilient_reactor_thread::ThreadWorker
/// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`stdin`]: std::io::stdin
pub fn consume_stdin_input_with_tx(
    worker: &mut MioPollWorker,
    tx: &Sender<PollerEvent>,
) -> Continuation {
    let read_res = worker
        .sources
        .stdin
        .read(&mut worker.stdin_unparsed_byte_buffer);
    match read_res {
        Ok(0) => {
            // EOF reached.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "mio_poller thread: EOF (0 bytes)");
            });
            drop(tx.send(PollerEvent::Stdin(StdinEvent::Eof)));
            Continuation::Stop
        }

        Ok(n) => parse_stdin_bytes_with_tx(worker, n, tx),

        Err(ref e) if e.kind() == ErrorKind::Interrupted => {
            // EINTR - retry.
            Continuation::Continue
        }

        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            // No more data available right now (spurious wakeup).
            Continuation::Continue
        }

        Err(e) => {
            // Other error - send and exit.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "mio_poller thread: read error",
                    error = ?e
                );
            });
            drop(tx.send(PollerEvent::Stdin(StdinEvent::Error)));
            Continuation::Stop
        }
    }
}

/// Parses bytes read from stdin into input events, using explicit `tx` parameter.
///
/// Parses bytes into VT100 events and sends them through the paste state machine.
pub fn parse_stdin_bytes_with_tx(
    worker: &mut MioPollWorker,
    n: usize,
    tx: &Sender<PollerEvent>,
) -> Continuation {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(message = "mio_poller thread: read bytes", bytes_read = n);
    });

    // `more` flag for ESC disambiguation.
    let more = n == STDIN_READ_BUFFER_SIZE;

    // Parse bytes into events.
    worker
        .vt_100_input_seq_parser
        .advance(&worker.stdin_unparsed_byte_buffer[..n], more);

    // Process all parsed events through paste state machine.
    for vt100_event in worker.vt_100_input_seq_parser.by_ref() {
        match apply_paste_state_machine(&mut worker.paste_collection_state, &vt100_event)
        {
            PasteStateResult::Emit(input_event) => {
                if tx
                    .send(PollerEvent::Stdin(StdinEvent::Input(input_event)))
                    .is_err()
                {
                    // Receiver dropped - exit gracefully.
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::debug!(
                            message = "mio_poller thread: receiver dropped, exiting"
                        );
                    });
                    return Continuation::Stop;
                }
            }
            PasteStateResult::Absorbed => {
                // Event absorbed (e.g., paste in progress).
            }
        }
    }

    Continuation::Continue
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR wakeup

//! Event handlers for stdin input processing.

use super::poller::MioPoller;
use crate::tui::{DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                 terminal_lib_backends::direct_to_ansi::input::{paste_state_machine::apply_paste_state_machine,
                                                                types::{PasteStateResult,
                                                                        ReaderThreadMessage,
                                                                        ThreadLoopContinuation}}};
use std::io::{ErrorKind, Read as _};

/// Read buffer size for stdin reads (`1_024` bytes).
///
/// When `read_count == STDIN_READ_BUFFER_SIZE`, more data is likely waiting in the
/// kernel buffer—this is the `more` flag used for ESC disambiguation.
pub const STDIN_READ_BUFFER_SIZE: usize = 1_024;

/// Handles [`stdin`] becoming readable.
///
/// Reads bytes from [`stdin`], parses them into [`VT100InputEventIR`] events, applies
/// the paste state machine, and sends final events to the channel.
///
/// # Returns
///
/// - [`ThreadLoopContinuation::Continue`]: Successfully processed or recoverable error.
/// - [`ThreadLoopContinuation::Return`]: EOF, fatal error, or receiver dropped.
///
/// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`stdin`]: std::io::stdin
pub fn consume_stdin_input(poller: &mut MioPoller) -> ThreadLoopContinuation {
    let read_res = poller
        .sources
        .stdin
        .read(&mut poller.stdin_unparsed_byte_buffer);
    match read_res {
        Ok(0) => {
            // EOF reached.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "mio-poller-thread: EOF (0 bytes)");
            });
            let _unused = poller.tx_parsed_input_events.send(ReaderThreadMessage::Eof);
            ThreadLoopContinuation::Return
        }

        Ok(n) => parse_stdin_bytes(poller, n),

        Err(ref e) if e.kind() == ErrorKind::Interrupted => {
            // EINTR ("Interrupted" — a signal arrived while the syscall was blocked).
            // Will retry on next poll iteration.
            // https://man7.org/linux/man-pages/man7/signal.7.html
            ThreadLoopContinuation::Continue
        }

        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            // No more data available right now (spurious wakeup).
            ThreadLoopContinuation::Continue
        }

        Err(e) => {
            // Other error - send and exit.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "mio-poller-thread: read error",
                    error = ?e
                );
            });
            let _unused = poller
                .tx_parsed_input_events
                .send(ReaderThreadMessage::Error);
            ThreadLoopContinuation::Return
        }
    }
}

/// Parses bytes read from stdin into input events.
///
/// Parses bytes into VT100 events and sends them through the paste state machine.
pub fn parse_stdin_bytes(poller: &mut MioPoller, n: usize) -> ThreadLoopContinuation {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(message = "mio-poller-thread: read bytes", bytes_read = n);
    });

    // `more` flag for ESC disambiguation.
    let more = n == STDIN_READ_BUFFER_SIZE;

    // Parse bytes into events.
    poller
        .vt_100_input_seq_parser
        .advance(&poller.stdin_unparsed_byte_buffer[..n], more);

    // Process all parsed events through paste state machine.
    for vt100_event in poller.vt_100_input_seq_parser.by_ref() {
        match apply_paste_state_machine(&mut poller.paste_collection_state, &vt100_event)
        {
            PasteStateResult::Emit(input_event) => {
                if poller
                    .tx_parsed_input_events
                    .send(ReaderThreadMessage::Event(input_event))
                    .is_err()
                {
                    // Receiver dropped - exit gracefully.
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::debug!(
                            message = "mio-poller-thread: receiver dropped, exiting"
                        );
                    });
                    return ThreadLoopContinuation::Return;
                }
            }
            PasteStateResult::Absorbed => {
                // Event absorbed (e.g., paste in progress).
            }
        }
    }

    ThreadLoopContinuation::Continue
}

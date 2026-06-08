// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR wakeup kevent EPOLLET fcntl setfl NONBLOCK EINTR

//! Event handlers for stdin input processing.

use super::{super::{channel_types::{PollerEvent, StdinEvent},
                    paste_state_machine::{PasteStateResult, apply_paste_state_machine}},
            MioPollWorker};
use crate::{Continuation, core::resilient_reactor_thread::RRTEvent,
            tui::DEBUG_TUI_SHOW_MIO_POLLER};
use std::io::{ErrorKind, Read as _};
use tokio::sync::broadcast::Sender;

/// Read buffer size for stdin reads (`1_024` bytes).
///
/// When `read_count == STDIN_READ_BUFFER_SIZE`, more data is likely waiting in the
/// kernel buffer—this is the `more` flag used for [`ESC`] disambiguation.
///
/// [`ESC`]: crate::EscSequence
pub const STDIN_READ_BUFFER_SIZE: usize = 1_024;

/// Handles [`stdin`] becoming readable, using explicit `sender` parameter.
///
/// Reads bytes from [`stdin`], parses them into [`VT100InputEventIR`] events, applies the
/// paste state machine, and sends final events to the channel. See [`EINTR` Handling] for
/// how interrupted syscalls are handled.
///
/// # Edge-Triggered vs. Level-Triggered Polling
///
/// In high-performance systems like [`mio`] (the underlying engine for [`tokio`]):
/// - **Level-Triggered Polling** repeatedly triggers kernel wakeups as long as *any* data
///   is present on the [`fd`]. If we read only part of the data, the next [`poll`] call
///   wakes up immediately. While simpler, this causes massive kernel-to-user
///   context-switch overhead and leads to the *thundering herd* problem where multiple
///   worker threads are woken up unnecessarily to compete for the same resource.
/// - **Edge-Triggered Polling** triggers exactly once on the state transition. This
///   minimizes system calls (specifically kernel wakeups like [`epoll_wait`] or
///   [`kevent`]) and avoids the thundering herd problem, but shifts the responsibility to
///   the application to fully drain the [`fd`] before yielding back to the poll loop.
///
/// # Edge-Triggered Polling & Deadlock Prevention
///
/// On Unix, [`mio`] uses edge-triggered polling ([`EPOLLET`] under the hood on Linux, and
/// emulated on macOS/[`kqueue`]). This means the OS only notifies the poller thread when
/// a file descriptor transitions from "empty" to "has data".
///
/// To prevent deadlocks, this function MUST drain the [`stdin`] [`fd`] completely by
/// reading in a loop until it encounters an [`ErrorKind::WouldBlock`] error. If we read
/// only once and leave any data behind on the [`fd`], the file descriptor will remain in
/// a "has data" state, the edge trigger will never reset, and [`mio::Poll`] will sleep
/// indefinitely, causing the UI to freeze.
///
/// This function is specifically designed to be called by [`MioPollWorker`], which
/// implements the generic [`RRTWorker`] trait and receives `sender` as a parameter.
///
/// # Why We Need Non-Blocking Read
///
/// By default, [`stdin`] is a blocking resource on Linux (and POSIX systems in general),
/// i.e., the  [`.read()`] [`syscall`] is blocking. Since edge-triggered polling requires
/// draining the [`fd`] in a `loop` until it is empty, repeatedly calling [`.read()`] on
/// an empty, blocking [`stdin`] [`fd`] would block the event loop thread forever, causing
/// a deadlock.
///
/// This non-blocking behavior implementation spans two files:
/// 1. In [`MioPollWorker::create_and_register_os_sources()`] we actually set non-blocking
///    mode explicitly on [`stdin`], using [`O_NONBLOCK`]. The original [`stdin`] flags
///    are saved to [`original_stdin_flags`] and restored in the [`Drop`] implementation
///    ([`RAII`] guard) to prevent breaking the terminal.
/// 1. In this file, we use the non-blocking mode [`stdin`], to ensure that if the
///    [`stdin`] [`fd`] is empty, [`.read()`] returns immediately with
///    [`ErrorKind::WouldBlock`]. This allows the loop to break and yield back to the
///    [`mio::Poll`] edge-trigger.
///
/// These tests verify that this blocking-read deadlock does not occur:
/// - [`test_pty_mio_poller_thread_lifecycle`]
/// - [`test_pty_mio_poller_subscribe`]
/// - [`test_production_factory_restart_cycle`]
///
/// ## How this affects [`stdout`] as well
///
/// Because [`stdin`] and [`stdout`] share the same underlying file description on Linux,
/// setting non-blocking mode on [`stdin`] makes [`stdout`] non-blocking as well. This
/// causes [`stdout`] to return [`ErrorKind::WouldBlock`] instead of safely sleeping the
/// thread when the terminal buffer is full.
///
/// To resolve this without removing the non-blocking behavior from [`stdin`], [`stdout`]
/// uses a polite polling mechanism. See [`FullBufferWaitingStdout`] / [`new_stdout()`]
/// for the implementation of the fix.
///
/// # Returns
///
/// - [`Continuation::Continue`]: Successfully processed or recoverable error.
/// - [`Continuation::Stop`]: [`EOF`] or fatal worker-domain error.
///
/// [`.read()`]: std::io::Read::read
/// [`Drop`]: super::MioPollWorker#method.drop
/// [`EINTR` Handling]: super#eintr-handling
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`epoll_wait`]: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
/// [`EPOLLET`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`ErrorKind::WouldBlock`]: std::io::ErrorKind::WouldBlock
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`FullBufferWaitingStdout`]: crate::core::terminal_io::FullBufferWaitingStdout
/// [`kevent`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`mio::Poll`]: mio::Poll
/// [`MioPollWorker::create_and_register_os_sources()`]:
///     super::MioPollWorker#method.create_and_register_os_sources
/// [`MioPollWorker`]: super::MioPollWorker
/// [`new_stdout()`]: crate::core::terminal_io::OutputDevice::new_stdout
/// [`O_NONBLOCK`]: rustix::fs::OFlags::NONBLOCK
/// [`original_stdin_flags`]: field@super::MioPollWorker::original_stdin_flags
/// [`poll`]: mio::Poll::poll
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`RRTWorker`]: crate::RRTWorker
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`test_production_factory_restart_cycle`]:
///     crate::core::resilient_reactor_thread::rrt_integration_tests::pty_test_production_factory_restart::test_production_factory_restart_cycle
/// [`test_pty_mio_poller_subscribe`]:
///     crate::core::ansi::vt_100_terminal_input_parser::vt_100_parser_integration_tests::pty_mio_poller_subscribe_test::test_pty_mio_poller_subscribe
/// [`test_pty_mio_poller_thread_lifecycle`]:
///     crate::core::ansi::vt_100_terminal_input_parser::vt_100_parser_integration_tests::pty_mio_poller_thread_lifecycle_test::test_pty_mio_poller_thread_lifecycle
/// [`tokio`]: tokio
/// [`VT100InputEventIR`]:
///     crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
pub fn consume_stdin_input_with_sender(
    worker: &mut MioPollWorker,
    sender: &Sender<RRTEvent<PollerEvent>>,
) -> Continuation {
    loop {
        let read_res = worker
            .sources
            .stdin
            .read(&mut worker.stdin_unparsed_byte_buffer);
        match read_res {
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // Handle non-blocking stdin read().
                //
                // No more data available right now (meaning that the stdin fd is fully
                // drained).
                return Continuation::Continue;
            }

            Ok(0) => {
                // EOF reached.
                DEBUG_TUI_SHOW_MIO_POLLER.then(|| {
                    tracing::debug!(message = "mio_poller thread: EOF (0 bytes)");
                });
                drop(sender.send(PollerEvent::Stdin(StdinEvent::Eof).into()));
                return Continuation::Stop;
            }

            Ok(n) => {
                if let Continuation::Stop =
                    parse_stdin_bytes_with_sender(worker, n, sender)
                {
                    return Continuation::Stop;
                }
            }

            #[allow(clippy::needless_continue)]
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                // EINTR - retry immediately.
                continue;
            }

            Err(e) => {
                // Other error - send and exit.
                DEBUG_TUI_SHOW_MIO_POLLER.then(|| {
                    tracing::debug!(
                        message = "mio_poller thread: read error",
                        error = ?e
                    );
                });
                drop(sender.send(PollerEvent::Stdin(StdinEvent::Error).into()));
                return Continuation::Stop;
            }
        }
    }
}

/// Parses bytes read from stdin into input events, using explicit `sender` parameter.
///
/// Parses bytes into VT100 events and sends them through the paste state machine.
pub fn parse_stdin_bytes_with_sender(
    worker: &mut MioPollWorker,
    n: usize,
    sender: &Sender<RRTEvent<PollerEvent>>,
) -> Continuation {
    DEBUG_TUI_SHOW_MIO_POLLER.then(|| {
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
                if sender
                    .send(PollerEvent::Stdin(StdinEvent::Input(input_event)).into())
                    .is_err()
                {
                    // Receiver dropped. Let run_worker_loop() evaluate shutdown.
                    DEBUG_TUI_SHOW_MIO_POLLER.then(|| {
                        tracing::debug!(
                            message =
                                "mio_poller thread: receiver dropped while sending input event"
                        );
                    });
                    return Continuation::Continue;
                }
            }
            PasteStateResult::Absorbed => {
                // Event absorbed (e.g., paste in progress).
            }
        }
    }

    Continuation::Continue
}

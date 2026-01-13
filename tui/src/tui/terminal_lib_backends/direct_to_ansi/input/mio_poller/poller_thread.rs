// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR epoll sigaction signalfd

//! Core [`MioPollerThread`] struct and lifecycle methods.

use super::{super::{channel_types::{PollerEvent, StdinEvent},
                    paste_state_machine::PasteCollectionState,
                    stateful_parser::StatefulInputParser},
            PollerThreadState, SourceKindReady, SourceRegistry,
            dispatcher::dispatch,
            handler_stdin::STDIN_READ_BUFFER_SIZE};
use crate::{Continuation, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use mio::{Events, Interest, Poll, Token, unix::SourceFd};
use signal_hook::consts::SIGWINCH;
use signal_hook_mio::v1_0::Signals;
use std::{io::ErrorKind, os::fd::AsRawFd as _, sync::Arc};

/// Capacity for the [`mio::Events`] buffer.
///
/// [`mio::Events`]: mio::Events
const EVENTS_CAPACITY: usize = 8;

/// Core poller struct managing the [`mio`] event loop.
///
/// See the [module-level documentation] for architecture details.
///
/// [module-level documentation]: super
#[allow(missing_debug_implementations)]
pub struct MioPollerThread {
    /// [`mio`] poll instance for efficient I/O multiplexing.
    ///
    /// - **Registered sources**: [`SourceRegistry::stdin`], [`SourceRegistry::signals`].
    /// - **Used by**: [`start()`] blocks on this until sources are ready.
    ///
    /// [`start()`]: MioPollerThread::start
    pub poll_handle: Poll,

    /// Buffer for events returned by [`Poll::poll()`].
    ///
    /// - **Populated by**: [`Poll::poll()`] fills this when [`std::io::stdin`] or
    ///   [`SIGWINCH`] becomes ready.
    /// - **Drained by**: [`start()`] iterates and dispatches to token-specific handlers
    ///   via [`dispatch()`].
    ///
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`dispatch()`]: crate::direct_to_ansi::input::mio_poller::dispatcher::dispatch
    /// [`start()`]: MioPollerThread::start
    pub ready_events_buffer: Events,

    /// Registry of all event sources monitored by [`poll_handle`].
    ///
    /// Centralizes management of heterogeneous sources ([`stdin`], [`signals`]).
    ///
    /// [`poll_handle`]: MioPollerThread::poll_handle
    /// [`signals`]: SourceRegistry::signals
    /// [`stdin`]: SourceRegistry::stdin
    pub sources: SourceRegistry,

    /// Buffer for reading unparsed bytes from [`std::io::stdin()`].
    ///
    /// - **Written by**: [`consume_stdin_input()`] reads into this buffer.
    /// - **Consumed by**: [`parse_stdin_bytes()`] parses bytes from here.
    ///
    /// [`consume_stdin_input()`]: crate::direct_to_ansi::input::mio_poller::handler_stdin::consume_stdin_input
    /// [`parse_stdin_bytes()`]: crate::direct_to_ansi::input::mio_poller::handler_stdin::parse_stdin_bytes
    pub stdin_unparsed_byte_buffer: [u8; STDIN_READ_BUFFER_SIZE],

    /// Stateful VT100 input sequence parser.
    ///
    /// - **Fed by**: [`parse_stdin_bytes()`] calls `advance()` with raw bytes.
    /// - **Yields**: [`VT100InputEventIR`] events via [`Iterator`] impl.
    ///
    /// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
    /// [`parse_stdin_bytes()`]: crate::direct_to_ansi::input::mio_poller::handler_stdin::parse_stdin_bytes
    pub vt_100_input_seq_parser: StatefulInputParser,

    /// Paste state machine for bracketed paste handling.
    ///
    /// - **Fed by**: [`parse_stdin_bytes()`] passes parsed events through this.
    /// - **Yields**: [`InputEvent`] after paste sequence handling.
    ///
    /// [`InputEvent`]: crate::InputEvent
    /// [`parse_stdin_bytes()`]: crate::direct_to_ansi::input::mio_poller::handler_stdin::parse_stdin_bytes
    pub paste_collection_state: PasteCollectionState,

    /// Shared thread lifecycle state. See [`PollerThreadState`] for documentation.
    pub thread_state: Arc<PollerThreadState>,
}

impl Drop for MioPollerThread {
    /// Marks the thread as terminated when the struct is dropped.
    ///
    /// Calls [`ThreadLiveness::mark_terminated()`] to set the **termination marker** in
    /// the thread lifecycle protocol, enabling [`allocate()`] to detect terminated
    /// threads and spawn new ones.
    ///
    /// **Panic-safe**: Even if [`start()`] panics, [`mark_terminated()`] is called during
    /// stack unwinding, so the next subscriber correctly detects the terminated thread.
    ///
    /// See [`PollerThreadState`] for the complete lifecycle documentation.
    ///
    /// [`ThreadLiveness::mark_terminated()`]: super::poller_thread_state::ThreadLiveness::mark_terminated
    /// [`allocate()`]: crate::direct_to_ansi::input::input_device_impl::global_input_resource::allocate
    /// [`mark_terminated()`]: super::poller_thread_state::ThreadLiveness::mark_terminated
    /// [`start()`]: MioPollerThread::start
    fn drop(&mut self) {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(
                message =
                    "mio-poller-thread: dropping, calling lifecycle.mark_terminated()"
            );
        });
        self.thread_state.thread_liveness.mark_terminated();
    }
}

impl MioPollerThread {
    /// Spawns the [`mio`] poller thread (can be relaunched if it exits).
    ///
    /// Takes shared ownership of [`PollerThreadState`] via [`Arc`]. The
    /// [`Drop`] impl sets `liveness` to `Terminated` when the thread exits, enabling
    /// [`allocate()`] to detect termination and spawn a new thread.
    ///
    /// # Panics
    ///
    /// Panics if thread spawning or [`mio`] registration fails.
    ///
    /// [`allocate()`]: crate::direct_to_ansi::input::input_device_impl::global_input_resource::allocate
    #[allow(clippy::new_ret_no_self)]
    pub fn new(poll: Poll, thread_state: Arc<PollerThreadState>) {
        let _unused = std::thread::Builder::new()
            .name("mio-poller".into())
            .spawn(move || {
                let mut mio_poller_thread = Self::setup(poll, thread_state);
                mio_poller_thread.start();
                // Drop impl sets liveness = Terminated (panic-safe).
            })
            .expect(
                "Failed to spawn mio-poller thread: OS denied thread creation. \
                 Check ulimit -u (max user processes) or available memory.",
            );
    }

    /// Initializes the [`mio`] poller, registering [`stdin`] and [`SIGWINCH`].
    ///
    /// # Panics
    ///
    /// Panics if registration of [`stdin`] or [`SIGWINCH`] fails.
    ///
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`stdin`]: std::io::stdin
    #[must_use]
    pub fn setup(poll: Poll, thread_state: Arc<PollerThreadState>) -> Self {
        let poll_handle = poll;
        let mio_registry = poll_handle.registry();

        // Register stdin with mio.
        let stdin = std::io::stdin();
        mio_registry
            .register(
                &mut SourceFd(&stdin.as_raw_fd()),
                SourceKindReady::Stdin.to_token(),
                Interest::READABLE,
            )
            .expect(
                "Failed to register stdin (fd 0) with mio: epoll_ctl failed. \
                 stdin may already be registered elsewhere or fd is invalid.",
            );

        // Register SIGWINCH with signal-hook-mio.
        let mut signals = Signals::new([SIGWINCH]).expect(
            "Failed to register SIGWINCH handler via signal-hook: \
             signal already has incompatible handler or sigaction failed.",
        );
        mio_registry
            .register(
                &mut signals,
                SourceKindReady::Signals.to_token(),
                Interest::READABLE,
            )
            .expect(
                "Failed to register SIGWINCH signal fd with mio: epoll_ctl failed on signalfd.",
            );

        let ready_events_buffer = Events::with_capacity(EVENTS_CAPACITY);

        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: started with mio::Poll");
        });

        Self {
            poll_handle,
            ready_events_buffer,
            sources: SourceRegistry { stdin, signals },
            stdin_unparsed_byte_buffer: [0u8; STDIN_READ_BUFFER_SIZE],
            vt_100_input_seq_parser: StatefulInputParser::default(),
            paste_collection_state: PasteCollectionState::Inactive,
            thread_state,
        }
    }

    /// Runs the main event loop until exit condition is met.
    ///
    /// Blocks on [`Poll::poll()`] waiting for [`stdin`] or [`SIGWINCH`] to become ready,
    /// then dispatches to the appropriate handler. See [EINTR Handling] for how
    /// interrupted syscalls are handled.
    ///
    /// <div class="warning">
    ///
    /// Note that the call to [`Poll::poll()`] is what [`inotifywait`] uses under the
    /// hood. The `timeout` set to `None` ensures that this will block. If we set a
    /// delay here, then the loop will continue after that delay and act as a
    /// busy-wait. This is similar to what `check.fish` does to implement a sliding
    /// window debounce for file changes with [`inotifywait`].
    ///
    /// </div>
    ///
    /// [EINTR Handling]: super#eintr-handling
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`inotifywait`]: https://linux.die.net/man/1/inotifywait
    /// [`stdin`]: std::io::stdin
    pub fn start(&mut self) {
        // Breaks borrow so dispatch can use `&mut self`.
        fn collect_ready_tokens(events: &Events) -> Vec<Token> {
            events.iter().map(mio::event::Event::token).collect()
        }

        loop {
            // Block until stdin or signals become ready.
            let poll_result = self.poll_handle.poll(&mut self.ready_events_buffer, None);

            // Handle poll errors.
            if let Err(err) = poll_result {
                // EINTR - retry (see module docs: EINTR Handling).
                if err.kind() == ErrorKind::Interrupted {
                    continue;
                }

                // Fatal error - notify consumers and exit loop.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "mio-poller-thread: poll error",
                        error = ?err
                    );
                });
                let _unused = self
                    .thread_state
                    .broadcast_tx
                    .send(PollerEvent::Stdin(StdinEvent::Error));
                break;
            }

            // Dispatch ready events.
            for token in collect_ready_tokens(&self.ready_events_buffer) {
                let continuation = dispatch(token, self);
                if continuation == Continuation::Stop {
                    return;
                }
            }
        }
    }
}

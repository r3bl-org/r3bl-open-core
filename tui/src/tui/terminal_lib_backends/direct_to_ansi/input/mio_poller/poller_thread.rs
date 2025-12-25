// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR epoll sigaction signalfd

//! Core [`MioPollerThread`] struct and lifecycle methods.

use super::{PollerThreadLifecycleState, SourceKindReady, SourceRegistry,
            ThreadLoopContinuation, dispatcher::dispatch,
            handler_stdin::STDIN_READ_BUFFER_SIZE};
use crate::tui::{DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                 terminal_lib_backends::direct_to_ansi::input::{channel_types::StdinReaderMessage,
                                                                paste_state_machine::PasteCollectionState,
                                                                stateful_parser::StatefulInputParser}};
use mio::{Events, Interest, Poll, Token, unix::SourceFd};
use signal_hook::consts::SIGWINCH;
use signal_hook_mio::v1_0::Signals;
use std::{io::ErrorKind, os::fd::AsRawFd as _};

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

    /// Thread lifecycle state — centralized broadcast sender, liveness flag, and waker.
    ///
    /// See [`PollerThreadLifecycleState`] for comprehensive documentation on the
    /// lifecycle protocol and the inherent race condition we handle.
    ///
    /// - **[`tx_stdin_reader_msg`]**: Broadcasts parsed events to async consumers.
    /// - **[`metadata`]**: Thread identity and liveness; set to terminated by [`Drop`].
    /// - **[`waker_signal_shutdown`]**: Used by [`InputDeviceResourceHandle::drop()`] to
    ///   wake this thread.
    ///
    /// [`InputDeviceResourceHandle::drop()`]: crate::direct_to_ansi::input::input_device::InputDeviceResourceHandle
    /// [`metadata`]: PollerThreadLifecycleState::metadata
    /// [`tx_stdin_reader_msg`]: PollerThreadLifecycleState::tx_stdin_reader_msg
    /// [`waker_signal_shutdown`]: PollerThreadLifecycleState::waker_signal_shutdown
    pub state: PollerThreadLifecycleState,
}

impl Drop for MioPollerThread {
    /// Marks the thread as terminated when the struct is dropped.
    ///
    /// Calls [`PollerThreadLifecycleState::mark_terminated()`] to set the **termination
    /// marker** in the thread lifecycle protocol, enabling [`allocate()`] to detect
    /// terminated threads and spawn new ones.
    ///
    /// **Panic-safe**: Even if [`start()`] panics, [`mark_terminated()`] is called during
    /// stack unwinding, so the next subscriber correctly detects the terminated thread.
    ///
    /// See [`PollerThreadLifecycleState`] for the complete lifecycle documentation.
    ///
    /// [`PollerThreadLifecycleState::mark_terminated()`]: PollerThreadLifecycleState::mark_terminated
    /// [`allocate()`]: crate::direct_to_ansi::input::global_input_resource::guarded_ops::allocate
    /// [`start()`]: MioPollerThread::start
    /// [`mark_terminated()`]: PollerThreadLifecycleState::mark_terminated
    fn drop(&mut self) {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(
                message =
                    "mio-poller-thread: dropping, calling lifecycle.mark_terminated()"
            );
        });
        self.state.mark_terminated();
    }
}

impl MioPollerThread {
    /// Spawns the [`mio`] poller thread, which can be relaunched if it exits.
    ///
    /// # Arguments
    ///
    /// - `poll`: The [`mio::Poll`] instance, pre-configured with the [`Waker`]
    ///   registered. The Poll is created in `initialize_input_resource()` so the
    ///   [`Waker`] can be shared with [`InputDeviceResourceHandle`].
    /// - `state`: The [`PollerThreadLifecycleState`] containing the broadcast sender and
    ///   liveness flag. Ownership is transferred to [`MioPollerThread`], whose [`Drop`]
    ///   impl sets `liveness` to `Terminated` when dropped. This enables [`allocate()`]
    ///   to detect a terminated thread and spawn a new one.
    ///
    /// This is the main entry point for starting the input handling system. It:
    /// 1. Spawns a dedicated thread named `"mio-poller"` -> useful for debugging, eg
    ///    using [`ps`] or [`htop`].
    /// 2. Registers [`stdin`] and [`SIGWINCH`] with the provided [`mio::Poll`].
    /// 3. Runs the polling loop until exit condition is met.
    /// 4. [`Drop`] impl sets `liveness` to `Terminated` (panic-safe).
    ///
    /// The thread is detached (its [`JoinHandle`] is dropped) - see the
    /// [Thread Lifecycle] section in the module docs for details.
    ///
    /// # Panic Safety
    ///
    /// Using [`Drop`] to set the liveness flag ensures correctness even if [`start()`]
    /// panics: the flag is set during stack unwinding.
    ///
    /// # Panics
    ///
    /// Panics if thread spawning or [`mio`] registration fails.
    ///
    /// [Thread Lifecycle]: super#thread-lifecycle
    /// [`InputDeviceResourceHandle`]: crate::direct_to_ansi::input::input_device::InputDeviceResourceHandle
    /// [`JoinHandle`]: std::thread::JoinHandle
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`Waker`]: mio::Waker
    /// [`allocate()`]: crate::direct_to_ansi::input::global_input_resource::guarded_ops::allocate
    /// [`htop`]: https://htop.dev/
    /// [`ps`]: https://man7.org/linux/man-pages/man1/ps.1.html
    /// [`start()`]: MioPollerThread::start
    /// [`stdin`]: std::io::stdin
    pub fn spawn(poll: Poll, state: PollerThreadLifecycleState) {
        let _unused = std::thread::Builder::new()
            .name("mio-poller".into())
            .spawn(move || {
                let mut mio_poller_thread = Self::setup(poll, state);
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
    /// # Arguments
    ///
    /// - `poll`: The [`mio::Poll`] instance, pre-configured with the [`Waker`]
    ///   registered.
    /// - `state`: The [`PollerThreadLifecycleState`] containing `tx`, `liveness`, and
    ///   `waker`.
    ///
    /// # Panics
    ///
    /// Panics if registration of [`stdin`] or [`SIGWINCH`] fails.
    ///
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`Waker`]: mio::Waker
    /// [`stdin`]: std::io::stdin
    #[must_use]
    pub fn setup(poll: Poll, state: PollerThreadLifecycleState) -> Self {
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
            state,
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
                    .state
                    .tx_stdin_reader_msg
                    .send(StdinReaderMessage::Error);
                break;
            }

            // Dispatch ready events.
            for token in collect_ready_tokens(&self.ready_events_buffer) {
                let continuation = dispatch(token, self);
                if continuation == ThreadLoopContinuation::Return {
                    return;
                }
            }
        }
    }
}

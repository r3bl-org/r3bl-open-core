// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR

//! Core [`MioPoller`] struct and lifecycle methods.

use super::{SourceKindReady, SourceRegistry, dispatcher::dispatch,
            handler_stdin::STDIN_READ_BUFFER_SIZE};
use crate::tui::{DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                 terminal_lib_backends::direct_to_ansi::input::{paste_state_machine::PasteCollectionState,
                                                                stateful_parser::StatefulInputParser,
                                                                types::{InputEventSender,
                                                                        ReaderThreadMessage,
                                                                        ThreadLoopContinuation}}};
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
pub struct MioPoller {
    /// [`mio`] poll instance for efficient I/O multiplexing.
    ///
    /// - **Registered sources**: [`SourceRegistry::stdin`], [`SourceRegistry::signals`].
    /// - **Used by**: [`start()`] blocks on this until sources are ready.
    ///
    /// [`start()`]: MioPoller::start
    pub poll_handle: Poll,

    /// Buffer for events returned by [`Poll::poll()`].
    ///
    /// - **Populated by**: [`Poll::poll()`] fills this when [`std::io::stdin`] or
    ///   `SIGWINCH` becomes ready.
    /// - **Drained by**: [`start()`] iterates and dispatches to token-specific handlers
    ///   via [`dispatch()`].
    ///
    /// [`dispatch()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::dispatcher::dispatch
    /// [`start()`]: MioPoller::start
    pub ready_events_buffer: Events,

    /// Registry of all event sources monitored by [`poll_handle`].
    ///
    /// Centralizes management of heterogeneous sources ([`stdin`], [`signals`]).
    ///
    /// [`poll_handle`]: MioPoller::poll_handle
    /// [`signals`]: SourceRegistry::signals
    /// [`stdin`]: SourceRegistry::stdin
    pub sources: SourceRegistry,

    /// Buffer for reading unparsed bytes from [`std::io::stdin()`].
    ///
    /// - **Written by**: [`consume_stdin_input()`] reads into this buffer.
    /// - **Consumed by**: [`parse_stdin_bytes()`] parses bytes from here.
    ///
    /// [`consume_stdin_input()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::consume_stdin_input
    /// [`parse_stdin_bytes()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::parse_stdin_bytes
    pub stdin_unparsed_byte_buffer: [u8; STDIN_READ_BUFFER_SIZE],

    /// Stateful VT100 input sequence parser.
    ///
    /// - **Fed by**: [`parse_stdin_bytes()`] calls `advance()` with raw bytes.
    /// - **Yields**: [`VT100InputEventIR`] events via [`Iterator`] impl.
    ///
    /// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
    /// [`parse_stdin_bytes()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::parse_stdin_bytes
    pub vt_100_input_seq_parser: StatefulInputParser,

    /// Paste state machine for bracketed paste handling.
    ///
    /// - **Fed by**: [`parse_stdin_bytes()`] passes parsed events through this.
    /// - **Yields**: [`InputEvent`] after paste sequence handling.
    ///
    /// [`InputEvent`]: crate::InputEvent
    /// [`parse_stdin_bytes()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::parse_stdin_bytes
    pub paste_collection_state: PasteCollectionState,

    /// Broadcast channel sender for parsed events.
    ///
    /// - **Sent to by**: [`parse_stdin_bytes()`] and [`consume_pending_signals()`].
    /// - **Received by**: Async consumers via [`subscribe_to_input_events()`].
    ///
    /// [`consume_pending_signals()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_signals::consume_pending_signals
    /// [`parse_stdin_bytes()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::parse_stdin_bytes
    /// [`subscribe_to_input_events()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::global_input_resource::subscribe_to_input_events
    pub tx_parsed_input_events: InputEventSender,
}

impl MioPoller {
    /// Spawns the [`mio`] poller thread, which runs for the process lifetime.
    ///
    /// # Arguments
    ///
    /// - `sender`: Broadcast channel sender injected by the caller. This decouples the
    ///   poller thread from channel ownership. The caller creates the channel and retains
    ///   the receiver side for [`subscribe_to_input_events()`].
    ///
    /// This is the main entry point for starting the input handling system. It:
    /// 1. Spawns a dedicated thread named `"mio-poller"` -> useful for debugging, eg
    ///    using [`ps`] or [`htop`].
    /// 2. Registers [`stdin`] and [`SIGWINCH`] with [`mio`].
    /// 3. Runs the polling loop until exit condition is met.
    ///
    /// The thread is detached (its [`JoinHandle`] is dropped) - see the
    /// [Thread Lifecycle] section in the module docs for details.
    ///
    /// # Panics
    ///
    /// Panics if thread spawning or [`mio`] registration fails.
    ///
    /// [Thread Lifecycle]: super#thread-lifecycle
    /// [`JoinHandle`]: std::thread::JoinHandle
    /// [`htop`]: https://htop.dev/
    /// [`ps`]: https://man7.org/linux/man-pages/man1/ps.1.html
    /// [`stdin`]: std::io::stdin
    /// [`subscribe_to_input_events()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::global_input_resource::subscribe_to_input_events
    pub fn spawn_thread(sender: InputEventSender) {
        let _unused = std::thread::Builder::new()
            .name("mio-poller".into())
            .spawn(move || {
                let mut mio_poller = Self::setup(sender);
                mio_poller.start();
            })
            .expect("Failed to spawn mio poller thread");
    }

    /// Initializes the [`mio`] poller, registering [`stdin`] and [`SIGWINCH`].
    ///
    /// # Panics
    ///
    /// Panics if [`mio::Poll`] creation or registration fails.
    ///
    /// [`stdin`]: std::io::stdin
    #[must_use]
    pub fn setup(tx_parsed_input_events: InputEventSender) -> Self {
        let poll_handle = Poll::new().expect("Failed to create mio::Poll");
        let mio_registry = poll_handle.registry();

        // Register stdin with mio.
        let stdin = std::io::stdin();
        mio_registry
            .register(
                &mut SourceFd(&stdin.as_raw_fd()),
                SourceKindReady::Stdin.to_token(),
                Interest::READABLE,
            )
            .expect("Failed to register stdin with mio");

        // Register SIGWINCH with signal-hook-mio.
        let mut signals =
            Signals::new([SIGWINCH]).expect("Failed to register SIGWINCH handler");
        mio_registry
            .register(
                &mut signals,
                SourceKindReady::Signals.to_token(),
                Interest::READABLE,
            )
            .expect("Failed to register SIGWINCH with mio");

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
            tx_parsed_input_events,
        }
    }

    /// Runs the main event loop until exit condition is met.
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
    /// [`inotifywait`]: https://linux.die.net/man/1/inotifywait
    pub fn start(&mut self) {
        // Breaks borrow so dispatch can use `&mut self`.
        fn collect_ready_tokens(events: &Events) -> Vec<Token> {
            events.iter().map(mio::event::Event::token).collect()
        }

        loop {
            // Block until stdin or signals become ready.
            if let Err(err) = self.poll_handle.poll(&mut self.ready_events_buffer, None) {
                match err.kind() {
                    // EINTR ("Interrupted" — a signal arrived while the syscall was
                    // blocked). Retry poll. https://man7.org/linux/man-pages/man7/signal.7.html
                    ErrorKind::Interrupted => continue,
                    _ => {
                        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                            tracing::debug!(
                                message = "mio-poller-thread: poll error",
                                error = ?err
                            );
                        });
                        let _unused =
                            self.tx_parsed_input_events.send(ReaderThreadMessage::Error);
                        break;
                    }
                }
            }

            // Dispatch ready events.
            for token in collect_ready_tokens(&self.ready_events_buffer) {
                let source_kind = SourceKindReady::from_token(token);
                if dispatch(source_kind, self, token) == ThreadLoopContinuation::Return {
                    return;
                }
            }
        }
    }
}

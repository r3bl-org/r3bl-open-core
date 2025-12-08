// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR wakeup kqueue epoll ttimeoutlen

//! State machine for the [`mio`] poller thread. See [`MioPoller`] docs.

use super::{paste_state_machine::{PasteCollectionState, apply_paste_state_machine},
            stateful_parser::StatefulInputParser,
            types::{InputEventSender, PasteStateResult, ReaderThreadMessage,
                    ThreadLoopContinuation}};
use crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;
use mio::{Events, Interest, Poll, Token, unix::SourceFd};
use signal_hook::consts::SIGWINCH;
use signal_hook_mio::v1_0::Signals;
use std::{io::{ErrorKind, Read as _, Stdin},
          os::fd::AsRawFd as _};

/// Read buffer size for stdin reads (`1_024` bytes).
///
/// When `read_count == STDIN_READ_BUFFER_SIZE`, more data is likely waiting in the
/// kernel buffer—this is the `more` flag used for ESC disambiguation.
const STDIN_READ_BUFFER_SIZE: usize = 1_024;

/// Capacity for the [`mio::Events`] buffer.
///
/// [`mio::Events`]: mio::Events
const EVENTS_CAPACITY: usize = 8;

/// Registry of all event sources monitored by [`mio::Poll`].
///
/// This struct centralizes the management of heterogeneous event sources ([`stdin`],
/// [signals]) that are registered with [`mio`] for I/O multiplexing. Each source has a
/// corresponding [`Token`] in [`SourceKindReady`] for dispatch routing.
///
/// # What is a "Source"?
///
/// In [`mio`] terminology, a "source" is anything registered with [`Poll`] to be
/// monitored for readiness. When a source becomes readable, [`Poll::poll()`] returns
/// an event with the source's [`Token`], and we must consume data from that source.
///
/// # Design Rationale
///
/// While a [`HashMap<Token, Source>`] might seem appealing, the sources have different
/// types ([`Stdin`] vs [`Signals`]) with different consumption patterns:
/// - **[`Stdin`]**: Call [`read()`] to get bytes.
/// - **[`Signals`]**: Call [`pending()`] to drain signal queue.
///
/// This struct provides type safety while formalizing the token→source relationship.
///
/// # Adding New Sources
///
/// To add a new event source:
/// 1. Add a new field to this struct.
/// 2. Add a new variant and token constant to [`SourceKindReady`].
/// 3. Register the source in [`MioPoller::setup()`].
/// 4. Add a handler method in [`MioPoller`].
/// 5. Add a match arm in [`SourceKindReady::dispatch()`].
///
/// [`HashMap<Token, Source>`]: std::collections::HashMap
/// [`MioPoller::setup()`]: MioPoller::setup
/// [`MioPoller`]: MioPoller
/// [`Poll::poll()`]: mio::Poll::poll
/// [`Poll`]: mio::Poll
/// [`Signals`]: signal_hook_mio::v1_0::Signals
/// [`SourceKindReady::dispatch()`]: SourceKindReady::dispatch
/// [`Stdin`]: std::io::Stdin
/// [`Token`]: mio::Token
/// [`pending()`]: signal_hook_mio::v1_0::Signals::pending
/// [`read()`]: std::io::Read::read
/// [`stdin`]: std::io::stdin
/// [signals]: signal_hook_mio::v1_0::Signals
#[allow(missing_debug_implementations)]
pub struct SourceRegistry {
    /// [`Stdin`] handle registered with [`MioPoller::poll_handle`].
    ///
    /// See [What is a "Source"?] for [`mio`] terminology.
    ///
    /// - **Token**: [`SourceKindReady::Stdin`].[`to_token()`].
    /// - **Handler**: [`MioPoller::consume_stdin_input()`].
    ///
    /// [What is a "Source"?]: SourceRegistry#what-is-a-source
    /// [`MioPoller::consume_stdin_input()`]: MioPoller::consume_stdin_input
    /// [`MioPoller::poll_handle`]: MioPoller::poll_handle
    /// [`to_token()`]: SourceKindReady::to_token
    pub stdin: Stdin,

    /// [`SIGWINCH`] signal handler registered with [`MioPoller::poll_handle`].
    ///
    /// See [What is a "Source"?] for [`mio`] terminology. [`signal_hook_mio`] provides
    /// an adapter that creates an internal pipe becoming readable when [`SIGWINCH`]
    /// arrives.
    ///
    /// - **Token**: [`SourceKindReady::Signals`].[`to_token()`].
    /// - **Handler**: [`MioPoller::consume_pending_signals()`].
    ///
    /// [What is a "Source"?]: SourceRegistry#what-is-a-source
    /// [`MioPoller::consume_pending_signals()`]: MioPoller::consume_pending_signals
    /// [`MioPoller::poll_handle`]: MioPoller::poll_handle
    /// [`signal_hook_mio`]: signal_hook_mio
    /// [`to_token()`]: SourceKindReady::to_token
    pub signals: Signals,
}

/// Identifies which event source became ready.
///
/// This enum is the single source of truth for [`mio`] [`Token`] ↔ source mapping.
/// Each variant (except [`Unknown`]) has an associated token used for registration
/// and dispatch.
///
/// # How Tokens Work
///
/// When [`Poll::poll()`] returns, each event carries a [`Token`] identifying which
/// registered source became ready. Use [`from_token()`] to convert a token to this
/// enum, then match on the variant to dispatch to the appropriate handler.
///
/// [`Poll::poll()`]: mio::Poll::poll
/// [`Token`]: mio::Token
/// [`Unknown`]: SourceKindReady::Unknown
/// [`from_token()`]: SourceKindReady::from_token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKindReady {
    /// [`SourceRegistry::stdin`] has data available to read.
    Stdin,
    /// [`SourceRegistry::signals`] received [`SIGWINCH`].
    Signals,
    /// Unknown token - should not happen in normal operation.
    Unknown,
}

impl SourceKindReady {
    /// Returns the [`Token`] associated with this source kind.
    ///
    /// Used when registering sources with [`mio::Registry`]. This is the inverse
    /// of [`from_token()`].
    ///
    /// # Panics
    ///
    /// Panics if called on [`SourceKindReady::Unknown`].
    ///
    /// [`Token`]: mio::Token
    /// [`from_token()`]: SourceKindReady::from_token
    /// [`mio::Registry`]: mio::Registry
    pub const fn to_token(self) -> Token {
        match self {
            Self::Stdin => Token(0),
            Self::Signals => Token(1),
            Self::Unknown => panic!("Unknown source has no token"),
        }
    }

    /// Converts a [`Token`] to the corresponding [`SourceKindReady`] variant.
    ///
    /// This is the inverse of [`to_token()`]. Used when dispatching ready events
    /// from [`Poll::poll()`].
    ///
    /// [`Poll::poll()`]: mio::Poll::poll
    /// [`Token`]: mio::Token
    /// [`to_token()`]: SourceKindReady::to_token
    pub const fn from_token(token: Token) -> Self {
        match token.0 {
            0 => Self::Stdin,
            1 => Self::Signals,
            _ => Self::Unknown,
        }
    }

    /// Dispatches to the appropriate handler for this source kind.
    ///
    /// This centralizes the token→handler mapping, making it easier to add new
    /// sources—just add a variant and its match arm here.
    ///
    /// # Arguments
    ///
    /// - `poller`: The [`MioPoller`] containing the handler methods.
    /// - `token`: The original [`Token`] for diagnostic logging on unknown tokens.
    ///
    /// # Returns
    ///
    /// - [`ThreadLoopContinuation::Continue`]: Event handled, continue polling.
    /// - [`ThreadLoopContinuation::Return`]: Exit condition met.
    pub fn dispatch(
        self,
        poller: &mut MioPoller,
        token: Token,
    ) -> ThreadLoopContinuation {
        match self {
            Self::Stdin => poller.consume_stdin_input(),
            Self::Signals => poller.consume_pending_signals(),
            Self::Unknown => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::warn!(
                        message = "mio-poller-thread: unknown token",
                        token = ?token
                    );
                });
                ThreadLoopContinuation::Continue
            }
        }
    }
}

/// This struct encapsulates all state and logic for the [`mio`] poller thread that it
/// [spawns]. It owns and manages the following:
///
/// | Resource       | Responsibility                                                                                                                      |
/// |:-------------- |:----------------------------------------------------------------------------------------------------------------------------------- |
/// | **Poll**       | Wait efficiently for [`stdin`] data and [`SIGWINCH`] signals                                                                        |
/// | **Stdin**      | Read bytes into buffer -> handle using [VT100 input parser] and [paste state machine] to generate [`ReaderThreadMessage::Event`]    |
/// | **Signals**    | Drain signals and generate [`ReaderThreadMessage::Resize`]                                                                          |
/// | **Channel**    | Publish [`ReaderThreadMessage`] variants to async consumers                                                                         |
///
/// # How It Works
///
/// Our design separates these two:
///
/// 1. **Blocking I/O** (the [`mio`] thread owns [`stdin`] exclusively).
/// 2. **Async consumption** ([`tokio`] tasks await on channel). The
///    [`tokio::sync::broadcast`] channel bridges sync and async worlds, supporting
///    multiple consumers that each receive all events.
///
/// The sections below explain each component in detail.
///
/// ## The [`mio`]-poller Thread
///
/// A dedicated [`std::thread`] runs for the process lifetime, using [`mio::Poll`] to
/// efficiently wait on multiple file descriptors:
///
/// ```text
/// ┌────────────────────────────────────┐           ┌─────────────────────────────────┐
/// │ Dedicated Thread (std::thread)     │           │ Async Consumers (tokio runtime) │
/// │                                    │           │                                 │
/// │ mio::Poll waits on:                ├───────────▶ rx.recv().await (fan-out)       │
/// │   • stdin fd (Token 0)             │ broadcast │                                 │
/// │   • SIGWINCH signal (Token 1)      │           │                                 │
/// └────────────────────────────────────┘           └─────────────────────────────────┘
/// ```
///
/// ### Thread Lifecycle
///
/// The dedicated thread can't be terminated or cancelled, and it safely owns [`stdin`]
/// exclusively. The OS is responsible for cleaning it up when the process exits.
///
/// | Exit Mechanism                | How Thread Exits                                |
/// | ----------------------------- | ----------------------------------------------- |
/// | Ctrl+C / `SIGINT`             | OS terminates process → all threads killed      |
/// | [`std::process::exit()`]      | OS terminates process → all threads killed      |
/// | `main()` returns              | Rust runtime exits → OS terminates process      |
/// | [`stdin`] EOF                 | `read()` returns 0 → thread exits naturally     |
///
/// This is ok because:
/// - [`INPUT_RESOURCE`] lives forever - it's a [`LazyLock`]`<...>` static, never dropped
///   until process exit.
/// - Thread is doing nothing when blocked - [`mio`] uses efficient OS primitives.
/// - No resources to leak - [`stdin`] is `fd` `0`, not owned by us.
///
/// The thread self-terminates gracefully in these scenarios:
/// - **EOF on [`stdin`]**: When [`stdin`] is closed (e.g., pipe closed, `Ctrl+D`),
///   `read()` returns 0 bytes. The thread sends [`ReaderThreadMessage::Eof`] and exits.
/// - **I/O error**: On read errors (except `EINTR` which is retried), the thread sends
///   [`ReaderThreadMessage::Error`] and exits.
/// - **Receiver dropped**: When [`INPUT_RESOURCE`] is dropped (process exit), the channel
///   receiver is dropped. The next `tx.send()` returns `Err`, and the thread exits
///   gracefully.
///
/// ## What is [`mio`]?
///
/// [`mio`] provides **synchronous I/O multiplexing** - a thin wrapper around OS
/// primitives:
/// - **Linux**: [`epoll`]
/// - **macOS**: [`kqueue`]
///
/// It's *blocking* but efficient - `poll.poll(&mut events, None)` blocks the thread until
/// something happens on either fd. Unlike [`select()`] or raw [`poll()`], mio uses the
/// optimal syscall per platform.
///
/// **Why not tokio for stdin?** Because [`tokio::io::stdin()`] uses a blocking threadpool
/// internally, and cancelling a [`tokio::select!`] branch doesn't stop the underlying
/// read - it keeps running as a "zombie", causing the problems described in
/// [`global_input_resource`].
///
/// ## The Two File Descriptors
///
/// A file descriptor (fd) is a Unix integer handle to an I/O resource (file, socket,
/// pipe, etc.). Two fds are registered with [`mio`]'s registry so a single `poll()` call
/// can wait on either becoming ready:
///
/// **1. `stdin` fd** - The raw file descriptor (fd 0) for standard input, obtained via
/// `std::io::stdin().as_raw_fd()`. We wrap it in [`SourceFd`] so mio can poll it:
///
/// <!-- It is ok to use ignore here -->
/// ```ignore
/// registry.register(&mut SourceFd(&stdin_fd), SourceKindReady::Stdin.to_token(), Interest::READABLE)
/// ```
///
/// **2. Signal watcher fd** - Signals aren't file descriptors, so [`signal_hook_mio`]
/// provides a clever adapter: it creates an internal pipe that becomes readable when
/// `SIGWINCH` arrives. This lets [`mio`] wait on signals just like any other fd:
///
/// <!-- It is ok to use ignore here -->
/// ```ignore
/// let mut signals = Signals::new([SIGWINCH])?;  // Creates internal pipe
/// registry.register(&mut signals, SourceKindReady::Signals.to_token(), Interest::READABLE)
/// ```
///
/// ## Parsing and the Channel
///
/// When bytes arrive from [`stdin`], they flow through a parsing pipeline:
///
/// ```text
/// Raw bytes → Parser::advance() → VT100InputEventIR → Paste state machine → InputEvent → Channel
/// ```
///
/// The parser handles three tricky cases:
/// - **`ESC` disambiguation**: The `more` flag indicates if more bytes might be waiting.
///   If `read_count == buffer_size`, we wait before deciding a lone `ESC` is the `ESC`
///   key.
/// - **Chunked input**: The buffer accumulates bytes until a complete sequence is parsed.
/// - **UTF-8**: Multi-byte characters can span multiple reads.
///
/// The channel sends [`ReaderThreadMessage`] variants to the async side:
/// - [`Event(InputEvent)`] - parsed keyboard/mouse input
/// - [`Resize`] - terminal window changed size
/// - [`Eof`] - stdin closed
/// - [`Error`] - I/O error
///
/// # Why [`mio`] Instead of Raw [`poll()`]?
///
/// We use [`mio`] instead of raw [`poll()`] or [`rustix::event::poll()`] because:
///
/// - **macOS compatibility**: [`poll()`] cannot monitor `/dev/tty` on macOS, but [`mio`]
///   uses [`kqueue`] which works correctly.
/// - **Platform abstraction**: [`mio`] uses the optimal syscall per platform ([`epoll`]
///   on Linux, [`kqueue`] on macOS/BSD).
///
/// # ESC Detection Limitations
///
/// Both the `ESC` key and escape sequences (like Up Arrow = `ESC [ A`) start with the
/// same byte (`1B`). When we read a lone `ESC` byte, is it the `ESC` key or the start of
/// a sequence?
///
/// ## The `more` Flag Heuristic
///
/// We use [`crossterm`]'s `more` flag pattern: `more = (read_count == buffer_size)`. The
/// idea is that if `read()` fills the entire buffer, more data is probably waiting in
/// the kernel. So:
///
/// - `more == true` + lone `ESC` → wait (might be start of escape sequence)
/// - `more == false` + lone `ESC` → emit `ESC` key (no more data waiting)
///
/// ## Why This is a Heuristic, Not a Guarantee
///
/// **This approach assumes that if `read()` returns fewer bytes than the buffer size, all
/// pending data has been consumed.** This is usually true, but not guaranteed:
///
/// - **Local terminals**: Escape sequences are typically written atomically, so they
///   arrive complete in one `read()`. The heuristic works well.
/// - **Over [SSH]**: TCP can fragment data arbitrarily. If `ESC` arrives in one packet
///   and `[ A` in the next (even microseconds later), we might incorrectly emit `ESC`.
/// - **High latency networks**: The more latency and packet fragmentation, the higher the
///   chance of incorrect `ESC` detection.
///
/// ## Why Not Use a Timeout Like `vim`?
///
/// Vim uses a [100ms `ttimeoutlen` delay] - if no more bytes arrive within 100ms after
/// `ESC`, it's the `ESC` key. This is more reliable but adds latency to every `ESC`
/// keypress.
///
/// We chose the `more` flag heuristic (following [`crossterm`]) because:
/// - Zero latency for `ESC` key in the common case (local terminal).
/// - Acceptable behavior for most [SSH] connections (TCP usually delivers related bytes
///   together). In our testing there were no issues over [SSH].
/// - The failure mode (`ESC` emitted early) is annoying but not catastrophic.
///
/// **Trade-off**: Faster `ESC` response vs. occasional incorrect detection on
/// high-latency connections.
///
/// [spawns]: MioPoller::spawn_thread
/// [`stdin`]: std::io::stdin
/// [`mio::Poll`]: mio::Poll
/// [`mio`]: mio
/// [`tokio::sync::broadcast`]: tokio::sync::broadcast
/// [`std::thread`]: std::thread
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
/// [`select()`]: https://man7.org/linux/man-pages/man2/select.2.html
/// [`poll()`]: https://man7.org/linux/man-pages/man2/poll.2.html
/// [`tokio::io::stdin()`]: tokio::io::stdin
/// [`tokio::select!`]: tokio::select
/// [`global_input_resource`]: super::global_input_resource
/// [`INPUT_RESOURCE`]: super::global_input_resource::INPUT_RESOURCE
/// [`LazyLock`]: std::sync::LazyLock
/// [`SourceFd`]: mio::unix::SourceFd
/// [`signal_hook_mio`]: signal_hook_mio
/// [`rustix::event::poll()`]: rustix::event::poll
/// [`crossterm`]: ::crossterm
/// [`Event(InputEvent)`]: ReaderThreadMessage::Event
/// [`Resize`]: ReaderThreadMessage::Resize
/// [`Eof`]: ReaderThreadMessage::Eof
/// [`Error`]: ReaderThreadMessage::Error
/// [100ms `ttimeoutlen` delay]:
///     https://vi.stackexchange.com/questions/24925/usage-of-timeoutlen-and-ttimeoutlen
/// [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
/// [VT100 input parser]: super::stateful_parser::StatefulInputParser
/// [paste state machine]: super::paste_state_machine::PasteCollectionState
/// [sender]: super::types::InputEventSender
#[allow(missing_debug_implementations)]
pub struct MioPoller {
    /// [`mio`] poll instance for efficient I/O multiplexing.
    ///
    /// - **Registered sources**: [`SourceRegistry::stdin`], [`SourceRegistry::signals`].
    /// - **Used by**: [`start()`] blocks on this until sources are ready.
    ///
    /// [`start()`]: MioPoller::start
    poll_handle: Poll,

    /// Buffer for events returned by [`Poll::poll()`].
    ///
    /// - **Populated by**: [`Poll::poll()`] fills this when [`std::io::stdin`] or
    ///   [`SIGWINCH`] becomes ready.
    /// - **Drained by**: [`start()`] iterates and dispatches to token-specific handlers
    ///   via [`SourceKindReady::dispatch()`].
    ///
    /// [`SourceKindReady::dispatch()`]: SourceKindReady::dispatch
    /// [`start()`]: MioPoller::start
    ready_events_buffer: Events,

    /// Registry of all event sources monitored by [`poll_handle`].
    ///
    /// Centralizes management of heterogeneous sources (stdin, signals) and provides
    /// the token→source dispatch logic via [`SourceRegistry::dispatch()`].
    ///
    /// [`SourceRegistry::dispatch()`]: SourceRegistry::dispatch
    /// [`poll_handle`]: MioPoller::poll_handle
    sources: SourceRegistry,

    /// Buffer for reading unparsed bytes from [`std::io::stdin()`].
    ///
    /// - **Written by**: [`consume_stdin_input()`] reads into this buffer.
    /// - **Consumed by**: [`parse_stdin_bytes()`] parses bytes from here.
    ///
    /// [`consume_stdin_input()`]: MioPoller::consume_stdin_input
    /// [`parse_stdin_bytes()`]: MioPoller::parse_stdin_bytes
    stdin_unparsed_byte_buffer: [u8; STDIN_READ_BUFFER_SIZE],

    /// Stateful VT100 input sequence parser.
    ///
    /// - **Fed by**: [`parse_stdin_bytes()`] calls `advance()` with raw bytes.
    /// - **Yields**: [`VT100InputEventIR`] events via [`Iterator`] impl.
    ///
    /// [`VT100InputEventIR`]: crate::VT100InputEventIR
    /// [`parse_stdin_bytes()`]: MioPoller::parse_stdin_bytes
    vt_100_input_seq_parser: StatefulInputParser,

    /// Paste state machine for bracketed paste handling.
    ///
    /// - **Fed by**: [`parse_stdin_bytes()`] passes parsed events through this.
    /// - **Yields**: [`InputEvent`] after paste sequence handling.
    ///
    /// [`InputEvent`]: crate::InputEvent
    /// [`parse_stdin_bytes()`]: MioPoller::parse_stdin_bytes
    paste_collection_state: PasteCollectionState,

    /// Broadcast channel sender for parsed events.
    ///
    /// - **Sent to by**: [`parse_stdin_bytes()`] and [`consume_pending_signals()`].
    /// - **Received by**: Async consumers via [`subscribe_to_input_events()`].
    ///
    /// [`consume_pending_signals()`]: MioPoller::consume_pending_signals
    /// [`parse_stdin_bytes()`]: MioPoller::parse_stdin_bytes
    /// [`subscribe_to_input_events()`]: super::global_input_resource::subscribe_to_input_events
    tx_parsed_input_events: InputEventSender,
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
    /// [Thread Lifecycle] section above for details.
    ///
    /// # Panics
    ///
    /// Panics if thread spawning or [`mio`] registration fails.
    ///
    /// [Thread Lifecycle]: MioPoller#thread-lifecycle
    /// [`JoinHandle`]: std::thread::JoinHandle
    /// [`htop`]: https://htop.dev/
    /// [`ps`]: https://man7.org/linux/man-pages/man1/ps.1.html
    /// [`stdin`]: std::io::stdin
    /// [`subscribe_to_input_events()`]: super::global_input_resource::subscribe_to_input_events
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
    /// Note that the call to [`Poll::poll()`] is what [`inotifywait`] uses under the
    /// hood. The `timeout` set to `None` ensures that this will block. If we set a
    /// delay here, then the loop will continue after that delay and act as a
    /// busy-wait. This is similar to what `check.fish` does to implement a sliding
    /// window debounce for file changes with [`inotifywait`].
    ///
    /// [`inotifywait`]: https://linux.die.net/man/1/inotifywait
    pub fn start(&mut self) {
        // Breaks borrow so dispatch can use `&mut self`.
        fn collect_ready_tokens(events: &Events) -> Vec<Token> {
            events.iter().map(|it| it.token()).collect()
        }

        loop {
            // Block until stdin or signals become ready.
            if let Err(err) = self.poll_handle.poll(&mut self.ready_events_buffer, None) {
                match err.kind() {
                    ErrorKind::Interrupted => continue, // EINTR - retry poll.
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
                if SourceKindReady::from_token(token).dispatch(self, token)
                    == ThreadLoopContinuation::Return
                {
                    return;
                }
            }
        }
    }

    /// Handles [`stdin`] becoming readable.
    ///
    /// Reads bytes from [`stdin`], parses them into [`VT100InputEventIR`] events, applies
    /// the paste state machine, and sends final events to the channel.
    ///
    /// # Returns
    ///
    /// - [`ThreadLoopContinuation::Continue`]: Successfully processed or recoverable
    ///   error
    /// - [`ThreadLoopContinuation::Return`]: EOF, fatal error, or receiver dropped
    ///
    /// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
    /// [`stdin`]: std::io::stdin
    pub fn consume_stdin_input(&mut self) -> ThreadLoopContinuation {
        let read_res = self
            .sources
            .stdin
            .read(&mut self.stdin_unparsed_byte_buffer);
        match read_res {
            Ok(0) => {
                // EOF reached.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(message = "mio-poller-thread: EOF (0 bytes)");
                });
                let _unused = self.tx_parsed_input_events.send(ReaderThreadMessage::Eof);
                ThreadLoopContinuation::Return
            }

            Ok(n) => self.parse_stdin_bytes(n),

            Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                // EINTR - will retry on next poll iteration.
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
                let _unused =
                    self.tx_parsed_input_events.send(ReaderThreadMessage::Error);
                ThreadLoopContinuation::Return
            }
        }
    }

    /// Parses bytes read from stdin into input events.
    ///
    /// Parses bytes into VT100 events and sends them through the paste state machine.
    pub fn parse_stdin_bytes(&mut self, n: usize) -> ThreadLoopContinuation {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: read bytes", bytes_read = n);
        });

        // `more` flag for ESC disambiguation.
        let more = n == STDIN_READ_BUFFER_SIZE;

        // Parse bytes into events.
        self.vt_100_input_seq_parser
            .advance(&self.stdin_unparsed_byte_buffer[..n], more);

        // Process all parsed events through paste state machine.
        for vt100_event in self.vt_100_input_seq_parser.by_ref() {
            match apply_paste_state_machine(
                &mut self.paste_collection_state,
                &vt100_event,
            ) {
                PasteStateResult::Emit(input_event) => {
                    if self
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

    /// Handles SIGWINCH signal (terminal resize).
    ///
    /// Drains all pending signals and sends a single resize event to the channel.
    /// Multiple coalesced signals result in one event since resize is idempotent—the
    /// consumer queries the current terminal size regardless of how many signals arrived.
    ///
    /// # Returns
    ///
    /// - [`ThreadLoopContinuation::Continue`]: Successfully processed
    /// - [`ThreadLoopContinuation::Return`]: Receiver dropped
    pub fn consume_pending_signals(&mut self) -> ThreadLoopContinuation {
        // Drain all pending signals and check if any SIGWINCH arrived.
        // Multiple signals may coalesce between polls, but we only need one Resize event.
        let sigwinch_arrived = self.sources.signals.pending().any(|sig| sig == SIGWINCH);

        if sigwinch_arrived {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "mio-poller-thread: SIGWINCH received");
            });
            if self
                .tx_parsed_input_events
                .send(ReaderThreadMessage::Resize)
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

        ThreadLoopContinuation::Continue
    }
}

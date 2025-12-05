// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH kqueue epoll wakeup eventfd ttimeoutlen

//! Global singleton (process bound) for terminal input with a dedicated reader thread.
//!
//! This module addresses three problems with terminal input in async Rust:
//! 1. **UI freezes** on terminal resize when using [`tokio::io::stdin()`]
//! 2. **Dropped keystrokes** when transitioning between TUI apps
//! 3. **Flawed `ESC` detection** over [SSH] (separate issue, see [ESC Detection
//!    Limitations](#esc-detection-limitations))
//!
//! The solution: a dedicated [`mio`]-based thread that owns [`stdin`] exclusively (using
//! sync code) and communicates with async code via channel.
//!
//! # Why This Design? (Historical Context)
//!
//! Our original "Tokio-heavy" approach created [`DirectToAnsiInputDevice`] on demand—one
//! instance per app, not process-bound. It used:
//! - [`tokio::io::stdin()`] for input handling
//! - [`tokio::signal`] for `SIGWINCH` handling
//!
//! **This caused three problems that led us to the current design:**
//!
//! **Problem 1: UI freeze on resize.** [Tokio's stdin] uses a blocking threadpool. In the
//! past, in [`DirectToAnsiInputDevice::try_read_event()`], when [`tokio::select!`]
//! cancelled a [`tokio::io::stdin()`] read to handle `SIGWINCH`, the blocking read kept
//! running in the background. The next read conflicted with this "zombie" read → UI
//! freeze.
//!
//! **Problem 2: Dropped keystrokes.** Creating a new [`stdin`] handle lost access to data
//! already in the kernel buffer. When TUI "App A" exited and "App B" started, keystrokes
//! typed during the transition vanished. This was easily reproducible by running `cargo
//! run --examples tui_apps`, starting one app, exiting, starting another, exiting, etc.
//!
//! **Problem 3: Flawed `ESC` detection over [SSH].** Our original approach had flawed
//! logic for distinguishing the `ESC` key from escape sequences (like `ESC [ A` for Up
//! Arrow). It worked locally but failed over [SSH]. We now use [`crossterm`]'s `more`
//! flag heuristic (see [ESC Detection Limitations](#esc-detection-limitations)).
//!
//! # The Solution
//!
//! A **process bound global singleton** with a dedicated reader thread. The thread
//! exclusively owns the [`stdin`] handle and uses [`mio::Poll`] to efficiently wait on
//! both [`stdin`] and `SIGWINCH` signals. Although sync and blocking, [`mio`] is
//! efficient - it uses OS primitives ([`epoll`]/[`kqueue`]) that put the thread to sleep
//! until data arrives, consuming no CPU while waiting (see [How It Works](#how-it-works)
//! for details):
//!
//! ```text
//!   Process-bound Global Singleton               Your Code
//! ┌─────────────────────────────────────┐      ┌─────────────────────────────┐
//! │ Sync Blocking (std::thread + mio)   │      │ Async Code (tokio)          │
//! │                                     │      │                             │
//! │ Owns exclusively:                   │      │                             │
//! │   • stdin handle (locked)           │      │                             │
//! │   • Parser state                    │      │                             │
//! │   • SIGWINCH watcher                │      │                             │
//! │                                     │      │                             │
//! │ tx.send(InputEvent)  ───────────────┼──────▶ stdin_rx.recv().await       │
//! │                                     │ mpsc │ (cancel-safe!)              │
//! └─────────────────────────────────────┘   │  └─────────────────────────────┘
//!                                           ▼
//!                                   Sync -> Async Bridge
//! ```
//!
//! This solves the first two problems completely:
//! - **Cancel-safe**: Channel receive is truly async - no zombie reads
//! - **Data preserved**: Global state survives TUI app transitions
//!
//! For `ESC` detection, we use [`crossterm`]'s `more` flag heuristic (see [ESC Detection
//! Limitations](#esc-detection-limitations) below).
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │ GLOBAL_INPUT_RESOURCE (static LazyLock<Mutex<...>>)                     │
//! │   • stdin_rx: channel receiver ← mio thread (process lifetime)          │
//! │   • event_queue: VecDeque (buffered events preserved)                   │
//! └───────────────────────────────────────┬─────────────────────────────────┘
//!                                         │
//!            ┌────────────────────────────┴────────────────────────┐
//!            │                                                     │
//! ┌──────────▼───────────────────┐           ┌─────────────────────▼────────┐
//! │ DirectToAnsiInputDevice A    │           │ DirectToAnsiInputDevice B    │
//! │   (TUI App context)          │           │   (Readline context)         │
//! │   • Zero-sized handle        │           │   • Zero-sized handle        │
//! │   • Delegates to global      │           │   • Delegates to global      │
//! └──────────────────────────────┘           └──────────────────────────────┘
//!
//! 🎉 Data preserved during transitions - same channel used throughout!
//! ```
//!
//! The key insight: `stdin` handles must persist across device lifecycle boundaries.
//! Multiple [`DirectToAnsiInputDevice`] instances can be created and dropped, but they
//! all share the same underlying channel and reader thread.
//!
//! # How It Works
//!
//! Our design separates **blocking I/O** (the [`mio`] thread owns stdin exclusively) from
//! **async consumption** (tokio tasks await on channel). The [`tokio::sync::mpsc`]
//! channel bridges sync and async worlds - it's designed for "send from sync, receive
//! from async".
//!
//! ## 1. The mio-poller Thread
//!
//! A dedicated `std::thread` runs for the process lifetime, using [`mio::Poll`] to
//! efficiently wait on multiple file descriptors:
//!
//! ```text
//! ┌─────────────────────────────────────┐      ┌─────────────────────────────────┐
//! │ Dedicated Thread (std::thread)      │      │ Async Code (tokio runtime)      │
//! │                                     │      │                                 │
//! │ mio::Poll waits on:                 ├──────▶   stdin_rx.recv().await         │
//! │   • stdin fd (Token 0)              │ mpsc │                                 │
//! │   • SIGWINCH signal (Token 1)       │      │                                 │
//! └─────────────────────────────────────┘      └─────────────────────────────────┘
//! ```
//!
//! ## 2. What is mio?
//!
//! [`mio`] provides **synchronous I/O multiplexing** - a thin wrapper around OS
//! primitives:
//! - **Linux**: [`epoll`]
//! - **macOS**: [`kqueue`]
//!
//! It's *blocking* but efficient - `poll.poll(&mut events, None)` blocks the thread until
//! something happens on either fd. Unlike [`select()`] or raw [`poll()`], mio uses the
//! optimal syscall per platform.
//!
//! **Why not tokio for stdin?** Because [`tokio::io::stdin()`] uses a blocking threadpool
//! internally, and cancelling a [`tokio::select!`] branch doesn't stop the underlying
//! read - it keeps running as a "zombie", causing the problems described above.
//!
//! ## 3. The Two File Descriptors
//!
//! A file descriptor (fd) is a Unix integer handle to an I/O resource (file, socket,
//! pipe, etc.). Two fds are registered with [`mio`]'s registry so a single `poll()` call
//! can wait on either becoming ready:
//!
//! **1. `stdin` fd** - The raw file descriptor (fd 0) for standard input, obtained via
//! `std::io::stdin().as_raw_fd()`. We wrap it in [`SourceFd`] so mio can poll it:
//!
//! <!-- It is ok to use ignore here -->
//! ```ignore
//! registry.register(&mut SourceFd(&stdin_fd), STDIN_TOKEN, Interest::READABLE)
//! ```
//!
//! **2. Signal watcher fd** - Signals aren't file descriptors, so [`signal_hook_mio`]
//! provides a clever adapter: it creates an internal pipe that becomes readable when
//! `SIGWINCH` arrives. This lets [`mio`] wait on signals just like any other fd:
//!
//! <!-- It is ok to use ignore here -->
//! ```ignore
//! let mut signals = Signals::new([SIGWINCH])?;  // Creates internal pipe
//! registry.register(&mut signals, SIGNAL_TOKEN, Interest::READABLE)
//! ```
//!
//! ## 4. Parsing and the Channel
//!
//! When bytes arrive from stdin, they flow through a parsing pipeline:
//!
//! ```text
//! Raw bytes → Parser::advance() → VT100InputEventIR → Paste state machine → InputEvent → Channel
//! ```
//!
//! The parser handles three tricky cases:
//! - **ESC disambiguation**: The `more` flag indicates if more bytes might be waiting. If
//!   `read_count == buffer_size`, we wait before deciding a lone ESC is the ESC key.
//! - **Chunked input**: The buffer accumulates bytes until a complete sequence is parsed.
//! - **UTF-8**: Multi-byte characters can span multiple reads.
//!
//! The channel sends [`ReaderThreadMessage`] variants to the async side:
//! - [`Event(InputEvent)`] - parsed keyboard/mouse input
//! - [`Resize`] - terminal window changed size
//! - [`Eof`] - stdin closed
//! - [`Error`] - I/O error
//!
//! # Why mio Instead of Raw [`poll()`]?
//!
//! We use [`mio`] instead of raw [`poll()`] or [`rustix::event::poll()`] because:
//!
//! - **macOS compatibility**: [`poll()`] cannot monitor `/dev/tty` on macOS, but [`mio`]
//!   uses [`kqueue`] which works correctly
//! - **Platform abstraction**: [`mio`] uses the optimal syscall per platform ([`epoll`]
//!   on Linux, [`kqueue`] on macOS/BSD)
//!
//! # ESC Detection Limitations
//!
//! Both the `ESC` key and escape sequences (like Up Arrow = `ESC [ A`) start with the
//! same byte (`1B`). When we read a lone `ESC` byte, is it the `ESC` key or the start of
//! a sequence?
//!
//! ## The `more` Flag Heuristic
//!
//! We use [`crossterm`]'s `more` flag pattern: `more = (read_count == buffer_size)`. The
//! idea is that if `read()` filled the entire buffer, more data is probably waiting in
//! the kernel. So:
//!
//! - `more == true` + lone `ESC` → wait (might be start of escape sequence)
//! - `more == false` + lone `ESC` → emit `ESC` key (no more data waiting)
//!
//! ## Why This is a Heuristic, Not a Guarantee
//!
//! **This approach assumes that if `read()` returns fewer bytes than the buffer size, all
//! pending data has been consumed.** This is usually true, but not guaranteed:
//!
//! - **Local terminals**: Escape sequences are typically written atomically, so they
//!   arrive complete in one `read()`. The heuristic works well.
//! - **Over [SSH]**: TCP can fragment data arbitrarily. If `ESC` arrives in one packet
//!   and `[ A` in the next (even microseconds later), we might incorrectly emit `ESC`.
//! - **High latency networks**: The more latency and packet fragmentation, the higher the
//!   chance of incorrect `ESC` detection.
//!
//! ## Why Not Use a Timeout Like `vim`?
//!
//! Vim uses a [100ms `ttimeoutlen` delay] - if no more bytes arrive within 100ms after
//! `ESC`, it's the `ESC` key. This is more reliable but adds latency to every `ESC`
//! keypress.
//!
//! We chose the `more` flag heuristic (following [`crossterm`]) because:
//! - Zero latency for `ESC` key in the common case (local terminal).
//! - Acceptable behavior for most [SSH] connections (TCP usually delivers related bytes
//!   together). In our testing there were no issues over [SSH].
//! - The failure mode (`ESC` emitted early) is annoying but not catastrophic.
//!
//! **Trade-off**: Faster `ESC` response vs. occasional incorrect detection on
//! high-latency connections.
//!
//! # Thread Lifecycle
//!
//! The dedicated thread can't be terminated or cancelled, so it safely owns stdin
//! exclusively. The OS is responsible for cleaning it up when the process exits.
//!
//! | Exit Mechanism             | How Thread Exits                             |
//! | -------------------------- | -------------------------------------------- |
//! | Ctrl+C / `SIGINT`          | OS terminates process → all threads killed   |
//! | [`std::process::exit()`]   | OS terminates process → all threads killed   |
//! | `main()` returns           | Rust runtime exits → OS terminates process   |
//! | `stdin` EOF                | `read()` returns 0 → thread exits naturally  |
//!
//! This is ok because:
//! - [`GLOBAL_INPUT_RESOURCE`] lives forever - it's a [`LazyLock`]`<...>` static, never
//!   dropped until process exit.
//! - Thread is doing nothing when blocked - [`mio`] uses efficient OS primitives
//! - No resources to leak - stdin is fd 0, not owned by us
//!
//! The thread self-terminates gracefully in these scenarios:
//! - **EOF on stdin**: When stdin is closed (e.g., pipe closed, Ctrl+D), `read()` returns
//!   0 bytes. The thread sends [`ReaderThreadMessage::Eof`] and exits.
//! - **I/O error**: On read errors (except `EINTR` which is retried), the thread sends
//!   [`ReaderThreadMessage::Error`] and exits.
//! - **Receiver dropped**: When [`GLOBAL_INPUT_RESOURCE`] is dropped (process exit), the
//!   channel receiver is dropped. The next `tx.send()` returns `Err`, and the thread
//!   exits gracefully.
//!
//! # Attribution: [`crossterm`]
//!
//! This implementation is based on [`crossterm`]'s architecture:
//!
//! - **Global state pattern**: [`crossterm`] uses a global [`INTERNAL_EVENT_READER`] that
//!   holds the tty file descriptor and event buffer, ensuring data in the kernel buffer
//!   is not lost when [`EventStream`] instances are created and dropped.
//! - **[`mio`]-based polling**: Their `mio.rs` uses [`mio::Poll`] with `signal-hook-mio`
//!   for `SIGWINCH` and we do the same.
//! - **ESC disambiguation**: The `more` flag heuristic for distinguishing ESC key from
//!   escape sequences without timeouts. We inherit both its benefits (zero latency) and
//!   limitations (see [ESC Detection Limitations](#esc-detection-limitations)).
//! - **Process-lifetime cleanup**: Both implementations rely on OS cleanup at process
//!   exit rather than explicit thread termination.
//!
//! # Data Flow Diagram
//!
//! See the [Data Flow Diagram] section in [`DirectToAnsiInputDevice`] for the complete
//! data flow showing how [`try_read_event()`] interacts with this global resource.
//!
//! # Why [`tokio::sync::Mutex`] (Not [`std::sync::Mutex`])
//!
//! We hold the mutex guard across `.await` points (during `stdin_rx.recv().await`):
//! - [`std::sync::MutexGuard`] is `!Send` and cannot be held across `.await` points
//! - [`tokio::sync::Mutex`] is async-native and yields to scheduler instead of blocking
//! - This prevents starving other tokio tasks while waiting for the lock
//!
//! [Tokio's stdin]: tokio::io::stdin
//! [100ms `ttimeoutlen` delay]:
//!     https://vi.stackexchange.com/questions/24925/usage-of-timeoutlen-and-ttimeoutlen
//! [`EventStream`]: ::crossterm::event::EventStream
//! [`INTERNAL_EVENT_READER`]:
//!     https://github.com/crossterm-rs/crossterm/blob/0.29.0/src/event.rs#L149
//! [crossterm]: ::crossterm
//! [`DirectToAnsiInputDevice::try_read_event()`]:
//!     super::input_device::DirectToAnsiInputDevice::try_read_event
//! [`DirectToAnsiInputDevice`]: super::input_device::DirectToAnsiInputDevice
//! [Data Flow Diagram]: super::input_device::DirectToAnsiInputDevice#data-flow-diagram
//! [`try_read_event()`]: super::input_device::DirectToAnsiInputDevice::try_read_event
//! [`LazyLock`]: std::sync::LazyLock
//! [`std::io::stdin()`]: std::io::stdin
//! [`std::process::exit()`]: std::process::exit
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::select!`]: tokio::select
//! [`tokio::sync::mpsc`]: tokio::sync::mpsc
//! [`stdin`]: std::io::stdin
//! [`mio`]: mio
//! [`mio::Poll`]: mio::Poll
//! [`signal-hook`]: signal_hook
//! [`signal-hook-mio`]: signal_hook_mio
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`select()`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`poll()`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`rustix::event::poll()`]: rustix::event::poll
//! [`SourceFd`]: mio::unix::SourceFd
//! [`Event(InputEvent)`]: ReaderThreadMessage::Event
//! [`Resize`]: ReaderThreadMessage::Resize
//! [`Eof`]: ReaderThreadMessage::Eof
//! [`Error`]: ReaderThreadMessage::Error
//! [`readline_async`]: mod@crate::readline_async
//! [SSH]: https://en.wikipedia.org/wiki/Secure_Shell

use super::{paste_state_machine::{PasteCollectionState, apply_paste_state_machine},
            stateful_parser::Parser,
            types::{LoopContinuationSignal, ReaderThreadMessage}};
use crate::{InputEvent, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use mio::{Events, Interest, Poll, Token, unix::SourceFd};
use mio_poller_thread::StdinReceiver;
use signal_hook::consts::SIGWINCH;
use signal_hook_mio::v1_0::Signals;
use std::{collections::VecDeque,
          io::{ErrorKind, Read as _},
          os::fd::AsRawFd as _,
          sync::LazyLock};

/// Global static singleton for input reader state - persists for process lifetime.
///
/// See the [module-level documentation](self) for details on why global state is
/// necessary and how the dedicated stdin reader thread works.
///
/// Note: `SIGWINCH` handling is now done in the dedicated reader thread via
/// [`mio::Poll`] and [`signal_hook_mio`], not via [`tokio::signal`]. This means
/// resize events arrive through the same channel as stdin data, as
/// [`ReaderThreadMessage::Resize`].
///
/// [`mio::Poll`]: mio::Poll
/// [`signal_hook_mio`]: signal_hook_mio
/// [`tokio::signal`]: tokio::signal
#[allow(missing_debug_implementations)]
pub struct DirectToAnsiInputResource {
    /// Receiver for events from the dedicated mio poller thread.
    ///
    /// This channel receives [`ReaderThreadMessage`] from a dedicated thread that
    /// uses [`mio::Poll`] to wait on both `stdin` and `SIGWINCH` signals.
    ///
    /// [`mio::Poll`]: mio::Poll
    pub stdin_rx: StdinReceiver,

    /// Buffered events that haven't been consumed yet.
    ///
    /// When multiple events arrive in quick succession, extras are queued here.
    /// Pre-allocated with capacity 32 for typical burst scenarios.
    pub event_queue: VecDeque<InputEvent>,
}

/// Global singleton - initialized on first access.
///
/// Uses [`LazyLock`] for thread-safe lazy initialization and [`tokio::sync::Mutex`]
/// for async-safe access. The [`Option`] allows initialization to happen on first
/// access.
pub static GLOBAL_INPUT_RESOURCE: LazyLock<
    tokio::sync::Mutex<Option<DirectToAnsiInputResource>>,
> = LazyLock::new(|| tokio::sync::Mutex::new(None));

/// Gets or initializes the global input resource.
///
/// On first call, spawns the dedicated reader thread that uses [`mio::Poll`] to wait
/// on both stdin and `SIGWINCH` signals. Creates the parse buffer and event queue.
/// Subsequent calls return a guard to the existing state.
///
/// # Reader Thread
///
/// The dedicated thread is spawned on first call and runs for the process lifetime.
/// It uses [`mio::Poll`] to efficiently wait on:
/// - **stdin**: Registered via [`SourceFd`] with `STDIN_TOKEN`
/// - **SIGWINCH**: Registered via [`signal_hook_mio::v1_0::Signals`] with `SIGNAL_TOKEN`
///
/// Results are sent through the channel stored in
/// [`DirectToAnsiInputResource::stdin_rx`]. See the [module-level documentation](self)
/// for the full architecture.
///
/// # Panics
///
/// Panics if:
/// - [`mio::Poll`] cannot be created
/// - stdin cannot be registered with mio
/// - `SIGWINCH` signal handler cannot be registered
///
/// [`DirectToAnsiInputDevice`]: super::input_device::DirectToAnsiInputDevice
/// [`mio::Poll`]: mio::Poll
/// [`SourceFd`]: mio::unix::SourceFd
pub async fn get_or_init_resource_guard()
-> tokio::sync::MutexGuard<'static, Option<DirectToAnsiInputResource>> {
    let mut guard = GLOBAL_INPUT_RESOURCE.lock().await;
    if guard.is_none() {
        *guard = Some(DirectToAnsiInputResource {
            stdin_rx: mio_poller_thread::spawn(),
            event_queue: VecDeque::with_capacity(32),
        });
    }
    guard
}

/// Dedicated thread that polls `stdin` and `SIGWINCH` using [`mio::Poll`].
///
/// This module encapsulates the mio-based polling thread that monitors both stdin
/// for keyboard/mouse input and SIGWINCH for terminal resize events. Events are
/// sent through a channel to the async side.
///
/// [`mio::Poll`]: mio::Poll
mod mio_poller_thread {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Read buffer size for `stdin` reads (1,024 bytes).
    ///
    /// When `read_count == TTY_BUFFER_SIZE`, we know more data is likely waiting in the
    /// kernel buffer—this is the `more` flag used for ESC disambiguation.
    const TTY_BUFFER_SIZE: usize = 1_024;

    /// Token for stdin file descriptor in [`mio::Poll`].
    const STDIN_TOKEN: Token = Token(0);

    /// Token for SIGWINCH signal in [`mio::Poll`].
    const SIGNAL_TOKEN: Token = Token(1);

    /// Sender end of the channel, held by the reader thread.
    pub type StdinSender = tokio::sync::mpsc::UnboundedSender<ReaderThreadMessage>;

    /// Receiver end of the channel, used by the async input device.
    pub type StdinReceiver = tokio::sync::mpsc::UnboundedReceiver<ReaderThreadMessage>;

    /// Creates a channel and spawns the dedicated mio poller thread.
    ///
    /// # Returns
    ///
    /// The receiver end of the channel. The sender is moved into the spawned thread.
    ///
    /// # Thread Lifetime
    ///
    /// The thread runs until:
    /// - `stdin` reaches EOF (returns `ReaderThreadMessage::Eof`)
    /// - An I/O error occurs (returns `ReaderThreadMessage::Error`)
    /// - The receiver is dropped (send fails, thread exits gracefully)
    ///
    /// Since the receiver is stored in [`GLOBAL_INPUT_RESOURCE`], the thread
    /// effectively runs for the process lifetime.
    ///
    /// [`GLOBAL_INPUT_RESOURCE`]: super::GLOBAL_INPUT_RESOURCE
    pub fn spawn() -> StdinReceiver {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::Builder::new()
            .name("mio-poller".into())
            .spawn(move || main_polling_loop(tx))
            .expect("Failed to spawn mio poller thread");

        rx
    }

    /// The main loop of the mio poller thread.
    ///
    /// Uses [`mio::Poll`] to efficiently wait on both `stdin` and `SIGWINCH` signals.
    /// On Linux, this uses `epoll`; on macOS, this uses `kqueue`. This avoids the
    /// limitations of raw `poll()` on macOS.
    ///
    /// # Event Handling
    ///
    /// - **`STDIN_TOKEN`**: Read bytes, parse into events, send as
    ///   [`ReaderThreadMessage::Event`]
    /// - **`SIGNAL_TOKEN`**: Drain pending signals, send [`ReaderThreadMessage::Resize`]
    ///
    /// # Exit Conditions
    ///
    /// - EOF on `stdin`
    /// - I/O error (except EINTR which is retried)
    /// - Channel receiver dropped
    ///
    /// [`mio::Poll`]: mio::Poll
    #[allow(clippy::too_many_lines)] // Polling loop is clearer as single function.
    fn main_polling_loop(tx: StdinSender) {
        // Create mio Poll instance.
        let mut poll = Poll::new().expect("Failed to create mio::Poll");
        let registry = poll.registry();

        // Register stdin with mio.
        let stdin = std::io::stdin();
        let stdin_fd = stdin.as_raw_fd();
        registry
            .register(&mut SourceFd(&stdin_fd), STDIN_TOKEN, Interest::READABLE)
            .expect("Failed to register stdin with mio");

        // Register SIGWINCH with signal-hook-mio.
        let mut signals =
            Signals::new([SIGWINCH]).expect("Failed to register SIGWINCH handler");
        registry
            .register(&mut signals, SIGNAL_TOKEN, Interest::READABLE)
            .expect("Failed to register SIGWINCH with mio");

        // Event buffer for mio::Poll.
        let mut events = Events::with_capacity(8);

        let mut buffer = [0u8; TTY_BUFFER_SIZE];
        let mut parser = Parser::default();

        // Paste state machine (accumulates text between Paste(Start) and Paste(End)).
        let mut paste_state = PasteCollectionState::Inactive;

        // Lock stdin for the duration of the loop.
        let mut stdin_lock = stdin.lock();

        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: started with mio::Poll");
        });

        loop {
            // Wait for events on stdin or SIGWINCH.
            match poll.poll(&mut events, None) {
                Ok(()) => {}
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                    // EINTR - retry poll.
                    continue;
                }
                Err(e) => {
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::debug!(
                            message = "mio-poller-thread: poll error",
                            error = ?e
                        );
                    });
                    drop(tx.send(ReaderThreadMessage::Error));
                    break;
                }
            }

            // Process all ready events.
            for event in &events {
                match event.token() {
                    STDIN_TOKEN => {
                        // Read from stdin.
                        match stdin_lock.read(&mut buffer) {
                            Ok(0) => {
                                // EOF reached.
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(
                                        message = "mio-poller-thread: EOF (0 bytes)"
                                    );
                                });
                                drop(tx.send(ReaderThreadMessage::Eof));
                                return; // Exit thread.
                            }
                            Ok(n) => {
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(
                                        message = "mio-poller-thread: read bytes",
                                        bytes_read = n
                                    );
                                });

                                // `more` flag for ESC disambiguation.
                                let more = n == TTY_BUFFER_SIZE;

                                // Parse bytes into events.
                                parser.advance(&buffer[..n], more);

                                // Process all parsed events through paste state machine.
                                for vt100_event in parser.by_ref() {
                                    match apply_paste_state_machine(
                                        &mut paste_state,
                                        &vt100_event,
                                    ) {
                                        LoopContinuationSignal::Emit(input_event) => {
                                            if tx
                                                .send(ReaderThreadMessage::Event(
                                                    input_event,
                                                ))
                                                .is_err()
                                            {
                                                // Receiver dropped - exit gracefully.
                                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(
                                                    || {
                                                        tracing::debug!(
                                                    message = "mio-poller-thread: receiver dropped, exiting"
                                                );
                                                    },
                                                );
                                                return; // Exit thread.
                                            }
                                        }
                                        LoopContinuationSignal::Continue => {
                                            // Event absorbed (e.g., paste in progress).
                                        }
                                    }
                                }
                            }
                            Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                                // EINTR - will retry on next poll iteration.
                            }
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                // No more data available right now (spurious wakeup).
                            }
                            Err(e) => {
                                // Other error - send and exit.
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(
                                        message = "mio-poller-thread: read error",
                                        error = ?e
                                    );
                                });
                                drop(tx.send(ReaderThreadMessage::Error));
                                return; // Exit thread.
                            }
                        }
                    }
                    SIGNAL_TOKEN => {
                        // Drain all pending signals.
                        for sig in signals.pending() {
                            if sig == SIGWINCH {
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(
                                        message = "mio-poller-thread: SIGWINCH received"
                                    );
                                });
                                if tx.send(ReaderThreadMessage::Resize).is_err() {
                                    // Receiver dropped - exit gracefully.
                                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                        tracing::debug!(
                                            message =
                                                "mio-poller-thread: receiver dropped, exiting"
                                        );
                                    });
                                    return; // Exit thread.
                                }
                            }
                        }
                    }
                    _ => {
                        // Unknown token - should never happen.
                        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                            tracing::warn!(
                                message = "mio-poller-thread: unknown token",
                                token = ?event.token()
                            );
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests_reader_thread_message {
    use super::*;
    use crate::{Key, KeyPress, SpecialKey};

    #[test]
    fn debug_impl_covers_all_variants() {
        let event = ReaderThreadMessage::Event(InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        }));
        assert!(format!("{event:?}").contains("Event"));

        let eof = ReaderThreadMessage::Eof;
        assert!(format!("{eof:?}").contains("Eof"));

        let error = ReaderThreadMessage::Error;
        assert!(format!("{error:?}").contains("Error"));

        let resize = ReaderThreadMessage::Resize;
        assert!(format!("{resize:?}").contains("Resize"));
    }
}

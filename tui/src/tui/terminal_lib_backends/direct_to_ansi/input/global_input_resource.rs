// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH kqueue epoll wakeup eventfd bcast

//! # Global singleton (process bound) for terminal input with a dedicated reader thread
//!
//! This module provides cancel-safe async terminal input for a process, by bridging a
//! synchronous [`mio`]-based reader thread with async consumers via a [`broadcast`]
//! channel. It handles keyboard input (including ANSI escape sequences for arrow keys,
//! function keys, etc.) and terminal resize signals ([`SIGWINCH`]) reliably, even over
//! [SSH].
//!
//! The broadcast channel allows **multiple async consumers** to receive all input events
//! simultaneously; this can be use useful for debugging, logging, or event recording
//! alongside the "primary" TUI app consumer.
//!
//! # Why This Design? (Historical Context)
//!
//! Our original "Tokio-heavy" approach created a [`DirectToAnsiInputDevice`] instance
//! on-demand, one-instance-per-app (which was not process-bound, rather it was bound
//! to each app-instance). It used:
//! - [`tokio::io::stdin()`] for input handling
//! - [`tokio::signal`] for [`SIGWINCH`] handling
//!
//! ## The Problems
//!
//! **This caused three problems that led us to the current design:**
//!
//! 1. **UI freeze on resize.** [Tokio's stdin] uses a blocking threadpool. In the past,
//!    in [`DirectToAnsiInputDevice::try_read_event()`], when [`tokio::select!`] cancelled
//!    a [`tokio::io::stdin()`] read to handle [`SIGWINCH`], the blocking read kept
//!    running in the background. The next read conflicted with this "zombie" read leading
//!    to a UI freeze.
//!
//! 2. **Dropped keystrokes.** Creating a new [`stdin`] handle lost access to data already
//!    in the kernel buffer. When TUI "App A" exited and "App B" started, keystrokes typed
//!    during the transition vanished. This was easily reproducible by:
//!    - Running `cargo run --examples tui_apps`.
//!    - Starting one app, exiting, **dropped keystrokes**, starting another, exit,
//!      **dropped keystrokes**, starting another, and so on.
//!
//! 3. **Flawed `ESC` detection over [SSH].** Our original approach had flawed logic for
//!    distinguishing the `ESC` key from escape sequences (like `ESC [ A` for Up Arrow).
//!    It worked locally but failed over [SSH]. We now use [`crossterm`]'s `more` flag
//!    heuristic (see [ESC Detection Limitations] in [`MioPoller`]).
//!
//! ## The Solution
//!
//! A **process bound global singleton** with a dedicated reader thread that exclusively
//! owns the [`stdin`] handle. The thread uses [`mio::Poll`] to wait on both [`stdin`]
//! data and [`SIGWINCH`] signals.
//!
//! Although sync and blocking, [`mio`] is efficient. It uses OS primitives ([`epoll`] on
//! Linux, [`kqueue`] on BSD/macOS) that put the thread to sleep until data arrives,
//! consuming no CPU while waiting. See [How It Works] in [`MioPoller`] for details.
//!
//! ```text
//!     Process-bound Global Singleton                       Async Consumers
//! ┌─────────────────────────────────────┐           ┌─────────────────────────────┐
//! │ Sync Blocking (std::thread + mio)   │           │ Primary: TUI input handler  │
//! │                                     │           │ Optional: Debug logger      │
//! │ Owns exclusively:                   │           │ Optional: Event recorder    │
//! │   • std::stdin handle (locked)      │           │                             │
//! │   • Parser state                    │           │                             │
//! │   • SIGWINCH watcher                │           │                             │
//! │                                     │ broadcast │                             │
//! │ tx.send(InputEvent)  ───────────────┼───────────▶ rx.recv().await             │
//! │                                     │ channel   │ (cancel-safe, fan-out!)     │
//! └─────────────────────────────────────┘    │      └─────────────────────────────┘
//!                                            ▼
//!                                     Sync -> Async Bridge
//! ```
//!
//! This solves the first two problems completely:
//! 1. **Cancel-safe**: Channel receive is truly async - no zombie reads
//! 2. **Data preserved**: Global state survives TUI app lifecycle transitions in the same
//!    process.
//!
//! To solve the third problem for `ESC` detection, we use [`crossterm`]'s `more` flag
//! heuristic (see [ESC Detection Limitations] in [`MioPoller`]).
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │ INPUT_RESOURCE (static LazyLock<Mutex<...>>)                            │
//! │ internal:                                                               │
//! │  • mio-poller thread: owns tx, stdin, vt100 parser (spawned 1st access) │
//! │ external:                                                               │
//! │  • stdin_rx: broadcast receiver (async consumers recv() from here)      │
//! └───────────────────────────────────────┬─────────────────────────────────┘
//!                                         │
//!            ┌────────────────────────────┴─────────────────────────┐
//!            │                                                      │
//! ┌──────────▼───────────────────┐            ┌─────────────────────▼───────┐
//! │ DirectToAnsiInputDevice A    │            │ DirectToAnsiInputDevice B   │
//! │   (TUI App context)          │            │   (Readline context)        │
//! │   • Zero-sized handle        │            │   • Zero-sized handle       │
//! │   • Delegates to global      │            │   • Delegates to global     │
//! └──────────────────────────────┘            └─────────────────────────────┘
//!
//! 🎉 Data preserved during transitions - same channel used throughout!
//! ```
//!
//! The key insight: [`stdin`] handles must persist across device lifecycle boundaries.
//! Multiple [`DirectToAnsiInputDevice`] instances can be created and dropped, but they
//! all share the same underlying channel and process global (singleton) reader thread.
//!
//! See [`MioPoller`] for details on how the mio poller thread works, including
//! file descriptor handling, parsing, thread lifecycle, and ESC detection limitations.
//!
//! # Data Flow Diagram
//!
//! See the [Data Flow Diagram] section in [`DirectToAnsiInputDevice`] for the complete
//! data flow showing how [`try_read_event()`] interacts with this global resource.
//!
//! # Attribution: [`crossterm`]
//!
//! We looked at [`crossterm`]'s source code for design inspiration:
//! 1. **Global state pattern**: [`crossterm`] uses a global [`INTERNAL_EVENT_READER`]
//!    that holds the `tty` file descriptor and event buffer, ensuring data in the kernel
//!    buffer is not lost when [`EventStream`] instances are created and dropped. And we
//!    have the same global singleton pattern here.
//! 2. **[`mio`]-based polling**: Their [`mio.rs`] uses [`mio::Poll`] with
//!    [`signal-hook-mio`] for [`SIGWINCH`] and we do the same.
//! 3. **ESC disambiguation**: The `more` flag heuristic for distinguishing ESC key from
//!    escape sequences without timeouts. We inherit both its benefits (zero latency) and
//!    limitations (see [ESC Detection Limitations] in [`MioPoller`]).
//! 4. **Process-lifetime cleanup**: They rely on OS cleanup at process exit rather than
//!    explicit thread termination, and so do we.
//!
//! [`broadcast`]: tokio::sync::broadcast
//! [Tokio's stdin]: tokio::io::stdin
//! [`EventStream`]: ::crossterm::event::EventStream
//! [`INTERNAL_EVENT_READER`]:
//!     https://github.com/crossterm-rs/crossterm/blob/0.29/src/event.rs#L149
//! [`mio.rs`]:
//!     https://github.com/crossterm-rs/crossterm/blob/0.29/src/event/source/unix/mio.rs
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
//! [`tokio::sync::broadcast`]: tokio::sync::broadcast
//! [`stdin`]: std::io::stdin
//! [`mio`]: mio
//! [`mio::Poll`]: mio::Poll
//! [`signal-hook`]: signal_hook
//! [`signal-hook-mio`]: signal_hook_mio
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`Event(InputEvent)`]: ReaderThreadMessage::Event
//! [`Resize`]: ReaderThreadMessage::Resize
//! [`Eof`]: ReaderThreadMessage::Eof
//! [`Error`]: ReaderThreadMessage::Error
//! [`readline_async`]: mod@crate::readline_async
//! [`SIGWINCH`]: signal_hook::consts::SIGWINCH
//! [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
//! [`MioPoller`]: super::mio_poller::MioPoller
//! [How It Works]: super::mio_poller::MioPoller#how-it-works
//! [ESC Detection Limitations]: super::mio_poller::MioPoller#esc-detection-limitations

use super::{mio_poller::MioPoller,
            types::{CHANNEL_CAPACITY, InputEventReceiver, InputEventSender}};
use std::sync::LazyLock;

/// Global singleton holding the [`broadcast::Sender`] that is [initialized] on first
/// access.
///
/// - Independent async consumers should use [`subscribe_to_input_events()`] to get input
///   events & signals.
/// - See the [module-level documentation] for details on why global state is necessary.
/// - See [`MioPoller`] docs for details on how the dedicated thread works.
///
/// [initialized]: initialize_input_resource
/// [module-level documentation]: self
/// [`stdin`]: std::io::stdin
/// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
pub static INPUT_RESOURCE: LazyLock<std::sync::Mutex<Option<InputEventSender>>> =
    LazyLock::new(|| std::sync::Mutex::new(None));

/// Subscribe your async consumer to the global input resource, in order to receive input
/// events.
///
/// The global static singleton [`INPUT_RESOURCE`] contains one [`broadcast::Sender`].
/// This channel acts as a bridge between sync the only [`MioPoller`] and the many
/// async consumers. We don't need to capture the broadcast channel itself in the
/// singleton, only the sender, since it is trivial to create new receivers from it.
///
/// # Returns
///
/// A new [`InputEventReceiver`] that independently receives all input events and resize
/// signals.
///
/// # Multiple Async Consumers
///
/// Each caller gets their own receiver via [`broadcast::Sender::subscribe()`]. Here are
/// examples of callers:
/// - TUI app that receives all input events.
/// - Logger receives all input events (independently).
/// - Debug recorder receives all input events (independently).
///
/// # Thread Spawning
///
/// On first call, this spawns the [`mio`] poller thread via [`MioPoller::spawn_thread()`]
/// which uses [`mio::Poll`] to wait on both [`stdin`] data and [`SIGWINCH`] signals.
/// See the [Thread Lifecycle] section in [`MioPoller`] for details on thread
/// lifetime and exit conditions.
///
/// # Panics
///
/// Panics if:
/// 1. Thread spawning fails; see [`MioPoller::spawn_thread()`] for details.
/// 2. The [`INPUT_RESOURCE`] mutex is poisoned.
/// 3. The [`INPUT_RESOURCE`] is `None` after initialization (invariant violation).
///
/// [Thread Lifecycle]: MioPoller#thread-lifecycle
/// [`stdin`]: std::io::stdin
/// [`INPUT_RESOURCE`]: INPUT_RESOURCE
/// [`broadcast::Sender::subscribe()`]: tokio::sync::broadcast::Sender::subscribe
/// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`mio::Poll`]: mio::Poll
pub fn subscribe_to_input_events() -> InputEventReceiver {
    let mut input_resource_guard = INPUT_RESOURCE
        .lock()
        .expect("INPUT_RESOURCE mutex poisoned");

    if input_resource_guard.is_none() {
        initialize_input_resource(&mut input_resource_guard);
    }

    input_resource_guard
        .as_ref()
        .expect("INPUT_RESOURCE should have been initialized already")
        .subscribe()
}

/// Creates the broadcast channel and spawns the [`MioPoller`] thread.
///
/// Called once on first access to [`INPUT_RESOURCE`].
pub fn initialize_input_resource(input_resource_guard: &mut Option<InputEventSender>) {
    let (tx_parsed_input_events, _): (InputEventSender, _) =
        tokio::sync::broadcast::channel(CHANNEL_CAPACITY);
    input_resource_guard.replace(tx_parsed_input_events.clone());
    MioPoller::spawn_thread(tx_parsed_input_events);
}

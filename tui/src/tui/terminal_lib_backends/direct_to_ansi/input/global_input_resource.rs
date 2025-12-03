// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR

//! Global singleton for the input resource with dedicated stdin reader thread.
//!
//! This module provides thread-safe, async-compatible access to the shared input state
//! that persists for the process lifetime, along with the dedicated stdin reader thread
//! that feeds it.
//!
//! # Architecture
//!
//! This module uses a **global static** input reader pattern. The key insight is that
//! `stdin` handles must persist across device lifecycle boundaries to prevent data loss
//! during TUI ↔ readline transitions (see [Why Global State?](#why-global-state) below).
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────────┐
//! │ GLOBAL_INPUT_RESOURCE (static LazyLock<Mutex<...>>)                       │
//! │   • stdin_rx: mpsc ← std::io::stdin in one thread (process lifetime)      │
//! │   • sigwinch_receiver: Signal (process lifetime)                          │
//! │   • parse_buffer: ParseBuffer (carries over partial sequences)            │
//! │   • paste_state: PasteCollectionState (mid-paste survives)                │
//! │   • event_queue: VecDeque (buffered events preserved)                     │
//! └───────────────────────────────────────────────────────────────────────────┘
//!                                         │
//!            ┌────────────────────────────┴────────────────────────┐
//!            │                                                     │
//! ┌──────────▼───────────────────┐           ┌─────────────────────▼────────┐
//! │ DirectToAnsiInputDevice A    │           │ DirectToAnsiInputDevice B    │
//! │   (TUI App context)          │           │   (Readline context)         │
//! │   • Zero-sized handle        │           │   • Zero-sized handle        │
//! │   • Delegates to global      │           │   • Delegates to global      │
//! │     resource                 │           │     resource                 │
//! └──────────────────────────────┘           └──────────────────────────────┘
//!
//! 🎉 Data preserved during transitions - same resources used throughout!
//! ```
//!
//! This mirrors [crossterm]'s architecture where a global [`INTERNAL_EVENT_READER`] holds
//! the tty file descriptor and event buffer, ensuring data in the kernel buffer is not
//! lost when [`EventStream`] instances are created and dropped.
//!
//! # Why Global State?
//!
//! Two problems stemming from our historical choice to use [`tokio::io::stdin()`]:
//!
//! ## Problem 1: UI freeze on terminal resize ([`9c20af80`])
//!
//! [`tokio::io::stdin()`] uses a blocking thread pool internally. When used in
//! [`tokio::select!`] alongside `SIGWINCH` signal handling, cancellation doesn't stop
//! the blocking read — it continues running in the background. The code in
//! [`DirectToAnsiInputDevice::try_read_event()`] uses [`tokio::select!`] to await either
//! stdin data or a `SIGWINCH` signal. The next `stdin` read conflicts with the
//! still-running operation, causing undefined behavior that manifests as a UI freeze
//! after resize.
//!
//! ## Problem 2: Dropped keys between TUI apps ([`6dbeae1d`])
//!
//! Creating a new [`tokio::io::stdin()`] handle after a TUI app exits, and before the
//! `readline_async` starts, makes data in the kernel buffer inaccessible. This was not a
//! hypothetical scenario, and could easily be reproduced by running `cargo run --example
//! tui_apps` and exiting then starting multiple TUI apps with key presses being dropped
//! during transitions.
//!
//! ## The Solution: Dedicated Thread with Channel
//!
//! **We use [`std::io::stdin()`]** (NOT [`tokio::io::stdin()`]) in a dedicated thread.
//! This thread performs blocking reads and sends results through a [`tokio::sync::mpsc`]
//! channel. The async side receives from this channel, which is truly async and
//! properly cancel-safe.
//!
//! ```text
//! ┌─────────────────────────┐       ┌─────────────────────────────────┐
//! │ Dedicated Thread        │       │ Async Task                      │
//! │ (std::thread::spawn)    │       │                                 │
//! │                         │       │ tokio::select! {                │
//! │ loop {                  │──────▶│   bytes = rx.recv() => { ... }  │
//! │   stdin.read_blocking() │ mpsc  │   signal = sigwinch => { ... }  │
//! │   tx.send(bytes)        │       │ }                               │
//! │ }                       │       │                                 │
//! └─────────────────────────┘       └─────────────────────────────────┘
//!
//! When SIGWINCH wins:
//!   1. select! cancels rx.recv() future
//!   2. Thread continues reading, but that's fine - it owns stdin exclusively
//!   3. Next rx.recv() gets the data the thread read
//!   4. No undefined behavior! ✓
//! ```
//!
//! This solves both problems:
//! - **Cancel-safe**: Channel receive is truly async and properly cancel-safe in
//!   [`tokio::select!`], unlike the blocking thread pool approach.
//! - **Data preserved**: The stdin handle and [parse buffer] survive across
//!   [`DirectToAnsiInputDevice`] lifetimes, tied to process lifetime.
//!
//! # The Problem with [`tokio::io::stdin()`]
//!
//! [`tokio::io::stdin()`] is **not truly async** - it spawns blocking reads on Tokio's
//! blocking thread pool. When used in [`tokio::select!`], if another branch wins (e.g.,
//! `SIGWINCH` arrives), the stdin read is "cancelled" but the blocking thread continues
//! running.
//!
//! ```text
//! BROKEN PATTERN:
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │ tokio::select! {                                                     │
//! │   result = tokio_stdin.read() => { ... }  // Spawns blocking thread! │
//! │   signal = sigwinch.recv() => { ... }     // True async ✓            │
//! │ }                                                                    │
//! └──────────────────────────────────────────────────────────────────────┘
//!
//! When `SIGWINCH` wins:
//!   1. [`tokio::select!`] "cancels" `tokio_stdin.read()` future
//!   2. BUT the blocking thread keeps running in the background
//!   3. Next `tokio_stdin.read()` → undefined behavior
//!      (two threads reading `tokio_stdin`!)
//! ```
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
//! - Thread is doing nothing - blocked on read, not consuming CPU
//! - No resources to leak - stdin is fd 0, not owned by us
//! - This matches [`crossterm`] - they also rely on process exit for cleanup
//!
//! The thread self-terminates gracefully in these scenarios:
//! - **EOF on stdin**: When stdin is closed (e.g., pipe closed, Ctrl+D), `read()`
//!   returns 0 bytes. The thread sends [`StdinReadResult::Eof`] and exits.
//! - **I/O error**: On read errors (except `EINTR` which is retried), the thread sends
//!   [`StdinReadResult::Error`] and exits.
//! - **Receiver dropped**: When [`GLOBAL_INPUT_RESOURCE`] is dropped (process exit), the
//!   channel receiver is dropped. The next `tx.send()` returns `Err`, and the thread
//!   exits gracefully.
//!
//! # Reference: How [`crossterm`] Solves This
//!
//! [`crossterm`] uses a similar pattern with a global [`INTERNAL_EVENT_READER`]:
//!
//! ```text
//! static INTERNAL_EVENT_READER: Mutex<Option<...>>
//!    source: UnixInternalEventSource (tty fd - PERSISTS)
//!    events: VecDeque<InternalEvent> (buffer - PERSISTS)
//! ```
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
//! [`EventStream`]: ::crossterm::event::EventStream
//! [`INTERNAL_EVENT_READER`]: https://github.com/crossterm-rs/crossterm/blob/0.29.0/src/event.rs#L149
//! [crossterm]: ::crossterm
//! [`DirectToAnsiInputDevice::try_read_event()`]: super::input_device::DirectToAnsiInputDevice::try_read_event
//! [`DirectToAnsiInputDevice`]: super::input_device::DirectToAnsiInputDevice
//! [parse buffer]: super::parse_buffer
//! [`9c20af80`]: https://github.com/r3bl-org/r3bl-open-core/commit/9c20af80
//! [`6dbeae1d`]: https://github.com/r3bl-org/r3bl-open-core/commit/6dbeae1d
//! [Data Flow Diagram]: super::input_device::DirectToAnsiInputDevice#data-flow-diagram
//! [`try_read_event()`]: super::input_device::DirectToAnsiInputDevice::try_read_event
//! [`LazyLock`]: std::sync::LazyLock
//! [`std::io::stdin()`]: std::io::stdin
//! [`std::process::exit()`]: std::process::exit
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::select!`]: tokio::select
//! [`tokio::sync::mpsc`]: tokio::sync::mpsc

use super::{parse_buffer::ParseBuffer, paste_state_machine::PasteCollectionState, types::StdinReadResult};
use crate::{tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND, InputEvent};
use std::{collections::VecDeque, io::Read as _, sync::LazyLock};
use tokio::signal::unix::{Signal, SignalKind};

/// Temporary read buffer size for stdin reads.
///
/// This is the read granularity: how much data we pull from the kernel in one
/// syscall. Too small (< 256): Excessive syscalls increase latency. Too large
/// (> 256): Delays response to time-sensitive input (e.g., arrow key repeat).
///
/// 256 bytes is optimal for terminal input: it's one page boundary on many
/// architectures, fits comfortably in the input buffer, and provides good syscall
/// efficiency without introducing noticeable latency.
const STDIN_READ_BUFFER_SIZE: usize = 256;

/// Sender end of the stdin channel, held by the reader thread.
pub type StdinSender = tokio::sync::mpsc::UnboundedSender<StdinReadResult>;

/// Receiver end of the stdin channel, used by the async input device.
pub type StdinReceiver = tokio::sync::mpsc::UnboundedReceiver<StdinReadResult>;

/// Global static singleton for input reader state - persists for process lifetime.
///
/// See the [module-level documentation](self) for details on why global state is
/// necessary and how the dedicated stdin reader thread works.
#[allow(missing_debug_implementations)]
pub struct DirectToAnsiInputResource {
    /// Receiver for data from the dedicated stdin reader thread.
    ///
    /// This channel receives [`StdinReadResult`] from a dedicated thread that
    /// performs blocking reads on stdin. This architecture solves the fundamental
    /// problem with [`tokio::io::stdin()`] which uses a blocking thread pool:
    /// when cancelled in [`tokio::select!`], the blocking read continues running,
    /// causing undefined behavior on the next read.
    ///
    /// With a dedicated thread + channel:
    /// - Only one thread ever reads from stdin (no conflicts)
    /// - Channel receive is truly async (properly cancel-safe)
    /// - Data read by the thread waits in the channel (no data loss)
    ///
    /// [`tokio::io::stdin()`]: tokio::io::stdin
    /// [`tokio::select!`]: tokio::select
    pub stdin_rx: StdinReceiver,

    /// Parse buffer encapsulating byte storage and position tracking.
    ///
    /// This buffer persists across device lifetimes, ensuring partial ANSI sequences
    /// are not lost during TUI ↔ readline transitions. The [`ParseBuffer`] type
    /// handles compaction automatically when consumed bytes exceed threshold.
    pub parse_buffer: ParseBuffer,

    /// State machine for collecting bracketed paste text.
    ///
    /// Tracks whether we're between `Paste(Start)` and `Paste(End)` markers.
    /// Persists across device lifetimes so mid-paste transitions don't lose data.
    pub paste_state: PasteCollectionState,

    /// Buffered events that haven't been consumed yet.
    ///
    /// When multiple events are parsed from a single read, extras are queued here.
    /// Pre-allocated with capacity 32 for typical burst scenarios.
    pub event_queue: VecDeque<InputEvent>,

    /// `SIGWINCH` signal receiver for terminal resize events (Unix-only).
    ///
    /// Terminal resize is not sent through stdin as ANSI sequences - it's delivered
    /// via the `SIGWINCH` signal. We use [`tokio::signal::unix::Signal`] to receive
    /// these asynchronously and convert them to [`InputEvent::Resize`].
    ///
    /// This is now part of the global singleton (not per-instance) because:
    /// - Signal handlers should be registered once per process
    /// - We already hold the mutex during `await_input()`, so sharing is safe
    /// - Consistent with stdin handling pattern
    ///
    /// # TODO(windows)
    ///
    /// Windows uses `WINDOW_BUFFER_SIZE_EVENT` via Console API instead of `SIGWINCH`.
    /// When adding Windows support for [`DirectToAnsi`], implement resize detection
    /// using `ReadConsoleInput` which returns window buffer size change events.
    /// See: <https://learn.microsoft.com/en-us/windows/console/window-buffer-size-record-str>
    ///
    /// [`DirectToAnsi`]: mod@super::super
    pub sigwinch_receiver: Signal,
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
/// On first call, spawns the dedicated [`std::io::stdin`] reader thread, registers the
/// `SIGWINCH` handler, creates the parse buffer, and event queue. Subsequent calls return
/// a guard to the existing state.
///
/// # [`stdin`] Reader Thread
///
/// The dedicated thread is spawned on first call and runs for the process lifetime.
/// It performs blocking reads on [`stdin`] and sends results through the channel stored
/// in [`DirectToAnsiInputResource::stdin_rx`]. See the [module-level documentation](self)
/// for why we use a dedicated thread instead of [`tokio::io::stdin()`].
///
/// # `SIGWINCH` Handler
///
/// The signal handler is registered once per process on first call. This is more
/// efficient than registering a new handler for each [`DirectToAnsiInputDevice`]
/// instance.
///
/// # Panics
///
/// Panics if the `SIGWINCH` signal handler cannot be registered (Unix-only).
/// This should only happen if the signal is already registered elsewhere.
///
/// [`DirectToAnsiInputDevice`]: super::input_device::DirectToAnsiInputDevice
/// [`tokio::io::stdin()`]: tokio::io::stdin
/// [`stdin`]: std::io::stdin
pub async fn get_resource_guard()
-> tokio::sync::MutexGuard<'static, Option<DirectToAnsiInputResource>> {
    let mut guard = GLOBAL_INPUT_RESOURCE.lock().await;
    if guard.is_none() {
        let sigwinch_receiver = tokio::signal::unix::signal(SignalKind::window_change())
            .expect("Failed to register SIGWINCH handler");

        *guard = Some(DirectToAnsiInputResource {
            stdin_rx: spawn_stdin_reader_thread(),
            parse_buffer: ParseBuffer::new(),
            paste_state: PasteCollectionState::Inactive,
            event_queue: VecDeque::with_capacity(32),
            sigwinch_receiver,
        });
    }
    guard
}

/// Creates a channel and spawns the dedicated stdin reader thread.
///
/// # Returns
///
/// The receiver end of the channel. The sender is moved into the spawned thread.
///
/// # Thread Lifetime
///
/// The thread runs until:
/// - stdin reaches EOF (returns `StdinReadResult::Eof`)
/// - An I/O error occurs (returns `StdinReadResult::Error`)
/// - The receiver is dropped (send fails, thread exits gracefully)
///
/// Since the receiver is stored in [`GLOBAL_INPUT_RESOURCE`], the thread effectively
/// runs for the process lifetime.
fn spawn_stdin_reader_thread() -> StdinReceiver {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    std::thread::Builder::new()
        .name("stdin-reader".into())
        .spawn(move || stdin_reader_loop(tx))
        .expect("Failed to spawn stdin reader thread");

    rx
}

/// The main loop of the stdin reader thread.
///
/// Continuously reads from stdin and sends results through the channel until:
/// - EOF is reached
/// - An error occurs
/// - The channel receiver is dropped
fn stdin_reader_loop(tx: StdinSender) {
    let mut stdin = std::io::stdin().lock();
    let mut buffer = [0u8; STDIN_READ_BUFFER_SIZE];

    loop {
        match stdin.read(&mut buffer) {
            Ok(0) => {
                // EOF reached.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(message = "stdin-reader-thread: EOF (0 bytes)");
                });
                drop(tx.send(StdinReadResult::Eof));
                break;
            }
            Ok(n) => {
                // Successfully read n bytes.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "stdin-reader-thread: read bytes",
                        bytes_read = n
                    );
                });
                let data = buffer[..n].to_vec();
                if tx.send(StdinReadResult::Data(data)).is_err() {
                    // Receiver dropped - exit gracefully.
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::debug!(
                            message = "stdin-reader-thread: receiver dropped, exiting"
                        );
                    });
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                // EINTR - retry immediately (loop continues).
            }
            Err(e) => {
                // Other error - send and exit.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "stdin-reader-thread: error",
                        error = ?e
                    );
                });
                drop(tx.send(StdinReadResult::Error(e.kind())));
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdin_read_result_debug() {
        // Verify Debug trait is implemented correctly.
        let data_result = StdinReadResult::Data(vec![0x1B, 0x5B, 0x41]);
        let debug_str = format!("{:?}", data_result);
        assert!(debug_str.contains("Data"));

        let eof_result = StdinReadResult::Eof;
        let debug_str = format!("{:?}", eof_result);
        assert!(debug_str.contains("Eof"));

        let error_result = StdinReadResult::Error(std::io::ErrorKind::WouldBlock);
        let debug_str = format!("{:?}", error_result);
        assert!(debug_str.contains("Error"));
    }
}

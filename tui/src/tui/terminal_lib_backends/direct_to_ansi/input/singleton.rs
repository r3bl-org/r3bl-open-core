// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Global singleton for the input core - initialized on first access.
//!
//! This module provides thread-safe, async-compatible access to the shared input
//! state that persists for the process lifetime.

use super::{buffer::ParseBuffer,
            stdin_reader_thread::{StdinReceiver, spawn_stdin_reader_thread},
            types::PasteCollectionState};
use crate::InputEvent;
use std::{collections::VecDeque, sync::LazyLock};
#[cfg(unix)]
use tokio::signal::unix::{Signal, SignalKind};

/// Global static singleton for input reader state - persists for process lifetime.
///
/// This mirrors [crossterm]'s architecture where a global [`INTERNAL_EVENT_READER`] holds
/// the tty file descriptor and event buffer, ensuring data in the kernel buffer is not
/// lost when [`EventStream`] instances are created and dropped.
///
/// # Architecture
///
/// This module uses a **global static** input reader pattern. The key insight is that
/// `stdin` handles must persist across device lifecycle boundaries to prevent data loss
/// during TUI ↔ readline transitions (see [Why Global State?](#why-global-state) below).
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────────┐
/// │ GLOBAL_INPUT_CORE (static LazyLock<Mutex<...>>)                           │
/// │   • stdin_rx: mpsc ← std::io::stdin in one thread (process lifetime)      │
/// │   • sigwinch_receiver: Signal (process lifetime)                          │
/// │   • parse_buffer: ParseBuffer (carries over partial sequences)            │
/// │   • paste_state: PasteCollectionState (mid-paste survives)                │
/// │   • event_queue: VecDeque (buffered events preserved)                     │
/// └───────────────────────────────────────────────────────────────────────────┘
///                                         │
///            ┌────────────────────────────┴────────────────────────┐
///            │                                                     │
/// ┌──────────▼───────────────────┐           ┌─────────────────────▼────────┐
/// │ DirectToAnsiInputDevice A    │           │ DirectToAnsiInputDevice B    │
/// │   (TUI App context)          │           │   (Readline context)         │
/// │   • Zero-sized handle        │           │   • Zero-sized handle        │
/// │   • Delegates to global core │           │   • Delegates to global core │
/// └──────────────────────────────┘           └──────────────────────────────┘
///
/// 🎉 Data preserved during transitions - same resources used throughout!
/// ```
///
/// # Why Global State?
///
/// Without global state, when a TUI app exits and a new readline context is created,
/// creating a new [`tokio::io::stdin()`] handle causes data in the kernel buffer to
/// become inaccessible. User keypresses during the transition are lost. This is not a
/// hypothetical scenario. Run `cargo run --example tui_apps` and when you start a TUI
/// app, then exit it, then start another one, etc. without this, you will notice that
/// some keypresses are dropped during transitions from one TUI app exit -> next TUI app
/// start!
///
/// With global state, the stdin handle and parse buffer survive across
/// [`DirectToAnsiInputDevice`] lifetimes, ensuring no data loss.
///
/// # Reference: How [`crossterm`] Solves This
///
/// [`crossterm`] uses a global [`INTERNAL_EVENT_READER`], here's an excerpt:
///
/// ```text
/// static INTERNAL_EVENT_READER: Mutex<Option<...>>
///    source: UnixInternalEventSource (tty fd - PERSISTS)
///    events: VecDeque<InternalEvent> (buffer - PERSISTS)
/// ```
///
/// All [`EventStream`] instances share this, so data is never lost during transitions.
///
/// # Why [`tokio::sync::Mutex`] (Not [`std::sync::Mutex`])
///
/// We hold the mutex guard across `.await` points (during `stdin_rx.recv().await`):
/// - [`std::sync::MutexGuard`] is `!Send` and cannot be held across `.await` points
/// - [`tokio::sync::Mutex`] is async-native and yields to scheduler instead of blocking
/// - This prevents starving other tokio tasks while waiting for the lock
///
/// [`EventStream`]: ::crossterm::event::EventStream
/// [`INTERNAL_EVENT_READER`]: https://github.com/crossterm-rs/crossterm/blob/0.29.0/src/event.rs#L149
/// [crossterm]: ::crossterm
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
    /// [`StdinReadResult`]: super::stdin_reader_thread::StdinReadResult
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
    #[cfg(unix)]
    pub sigwinch_receiver: Signal,
}

/// Global singleton - initialized on first access.
///
/// Uses [`LazyLock`] for thread-safe lazy initialization and [`tokio::sync::Mutex`]
/// for async-safe access. The [`Option`] allows initialization to happen on first
/// access.
pub static GLOBAL_INPUT_CORE: LazyLock<
    tokio::sync::Mutex<Option<DirectToAnsiInputResource>>,
> = LazyLock::new(|| tokio::sync::Mutex::new(None));

/// Gets or initializes the global input core.
///
/// On first call, spawns the dedicated stdin reader thread, registers the `SIGWINCH`
/// handler, creates the parse buffer, and event queue. Subsequent calls return a
/// guard to the existing state.
///
/// # Stdin Reader Thread
///
/// The dedicated thread is spawned on first call and runs for the process lifetime.
/// It performs blocking reads on stdin and sends results through the channel stored
/// in [`DirectToAnsiInputResource::stdin_rx`]. This solves the problem with
/// [`tokio::io::stdin()`] which uses a blocking thread pool that doesn't cancel
/// properly in [`tokio::select!`].
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
/// [`tokio::select!`]: tokio::select
pub async fn get_or_init_global_core()
-> tokio::sync::MutexGuard<'static, Option<DirectToAnsiInputResource>> {
    let mut guard = GLOBAL_INPUT_CORE.lock().await;
    if guard.is_none() {
        #[cfg(unix)]
        let sigwinch_receiver = tokio::signal::unix::signal(SignalKind::window_change())
            .expect("Failed to register SIGWINCH handler");

        *guard = Some(DirectToAnsiInputResource {
            stdin_rx: spawn_stdin_reader_thread(),
            parse_buffer: ParseBuffer::new(),
            paste_state: PasteCollectionState::Inactive,
            event_queue: VecDeque::with_capacity(32),
            #[cfg(unix)]
            sigwinch_receiver,
        });
    }
    guard
}

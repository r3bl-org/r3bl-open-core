// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize EINTR SIGWINCH kqueue epoll wakeup eventfd bcast
// cspell:words reinit

//! [`DirectToAnsiInputDevice`] struct and the global input resource singleton.
//!
//! Also see [`PollerSubscriptionHandle`], [`InputResourceState`], and [`INPUT_RESOURCE`]
//! for more details.

use super::{channel_types::{PollerEvent, PollerEventReceiver, SignalEvent, StdinEvent},
            mio_poller::{MioPollerThread, PollerThreadLifecycleState, SourceKindReady,
                         ThreadLiveness}};
use crate::{InputEvent, get_size, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use mio::{Poll, Waker};
use std::{fmt::Debug,
          sync::{Arc, LazyLock, Mutex}};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Global Input Resource (Singleton)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Global singleton holding the input resource state, initialized on first access (see
/// [`allocate()`]).
///
/// - Independent async consumers should use [`allocate()`] to get input events & signals.
/// - See the [Architecture] section in [`DirectToAnsiInputDevice`] for details on why
///   global state is necessary.
/// - See [`MioPollerThread`] docs for details on how the dedicated thread works.
///
/// [Architecture]: DirectToAnsiInputDevice#architecture
/// [`allocate()`]: guarded_ops::allocate
/// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
/// [`stdin`]: std::io::stdin
pub static INPUT_RESOURCE: LazyLock<Mutex<Option<InputResourceState>>> =
    LazyLock::new(|| Mutex::new(None));

/// Container for global input resource state.
///
/// Separates concerns:
/// - [`lifecycle`]: Thread state (sender, metadata) — passed to [`MioPollerThread`]
/// - [`waker`]: Shutdown signal — only needed by [`PollerSubscriptionHandle`], not the
///   thread
///
/// [`MioPollerThread`]: super::mio_poller::MioPollerThread
/// [`PollerSubscriptionHandle`]: PollerSubscriptionHandle
/// [`lifecycle`]: InputResourceState::lifecycle
/// [`waker`]: InputResourceState::waker
#[allow(missing_debug_implementations)]
pub struct InputResourceState {
    /// Thread lifecycle state passed to [`MioPollerThread::new()`].
    ///
    /// [`MioPollerThread::new()`]: super::mio_poller::MioPollerThread::new
    pub lifecycle: PollerThreadLifecycleState,

    /// Waker to signal thread shutdown. Cloned to each [`PollerSubscriptionHandle`].
    ///
    /// NOT passed to the thread — only used as a distribution point for handles.
    pub waker: Arc<mio::Waker>,
}

/// Functions that acquire or operate under [`INPUT_RESOURCE`]'s mutex lock.
///
/// All public functions in this module acquire the [`INPUT_RESOURCE`] mutex guard
/// internally. The `guarded_ops::` prefix at call sites serves as documentation that the
/// call accesses the mutex-protected global singleton.
pub mod guarded_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Subscribe your async consumer to the global input resource, in order to receive
    /// input events.
    ///
    /// The global static singleton [`INPUT_RESOURCE`] contains one [`broadcast::Sender`].
    /// This channel acts as a bridge between sync the only [`MioPollerThread`] and the
    /// many async consumers. We don't need to capture the broadcast channel itself in the
    /// singleton, only the sender, since it is trivial to create new receivers from it.
    ///
    /// # Returns
    ///
    /// A new [`PollerSubscriptionHandle`] that independently receives all input events
    /// and resize signals.
    ///
    /// # Multiple Async Consumers
    ///
    /// Each caller gets their own receiver via [`broadcast::Sender::subscribe()`]. Here
    /// are examples of callers:
    /// - TUI app that receives all input events.
    /// - Logger receives all input events (independently).
    /// - Debug recorder receives all input events (independently).
    ///
    /// # Thread Spawning
    ///
    /// On first call, this spawns the [`mio`] poller thread via
    /// [`MioPollerThread::new()`] which uses [`mio::Poll`] to wait on both
    /// [`stdin`] data and [`SIGWINCH`] signals. See the [Thread Lifecycle] section in
    /// [`MioPollerThread`] for details on thread lifetime and exit conditions.
    ///
    /// # Fast-Path Thread Reuse
    ///
    /// If the thread is still running (`liveness == Running`), we skip spawning a new one
    /// and reuse the existing thread. This handles the inherent race condition where a
    /// new subscriber appears during the thread's shutdown check window.
    ///
    /// See [The Inherent Race Condition] in [`PollerThreadLifecycleState`] for complete
    /// documentation on the race window and why thread reuse is safe.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// 1. Thread spawning fails; see [`MioPollerThread::new()`] for details.
    /// 2. The [`INPUT_RESOURCE`] mutex is poisoned.
    /// 3. The [`INPUT_RESOURCE`] is `None` after initialization (invariant violation).
    ///
    /// [The Inherent Race Condition]: PollerThreadLifecycleState#the-inherent-race-condition
    /// [Thread Lifecycle]: MioPollerThread#thread-lifecycle
    /// [`INPUT_RESOURCE`]: super::INPUT_RESOURCE
    /// [`PollerThreadLifecycleState`]: PollerThreadLifecycleState
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`broadcast::Sender::subscribe()`]: tokio::sync::broadcast::Sender::subscribe
    /// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
    /// [`mio::Poll`]: mio::Poll
    /// [`stdin`]: std::io::stdin
    pub fn allocate() -> PollerSubscriptionHandle {
        let mut guard = INPUT_RESOURCE.lock().expect(
            "INPUT_RESOURCE mutex poisoned: another thread panicked while holding this lock. \
             Terminal input is unavailable. This is unrecoverable.",
        );

        // Fast-path thread reuse: If thread is still running, skip spawning a new one.
        let apply_fast_path_thread_reuse =
            guard.as_ref().is_some_and(|input_resource_state| {
                input_resource_state.lifecycle.metadata.is_running()
                    == ThreadLiveness::Running
            });

        if !apply_fast_path_thread_reuse {
            // Create a poll and waker.
            let new_poll = Poll::new().expect(
                "Failed to create mio::Poll: OS denied epoll/kqueue creation. \
                 Check ulimit -n (max open files) or /proc/sys/fs/epoll/max_user_watches.",
            );
            let new_registry = new_poll.registry();
            let new_waker =
                Waker::new(new_registry, SourceKindReady::ReceiverDropWaker.to_token())
                    .expect(
                        "Failed to create mio::Waker: eventfd/pipe creation failed. \
                     Check ulimit -n (max open files).",
                    );

            // Create a new lifecycle state for this thread.
            let thread_lifecycle_state = PollerThreadLifecycleState::new();

            // Spawn the thread with a handle to the shared state.
            MioPollerThread::new(new_poll, thread_lifecycle_state.clone_handle());

            // Save the state & waker to the global singleton.
            guard.replace(InputResourceState {
                lifecycle: thread_lifecycle_state,
                waker: Arc::new(new_waker),
            });
        }

        // Guard is guaranteed to be Some at this point.
        debug_assert!(guard.is_some());
        let input_resource_state = guard.as_ref().unwrap();

        PollerSubscriptionHandle {
            maybe_poller_rx: Some(
                input_resource_state.lifecycle.tx_poller_event.subscribe(),
            ),
            mio_poller_thread_waker: Arc::clone(&input_resource_state.waker),
        }
    }

    /// Checks if the [`mio_poller`] thread is currently running.
    ///
    /// This is useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// - [`ThreadLiveness::Running`] if the thread is running.
    /// - [`ThreadLiveness::Terminated`] if [`INPUT_RESOURCE`] is uninitialized or the
    ///   thread has exited.
    ///
    /// See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for details on how threads
    /// spawn and exit.
    ///
    /// [Device Lifecycle]: DirectToAnsiInputDevice#device-lifecycle
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub fn is_thread_running() -> ThreadLiveness {
        INPUT_RESOURCE
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .as_ref()
                    .map(|state| state.lifecycle.metadata.is_running())
            })
            .unwrap_or(ThreadLiveness::Terminated)
    }

    /// Queries how many receivers are subscribed to the input broadcast channel.
    ///
    /// This is useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// The number of active receivers, or `0` if [`INPUT_RESOURCE`] is uninitialized.
    ///
    /// The [`mio_poller`] thread exits gracefully when this count reaches `0` (all
    /// receivers dropped). See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for
    /// details.
    ///
    /// [Device Lifecycle]: DirectToAnsiInputDevice#device-lifecycle
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    pub fn get_receiver_count() -> usize {
        INPUT_RESOURCE
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .as_ref()
                    .map(|state| state.lifecycle.tx_poller_event.receiver_count())
            })
            .unwrap_or(0)
    }

    /// Returns the current thread generation number.
    ///
    /// Each time a new [`mio_poller`] thread is spawned, the generation increments. This
    /// allows tests to verify whether a thread was reused or relaunched:
    ///
    /// - **Same generation**: Thread was reused (device B subscribed before thread
    ///   exited).
    /// - **Different generation**: Thread was relaunched (a new thread was spawned).
    ///
    /// # Returns
    ///
    /// The current generation number, or `0` if [`INPUT_RESOURCE`] is uninitialized.
    ///
    /// See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for details on thread
    /// spawn/exit/relaunch.
    ///
    /// [Device Lifecycle]: DirectToAnsiInputDevice#device-lifecycle
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    pub fn get_thread_generation() -> u8 {
        INPUT_RESOURCE
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .as_ref()
                    .map(|state| state.lifecycle.metadata.generation)
            })
            .unwrap_or(0)
    }

    /// Subscribe to input events from an existing thread.
    ///
    /// This is a lightweight operation that creates a new subscriber to the existing
    /// broadcast channel. Use this for additional consumers (logging, debugging, etc.)
    /// after a [`DirectToAnsiInputDevice`] has been created.
    ///
    /// When the returned handle is dropped, it notifies the [`mio_poller`] thread to
    /// check if it should exit (when all subscribers are dropped, the thread exits).
    ///
    /// # Panics
    ///
    /// - If the [`INPUT_RESOURCE`] mutex is poisoned (another thread panicked while
    ///   holding the lock).
    /// - If no device exists yet. Call [`allocate`] first.
    ///
    /// [`mio_poller`]: super::super::mio_poller
    pub fn subscribe_to_existing() -> PollerSubscriptionHandle {
        let guard = INPUT_RESOURCE.lock().expect(
            "INPUT_RESOURCE mutex poisoned: another thread panicked while holding this lock.",
        );

        let state = guard.as_ref().expect(
            "subscribe_to_existing() called before DirectToAnsiInputDevice::new(). \
             Create a device first, then call device.subscribe().",
        );

        PollerSubscriptionHandle {
            maybe_poller_rx: Some(state.lifecycle.tx_poller_event.subscribe()),
            mio_poller_thread_waker: Arc::clone(&state.waker),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DirectToAnsiInputDevice
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Async input device for [`DirectToAnsi`] backend.
///
/// One of two real [`InputDevice`] backends (the other being [`CrosstermInputDevice`]).
/// Selected via [`TERMINAL_LIB_BACKEND`] on Linux; talks directly to the terminal using
/// ANSI/VT100 protocols with zero external dependencies. Uses the **crossterm `more` flag
/// pattern** for reliable ESC key disambiguation without fixed timeouts.
///
/// This is a **thin wrapper** that delegates to [`INPUT_RESOURCE`] for
/// [`std::io::Stdin`] reading and buffer management. The global resource pattern mirrors
/// crossterm's architecture, ensuring [`stdin`] handles persist across device lifecycle
/// boundaries.
///
/// Manages asynchronous reading from terminal [`stdin`] via dedicated thread + channel:
/// - [`stdin`] channel receiver (process global singleton, outlives device instances)
/// - Parsing happens in the reader thread using the `more` flag pattern
/// - Smart ESC disambiguation: waits for more bytes only when data is likely pending
/// - Dispatch to protocol parsers (keyboard, mouse, terminal events, UTF-8)
///
/// # Architecture
///
/// This module provides cancel-safe async terminal input for a process, by bridging a
/// synchronous [`mio`]-based reader thread with async consumers via a [`broadcast`]
/// channel. It handles keyboard input (including ANSI escape sequences for arrow keys,
/// function keys, etc.) and terminal resize signals ([`SIGWINCH`]) reliably, even over
/// [SSH].
///
/// The broadcast channel allows **multiple async consumers** to receive all input events
/// simultaneously; this can be use useful for debugging, logging, or event recording
/// alongside the "primary" TUI app consumer.
///
/// ## Why This Design? (Historical Context)
///
/// Our original "Tokio-heavy" approach created a [`DirectToAnsiInputDevice`] instance
/// on-demand, one-instance-per-app (which was not process-bound, rather it was bound
/// to each app-instance). It used:
/// - [`tokio::io::stdin()`] for input handling
/// - [`tokio::signal`] for [`SIGWINCH`] handling
///
/// ### The Problems
///
/// **This caused three problems that led us to the current design:**
///
/// 1. **UI freeze on resize.** [Tokio's stdin] uses a blocking threadpool. In the past,
///    in [`next()`], when [`tokio::select!`] cancelled a [`tokio::io::stdin()`] read to
///    handle [`SIGWINCH`], the blocking read kept running in the background. The next
///    read conflicted with this "zombie" read leading to a UI freeze.
///
/// 2. **Dropped keystrokes.** Creating a new [`stdin`] handle lost access to data already
///    in the kernel buffer. When TUI "App A" exited and "App B" started, keystrokes typed
///    during the transition vanished. This was easily reproducible by:
///    - Running `cargo run --examples tui_apps`.
///    - Starting one app, exiting, **dropped keystrokes**, starting another, exit,
///      **dropped keystrokes**, starting another, and so on.
///
/// 3. **Flawed `ESC` detection over [SSH].** Our original approach had flawed logic for
///    distinguishing the `ESC` key from escape sequences (like `ESC [ A` for Up Arrow).
///    It worked locally but failed over [SSH]. We now use [`crossterm`]'s `more` flag
///    heuristic (see [ESC Detection Limitations] in [`MioPollerThread`]).
///
/// ### The Solution
///
/// A **process bound global singleton** with a dedicated reader thread that is the
/// **designated reader** of [`stdin`]. The thread uses [`mio::Poll`] to wait on both
/// [`stdin`] data and [`SIGWINCH`] signals.
///
/// <div class="warning">
///
/// **No exclusive access**: Any thread can call [`std::io::stdin()`] and read from it—
/// there is no OS or Rust mechanism to prevent this. If another thread reads from
/// [`stdin`], bytes will be stolen, causing interleaved reads that corrupt the input
/// stream and break the VT100 parser. See [No exclusive access] in [`MioPollerThread`].
///
/// </div>
///
/// Although sync and blocking, [`mio`] is efficient. It uses OS primitives ([`epoll`] on
/// Linux, [`kqueue`] on BSD/macOS) that put the thread to sleep until data arrives,
/// consuming no CPU while waiting. See [How It Works] in [`MioPollerThread`] for details.
///
/// ```text
///     Process-bound Global Singleton                       Async Consumers
/// ┌─────────────────────────────────────┐           ┌─────────────────────────────┐
/// │ Sync Blocking (std::thread + mio)   │           │ Primary: TUI input handler  │
/// │                                     │           │ Optional: Debug logger      │
/// │ Designated reader of:               │           │ Optional: Event recorder    │
/// │   • stdin (not exclusive access!)   │           │                             │
/// │   • Parser state                    │           │                             │
/// │   • SIGWINCH watcher                │           │                             │
/// │                                     │ broadcast │                             │
/// │ tx.send(InputEvent)  ───────────────┼───────────▶ rx.recv().await             │
/// │                                     │ channel   │ (cancel-safe, fan-out!)     │
/// └─────────────────────────────────────┘    │      └─────────────────────────────┘
///                                            ▼
///                                     Sync -> Async Bridge
/// ```
///
/// This solves the first two problems completely:
/// 1. **Cancel-safe**: Channel receive is truly async - no zombie reads
/// 2. **Data preserved**: Global state survives TUI app lifecycle transitions in the same
///    process.
///
/// To solve the third problem for `ESC` detection, we use [`crossterm`]'s `more` flag
/// heuristic (see [ESC Detection Limitations] in [`MioPollerThread`]).
///
/// ## Architecture Overview
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────────────┐
/// │ INPUT_RESOURCE (static LazyLock<Mutex<...>>)                            │
/// │ internal:                                                               │
/// │  • mio-poller thread: holds tx, reads stdin, runs vt100 parser          │
/// │ external:                                                               │
/// │  • stdin_rx: broadcast receiver (async consumers recv() from here)      │
/// └───────────────────────────────────────┬─────────────────────────────────┘
///                                         │
///            ┌────────────────────────────┴─────────────────────────┐
///            │                                                      │
/// ┌──────────▼───────────────────┐            ┌─────────────────────▼───────┐
/// │ DirectToAnsiInputDevice A    │            │ DirectToAnsiInputDevice B   │
/// │   (TUI App context)          │            │   (Readline context)        │
/// │   • Zero-sized handle        │            │   • Zero-sized handle       │
/// │   • Delegates to global      │            │   • Delegates to global     │
/// └──────────────────────────────┘            └─────────────────────────────┘
///
/// 🎉 Data preserved during transitions - same channel used throughout!
/// ```
///
/// The key insight: [`stdin`] handles must persist across device lifecycle boundaries.
/// Multiple [`DirectToAnsiInputDevice`] instances can be created and dropped, but they
/// all share the same underlying channel and process global (singleton) reader thread.
///
/// See [`MioPollerThread`] for details on how the mio poller thread works, including
/// file descriptor handling, parsing, thread lifecycle, and ESC detection limitations.
///
/// # Device Lifecycle
///
/// A single process can create and drop [`DirectToAnsiInputDevice`] instances repeatedly.
/// The global [`INPUT_RESOURCE`] **static** persists, but the **thread** spawns and exits
/// with each app lifecycle:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────────────┐
/// │ PROCESS LIFETIME                                                              │
/// │                                                                               │
/// │ INPUT_RESOURCE: LazyLock<Mutex<Option<PollerThreadLifecycleState>>>           │
/// │ (static persists, but contents are replaced on each thread spawn)             │
/// │                                                                               │
/// │ ═════════════════════════════════════════════════════════════════════════════ │
/// │                                                                               │
/// │ ┌───────────────────────────────────────────────────────────────────────────┐ │
/// │ │ TUI app A lifecycle                                                       │ │
/// │ │                                                                           │ │
/// │ │  1. DirectToAnsiInputDevice::new()                                        │ │
/// │ │  2. next() → allocate()                                                   │ │
/// │ │  3. INPUT_RESOURCE is None → initialize_input_resource()                  │ │
/// │ │       • Creates PollerThreadLifecycleState { tx, liveness: Running }      │ │
/// │ │       • Spawns mio-poller thread #1                                       │ │
/// │ │       • thread #1 owns MioPollerThread struct                             │ │
/// │ │  4. TUI app A runs, receiving events from rx                              │ │
/// │ │  5. TUI app A exits → device dropped → receiver dropped                   │ │
/// │ │  6. Thread #1 detects 0 receivers → exits gracefully                      │ │
/// │ │  7. MioPollerThread::drop() → liveness = Terminated                       │ │
/// │ │                                                                           │ │
/// │ └───────────────────────────────────────────────────────────────────────────┘ │
/// │                                                                               │
/// │ ┌───────────────────────────────────────────────────────────────────────────┐ │
/// │ │ TUI app B lifecycle                                                       │ │
/// │ │                                                                           │ │
/// │ │  1. DirectToAnsiInputDevice::new()                                        │ │
/// │ │  2. next() → allocate()                                                   │ │
/// │ │  3. INPUT_RESOURCE has state, but liveness == Terminated                  │ │
/// │ │       → needs_init = true → initialize_input_resource()                   │ │
/// │ │       • Creates NEW PollerThreadLifecycleState { tx, liveness: Running }  │ │
/// │ │       • Spawns mio-poller thread #2 (NOT the same as #1!)                 │ │
/// │ │       • thread #2 owns its own MioPollerThread struct                     │ │
/// │ │  4. TUI app B runs, receiving events from rx                              │ │
/// │ │  5. TUI app B exits → device dropped → receiver dropped                   │ │
/// │ │  6. Thread #2 detects 0 receivers → exits gracefully                      │ │
/// │ │  7. MioPollerThread::drop() → liveness = Terminated                       │ │
/// │ │                                                                           │ │
/// │ └───────────────────────────────────────────────────────────────────────────┘ │
/// │                                                                               │
/// │ ... pattern repeats for App C, D, etc. ...                                    │
/// │                                                                               │
/// └───────────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// **Key insight**: The [`mio_poller`] thread is NOT persistent across the lifetime of
/// the process. Each app lifecycle spawns a new thread. The [`metadata`] field
/// enables this by allowing [`allocate()`] to detect when a thread
/// has exited and spawn a new one.
///
/// ## Why Keystrokes Aren't Lost During Transitions
///
/// Given the [Device Lifecycle] above—where threads exit and restart between apps—a
/// natural question arises: **why don't keystrokes get lost during the transition?**
///
/// The historical problem (see [The Problems]) was that the old "Tokio-heavy" approach
/// created a new [`tokio::io::stdin()`] handle per app. When App A exited and App B
/// started, keystrokes typed during the transition vanished because
/// [`tokio::io::stdin()`] uses **application-level buffering**—when that handle is
/// dropped, its internal buffer is lost forever.
///
/// The current design provides **three layers of protection**:
///
/// | Layer                       | Protection Mechanism                                                                                           |
/// | :-------------------------- | :------------------------------------------------------------------------------------------------------------- |
/// | **Kernel buffer persists**  | Even after thread restart, unread bytes remain in the kernel's [`fd`] `0` buffer                               |
/// | **No app-level buffering**  | Direct [`std::io::Stdin`] reads with immediate parsing—no internal buffer to lose                              |
/// | **Fast-path reuse**         | If new app subscribes before thread exits, existing thread continues; see [`pty_mio_poller_thread_reuse_test`] |
///
/// ### Data Flow During App Switching
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────────────┐
/// │ App A exits, drops receiver                                              │
/// │   • PollerSubscriptionHandle::drop() calls waker.wake()                  │
/// │   • Thread may continue running (fast) OR exit (slow)                    │
/// └──────────────────────────────┬───────────────────────────────────────────┘
///                                │
/// ┌──────────────────────────────▼───────────────────────────────────────────┐
/// │ User types keystrokes during transition                                  │
/// │   • Bytes arrive in kernel stdin buffer (fd 0)                           │
/// │   • If thread still running: reads immediately, sends to channel         │
/// │   • If thread exited: kernel buffer holds bytes until new thread reads   │
/// └──────────────────────────────┬───────────────────────────────────────────┘
///                                │
/// ┌──────────────────────────────▼───────────────────────────────────────────┐
/// │ App B starts, calls DirectToAnsiInputDevice::new()                       │
/// │   • allocate() checks liveness flag                                      │
/// │   • If Running: reuses existing thread (no gap in reading)               │
/// │   • If Terminated: spawns new thread → reads kernel buffer → no data loss│
/// └──────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// The key insight: the **kernel's [`stdin`] buffer for [`fd`] `0`
/// persists** regardless of which thread is reading. Unlike [`tokio::io::stdin()`]'s
/// application-level buffer, the kernel buffer survives handle creation/destruction. When
/// a new thread calls [`std::io::stdin()`], it gets a handle to the **same kernel
/// buffer** containing any unread bytes.
///
/// ## Call Chain to [`allocate()`]
///
/// ```text
/// DirectToAnsiInputDevice::new()                (input_device.rs)
///     │
///     └─► allocate()                            (input_device.rs)
///             │
///             ├─► INPUT_RESOURCE.lock()
///             │
///             ├─► needs_init = None || liveness == Terminated
///             │       │
///             │       └─► if needs_init: initialize_input_resource()
///             │               │
///             │               ├─► Create PollerThreadLifecycleState
///             │               ├─► MioPollerThread::new(state.clone())
///             │               └─► guard.replace(state)
///             │
///             └─► return state.tx_input_event.subscribe() ← new broadcast receiver
///
/// DirectToAnsiInputDevice::next()               (input_device.rs)
///     │
///     └─► stdin_rx.recv().await
/// ```
///
/// **Key points:**
/// - [`DirectToAnsiInputDevice`] is a thin wrapper holding [`PollerSubscriptionHandle`]
/// - Global state ([`INPUT_RESOURCE`]) persists - channel and thread survive device drops
/// - Eager subscription - each device subscribes at construction time in [`new()`]
/// - Thread liveness check - if thread died, next subscribe reinitializes everything
///
/// # Full I/O Pipeline
///
/// This device sits in the backend executor layer, bridging raw I/O to the protocol
/// parser, then converting protocol IR to the public API:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────┐
/// │ Raw ANSI bytes: "1B[A" (hex)                                      │
/// │ std::io::stdin in mio-poller thread (INPUT_RESOURCE)              │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ mio-poller thread (input_device.rs)                               │
/// │   • mio::Poll waits on stdin data + SIGWINCH signals              │
/// │   • Parses bytes using `more` flag for ESC disambiguation         │
/// │   • Applies paste state machine                                   │
/// │   • Sends InputEvent through broadcast channel                    │
/// │                                                                   │
/// │ vt_100_terminal_input_parser/ (Protocol Layer - IR)               │
/// │   try_parse_input_event() dispatches to:                          │
/// │   ├─ parse_keyboard_sequence() → VT100InputEventIR::Keyboard      │
/// │   ├─ parse_mouse_sequence()    → VT100InputEventIR::Mouse         │
/// │   ├─ parse_terminal_event()    → VT100InputEventIR::Focus/Resize  │
/// │   └─ parse_utf8_text()         → VT100InputEventIR::Keyboard      │
/// │                                                                   │
/// │ protocol_conversion.rs (IR → Public API)                          │
/// │   convert_input_event()           VT100InputEventIR → InputEvent  │
/// │   convert_key_code_to_keypress()  VT100KeyCodeIR → KeyPress       │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │ broadcast channel
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ THIS DEVICE: DirectToAnsiInputDevice (Backend Executor)           │
/// │   • Zero-sized handle struct (delegates to INPUT_RESOURCE)        │
/// │   • Receives pre-parsed InputEvent from channel                   │
/// │   • Resize events include Option<Size> (fallback to get_size())   │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ Public API (Application Layer)                                    │
/// │   InputEvent::Keyboard(KeyPress)                                  │
/// │   InputEvent::Mouse(MouseInput)                                   │
/// │   InputEvent::Resize(Size)                                        │
/// │   InputEvent::Focus(FocusEvent)                                   │
/// │   InputEvent::Paste(String)                                       │
/// └───────────────────────────────────────────────────────────────────┘
/// ```
///
/// For details on thread lifecycle (spawn/exit/relaunch), see the [Device Lifecycle]
/// section above.
///
/// # Data Flow Diagram
///
/// Here's the complete data flow for [`next()`]:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────────┐
/// │ 0. DirectToAnsiInputDevice::new() called                                  │
/// │    └─► allocate() (eager, at construction time)                           │
/// │        └─► If no thread running: spawns mio-poller thread                 │
/// │                                                                           │
/// │ 1. next() called                                                          │
/// │    └─► Loop: stdin_rx.recv().await                                        │
/// │         ├─► Event(e) → return e                                           │
/// │         ├─► Resize(Some(size)) → return InputEvent::Resize(size)          │
/// │         ├─► Resize(None) → retry get_size(), return if Ok, else continue  │
/// │         └─► Eof/Error → return None                                       │
/// └───────────────────────────────────▲───────────────────────────────────────┘
///                                     │ broadcast channel
/// ┌───────────────────────────────────┴───────────────────────────────────────┐
/// │ 2. mio-poller thread                                                      │
/// │    std::thread::spawn("mio-poller")                                       │
/// │                                                                           │
/// │    Uses mio::Poll to wait on stdin data + SIGWINCH signals:               │
/// │    ┌───────────────────────────────────────────────────────────────────┐  │
/// │    │ loop {                                                            │  │
/// │    │   poll.poll(&mut events, None)?;        // Wait for stdin/signal  │  │
/// │    │   let n = stdin.read(&mut buffer)?;     // Read available bytes   │  │
/// │    │   let more = n == TTY_BUFFER_SIZE;      // ESC disambiguation     │  │
/// │    │   parser.advance(&buffer[..n], more);   // Parse with `more` flag │  │
/// │    │   for event in parser { tx.send(Event(event))?; }                 │  │
/// │    │ }                                                                 │  │
/// │    └───────────────────────────────────────────────────────────────────┘  │
/// │    See module docs for thread lifecycle (exits when all receivers drop)   │
/// └───────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Underlying Protocol Parser
///
/// - [`vt_100_terminal_input_parser`]: The protocol parser that converts raw bytes to
///   [`VT100InputEventIR`]. This device calls [`try_parse_input_event`] to perform the
///   actual parsing.
///
/// # ESC Key Disambiguation (crossterm `more` flag pattern)
///
/// **The Problem**: Distinguishing ESC key presses from escape sequences (e.g., Up Arrow
/// = `ESC [ A`). When we see a lone `0x1B` byte, is it the ESC key or the start of an
/// escape sequence?
///
/// **The Solution**: We use crossterm's `more` flag pattern—a clever heuristic based on
/// read buffer fullness:
///
/// ```text
/// let n = stdin.read(&mut buffer)?;  // Read available bytes
/// let more = n == TTY_BUFFER_SIZE;   // true if buffer was filled completely
///
/// // In parser:
/// if buffer == [ESC] && more {
///     return None;  // Wait for more bytes (likely escape sequence)
/// } else if buffer == [ESC] && !more {
///     return ESC key;  // No more data, user pressed ESC
/// }
/// ```
///
/// ## How It Works
///
/// - **`more = true`**: Read filled the entire buffer, meaning more data is likely
///   waiting in the kernel buffer. Wait before deciding—this `ESC` is probably the start
///   of an escape sequence.
/// - **`more = false`**: Read returned fewer bytes than buffer size, meaning we've
///   drained all available input. A lone `ESC` is the ESC key.
///
/// ## Why This Works
///
/// Terminal emulators send escape sequences atomically in a single `write()` syscall.
/// When you press Up Arrow, the terminal sends `ESC [ A` (3 bytes) together. The kernel
/// buffers these bytes, and our `read()` typically gets all of them at once.
///
/// ```text
/// User presses Up Arrow
///   ↓
/// Terminal: write(stdout, "1B[A" (hex), 3)  ← One syscall, 3 bytes
///   ↓
/// Kernel buffer: [1B, 5B, 41]               ← All bytes arrive together
///   ↓
/// stdin.read() → 3 bytes                    ← We get all 3 bytes
///   ↓
/// more = (3 == 1024) = false                ← Buffer not full
///   ↓
/// Parser sees [ESC, '[', 'A']               → Up Arrow event ✓
/// ```
///
/// ## SSH and High-Latency Connections
///
/// Over SSH with network latency, bytes might arrive in separate packets. The `more`
/// flag handles this correctly:
///
/// ```text
/// First packet:  [ESC]       read() → 1 byte, more = false
///                            BUT: next poll() wakes immediately when more data arrives
/// Second packet: ['[', 'A']  read() → 2 bytes
///                            Parser accumulates: [ESC, '[', 'A'] → Up Arrow ✓
/// ```
///
/// The key insight: if bytes arrive separately, the next `mio::Poll` wake happens
/// almost immediately when more data arrives. The parser accumulates bytes across
/// reads, so escape sequences are correctly reassembled.
///
/// ## Attribution
///
/// This pattern is adapted from crossterm's `mio.rs` implementation. See the
/// [Architecture] section above for details on our mio-based architecture.
///
/// We looked at [`crossterm`]'s source code for design inspiration:
/// 1. **Global state pattern**: [`crossterm`] uses a global [`INTERNAL_EVENT_READER`]
///    that holds the `tty` file descriptor and event buffer, ensuring data in the kernel
///    buffer is not lost when [`EventStream`] instances are created and dropped. And we
///    have the same global singleton pattern here.
/// 2. **[`mio`]-based polling**: Their [`mio.rs`] uses [`mio::Poll`] with
///    [`signal-hook-mio`] for [`SIGWINCH`] and we do the same.
/// 3. **ESC disambiguation**: The `more` flag heuristic for distinguishing ESC key from
///    escape sequences without timeouts. We inherit both its benefits (zero latency) and
///    limitations (see [ESC Detection Limitations] in [`MioPollerThread`]).
/// 4. **Process-lifetime cleanup**: They rely on OS cleanup at process exit rather than
///    explicit thread termination, and so do we.
///
/// # Drop behavior
///
/// When this device is dropped:
/// 1. [`super::at_most_one_instance_assert::release()`] is called, allowing a new device
///    to be created.
/// 2. Rust's drop glue drops [`Self::resource_handle`], triggering
///    [`PollerSubscriptionHandle`'s drop behavior] (thread lifecycle protocol).
///
/// For the complete lifecycle diagram including the [race condition] where a fast
/// subscriber can reuse the existing thread, see [`PollerThreadLifecycleState`].
///
/// [Architecture]: Self#architecture
/// [Device Lifecycle]: Self#device-lifecycle
/// [ESC Detection Limitations]: super::mio_poller::MioPollerThread#esc-detection-limitations
/// [How It Works]: super::mio_poller::MioPollerThread#how-it-works
/// [No exclusive access]: super::mio_poller#no-exclusive-access
/// [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
/// [The Problems]: Self#the-problems
/// [Tokio's stdin]: tokio::io::stdin
/// [`CrosstermInputDevice`]: crate::tui::terminal_lib_backends::crossterm_backend::CrosstermInputDevice
/// [`DirectToAnsi`]: mod@crate::direct_to_ansi
/// [`EventStream`]: crossterm::event::EventStream
/// [`INPUT_RESOURCE`]: INPUT_RESOURCE
/// [`INTERNAL_EVENT_READER`]: https://github.com/crossterm-rs/crossterm/blob/0.29/src/event.rs#L149
/// [`InputDevice`]: crate::InputDevice
/// [`MioPollerThread`]: super::mio_poller::MioPollerThread
/// [`PollerSubscriptionHandle`'s drop behavior]: PollerSubscriptionHandle#drop-behavior
/// [`PollerThreadLifecycleState`]: super::mio_poller::PollerThreadLifecycleState
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
/// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`allocate()`]: guarded_ops::allocate
/// [`broadcast`]: tokio::sync::broadcast
/// [`crossterm`]: crossterm
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
/// [`metadata`]: super::mio_poller::PollerThreadLifecycleState::metadata
/// [`mio.rs`]: https://github.com/crossterm-rs/crossterm/blob/0.29/src/event/source/unix/mio.rs
/// [`mio::Poll`]: mio::Poll
/// [`mio_poller`]: super::mio_poller
/// [`mio`]: mio
/// [`new()`]: Self::new
/// [`next()`]: Self::next
/// [`pty_mio_poller_thread_reuse_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_thread_reuse_test
/// [`signal-hook-mio`]: signal_hook_mio
/// [`std::io::Stdin`]: std::io::Stdin
/// [`std::io::stdin()`]: std::io::stdin
/// [`stdin`]: std::io::stdin
/// [`super::at_most_one_instance_assert::release()`]: super::at_most_one_instance_assert::release
/// [`tokio::io::stdin()`]: tokio::io::stdin
/// [`tokio::select!`]: tokio::select
/// [`tokio::signal`]: tokio::signal
/// [`try_parse_input_event`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser
/// [race condition]: super::mio_poller::PollerThreadLifecycleState#the-inherent-race-condition
pub struct DirectToAnsiInputDevice {
    /// This device's subscription to the global input broadcast channel.
    ///
    /// Initialized eagerly in [`new()`] to ensure the thread sees a receiver if one is
    /// needed. This is critical for correct thread lifecycle management: if a device is
    /// dropped and a new one created "immediately", the new device's subscription must
    /// be visible to the thread when it checks [`receiver_count()`].
    ///
    /// Uses [`PollerSubscriptionHandle`] to wake the [`mio_poller`] thread when
    /// dropped, allowing thread lifecycle management.
    ///
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    /// [`new()`]: Self::new
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    pub resource_handle: PollerSubscriptionHandle,
}

impl DirectToAnsiInputDevice {
    /// Create the input device.
    ///
    /// # Singleton Semantics
    ///
    /// Only ONE [`DirectToAnsiInputDevice`] can exist at a time. There is only one
    /// [`stdin`], so having multiple "devices" is semantically incorrect.
    ///
    /// To get additional receivers for logging, debugging, or multiple consumers, use
    /// [`subscribe()`] instead of calling [`new()`] again.
    ///
    /// # Panics
    ///
    /// Panics if called while another device exists. The panic message guides you to use
    /// [`subscribe()`] for additional receivers.
    ///
    /// # Thread Lifecycle
    ///
    /// Creates the [`mio_poller`] thread **eagerly** if it doesn't exist. This is
    /// critical for correct lifecycle: if device A is dropped and device B is created
    /// immediately, device B's subscription must be visible to the thread when it
    /// checks [`receiver_count()`] (before it decides to exit). See the [race condition
    /// documentation] in [`PollerSubscriptionHandle`] for details.
    ///
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    /// [`new()`]: Self::new
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`stdin`]: std::io::Stdin
    /// [`subscribe()`]: Self::subscribe
    /// [race condition documentation]: PollerSubscriptionHandle#race-condition-and-correctness
    #[must_use]
    pub fn new() -> Self {
        super::at_most_one_instance_assert::claim_and_assert();
        Self {
            resource_handle: guarded_ops::allocate(),
        }
    }

    /// Get an additional subscriber (async consumer) to input events.
    ///
    /// Use this for logging, debugging, or multiple concurrent consumers. Each
    /// subscriber independently receives all input events. When dropped, notifies
    /// the [`mio_poller`] thread to check if it should exit.
    ///
    /// Even though we don't use `&self` in this method, it creates a capability gate. You
    /// can only [`subscribe()`] if you have allocated a device using [`new()`], enforcing
    /// the invariant that subscription requires prior allocation.
    ///
    /// See [`pty_mio_poller_subscribe_test`] for integration tests demonstrating
    /// broadcast semantics (both device and subscriber receive the same events).
    ///
    /// [`mio_poller`]: super::mio_poller
    /// [`new()`]: Self::new
    /// [`pty_mio_poller_subscribe_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_subscribe_test
    /// [`subscribe()`]: Self::subscribe
    #[must_use]
    pub fn subscribe(&self) -> PollerSubscriptionHandle {
        guarded_ops::subscribe_to_existing()
    }

    /// Read the next input event asynchronously.
    ///
    /// # Returns
    ///
    /// `None` if stdin is closed ([`EOF`]). Or [`InputEvent`] variants for:
    /// - **Keyboard**: Character input, arrow keys, function keys, modifiers (with 0ms
    ///   `ESC` latency)
    /// - **Mouse**: Clicks, drags, motion, scrolling with position and modifiers
    /// - **Resize**: Terminal window size change (rows, cols)
    /// - **Focus**: Terminal gained/lost focus
    /// - **Paste**: Bracketed paste mode text
    ///
    /// # Usage
    ///
    /// This method is called once by the main event loop of the program using this
    /// [`InputDevice`] and this [`DirectToAnsiInputDevice`] struct is persisted
    /// for the entire lifetime of the program's event loop. Typical usage pattern:
    ///
    /// ```no_run
    /// # use r3bl_tui::DirectToAnsiInputDevice;
    /// # use r3bl_tui::InputEvent;
    /// # use tokio::signal;
    ///
    /// #[tokio::main]
    /// async fn main() -> miette::Result<()> {
    ///     // Create device once at startup, reuse until program exit
    ///     let mut input_device = DirectToAnsiInputDevice::new();
    /// #   let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
    /// #       .map_err(|e| miette::miette!("Failed to setup signal handler: {}", e))?;
    ///
    ///     // Main event loop - handle multiple concurrent event sources with tokio::select!
    ///     loop {
    ///         tokio::select! {
    ///             // Handle terminal input events
    ///             input_event = input_device.next() => {
    ///                 match input_event {
    ///                     Some(InputEvent::Keyboard(key_press)) => {
    ///                         // Handle keyboard input
    ///                     }
    ///                     Some(InputEvent::Mouse(mouse_input)) => {
    ///                         // Handle mouse input
    ///                     }
    ///                     Some(InputEvent::Resize(size)) => {
    ///                         // Handle terminal resize
    ///                     }
    ///                     Some(InputEvent::BracketedPaste(text)) => {
    ///                         // Handle bracketed paste
    ///                     }
    ///                     Some(InputEvent::Focus(_)) => {
    ///                         // Handle focus events
    ///                     }
    ///                     Some(_) => {
    ///                         // Handle future/unknown event types
    ///                     }
    ///                     None => {
    ///                         // stdin closed (EOF) - signal program to exit
    ///                         break;
    ///                     }
    ///                 }
    ///             }
    ///             // Handle system signals (e.g., Ctrl+C)
    ///             _ = sigint.recv() => {
    /// #               eprintln!("Received SIGINT, shutting down...");
    /// #               break;
    ///             }
    ///             // Handle other concurrent tasks as needed
    ///             // _ = some_background_task => { ... }
    ///         }
    ///     }
    /// #   Ok(())
    /// }
    /// ```
    ///
    /// **Key points:**
    /// - The device can be **created and dropped multiple times** - global state persists
    /// - This method is **called repeatedly** by the main event loop via
    ///   [`InputDevice::next()`], which dispatches to [`Self::next()`]
    /// - **Buffer state is preserved** across device lifetimes via [`INPUT_RESOURCE`]
    /// - Returns `None` when stdin is closed (program should exit)
    ///
    /// # Global State
    ///
    /// This method accesses the global input resource ([`INPUT_RESOURCE`]) which
    /// holds:
    /// - The channel receiver for stdin data and resize signals (from dedicated reader
    ///   thread using [`mio::Poll`])
    /// - The parse buffer and position
    /// - The event queue for buffered events
    /// - The paste collection state
    ///
    /// Note: `SIGWINCH` signals are now handled by the dedicated reader thread via
    /// [`mio::Poll`] and [`signal_hook_mio`], arriving as [`PollerEvent::Signal`]
    /// through the same channel as stdin data.
    ///
    /// See the [Architecture] section for the rationale behind this design.
    ///
    /// # Implementation
    ///
    /// Async loop receiving pre-parsed events:
    /// 1. Check event queue for buffered events (from previous reads)
    /// 2. Wait for events from stdin reader channel (yields until data ready)
    /// 3. Apply paste state machine and return event
    ///
    /// Events arrive fully parsed from the reader thread. See [struct-level
    /// documentation] for zero-latency ESC detection.
    ///
    /// # Cancel Safety
    ///
    /// This method is cancel-safe. The internal broadcast channel receive
    /// ([`tokio::sync::broadcast::Receiver::recv`]) is truly cancel-safe: if
    /// cancelled, the data remains in the channel for the next receive.
    ///
    /// See the [Architecture] section for why we use a dedicated thread with
    /// [`mio::Poll`] and channel instead of [`tokio::io::stdin()`] (which is NOT
    /// cancel-safe).
    ///
    /// # Panics
    ///
    /// Panics if called after [`Drop`] has already been invoked (internal invariant).
    ///
    /// [Architecture]: Self#architecture
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`INPUT_RESOURCE`]: INPUT_RESOURCE
    /// [`InputDevice::next()`]: crate::InputDevice::next
    /// [`InputDevice`]: crate::InputDevice
    /// [`PollerEvent::Signal`]: super::channel_types::PollerEvent::Signal
    /// [`Self::next()`]: Self::next
    /// [`mio::Poll`]: mio::Poll
    /// [`signal_hook_mio`]: signal_hook_mio
    /// [`tokio::io::stdin()`]: tokio::io::stdin
    /// [`tokio::sync::broadcast::Receiver::recv`]: tokio::sync::broadcast::Receiver::recv
    /// [struct-level documentation]: Self
    pub async fn next(&mut self) -> Option<InputEvent> {
        // Receiver was subscribed eagerly in new() - just use it.
        let res_handle = &mut self.resource_handle;

        // Wait for fully-formed InputEvents through the broadcast channel.
        loop {
            let poller_rx_result = res_handle
                .maybe_poller_rx
                .as_mut()
                .expect("PollerEventReceiver is None - this is a bug")
                .recv()
                .await;

            let poller_event = match poller_rx_result {
                // Got a message from the channel.
                Ok(msg) => msg,
                // The sender was dropped.
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Channel closed - reader thread exited.
                    return None;
                }
                // This receiver fell behind and messages were dropped.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    // This receiver fell behind - some messages were dropped.
                    // Log and continue from the current position.
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        eprintln!(
                            "DirectToAnsiInputDevice: receiver lagged, skipped {skipped} messages"
                        );
                    });
                    continue;
                }
            };

            match poller_event {
                PollerEvent::Stdin(StdinEvent::Input(event)) => {
                    return Some(event);
                }
                PollerEvent::Stdin(StdinEvent::Eof | StdinEvent::Error) => {
                    return None;
                }
                PollerEvent::Signal(SignalEvent::Resize(maybe_size)) => {
                    // Use size from event, or retry get_size() if poller couldn't get it.
                    let size = maybe_size.or_else(|| get_size().ok());
                    if let Some(size) = size {
                        return Some(InputEvent::Resize(size));
                    }
                    // Both failed - continue waiting for next event.
                }
            }
        }
    }
}

impl Debug for DirectToAnsiInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectToAnsiInputDevice")
            .field("global_resource", &"<INPUT_RESOURCE>")
            .finish()
    }
}

impl Default for DirectToAnsiInputDevice {
    fn default() -> Self { Self::new() }
}

impl Drop for DirectToAnsiInputDevice {
    /// Clears gate, then Rust drops [`Self::resource_handle`], which triggers
    /// [`PollerSubscriptionHandle::drop()`]. See [Drop behavior] for full mechanism.
    ///
    /// [Drop behavior]: DirectToAnsiInputDevice#drop-behavior
    /// [`PollerSubscriptionHandle::drop()`]: PollerSubscriptionHandle#drop-behavior
    fn drop(&mut self) { super::at_most_one_instance_assert::release(); }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PollerSubscriptionHandle
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Receiver wrapper that wakes the [`mio_poller`] thread on drop.
///
/// # Drop behavior
///
/// When this handle is dropped:
/// 1. [`maybe_poller_rx`] is dropped first, which causes Tokio's broadcast channel to
///    atomically decrement the [`Sender`]'s internal [`receiver_count()`].
/// 2. Then [`mio::Waker::wake()`] interrupts the poll loop.
/// 3. The [`mio_poller`] thread wakes and [`handle_receiver_drop_waker()`] checks
///    [`receiver_count()`] to decide if the thread should exit (when count reaches `0`).
///
/// # Race Condition and Correctness
///
/// There is a race window between when the receiver is dropped and when
/// [`handle_receiver_drop_waker()`] checks [`receiver_count()`]. This is the **fast-path
/// thread reuse** scenario — if a new subscriber appears during the window, the thread
/// correctly continues serving it instead of exiting.
///
/// See [`PollerThreadLifecycleState`] for comprehensive documentation:
/// - [The Inherent Race Condition] — timeline diagram
/// - [What Happens If We Exit Blindly] — zombie device scenario
/// - [Why Thread Reuse Is Safe] — resource safety table
///
/// [The Inherent Race Condition]: super::mio_poller::PollerThreadLifecycleState#the-inherent-race-condition
/// [What Happens If We Exit Blindly]: super::mio_poller::PollerThreadLifecycleState#what-happens-if-we-exit-blindly
/// [Why Thread Reuse Is Safe]: super::mio_poller::PollerThreadLifecycleState#why-thread-reuse-is-safe
/// [`PollerThreadLifecycleState`]: super::mio_poller::PollerThreadLifecycleState
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`handle_receiver_drop_waker()`]: super::mio_poller::handler_receiver_drop::handle_receiver_drop_waker
/// [`maybe_poller_rx`]: Self::maybe_poller_rx
/// [`mio_poller`]: super::mio_poller
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
#[allow(missing_debug_implementations)]
pub struct PollerSubscriptionHandle {
    /// The actual broadcast receiver for poller events.
    pub maybe_poller_rx: Option<PollerEventReceiver>,

    /// Waker to signal the [`mio_poller`] thread.
    ///
    /// [`mio_poller`]: super::mio_poller
    pub mio_poller_thread_waker: Arc<mio::Waker>,
}

impl Drop for PollerSubscriptionHandle {
    /// Drops receiver then wakes thread. See [Drop behavior] for the full mechanism.
    /// Also see [`DirectToAnsiInputDevice`'s drop behavior] for when this is triggered.
    ///
    /// [Drop behavior]: PollerSubscriptionHandle#drop-behavior
    /// [`DirectToAnsiInputDevice`'s drop behavior]: DirectToAnsiInputDevice#drop-behavior
    fn drop(&mut self) {
        // Drop receiver first so Sender::receiver_count() decrements.
        drop(self.maybe_poller_rx.take());

        // Wake the thread so it can check if it should exit.
        let wake_result = self.mio_poller_thread_waker.wake();

        // Log failure (non-fatal: thread may have already exited).
        if let Err(err) = wake_result {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "PollerSubscriptionHandle::drop: wake failed",
                    error = ?err
                );
            });
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Comprehensive testing is performed in PTY integration tests:
/// - [`test_pty_input_device`]
/// - [`test_pty_mouse_events`]
/// - [`test_pty_keyboard_modifiers`]
/// - [`test_pty_utf8_text`]
/// - [`test_pty_terminal_events`]
///
/// [`test_pty_input_device`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_input_device_test::test_pty_input_device
/// [`test_pty_keyboard_modifiers`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_keyboard_modifiers_test::test_pty_keyboard_modifiers
/// [`test_pty_mouse_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mouse_events_test::test_pty_mouse_events
/// [`test_pty_terminal_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_terminal_events_test::test_pty_terminal_events
/// [`test_pty_utf8_text`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_utf8_text_test::test_pty_utf8_text
#[cfg(test)]
mod tests {
    use super::{super::{paste_state_machine::PasteCollectionState,
                        protocol_conversion::convert_input_event},
                *};
    use crate::{ByteOffset,
                core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                           VT100KeyCodeIR,
                                                           VT100KeyModifiersIR,
                                                           VT100PasteModeIR,
                                                           parse_keyboard_sequence,
                                                           parse_utf8_text}};

    #[tokio::test]
    async fn test_event_parsing() {
        // Test event parsing from buffer - verify parsers handle different sequence
        // types. These tests use the parser functions directly since they don't
        // need the device.

        // Test 1: Parse UTF-8 text (simplest case).
        let buffer: &[u8] = b"A";
        if let Some((vt100_event, bytes_consumed)) = parse_utf8_text(buffer) {
            assert_eq!(bytes_consumed, ByteOffset(1));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert UTF-8 text event");
            }
        } else {
            panic!("Failed to parse UTF-8 text 'A'");
        }

        // Test 2: ESC key (single byte).
        let esc_buffer: [u8; 1] = [0x1B];
        assert_eq!(esc_buffer.len(), 1);
        assert_eq!(esc_buffer[0], 0x1B);

        // Test 3: CSI sequence for keyboard (Up Arrow: ESC [ A).
        let csi_buffer: [u8; 3] = [0x1B, 0x5B, 0x41];
        if let Some((vt100_event, bytes_consumed)) = parse_keyboard_sequence(&csi_buffer)
        {
            assert_eq!(bytes_consumed, ByteOffset(3));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert keyboard event");
            }
        }
    }

    #[tokio::test]
    async fn test_paste_state_machine_basic() {
        // Test: Basic paste collection - Start marker, text, End marker.
        // Use a local paste_state to test the state machine logic directly.
        let mut paste_state = PasteCollectionState::Inactive;

        // Verify initial state is Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Simulate receiving Paste(Start) event.
        let start_event = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        match (&mut paste_state, &start_event) {
            (
                state @ PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::Start),
            ) => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!("State machine should handle Paste(Start)"),
        }

        // Verify we're now collecting.
        assert!(matches!(paste_state, PasteCollectionState::Accumulating(_)));

        // Simulate receiving keyboard events (the pasted text).
        for ch in &['H', 'e', 'l', 'l', 'o'] {
            let keyboard_event = VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(*ch),
                modifiers: VT100KeyModifiersIR::default(),
            };
            match (&mut paste_state, &keyboard_event) {
                (
                    PasteCollectionState::Accumulating(buffer),
                    VT100InputEventIR::Keyboard {
                        code: VT100KeyCodeIR::Char(ch),
                        ..
                    },
                ) => {
                    buffer.push(*ch);
                }
                _ => panic!("State machine should accumulate text while collecting"),
            }
        }

        // Simulate receiving Paste(End) event.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let collected_text = match (&mut paste_state, &end_event) {
            (
                state @ PasteCollectionState::Accumulating(_),
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => {
                if let PasteCollectionState::Accumulating(text) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    text
                } else {
                    panic!("Should have collected text")
                }
            }
            _ => panic!("State machine should handle Paste(End)"),
        };

        // Verify we collected the correct text.
        assert_eq!(collected_text, "Hello");

        // Verify we're back to Inactive state.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_multiline() {
        // Test: Paste with newlines.
        let mut paste_state = PasteCollectionState::Inactive;

        // Start collection.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            PasteCollectionState::Accumulating(_) => panic!(),
        }

        // Accumulate "Line1\nLine2".
        for ch in "Line1\nLine2".chars() {
            match &mut paste_state {
                PasteCollectionState::Accumulating(buffer) => {
                    buffer.push(ch);
                }
                PasteCollectionState::Inactive => panic!(),
            }
        }

        // End collection.
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "Line1\nLine2");
    }

    #[tokio::test]
    async fn test_paste_state_machine_orphaned_end() {
        // Test: Orphaned End marker (without Start) should be handled gracefully.
        let mut paste_state = PasteCollectionState::Inactive;

        // Should be Inactive initially.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Receive End marker without Start - should emit empty paste.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let result = match (&mut paste_state, &end_event) {
            (
                PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => Some(InputEvent::BracketedPaste(String::new())),
            _ => None,
        };

        assert!(matches!(result, Some(InputEvent::BracketedPaste(s)) if s.is_empty()));

        // Should still be Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_empty_paste() {
        // Test: Empty paste (Start immediately followed by End).
        let mut paste_state = PasteCollectionState::Inactive;

        // Start.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!(),
        }

        // End (without any characters in between).
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "");
    }
}

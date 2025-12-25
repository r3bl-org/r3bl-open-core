// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH kqueue epoll wakeup eventfd bcast reinit

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
//!    heuristic (see [ESC Detection Limitations] in [`MioPollerThread`]).
//!
//! ## The Solution
//!
//! A **process bound global singleton** with a dedicated reader thread that is the
//! **designated reader** of [`stdin`]. The thread uses [`mio::Poll`] to wait on both
//! [`stdin`] data and [`SIGWINCH`] signals.
//!
//! <div class="warning">
//!
//! **No exclusive access**: Any thread can call [`std::io::stdin()`] and read from it—
//! there is no OS or Rust mechanism to prevent this. If another thread reads from
//! [`stdin`], bytes will be stolen, causing interleaved reads that corrupt the input
//! stream and break the VT100 parser. See [No exclusive access] in [`MioPollerThread`].
//!
//! </div>
//!
//! Although sync and blocking, [`mio`] is efficient. It uses OS primitives ([`epoll`] on
//! Linux, [`kqueue`] on BSD/macOS) that put the thread to sleep until data arrives,
//! consuming no CPU while waiting. See [How It Works] in [`MioPollerThread`] for details.
//!
//! ```text
//!     Process-bound Global Singleton                       Async Consumers
//! ┌─────────────────────────────────────┐           ┌─────────────────────────────┐
//! │ Sync Blocking (std::thread + mio)   │           │ Primary: TUI input handler  │
//! │                                     │           │ Optional: Debug logger      │
//! │ Designated reader of:               │           │ Optional: Event recorder    │
//! │   • stdin (not exclusive access!)   │           │                             │
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
//! heuristic (see [ESC Detection Limitations] in [`MioPollerThread`]).
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │ INPUT_RESOURCE (static LazyLock<Mutex<...>>)                            │
//! │ internal:                                                               │
//! │  • mio-poller thread: holds tx, reads stdin, runs vt100 parser          │
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
//! See [`MioPollerThread`] for details on how the mio poller thread works, including
//! file descriptor handling, parsing, thread lifecycle, and ESC detection limitations.
//!
//! # Device Lifecycle
//!
//! A single process can create and drop [`DirectToAnsiInputDevice`] instances repeatedly.
//! The global [`INPUT_RESOURCE`] **static** persists, but the **thread** spawns and exits
//! with each app lifecycle:
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────────────┐
//! │ PROCESS LIFETIME                                                              │
//! │                                                                               │
//! │ INPUT_RESOURCE: LazyLock<Mutex<Option<PollerThreadLifecycleState>>>           │
//! │ (static persists, but contents are replaced on each thread spawn)             │
//! │                                                                               │
//! │ ═════════════════════════════════════════════════════════════════════════════ │
//! │                                                                               │
//! │ ┌───────────────────────────────────────────────────────────────────────────┐ │
//! │ │ TUI app A lifecycle                                                       │ │
//! │ │                                                                           │ │
//! │ │  1. DirectToAnsiInputDevice::new()                                        │ │
//! │ │  2. try_read_event() → allocate()                                         │ │
//! │ │  3. INPUT_RESOURCE is None → initialize_input_resource()                  │ │
//! │ │       • Creates PollerThreadLifecycleState { tx, liveness: Running }      │ │
//! │ │       • Spawns mio-poller thread #1                                       │ │
//! │ │       • thread #1 owns MioPollerThread struct                             │ │
//! │ │  4. TUI app A runs, receiving events from rx                              │ │
//! │ │  5. TUI app A exits → device dropped → receiver dropped                   │ │
//! │ │  6. Thread #1 detects 0 receivers → exits gracefully                      │ │
//! │ │  7. MioPollerThread::drop() → liveness = Terminated                       │ │
//! │ │                                                                           │ │
//! │ └───────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                               │
//! │ ┌───────────────────────────────────────────────────────────────────────────┐ │
//! │ │ TUI app B lifecycle                                                       │ │
//! │ │                                                                           │ │
//! │ │  1. DirectToAnsiInputDevice::new()                                        │ │
//! │ │  2. try_read_event() → allocate()                                         │ │
//! │ │  3. INPUT_RESOURCE has state, but liveness == Terminated                  │ │
//! │ │       → needs_init = true → initialize_input_resource()                   │ │
//! │ │       • Creates NEW PollerThreadLifecycleState { tx, liveness: Running }  │ │
//! │ │       • Spawns mio-poller thread #2 (NOT the same as #1!)                 │ │
//! │ │       • thread #2 owns its own MioPollerThread struct                     │ │
//! │ │  4. TUI app B runs, receiving events from rx                              │ │
//! │ │  5. TUI app B exits → device dropped → receiver dropped                   │ │
//! │ │  6. Thread #2 detects 0 receivers → exits gracefully                      │ │
//! │ │  7. MioPollerThread::drop() → liveness = Terminated                       │ │
//! │ │                                                                           │ │
//! │ └───────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                               │
//! │ ... pattern repeats for App C, D, etc. ...                                    │
//! │                                                                               │
//! └───────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! **Key insight**: The [`mio_poller`] thread is NOT persistent across the lifetime of
//! the process. Each app lifecycle spawns a new thread. The [`metadata`] field
//! enables this by allowing [`allocate()`] to detect when a thread
//! has exited and spawn a new one.
//!
//! ## Why Keystrokes Aren't Lost During Transitions
//!
//! Given the [Device Lifecycle] above—where threads exit and restart between apps—a
//! natural question arises: **why don't keystrokes get lost during the transition?**
//!
//! The historical problem (see [The Problems]) was that the old "Tokio-heavy" approach
//! created a new [`tokio::io::stdin()`] handle per app. When App A exited and App B
//! started, keystrokes typed during the transition vanished because
//! [`tokio::io::stdin()`] uses **application-level buffering**—when that handle is
//! dropped, its internal buffer is lost forever.
//!
//! The current design provides **three layers of protection**:
//!
//! | Layer                       | Protection Mechanism                                                                                           |
//! | :-------------------------- | :------------------------------------------------------------------------------------------------------------- |
//! | **Kernel buffer persists**  | Even after thread restart, unread bytes remain in the kernel's [`fd`] `0` buffer                               |
//! | **No app-level buffering**  | Direct [`std::io::Stdin`] reads with immediate parsing—no internal buffer to lose                              |
//! | **Fast-path reuse**         | If new app subscribes before thread exits, existing thread continues; see [`pty_mio_poller_thread_reuse_test`] |
//!
//! ### Data Flow During App Switching
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │ App A exits, drops receiver                                              │
//! │   • InputDeviceResourceHandle::drop() calls waker.wake()                 │
//! │   • Thread may continue running (fast) OR exit (slow)                    │
//! └──────────────────────────────┬───────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼───────────────────────────────────────────┐
//! │ User types keystrokes during transition                                  │
//! │   • Bytes arrive in kernel stdin buffer (fd 0)                           │
//! │   • If thread still running: reads immediately, sends to channel         │
//! │   • If thread exited: kernel buffer holds bytes until new thread reads   │
//! └──────────────────────────────┬───────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼───────────────────────────────────────────┐
//! │ App B starts, calls DirectToAnsiInputDevice::new()                       │
//! │   • allocate() checks liveness flag                                      │
//! │   • If Running: reuses existing thread (no gap in reading)               │
//! │   • If Terminated: spawns new thread → reads kernel buffer → no data loss│
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The key insight: the **kernel's [`stdin`] buffer for [`fd`] `0`
//! persists** regardless of which thread is reading. Unlike [`tokio::io::stdin()`]'s
//! application-level buffer, the kernel buffer survives handle creation/destruction. When
//! a new thread calls [`std::io::stdin()`], it gets a handle to the **same kernel
//! buffer** containing any unread bytes.
//!
//! ## Call Chain to [`allocate()`]
//!
//! ```text
//! DirectToAnsiInputDevice::new()                (input_device.rs)
//!     │
//!     └─► allocate()                            (global_input_resource.rs)
//!             │
//!             ├─► INPUT_RESOURCE.lock()
//!             │
//!             ├─► needs_init = None || liveness == Terminated
//!             │       │
//!             │       └─► if needs_init: initialize_input_resource()
//!             │               │
//!             │               ├─► Create PollerThreadLifecycleState
//!             │               ├─► MioPollerThread::spawn(state.clone())
//!             │               └─► guard.replace(state)
//!             │
//!             └─► return state.tx_input_event.subscribe() ← new broadcast receiver
//!
//! DirectToAnsiInputDevice::try_read_event()     (input_device.rs)
//!     │
//!     └─► stdin_rx.recv().await
//! ```
//!
//! **Key points:**
//! - [`DirectToAnsiInputDevice`] is a thin wrapper holding [`InputDeviceResourceHandle`]
//! - Global state ([`INPUT_RESOURCE`]) persists - channel and thread survive device drops
//! - Eager subscription - each device subscribes at construction time in [`new()`]
//! - Thread liveness check - if thread died, next subscribe reinitializes everything
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
//!    limitations (see [ESC Detection Limitations] in [`MioPollerThread`]).
//! 4. **Process-lifetime cleanup**: They rely on OS cleanup at process exit rather than
//!    explicit thread termination, and so do we.
//!
//! [Data Flow Diagram]: super::input_device::DirectToAnsiInputDevice#data-flow-diagram
//! [Device Lifecycle]: self#device-lifecycle
//! [ESC Detection Limitations]: super::mio_poller::MioPollerThread#esc-detection-limitations
//! [How It Works]: super::mio_poller::MioPollerThread#how-it-works
//! [No exclusive access]: super::mio_poller#no-exclusive-access
//! [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
//! [The Problems]: self#the-problems
//! [Tokio's stdin]: tokio::io::stdin
//! [`DirectToAnsiInputDevice::try_read_event()`]: super::input_device::DirectToAnsiInputDevice::try_read_event
//! [`DirectToAnsiInputDevice`]: super::input_device::DirectToAnsiInputDevice
//! [`Eof`]: super::channel_types::StdinReaderMessage::Eof
//! [`Error`]: super::channel_types::StdinReaderMessage::Error
//! [`Event(InputEvent)`]: super::channel_types::StdinReaderMessage::Event
//! [`EventStream`]: crossterm::event::EventStream
//! [`INTERNAL_EVENT_READER`]: https://github.com/crossterm-rs/crossterm/blob/0.29/src/event.rs#L149
//! [`LazyLock`]: std::sync::LazyLock
//! [`MioPollerThread`]: super::mio_poller::MioPollerThread
//! [`Resize`]: super::channel_types::StdinReaderMessage::Resize
//! [`SIGWINCH`]: signal_hook::consts::SIGWINCH
//! [`broadcast`]: tokio::sync::broadcast
//! [`crossterm`]: crossterm
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`metadata`]: PollerThreadLifecycleState::metadata
//! [`mio.rs`]: https://github.com/crossterm-rs/crossterm/blob/0.29/src/event/source/unix/mio.rs
//! [`mio::Poll`]: mio::Poll
//! [`mio_poller`]: super::mio_poller
//! [`mio`]: mio
//! [`new()`]: super::input_device::DirectToAnsiInputDevice::new
//! [`pty_mio_poller_thread_reuse_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_thread_reuse_test
//! [`readline_async`]: mod@crate::readline_async
//! [`signal-hook-mio`]: signal_hook_mio
//! [`signal-hook`]: signal_hook
//! [`std::io::Stdin`]: std::io::Stdin
//! [`std::io::stdin()`]: std::io::stdin
//! [`std::io::stdin()`]: std::io::stdin
//! [`std::process::exit()`]: std::process::exit
//! [`stdin`]: std::io::stdin
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::select!`]: tokio::select
//! [`tokio::sync::broadcast`]: tokio::sync::broadcast
//! [`try_read_event()`]: super::input_device::DirectToAnsiInputDevice::try_read_event
//! [`allocate()`]: guarded_ops::allocate

// Re-export ThreadLiveness for callers who use guarded_ops::is_thread_running().
pub use super::mio_poller::ThreadLiveness;
use super::mio_poller::{MioPollerThread, PollerThreadLifecycleState, SourceKindReady};
use crate::InputDeviceResourceHandle;
use mio::{Poll, Waker};
use std::sync::{Arc, LazyLock, Mutex};

/// Global singleton holding the [`PollerThreadLifecycleState`] that is initialized on
/// first access (see [`allocate()`]).
///
/// - Independent async consumers should use [`allocate()`] to get input events & signals.
/// - See the [module-level documentation] for details on why global state is necessary.
/// - See [`MioPollerThread`] docs for details on how the dedicated thread works.
///
/// [`allocate()`]: guarded_ops::allocate
/// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
/// [`stdin`]: std::io::stdin
/// [module-level documentation]: self
pub static INPUT_RESOURCE: LazyLock<Mutex<Option<PollerThreadLifecycleState>>> =
    LazyLock::new(|| Mutex::new(None));

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
    /// A new [`InputDeviceResourceHandle`] that independently receives all input events
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
    /// [`MioPollerThread::spawn()`] which uses [`mio::Poll`] to wait on both
    /// [`stdin`] data and [`SIGWINCH`] signals. See the [Thread Lifecycle] section in
    /// [`MioPollerThread`] for details on thread lifetime and exit conditions.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// 1. Thread spawning fails; see [`MioPollerThread::spawn()`] for details.
    /// 2. The [`INPUT_RESOURCE`] mutex is poisoned.
    /// 3. The [`INPUT_RESOURCE`] is `None` after initialization (invariant violation).
    ///
    /// [Thread Lifecycle]: MioPollerThread#thread-lifecycle
    /// [`INPUT_RESOURCE`]: super::INPUT_RESOURCE
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`broadcast::Sender::subscribe()`]: tokio::sync::broadcast::Sender::subscribe
    /// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
    /// [`mio::Poll`]: mio::Poll
    /// [`stdin`]: std::io::stdin
    pub fn allocate() -> InputDeviceResourceHandle {
        let mut guard = INPUT_RESOURCE.lock().expect(
            "INPUT_RESOURCE mutex poisoned: another thread panicked while holding this lock. \
             Terminal input is unavailable. This is unrecoverable.",
        );

        // Spawn new thread if never initialized or thread has terminated.
        if guard
            .as_ref()
            .is_none_or(|state| state.is_running() == ThreadLiveness::Terminated)
        {
            // Create Poll first so we can get the registry for the Waker.
            let poll = Poll::new().expect(
                "Failed to create mio::Poll: OS denied epoll/kqueue creation. \
                 Check ulimit -n (max open files) or /proc/sys/fs/epoll/max_user_watches.",
            );

            // Create Waker and wrap in Arc for sharing.
            let waker = Waker::new(
                poll.registry(),
                SourceKindReady::ReceiverDropWaker.to_token(),
            )
            .expect(
                "Failed to create mio::Waker: eventfd/pipe creation failed. \
                 Check ulimit -n (max open files).",
            );

            let state = PollerThreadLifecycleState::new(Arc::new(waker));

            // Spawn the thread with a handle to the shared state.
            MioPollerThread::spawn(poll, state.clone_handle());

            // Save the state to the global singleton.
            guard.replace(state);
        }

        // Guard is guaranteed to be Some at this point.
        debug_assert!(guard.is_some());
        let state = guard.as_ref().unwrap();

        InputDeviceResourceHandle {
            maybe_stdin_rx: Some(state.tx_stdin_reader_msg.subscribe()),
            mio_poller_thread_waker: Arc::clone(&state.waker_signal_shutdown),
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
    /// See [Device Lifecycle] for details on how threads spawn and exit.
    ///
    /// [Device Lifecycle]: self#device-lifecycle
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub fn is_thread_running() -> ThreadLiveness {
        INPUT_RESOURCE
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|state| state.is_running()))
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
    /// receivers dropped). See [Device Lifecycle] for details.
    ///
    /// [Device Lifecycle]: self#device-lifecycle
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    pub fn get_receiver_count() -> usize {
        INPUT_RESOURCE
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .as_ref()
                    .map(|state| state.tx_stdin_reader_msg.receiver_count())
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
    /// See [Device Lifecycle] for details on thread spawn/exit/relaunch.
    ///
    /// [Device Lifecycle]: self#device-lifecycle
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    pub fn get_thread_generation() -> u16 {
        INPUT_RESOURCE
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|state| state.generation()))
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
    /// [`DirectToAnsiInputDevice`]: super::super::input_device::DirectToAnsiInputDevice
    /// [`mio_poller`]: super::super::mio_poller
    pub fn subscribe_to_existing() -> crate::InputDeviceResourceHandle {
        let guard = INPUT_RESOURCE.lock().expect(
            "INPUT_RESOURCE mutex poisoned: another thread panicked while holding this lock.",
        );

        let state = guard.as_ref().expect(
            "subscribe_to_existing() called before DirectToAnsiInputDevice::new(). \
             Create a device first, then call device.subscribe().",
        );

        crate::InputDeviceResourceHandle {
            maybe_stdin_rx: Some(state.tx_stdin_reader_msg.subscribe()),
            mio_poller_thread_waker: Arc::clone(&state.waker_signal_shutdown),
        }
    }
}

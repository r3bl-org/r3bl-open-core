// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread lifecycle state container. See [`PollerThreadLifecycleState`] for
//! documentation.

use crate::terminal_lib_backends::direct_to_ansi::input::channel_types::PollerEventSender;
use std::sync::{Arc,
                atomic::{AtomicBool, AtomicU8, Ordering}};

/// Counter for thread generations. Incremented each time a new thread is spawned.
///
/// The actual value has no semantic meaning—it's just a counter. Tests compare
/// generations to detect whether the underlying thread changed (same generation =
/// thread reused, different generation = new thread spawned).
///
/// Wraps naturally from `255` → `0`.
static THREAD_GENERATION: AtomicU8 = AtomicU8::new(0);

/// Capacity of the broadcast channel for input events.
///
/// When the buffer is full, the oldest message is dropped to make room for new ones.
/// Slow consumers will receive [`Lagged`] on their next [`recv()`] call, indicating how
/// many messages they missed.
///
/// `4_096` is generous for terminal input (you'd never have that many pending
/// keypresses), but it's cheap (each [`PollerEvent`] is small) and provides
/// headroom for debug/logging consumers that might occasionally lag.
///
/// [`Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
/// [`PollerEvent`]: super::super::channel_types::PollerEvent
/// [`recv()`]: tokio::sync::broadcast::Receiver::recv
const CHANNEL_CAPACITY: usize = 4_096;

/// Thread lifecycle state and the inherent race condition due to blocking [`poll()`] and
/// syscall delays.
///
/// This struct bundles resources needed to manage the [`mio_poller`] thread's lifecycle
/// and documents the **inherent race condition** we handle correctly.
///
/// # Thread Lifecycle Overview
///
/// The [`mio_poller`] thread can be **relaunched** if it exits. Two mechanisms work
/// together:
///
/// 1. **Liveness flag** ([`metadata`]): Set to `false` via [`Drop`] when thread exits
/// 2. **[`mio::Waker`]**: Immediately wakes thread when receiver drops
///
/// Lifecycle sequence:
/// 1. On spawn: `liveness = Running`
/// 2. On receiver drop: [`PollerSubscriptionHandle::drop()`] calls [`waker.wake()`]
/// 3. [`handle_receiver_drop_waker()`] checks [`receiver_count()`] → if 0, exits
/// 4. [`MioPollerThread::drop()`] sets `liveness = Terminated`
/// 5. On next [`allocate()`]: detects terminated thread → reinitializes
///
/// # The Inherent Race Condition
///
/// **We don't cause the race condition — we handle it.** The race is inherent to the
/// architecture: the [`poll()`] syscall blocks, and there's unavoidable delay between
/// the waker signal and the exit check (kernel scheduling, context switch, syscall
/// return).
///
/// The waker signal means **"please check if you should exit"**, NOT "you must exit now".
///
/// ## Race Window Timeline
///
/// ```text
/// Timeline:
/// ─────────────────────────────────────────────────────────────────►
///      wake()          kernel         poll()         check
///      called         schedules       returns     receiver_count
///         │              │               │              │
///         └──────────────┴───────────────┴──────────────┘
///                     RACE WINDOW
///               (new subscriber can appear here)
/// ```
///
/// **During this window**, a new subscriber can appear:
/// 1. Old receiver drops → [`receiver_count`] = `0`, [`wake()`] called
/// 2. New device subscribes → [`receiver_count`] = 1
/// 3. Thread wakes, checks → sees [`receiver_count`] = 1
/// 4. Thread continues running (correct!)
///
/// This is the **fast-path thread reuse** scenario. The delay exists because:
/// 1. **Kernel scheduling** — Thread is blocked in a syscall; kernel must schedule it
/// 2. **Context switch** — CPU saves/restores thread state
/// 3. **Syscall return** — [`poll()`] must return through kernel → userspace
/// 4. **Code execution** — Thread iterates events, dispatches, *then* checks count
///
/// # What Happens If We Exit Blindly
///
/// If we ignored [`receiver_count()`] and always exited when woken, the new device
/// would become a **zombie**:
///
/// ```text
/// Thread A (old device)     Thread B (new device)          mio_poller Thread
/// ─────────────────────     ─────────────────────          ─────────────────
/// 1. drop(device_a)                                        │ blocked in poll()
///    → receiver_count = 0                                  │
///    → wake()                                              │
///                           2. new() called                │
///                              → liveness == Running ✓      │
///                              → subscribes to broadcast   │
///                              → receiver_count = 1        │
///                                                          ▼
///                                                          3. wakes up
///                                                          → blindly exits! 💀
///                                                          → tx dropped
///
///                           4. next().await
///                              → recv().await
///                              → RecvError::Closed! 💀
///                              → returns None forever
/// ```
///
/// Device B becomes a **zombie**: subscribed to a dead channel, forever returning
/// `None`. By checking the **current** [`receiver_count()`], we don't abandon devices
/// that subscribed during the race window.
///
/// # Why Thread Reuse Is Safe
///
/// When `receiver_count > 0` and we continue, the thread is reused. This is safe
/// because **nothing is leaked or corrupted**:
///
/// | Resource             | Status            | Why safe                                |
/// | :------------------- | :---------------- | :-------------------------------------- |
/// | Broadcast [`Sender`] | Same instance     | New receivers subscribe to same channel |
/// | [`stdin`] [`fd`]     | Same [`fd`] `0`   | OS-level, never changes                 |
/// | [`mio::Poll`]        | Same instance     | [`stdin`]/[`signals`] still registered  |
/// | Parser state         | Same instance     | Stateful parsing continues correctly    |
/// | [`Waker`]            | Same instance     | Shared via [`Arc`], still valid         |
///
/// The thread is essentially **stateless with respect to subscribers** — it just
/// reads [`stdin`], parses, and broadcasts. It doesn't care *who* is listening, only
/// *that someone* is listening.
///
/// # Related Tests
///
/// Four PTY-based integration tests validate the lifecycle behavior:
///
/// | Test                                     | Scenario                                           |
/// | :--------------------------------------- | :------------------------------------------------- |
/// | [`pty_mio_poller_thread_lifecycle_test`] | Full cycle: spawn → exit → respawn (with delay)    |
/// | [`pty_mio_poller_thread_reuse_test`]     | Race condition: fast subscriber reuses thread      |
/// | [`pty_mio_poller_singleton_test`]        | Singleton semantics: only one device at a time     |
/// | [`pty_mio_poller_subscribe_test`]        | Multiple subscriber broadcast semantics            |
///
/// [`Arc`]: std::sync::Arc
/// [`INPUT_RESOURCE`]: super::super::input_device::INPUT_RESOURCE
/// [`MioPollerThread::drop()`]: super::poller_thread::MioPollerThread
/// [`MioPollerThread::new()`]: super::poller_thread::MioPollerThread::new
/// [`PollerSubscriptionHandle::drop()`]: super::super::input_device::PollerSubscriptionHandle
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`Waker`]: mio::Waker
/// [`allocate()`]: super::super::input_device::guarded_ops::allocate
/// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
/// [`handle_receiver_drop_waker()`]: super::handler_receiver_drop::handle_receiver_drop_waker
/// [`metadata`]: PollerThreadLifecycleState::metadata
/// [`mio::Poll`]: mio::Poll
/// [`mio::Waker`]: mio::Waker
/// [`mio_poller`]: super
/// [`poll()`]: mio::Poll::poll
/// [`pty_mio_poller_singleton_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_singleton_test
/// [`pty_mio_poller_subscribe_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_subscribe_test
/// [`pty_mio_poller_thread_lifecycle_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_thread_lifecycle_test
/// [`pty_mio_poller_thread_reuse_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_thread_reuse_test
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver_count`]: tokio::sync::broadcast::Sender::receiver_count
/// [`signals`]: super::sources::SourceKindReady::Signals
/// [`stdin`]: super::sources::SourceKindReady::Stdin
/// [`wake()`]: mio::Waker::wake
/// [`waker.wake()`]: mio::Waker::wake
#[allow(missing_debug_implementations)]
pub struct PollerThreadLifecycleState {
    /// Broadcast sender for poller events.
    pub tx_poller_event: PollerEventSender,

    /// Thread metadata bundling identity (generation) and liveness.
    pub metadata: Arc<ThreadMetadata>,
}

impl Default for PollerThreadLifecycleState {
    fn default() -> Self { Self::new() }
}

impl PollerThreadLifecycleState {
    /// Creates a new lifecycle state with fresh [`ThreadMetadata`] and broadcast channel.
    #[must_use]
    pub fn new() -> Self {
        let channel = tokio::sync::broadcast::channel(CHANNEL_CAPACITY);
        Self {
            tx_poller_event: channel.0,
            metadata: Arc::new(ThreadMetadata::new()),
        }
    }

    /// Creates a handle to the same shared state (2 Arc bumps, no copies).
    ///
    /// Used by [`allocate()`] to pass a handle to [`MioPollerThread::new()`] while
    /// retaining ownership in [`INPUT_RESOURCE`].
    ///
    /// Use this instead of `.clone()` for semantic clarity, to make it explicit that this
    /// is a cheap reference-count increment, not a deep copy. All fields are
    /// [`Arc`]-wrapped, so cloning just bumps reference counts.
    ///
    /// [`INPUT_RESOURCE`]: super::super::input_device::INPUT_RESOURCE
    /// [`MioPollerThread::new()`]: super::poller_thread::MioPollerThread::new
    /// [`allocate()`]: super::super::input_device::guarded_ops::allocate
    #[must_use]
    pub fn clone_handle(&self) -> Self {
        Self {
            tx_poller_event: self.tx_poller_event.clone(),
            metadata: Arc::clone(&self.metadata),
        }
    }

    /// Checks if the thread should self-terminate (no receivers left).
    ///
    /// This is the **termination check** in the thread lifecycle protocol. Called by
    /// [`handle_receiver_drop_waker()`] when the thread wakes from a drop signal.
    ///
    /// Returns [`ShutdownDecision::ShutdownNow`] if [`receiver_count()`] is `0`, meaning
    /// no async consumers are listening. Returns [`ShutdownDecision::ContinueRunning`]
    /// otherwise. See [The Inherent Race Condition] for why we check the **current**
    /// count instead of trusting the wake signal.
    ///
    /// [The Inherent Race Condition]: PollerThreadLifecycleState#the-inherent-race-condition
    /// [`handle_receiver_drop_waker()`]: super::handler_receiver_drop::handle_receiver_drop_waker
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    #[must_use]
    pub fn should_self_terminate(&self) -> ShutdownDecision {
        if self.tx_poller_event.receiver_count() == 0 {
            ShutdownDecision::ShutdownNow
        } else {
            ShutdownDecision::ContinueRunning
        }
    }
}

/// Thread metadata bundling identity (immutable) and liveness (mutable).
///
/// This struct separates into two semantic parts:
///
/// - **Identity** ([`generation`]): Immutable after creation. Identifies which thread
///   instance this metadata belongs to.
/// - **Liveness** ([`is_running`]): Mutable. Tracks whether the thread is still running.
///
/// # Why [`AtomicBool`] instead of [`Mutex<bool>`]?
///
/// **Do not use [`Mutex<bool>`] here.** The [`is_running()`] method is called while
/// holding the [`INPUT_RESOURCE`] lock (in [`allocate()`] and [`is_thread_running()`]).
/// Using [`Mutex<bool>`] would create nested locking, risking **deadlock** if
/// [`mark_terminated()`] is called from the [`mio_poller`] thread while another thread
/// holds [`INPUT_RESOURCE`].
///
/// [`AtomicBool`] is **lock-free** — no deadlock possible.
///
/// [`Mutex<bool>`]: std::sync::Mutex
///
/// [`INPUT_RESOURCE`]: super::super::input_device::INPUT_RESOURCE
/// [`allocate()`]: super::super::input_device::guarded_ops::allocate
/// [`generation`]: ThreadMetadata::generation
/// [`is_running()`]: ThreadMetadata::is_running
/// [`is_running`]: Self::is_running
/// [`is_thread_running()`]: super::super::input_device::guarded_ops::is_thread_running
/// [`mark_terminated()`]: ThreadMetadata::mark_terminated
/// [`mio_poller`]: super
#[allow(missing_debug_implementations)]
pub struct ThreadMetadata {
    /// Whether the thread is currently running. Set to `false` by [`mark_terminated()`].
    ///
    /// [`mark_terminated()`]: ThreadMetadata::mark_terminated
    pub is_running: AtomicBool,

    /// Thread generation number. Immutable after creation.
    ///
    /// Incremented each time a new thread is spawned. Used to verify thread reuse vs
    /// relaunch in tests. If two observations have the same generation, the same thread
    /// is serving both. See [Related Tests] for details.
    ///
    /// [Related Tests]: PollerThreadLifecycleState#related-tests
    pub generation: u8,
}

impl ThreadMetadata {
    /// Creates new metadata in the [`Running`] state with a fresh generation.
    ///
    /// Increments [`THREAD_GENERATION`] atomically and captures the new value.
    /// Wraps naturally from `255` → `0`.
    ///
    /// [`Running`]: ThreadLiveness::Running
    #[must_use]
    fn new() -> Self {
        Self {
            is_running: AtomicBool::new(true),
            generation: THREAD_GENERATION
                .fetch_add(1, Ordering::SeqCst)
                .wrapping_add(1),
        }
    }

    /// Marks the thread as terminated.
    ///
    /// This is the **termination marker** in the thread lifecycle protocol. Called by
    /// [`MioPollerThread::drop()`] when the thread exits (either gracefully or via
    /// panic during stack unwinding).
    ///
    /// Sets [`is_running`] to `false`, allowing [`allocate()`] to detect the terminated
    /// thread and spawn a new one on the next subscription.
    ///
    /// [`MioPollerThread::drop()`]: super::poller_thread::MioPollerThread
    /// [`allocate()`]: super::super::input_device::guarded_ops::allocate
    /// [`is_running`]: Self::is_running
    pub fn mark_terminated(&self) { self.is_running.store(false, Ordering::SeqCst); }

    /// Checks if the thread is currently running.
    ///
    /// Used by [`allocate()`] to detect terminated threads that need respawning.
    ///
    /// [`allocate()`]: super::super::input_device::guarded_ops::allocate
    #[must_use]
    pub fn is_running(&self) -> ThreadLiveness {
        if self.is_running.load(Ordering::SeqCst) {
            ThreadLiveness::Running
        } else {
            ThreadLiveness::Terminated
        }
    }
}

/// Indicates whether the [`mio_poller`] thread is running or terminated.
///
/// Used by [`ThreadMetadata::is_running()`] to provide a self-documenting
/// return type instead of a bare `bool`.
///
/// [`mio_poller`]: super
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadLiveness {
    /// The [`mio_poller`] thread is running and accepting input.
    ///
    /// [`mio_poller`]: super
    Running,
    /// The [`mio_poller`] thread has exited or was never started.
    ///
    /// [`mio_poller`]: super
    Terminated,
}

/// Indicates whether the [`mio_poller`] thread should self-terminate or continue running.
///
/// Returned by [`PollerThreadLifecycleState::should_self_terminate()`] to provide a
/// self-documenting return type instead of a bare `bool`.
///
/// [`mio_poller`]: super
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownDecision {
    /// The thread should continue running because receivers are still listening.
    ContinueRunning,
    /// The thread should shut down now because no receivers are listening.
    ShutdownNow,
}

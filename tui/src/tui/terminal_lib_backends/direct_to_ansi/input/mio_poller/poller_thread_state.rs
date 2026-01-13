// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Poller thread state: shared state between [`SINGLETON`] and [`mio_poller`] thread.
//!
//! See [`PollerThreadState`] for documentation.
//!
//! [`SINGLETON`]: super::super::input_device_impl::global_input_resource::SINGLETON
//! [`mio_poller`]: super

use crate::terminal_lib_backends::direct_to_ansi::input::channel_types::PollerEventSender;
use mio::Waker;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

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

/// Shared state between the process-global [`SINGLETON`] and the [`mio_poller`] thread.
///
/// Centralizes thread lifecycle, event broadcasting, and race condition handling in one
/// place. Shared via [`Arc`] between the singleton and thread.
///
/// # Contents
///
/// - [`broadcast_tx`]: Channel sender for parsed input events
/// - [`thread_liveness`]: Running state and generation tracking
/// - [`waker`]: Shutdown signal (see [Waker Coupled To Poll])
///
/// # Thread Lifecycle Overview
///
/// The [`mio_poller`] thread can be **relaunched** if it exits. Two mechanisms work
/// together:
///
/// 1. **Liveness flag** ([`thread_liveness`]): Set to `false` via [`Drop`] when thread
///    exits
/// 2. **[`mio::Waker`]**: Immediately wakes thread when receiver drops
///
/// Lifecycle sequence:
/// 1. On spawn: `liveness = Running`
/// 2. On receiver drop: [`SubscriberGuard::drop()`] calls [`waker.wake()`]
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
/// ## Broadcast Channel Decoupling
///
/// The [`broadcast`] channel creates a clean separation between producer and consumers:
///
/// - **Producer-agnostic consumers**: Devices receive events via [`broadcast::Receiver`]
///   — they hold no reference to the thread, don't know its generation, and don't care if
///   it's "old" or "new"
/// - **Consumer-agnostic producer**: The thread sends via [`broadcast::Sender`] — it has
///   no references to specific receivers, no session state, no "who subscribed when"
///   tracking
/// - **Stateless events**: Each [`PollerEvent`] is self-contained (parsed input). No
///   accumulated state that could become "stale"
///
/// This is fundamentally different from architectures where servers track client
/// sessions or messages reference client IDs. Here, the broadcast channel is a pure
/// event stream — subscribers join anytime, leave anytime, no coordination needed.
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
/// [Waker Coupled To Poll]: PollerThreadState#waker-coupled-to-poll
/// [`Arc`]: std::sync::Arc
/// [`SubscriberGuard::drop()`]: super::super::input_device_impl::subscriber::SubscriberGuard#impl-Drop-for-SubscriberGuard
/// [`MioPollerThread::drop()`]: super::poller_thread::MioPollerThread
/// [`MioPollerThread::new()`]: super::poller_thread::MioPollerThread::new
/// [`PollerEvent`]: super::super::channel_types::PollerEvent
/// [`SINGLETON`]: super::super::input_device_impl::global_input_resource::SINGLETON
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`Waker`]: mio::Waker
/// [`allocate()`]: super::super::input_device_impl::global_input_resource::allocate
/// [`broadcast::Receiver`]: tokio::sync::broadcast::Receiver
/// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
/// [`broadcast_tx`]: Self::broadcast_tx
/// [`broadcast`]: tokio::sync::broadcast
/// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
/// [`handle_receiver_drop_waker()`]: super::handler_receiver_drop::handle_receiver_drop_waker
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
/// [`thread_liveness`]: Self::thread_liveness
/// [`wake()`]: mio::Waker::wake
/// [`waker.wake()`]: mio::Waker::wake
/// [`waker`]: Self::waker
#[allow(missing_debug_implementations)]
pub struct PollerThreadState {
    /// Broadcasts parsed input events to async subscribers.
    pub broadcast_tx: PollerEventSender,

    /// Thread liveness and incarnation tracking.
    ///
    /// See [`ThreadLiveness`] for why this uses [`AtomicBool`] instead of
    /// [`Mutex<bool>`].
    ///
    /// [`Mutex<bool>`]: std::sync::Mutex
    ///
    /// [`AtomicBool`]: std::sync::atomic::AtomicBool
    pub thread_liveness: ThreadLiveness,

    /// Waker to signal thread shutdown.
    ///
    /// Called by [`SubscriberGuard::drop()`] to wake the thread so it can check
    /// [`receiver_count()`] and decide whether to exit.
    ///
    /// # Waker Coupled To Poll
    ///
    /// The [`Waker`] was created from the same [`Poll`] instance passed to
    /// [`MioPollerThread::new()`]. They share an OS-level bond:
    ///
    /// ```text
    /// Poll (epoll/kqueue) ──owns──► Registry ──creates──► Waker
    /// ```
    ///
    /// When [`waker.wake()`] is called, it triggers an event that [`poll()`] returns.
    /// **If [`Poll`] is dropped, this [`Waker`] becomes useless** — it would signal an
    /// event mechanism that no longer exists.
    ///
    /// This is why the slow path in [`allocate()`] replaces the entire
    /// [`PollerThreadState`] — the [`Poll`], [`Waker`], and thread must be created
    /// together. See [Poll → Registry → Waker Chain] for how they're created.
    ///
    /// # Why Waker Is Not Passed to the Thread
    ///
    /// The thread doesn't need a reference to [`Waker`] — it only needs to *respond* to
    /// wake events. When [`allocate()`] creates the [`Poll`] and [`Waker`], the waker is
    /// registered with [`Poll`]'s registry. This means:
    ///
    /// - When **any** [`SubscriberGuard`] calls [`waker.wake()`], the thread's [`poll()`]
    ///   returns with a [`ReceiverDropWaker`] token
    /// - The thread handles this via [`handle_receiver_drop_waker()`], checking if it
    ///   should exit
    ///
    /// The singleton keeps the [`Waker`] as a **distribution point** — the
    /// [`Arc<PollerThreadState>`] is cloned to each [`SubscriberGuard`] on subscription.
    /// The thread never touches it directly.
    ///
    /// [`Arc<PollerThreadState>`]: std::sync::Arc
    ///
    /// [Poll → Registry → Waker Chain]: super::super::input_device_impl::global_input_resource::SINGLETON#poll--registry--waker-chain
    /// [`SubscriberGuard::drop()`]: super::super::input_device_impl::subscriber::SubscriberGuard#impl-Drop-for-SubscriberGuard
    /// [`SubscriberGuard`]: super::super::input_device_impl::subscriber::SubscriberGuard
    /// [`MioPollerThread::new()`]: super::poller_thread::MioPollerThread::new
    /// [`Poll`]: mio::Poll
    /// [`PollerThreadState`]: PollerThreadState
    /// [`ReceiverDropWaker`]: super::SourceKindReady::ReceiverDropWaker
    /// [`Waker`]: mio::Waker
    /// [`allocate()`]: super::super::input_device_impl::global_input_resource::allocate
    /// [`handle_receiver_drop_waker()`]: super::handler_receiver_drop::handle_receiver_drop_waker
    /// [`poll()`]: mio::Poll::poll
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`waker.wake()`]: mio::Waker::wake
    pub waker: Waker,
}

impl PollerThreadState {
    /// Creates new thread state with fresh [`ThreadLiveness`] and broadcast channel.
    ///
    /// The `waker` must be created from the same [`Poll`] instance that will be passed
    /// to [`MioPollerThread::new()`]. See [Waker Coupled To Poll] for why.
    ///
    /// [Waker Coupled To Poll]: PollerThreadState#waker-coupled-to-poll
    /// [`MioPollerThread::new()`]: super::poller_thread::MioPollerThread::new
    /// [`Poll`]: mio::Poll
    #[must_use]
    pub fn new(waker: Waker) -> Self {
        let channel = tokio::sync::broadcast::channel(CHANNEL_CAPACITY);
        Self {
            broadcast_tx: channel.0,
            thread_liveness: ThreadLiveness::new(),
            waker,
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
    /// [The Inherent Race Condition]: PollerThreadState#the-inherent-race-condition
    /// [`handle_receiver_drop_waker()`]: super::handler_receiver_drop::handle_receiver_drop_waker
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    #[must_use]
    pub fn should_self_terminate(&self) -> ShutdownDecision {
        if self.broadcast_tx.receiver_count() == 0 {
            ShutdownDecision::ShutdownNow
        } else {
            ShutdownDecision::ContinueRunning
        }
    }
}

/// Thread liveness: running state and incarnation generation.
///
/// - [`is_running`]: Current liveness (mutable via [`mark_terminated()`])
/// - [`generation`]: Which incarnation of the thread (immutable)
///
/// # Why [`AtomicBool`] instead of [`Mutex<bool>`]?
///
/// - **Do not use [`Mutex<bool>`] here.** The [`is_running()`] method is called while
///   holding the [`SINGLETON`] lock (in [`allocate()`] and [`is_thread_running()`]).
///   Using [`Mutex<bool>`] would create nested locking, risking **deadlock** if
///   [`mark_terminated()`] is called from the [`mio_poller`] thread while another thread
///   holds [`SINGLETON`].
/// - [`AtomicBool`] is **lock-free** — no deadlock possible.
///
/// [`Mutex<bool>`]: std::sync::Mutex
///
/// [`SINGLETON`]: super::super::input_device_impl::global_input_resource::SINGLETON
/// [`allocate()`]: super::super::input_device_impl::global_input_resource::allocate
/// [`generation`]: Self::generation
/// [`is_running()`]: Self::is_running()
/// [`is_running`]: Self::is_running
/// [`is_thread_running()`]: super::super::input_device_impl::global_input_resource::is_thread_running
/// [`mark_terminated()`]: Self::mark_terminated
/// [`mio_poller`]: super
#[allow(missing_debug_implementations)]
pub struct ThreadLiveness {
    /// Whether the thread is currently running. Set to `false` by [`mark_terminated()`].
    ///
    /// [`mark_terminated()`]: Self::mark_terminated
    pub is_running: AtomicBool,

    /// Thread generation number. Immutable after creation.
    ///
    /// Incremented each time a new thread is spawned. Used to verify thread reuse vs
    /// relaunch in tests. See [Related Tests] for details.
    ///
    /// [Related Tests]: PollerThreadState#related-tests
    pub generation: u8,
}

impl ThreadLiveness {
    /// Creates new liveness in the [`Running`] state with a fresh generation.
    ///
    /// [`Running`]: LivenessState::Running
    #[must_use]
    fn new() -> Self {
        Self {
            is_running: AtomicBool::new(true),
            generation: THREAD_GENERATION
                .fetch_add(1, Ordering::SeqCst)
                .wrapping_add(1),
        }
    }

    /// Marks the thread as terminated. Called by [`MioPollerThread::drop()`].
    ///
    /// [`MioPollerThread::drop()`]: super::poller_thread::MioPollerThread
    pub fn mark_terminated(&self) { self.is_running.store(false, Ordering::SeqCst); }

    /// Checks if the thread is currently running.
    #[must_use]
    pub fn is_running(&self) -> LivenessState {
        if self.is_running.load(Ordering::SeqCst) {
            LivenessState::Running
        } else {
            LivenessState::Terminated
        }
    }
}

/// Indicates whether the [`mio_poller`] thread is running or terminated.
///
/// Used by [`ThreadLiveness::is_running()`] to provide a self-documenting
/// return type instead of a bare `bool`.
///
/// [`mio_poller`]: super
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
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
/// Returned by [`PollerThreadState::should_self_terminate()`] to provide a
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

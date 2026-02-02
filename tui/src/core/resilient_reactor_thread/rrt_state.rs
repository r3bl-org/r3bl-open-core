// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words epoll kqueue

//! Shared state container for the Resilient Reactor Thread pattern. See [`RRTState`].

use super::{RRTLiveness, RRTWaker, ShutdownDecision};
use tokio::sync::broadcast::Sender;

/// Capacity of the broadcast channel for events.
///
/// When the buffer is full, the oldest message is dropped to make room for new ones.
/// Slow consumers will receive [`Lagged`] on their next [`recv()`] call, indicating how
/// many messages they missed.
///
/// `4_096` is generous for typical event streams, but cheap (events are usually small)
/// and provides headroom for debug/logging consumers that might occasionally lag.
///
/// [`Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
/// [`recv()`]: tokio::sync::broadcast::Receiver::recv
pub const CHANNEL_CAPACITY: usize = 4_096;

/// A shared state container between the process-global singleton and the worker thread.
///
/// This struct centralizes thread lifecycle, event broadcasting, and wake signaling in
/// one place.
/// Shared via [`Arc`] between the singleton and thread.
///
/// # Contents
///
/// - [`broadcast_tx`]: Channel sender for events
/// - [`liveness`]: Running state and generation tracking
/// - [`waker`]: Shutdown signal (see [Waker Lifecycle])
///
/// # Thread Lifecycle Overview
///
/// The worker thread can be **relaunched** if it exits. Two mechanisms work together:
///
/// 1. **Liveness flag** ([`liveness`]): Set to `Terminated` via [`Drop`] when thread
///    exits
/// 2. **Waker**: Immediately wakes thread when receiver drops
///
/// Lifecycle sequence:
/// 1. On spawn: `liveness = Running`
/// 2. On receiver drop: [`SubscriberGuard::drop()`] calls [`waker.wake()`]
/// 3. Worker checks [`receiver_count()`] → if `0`, exits
/// 4. Worker's [`Drop`] sets `liveness = Terminated`
/// 5. On next [`subscribe()`]: detects terminated thread → reinitializes
///
/// # Waker Lifecycle
///
/// The waker is **coupled to the worker's resources**. For example, with [`mio`]:
///
/// ```text
/// mio::Poll (epoll/kqueue) ──owns──► Registry ──creates──► Waker
/// ```
///
/// When [`waker.wake()`] is called, it triggers an event that the worker's blocking call
/// returns. **If the worker's resources are dropped, the waker becomes useless** — it
/// would signal a mechanism that no longer exists.
///
/// This is why the slow path in [`subscribe()`] replaces the entire [`RRTState`] — the
/// worker resources, waker, and thread must be created together.
///
/// [`Arc`]: std::sync::Arc
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`broadcast_tx`]: Self::broadcast_tx
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`liveness`]: Self::liveness
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`subscribe()`]: super::RRTSafeGlobalState::subscribe
/// [`waker.wake()`]: RRTWaker::wake
/// [`waker`]: Self::waker
#[allow(missing_debug_implementations)]
pub struct RRTState<W, E>
where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    /// Broadcasts events to async subscribers.
    pub broadcast_tx: Sender<E>,

    /// Thread liveness and incarnation tracking.
    ///
    /// See [`RRTLiveness`] for why this uses [`AtomicBool`] instead of `Mutex<bool>`.
    ///
    /// [`AtomicBool`]: std::sync::atomic::AtomicBool
    /// [`RRTLiveness`]: super::RRTLiveness
    pub liveness: RRTLiveness,

    /// Waker to signal thread for shutdown check.
    ///
    /// Called by [`SubscriberGuard::drop()`] to wake the thread so it can check
    /// [`receiver_count()`] and decide whether to exit.
    ///
    /// [`SubscriberGuard::drop()`]: super::SubscriberGuard
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    pub waker: W,
}

impl<W, E> RRTState<W, E>
where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    /// Creates new thread state with fresh [`RRTLiveness`] and broadcast channel.
    ///
    /// The `waker` must be created from the same resources that the worker owns.
    /// See [Waker Lifecycle] for why they're coupled.
    ///
    /// [Waker Lifecycle]: RRTState#waker-lifecycle
    /// [`RRTLiveness`]: super::RRTLiveness
    #[must_use]
    pub fn new(waker: W) -> Self {
        let (broadcast_tx, _) = tokio::sync::broadcast::channel(CHANNEL_CAPACITY);
        Self {
            broadcast_tx,
            liveness: RRTLiveness::new(),
            waker,
        }
    }

    /// Checks if the thread should self-terminate (no receivers left).
    ///
    /// This is the **termination check** in the thread lifecycle protocol. Called by the
    /// worker when it receives a wake signal.
    ///
    /// Returns [`ShutdownDecision::ShutdownNow`] if [`receiver_count()`] is `0`, meaning
    /// no async consumers are listening. Returns [`ShutdownDecision::ContinueRunning`]
    /// otherwise.
    ///
    /// # The Inherent Race Condition
    ///
    /// We check the **current** count, not the count when `wake()` was called. This
    /// handles the race where a new subscriber appears between the wake signal and this
    /// check:
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
    /// The [kernel] schedules threads independently, so if a new subscriber appeared
    /// during the window, [`receiver_count()`] will be > `0`, and we correctly continue
    /// running instead of abandoning the new subscriber.
    ///
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [kernel]: https://en.wikipedia.org/wiki/Kernel_(operating_system)
    #[must_use]
    pub fn should_self_terminate(&self) -> ShutdownDecision {
        if self.broadcast_tx.receiver_count() == 0 {
            ShutdownDecision::ShutdownNow
        } else {
            ShutdownDecision::ContinueRunning
        }
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread liveness tracking for the Resilient Reactor Thread pattern. See
//! [`RRTLiveness`] and [`LivenessState`].

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Counter for thread generations. Incremented each time a new thread is spawned.
///
/// The actual value has no semantic meaning - it's just a counter. Tests compare
/// generations to detect whether the underlying thread changed (same generation = thread
/// reused, different generation = new thread spawned).
///
/// Wraps naturally from `255` → `0`.
static THREAD_GENERATION: AtomicU8 = AtomicU8::new(0);

/// A tracker for thread liveness state and incarnation generation.
///
/// - [`is_running`]: Current liveness (mutable via [`mark_terminated()`])
/// - [`generation`]: Which incarnation of the thread (immutable)
///
/// # Generation Tracking
///
/// Each time a new thread is spawned via [`RRT::subscribe()`], the
/// generation counter increments. This allows tests to verify thread reuse vs relaunch:
///
/// - **Same generation**: Thread was reused (new subscriber appeared before thread
///   exited)
/// - **Different generation**: Thread was relaunched (a new thread was spawned)
///
/// # Why [`AtomicBool`] Instead of [`Mutex<bool>`]?
///
/// The [`is_running()`] method is called while holding the global state lock (in
/// [`subscribe()`] and query functions). Using [`Mutex<bool>`] would create nested
/// locking, risking [deadlock] if [`mark_terminated()`] is called from the dedicated
/// thread while another thread holds the global lock.
///
/// [`AtomicBool`] is [lock-free] - no [deadlock] possible. All atomic operations use
/// [`SeqCst`] ordering for simplicity and correctness.
///
/// [`Mutex<bool>`]: std::sync::Mutex
///
/// [`AtomicBool`]: std::sync::atomic::AtomicBool
/// [`RRT::subscribe()`]: super::RRT::subscribe
/// [`SeqCst`]: std::sync::atomic::Ordering::SeqCst
/// [`generation`]: Self::generation
/// [`is_running()`]: Self::is_running
/// [`is_running`]: Self::is_running
/// [`mark_terminated()`]: Self::mark_terminated
/// [`subscribe()`]: super::RRT::subscribe
/// [deadlock]: https://en.wikipedia.org/wiki/Deadlock
/// [lock-free]: https://en.wikipedia.org/wiki/Non-blocking_algorithm
#[allow(missing_debug_implementations)]
pub struct RRTLiveness {
    /// Whether the thread is currently running. Set to `false` by [`mark_terminated()`].
    ///
    /// [`mark_terminated()`]: Self::mark_terminated
    pub is_running: AtomicBool,

    /// Thread generation number. Immutable after creation.
    ///
    /// Incremented each time a new thread is spawned. Used to verify thread reuse vs
    /// relaunch in tests.
    pub generation: u8,
}

impl RRTLiveness {
    /// Creates new liveness in the [`Running`] state with a fresh generation.
    ///
    /// The generation number is atomically incremented from the global counter.
    ///
    /// [`Running`]: LivenessState::Running
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_running: AtomicBool::new(true),
            generation: THREAD_GENERATION
                .fetch_add(1, Ordering::SeqCst)
                .wrapping_add(1),
        }
    }

    /// Marks the thread as terminated.
    ///
    /// Called by the dedicated thread's [`Drop`] implementation when it exits.
    /// After this call, [`is_running()`] will return [`LivenessState::Terminated`].
    ///
    /// [`is_running()`]: Self::is_running
    pub fn mark_terminated(&self) { self.is_running.store(false, Ordering::SeqCst); }

    /// Checks if the thread is currently running.
    ///
    /// Returns [`LivenessState::Running`] if the thread is active, or
    /// [`LivenessState::Terminated`] if it has exited.
    #[must_use]
    pub fn is_running(&self) -> LivenessState {
        if self.is_running.load(Ordering::SeqCst) {
            LivenessState::Running
        } else {
            LivenessState::Terminated
        }
    }
}

impl Default for RRTLiveness {
    fn default() -> Self { Self::new() }
}

/// An indication of whether the dedicated thread is running or terminated.
///
/// Used by [`RRTLiveness::is_running()`] to provide a self-documenting return type
/// instead of a bare `bool`.
///
/// # Why Not Just `bool`?
///
/// `bool` requires remembering what `true` means. With this enum:
/// - [`LivenessState::Running`] is unambiguous
/// - Pattern matching catches all cases
/// - Code reads like documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    /// The dedicated thread is running and processing events.
    Running,
    /// The dedicated thread has exited or was never started.
    Terminated,
}

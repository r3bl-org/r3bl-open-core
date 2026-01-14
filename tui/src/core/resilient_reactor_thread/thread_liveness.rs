// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread liveness tracking for the Resilient Reactor Thread pattern.
//!
//! This module provides:
//! - [`ThreadLiveness`]: Tracks whether a thread is running and its generation
//! - [`LivenessState`]: Self-documenting enum for running/terminated state
//! - [`ShutdownDecision`]: Self-documenting enum for continue/shutdown decision
//!
//! # Why [`AtomicBool`] Instead of [`Mutex<bool>`]?
//!
//! The [`ThreadLiveness::is_running()`] method is called while holding the global state
//! lock (in [`allocate()`] and query functions). Using [`Mutex<bool>`] would create
//! nested locking, risking [deadlock] if [`mark_terminated()`] is called from the worker
//! thread while another thread holds the global lock.
//!
//! [`AtomicBool`] is [lock-free] — no [deadlock] possible.
//!
//! [`Mutex<bool>`]: std::sync::Mutex
//!
//! [`AtomicBool`]: std::sync::atomic::AtomicBool
//! [`allocate()`]: super::ThreadSafeGlobalState::allocate
//! [`mark_terminated()`]: ThreadLiveness::mark_terminated
//! [deadlock]: https://en.wikipedia.org/wiki/Deadlock
//! [lock-free]: https://en.wikipedia.org/wiki/Non-blocking_algorithm

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Counter for thread generations. Incremented each time a new thread is spawned.
///
/// The actual value has no semantic meaning — it's just a counter. Tests compare
/// generations to detect whether the underlying thread changed (same generation = thread
/// reused, different generation = new thread spawned).
///
/// Wraps naturally from `255` → `0`.
static THREAD_GENERATION: AtomicU8 = AtomicU8::new(0);

/// Thread liveness: running state and incarnation generation.
///
/// - [`is_running`]: Current liveness (mutable via [`mark_terminated()`])
/// - [`generation`]: Which incarnation of the thread (immutable)
///
/// # Generation Tracking
///
/// Each time a new thread is spawned via [`ThreadSafeGlobalState::allocate()`], the
/// generation counter increments. This allows tests to verify thread reuse vs relaunch:
///
/// - **Same generation**: Thread was reused (new subscriber appeared before thread
///   exited)
/// - **Different generation**: Thread was relaunched (a new thread was spawned)
///
/// # Thread Safety
///
/// Uses [`AtomicBool`] for the running flag to avoid nested locking. All atomic
/// operations use [`SeqCst`] ordering for simplicity and correctness. See [module docs]
/// for why atomics are used instead of a mutex.
///
/// [`AtomicBool`]: std::sync::atomic::AtomicBool
/// [`SeqCst`]: std::sync::atomic::Ordering::SeqCst
/// [`ThreadSafeGlobalState::allocate()`]: super::ThreadSafeGlobalState::allocate
/// [`generation`]: Self::generation
/// [`is_running`]: Self::is_running
/// [`mark_terminated()`]: Self::mark_terminated
/// [module docs]: super
#[allow(missing_debug_implementations)]
pub struct ThreadLiveness {
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

impl ThreadLiveness {
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
    /// Called by the worker thread's [`Drop`] implementation when the thread exits.
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

impl Default for ThreadLiveness {
    fn default() -> Self { Self::new() }
}

/// Indicates whether the worker thread is running or terminated.
///
/// Used by [`ThreadLiveness::is_running()`] to provide a self-documenting return type
/// instead of a bare `bool`.
///
/// # Why Not Just `bool`?
///
/// `bool` requires remembering what `true` means. With this enum:
/// - `LivenessState::Running` is unambiguous
/// - Pattern matching catches all cases
/// - Code reads like documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    /// The worker thread is running and processing events.
    Running,
    /// The worker thread has exited or was never started.
    Terminated,
}

/// Indicates whether the worker thread should self-terminate or continue running.
///
/// Returned by [`ThreadState::should_self_terminate()`] to provide a self-documenting
/// return type instead of a bare `bool`.
///
/// # Decision Logic
///
/// The thread should:
/// - **Continue** if [`receiver_count()`] > 0 (someone is listening)
/// - **Shutdown** if [`receiver_count()`] == 0 (no one listening)
///
/// This check is performed **when the thread wakes**, not when the receiver drops. This
/// handles the race condition where a new subscriber appears between the wake signal and
/// the exit check.
///
/// [`ThreadState::should_self_terminate()`]: super::ThreadState::should_self_terminate
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownDecision {
    /// The thread should continue running because receivers are still listening.
    ContinueRunning,
    /// The thread should shut down now because no receivers are listening.
    ShutdownNow,
}

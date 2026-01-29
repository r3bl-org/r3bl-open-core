// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH epoll kqueue threadwaker

//! Core traits for the Resilient Reactor Thread (RRT) pattern.
//!
//! - [`RRTWaker`]: Interrupt a blocked thread
//! - [`RRTWorker`]: Work loop running on the thread
//! - [`RRTFactory`]: Creates coupled worker + waker
//!
//! See [module docs] for the full RRT pattern explanation.
//!
//! [module docs]: super

use crate::core::common::Continuation;
use miette::Report;
use tokio::sync::broadcast::Sender;

/// Waker abstraction for interrupting a blocking thread.
///
/// Called by [`SubscriberGuard::drop()`] to signal the worker thread to check if it
/// should exit.
///
/// # Bounds
///
/// Stored in [`Arc`] within [`ThreadState`], shared across [`SubscriberGuard`]s:
///
/// - **[`Send`] + [`Sync`]**: Required for [`Arc<T>`] to be [`Send`]
/// - **`'static`**: Required for thread spawning
///
/// [`Arc<T>`]: std::sync::Arc
///
/// # Concrete Implementation
///
/// See [`MioPollWaker`] for a concrete implementation using [`mio::Waker`].
///
/// # Why User-Provided?
///
/// Wake strategies are backend-specific. See [Why is `RRTWaker` User-Provided?]
///
/// [Why is `RRTWaker` User-Provided?]: super#why-is-threadwaker-user-provided
/// [`Arc`]: std::sync::Arc
/// [`MioPollWaker`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWaker
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`ThreadState`]: super::ThreadState
/// [`SubscriberGuard`]: super::SubscriberGuard
pub trait RRTWaker: Send + Sync + 'static {
    /// Wake the thread so it can check if it should exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the wake signal cannot be sent (typically non-fatal).
    fn wake(&self) -> std::io::Result<()>;
}

/// Worker that runs on a dedicated thread.
///
/// Implements the actual work loop logic. Called repeatedly by the framework until
/// [`Continuation::Stop`] is returned.
///
/// # Bounds
///
/// Moves to the spawned thread and is owned exclusively by it:
///
/// - **[`Send`]**: Required to move across thread boundary
/// - **`'static`**: Required for [`thread::spawn()`]
///
/// Note: No [`Sync`] needed — the worker is owned, not shared.
///
/// # Concrete Implementation
///
/// See [`MioPollWorker`] for a concrete implementation that monitors stdin and signals.
///
/// # Design Rationale
///
/// The `poll_once() → Continuation` design (vs `run()`) provides:
/// - **Framework control**: Inject logging, metrics between iterations
/// - **Single responsibility**: Worker handles events, framework handles lifecycle
/// - **Testability**: Unit test `poll_once()` in isolation
///
/// [`MioPollWorker`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker
/// [`thread::spawn()`]: std::thread::spawn
pub trait RRTWorker: Send + 'static {
    /// Event type this worker produces.
    ///
    /// Must be [`Clone`] + [`Send`] + `'static` for the broadcast channel.
    type Event: Clone + Send + 'static;

    /// Run one iteration of the work loop.
    ///
    /// Called in a loop by the framework. Return [`Continuation::Continue`] to keep
    /// running, or [`Continuation::Stop`] to exit the thread.
    ///
    /// Use [`tx.receiver_count()`] to check if anyone is still listening.
    ///
    /// [`tx.receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn poll_once(&mut self, tx: &Sender<Self::Event>) -> Continuation;
}

/// Factory that creates coupled worker and waker together.
///
/// Solves the **chicken-egg problem**: waker creation depends on resources the worker
/// owns. For example, [`mio::Waker`] needs [`mio::Poll`]'s registry, but the Poll must
/// move to the spawned thread.
///
/// # Concrete Implementation
///
/// See [`MioPollWorkerFactory`] for a concrete implementation.
///
/// [`MioPollWorkerFactory`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorkerFactory
pub trait RRTFactory {
    /// Event type broadcast to subscribers.
    type Event;

    /// Worker type that runs on the thread.
    type Worker: RRTWorker<Event = Self::Event>;

    /// Waker type for interrupting the worker.
    type Waker: RRTWaker;

    /// Create [`Worker`] and [`Waker`] together.
    ///
    /// - [`Worker`] → moves to spawned thread
    /// - [`Waker`] → stored in [`ThreadState`] for [`SubscriberGuard`]s to call
    ///   [`wake()`]
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created.
    ///
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`ThreadState`]: super::ThreadState
    /// [`wake()`]: RRTWaker::wake
    /// [`Worker`]: Self::Worker
    /// [`Waker`]: Self::Waker
    fn create() -> Result<(Self::Worker, Self::Waker), Report>;
}

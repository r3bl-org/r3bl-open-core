// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH epoll kqueue threadwaker rrtwaker

//! Core traits for the Resilient Reactor Thread (RRT) pattern.
//!
//! - [`RRTFactory`]: Creates coupled worker thread + waker
//! - [`RRTWorker`]: Work loop running on the thread
//! - [`RRTWaker`]: Interrupt a blocked thread
//!
//! See [module docs] for the full RRT pattern explanation.
//!
//! [module docs]: super

use crate::core::common::Continuation;
use miette::Report;
use tokio::sync::broadcast::Sender;

/// A trait for creating a coupled [`Worker`] + [`Waker`] pair atomically.
///
/// This trait solves the [coupled resource creation] problem — your implementation
/// provides the [`Worker`] + [`Waker`] pair that the [framework] needs to manage its
/// dedicated RRT thread.
///
/// For more details, see [module docs] for the full diagram.
///
/// # Example
///
/// See [`MioPollWorkerFactory`] for an example implementation.
///
/// [coupled resource creation]: super#the-coupled-resource-creation-problem
/// [framework]: super#the-rrt-contract-and-benefits
/// [module docs]: super#the-coupled-resource-creation-problem
/// [`MioPollWorkerFactory`]: crate::terminal_lib_backends::MioPollWorkerFactory
/// [`Waker`]: Self::Waker
/// [`Worker`]: Self::Worker
pub trait RRTFactory {
    /// The concrete type broadcast from your [`Worker`] implementation to async
    /// subscribers on the [framework]-managed dedicated RRT thread.
    ///
    /// See [`RRTWorker::Event`] for details.
    ///
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [`Worker`]: Self::Worker
    type Event;

    /// Your concrete type implementing one iteration of the blocking I/O loop on the
    /// [framework]-managed dedicated RRT thread.
    ///
    /// See [`RRTWorker`] for trait bounds rationale and design details.
    ///
    /// [framework]: super#the-rrt-contract-and-benefits
    type Worker: RRTWorker<Event = Self::Event>;

    /// Your concrete type for interrupting the blocked dedicated RRT worker thread.
    ///
    /// One waker instance is shared by all [`SubscriberGuard`]s. See [`RRTWaker`] for the
    /// shared-access pattern diagram.
    ///
    /// [`SubscriberGuard`]: super::SubscriberGuard
    type Waker: RRTWaker;

    /// Creates both of your [`Worker`] and [`Waker`] concrete types together.
    ///
    /// This method does not spawn the [framework]-managed dedicated RRT thread. This
    /// thread is created by the [framework] — when the TUI app (ie, async consumers)
    /// call [`subscribe()`].
    ///
    /// Your concrete type (that implements this method) is an injected dependency
    /// containing business logic that the [framework] is not aware of (and does not need
    /// to be).
    ///
    /// # Returns
    ///
    /// 1. The [`Worker`] concrete type → moves to the [framework]-managed dedicated RRT
    ///    worker thread
    /// 2. The [`Waker`] concrete type → stored in [`ThreadState`], which is wrapped in
    ///    [`Arc`] and held by each [`SubscriberGuard`]; this ONE [`waker`] is shared by
    ///    all async subscribers
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created.
    ///
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`subscribe()`]: super::ThreadSafeGlobalState::subscribe
    /// [`ThreadState`]: super::ThreadState
    /// [`Waker`]: Self::Waker
    /// [`waker`]: Self::Waker
    /// [`Worker`]: Self::Worker
    fn create() -> Result<(Self::Worker, Self::Waker), Report>;
}

/// A trait for implementing one iteration of the blocking I/O loop on the
/// [framework]-managed dedicated RRT thread.
///
/// The [framework] repeatedly calls [`poll_once()`] on the implementing type until it
/// returns [`Continuation::Stop`].
///
/// # Trait Bounds - [`Send`] + `'static`
///
/// An instance of the implementing type moves to the [framework]-managed dedicated RRT
/// worker thread and is owned exclusively by it. This is why
/// we need the following trait bounds:
/// - ✓ [`Send`]: The implementing type must be [`Send`] to move from the [async executor
///   thread] (on which [`subscribe()`] runs) to the [framework]-managed dedicated RRT
///   worker thread.
/// - ✓ `'static`: Required for [`std::thread::spawn()`] - any references the implementing
///   type contains must be `'static`, or it can own all its data with no references at
///   all.
/// - ✗ No [`Sync`] needed — the worker is owned, not shared.
///
/// # Example
///
/// See [`MioPollWorker`] for an example implementation that monitors [`stdin`] and
/// [`signals`].
///
/// # Design Rationale
///
/// This trait requires [`poll_once()`] (one iteration) rather than `run()` (entire loop).
/// This inversion of control provides:
///
/// - **Framework control**: Inject logging, metrics between iterations
/// - **Single responsibility**: Worker handles events, framework handles lifecycle
/// - **Testability**: Unit test [`poll_once()`] in isolation
///
/// [async executor thread]: tokio::runtime
/// [framework]: super#the-rrt-contract-and-benefits
/// [`MioPollWorker`]: crate::terminal_lib_backends::MioPollWorker
/// [`poll_once()`]: Self::poll_once
/// [`signals`]: https://en.wikipedia.org/wiki/Signal_(IPC)
/// [`stdin`]: std::io::stdin
/// [`subscribe()`]: super::ThreadSafeGlobalState::subscribe
pub trait RRTWorker: Send + 'static {
    /// The type containing domain-specific data to broadcast from your implementation to
    /// async consumers.
    ///
    /// This type must be [`Clone`] + [`Send`] + `'static` to satisfy the requirements of
    /// [`broadcast channel`]:
    /// - ✓ [`Clone`]: The [`broadcast channel`] clones each event for every [`Receiver`]
    ///   resulting in one clone per [`SubscriberGuard`].
    /// - ✓ [`Send`]: Events are produced on the [framework]-managed dedicated RRT worker
    ///   thread and consumed by async consumers / tasks running on [`tokio`] [executor
    ///   threads] (in the [multithreaded runtime]).
    /// - ✓ `'static`: Any references this event type contains must be `'static` (or the
    ///   event can own all its data with no references at all).
    ///
    /// [executor threads]: tokio::runtime
    /// [multithreaded runtime]: tokio::runtime::Builder::new_multi_thread
    /// [`Receiver`]: tokio::sync::broadcast::Receiver
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`broadcast channel`]: tokio::sync::broadcast
    /// [`tokio`]: tokio
    type Event: Clone + Send + 'static;

    /// Runs one iteration of the work loop.
    ///
    /// The [framework] calls this method repeatedly until [`Continuation::Stop`] is
    /// returned. Call [`tx.receiver_count()`] to check if any subscribers remain.
    ///
    /// # Returns
    ///
    /// - [`Continuation::Continue`] to keep the loop running
    /// - [`Continuation::Stop`] to exit the thread
    ///
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [`tx.receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn poll_once(&mut self, tx: &Sender<Self::Event>) -> Continuation;
}

/// A trait for interrupting the blocked [framework]-managed dedicated RRT worker thread.
///
/// [`SubscriberGuard::drop()`] calls [`wake()`] on implementors of this trait to signal
/// the worker to check if it should exit.
///
/// # Trait Bounds - [`Send`] + [`Sync`] + `'static`
///
/// There is exactly **one waker** inside the single [`ThreadState`], which all
/// [`SubscriberGuard::state`] instances share via [`Arc`]. When any async [`tokio`] task
/// drops its guard, the guard's [`Drop`] impl calls [`wake()`] on this shared waker to
/// interrupt the blocking thread:
///
/// ```text
/// ┌─────────────────────┐
/// │  Dedicated Thread   │ ◄─── wake() interrupts blocking call
/// │  (blocking on Poll) │
/// └─────────────────────┘
///           ▲
///           │
///    ┌──────┴──────┐
///    │     ONE     │ ◄─┬─── Async Task A drops guard ──► wake()
///    │    waker    │   │
///    │   (shared)  │ ◄─┴─── Async Task B drops guard ──► wake()
///    └─────────────┘
/// ```
///
/// This shared-access pattern requires the following trait bounds:
///
/// - **[`Send`]**: The implementor type lives inside [`SubscriberGuard::state`] (an
///   [`Arc<ThreadState>`]). For `Arc<T>` to be `Send`, `T` must be `Send + Sync` — so the
///   implementor type must be `Send`.
/// - **[`Sync`]**: Multiple async tasks (each holding a [`SubscriberGuard`]) may call
///   [`wake(&self)`] on the same implementor type concurrently from different [runtime
///   threads]. This bound is a **compile-time contract** — implementors must ensure
///   [`wake()`] is thread-safe:
///   - Types that aren't [`Sync`] (e.g., [`RefCell`]) cannot implement this trait.
///   - Types that ARE [`Sync`] (e.g., [`mio::Waker`] which uses thread-safe OS primitives
///     like [`eventfd`]) can.
/// - **`'static`**: Required for [`thread::spawn()`] - any references the implementor
///   type contains must be `'static` (or it can own all its data with no references at
///   all).
///
/// # Example
///
/// See [`MioPollWaker`] for an example implementation using [`mio::Waker`].
///
/// # Why User-Provided?
///
/// Wake strategies are backend-specific. See [Why is `RRTWaker` User-Provided?]
///
/// [`Arc<ThreadState>`]: super::ThreadState
/// [Why is `RRTWaker` User-Provided?]: super#why-is-rrtwaker-user-provided
/// [`Arc`]: std::sync::Arc
/// [`MioPollWaker`]: crate::terminal_lib_backends::MioPollWaker
/// [`RefCell`]: std::cell::RefCell
/// [runtime threads]: tokio::runtime
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`SubscriberGuard::state`]: super::SubscriberGuard::state
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`ThreadState`]: super::ThreadState
/// [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
/// [`mio::Waker`]: mio::Waker
/// [`thread::spawn()`]: std::thread::spawn
/// [`tokio`]: tokio
/// [`wake(&self)`]: RRTWaker::wake
/// [`wake()`]: RRTWaker::wake
pub trait RRTWaker: Send + Sync + 'static {
    /// Wake the thread so it can check if it should exit.
    ///
    /// # Idempotency
    ///
    /// Multiple concurrent calls are safe and harmless. Wakes may coalesce (worker
    /// wakes once) or cause multiple wakeups (worker loops again). Either way, the
    /// worker just checks [`receiver_count()`] and decides whether to exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the wake signal cannot be sent (typically non-fatal).
    ///
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn wake(&self) -> std::io::Result<()>;
}

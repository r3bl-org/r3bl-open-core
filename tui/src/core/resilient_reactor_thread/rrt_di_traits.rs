// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH epoll kqueue threadwaker rrtwaker

//! Core traits that allow you to add your business logic, using [dependency injection],
//! into the reusable Resilient Reactor Thread (RRT) [framework]. See the following
//! for more details: [`RRTFactory`], [`RRTWorker`], [`RRTWaker`].
//!
//! [framework]: super#the-rrt-contract-and-benefits
//! [dependency injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [`RRT`]: super::RRT
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html

use crate::core::common::Continuation;
use miette::Report;
use tokio::sync::broadcast::Sender;

/// A trait for creating OS resources which work with a coupled [`Worker`] and [`Waker`]
/// pair, that are used by the [framework]-managed dedicated RRT thread.
///
/// This is the main "entry point" for you to use the RRT [framework]. The journey begins
/// with you defining a static singleton of type [`RRT`] in your code and providing
/// concrete types that implement this trait (as well as the others). See the
/// [DI overview] for what each type ([`Worker`], [`Waker`], [`Event`]) provides and how
/// the [framework] orchestrates them.
///
/// This trait solves the [coupled resource creation] problem — see the [module docs] for
/// the full diagram. The [framework] is unaware, by design, of what blocking [`syscalls`]
/// are used in your implementation, and what sources are registered with them.
///
/// # Example
///
/// See [`MioPollWorkerFactory`] for an example implementation.
///
/// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [coupled resource creation]: super#the-coupled-resource-creation-problem
/// [DI overview]: super#separation-of-concerns-and-dependency-injection-di
/// [framework]: super#the-rrt-contract-and-benefits
/// [module docs]: super#the-coupled-resource-creation-problem
/// [`MioPollWorkerFactory`]: crate::terminal_lib_backends::MioPollWorkerFactory
/// [`Event`]: Self::Event
/// [`Waker`]: Self::Waker
/// [`Worker`]: Self::Worker
/// [`RRT`]: super::RRT
pub trait RRTFactory {
    /// The concrete type of the domain-specific payload broadcast from your [`Worker`]
    /// implementation to async subscribers. The [`Worker`] runs on the
    /// [framework]-managed dedicated RRT thread.
    ///
    /// See [`RRTWorker::Event`] for details.
    ///
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [`Worker`]: Self::Worker
    type Event;

    /// The concrete type implementing one iteration of the blocking I/O loop on the
    /// [framework]-managed dedicated RRT thread.
    ///
    /// See [`RRTWorker`] for trait bounds rationale and design details.
    ///
    /// [framework]: super#the-rrt-contract-and-benefits
    type Worker: RRTWorker<Event = Self::Event>;

    /// The concrete type for interrupting the blocked [framework]-managed dedicated RRT
    /// thread.
    ///
    /// One waker instance is shared by all [`SubscriberGuard`]s. See [`RRTWaker`] for the
    /// shared-access pattern diagram.
    ///
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [framework]: super#the-rrt-contract-and-benefits
    type Waker: RRTWaker;

    /// Creates OS resources and coupled [`Worker`] and [`Waker`] pair.
    ///
    /// The specifics of which [`syscalls`] your implementation uses, and what sources are
    /// registered, are totally left up to your implementation of this method. Your
    /// concrete type (that implements this method) is an injected dependency
    /// containing business logic that the [framework] is not aware of by design.
    ///
    /// See [trait docs] on details of orchestration between the [framework] and your code
    /// occurs via this trait and this method.
    ///
    /// This method does not spawn the [framework]-managed dedicated RRT thread. The RRT
    /// thread is created by the [framework] — when the TUI app (ie, async consumers) call
    /// [`subscribe()`].
    ///
    /// # Returns
    ///
    /// A coupled [`Worker`] + [`Waker`] pair — see [two-phase setup] for how these are
    /// distributed between the spawned thread and [`RRTState`].
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created.
    ///
    /// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [trait docs]: Self
    /// [two-phase setup]: super#the-coupled-resource-creation-problem
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [`subscribe()`]: super::RRT::subscribe
    /// [`RRTState`]: super::RRTState
    /// [`Waker`]: Self::Waker
    /// [`Worker`]: Self::Worker
    fn create() -> Result<(Self::Worker, Self::Waker), Report>;
}

/// A trait for implementing one iteration of the blocking I/O loop on the
/// [framework]-managed dedicated RRT thread.
///
/// The [framework] repeatedly calls [`poll_once()`] on the implementing type until it
/// returns [`Continuation::Stop`]. This is where the implementing type can add business
/// logic, to inject it into the [framework]. Typically, this business logic gets any data
/// from sources that are ready and then converts them a domain-specific [`event`] type
/// that is broadcast to all the async consumers.
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
/// [`subscribe()`]: super::RRT::subscribe
/// [`event`]: Self::Event
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
    /// [framework]: super#the-rrt-contract-and-benefits
    type Event: Clone + Send + 'static;

    /// Runs one iteration of the work loop; this loop is owned by the [framework] and it
    /// runs on the [framework]-managed dedicated RRT thread.
    ///
    /// The [framework] calls this method repeatedly until [`Continuation::Stop`] is
    /// returned. See the [trait docs] for details on the business logic that you will
    /// typically add here in your implementation of this trait method.
    ///
    /// Pro-tip - you can call [`tx.receiver_count()`] to check if any subscribers remain.
    ///
    /// # Example
    ///
    /// See [`mio_poller::MioPollWorker`] for a real example of implementing this method.
    ///
    /// # Returns
    ///
    /// - [`Continuation::Continue`] to keep the loop running
    /// - [`Continuation::Stop`] to exit the thread
    ///
    /// [`mio_poller::MioPollWorker`]: crate::direct_to_ansi::input::mio_poller::MioPollWorker
    /// [trait docs]: Self
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
/// There is exactly **one waker** inside the single [`RRTState`], which all
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
///   [`Arc<RRTState>`]). For `Arc<T>` to be `Send`, `T` must be `Send + Sync` — so the
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
/// [`Arc<RRTState>`]: super::RRTState
/// [Why is `RRTWaker` User-Provided?]: super#why-is-rrtwaker-user-provided
/// [`Arc`]: std::sync::Arc
/// [`MioPollWaker`]: crate::terminal_lib_backends::MioPollWaker
/// [`RefCell`]: std::cell::RefCell
/// [runtime threads]: tokio::runtime
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`SubscriberGuard::state`]: super::SubscriberGuard::state
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`RRTState`]: super::RRTState
/// [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
/// [`mio::Waker`]: mio::Waker
/// [`thread::spawn()`]: std::thread::spawn
/// [`tokio`]: tokio
/// [`wake(&self)`]: RRTWaker::wake
/// [`wake()`]: RRTWaker::wake
/// [framework]: super#the-rrt-contract-and-benefits
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH epoll kqueue threadwaker rrtwaker

//! Core traits that allow you to add your business logic, using [dependency injection],
//! into the reusable Resilient Reactor Thread ([`RRT`]) [framework]. See the following
//! for more details: [`RRTFactory`], [`RRTWorker`], [`RRTWaker`].
//!
//! [`RRT`]: super::RRT
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [dependency injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [framework]: super#the-rrt-contract-and-benefits

use super::{RRTEvent, RestartPolicy};
use crate::core::common::Continuation;
use miette::Report;
use tokio::sync::broadcast::Sender;

/// A trait for creating OS resources which work with a coupled [`RRTWorker`] and
/// [`RRTWaker`] pair, that are used by the [framework]-managed dedicated [`RRT`] thread.
///
/// This is the main "entry point" for you to use the RRT [framework]. The journey begins
/// with you defining a static singleton of type [`RRT`] in your code and providing
/// concrete types that implement this trait (as well as the others). See the [DI
/// overview] for what each type ([`RRTWorker`], [`RRTWaker`], [`Event`]) provides and how
/// the [framework] orchestrates them.
///
/// This trait implements [two-phase setup] - see the [module docs] for the full diagram.
/// The [framework] is unaware, by design, of what blocking [`syscalls`] are used in your
/// implementation, and what sources are registered with them.
///
/// If you don't want to use the [default policy], simply override [`restart_policy()`] to
/// customize [self-healing restart] behavior.
///
/// # Example
///
/// See [`MioPollWorkerFactory`] for an example implementation.
///
/// [DI overview]: super#separation-of-concerns-and-dependency-injection-di
/// [`Event`]: Self::Event
/// [`MioPollWorkerFactory`]: crate::terminal_lib_backends::MioPollWorkerFactory
/// [`RRT`]: super::RRT
/// [`restart_policy()`]: Self::restart_policy
/// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
/// [framework]: super#the-rrt-contract-and-benefits
/// [module docs]: super#two-phase-setup
/// [self-healing restart]: super#self-healing-restart-details
/// [two-phase setup]: super#two-phase-setup
pub trait RRTFactory {
    /// The concrete type of the domain-specific payload broadcast from your [`RRTWorker`]
    /// trait implementation to async subscribers. Your [`RRTWorker`] trait implementation
    /// runs on the [framework]-managed dedicated RRT thread.
    ///
    /// See [`RRTWorker::Event`] for details.
    ///
    /// [framework]: super#the-rrt-contract-and-benefits
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
    /// One [`RRTWaker`] instance is shared by all [`SubscriberGuard`]s. See [`RRTWaker`]
    /// for the shared-access pattern diagram.
    ///
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [framework]: super#the-rrt-contract-and-benefits
    type Waker: RRTWaker;

    /// Creates OS resources and coupled [`RRTWorker`] and [`RRTWaker`] pair.
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
    /// thread is created by the [framework] - when your app (ie, async consumers) calls
    /// [`subscribe()`].
    ///
    /// This method is also called during [self-healing restart] to create fresh OS
    /// resources after the current [`RRTWorker`] is dropped.
    ///
    /// # Returns
    ///
    /// A coupled [`RRTWorker`] + [`RRTWaker`] pair - see [two-phase setup] for how these
    /// are distributed between the spawned thread and [`RRT`]'s shared waker wrapper.
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created.
    ///
    /// [`RRT`]: super::RRT
    /// [`subscribe()`]: super::RRT::subscribe
    /// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [self-healing restart]: super#self-healing-restart-details
    /// [trait docs]: Self
    /// [two-phase setup]: super#two-phase-setup
    fn create() -> Result<(Self::Worker, Self::Waker), Report>;

    /// Returns the restart policy for this factory.
    ///
    /// The framework consults this policy when your [`RRTWorker`] trait implementation
    /// returns [`Continuation::Restart`]. Override to customize the number of restart
    /// attempts, delay, and backoff behavior.
    ///
    /// See the [default policy] for the
    /// specific values used when this method is not overridden.
    ///
    /// See [self-healing restart details] for the full restart lifecycle.
    ///
    /// [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
    /// [self-healing restart details]: super#self-healing-restart-details
    fn restart_policy() -> RestartPolicy { RestartPolicy::default() }
}

/// A trait for implementing one iteration of the blocking I/O loop on the
/// [framework]-managed dedicated RRT thread.
///
/// The [framework] repeatedly calls [`poll_once()`] on the implementing type until it
/// returns [`Continuation::Stop`] or [`Continuation::Restart`]. This is where the
/// implementing type can add business logic, to inject it into the [framework].
/// Typically, this business logic gets any data from sources that are ready and then
/// converts them into a domain-specific [`event`] type that is broadcast to all the async
/// consumers.
///
/// Returning [`Continuation::Restart`] triggers [self-healing restart] - the framework
/// drops your [`RRTWorker`], applies the [`RestartPolicy`], and creates a fresh
/// [`RRTWorker`] via [`RRTFactory::create()`]. Use this for recoverable failures like OS
/// event mechanism errors.
///
/// # Trait Bounds - [`Send`] + `'static`
///
/// An instance of the implementing type moves to the [framework]-managed dedicated RRT
/// thread and is owned exclusively by it. This is why
/// we need the following trait bounds:
/// - ✓ [`Send`]: The implementing type must be [`Send`] to move from the [async executor
///   thread] (on which [`subscribe()`] runs) to the [framework]-managed dedicated RRT
///   thread.
/// - ✓ `'static`: Required for [`std::thread::spawn()`] - any references the implementing
///   type contains must be `'static`, or it can own all its data with no references at
///   all.
/// - ✗ No [`Sync`] needed - your `RRTWorker` instance is owned by the dedicated thread,
///   not shared.
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
/// - **Single responsibility**: Your [`RRTWorker`] implementation handles events,
///   framework handles lifecycle
/// - **Testability**: Unit test [`poll_once()`] in isolation
///
/// [`MioPollWorker`]: crate::terminal_lib_backends::MioPollWorker
/// [`event`]: Self::Event
/// [`poll_once()`]: Self::poll_once
/// [`signals`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`stdin`]: std::io::stdin
/// [`subscribe()`]: super::RRT::subscribe
/// [async executor thread]: tokio::runtime
/// [framework]: super#the-rrt-contract-and-benefits
/// [self-healing restart]: super#self-healing-restart-details
pub trait RRTWorker: Send + 'static {
    /// The type containing domain-specific data to broadcast from your implementation to
    /// async consumers.
    ///
    /// This type must be [`Clone`] + [`Send`] + `'static` to satisfy the requirements of
    /// - ✓ [`Clone`]: The [`broadcast channel`] clones each event for every [`Receiver`]
    ///   resulting in one clone per [`SubscriberGuard`].
    /// - ✓ [`Send`]: Events are produced on the [framework]-managed dedicated RRT thread
    ///   and consumed by async consumers / tasks running on [`tokio`] [executor threads]
    ///   (in the [multithreaded runtime]).
    /// - ✓ `'static`: Any references this event type contains must be `'static` (or the
    ///   event can own all its data with no references at all).
    ///
    /// [`Receiver`]: tokio::sync::broadcast::Receiver
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`broadcast channel`]:
    /// [`broadcast channel`]: tokio::sync::broadcast
    /// [`tokio`]: tokio
    /// [executor threads]: tokio::runtime
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [multithreaded runtime]: tokio::runtime::Builder::new_multi_thread
    type Event: Clone + Send + 'static;

    /// Runs one iteration of the work loop; this loop is owned by the [framework] and it
    /// runs on the [framework]-managed dedicated RRT thread.
    ///
    /// The [framework] calls this method repeatedly until [`Continuation::Stop`] or
    /// [`Continuation::Restart`] is returned. See the [trait docs] for details on the
    /// business logic that you will typically add here in your implementation of this
    /// trait method.
    ///
    /// Your implementation wraps domain events in [`RRTEvent::Worker(...)`] before
    /// sending them through `tx`. The framework uses `tx` to send
    /// [`RRTEvent::Shutdown`] when the restart policy is exhausted.
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
    /// - [`Continuation::Stop`] to exit the thread (always respected)
    /// - [`Continuation::Restart`] to request a fresh worker via [`RRTFactory::create()`]
    ///
    /// [`RRTEvent::Shutdown`]: RRTEvent::Shutdown
    /// [`RRTEvent::Worker(...)`]: RRTEvent::Worker
    /// [`RRTFactory::create()`]: RRTFactory::create
    /// [`mio_poller::MioPollWorker`]: crate::direct_to_ansi::input::mio_poller::MioPollWorker
    /// [`tx.receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [trait docs]: Self
    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation;
}

/// A trait for interrupting the blocked [framework]-managed dedicated RRT thread.
///
/// [`SubscriberGuard::drop()`] calls [`wake()`] on implementors of this trait to signal
/// the dedicated thread to check if it should exit.
///
/// # Trait Bounds - [`Send`] + [`Sync`] + `'static`
///
/// There is exactly **one [`RRTWaker`] trait implementation** inside [`RRT`]'s shared
/// [`RRTWaker`] wrapper (`Arc<Mutex<Option<W>>>`), which all [`SubscriberGuard`]
/// instances share via [`Arc`]. When any async [`tokio`] task drops its guard, the
/// guard's [`Drop`] impl calls [`wake()`] on this shared [`RRTWaker`] trait
/// implementation to interrupt the blocking thread:
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
/// - **[`Send`]**: The implementor type lives inside [`RRT`]'s shared [`RRTWaker`]
///   wrapper (an `Arc<Mutex<Option<W>>>`). For `Arc<T>` to be `Send`, `T` must be `Send +
///   Sync` - so the implementor type must be `Send`.
/// - **[`Sync`]**: Multiple async tasks (each holding a [`SubscriberGuard`]) may lock the
///   shared [`RRTWaker`] and call [`wake(&self)`] on the same implementor type
///   concurrently from different [runtime threads]. This bound is a **compile-time
///   contract** - implementors must ensure [`wake()`] is thread-safe:
///  - Types that aren't [`Sync`] (e.g., [`RefCell`]) cannot implement this trait.
///  - Types that ARE [`Sync`] (e.g., [`mio::Waker`] which uses thread-safe OS primitives
///    like [`eventfd`]) can.
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
/// [Why is `RRTWaker` User-Provided?]: super#why-is-rrtwaker-user-provided
/// [`Arc`]: std::sync::Arc
/// [`MioPollWaker`]: crate::terminal_lib_backends::MioPollWaker
/// [`RRT`]: super::RRT
/// [`RefCell`]: std::cell::RefCell
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
/// [`mio::Waker`]: mio::Waker
/// [`thread::spawn()`]: std::thread::spawn
/// [`tokio`]: tokio
/// [`wake(&self)`]: RRTWaker::wake
/// [`wake()`]: RRTWaker::wake
/// [framework]: super#the-rrt-contract-and-benefits
/// [runtime threads]: tokio::runtime
pub trait RRTWaker: Send + Sync + 'static {
    /// Wake the thread so it can check if it should exit.
    ///
    /// # Idempotency
    ///
    /// Multiple concurrent calls are safe and harmless. Wakes may coalesce (the dedicated
    /// thread wakes once) or cause multiple wakeups (it loops again). Either way, the
    /// dedicated thread just checks [`receiver_count()`] and decides whether to exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the wake signal cannot be sent (typically non-fatal).
    ///
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn wake(&self) -> std::io::Result<()>;
}

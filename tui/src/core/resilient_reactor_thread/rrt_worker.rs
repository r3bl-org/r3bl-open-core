// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH epoll kqueue threadwaker rrtwaker

//! Core traits for adding your business logic, using [dependency injection], into the
//! reusable Resilient Reactor Thread ([`RRT`]) [framework]. See the following for more
//! details: [`RRTWorker`], [`RRTWaker`].
//!
//! [`RRT`]: super::RRT
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [dependency injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [framework]: super#the-rrt-contract-and-benefits

use super::{RRTEvent, RestartPolicy};
use crate::core::common::Continuation;
use tokio::sync::broadcast::Sender;

/// A trait for waking the blocked [framework]-managed dedicated RRT thread.
///
/// Your implementation of [`RRTWaker`] wraps whatever mechanism your [blocking I/O
/// backend] provides for interrupt signaling. For example, [`MioPollWaker`] wraps a
/// [`mio::Waker`] that triggers an [`epoll`]/[`kqueue`] wakeup.
///
/// [`SubscriberGuard::drop()`] calls [`wake()`] to signal the dedicated thread to check
/// if it should exit.
///
/// # Trait Bounds - [`Send`] + [`Sync`] + `'static`
///
/// There is exactly **one waker** inside [`RRT`]'s shared [`waker`] wrapper, which all
/// [`SubscriberGuard`] instances share. When any subscriber drops its guard, the guard's
/// [`Drop`] impl calls [`wake()`] to interrupt the blocking thread:
///
/// ```text
/// ┌─────────────────────┐
/// │  Dedicated Thread   │ ◄─── waker.wake() interrupts blocking call
/// │  (blocking on Poll) │
/// └─────────────────────┘
///           ▲
///           │
///    ┌──────┴──────┐
///    │     ONE     │ ◄─┬─── Async Task A drops guard ──► waker.wake()
///    │    waker    │   │
///    │   (shared)  │ ◄─┴─── Async Task B drops guard ──► waker.wake()
///    └─────────────┘
/// ```
///
/// This shared-access pattern requires [`Send`] + [`Sync`]:
///
/// - **[`Send`]**: The closure lives inside [`RRT`]'s shared waker wrapper. For `Arc<T>`
///   to be `Send`, `T` must be `Send + Sync`, so the closure must be `Send`.
/// - **[`Sync`]**: Multiple async tasks (each holding a [`SubscriberGuard`]) may lock the
///   shared waker and call the closure concurrently from different [runtime threads]. The
///   closure must be thread-safe.
///
/// # Idempotency
///
/// Multiple concurrent calls are safe and harmless. Wakes may coalesce (the dedicated
/// thread wakes once) or cause multiple wakeups (it loops again). Either way, the
/// dedicated thread just checks [`receiver_count()`] and decides whether to exit.
///
/// # Why User-Provided?
///
/// Wake strategies are backend-specific. See [Why is `RRTWaker` User-Provided?]
///
/// [Why is `RRTWaker` User-Provided?]: super#why-is-rrtwaker-user-provided
/// [`Arc`]: std::sync::Arc
/// [`MioPollWaker`]: crate::terminal_lib_backends::MioPollWaker
/// [`RRT`]: super::RRT
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`mio::Waker`]: mio::Waker
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`tokio`]: tokio
/// [`wake()`]: Self::wake
/// [`waker`]: field@super::RRT::waker
/// [blocking I/O backend]: super#understanding-blocking-io
/// [framework]: super#the-rrt-contract-and-benefits
/// [runtime threads]: tokio::runtime
pub trait RRTWaker: Send + Sync + 'static {
    /// Wakes the blocked dedicated RRT thread.
    ///
    /// This method is called by [`SubscriberGuard::drop()`] to interrupt the dedicated
    /// thread's blocking call (e.g., [`mio::Poll::poll()`]). The thread then checks
    /// [`receiver_count()`] and decides whether to exit.
    ///
    /// Implementations should be idempotent - multiple concurrent calls must be safe.
    ///
    /// [`SubscriberGuard::drop()`]: super::SubscriberGuard
    /// [`mio::Poll::poll()`]: mio::Poll::poll
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn wake(&self);
}

/// A trait for implementing the blocking I/O worker on the [framework]-managed dedicated
/// RRT thread.
///
/// This is the main "entry point" for you to use the RRT [framework]. The journey begins
/// with you defining a static singleton of type [`RRT`] in your code and providing a
/// concrete type that implements this trait. See the [DI overview] for what each piece
/// ([`RRTWorker`], [`RRTWaker`], [`Event`]) provides and how the [framework] orchestrates
/// them.
///
/// This trait handles both **resource creation** ([`create()`]) and **one iteration of
/// the blocking I/O loop** ([`poll_once()`]).
///
/// [`create()`] implements [two-phase setup] - see the [module docs] for the full
/// diagram. The [framework] is unaware, by design, of what blocking [`syscalls`] are used
/// in your implementation, and what sources are registered with them.
///
/// If you don't want to use the [default policy], simply override [`restart_policy()`] to
/// customize [self-healing restart] behavior.
///
/// The [framework] repeatedly calls [`poll_once()`] on the implementing type until it
/// returns [`Continuation::Stop`] or [`Continuation::Restart`]. Typically, your business
/// logic gets any data from sources that are ready and then converts them into a
/// domain-specific [`event`] type that is broadcast to all the async consumers.
///
/// Returning [`Continuation::Restart`] triggers [self-healing restart] - the framework
/// drops the current worker, applies the [`RestartPolicy`], and creates a fresh worker
/// via [`create()`].
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
/// [DI overview]: super#separation-of-concerns-and-dependency-injection-di
/// [Event]: Self::Event
/// [`MioPollWorker`]: crate::terminal_lib_backends::MioPollWorker
/// [`RRT`]: super::RRT
/// [`create()`]: Self::create
/// [`event`]: Self::Event
/// [`poll_once()`]: Self::poll_once
/// [`restart_policy()`]: Self::restart_policy
/// [`signals`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`stdin`]: std::io::stdin
/// [`subscribe()`]: super::RRT::subscribe
/// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
/// [framework]: super#the-rrt-contract-and-benefits
/// [module docs]: super#two-phase-setup
/// [self-healing restart]: super#self-healing-restart-details
/// [two-phase setup]: super#two-phase-setup
pub trait RRTWorker: Send + 'static {
    /// Capacity of the [`broadcast channel`] for events.
    ///
    /// When the buffer is full, the oldest message is dropped to make room for new
    /// ones. Slow consumers will receive [`Lagged`] on their next [`recv()`] call,
    /// indicating how many messages they missed.
    ///
    /// `4_096` is generous for typical event streams, but cheap (events are usually
    /// small) and provides headroom for debug/logging consumers that might occasionally
    /// lag.
    ///
    /// Override this in your [`RRTWorker`] implementation to customize the channel
    /// capacity.
    ///
    /// [`Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
    /// [`broadcast channel`]: tokio::sync::broadcast
    /// [`recv()`]: tokio::sync::broadcast::Receiver::recv
    const CHANNEL_CAPACITY: usize = 4_096;

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
    type Event: Clone + Send + Sync + 'static;

    /// Creates OS resources and a coupled worker + [waker] pair.
    ///
    /// The specifics of which [`syscalls`] your implementation uses, and what sources are
    /// registered, are totally left up to your implementation of this method. Your
    /// concrete type (that implements this method) is an injected dependency containing
    /// business logic that the [framework] is not aware of by design.
    ///
    /// The [waker] is tightly coupled to the worker's blocking mechanism (e.g.,
    /// [`mio::Poll`]). Since a [`mio::Waker`] is bound to the [`Poll`] instance it was
    /// created from, this worker and [waker] must be created together. This is why this
    /// method returns both as a pair.
    ///
    /// The concrete waker type is erased via [`impl RRTWaker`] so it does not need to
    /// appear as a generic parameter on framework types.
    ///
    /// This method does not spawn the dedicated thread - that happens when your app calls
    /// [`subscribe()`].
    ///
    /// This method is also called during [self-healing restart] to create fresh OS
    /// resources after the current worker is dropped.
    ///
    /// # Returns
    ///
    /// This worker and its [waker] pair. See [two-phase setup] for how these are
    /// distributed between the spawned thread and [`RRT`]'s shared waker wrapper.
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created.
    ///
    /// [`Poll`]: mio::Poll
    /// [`RRT`]: super::RRT
    /// [`impl RRTWaker`]: RRTWaker
    /// [`mio::Poll`]: mio::Poll
    /// [`mio::Waker`]: mio::Waker
    /// [`subscribe()`]: super::RRT::subscribe
    /// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [self-healing restart]: super#self-healing-restart-details
    /// [two-phase setup]: super#two-phase-setup
    /// [waker]: super::RRTWaker
    fn create() -> miette::Result<(Self, impl RRTWaker)>
    where
        Self: Sized;

    /// Returns the restart policy for this worker.
    ///
    /// The framework consults this policy when your [`RRTWorker`] implementation returns
    /// [`Continuation::Restart`]. Override to customize the number of restart attempts,
    /// delay, and backoff behavior.
    ///
    /// See the [default policy] for the specific values used when this method is not
    /// overridden.
    ///
    /// See [self-healing restart details] for the full restart lifecycle.
    ///
    /// [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
    /// [self-healing restart details]: super#self-healing-restart-details
    fn restart_policy() -> RestartPolicy
    where
        Self: Sized,
    {
        RestartPolicy::default()
    }

    /// Runs one iteration of the work loop; this loop is owned by the [framework] and it
    /// runs on the [framework]-managed dedicated RRT thread.
    ///
    /// The [framework] calls this method repeatedly until [`Continuation::Stop`] or
    /// [`Continuation::Restart`] is returned. See the [trait docs] for details on the
    /// business logic that you will typically add here in your implementation of this
    /// trait method.
    ///
    /// Your implementation wraps domain events in [`RRTEvent::Worker(...)`] before
    /// sending them through `sender`. The framework uses `sender` to send
    /// [`RRTEvent::Shutdown`] when the restart policy is exhausted.
    ///
    /// Pro-tip - you can call [`sender.receiver_count()`] to check if any subscribers
    /// remain.
    ///
    /// # Example
    ///
    /// See [`mio_poller::MioPollWorker`] for a real example of implementing this method.
    ///
    /// # Returns
    ///
    /// - [`Continuation::Continue`] to keep the loop running
    /// - [`Continuation::Stop`] to exit the thread (always respected)
    /// - [`Continuation::Restart`] to request a fresh worker via [`create()`]
    ///
    /// [`RRTEvent::Shutdown`]: RRTEvent::Shutdown
    /// [`RRTEvent::Worker(...)`]: RRTEvent::Worker
    /// [`create()`]: Self::create
    /// [`mio_poller::MioPollWorker`]: crate::direct_to_ansi::input::mio_poller::MioPollWorker
    /// [`sender.receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [trait docs]: Self
    fn poll_once(&mut self, sender: &Sender<RRTEvent<Self::Event>>) -> Continuation;
}

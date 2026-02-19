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
/// [`SubscriberGuard::drop()`] calls [`wake_and_unblock_dedicated_thread()`] to signal
/// the dedicated thread to check if it should exit.
///
/// # Trait Bounds - [`Send`] + [`Sync`] + `'static`
///
/// There is exactly **one waker** inside [`RRT`]'s shared [`waker`] wrapper, which all
/// [`SubscriberGuard`] instances share. When any subscriber drops its guard, the guard's
/// [`Drop`] impl calls [`wake_and_unblock_dedicated_thread()`] to interrupt the blocking
/// thread:
///
/// ```text
/// ┌─────────────────────┐
/// │  Dedicated Thread   │ ◄─── waker.wake_and_unblock_dedicated_thread()
/// │  (blocking on Poll) │      interrupts blocking call
/// └─────────────────────┘
///           ▲
///           │
///    ┌──────┴──────┐
///    │     ONE     │ ◄─┬─── Async Task A drops guard
///    │    waker    │   │    └──► waker.wake_and_unblock_dedicated_thread()
///    │   (shared)  │ ◄─┴─── Async Task B drops guard
///    └─────────────┘        └──► waker.wake_and_unblock_dedicated_thread()
/// ```
///
/// This shared-access pattern requires [`Send`] + [`Sync`]:
///
/// - **[`Send`]**: The closure lives inside [`RRT`]'s shared waker wrapper. For `Arc<T>`
///   to be `Send`, `T` must be `Send + Sync`, so the closure must be `Send`.
/// - **[`Sync`]**: Multiple async tasks (each holding a [`SubscriberGuard`]) may lock the
///   shared waker and call the closure concurrently from different [runtime threads]. The
///   closure must be thread-safe.
/// - **[`'static`]**: Required for thread spawning. See [`RRT`] for a detailed
///   explanation of the [`'static` trait bound][`'static`].
///
///
/// # Idempotency
///
/// Multiple concurrent calls are safe and harmless. Wakes may coalesce (the dedicated
/// thread wakes once) or cause multiple wakeups (it loops again). Either way, the
/// dedicated thread just checks [`receiver_count()`] and decides whether to exit.
///
/// # Poll -> Registry -> Waker Chain
///
/// Your [`RRTWaker`] implementation is tightly coupled to its [blocking mechanism] (e.g.,
/// [`mio::Poll`]):
///
/// ```text
/// mio::Poll::new()                       // Creates OS event mechanism
///       │                                // (epoll fd / kqueue)
///       ▼
/// poll.registry()                        // Handle to register interest
///       │
///       ▼
/// Waker::new(registry)                   // Registers with THIS Poll's mechanism
///       │
///       ▼
/// waker                                  // Triggers event → poll.poll() returns
/// .wake_and_unblock_dedicated_thread()
/// ```
///
/// Since a [`Waker`] is bound to the [`Poll`] instance it was created from, replacing one
/// without the other leaves a dead reference. This is why the slow path replaces **both**
/// together (see [two-phase setup]).
///
/// # Why User-Provided?
///
/// Wake strategies are backend-specific. See [Why is `RRTWaker` User-Provided?]
///
/// [Why is `RRTWaker` User-Provided?]: super#why-is-rrtwaker-user-provided
/// [`'static`]: super::RRT#static-trait-bound-vs-static-lifetime-annotation
/// [`Arc`]: std::sync::Arc
/// [`MioPollWaker`]: crate::terminal_lib_backends::MioPollWaker
/// [`Poll`]: mio::Poll
/// [`RRT`]: super::RRT
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`Waker`]: mio::Waker
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`mio::Waker`]: mio::Waker
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`tokio`]: tokio
/// [`wake_and_unblock_dedicated_thread()`]: Self::wake_and_unblock_dedicated_thread
/// [`waker`]: field@super::RRT::waker
/// [blocking I/O backend]: super#understanding-blocking-io
/// [blocking mechanism]: super#understanding-blocking-io
/// [framework]: super#the-rrt-contract-and-benefits
/// [runtime threads]: tokio::runtime
/// [two-phase setup]: super#two-phase-setup
pub trait RRTWaker: Send + Sync + 'static {
    /// Wakes the OS event mechanism registered during
    /// [`create_and_register_os_sources()`], unblocking the dedicated RRT thread's
    /// [`block_until_ready_then_dispatch()`] call.
    ///
    /// This method is called by [`SubscriberGuard::drop()`] to interrupt the dedicated
    /// thread's blocking [`syscall`] (e.g., [`mio::Poll::poll()`]). The thread then
    /// checks [`receiver_count()`] and decides whether to exit.
    ///
    /// Implementations should be idempotent - multiple concurrent calls must be safe.
    ///
    /// [`SubscriberGuard::drop()`]: super::SubscriberGuard
    /// [`block_until_ready_then_dispatch()`]: RRTWorker::block_until_ready_then_dispatch
    /// [`create_and_register_os_sources()`]: RRTWorker::create_and_register_os_sources
    /// [`mio::Poll::poll()`]: mio::Poll::poll
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    fn wake_and_unblock_dedicated_thread(&self);
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
/// This trait handles both **resource creation**
/// ([`create_and_register_os_sources()`]) and **one iteration of the blocking I/O loop**
/// ([`block_until_ready_then_dispatch()`]).
///
/// [`create_and_register_os_sources()`] implements [two-phase setup] - see the [module
/// docs] for the full diagram. The [framework] is unaware, by design, of what blocking
/// [`syscalls`] are used in your implementation, and what sources are registered with
/// them.
///
/// If you don't want to use the [default policy], simply override [`restart_policy()`] to
/// customize [self-healing restart] behavior.
///
/// The [framework] repeatedly calls [`block_until_ready_then_dispatch()`] on the
/// implementing type until it returns [`Continuation::Stop`] or
/// [`Continuation::Restart`]. Typically, your business logic gets any data from sources
/// that are ready and then converts them into a domain-specific [`event`] type that is
/// broadcast to all the async consumers.
///
/// Returning [`Continuation::Restart`] triggers [self-healing restart] - the framework
/// drops the current worker, applies the [`RestartPolicy`], and creates a fresh worker
/// via [`create_and_register_os_sources()`].
///
/// # Trait Bounds - [`Send`] + `'static`
///
/// An instance of the implementing type moves to the [framework]-managed dedicated RRT
/// thread and is owned exclusively by it. This is why
/// we need the following trait bounds:
/// - ✓ [`Send`]: The implementing type must be [`Send`] to move from the [async executor
///   thread] (on which [`subscribe()`] runs) to the [framework]-managed dedicated RRT
///   thread.
/// - ✓ [`'static`]: Required for [`std::thread::spawn()`]. See [`RRT`] for a detailed
///   explanation of the [`'static` trait bound][`'static`].
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
/// This trait requires [`block_until_ready_then_dispatch()`] (one iteration) rather than
/// `run()` (entire loop). This inversion of control provides:
///
/// - **Framework control**: Inject logging, metrics between iterations
/// - **Single responsibility**: Your [`RRTWorker`] implementation handles events,
///   framework handles lifecycle
/// - **Testability**: Unit test [`block_until_ready_then_dispatch()`] in isolation
///
/// [DI overview]: super#separation-of-concerns-and-dependency-injection-di
/// [Event]: Self::Event
/// [`'static`]: super::RRT#static-trait-bound-vs-static-lifetime-annotation
/// [`MioPollWorker`]: crate::terminal_lib_backends::MioPollWorker
/// [`RRT`]: super::RRT
/// [`block_until_ready_then_dispatch()`]: Self::block_until_ready_then_dispatch
/// [`create_and_register_os_sources()`]: Self::create_and_register_os_sources
/// [`event`]: Self::Event
/// [`restart_policy()`]: Self::restart_policy
/// [`signals`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`stdin`]: std::io::stdin
/// [`subscribe()`]: super::RRT::subscribe
/// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
/// [framework]: super#the-rrt-contract-and-benefits
/// [module docs]: super::RRT#two-phase-setup
/// [self-healing restart]: super#self-healing-restart-details
/// [two-phase setup]: super::RRT#two-phase-setup
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
    /// - ✓ [`'static`]: Required for thread spawning. See [`RRT`] for a detailed
    ///   explanation of the [`'static` trait bound][`'static`].
    ///
    /// [`'static`]: super::RRT#static-trait-bound-vs-static-lifetime-annotation
    /// [`RRT`]: super::RRT
    /// [`Receiver`]: tokio::sync::broadcast::Receiver
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`broadcast channel`]:
    /// [`broadcast channel`]: tokio::sync::broadcast
    /// [`tokio`]: tokio
    /// [executor threads]: tokio::runtime
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [multithreaded runtime]: tokio::runtime::Builder::new_multi_thread
    type Event: Clone + Send + Sync + 'static;

    /// The concrete waker type returned by [`create_and_register_os_sources()`]
    /// alongside the worker.
    ///
    /// This associated type threads the concrete waker through framework types
    /// ([`SharedWakerSlot`], [`SubscriberGuard`], [`TerminationGuard`]) at the type
    /// level, eliminating dynamic dispatch.
    ///
    /// [`SharedWakerSlot`]: super::SharedWakerSlot
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`TerminationGuard`]: super::TerminationGuard
    /// [`create_and_register_os_sources()`]: Self::create_and_register_os_sources
    type Waker: RRTWaker;

    /// Creates OS resources, registers event sources, and returns a coupled worker +
    /// [waker] pair.
    ///
    /// The specifics of which [`syscalls`] your implementation uses, and what sources are
    /// registered, are totally left up to your implementation of this method. Your
    /// concrete type (that implements this method) is an injected dependency containing
    /// business logic that the [framework] is not aware of by design.
    ///
    /// The [waker] is tightly coupled to the worker's blocking mechanism (e.g.,
    /// [`mio::Poll`]). Since a [`mio::Waker`] is bound to the [`Poll`] instance it was
    /// created from, this worker and [waker] must be created together. This is why this
    /// method returns both as a pair. See [`RRT`]'s [Two-Phase Setup] section for the
    /// full explanation of why the pair is returned and where each piece goes (waker to
    /// [`shared_waker_slot`], worker to thread-local stack in [`run_worker_loop()`]).
    ///
    /// The concrete waker type [`Self::Waker`] is threaded through framework types
    /// at the type level, eliminating dynamic dispatch.
    ///
    /// This method does not spawn the dedicated thread - that happens when your app calls
    /// [`subscribe()`]. See [Thread Lifecycle] for the full spawn/reuse/terminate
    /// sequence.
    ///
    /// This method is also called during [self-healing restart] to create fresh OS
    /// resources after the current worker is dropped.
    ///
    /// # Returns
    ///
    /// This worker and its [waker] pair. See [Two-Phase Setup] for how these are
    /// distributed between the spawned thread and [`RRT`]'s shared waker wrapper.
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created.
    ///
    /// [Thread Lifecycle]: super::RRT#thread-lifecycle
    /// [Two-Phase Setup]: super::RRT#two-phase-setup
    /// [`Poll`]: mio::Poll
    /// [`RRT`]: super::RRT
    /// [`Self::Waker`]: Self::Waker
    /// [`mio::Poll`]: mio::Poll
    /// [`mio::Waker`]: mio::Waker
    /// [`run_worker_loop()`]: super::run_worker_loop
    /// [`shared_waker_slot`]: field@super::RRT::shared_waker_slot
    /// [`subscribe()`]: super::RRT::subscribe
    /// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [self-healing restart]: super#self-healing-restart-details
    /// [waker]: super::RRTWaker
    fn create_and_register_os_sources() -> miette::Result<(Self, Self::Waker)>
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

    /// Blocks until at least one I/O source is ready, then processes ready sources and
    /// dispatches domain events to async consumers; this is one iteration of the work
    /// loop owned by the [framework], running on the [framework]-managed dedicated RRT
    /// thread.
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
    /// - [`Continuation::Restart`] to request a fresh worker via
    ///   [`create_and_register_os_sources()`]
    ///
    /// [`RRTEvent::Shutdown`]: RRTEvent::Shutdown
    /// [`RRTEvent::Worker(...)`]: RRTEvent::Worker
    /// [`create_and_register_os_sources()`]: Self::create_and_register_os_sources
    /// [`mio_poller::MioPollWorker`]: crate::direct_to_ansi::input::mio_poller::MioPollWorker
    /// [`sender.receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [framework]: super#the-rrt-contract-and-benefits
    /// [trait docs]: Self
    fn block_until_ready_then_dispatch(
        &mut self,
        sender: &Sender<RRTEvent<Self::Event>>,
    ) -> Continuation;
}

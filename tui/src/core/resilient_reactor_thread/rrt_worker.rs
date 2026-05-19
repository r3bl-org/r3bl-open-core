// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH epoll kqueue threadwaker rrtwaker rrtsoftwareinterrupt
// cspell:words IORING

//! Core traits for adding your business logic, using [dependency injection], into the
//! reusable Resilient Reactor Thread ([`RRT`]) [framework]. See the following for more
//! details: [`RRTWorker`], [`RRTSoftwareInterrupt`].
//!
//! [`RRT`]: super::RRT
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [dependency injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [framework]: super#the-rrt-contract-and-benefits

use super::{RRTEvent, RestartPolicy};
use crate::core::common::Continuation;
use std::fmt::Debug;
use tokio::sync::broadcast::Sender;

/// A trait for interrupting the blocked [framework]-managed dedicated RRT thread.
///
/// Your implementation of [`RRTSoftwareInterrupt`] wraps whatever mechanism your
/// [blocking I/O backend] provides for interrupt signaling. For example,
/// [`MioSoftwareInterrupt`] wraps a [`mio::Waker`] that triggers an [`epoll`]/[`kqueue`]
/// wakeup.
///
/// When a [`SubscriberGuard`] is dropped, its [`Drop`] impl calls
/// [`ThreadLifecycleMonitor::interrupt_if_running()`], which acquires the [`lock()`] lock
/// and (if the variant is [`Running`]) reaches into the wrapped [`InterruptHandle`] and
/// calls [`trigger_software_interrupt()`] - signaling the dedicated thread to check
/// whether it should exit.
///
/// # Trait Bounds - [`Send`] + [`Sync`] + `'static` + [`Debug`]
///
/// There is exactly **one interrupt handle** at any time, wrapped in a
/// [`InterruptHandle`] held by the [`Running`] variant of [`ThreadState`] inside
/// [`RRT`]'s shared [`ThreadLifecycleMonitor`]. Every [`SubscriberGuard`] holds an
/// [`Arc`] clone of the same monitor. When any subscriber drops its guard, the chain runs
/// through the monitor:
///
/// ```text
/// ┌─────────────────────┐
/// │  Dedicated Thread   │ ◄─── trigger_software_interrupt()
/// │  (blocking on Poll) │      interrupts blocking call
/// └─────────────────────┘
///            ▲
///            │
///    ┌──────────────────┐
///    │       ONE        │ ◄─┬─── Async Task A drops guard
///    │ InterruptHandle  │   │    └──► monitor.interrupt_if_running()
///    │   (in Running)   │ ◄─┴─── Async Task B drops guard
///    └──────────────────┘        └──► monitor.interrupt_if_running()
/// ```
///
/// The shared-monitor access pattern requires [`Send`] + [`Sync`]:
///
/// - **[`Send`] + [`Sync`]**: An instance of `Self` lives inside the [`Running`] variant
///   of [`ThreadState`], owned by [`ThreadLifecycleMonitor`], and shared via [`Arc`] to
///   every [`SubscriberGuard`] across [`tokio`] [runtime threads]. For the [`Arc`] to
///   cross thread boundaries, every type it transitively contains - including `Self`
///   (wrapped in an [`InterruptHandle`]) - must be [`Send`] + [`Sync`].
/// - **[`'static`]**: Required for thread spawning. See [`RRT`] for a detailed
///   explanation of the [`'static` trait bound][`'static`].
/// - **[`Debug`]**: Required for framework observability and logging.
///
/// # Idempotency
///
/// Multiple concurrent calls are safe and harmless. Software interrupts may coalesce (the
/// dedicated thread wakes once) or cause multiple wakeups (it loops again). Either way,
/// the dedicated thread just checks [`receiver_count()`] and decides whether to exit.
///
/// # Poll -> Registry -> Software Interrupt Chain
///
/// Your [`RRTSoftwareInterrupt`] implementation is tightly coupled to its [blocking
/// mechanism] (e.g., [`Poll`]):
///
/// ```text
/// Poll::new()                            // Creates OS event mechanism
///       │                                // (epoll fd / kqueue)
///       ▼
/// poll.registry()                        // Handle to register interest
///       │
///       ▼
/// SoftwareInterrupt::new(registry)       // Registers with THIS Poll's mechanism
///       │
///       ▼
/// interrupt                              // Triggers event → poll.poll() returns
/// .trigger_software_interrupt()
/// ```
///
/// Since a [`SoftwareInterrupt`] is bound to the [`Poll`] instance it was created
/// from, replacing one without the other leaves a dead reference. This is why the worker
/// and its interrupt handle are always created together as a pair (see [two-phase
/// setup]).
///
/// # Why User-Provided?
///
/// Interrupt strategies are backend-specific. See [Why is `RRTSoftwareInterrupt`
/// User-Provided?]
///
/// [`'static`]: super::RRT#static-trait-bound-vs-static-lifetime-annotation
/// [`Arc`]: std::sync::Arc
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`InterruptHandle`]: super::InterruptHandle
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`lock()`]: super::ThreadLifecycleMonitor::lock
/// [`mio::Waker`]: mio::Waker
/// [`MioSoftwareInterrupt`]: crate::mio_poller::MioSoftwareInterrupt
/// [`Poll`]: mio::Poll
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`RRT`]: super::RRT
/// [`Running`]: super::ThreadState::Running
/// [`SoftwareInterrupt`]: crate::mio_poller::MioSoftwareInterrupt
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`ThreadLifecycleMonitor::interrupt_if_running()`]:
///     super::ThreadLifecycleMonitor::interrupt_if_running
/// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
/// [`ThreadState`]: super::ThreadState
/// [`tokio`]: tokio
/// [`trigger_software_interrupt()`]: Self::trigger_software_interrupt
/// [blocking I/O backend]: super#understanding-blocking-io
/// [blocking mechanism]: super#understanding-blocking-io
/// [framework]: crate::core::resilient_reactor_thread#the-rrt-contract-and-benefits
/// [runtime threads]: tokio::runtime
/// [two-phase setup]: super#two-phase-setup
/// [Why is `RRTSoftwareInterrupt` User-Provided?]:
///     super#why-is-rrtsoftwareinterrupt-user-provided
pub trait RRTSoftwareInterrupt: Send + Sync + Debug + 'static {
    /// Triggers the OS event mechanism registered during
    /// [`create_and_register_os_sources()`], unblocking the dedicated RRT thread's
    /// [`block_until_ready_then_dispatch()`] call.
    ///
    /// This method is called via [`ThreadLifecycleMonitor::interrupt_if_running()`] when
    /// a [`SubscriberGuard`] is dropped, to interrupt the dedicated thread's blocking
    /// [`syscall`] (e.g., [`Poll::poll()`]). The thread then checks
    /// [`receiver_count()`] and decides whether to exit.
    ///
    /// Implementations should be idempotent - multiple concurrent calls must be safe.
    ///
    /// # Design Rationale: Why Not `Waker` Directly?
    ///
    /// Different I/O backends need different interrupt strategies:
    /// - [`Poll`] uses [`Waker`] (which typically uses [`eventfd`] or a self-pipe).
    /// - A TCP [`accept()`] loop might need a "connect-to-self" pattern.
    /// - [`io_uring`] might use [`IORING_OP_MSG_RING`].
    ///
    /// Since the [`Interrupt`] is bound to the specific OS resources it was created from
    /// (e.g., a [`Waker`] is bound to the [`Poll`] registry it was registered
    /// with), the framework doesn't create it. Instead, your [`RRTWorker`]
    /// implementation creates the appropriate [`Interrupt`] during its two-phase
    /// setup.
    ///
    /// [`accept()`]: std::net::TcpListener::accept
    /// [`block_until_ready_then_dispatch()`]: RRTWorker::block_until_ready_then_dispatch
    /// [`create_and_register_os_sources()`]: RRTWorker::create_and_register_os_sources
    /// [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
    /// [`Interrupt`]: Self
    /// [`io_uring`]: https://kernel.dk/io_uring.pdf
    /// [`IORING_OP_MSG_RING`]:
    ///     https://man7.org/linux/man-pages/man3/io_uring_prep_msg_ring.3.html
    /// [`Poll::poll()`]: mio::Poll::poll
    /// [`Poll`]: mio::Poll
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [`ThreadLifecycleMonitor::interrupt_if_running()`]:
    ///     super::ThreadLifecycleMonitor::interrupt_if_running
    /// [`Waker`]: mio::Waker
    fn trigger_software_interrupt(&self);
}

/// A trait for implementing the blocking I/O worker on the [framework]-managed dedicated
/// RRT thread.
///
/// This is the main "entry point" for you to use the RRT [framework]. The journey begins
/// with you defining a static singleton of type [`RRT`] in your code and providing a
/// concrete type that implements this trait. See the [DI overview] for what each piece
/// ([`RRTWorker`], [`RRTSoftwareInterrupt`], [`Event`]) provides and how the [framework]
/// orchestrates them.
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
/// Returning [`Continuation::Restart`] triggers [self-healing restart] - the [framework]
/// drops the current worker, applies the [`RestartPolicy`], and creates a fresh worker
/// via [`create_and_register_os_sources()`].
///
/// # Trait Bounds - [`Send`] + `'static`
///
/// An instance of the implementing type moves to the [framework]-managed dedicated RRT
/// thread and is owned exclusively by it. This is why
/// we need the following trait bounds:
/// - ✓ [`Send`]: The implementing type must be [`Send`] to move from the [async executor
///   thread] (on which [`try_subscribe()`] runs) to the [framework]-managed dedicated RRT
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
/// - **[Framework] control**: Inject logging, metrics between iterations
/// - **Single responsibility**: Your [`RRTWorker`] implementation handles events,
///   [framework] handles lifecycle
/// - **Testability**: Unit test [`block_until_ready_then_dispatch()`] in isolation
///
/// [`'static`]: super::RRT#static-trait-bound-vs-static-lifetime-annotation
/// [`block_until_ready_then_dispatch()`]: Self::block_until_ready_then_dispatch
/// [`create_and_register_os_sources()`]: Self::create_and_register_os_sources
/// [`event`]: Self::Event
/// [`MioPollWorker`]: crate::mio_poller::MioPollWorker
/// [`restart_policy()`]: Self::restart_policy
/// [`RRT`]: super::RRT
/// [`signals`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`stdin`]: std::io::stdin
/// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`try_subscribe()`]: super::RRT::try_subscribe
/// [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
/// [DI overview]: super#separation-of-concerns-and-dependency-injection-di
/// [Event]: Self::Event
/// [Framework]: crate::core::resilient_reactor_thread#the-rrt-contract-and-benefits
/// [framework]: crate::core::resilient_reactor_thread#the-rrt-contract-and-benefits
/// [module docs]: super::RRT#two-phase-setup
/// [self-healing restart]: super#self-healing-restart-details
/// [two-phase setup]: super::RRT#two-phase-setup
pub trait RRTWorker: Send + Debug + 'static {
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
    /// [`broadcast channel`]: tokio::sync::broadcast
    /// [`Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
    /// [`recv()`]: tokio::sync::broadcast::Receiver::recv
    const CHANNEL_CAPACITY: usize = 4_096;

    /// The type containing domain-specific data to broadcast from your implementation to
    /// async consumers.
    ///
    /// This type must be [`Clone`] + [`Send`] + `'static` to satisfy the requirements of
    /// - ✓ [`Clone`]: The [`broadcast channel`] clones each event for every [`Receiver`]
    ///   resulting in one clone per [`SubscriberGuard`].
    /// - ✓ [`Send`]: Events are produced on the framework-managed dedicated RRT thread
    ///   and consumed by async consumers / tasks running on [`tokio`] [executor threads]
    ///   (in the [multithreaded runtime]).
    /// - ✓ [`'static`]: Required for thread spawning. See [`RRT`] for a detailed
    ///   explanation of the [`'static` trait bound][`'static`].
    ///
    /// [`'static`]: super::RRT#static-trait-bound-vs-static-lifetime-annotation
    /// [`broadcast channel`]: tokio::sync::broadcast
    /// [`Receiver`]: tokio::sync::broadcast::Receiver
    /// [`RRT`]: super::RRT
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`tokio`]: tokio
    /// [executor threads]: tokio::runtime
    /// [multithreaded runtime]: tokio::runtime::Builder::new_multi_thread
    type Event: Clone + Send + Sync + 'static;

    /// The concrete interrupt handle type returned by
    /// [`create_and_register_os_sources()`] alongside the worker.
    ///
    /// The framework wraps this returned interrupt handle in an [`InterruptHandle`]
    /// inside the [`Running`] state. This associated type threads your concrete interrupt
    /// handle through the framework types ([`InterruptHandle`],
    /// [`ThreadLifecycleMonitor`], [`SubscriberGuard`], [`TerminationGuard`]) at the type
    /// level, eliminating dynamic dispatch.
    ///
    /// [`create_and_register_os_sources()`]: Self::create_and_register_os_sources
    /// [`InterruptHandle`]: super::InterruptHandle
    /// [`Running`]: super::ThreadState::Running
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`TerminationGuard`]: super::TerminationGuard
    /// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
    type Interrupt: RRTSoftwareInterrupt;

    /// Creates OS resources, registers event sources, and returns a coupled worker +
    /// [interrupt handle] pair.
    ///
    /// The specifics of which [`syscalls`] your implementation uses, and what sources are
    /// registered, are totally left up to your implementation of this method. Your
    /// concrete type (that implements this method) is an injected dependency containing
    /// business logic that the [framework] is not aware of by design.
    ///
    /// The [interrupt handle] is tightly coupled to the worker's blocking mechanism
    /// (e.g., [`Poll`]). Since a [`SoftwareInterrupt`] is bound to the [`Poll`]
    /// instance it was created from, this worker and [interrupt handle] must be
    /// created together. This is why this method returns both as a pair. See
    /// [`RRT`]'s [Two-Phase Setup] section for the full explanation of why the pair
    /// is returned and where each piece goes (interrupt handle wrapped in an
    /// [`InterruptHandle`] held by the [`Running`] variant of [`ThreadState`] inside
    /// [`ThreadLifecycleMonitor`], worker to thread-local stack in
    /// [`run_worker_loop()`]).
    ///
    /// The concrete interrupt handle type [`Self::Interrupt`] is threaded through
    /// [framework] types at the type level, eliminating dynamic dispatch.
    ///
    /// This method does not spawn the dedicated thread - that happens when your app calls
    /// [`try_subscribe()`]. See [Thread Lifecycle] for the full spawn/reuse/terminate
    /// sequence.
    ///
    /// This method is also called during [self-healing restart] to create fresh OS
    /// resources after the current worker is dropped.
    ///
    /// # Returns
    ///
    /// This worker and its [interrupt handle] pair. See [Two-Phase Setup] for how these
    /// are distributed between the spawned thread and the shared
    /// [`ThreadLifecycleMonitor`].
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created.
    ///
    /// [`InterruptHandle`]: super::InterruptHandle
    /// [`mio::Poll`]: mio::Poll
    /// [`Poll`]: mio::Poll
    /// [`RRT`]: super::RRT
    /// [`run_worker_loop()`]: super::run_worker_loop
    /// [`Running`]: super::ThreadState::Running
    /// [`Self::Interrupt`]: Self::Interrupt
    /// [`SoftwareInterrupt`]: crate::mio_poller::MioSoftwareInterrupt
    /// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
    /// [`ThreadState`]: super::ThreadState
    /// [`try_subscribe()`]: super::RRT::try_subscribe
    /// [framework]: crate::core::resilient_reactor_thread#the-rrt-contract-and-benefits
    /// [interrupt handle]: super::RRTSoftwareInterrupt
    /// [self-healing restart]: super#self-healing-restart-details
    /// [Thread Lifecycle]: super::RRT#thread-lifecycle
    /// [Two-Phase Setup]: super::RRT#two-phase-setup
    fn create_and_register_os_sources() -> miette::Result<(Self, Self::Interrupt)>
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
    #[must_use]
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
    /// sending them through `sender`. The [framework] uses `sender` to send
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
    /// [`create_and_register_os_sources()`]: Self::create_and_register_os_sources
    /// [`mio_poller::MioPollWorker`]: crate::direct_to_ansi::input::mio_poller::MioPollWorker
    /// [`RRTEvent::Shutdown`]: RRTEvent::Shutdown
    /// [`RRTEvent::Worker(...)`]: RRTEvent::Worker
    /// [`sender.receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [framework]: crate::core::resilient_reactor_thread#the-rrt-contract-and-benefits
    /// [trait docs]: Self
    fn block_until_ready_then_dispatch(
        &mut self,
        sender: &Sender<RRTEvent<Self::Event>>,
    ) -> Continuation;
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread lifecycle state definitions for the Resilient Reactor Thread (RRT) pattern. See
//! [`ThreadState`] for details.

use super::{InterruptHandle, RRTWorker, StopReason};

/// Represents the definitive lifecycle state of the dedicated thread (typically used for
/// blocking I/O).
///
/// # Historical context
///
/// Previous designs had problems with the accurate computation of "is this thread alive"
/// condition. This resulted in many race conditions & timing bugs which manifested in
/// these transition phases:
/// 1. Going from **stopped** to **starting** to **running**.
/// 2. Going from **running** to **stopping** to **stopped**.
///
/// See the [`Historical Context: Race Conditions Eliminated`] for details.
///
/// # Making Illegal States Unrepresentable
///
/// This architecture combines two powerful Rust patterns to eliminate race conditions:
/// 1. State Machine: We define discrete, mutually exclusive phases ([`Starting`],
///    [`Running`], [`Stopping`], etc.) and explicit rules for transitioning between them.
/// 2. [Typestate Pattern] (via Algebraic Data Types): We embed the critical data payload
///    (the [software interrupt handle]) directly inside the variant
///    (`Running(InterruptHandle<W::Interrupt>)`).
///
/// This combination ensures that the [software interrupt handle] is only accessible when
/// the thread is definitively alive and ready to accept subscribers (async consumers, in
/// the form of [`TUI`] and [`readline_async`] apps).
///
/// It structurally prevents access to the [software interrupt handle] during
/// unpredictable teardown or allocation operations. Because the [software interrupt
/// handle] only exists in the type signature of the [`Running`] variant of the
/// [`ThreadState`] enum, the compiler prevents you from accessing it when the thread is
/// [`Stopping`] or [`Stopped`].
/// - You literally cannot extract it without pattern matching on
///   [`Running(interrupt_handle)`].
/// - If the state is [`Stopping`] or [`Stopped`], the memory holding the [software
///   interrupt handle] is literally gone.
///
/// # Locking Contract
///
/// 1. This enum is never accessed directly; it is wrapped inside a [`Mutex`] in the
///    [`state`] field of [`ThreadLifecycleMonitor`] struct. You access that field to get
///    to this enum.
/// 2. Threads (both the dedicated background thread and any [`tokio`] subscriber threads)
///    only hold a lock to this [`Mutex`] for very short periods of time to transition the
///    state or check the current state.
/// 3. If a thread needs to perform a blocking operation (like spawning an OS thread,
///    allocating OS resources, or waiting for another thread to die), it **must** release
///    this lock first—either manually (by dropping the guard) or automatically via
///    [`Condvar`].
///
/// [`Condvar`]: std::sync::Condvar
/// [`Condvar`]: super::ThreadLifecycleMonitor::wait()
/// [`Historical Context: Race Conditions Eliminated`]:
///     super#historical-context-race-conditions-eliminated
/// [`Mutex`]: std::sync::Mutex
/// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
/// [`Running(interrupt_handle)`]: ThreadState::Running
/// [`Running`]: ThreadState::Running
/// [`Starting`]: ThreadState::Starting
/// [`state`]: super::ThreadLifecycleMonitor::lock()
/// [`Stopped`]: ThreadState::Stopped
/// [`Stopping`]: ThreadState::Stopping
/// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
/// [`tokio`]: tokio
/// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
/// [software interrupt handle]: crate::RRTSoftwareInterrupt
/// [Typestate Pattern]:
///     https://docs.rust-embedded.org/book/static-guarantees/typestate-programming.html
#[derive(Debug)]
pub enum ThreadState<W: RRTWorker> {
    /// # Current State
    ///
    /// The dedicated thread is not running, and no OS resources are currently allocated.
    /// This is the initial state, and the state returned to after this thread exits.
    ///
    /// # Transition
    ///
    /// This state will not change until we have new arrivals below to trigger the state
    /// to change. A subscriber arriving in this state will acquire the [lock for the
    /// `state`], transition the state to `Starting`, and then **immediately release
    /// the lock** before performing the heavy OS allocation and thread spawn.
    ///
    /// # New Arrivals
    ///
    /// A new subscriber (async consumers, in the form of [`TUI`] and [`readline_async`]
    /// apps calling [`try_subscribe()`]) arriving in this state will trigger OS
    /// allocation and spawn a new dedicated thread, and this state will transition to
    /// [`Running`].
    ///
    /// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
    /// [`Running`]: ThreadState::Running
    /// [`try_subscribe()`]: crate::RRT::try_subscribe
    /// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
    /// [lock for the `state`]: super::ThreadLifecycleMonitor::lock()
    Stopped,

    /// # Current State
    ///
    /// OS resources are currently being allocated (e.g., [`epoll`] file descriptors)
    /// and the thread is in the process of being spawned.
    ///
    /// # Transition
    ///
    /// When the resource allocation is complete and the dedicated thread has been
    /// spawned, the spawning thread will re-acquire the [lock for the `state`] one
    /// last time to transition it to [`Running`] and notify any blocked subscribers
    /// via the [`Condvar`].
    ///
    /// # New Arrivals
    ///
    /// Other subscribers (async consumers, in the form of [`TUI`] and [`readline_async`]
    /// apps calling [`try_subscribe()`]) arriving in this state must wait on the
    /// [`Condvar`] until the state becomes [`Running`] or [`Stopped`]. So they are going
    /// to be temporarily blocked during this transition periods.
    ///
    /// [`Condvar`]: super::ThreadLifecycleMonitor::wait()
    /// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
    /// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
    /// [`Running`]: ThreadState::Running
    /// [`Stopped`]: ThreadState::Stopped
    /// [`try_subscribe()`]: crate::RRT::try_subscribe
    /// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
    /// [lock for the `state`]: super::ThreadLifecycleMonitor::lock()
    Starting,

    /// # Current State
    ///
    /// The thread is fully alive, initialized, and polling for blocking I/O using
    /// OS [`syscalls`] and [`file descriptors`].
    ///
    /// # Transition
    ///
    /// In [`run_worker_loop()`], the dedicated thread briefly acquires the
    /// [lock for the `state`] at the top of its loop to check [`receiver_count`].
    ///
    /// - If it is zero (or if the [worker] explicitly returns [`Continuation::Stop`] on
    ///   [`EOF`]), it transitions the state to [`Stopping`] and immediately releases the
    ///   lock before beginning its teardown.
    /// - If the [worker] returns [`Continuation::Restart`], it transitions to
    ///   [`Restarting`] and immediately releases the lock to begin dropping and
    ///   recreating OS resources.
    /// - It also transitions to [`Stopped`] if the dedicated thread panics (this is
    ///   handled by the RAII [cleanup guard]).
    ///
    /// # New Arrivals
    ///
    /// The OS-level software interrupt handle is available to be used by new subscribers
    /// (async consumers, in the form of [`TUI`] and [`readline_async`] apps calling
    /// [`try_subscribe()`]). Access happens only through
    /// [`ThreadLifecycleMonitor::interrupt_if_running()`], which reads the current
    /// generation's interrupt handle under the state lock every time. The handle itself
    /// is wrapped in a non-clonable [`InterruptHandle`], so it cannot be captured by a
    /// subscriber and reused against a later generation.
    ///
    /// # Departures
    ///
    /// When a subscriber drops its [`SubscriberGuard`] (because the [`TUI`] or
    /// [`readline_async`] app exits), the guard uses the current interrupt handle (via
    /// [`interrupt_if_running()`]) to interrupt the dedicated thread's event loop (which
    /// may be blocked on a [`syscall`] like [`epoll`] or [`kqueue`] waiting for
    /// multiplexed OS [sources] to become ready), thus forcing it to wake up and
    /// evaluate its shutdown conditions (and possibly self terminate - which takes us to
    /// the [`Stopping`] state).
    ///
    /// [`Continuation::Restart`]: crate::Continuation::Restart
    /// [`Continuation::Stop`]: crate::Continuation::Stop
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
    /// [`file descriptors`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`interrupt_if_running()`]: super::ThreadLifecycleMonitor::interrupt_if_running
    /// [`InterruptHandle`]: super::InterruptHandle
    /// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
    /// [`readline_async`]: crate::readline_async::ReadlineAsyncContext
    /// [`receiver_count`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`Restarting`]: ThreadState::Restarting
    /// [`run_worker_loop()`]: crate::resilient_reactor_thread::run_worker_loop
    /// [`Stopped`]: ThreadState::Stopped
    /// [`Stopping`]: ThreadState::Stopping
    /// [`SubscriberGuard`]: crate::resilient_reactor_thread::SubscriberGuard
    /// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [`ThreadLifecycleMonitor::interrupt_if_running()`]:
    ///     super::ThreadLifecycleMonitor::interrupt_if_running
    /// [`try_subscribe()`]: crate::RRT::try_subscribe
    /// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
    /// [cleanup guard]: crate::resilient_reactor_thread::TerminationGuard
    /// [lock for the `state`]: super::ThreadLifecycleMonitor::lock()
    /// [software interrupt handle]: crate::RRTSoftwareInterrupt
    /// [sources]: mio::event::Source
    /// [worker]: crate::RRTWorker
    Running(InterruptHandle<W::Interrupt>),

    /// # Current State
    ///
    /// In [`run_worker_loop()`] the dedicated thread has evaluated its shutdown
    /// conditions and decided to shut down (either due to zero subscribers or a
    /// worker-initiated stop like [`EOF`]).
    ///
    /// The specific reason is captured in [`StopReason`].
    ///
    /// # Transition
    ///
    /// In [`run_worker_loop()`] once the dedicated thread drops its OS resources and
    /// executes its cleanup logic, it will drop the [`TerminationGuard`]. When this is
    /// dropped, the state will transition to [`Stopped`] by acquiring the [lock for the
    /// `state`].
    ///
    /// # New Arrivals
    ///
    /// New subscribers calling [`try_subscribe()`] cannot attach to this dying dedicated
    /// thread. This is what happens when they call this function:
    /// 1. The [`tokio`] thread calling the [`try_subscribe()`] function must acquire the
    ///    [lock for the `state`] for a very short period of time. It checks to see that
    ///    the state is [`Stopping`] and then releases the lock and waits on the
    ///    [`Condvar`] and blocks. It won't get unblocked until this dedicated thread
    ///    fully exits and the state transitions to [`Stopped`].
    /// 2. Because the blocked [`tokio`] thread released the [lock for the `state`], the
    ///    dying dedicated thread is able to acquire the lock one last time to change the
    ///    state to [`Stopped`]. This happens in [`run_worker_loop()`] when the
    ///    [`TerminationGuard`] is dropped, which releases this lock and calls
    ///    [`notify_all()`] on the [`Condvar`] potentially unblocking all blocked
    ///    [`tokio`] subscriber threads.
    /// 3. The blocked subscriber thread will wake up as a result of the [`notify_all()`]
    ///    call on the [`Condvar`], and reacquire the [lock for the `state`] (which was
    ///    released by the dedicated thread). Because [`try_subscribe()`] is implemented
    ///    as an outer `loop { match state { ... } }`, the subscriber will loop around,
    ///    hit the [`Stopped`] match arm, take ownership of the state, and immediately
    ///    transition it to [`Starting`] to spawn a fresh thread.
    ///
    /// [`Condvar`]: super::ThreadLifecycleMonitor::wait()
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`notify_all()`]: std::sync::Condvar::notify_all
    /// [`run_worker_loop()`]: crate::resilient_reactor_thread::run_worker_loop
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopped`]: ThreadState::Stopped
    /// [`Stopping`]: ThreadState::Stopping
    /// [`StopReason`]: super::StopReason
    /// [`TerminationGuard`]: crate::resilient_reactor_thread::TerminationGuard
    /// [`tokio`]: tokio
    /// [`try_subscribe()`]: crate::RRT::try_subscribe
    /// [lock for the `state`]: super::ThreadLifecycleMonitor::lock()
    Stopping(StopReason),

    /// # Current State
    ///
    /// The dedicated thread is intentionally recycling its OS resources (e.g., to clear
    /// corrupted state after a [`Continuation::Restart`], triggering a
    /// [self-healing restart]).
    ///
    /// # Transition
    ///
    /// In [`run_worker_loop()`], the old OS resources are currently being dropped,
    /// and new ones are being allocated via [`create_and_register_os_sources()`].
    /// 1. If successful, the dedicated thread re-acquires the [lock for the `state`] one
    ///    last time to transition the state back to [`Running`] and notify any blocked
    ///    subscribers via the [`Condvar`].
    /// 2. If it exhausts its restart budget or fails allocation, it acquires the lock to
    ///    transition to [`Stopped`] and notifies any blocked subscribers.
    ///
    /// # New Arrivals
    ///
    /// The [`tokio`] thread calling [`try_subscribe()`] must wait (block) on the
    /// [`Condvar`] until the dedicated thread changes state to [`Running`] or fails
    /// and changes state to [`Stopped`].
    /// 1. If state transitions to [`Running`], the async consumer/subscriber subscribes
    ///    to the existing thread.
    /// 2. If state transitions to [`Stopped`], the async consumer/subscriber wakes up,
    ///    takes ownership, and spawn a fresh thread themselves. Because
    ///    [`try_subscribe()`] is implemented as an outer `loop { match state { ... } }`,
    ///    the subscriber will loop around, hit the [`Stopped`] match arm, take ownership
    ///    of the state, and immediately transition it to [`Starting`] to spawn a fresh
    ///    dedicated thread.
    ///
    /// [`Condvar`]: super::ThreadLifecycleMonitor::wait()
    /// [`Continuation::Restart`]: crate::Continuation::Restart
    /// [`create_and_register_os_sources()`]:
    ///     crate::RRTWorker::create_and_register_os_sources
    /// [`run_worker_loop()`]: crate::resilient_reactor_thread::run_worker_loop
    /// [`Running`]: ThreadState::Running
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopped`]: ThreadState::Stopped
    /// [`tokio`]: tokio
    /// [`try_subscribe()`]: crate::RRT::try_subscribe
    /// [lock for the `state`]: super::ThreadLifecycleMonitor::lock()
    /// [self-healing restart]: super#self-healing-restart-details
    Restarting,
}

impl<W: RRTWorker> ThreadState<W> {
    /// Returns `true` if the state is considered stable (not in a transient transition
    /// phase).
    ///
    /// Stable states:
    /// - [`Running`]: The thread is fully alive and polling for I/O.
    /// - [`Stopped`]: No thread exists; ready for a fresh spawn.
    ///
    /// Transient states ([`Starting`], [`Stopping`], [`Restarting`]) return `false`.
    ///
    /// [`Restarting`]: ThreadState::Restarting
    /// [`Running`]: ThreadState::Running
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopped`]: ThreadState::Stopped
    /// [`Stopping`]: ThreadState::Stopping
    #[must_use]
    pub const fn is_stable(&self) -> bool {
        matches!(self, ThreadState::Running(_) | ThreadState::Stopped)
    }

    /// Returns `true` if the state is in a transient transition phase.
    ///
    /// Transient states ([`Starting`], [`Stopping`], [`Restarting`]) indicate that the
    /// framework is currently mutating OS resources or spawning/joining threads.
    ///
    /// [`Restarting`]: ThreadState::Restarting
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopping`]: ThreadState::Stopping
    #[must_use]
    pub const fn is_transient(&self) -> bool { !self.is_stable() }
}

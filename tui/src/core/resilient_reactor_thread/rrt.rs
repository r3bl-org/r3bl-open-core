// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words maxfiles taskthreads rrtwaker

//! Thread lifecycle manager and entry point for the Resilient Reactor Thread (RRT)
//! pattern. See [`RRT`] for details.

use super::{BroadcastSender, InterruptHandle, RRTWorker, SubscribeError,
            SubscriberGuard, ThreadLifecycleMonitor, ThreadState, run_worker_loop};
use crate::{DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD, core::common::AtomicU8Ext};
use std::sync::{Arc, LazyLock};
use tokio::sync::broadcast;

/// Tries to subscribe to a thread. This is a shared implementation used by both
/// [`RRT::try_subscribe()`] and [`SubscriberGuard::try_subscribe()`].
///
/// # Errors
///
/// Returns [`SubscribeError`] if the operation fails:
/// - [`MutexPoisoned`] if the lifecycle lock or condvar wait is poisoned.
/// - [`WorkerCreation`] if creating the worker/interrupt pair fails.
/// - [`ThreadSpawn`] if spawning the dedicated thread fails.
///
/// [`Condvar`]: std::sync::Condvar
/// [`Mutex`]: std::sync::Mutex
/// [`MutexPoisoned`]: SubscribeError::MutexPoisoned
/// [`RRT::try_subscribe()`]: crate::resilient_reactor_thread::RRT::try_subscribe
/// [`SubscriberGuard::try_subscribe()`]:
///     crate::resilient_reactor_thread::SubscriberGuard::try_subscribe
/// [`ThreadSpawn`]: SubscribeError::ThreadSpawn
/// [`WorkerCreation`]: SubscribeError::WorkerCreation
pub fn try_subscribe<W: RRTWorker>(
    sender: &BroadcastSender<W::Event>,
    shared_state: &Arc<ThreadLifecycleMonitor<W>>,
) -> Result<SubscriberGuard<W>, SubscribeError> {
    let state_guard = shared_state.block_until_stable_state_reached();

    let (strategy, mut state_guard) =
        shared_state.read_state(state_guard, |state| state.into());

    match strategy {
        SubscriptionStrategy::FastPath => {
            // sender.subscribe() atomically increments receiver_count. The tokio thread
            // calling this method does so while holding the state lock to serialize its
            // attachment with the dedicated thread's zero-receiver exit check (in
            // run_worker_loop). Since the dedicated thread acquires this same lock before
            // reading the count, it will either see the incremented count or be blocked
            // waiting for the tokio thread to release the lock, preventing the dedicated
            // thread from exiting just as the new subscriber joins.
            let receiver = sender.subscribe();
            drop(state_guard);

            Ok(SubscriberGuard::new(
                sender.clone(),
                receiver,
                Arc::clone(shared_state),
            ))
        }
        SubscriptionStrategy::SlowPath => {
            // Transition Stopped → Starting.
            state_guard = shared_state.set_state(state_guard, ThreadState::Starting);
            drop(state_guard);

            // Allocate OS resources (outside the lock).
            let (worker, interrupt) = match W::create_and_register_os_sources() {
                Ok(pair) => pair,
                Err(err) => {
                    DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                        tracing::error!(
                            message = "RRT: Failed to allocate OS resources.",
                            error = ?err
                        );
                    });
                    let state_guard = shared_state.lock();
                    drop(shared_state.set_state(state_guard, ThreadState::Stopped));
                    shared_state.notify_all();
                    return Err(SubscribeError::WorkerCreation(err));
                }
            };

            // Re-acquire lock to perform atomic Running transition and spawn.
            let mut state_guard = shared_state.lock();

            // Under lock: subscribe, increment generation, transition, and spawn.
            let receiver = sender.subscribe();

            // Increment generation.
            let thread_generation = shared_state.thread_generation.increment();

            state_guard = shared_state.set_state(
                state_guard,
                ThreadState::Running(InterruptHandle::new(interrupt)),
            );

            let thread_move_sender = sender.clone();
            let thread_move_shared_state = Arc::clone(shared_state);

            DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                tracing::info!(
                    message = "RRT: Spawning new dedicated thread.",
                    generation = thread_generation
                );
            });

            let res_join_handle = std::thread::Builder::new()
                .name(format!("rrt-worker-gen-{thread_generation}"))
                .spawn(move || {
                    run_worker_loop::<W>(
                        worker,
                        thread_move_sender,
                        thread_move_shared_state,
                    );
                });

            match res_join_handle {
                Ok(_) => {
                    shared_state.notify_all();
                    drop(state_guard);
                    Ok(SubscriberGuard::new(
                        sender.clone(),
                        receiver,
                        Arc::clone(shared_state),
                    ))
                }
                Err(err) => {
                    DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                        tracing::error!(
                            message = "RRT: Failed to spawn dedicated thread.",
                            error = ?err
                        );
                    });

                    // Revert to Stopped (we already hold the lock).
                    state_guard =
                        shared_state.set_state(state_guard, ThreadState::Stopped);
                    shared_state.notify_all();
                    drop(state_guard);

                    // Explicitly drop the receiver (allocated under lock but no
                    // thread exists).
                    drop(receiver);

                    Err(SubscribeError::ThreadSpawn(err))
                }
            }
        }
    }
}

/// The entry point for the Resilient Reactor Thread (RRT) framework.
///
/// This struct manages the lifecycle of a single dedicated thread (at most one at a time)
/// with automatic spawn/shutdown/reuse semantics.
///
/// # Static vs. Flexible Usage
///
/// The [`RRT`] struct supports two primary usage patterns, depending on your needs.
/// Initialization always happens lazily on the first [`try_subscribe()`] call, enabled by
/// internal [`LazyLock`] fields.
///
/// 1. **Static Singleton**: Ideal for global OS resources (like `stdin` or `signals`).
///    Using a `static` declaration with [`RRT::new()`] ensures only one instance exists
///    for the entire process, preventing resource contention.
///
///    ```no_run
///    use r3bl_tui::{MioPollWorker, ok};
///    use r3bl_tui::core::resilient_reactor_thread::RRT;
///    // Global resources (static + const fn = singleton).
///    static SINGLETON: RRT<MioPollWorker> = RRT::new();
///
///    fn main() -> miette::Result<()> {
///        // Subscribe to get a guard that auto-manages the thread.
///        let guard = SINGLETON.try_subscribe()?;
///        ok!()
///    }
///    ```
///
/// 2. **Local Variable**: Ideal for when you need more control over the `RRT` instance's
///    scope (e.g., within a specific component or test). You can store it in a local
///    variable or another struct.
///
///    ```no_run
///    use r3bl_tui::{MioPollWorker, ok};
///    use r3bl_tui::core::resilient_reactor_thread::RRT;
///    fn main() -> miette::Result<()> {
///         // Local instance.
///         let rrt: RRT<MioPollWorker> = RRT::new();
///
///         // Subscribe to get a guard that auto-manages the thread.
///         let guard = rrt.try_subscribe()?;
///         ok!()
///    }
///    ```
///
/// > See the [`direct_to_ansi`] input singleton ([`global_input_resource::SINGLETON`])
/// > for the real usage in the terminal input system.
///
/// This struct has two top-level fields, each using a synchronization primitive that
/// matches its lifetime (see each field's documentation for more details):
///
/// - **[`sender`]**: Broadcast channel, lazily initialized, never replaced. Bridges async
///   consumers (subscribers) and the dedicated thread (worker). Outlives every thread
///   [generation] - subscribers always share the same channel.
/// - **[`shared_state`]**: Lazily initialized [`ThreadLifecycleMonitor`] - the explicit
///   state machine and synchronization primitive that governs the dedicated thread's
///   lifecycle. Manages a 5-variant [`ThreadState`] enum (`Stopped`, `Starting`,
///   `Running`, `Stopping`, `Restarting`) via an internal [`Monitor`]. The interrupt
///   handle lives only inside the [`Running`] variant - structurally inaccessible in any
///   other state. Also holds the [generation] counter ([`AtomicU8`]) that increments each
///   time a fresh thread is spawned.
///
/// See also:
///
/// - [architecture overview] - how these fields work together.
/// - [What Is the RRT Pattern?] - a rundown of the design.
/// - The [`direct_to_ansi`] input singleton ([`global_input_resource::SINGLETON`]) - the
///   real implementation used by the terminal input system.
///
/// # Thread Lifecycle
///
/// The dedicated worker thread's lifecycle progresses through the variants of
///
/// 1. **[`Stopped`]** - Initial state. [broadcast channel] is uninitialized ([`LazyLock`]
///    defers creation until first access). The [`RRTWorker`] and [`RRTSoftwareInterrupt`]
///    do not exist.
/// 2. **[`Starting`]** - First [`try_subscribe()`] call acquires the state lock, sees
///    [`Stopped`], and transitions to [`Starting`]. It then:
///    - Drops the lock and performs the heavy OS allocation (creating the
///      [`RRTWorker`]/[`RRTSoftwareInterrupt`] pair).
///    - Re-acquires the lock and performs the remaining setup atomically:
///      - Initializes the [broadcast channel] (via [`LazyLock`]).
///      - Updates the thread's [generation] identifier.
///      - Transitions state to [`Running`] (moving the [`RRTSoftwareInterrupt`] inside
///        the enum variant).
///      - **Spawns the dedicated thread** while still holding the lock.
///      - Calls [`notify_all()`] to wake any other subscribers that blocked while it was
///        [`Starting`].
///    - Releases the lock and returns a [`SubscriberGuard`]. This sequence ensures that
///      the "Running" state and the live thread are published atomically.
/// 3. **While [`Running`]** - inside [`run_worker_loop(worker, ...)`], the thread enters
///    a loop that calls [`block_until_ready_then_dispatch()`] repeatedly; this is a
///    blocking function. It unblocks when at least one of its I/O sources is ready (e.g.,
///    [`epoll`]/[`kqueue`] readiness, [`io_uring`] completion). Your
///    [`block_until_ready_then_dispatch()`] implementation (see
///    [`MioPollWorker::block_until_ready_then_dispatch_impl()`] for a concrete example)
///    processes the data from each ready source, broadcasts events to subscribers, and
///    finally returns a [`Continuation`] that directs the framework:
///    - [`Continuation::Continue`] - iteration handled; loop continues.
///    - [`Continuation::Stop`] - thread must exit (see step 4).
///    - [`Continuation::Restart`] - worker requested a self-healing restart. See
///      [Self-Healing Restart Sequence](#self-healing-restart-sequence).
/// 4. **[`ThreadState::Stopping`] & Thread Exits** - The dedicated thread can exit
///    through two distinct paths, both of which transition the state to [`Stopping`]
///    carrying a specific [`StopReason`]:
///    - **Framework-initiated Stop** ([`StopReason::ZeroReceivers`]): When all
///      subscribers have dropped, the framework sees [`receiver_count() == 0`] and
///      decides to stop.
///    - **Worker-initiated Stop** ([`StopReason::WorkerRequested`]): When the
///      [`RRTWorker`] encounters a domain stop condition (like an [`EOF`] on [`stdin`]),
///      it returns [`Continuation::Stop`].
///    - On exit, [`run_worker_loop(worker, ...)`] returns, and the [`RRTWorker`] goes out
///      of scope, triggering [`RAII`] cleanup on the OS resources it owns.
///    - [`TerminationGuard::drop()`] (a local [`RAII`] guard) runs, transitioning the
///      state to [`Stopped`] and calling [`notify_all()`] so new subscribers can spawn a
///      fresh thread.
/// 5. **Panic Exit** - if your [`block_until_ready_then_dispatch()`] implementation
///    panics, it does not take down the process. The framework catches it (via
///    [`catch_unwind`]), sends [`Shutdown(Panic)`] to subscribers, and exits the thread.
///    [`TerminationGuard::drop()`] still runs, transitioning state to [`Stopped`]. See
///    [Panic Handling](#panic-handling).
/// 6. **Next [`try_subscribe()`]** - A new subscriber sees [`Stopped`] and the cycle
///    repeats from step 2.
///
/// # Two-Phase Setup
///
/// Creating the dedicated [thread] has an ordering conflict:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────┐
/// │                      THE ORDERING CONFLICT                            │
/// ├───────────────────────────────────────────────────────────────────────┤
/// │                                                                       │
/// │  To interrupt the thread, a SubscriberGuard must trigger a software   │
/// │  interrupt.                                                           │
/// │  To create the software interrupt mechanism (the interrupt), we need  │
/// │  mio::Poll.                                                           │
/// │  But mio::Poll must MOVE to the spawned thread.                       │
/// │                                                                       │
/// │  ┌─────────────────────────────────────────────────────────────────┐  │
/// │  │  PROBLEM: After thread::spawn(), Poll is gone - too late to     │  │
/// │  │           create a software interrupt from its registry!        │  │
/// │  └─────────────────────────────────────────────────────────────────┘  │
/// │                                                                       │
/// │  Timeline without solution:                                           │
/// │                                                                       │
/// │    create Poll ──► spawn thread ──► Poll moves ──► x can't create     │
/// │                    (Poll gone!)                      interrupt anymore│
/// │                                                                       │
/// └───────────────────────────────────────────────────────────────────────┘
///
/// ┌───────────────────────────────────────────────────────────────────────┐
/// │                    THE SOLUTION: TWO-PHASE SETUP                      │
/// ├───────────────────────────────────────────────────────────────────────┤
/// │                                                                       │
/// │   Phase 1: create_and_register_os_sources() - resources only,         │
/// │            no thread spawned                                          │
/// │   ┌─────────────────────────────────────────────────────────────────┐ │
/// │   │  Creates BOTH from the same mio::Poll registry:                 │ │
/// │   │                                                                 │ │
/// │   │     mio::Poll ──registry──► mio::Waker                          │ │
/// │   │         │                       │                               │ │
/// │   │         ▼                       ▼                               │ │
/// │   │      Worker                  Interrupt                          │ │
/// │   │    (owns Poll)         (wraps mio::Waker)                       │ │
/// │   └─────────────────────────────────────────────────────────────────┘ │
/// │                    │                       │                          │
/// │                    ▼                       ▼                          │
/// │   Phase 2: Split and distribute                                       │
/// │   ┌────────────────────┐    ┌─────────────────────────────────────┐   │
/// │   │  Spawned Thread    │    │ RRT (shared ThreadLifecycleMonitor) │   │
/// │   │  ──────────────    │    │ ─────────────────────────────────── │   │
/// │   │  Worker moves here │    │ InterruptHandle stored in Running   │   │
/// │   │  (owns mio::Poll)  │    │ state inside the Arc<Mutex> monitor.│   │
/// │   │                    │◄───│ SubscriberGuard calls               │   │
/// │   │                    │    │ interrupt_if_running() on monitor.  │   │
/// │   └────────────────────┘    └─────────────────────────────────────┘   │
/// │                                                                       │
/// └───────────────────────────────────────────────────────────────────────┘
/// ```
///
/// The key insight: **unified creation, then separation**. Both resources are created
/// together from the same [`mio::Poll`] registry, then split:
///
/// - The **[`InterruptHandle`]** (wrapping your [`RRTSoftwareInterrupt`]) is stored in
///   the [`Running`] variant of [`ThreadState`] inside the [`shared_state`] monitor (an
///   [`Arc<ThreadLifecycleMonitor>`] shared with all [`SubscriberGuard`]s) - it lives as
///   long as the thread is in the [`Running`] state.
/// - The **[`RRTWorker`]** moves to the spawned [thread] as a local `mut` variable on the
///   stack inside [`run_worker_loop(worker, ...)`] - it lives only as long as that
///   thread.
///
/// This is why [`create_and_register_os_sources()`] returns both as a pair: they have
/// **different owners and different lifetimes**, but must be created together because the
/// interrupt handle is bound to the worker's blocking mechanism (e.g., [`mio::Poll`]'s
/// registry).
///
/// # `const` Expression vs `const` Declaration vs `static` Declaration
///
/// These are related but different concepts:
///
/// | Term                     | Meaning                                         | Example                                 |
/// | :----------------------- | :---------------------------------------------- | :-------------------------------------- |
/// | **`const` expression**   | Value the compiler can compute at compile time  | `1 + 2`, `Mutex::new(None)`             |
/// | **`const`** function     | Function callable in const context              | `const fn new() -> Option<T> { None }`  |
/// | **`const` declaration**  | Read-only, no address (value is inlined)        | `const G: Mutex<T> = /* const expr */`  |
/// | **`static` declaration** | Single instance, fixed addr, can be mutable     | `static G: Mutex<T> = /* const expr */` |
///
/// **Key point:** Both `static` and `const` declarations look nearly identical at the
/// **declaration site**, but they have opposite behaviors at **use sites** (where you
/// reference the variable):
///
/// ```
/// use std::sync::Mutex;
/// // ── Declaration sites (look almost the same) ──
/// static S: Mutex<Option<i32>> = Mutex::new(None);
/// const  C: Mutex<Option<i32>> = Mutex::new(None);
///
/// // ── Use sites (behavior diverges) ──
/// S.lock().unwrap().replace(42);            // mutates the single instance
/// assert_eq!(*S.lock().unwrap(), Some(42)); // ✅ same Mutex
///
/// C.lock().unwrap().replace(42);            // mutates a fresh copy
/// assert_eq!(*C.lock().unwrap(), None);     // ❌ different Mutex!
/// ```
///
/// `const` inlines a fresh copy at every use site (like a macro expansion), so mutations
/// are silently lost. For a singleton, always use `static`.
///
/// # `'static` Trait Bound vs `'static` Lifetime Annotation
///
/// These are different concepts that share the `'static` keyword:
///
/// | Context                 | Syntax Example | Meaning                                            |
/// | :---------------------- | :------------- | :------------------------------------------------- |
/// | **Lifetime annotation** | `&'static str` | Reference valid for entire gram (data in binary)   |
/// | **Trait bound**         | `T: 'static`   | Type contains no references shorter than `'static` |
///
/// `T: 'static` does NOT mean "contains no references". It means "can be held
/// indefinitely without becoming invalid". A type satisfying this bound:
/// - **CAN** contain `'static` references (e.g., `&'static str`)
/// - **CANNOT** contain references with shorter lifetimes (e.g., `&'a str`)
///
/// [`String`] satisfies `T: 'static` even though it can be dropped at any time - the
/// bound means "won't dangle", not "lives forever".
///
/// Here's what the `T: 'static` trait bound looks like in real code:
///
/// ```no_run
/// use std::thread::spawn;
/// fn spawn_thread<T: Send + 'static>(it: T) { spawn(move || drop(it)); }
/// spawn_thread(String::from("owned"));    // ✅ String: 'static
/// spawn_thread("literal");                // ✅ &'static str: 'static
/// ```
///
/// This fails to compile - `&String` has a non-`'static` lifetime:
///
/// ```compile_fail
/// use std::thread::spawn;
/// fn spawn_thread<T: Send + 'static>(it: T) { spawn(move || drop(it)); }
/// let local = String::from("local");
/// spawn_thread(&local);                   // ❌ &String is not 'static
/// ```
///
/// Here's a quick reference for which types satisfy `T: 'static`:
///
/// | Type                      | `T: 'static`? | Why?                                  |
/// | :------------------------ | :------------ | :------------------------------------ |
/// | [`String`]                | ✅ Yes        | Owned data, no references             |
/// | [`Vec<u8>`]               | ✅ Yes        | Owned data, no references             |
/// | `&'static str`            | ✅ Yes        | Reference with `'static` lifetime     |
/// | `Foo { s: &'static str }` | ✅ Yes        | Struct with only `'static` references |
/// | `&'a str`                 | ❌ No         | Reference with non-`'static` lifetime |
/// | `Foo<'a> { s: &'a str }`  | ❌ No         | Struct with non-`'static` references  |
///
/// For thread spawning, `T: 'static` is required because the spawned thread could outlive
/// the caller - any borrowed data with a shorter lifetime might become invalid. This is
/// why the traits [`RRTWorker`], [`RRTSoftwareInterrupt`], and the associated [`Event`]
/// type all require `'static`.
///
/// # Self-Healing Restart Sequence
///
/// When [`block_until_ready_then_dispatch()`] returns [`Continuation::Restart`], the
/// framework executes the following sequence:
///
/// 1. The framework acquires the [`shared_state`] lock, transitions from [`Running`] to
///    [`Restarting`] (consuming the old [`RRTSoftwareInterrupt`]), **and immediately
///    releases the lock to begin dropping and recreating OS resources**.
/// 2. The current [`RRTWorker`] is dropped and [`RAII`] cleanup releases OS resources.
/// 3. The framework sleeps for the configured delay (see [`RestartPolicy`]).
/// 4. [`RRTWorker::create_and_register_os_sources()`] is called to create a fresh
///    [`RRTWorker`] + [`RRTSoftwareInterrupt`] pair. The new [`RRTWorker`] allocates new
///    OS resources.
/// 5. The framework re-acquires the [`shared_state`] lock, transitions from
///    [`Restarting`] back to [`Running`] (moving the new [`RRTSoftwareInterrupt`] into
///    the state), and calls [`notify_all()`] to unblock any subscribers waiting during
///    the restart.
/// 6. The poll loop resumes with the fresh [`RRTWorker`]. The restart budget resets.
///
/// If [`RRTWorker::create_and_register_os_sources()`] itself fails, the framework retries
/// on the pre-existing thread until success or budget exhaustion. If exhausted, the
/// thread exits and transitions to [`Stopped`].
///
/// # Panic Handling
///
/// The loop body is wrapped in [`catch_unwind`] to detect panics from
/// [`block_until_ready_then_dispatch()`]. If a panic is caught, the framework sends
/// [`RRTEvent::Shutdown(Panic)`] to notify subscribers, then exits the thread. No restart
/// is attempted - a panic signals a logic bug, not a transient resource issue.
/// Subscribers can call [`try_subscribe()`] to relaunch a fresh thread if appropriate.
///
/// See [`rrt_restart_pty_tests`] for a [`PTY`] integration test that exercises restart
/// cycles.
///
/// When the loop exits (normally or via panic), [`TerminationGuard::drop()`] runs,
/// transitioning the state to [`Stopped`] and calling [`notify_all()`] so the next
/// [`try_subscribe()`] call can cleanly spawn a new thread.
///
/// [`Arc<ThreadLifecycleMonitor>`]: std::sync::Arc
/// [`AtomicU8`]: std::sync::atomic::AtomicU8
/// [`block_until_ready_then_dispatch()`]:
///     super::RRTWorker::block_until_ready_then_dispatch
/// [`catch_unwind`]: std::panic::catch_unwind
/// [`Condvar`]: std::sync::Condvar
/// [`Continuation::Continue`]: crate::core::common::Continuation::Continue
/// [`Continuation::Restart`]: crate::core::common::Continuation::Restart
/// [`Continuation::Stop`]: crate::core::common::Continuation::Stop
/// [`Continuation`]: crate::core::common::Continuation
/// [`create_and_register_os_sources()`]: super::RRTWorker::create_and_register_os_sources
/// [`direct_to_ansi`]: crate::terminal_lib_backends::direct_to_ansi
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`Event`]: super::RRTWorker::Event
/// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
/// [`global_input_resource::SINGLETON`]:
///     crate::terminal_lib_backends::direct_to_ansi::input::global_input_resource::SINGLETON
/// [`InterruptHandle`]: super::InterruptHandle
/// [`io_uring`]: https://kernel.dk/io_uring.pdf
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
/// [`LazyLock`]: std::sync::LazyLock
/// [`mio::Poll`]: mio::Poll
/// [`MioPollWorker::block_until_ready_then_dispatch_impl()`]:
///     crate::terminal_lib_backends::MioPollWorker::block_until_ready_then_dispatch_impl
/// [`Monitor`]: crate::core::common::Monitor
/// [`Mutex`]: std::sync::Mutex
/// [`notify_all()`]: std::sync::Condvar::notify_all
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`Restarting`]: super::ThreadState::Restarting
/// [`RestartPolicy`]: super::RestartPolicy
/// [`rrt_restart_pty_tests`]:
///     super::rrt_integration_tests::pty_test_production_factory_restart
/// [`RRTEvent::Shutdown(Panic)`]: super::ShutdownReason::Panic
/// [`RRTEvent::Shutdown`]: super::RRTEvent::Shutdown
/// [`RRTEvent::Worker`]: super::RRTEvent::Worker
/// [`RRTEvent`]: super::RRTEvent
/// [`RRTSoftwareInterrupt`]: super::RRTSoftwareInterrupt
/// [`RRTWorker::create_and_register_os_sources()`]:
///     super::RRTWorker::create_and_register_os_sources
/// [`RRTWorker`]: super::RRTWorker
/// [`run_worker_loop(worker, ...)`]: super::run_worker_loop
/// [`Running`]: super::ThreadState::Running
/// [`sender`]: field@Self::sender
/// [`shared_state`]: field@Self::shared_state
/// [`Shutdown(Panic)`]: super::ShutdownReason::Panic
/// [`signals`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`Starting`]: super::ThreadState::Starting
/// [`stdin`]: https://en.wikipedia.org/wiki/Standard_streams#Standard_input_(stdin)
/// [`Stopped`]: super::ThreadState::Stopped
/// [`Stopping`]: super::ThreadState::Stopping
/// [`StopReason::WorkerRequested`]: super::StopReason::WorkerRequested
/// [`StopReason::ZeroReceivers`]: super::StopReason::ZeroReceivers
/// [`StopReason`]: super::StopReason
/// [`String`]: std::string::String
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`TerminationGuard::drop()`]: super::TerminationGuard#method.drop
/// [`TerminationGuard`]: super::TerminationGuard
/// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
/// [`ThreadState`]: super::ThreadState
/// [`try_subscribe()`]: Self::try_subscribe
/// [`Vec<u8>`]: std::vec::Vec
/// [architecture overview]: super#architecture-overview
/// [broadcast channel]: tokio::sync::broadcast
/// [generation]: Self::get_thread_generation
/// [parent module documentation]: super
/// [Static vs. Flexible Usage]: super#static-vs-flexible-usage
/// [thread]: https://en.wikipedia.org/wiki/Thread_(computing)
/// [two-phase setup]: super#two-phase-setup
/// [What Is the RRT Pattern?]: super#what-is-the-rrt-pattern
#[allow(missing_debug_implementations)]
pub struct RRT<W: RRTWorker> {
    /// Broadcast channel's [sender]-half - lazily initialized on first access, and never
    /// replaced.
    ///
    /// This [sender]-half broadcasts both of these kinds of events to all async
    /// consumers (in your [`TUI`] or [`readline_async`] app):
    /// 1. Domain events ([`RRTEvent::Worker`], from your injected code in
    ///    [`RRTWorker::block_until_ready_then_dispatch`]).
    /// 2. Framework events ([`RRTEvent::Shutdown`], from [`RRT`] itself).
    ///
    /// The [broadcast channel] outlives every [thread generation], so old and new
    /// subscribers always share the same channel.
    ///
    /// We don't need to save the [broadcast channel] instance, or its [receiver]-half,
    /// only its [sender]-half. We can always create a new [receiver]-half via
    /// [`sender.subscribe()`].
    ///
    /// We use [`LazyLock`] because [`broadcast::channel()`] is not a [const expression].
    /// [`LazyLock`] defers creation to first access, and the actual channel is created
    /// transparently via [`Deref`].
    ///
    /// [`broadcast::channel()`]: tokio::sync::broadcast::channel()
    /// [`Deref`]: std::ops::Deref
    /// [`LazyLock`]: std::sync::LazyLock
    /// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
    /// [`RRTEvent::Shutdown`]: super::RRTEvent::Shutdown
    /// [`RRTEvent::Worker`]: super::RRTEvent::Worker
    /// [`sender.subscribe()`]: tokio::sync::broadcast::Sender::subscribe()
    /// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
    /// [broadcast channel]: tokio::sync::broadcast
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    /// [receiver]: tokio::sync::broadcast::Receiver
    /// [sender]: tokio::sync::broadcast::Sender
    /// [thread generation]: RRT#thread-lifecycle
    pub sender: LazyLock<BroadcastSender<W::Event>>,

    /// Lazily initialized [`ThreadLifecycleMonitor`] - the [monitor] holding the state
    /// machine (wrapped in a [`Mutex`]) and [`Condvar`] that govern the dedicated
    /// thread's lifecycle.
    ///
    /// # Why [`Arc`]?
    ///
    /// Multiple actors hold clones of this [`Arc`] and coordinate through the same
    /// [`Mutex`] / [`Condvar`] pair: the owning [`RRT`] singleton, each
    /// [`SubscriberGuard`], the dedicated thread itself, and the [`TerminationGuard`].
    /// All clones see the same monitor instance. The [`LazyLock`] wrapper exists for the
    /// same reason as on [`sender`] - [`Arc::new(...)`] is not a [const expression], so
    /// initialization is deferred to first access.
    ///
    /// [`Arc::new(...)`]: std::sync::Arc::new
    /// [`Arc`]: std::sync::Arc
    /// [`Condvar`]: std::sync::Condvar
    /// [`InterruptHandle`]: super::InterruptHandle
    /// [`LazyLock`]: std::sync::LazyLock
    /// [`Mutex`]: std::sync::Mutex
    /// [`Restarting`]: super::ThreadState::Restarting
    /// [`Running`]: super::ThreadState::Running
    /// [`sender`]: field@Self::sender
    /// [`Starting`]: super::ThreadState::Starting
    /// [`Stopped`]: super::ThreadState::Stopped
    /// [`Stopping`]: super::ThreadState::Stopping
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`TerminationGuard`]: super::TerminationGuard
    /// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
    /// [`ThreadState`]: super::ThreadState
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    /// [fast-path race]: super#the-fast-path-race
    /// [historical context]: super#historical-context-race-conditions-eliminated
    /// [Monitor]: https://en.wikipedia.org/wiki/Monitor_(synchronization)
    /// [monitor]: https://en.wikipedia.org/wiki/Monitor_(synchronization)
    /// [zombie interrupt bug]: super#the-zombie-interrupt-bug
    pub shared_state: LazyLock<Arc<ThreadLifecycleMonitor<W>>>,
}

impl<W: RRTWorker> Default for RRT<W> {
    fn default() -> Self { Self::new() }
}

impl<W: RRTWorker> RRT<W> {
    /// Creates a new [`RRT`] instance.
    ///
    /// This is a [`const fn`] because it uses [`LazyLock`] to wrap all fields that
    /// require runtime setup (like [`Mutex`], [`Condvar`], and the [`broadcast
    /// channel`]). By deferring their initialization until they are first accessed,
    /// [`RRT`] can be declared as a `static` singleton while still being initialized with
    /// a [const expression].
    ///
    /// Initialization happens automatically the first time [`try_subscribe()`] is called.
    ///
    /// # Returns
    ///
    /// Returns a new instance with a [`Stopped`] liveness state.
    ///
    /// [`broadcast channel`]: tokio::sync::broadcast
    /// [`Condvar`]: std::sync::Condvar
    /// [`const fn`]: #const-expression-vs-const-declaration-vs-static-declaration
    /// [`LazyLock`]: std::sync::LazyLock
    /// [`Mutex`]: std::sync::Mutex
    /// [`Stopped`]: super::ThreadState::Stopped
    /// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
    /// [`try_subscribe()`]: RRT::try_subscribe
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sender: LazyLock::new(|| broadcast::channel(W::CHANNEL_CAPACITY).0),
            shared_state: LazyLock::new(|| {
                Arc::new(ThreadLifecycleMonitor::new(ThreadState::Stopped))
            }),
        }
    }

    /// Subscribes to the dedicated thread, spawning it if it is not already running.
    ///
    /// This is the primary entry point for async consumers (e.g., [`TUI`] and
    /// [`readline_async`] apps) to interact with the RRT framework. It returns a
    /// [`SubscriberGuard`] that keeps the thread alive as long as it is held.
    ///
    /// # The Typestate Loop
    ///
    /// This method uses a `loop { match state { ... } }` pattern paired with a
    /// [`Condvar`] to guarantee race-free thread spawning and subscription. It acquires
    /// the [lock for the `state`] and evaluates the current [`ThreadState`]:
    ///
    /// - **[`Stopped`]**: The first caller (the [`tokio`] thread that runs
    ///   [`try_subscribe()`]) to arrive and find the dedicated thread stopped takes on
    ///   the responsibility of starting it up. To prevent everyone from trying to start
    ///   the thread at the same time, this first caller immediately changes the state to
    ///   [`Starting`]. This acts like a "work in progress" sign, telling any other
    ///   callers who arrive later to just wait. This first caller then does the heavy
    ///   lifting: allocating OS resources and spawning the actual thread. Once everything
    ///   is ready, they change the state to [`Running`] and signal all the waiting
    ///   callers that they can now proceed. Finally, this first caller receives their
    ///   [`SubscriberGuard`].
    /// - **[`Running`]**: Fast path. The thread is fully alive and ready. Returns a
    ///   [`SubscriberGuard`] immediately.
    /// - **[`Starting`], [`Stopping`], [`Restarting`]**: Transient phases. The method
    ///   blocks on the [`Condvar`] (which atomically releases the lock). When woken via
    ///   [`notify_all()`], it loops back to the top, re-evaluates the state, and proceeds
    ///   based on the new variant (typically seeing `Running` or `Stopped`).
    ///
    /// # Returns
    ///
    /// - [`Ok(SubscriberGuard)`]: Successfully subscribed (and potentially spawned).
    /// - [`Err(SubscribeError)`]: Failed to allocate OS resources during spawn.
    ///
    /// # Errors
    ///
    /// [`SubscribeError`] for state-machine failures:
    ///   - [`MutexPoisoned`] if the lifecycle lock or condvar wait is poisoned.
    ///   - [`WorkerCreation`] if creating the worker/interrupt pair fails.
    ///   - [`ThreadSpawn`] if spawning the dedicated thread fails.
    ///
    /// [`Condvar`]: std::sync::Condvar
    /// [`direct_to_ansi`]: crate::terminal_lib_backends::direct_to_ansi
    /// [`Err(SubscribeError)`]: SubscribeError
    /// [`MutexPoisoned`]: SubscribeError::MutexPoisoned
    /// [`notify_all()`]: std::sync::Condvar::notify_all
    /// [`Ok(SubscriberGuard)`]: SubscriberGuard
    /// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
    /// [`Restarting`]: ThreadState::Restarting
    /// [`Running`]: ThreadState::Running
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopped`]: ThreadState::Stopped
    /// [`Stopping`]: ThreadState::Stopping
    /// [`SubscribeError`]: SubscribeError
    /// [`SubscriberGuard`]: SubscriberGuard
    /// [`ThreadSpawn`]: SubscribeError::ThreadSpawn
    /// [`ThreadState`]: ThreadState
    /// [`tokio`]: tokio
    /// [`try_subscribe()`]: RRT::try_subscribe
    /// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
    /// [`WorkerCreation`]: SubscribeError::WorkerCreation
    /// [lock for the `state`]: super::ThreadLifecycleMonitor::lock()
    pub fn try_subscribe(&self) -> Result<SubscriberGuard<W>, SubscribeError> {
        try_subscribe::<W>(&self.sender, &self.shared_state)
    }

    /// Queries how many receivers are subscribed to the broadcast channel.
    ///
    /// Useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// The number of active receivers, or `0` if uninitialized.
    #[must_use]
    pub fn get_receiver_count(&self) -> usize { self.sender.receiver_count() }

    /// Returns the current thread generation number.
    ///
    /// Each time a new dedicated thread is spawned, the generation increments. This
    /// allows tests to verify whether a thread was reused or relaunched:
    ///
    /// - **Same generation**: Thread was reused (new subscriber appeared before thread
    ///   exited)
    /// - **Different generation**: Thread was relaunched (a new thread was spawned)
    ///
    /// # Returns
    ///
    /// The current generation number.
    #[must_use]
    pub fn get_thread_generation(&self) -> u8 {
        self.shared_state.thread_generation.get()
    }
}

/// Internal enum used by [`try_subscribe()`] to decide how to handle a new subscriber
/// based on the current stable [`ThreadState`].
enum SubscriptionStrategy {
    /// The thread is already [`Running`]. We can just
    /// subscribe to the existing broadcast channel.
    ///
    /// [`Running`]: ThreadState::Running
    FastPath,

    /// The thread is [`Stopped`]. We need to transition to
    /// [`Starting`], allocate OS resources, and spawn
    /// a new dedicated thread.
    ///
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopped`]: ThreadState::Stopped
    SlowPath,
}

impl<W: RRTWorker> From<&ThreadState<W>> for SubscriptionStrategy {
    /// Converts a stable [`ThreadState`] into a [`SubscriptionStrategy`].
    ///
    /// # Panics
    ///
    /// Panics if the state is transient ([`Starting`], [`Stopping`], or [`Restarting`]).
    /// The caller ([`try_subscribe()`]) ensures the state is stable by calling
    /// [`block_until_stable_state_reached()`] before this conversion.
    ///
    /// [`block_until_stable_state_reached()`]:
    ///     ThreadLifecycleMonitor::block_until_stable_state_reached
    /// [`Restarting`]: ThreadState::Restarting
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopping`]: ThreadState::Stopping
    fn from(state: &ThreadState<W>) -> Self {
        match state {
            ThreadState::Stopped => SubscriptionStrategy::SlowPath,
            ThreadState::Running(_) => SubscriptionStrategy::FastPath,
            _ => {
                unreachable!("block_until_stable_state_reached guarantees a stable state")
            }
        }
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words maxfiles taskthreads rrtwaker

//! Thread lifecycle manager and entry point for the Resilient Reactor Thread pattern.
//! See [`RRT`] for details.
use super::{RRTEvent, RRTWorker, RestartPolicy, ShutdownReason, SubscriberGuard};
use crate::{core::common::{AtomicU8Ext, Continuation},
            ok};
use std::{panic::{AssertUnwindSafe, catch_unwind},
          sync::{Arc, LazyLock, Mutex, atomic::AtomicU8},
          time::Duration};
use tokio::sync::broadcast;

pub type BroadcastSender<E> = broadcast::Sender<RRTEvent<E>>;
pub type SharedWakerSlot<K> = Arc<Mutex<Option<K>>>;

/// The entry point for the Resilient Reactor Thread (RRT) framework.
///
/// This struct manages the lifecycle of a single dedicated thread (at most one at a time)
/// with automatic spawn/shutdown/reuse semantics. It is designed to be used as a
/// **singleton** via a [`static` declaration][const], using [`RRT::new()`].
///
/// ```no_run
/// # use r3bl_tui::MioPollWorker;
/// # use r3bl_tui::core::resilient_reactor_thread::RRT;
/// // Declaration site (static + const fn = singleton).
/// static SINGLETON: RRT<MioPollWorker> = RRT::new();
///
/// # fn main() -> miette::Result<()> {
/// // Use site (subscribe to get a guard that auto-manages the thread).
/// let guard = SINGLETON.subscribe()?;
/// # Ok(())
/// # }
/// ```
///
/// See [`global_input_resource::SINGLETON`] for the real usage in the terminal input
/// system.
///
/// The struct has three top-level fields, each using a synchronization primitive that
/// matches its lifetime (see each field's documentation for details):
///
/// - **[`sender`]**: Broadcast channel, lazily initialized, never replaced. This acts as
///   the bridge between the async consumers (subscribers) and the dedicated thread
///   (worker).
/// - **[`shared_waker_slot`]**: Shared waker wrapper, lazily initialized. Its [`Option`]
///   state IS the liveness signal (`Some` = running, `None` = terminated or not started).
/// - **[`generation`]**: Per-thread-generation counter via [`AtomicU8`].
///
/// See also:
///
/// - [architecture overview] - how these fields work together.
/// - [What Is the RRT Pattern?] - a rundown of the design.
/// - [`global_input_resource::SINGLETON`] - the real implementation used by the terminal
///   input system.
///
/// # Thread Lifecycle
///
/// The dedicated ([worker]) thread's lifecycle progresses through these phases:
///
/// 1. **Before first [`subscribe()`]** - [broadcast channel] and [waker] are
///    uninitialized ([`LazyLock`] defers creation until first access),
///    [`shared_waker_slot`] is `None`, meaning liveness is [terminated or not started].
/// 2. **First [`subscribe()`]** - sets everything up:
///    - Initializes the [broadcast channel] (via [`LazyLock`]).
///    - Creates a [worker]/[waker] pair via [`RRTWorker::create()`].
///    - Stores the [waker] in [`shared_waker_slot`].
///    - Spawns the dedicated thread, moving the [worker] into [`run_worker_loop(worker,
///      ...)`] as a local `mut` variable on the thread's stack.
///    - Updates the thread's [generation] identifier.
/// 3. **While [running]** - inside [`run_worker_loop(worker, ...)`], the thread enters a
///    loop that calls [`poll_once()`] repeatedly; this is a blocking function. It
///    unblocks when at least one of its I/O sources is ready (e.g., [`epoll`]/[`kqueue`]
///    readiness, [`io_uring`] completion). Your [`poll_once()`] implementation (see
///    [`MioPollWorker::poll_once_impl()`] for a concrete example) processes the data from
///    each ready source, broadcasts [events] to subscribers, and finally returns a
///    [`Continuation`] that directs the framework as follows:
///    - [`Continuation::Continue`] - iteration handled; the framework calls
///      [`poll_once()`] again, which blocks until the next source is ready.
///    - [`Continuation::Stop`] - thread exits (see step 4).
///    - [`Continuation::Restart`] - your code detected that OS resources are broken
///      (e.g., a dead [`fd`] or corrupted [`mio::Poll`]); the framework creates a fresh
///      [worker]/[waker] pair via [`RRTWorker::create()`], drops the old [worker] and
///      replaces it on the thread's stack, and swaps the new [waker] into
///      [`shared_waker_slot`]. The thread stays alive; subscribers are unaffected. See
///      [self-healing restart details].
/// 4. **Thread exits** - when all subscribers (async consumers in your app) drop their
///    [`SubscriberGuard`]s (see [Drop Behavior]):
///    - [`receiver`] (a field in [`SubscriberGuard`]) is dropped first, decrementing
///      [`receiver_count()`] on the [broadcast channel].
///    - [`WakeOnDrop`] (a field in [`SubscriberGuard`]) is dropped next, calling
///      [`RRTWaker::wake()`] which unblocks the [`poll_once()`] call that is currently
///      parked waiting for I/O readiness.
///    - The thread wakes, checks whether the [broadcast channel]'s [`receiver_count()`]
///      `== 0`, and exits if it is.
///    - [`TerminationGuard::drop()`] (a local [RAII] guard in [`run_worker_loop(worker,
///      ...)`]) clears the [waker] to [`None`], leaving liveness in the [terminated or
///      not started] state.
/// 5. **Panic exit** - if your [`poll_once()`] implementation panics, it does not take
///    down the process. The framework catches it (via [`catch_unwind`]), sends
///    [`Shutdown(Panic)`] to subscribers, and exits the thread. No restart is attempted -
///    a panic signals a logic bug that self-healing cannot fix. Subscribers can call
///    [`subscribe()`] to relaunch a fresh thread. See [thread termination paths] for all
///    exit paths.
/// 6. **Next [`subscribe()`]** - detects [waker] is [`None`], spawns a fresh thread,
///    installs a new [waker], and updates the [generation]. The cycle repeats from step
///    3.
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
/// # use std::sync::Mutex;
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
/// # use std::thread::spawn;
/// fn spawn_thread<T: Send + 'static>(it: T) { spawn(move || drop(it)); }
/// spawn_thread(String::from("owned"));    // ✅ String: 'static
/// spawn_thread("literal");                // ✅ &'static str: 'static
/// ```
///
/// This fails to compile - `&String` has a non-`'static` lifetime:
///
/// ```compile_fail
/// # use std::thread::spawn;
/// # fn spawn_thread<T: Send + 'static>(it: T) { spawn(move || drop(it)); }
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
/// why the traits [`RRTWorker`], [`RRTWaker`], and the associated [`Event`] type all
/// require `'static`.
///
/// [Drop Behavior]: super::SubscriberGuard#drop-behavior
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [What Is the RRT Pattern?]: super#what-is-the-rrt-pattern
/// [`AtomicU8`]: std::sync::atomic::AtomicU8
/// [`Continuation::Continue`]: crate::Continuation::Continue
/// [`Continuation::Restart`]: crate::Continuation::Restart
/// [`Continuation::Stop`]: crate::Continuation::Stop
/// [`Event`]: super::RRTWorker::Event
/// [`LazyLock`]: std::sync::LazyLock
/// [`MioPollWorker::poll_once_impl()`]:
///     function@crate::terminal_lib_backends::MioPollWorker::poll_once_impl
/// [`RRTWaker::wake()`]: super::RRTWaker::wake
/// [`RRTWaker`]: super::RRTWaker
/// [`RRTWorker::create()`]: super::RRTWorker::create
/// [`RRTWorker`]: super::RRTWorker
/// [`Shutdown(Panic)`]: super::ShutdownReason::Panic
/// [`String`]: std::string::String
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`TerminationGuard::drop()`]: TerminationGuard#impl-Drop-for-TerminationGuard
/// [`TerminationGuard`]: TerminationGuard
/// [`Vec<u8>`]: std::vec::Vec
/// [`WakeOnDrop`]: super::rrt_subscriber_guard::WakeOnDrop
/// [`catch_unwind`]: std::panic::catch_unwind
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`generation`]: field@Self::thread_generation
/// [`global_input_resource::SINGLETON`]:
///     crate::terminal_lib_backends::global_input_resource::SINGLETON
/// [`io_uring`]: https://kernel.dk/io_uring.pdf
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`mio::Poll`]: mio::Poll
/// [`poll_once()`]: super::RRTWorker::poll_once
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
/// [`receiver`]: super::SubscriberGuard::receiver
/// [`run_worker_loop(worker, ...)`]: run_worker_loop
/// [`sender`]: field@Self::sender
/// [`shared_waker_slot`]: field@Self::shared_waker_slot
/// [`subscribe()`]: Self::subscribe
/// [architecture overview]: super#architecture-overview
/// [broadcast channel]: tokio::sync::broadcast
/// [const]: #const-expression-vs-const-declaration-vs-static-declaration
/// [events]: super::RRTEvent
/// [generation]: Self::get_thread_generation
/// [running]: LivenessState::Running
/// [self-healing restart details]: super#self-healing-restart-details
/// [terminated or not started]: LivenessState::TerminatedOrNotStarted
/// [thread termination paths]: super#thread-termination-paths
/// [two-phase setup]: super#two-phase-setup
/// [waker]: super::RRTWaker
/// [worker]: super::RRTWorker
#[allow(missing_debug_implementations)]
pub struct RRT<W: RRTWorker> {
    /// Broadcast channel sender - lazily initialized on first access, never replaced.
    ///
    /// The channel outlives every thread generation, so old and new subscribers always
    /// share the same channel. It carries both:
    /// 1. Domain events ([`RRTEvent::Worker`]) generated by your injected code.
    /// 2. Framework infrastructure events ([`RRTEvent::Shutdown`]) generated by [`RRT`]
    ///    itself.
    ///
    /// We use [`LazyLock`] because [`tokio::sync::broadcast::channel()`] is not a [const
    /// expression] - [`LazyLock`] defers creation to first access, and the actual
    /// channel is created transparently via [`Deref`].
    ///
    /// [`Deref`]: std::ops::Deref
    /// [`LazyLock`]: std::sync::LazyLock
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    pub sender: LazyLock<BroadcastSender<W::Event>>,

    /// Shared waker wrapper - lazily initialized on first access, inner value swapped
    /// per generation.
    ///
    /// The [`SharedWakerSlot`]'s [`Option`] state serves as the liveness signal:
    /// `Some(waker)` = running, `None` = terminated or not started.
    ///
    /// - The [`Arc<Mutex<...>>`] wrapper ("shared") is created once via [`LazyLock`].
    /// - The inner [`Option<W::Waker>`] ("slot") is swapped on relaunch (set to
    ///   `Some(new_waker)`) and cleared to [`None`] when the thread dies.
    /// - All [`SubscriberGuard`]s hold a clone of this [`Arc`], so they always read the
    ///   *current* waker - solving the **zombie thread bug** where old subscribers would
    ///   call a stale waker targeting a dead [`mio::Poll`].
    ///
    /// We use [`LazyLock`] because [`Arc::new(Mutex::new(...))`] is not a [const
    /// expression] - [`LazyLock`] defers creation to first access, and the wrapper is
    /// created transparently via [`Deref`].
    ///
    /// [`Arc::new(Mutex::new(...))`]: std::sync::Arc::new
    /// [`Arc`]: std::sync::Arc
    /// [`Deref`]: std::ops::Deref
    /// [`LazyLock`]: std::sync::LazyLock
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`TerminationGuard`]: TerminationGuard
    /// [`mio::Poll`]: mio::Poll
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    pub shared_waker_slot: LazyLock<SharedWakerSlot<W::Waker>>,

    /// Per-thread-generation counter. Incremented each time a new thread is spawned.
    ///
    /// Unlike [`sender`] and [`shared_waker_slot`], generation tracking is
    /// per-thread-generation. Each relaunch stores a new generation number via
    /// [`AtomicU8`]. No [`Mutex`] needed - atomic operations are sufficient for a single
    /// counter.
    ///
    /// [`AtomicU8`]: std::sync::atomic::AtomicU8
    /// [`Mutex`]: std::sync::Mutex
    /// [`sender`]: field@Self::sender
    /// [`shared_waker_slot`]: field@Self::shared_waker_slot
    /// [`thread_generation`]: field@Self::thread_generation
    pub thread_generation: AtomicU8,
}

impl<W: RRTWorker> RRT<W> {
    /// Creates a new uninitialized global state.
    ///
    /// This is a [const expression][const] so it can be used in [static
    /// declarations][const]. See [`global_input_resource::SINGLETON`] for a real usage
    /// example.
    ///
    /// [`global_input_resource::SINGLETON`]:
    ///     crate::terminal_lib_backends::global_input_resource::SINGLETON
    /// [const]: #const-expression-vs-const-declaration-vs-static-declaration
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sender: LazyLock::new(|| broadcast::channel(W::CHANNEL_CAPACITY).0),
            shared_waker_slot: LazyLock::new(|| Arc::new(Mutex::new(None))),
            thread_generation: AtomicU8::new(0),
        }
    }

    /// Allocates a subscription, spawning the dedicated thread if needed.
    ///
    /// # Two Allocation Paths
    ///
    /// The waker's [`Option`] state is the liveness signal: `Some` means the thread is
    /// running, `None` means it terminated (cleared by [`TerminationGuard`] on thread
    /// exit).
    ///
    /// | Condition              | Path          | What Happens                     |
    /// | ---------------------- | ------------- | -------------------------------- |
    /// | `waker == Some`        | **Fast path** | Reuse existing thread            |
    /// | `waker == None`        | **Slow path** | Spawn new thread, install waker  |
    ///
    /// ## Fast Path (Thread Reuse)
    ///
    /// If the thread is still running (waker is `Some`), we **reuse everything**:
    /// - Same broadcast channel (singleton-lifetime, never replaced)
    /// - Same waker (still valid for this generation)
    /// - Same thread (continues serving the new subscriber)
    ///
    /// This handles the [race condition] where a new subscriber appears before the thread
    /// checks [`receiver_count()`].
    ///
    /// ## Slow Path (Thread Restart)
    ///
    /// If the thread has terminated (or never started):
    /// 1. `&*sender` - lazily creates channel on first access
    /// 2. `&*shared_waker_slot` - lazily creates wrapper on first access
    /// 3. [`RRTWorker::create()`] - creates fresh worker + waker pair (see [Two-Phase
    ///    Setup] in the module docs)
    /// 4. Swap waker: `*shared_waker_slot.lock() = Some(new_waker)`
    /// 5. Record new generation, spawn thread
    ///
    /// The broadcast channel and waker wrapper are **never replaced** - only the inner
    /// waker value and generation change on relaunch.
    ///
    /// # Errors
    ///
    /// Returns [`SubscribeError`] - see its variants for the three failure modes: mutex
    /// poisoning, worker creation failure, and thread spawn failure.
    ///
    /// [Two-Phase Setup]: super#two-phase-setup
    /// [`RRTWorker::create()`]: RRTWorker::create
    /// [`TerminationGuard`]: TerminationGuard
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [race condition]: super#the-inherent-race-condition
    pub fn subscribe(&self) -> Result<SubscriberGuard<W>, SubscribeError> {
        let sender = &*self.sender;
        let shared_waker_slot = &*self.shared_waker_slot;

        // Is thread running? Lock the waker and check.
        let mut waker_guard = shared_waker_slot
            .lock()
            .map_err(|_| SubscribeError::MutexPoisoned { which: "waker" })?;
        let thread_is_running = waker_guard.is_some();

        // FAST PATH: thread is running -> reuse it.
        if thread_is_running {
            let receiver = sender.subscribe();
            let shared_waker_slot = shared_waker_slot.clone();
            return ok!(SubscriberGuard::new(receiver, shared_waker_slot));
        }

        // SLOW PATH: thread terminated (or never started). Hold the waker lock (don't
        // drop the waker_guard) for the entire slow path to serialize concurrent
        // subscribe() calls.

        // Create worker and waker pair.
        let (worker, waker) = W::create().map_err(SubscribeError::WorkerCreation)?;

        // Install new waker while holding the lock.
        *waker_guard = Some(waker);

        // Increment thread generation.
        let thread_generation = self.thread_generation.increment();

        // Spawn worker thread.
        let sender_for_thread = sender.clone();
        let shared_waker_slot_for_thread = shared_waker_slot.clone();
        std::thread::Builder::new()
            .name(format!("rrt-worker-gen-{thread_generation}"))
            .spawn(move || {
                run_worker_loop::<W>(
                    worker,
                    sender_for_thread,
                    shared_waker_slot_for_thread,
                );
            })
            .map_err(SubscribeError::ThreadSpawn)?;

        ok!({
            let receiver = sender.subscribe();
            let shared_waker_slot = shared_waker_slot.clone();
            SubscriberGuard::new(receiver, shared_waker_slot)
        })
    }

    /// Checks if the dedicated thread is currently running.
    ///
    /// Useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// - [`LivenessState::Running`] if the thread is running
    /// - [`LivenessState::TerminatedOrNotStarted`] if uninitialized or the thread has
    ///   exited
    #[must_use]
    pub fn is_thread_running(&self) -> LivenessState {
        self.shared_waker_slot
            .lock()
            .ok()
            .map(|guard| {
                if guard.is_some() {
                    LivenessState::Running
                } else {
                    LivenessState::TerminatedOrNotStarted
                }
            })
            .unwrap_or(LivenessState::TerminatedOrNotStarted)
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
    pub fn get_thread_generation(&self) -> u8 { self.thread_generation.get() }

    /// Subscribes to events from an existing thread.
    ///
    /// This is a lightweight operation that creates a new subscriber to the existing
    /// broadcast channel. Use this for additional consumers (logging, debugging, etc.)
    /// after an initial allocation.
    ///
    /// [`subscribe()`]: Self::subscribe
    pub fn subscribe_to_existing(&self) -> SubscriberGuard<W> {
        let sender = &*self.sender;
        let shared_waker_slot = &*self.shared_waker_slot;

        let receiver = sender.subscribe();
        let shared_waker_slot = shared_waker_slot.clone();
        SubscriberGuard::new(receiver, shared_waker_slot)
    }
}

/// [RAII] guard that clears the waker to [`None`] when the dedicated thread's work loop
/// exits.
///
/// The waker's [`Option`] state IS the liveness signal: `Some(waker)` means the thread is
/// running, `None` means it has terminated. Clearing it to `None` is the only cleanup
/// needed - [`subscribe()`] checks `is_none()` to detect termination.
///
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`subscribe()`]: RRT::subscribe
#[allow(missing_debug_implementations)]
pub struct TerminationGuard<W: RRTWorker> {
    shared_waker_slot: SharedWakerSlot<W::Waker>,
}

impl<W: RRTWorker> Drop for TerminationGuard<W> {
    fn drop(&mut self) {
        // Clear waker so no subscriber can call stale wake(), and so subscribe()
        // detects termination via is_none().
        if let Ok(mut guard) = self.shared_waker_slot.lock() {
            *guard = None;
        }
    }
}

/// Runs the poll loop on the dedicated thread with restart policy support.
///
/// Called from the spawned dedicated thread. The loop handles three [`Continuation`]
/// variants:
///
/// - [`Continue`]: Call [`poll_once()`] again.
/// - [`Stop`]: Always respected. Thread exits cleanly.
/// - [`Restart`]: Drop the current [`RRTWorker`], apply the [`RestartPolicy`], and call
///   [`W::create()`] to get a fresh [`RRTWorker`]. On success the restart budget resets
///   so each fresh [`RRTWorker`] gets the full allowance for future incidents. If the
///   policy is exhausted before [`W::create()`] succeeds, send [`RRTEvent::Shutdown`] to
///   subscribers and exit.
///
/// **Panic handling** - The loop body is wrapped in [`catch_unwind`] to detect panics
/// from [`poll_once()`]. If a panic is caught, the framework sends
/// [`RRTEvent::Shutdown(Panic)`] to notify subscribers, then exits the thread. No restart
/// is attempted - a panic signals a logic bug, not a transient resource issue.
/// Subscribers can call [`subscribe()`] to relaunch a fresh thread if appropriate.
///
/// See [self-healing restart details] for the full restart lifecycle, backoff sequence,
/// and two-tier event model. See [`rrt_restart_pty_tests`] for a PTY integration test
/// that exercises restart cycles with production [`MioPollWorker::create()`] calls.
///
/// When the loop exits, [`TerminationGuard`] clears the waker to [`None`] so the next
/// [`subscribe()`] call detects termination and spawns a new thread.
///
/// [`Continue`]: Continuation::Continue
/// [`MioPollWorker::create()`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker::create
/// [`RRTEvent::Shutdown(Panic)`]: ShutdownReason::Panic
/// [`Restart`]: Continuation::Restart
/// [`Stop`]: Continuation::Stop
/// [`W::create()`]: RRTWorker::create
/// [`catch_unwind`]: std::panic::catch_unwind
/// [`poll_once()`]: super::RRTWorker::poll_once
/// [`rrt_restart_pty_tests`]: super::tests::rrt_restart_pty_tests
/// [`subscribe()`]: RRT::subscribe
/// [self-healing restart details]: super#self-healing-restart-details
pub fn run_worker_loop<W>(
    mut worker: W,
    sender: BroadcastSender<W::Event>,
    shared_waker_slot: SharedWakerSlot<W::Waker>,
) where
    W: RRTWorker,
{
    let _guard = TerminationGuard::<W> {
        shared_waker_slot: shared_waker_slot.clone(),
    };

    let policy = W::restart_policy();
    let mut restart_count: u8 = 0;
    let mut current_delay = policy.initial_delay;

    // Clone sender before the closure so it remains available for panic notification.
    let sender_for_panic = sender.clone();

    // Safety: AssertUnwindSafe is sound here. The closure captures &mut worker, &sender,
    // &shared_waker_slot, &policy, &mut restart_count, and &mut current_delay. After
    // catching a panic we don't touch any of the captured loop state - we only send a
    // Shutdown(Panic) notification via the pre-cloned sender_for_panic and then exit. No
    // potentially-corrupted state is observed or reused.
    let result = catch_unwind(AssertUnwindSafe(|| {
        loop {
            match worker.poll_once(&sender) {
                Continuation::Continue => {}

                Continuation::Stop => break,

                Continuation::Restart => {
                    // Inner retry loop: handles both "restart worker" and "W::create()
                    // itself failed" cases.
                    let exhausted = loop {
                        restart_count += 1;
                        if restart_count > policy.max_restarts {
                            drop(sender.send(RRTEvent::Shutdown(
                                ShutdownReason::RestartPolicyExhausted {
                                    attempts: restart_count,
                                },
                            )));
                            break true;
                        }

                        // Apply delay before attempting restart.
                        if let Some(delay) = current_delay {
                            std::thread::sleep(delay);
                            current_delay = advance_backoff_delay(delay, &policy);
                        }

                        match W::create() {
                            Ok((new_worker, new_waker)) => {
                                worker = new_worker;
                                // Store the concrete waker directly.
                                if let Ok(mut guard) = shared_waker_slot.lock() {
                                    *guard = Some(new_waker);
                                }
                                // Reset budget so the fresh worker gets a full allowance
                                // for future incidents.
                                restart_count = 0;
                                current_delay = policy.initial_delay;
                                break false; // Success - back to outer poll loop.
                            }
                            Err(_) => continue, // Retry create with next delay.
                        }
                    };

                    // If policy exhausted, exit thread.
                    if exhausted {
                        break;
                    }
                }
            }
        }
    }));

    // If the worker panicked, notify subscribers so they can take corrective action
    // (e.g., call subscribe() to relaunch a fresh thread).
    if result.is_err() {
        drop(sender_for_panic.send(RRTEvent::Shutdown(ShutdownReason::Panic)));
    }

    // _guard dropped here, clearing waker to None.
}

/// Advances the backoff delay for the next restart attempt.
pub fn advance_backoff_delay(
    current: Duration,
    policy: &RestartPolicy,
) -> Option<Duration> {
    match policy.backoff_multiplier {
        Some(multiplier) => {
            let next = current.mul_f64(multiplier);
            Some(match policy.max_delay {
                Some(max) => next.min(max),
                None => next,
            })
        }
        None => Some(current), // No backoff - constant delay.
    }
}

/// Errors from [`RRT::subscribe()`].
///
/// Each variant represents a distinct failure mode with a dedicated OS specific (where
/// appropriate) [diagnostic code] and actionable help text. The three failure modes are:
///
/// | Variant             | Cause                                                       | Recoverable? |
/// | :------------------ | :---------------------------------------------------------- | :----------- |
/// | [`MutexPoisoned`]   | A prior thread panicked while holding an internal RRT lock  | No           |
/// | [`WorkerCreation`]  | [`RRTWorker::create()`] failed (OS resource exhaustion)     | Maybe        |
/// | [`ThreadSpawn`]     | [`std::thread::Builder::spawn()`] failed (thread limits)    | Maybe        |
///
/// [`MutexPoisoned`]: Self::MutexPoisoned
/// [`RRTWorker::create()`]: RRTWorker::create
/// [`ThreadSpawn`]: Self::ThreadSpawn
/// [`WorkerCreation`]: Self::WorkerCreation
/// [diagnostic code]: miette::Diagnostic::code
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum SubscribeError {
    /// Internal mutex was poisoned by a prior thread panic.
    #[error("RRT internal mutex poisoned ({which})")]
    #[diagnostic(
        code(r3bl_tui::rrt::mutex_poisoned),
        help(
            "A prior thread panicked while holding an RRT lock. \
             Consider restarting the application."
        )
    )]
    MutexPoisoned {
        /// Which mutex was poisoned (`"liveness"` or `"waker"`).
        which: &'static str,
    },

    /// [`RRTWorker::create()`] failed to acquire OS resources.
    ///
    /// The inner [`miette::Report`] preserves the full error chain from the worker
    /// implementation (e.g., [`PollCreationError`], [`WakerCreationError`]). Access it
    /// via pattern matching.
    ///
    /// [`PollCreationError`]: crate::terminal_lib_backends::PollCreationError
    /// [`WakerCreationError`]: crate::terminal_lib_backends::WakerCreationError
    #[error("Failed to create worker thread resources")]
    #[diagnostic(code(r3bl_tui::rrt::worker_creation))]
    #[cfg_attr(
        target_os = "linux",
        diagnostic(help(
            "Check OS resource limits - \
             use `ulimit -n` for file descriptors, \
             `cat /proc/sys/fs/file-max` for system-wide limit"
        ))
    )]
    #[cfg_attr(
        target_os = "macos",
        diagnostic(help(
            "Check OS resource limits - \
             use `ulimit -n` for file descriptors, \
             `launchctl limit maxfiles` for system-wide limit"
        ))
    )]
    #[cfg_attr(
        target_os = "windows",
        diagnostic(help(
            "Check OS resource limits - \
             Windows handle limits are typically high, \
             but check Task Manager for handle count"
        ))
    )]
    WorkerCreation(miette::Report),

    /// [`std::thread::Builder::spawn()`] failed.
    #[error("Failed to spawn RRT worker thread")]
    #[diagnostic(code(r3bl_tui::rrt::thread_spawn))]
    #[cfg_attr(
        target_os = "linux",
        diagnostic(help(
            "The system may have reached its thread limit - \
             check `ulimit -u` for per-user limit, \
             `cat /proc/sys/kernel/threads-max` for system-wide limit"
        ))
    )]
    #[cfg_attr(
        target_os = "macos",
        diagnostic(help(
            "The system may have reached its thread limit - \
             check `ulimit -u` for per-user limit, \
             `sysctl kern.num_taskthreads` for per-process limit"
        ))
    )]
    #[cfg_attr(
        target_os = "windows",
        diagnostic(help(
            "The system may have reached its thread limit - \
             check Task Manager for thread count, \
             or use `Get-Process` in PowerShell to inspect per-process threads"
        ))
    )]
    ThreadSpawn(#[source] std::io::Error),
}

/// An indication of whether the dedicated thread is running or terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    Running,
    TerminatedOrNotStarted,
}

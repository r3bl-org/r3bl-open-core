// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words maxfiles taskthreads rrtwaker

//! Thread lifecycle manager and entry point for the Resilient Reactor Thread pattern.
//! See [`RRT`] for details.

use super::{LivenessState, RRTEvent, RRTLiveness, RRTWaker, RRTWorker, RestartPolicy,
            ShutdownReason, SubscriberGuard};
use crate::core::common::Continuation;
use std::{panic::{AssertUnwindSafe, catch_unwind},
          sync::{Arc, Mutex, OnceLock},
          time::Duration};
use tokio::sync::broadcast;

/// Capacity of the broadcast channel for events.
///
/// When the buffer is full, the oldest message is dropped to make room for new ones.
/// Slow consumers will receive [`Lagged`] on their next [`recv()`] call, indicating how
/// many messages they missed.
///
/// `4_096` is generous for typical event streams, but cheap (events are usually small)
/// and provides headroom for debug/logging consumers that might occasionally lag.
///
/// [`Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
/// [`recv()`]: tokio::sync::broadcast::Receiver::recv
pub const CHANNEL_CAPACITY: usize = 4_096;

/// The entry point for the Resilient Reactor Thread (RRT) framework.
///
/// This struct manages the lifecycle of a single dedicated thread (at most one at a time)
/// with automatic spawn/shutdown/reuse semantics. It is a **static container** with three
/// top-level fields, each using a synchronization primitive that matches its lifetime -
/// see each field's documentation for details:
///
/// - **[`broadcast_tx`]**: [`broadcast channel`] created once on first [`subscribe()`]
///   via [`OnceLock`], never replaced.
///   - [`tokio::sync::broadcast::channel()`] is not a [const expression], so it can't
///     live in the `static` [`SINGLETON`] declaration directly.
///   - [`OnceLock`] bridges this gap by deferring creation to the first [`subscribe()`]
///     call. The [`broadcast channel`] outlives every thread generation, so old and new
///     subscribers always share the same channel.
///
/// - **[`waker`]**: The [`Arc<Mutex<...>>`] wrapper is also created once via [`OnceLock`]
///   (same rationale as [`broadcast_tx`]).
///   - The *inner* `Option<Box<dyn RRTWaker>>` is swapped on each [thread relaunch] and
///     cleared to [`None`] when the thread dies.
///   - Because every [`SubscriberGuard`] holds a clone of the same [`Arc`], old and new
///     subscribers always read the *current* waker.
///
/// - **[`liveness`]**: Per-thread-generation, unlike the singleton-lifetime channel and
///   waker.
///   - Each [thread relaunch] creates a fresh [`RRTLiveness`] (with an incremented
///     generation counter).
///   - [`Mutex<Option<...>>`] allows [`subscribe()`] to atomically check and replace the
///     liveness state ([`RRTLiveness`]).
///
/// # Thread Lifecycle
///
/// A typical lifetime progresses through these phases:
///
/// 1. **Before first [`subscribe()`]** - all fields empty (the `static` initializer uses
///    [`OnceLock::new()`] and `Mutex::new(None)`).
/// 2. **First [`subscribe()`]** - initializes the channel and waker wrapper, spawns the
///    dedicated thread, and installs a fresh [`RRTLiveness`].
/// 3. **While running** - [`RRTWorker`] polls in a loop. If it returns
///    [`Continuation::Restart`], the worker is replaced in-place (thread stays alive,
///    subscribers unaffected). See [self-healing restart details].
/// 4. **Thread exits** - when all subscribers drop, the thread exits: waker is cleared to
///    [`None`], liveness is marked [`Terminated`].
/// 5. **Next [`subscribe()`]** - detects [`Terminated`] liveness, spawns a fresh thread,
///    swaps in a new waker, and replaces liveness. The cycle repeats from step 3.
///
/// # Usage
///
/// See [`SINGLETON`] for the real implementation used by the terminal input system.
///
/// ## `const` Expression vs `const` Declaration vs `static` Declaration
///
/// These are different concepts that share the `const` keyword:
///
/// | Term                     | Meaning                                        | Example                                |
/// | :----------------------- | :--------------------------------------------- | :------------------------------------- |
/// | **`const` expression**   | Value the compiler can compute at compile time | `1 + 2`, `Mutex::new(None)`            |
/// | **`const fn`**           | Function callable in const context             | `const fn new() -> Option<T> { None }` |
/// | **`const` declaration**  | Inlined constant (no fixed address)            | `const PI: f64 = 3.14;`                |
/// | **`static` declaration** | Fixed address, single instance (singleton)     | `static GLOBAL: T = ...;`              |
///
/// **Key point:** Both `static` and `const` require a `const` expression as the
/// initializer, but they have opposite runtime behaviors when used in a **declaration**:
///
/// <!-- It is ok to use ignore here - showing static vs const declaration difference -->
///
/// ```ignore
/// // ✅ Singleton: one instance at a fixed address
/// static GLOBAL: Mutex<Option<T>> = Mutex::new(None);
///
/// // ❌ NOT a singleton: value copied at each use site (like a macro expansion)
/// const GLOBAL: Mutex<Option<T>> = Mutex::new(None);
/// ```
///
/// For a singleton, always use `static` keyword in the declaration statement.
///
/// ## `'static` Trait Bound vs `'static` Lifetime Annotation
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
/// why [`RRTWorker`] and the `E` (event) type parameter require `'static`.
///
/// # Poll -> Registry -> Waker Chain
///
/// Your [`RRTWaker`] implementation is tightly coupled to its [blocking mechanism] (e.g.,
/// [`mio::Poll`]):
///
/// ```text
/// mio::Poll::new()      // Creates OS event mechanism (epoll fd / kqueue)
///       │
///       ▼
/// poll.registry()       // Handle to register interest
///       │
///       ▼
/// Waker::new(registry)  // Registers with THIS Poll's mechanism
///       │
///       ▼
/// waker.wake()          // Triggers event → poll.poll() returns
/// ```
///
/// Since a [`Waker`] is bound to the [`Poll`] instance it was created from, replacing one
/// without the other leaves a dead reference. This is why the slow path replaces **both**
/// together (see [two-phase setup]).
///
/// [`Arc<Mutex<...>>`]: std::sync::Arc
/// [`Arc`]: std::sync::Arc
/// [`Continuation::Restart`]: crate::Continuation::Restart
/// [`LivenessState`]: super::LivenessState
/// [`OnceLock::new()`]: std::sync::OnceLock::new
/// [`OnceLock`]: std::sync::OnceLock
/// [`Option<W::Waker>`]: super::RRTWaker
/// [`Poll`]: mio::Poll
/// [`RRTLiveness`]: super::RRTLiveness
/// [`RRTWaker`]: super::RRTWaker
/// [`RRTWorker`]: super::RRTWorker
/// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
/// [`String`]: std::string::String
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`Terminated`]: super::LivenessState::Terminated
/// [`Vec<u8>`]: std::vec::Vec
/// [`Waker`]: mio::Waker
/// [`broadcast channel`]: tokio::sync::broadcast::channel
/// [`broadcast_tx`]: field@Self::broadcast_tx
/// [`liveness`]: field@Self::liveness
/// [`mio::Poll`]: mio::Poll
/// [`subscribe()`]: Self::subscribe
/// [`tokio::sync::broadcast::channel()`]: tokio::sync::broadcast::channel
/// [`waker`]: field@Self::waker
/// [blocking mechanism]: super#understanding-blocking-io
/// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
/// [self-healing restart details]: super#self-healing-restart-details
/// [thread relaunch]: super#how-it-works
/// [two-phase setup]: super#two-phase-setup
#[allow(missing_debug_implementations)]
pub struct RRT<W>
where
    W: RRTWorker,
    W::Event: Clone + Send + Sync + 'static,
{
    /// Broadcast channel sender - created once on first [`subscribe()`], never replaced.
    ///
    /// The channel outlives every thread generation, so old and new subscribers always
    /// share the same channel. It carries both:
    /// 1. Domain events ([`RRTEvent::Worker`]) generated by your injected code.
    /// 2. Framework infrastructure events ([`RRTEvent::Shutdown`]) generated by [`RRT`]
    ///    itself.
    ///
    /// We use [`OnceLock`] because [`tokio::sync::broadcast::channel()`] is not a [const
    /// expression] - the `static` is initialized with an empty [`OnceLock`] (which *is*
    /// const), and the actual channel is created lazily on the first [`subscribe()`]
    /// call.
    ///
    /// [`subscribe()`]: Self::subscribe
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    pub broadcast_tx: OnceLock<broadcast::Sender<RRTEvent<W::Event>>>,

    /// Shared waker wrapper - created once, inner value swapped per generation.
    ///
    /// - The [`Arc<Mutex<...>>`] wrapper is created once via [`OnceLock`].
    /// - The inner [`Option<W::Waker>`] is swapped on relaunch (set to
    ///   `Some(new_waker)`) and cleared to [`None`] when the thread dies.
    /// - All [`SubscriberGuard`]s hold a clone of this [`Arc`], so they always read the
    ///   *current* waker - solving the **zombie thread bug** where old subscribers would
    ///   call a stale waker targeting a dead [`mio::Poll`].
    ///
    /// We use [`OnceLock`] because [`Arc::new(Mutex::new(...))`] is not a [const
    /// expression] - the `static` is initialized with an empty [`OnceLock`] (which *is*
    /// const), and the wrapper is created lazily on the first [`subscribe()`] call.
    ///
    /// [`Arc::new(Mutex::new(...))`]: std::sync::Arc::new
    /// [`Arc`]: std::sync::Arc
    /// [`OnceLock`]: std::sync::OnceLock
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`TerminationGuard`]: TerminationGuard
    /// [`mio::Poll`]: mio::Poll
    /// [`subscribe()`]: Self::subscribe
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    pub waker: OnceLock<Arc<Mutex<Option<Box<dyn RRTWaker>>>>>,

    /// Per-thread-generation liveness tracking. Replaced on each [thread relaunch].
    ///
    /// Unlike [`broadcast_tx`] and [`waker`], liveness state is per-thread-generation.
    /// Each relaunch creates a fresh [`RRTLiveness`] (with an incremented generation
    /// counter). [`Mutex<Option<...>>`] allows [`subscribe()`] to atomically check and
    /// replace the liveness state.
    ///
    /// [`RRTLiveness`]: super::RRTLiveness
    /// [`broadcast_tx`]: field@Self::broadcast_tx
    /// [`subscribe()`]: Self::subscribe
    /// [`waker`]: field@Self::waker
    /// [thread relaunch]: super#how-it-works
    pub liveness: Mutex<Option<Arc<RRTLiveness>>>,
}

impl<W> RRT<W>
where
    W: RRTWorker,
    W::Event: Clone + Send + Sync + 'static,
{
    /// Creates a new uninitialized global state.
    ///
    /// This is a [const expression] so it can be used in [static declarations]. See
    /// [`SINGLETON`] for a real usage example.
    ///
    /// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
    /// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
    /// [static declarations]: #const-expression-vs-const-declaration-vs-static-declaration
    #[must_use]
    pub const fn new() -> Self {
        Self {
            broadcast_tx: OnceLock::new(),
            waker: OnceLock::new(),
            liveness: Mutex::new(None),
        }
    }

    /// Allocates a subscription, spawning the dedicated thread if needed.
    ///
    /// # Two Allocation Paths
    ///
    /// | Condition                | Path          | What Happens                     |
    /// | ------------------------ | ------------- | -------------------------------- |
    /// | `liveness == Running`    | **Fast path** | Reuse existing thread            |
    /// | `liveness == Terminated` | **Slow path** | Spawn new thread, swap waker     |
    ///
    /// ## Fast Path (Thread Reuse)
    ///
    /// If the thread is still running, we **reuse everything**:
    /// - Same broadcast channel (singleton-lifetime, never replaced)
    /// - Same liveness tracker (still valid for this generation)
    /// - Same thread (continues serving the new subscriber)
    ///
    /// This handles the [race condition] where a new subscriber appears before the thread
    /// checks [`receiver_count()`].
    ///
    /// ## Slow Path (Thread Restart)
    ///
    /// If the thread has terminated (or never started):
    /// 1. `broadcast_tx.get_or_init(...)` - idempotent channel creation
    /// 2. `waker.get_or_init(...)` - idempotent wrapper creation
    /// 3. [`RRTWorker::create()`] - creates fresh worker + waker pair
    /// 4. Swap waker: `*shared_waker.lock() = Some(new_waker)`
    /// 5. Create fresh [`RRTLiveness`], spawn thread
    ///
    /// The broadcast channel and waker wrapper are **never replaced** - only the inner
    /// waker value and liveness state change on relaunch.
    ///
    /// # Errors
    ///
    /// Returns [`SubscribeError`] - see its variants for the three failure modes: mutex
    /// poisoning, worker creation failure, and thread spawn failure.
    ///
    /// [`RRTLiveness`]: super::RRTLiveness
    /// [`RRTWorker::create()`]: RRTWorker::create
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [race condition]: super#the-inherent-race-condition
    pub fn subscribe(&self) -> Result<SubscriberGuard<W::Event>, SubscribeError> {
        // Idempotent static initialization - channel created once on first subscribe,
        // never replaced.
        let tx = self
            .broadcast_tx
            .get_or_init(|| broadcast::channel(CHANNEL_CAPACITY).0);
        // Idempotent static initialization - waker wrapper created once on first
        // subscribe. Subsequent thread relaunches swap the *inner* waker value, but the
        // wrapper.
        let shared_waker = self.waker.get_or_init(|| Arc::new(Mutex::new(None)));

        let mut liveness_guard = self
            .liveness
            .lock()
            .map_err(|_| SubscribeError::MutexPoisoned { which: "liveness" })?;

        // FAST PATH: Reuse existing thread.
        let is_running = liveness_guard
            .as_ref()
            .is_some_and(|liveness| liveness.is_running() == LivenessState::Running);

        // SLOW PATH: Thread terminated (or never started) -> create fresh.
        if !is_running {
            // Explicitly clear stale liveness (if any).
            drop(liveness_guard.take());

            // Create worker and waker atomically.
            // See: "Two-Phase Setup" section in mod.rs.
            let (worker, new_waker) =
                W::create().map_err(SubscribeError::WorkerCreation)?;

            // Swap waker: old subscribers now read the new waker.
            {
                let mut waker_guard = shared_waker
                    .lock()
                    .map_err(|_| SubscribeError::MutexPoisoned { which: "waker" })?;
                let boxed: Box<dyn RRTWaker> = Box::new(new_waker);
                *waker_guard = Some(boxed);
            }

            // Create fresh liveness for this generation.
            let liveness = Arc::new(RRTLiveness::new());

            // Spawn worker thread.
            let tx_clone = tx.clone();
            let liveness_for_thread = Arc::clone(&liveness);
            let waker_for_thread = Arc::clone(shared_waker);
            std::thread::Builder::new()
                .name(format!("rrt-worker-gen-{}", liveness.generation))
                .spawn(move || {
                    run_worker_loop::<W>(
                        worker,
                        tx_clone,
                        liveness_for_thread,
                        waker_for_thread,
                    );
                })
                .map_err(SubscribeError::ThreadSpawn)?;

            *liveness_guard = Some(liveness);
        }

        Ok(SubscriberGuard {
            receiver: Some(tx.subscribe()),
            waker: Arc::clone(shared_waker),
        })
    }

    /// Checks if the dedicated thread is currently running.
    ///
    /// Useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// - [`LivenessState::Running`] if the thread is running
    /// - [`LivenessState::Terminated`] if uninitialized or the thread has exited
    #[must_use]
    pub fn is_thread_running(&self) -> LivenessState {
        self.liveness
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|liveness| liveness.is_running()))
            .unwrap_or(LivenessState::Terminated)
    }

    /// Queries how many receivers are subscribed to the broadcast channel.
    ///
    /// Useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// The number of active receivers, or `0` if uninitialized.
    #[must_use]
    pub fn get_receiver_count(&self) -> usize {
        self.broadcast_tx
            .get()
            .map(|tx| tx.receiver_count())
            .unwrap_or(0)
    }

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
    /// The current generation number, or `0` if uninitialized.
    #[must_use]
    pub fn get_thread_generation(&self) -> u8 {
        self.liveness
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|liveness| liveness.generation))
            .unwrap_or(0)
    }

    /// Subscribes to events from an existing thread.
    ///
    /// This is a lightweight operation that creates a new subscriber to the existing
    /// broadcast channel. Use this for additional consumers (logging, debugging, etc.)
    /// after an initial allocation.
    ///
    /// # Panics
    ///
    /// - If the broadcast channel hasn't been initialized yet (call [`subscribe()`]
    ///   first)
    /// - If the waker wrapper hasn't been initialized yet
    ///
    /// [`subscribe()`]: Self::subscribe
    pub fn subscribe_to_existing(&self) -> SubscriberGuard<W::Event> {
        let tx = self.broadcast_tx.get().expect(
            "subscribe_to_existing() called before subscribe(). \
             Subscribe first to create the thread, then add more subscribers.",
        );

        let shared_waker = self.waker.get().expect(
            "subscribe_to_existing() called before subscribe(). \
             Waker wrapper should have been initialized.",
        );

        SubscriberGuard {
            receiver: Some(tx.subscribe()),
            waker: Arc::clone(shared_waker),
        }
    }
}

impl<W> Default for RRT<W>
where
    W: RRTWorker,
    W::Event: Clone + Send + Sync + 'static,
{
    fn default() -> Self { Self::new() }
}

/// [RAII] guard that clears the waker and calls [`mark_terminated()`] when the dedicated
/// thread's work loop exits.
///
/// **Drop ordering matters**: The waker is cleared to `None` *before* marking terminated.
/// If we marked terminated first, [`subscribe()`] could race in, install a new waker, and
/// our cleanup would clear it.
///
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`mark_terminated()`]: super::RRTLiveness::mark_terminated
/// [`subscribe()`]: RRT::subscribe
#[allow(missing_debug_implementations)]
pub struct TerminationGuard {
    liveness: Arc<RRTLiveness>,
    waker: Arc<Mutex<Option<Box<dyn RRTWaker>>>>,
}

impl Drop for TerminationGuard {
    fn drop(&mut self) {
        // Clear waker FIRST so no subscriber can call stale wake(). Order matters: if we
        // mark_terminated() first, subscribe() could race in, install a new waker, and
        // our cleanup would clear it.
        if let Ok(mut guard) = self.waker.lock() {
            *guard = None;
        }
        self.liveness.mark_terminated();
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
/// and two-tier event model.
///
/// When the loop exits, [`TerminationGuard`] clears the waker to [`None`] and calls
/// [`mark_terminated()`] so the next [`subscribe()`] call knows to spawn a new thread.
///
/// [`Continue`]: Continuation::Continue
/// [`RRTEvent::Shutdown(Panic)`]: ShutdownReason::Panic
/// [`Restart`]: Continuation::Restart
/// [`Stop`]: Continuation::Stop
/// [`W::create()`]: RRTWorker::create
/// [`catch_unwind`]: std::panic::catch_unwind
/// [`mark_terminated()`]: super::RRTLiveness::mark_terminated
/// [`poll_once()`]: super::RRTWorker::poll_once
/// [`subscribe()`]: RRT::subscribe
/// [self-healing restart details]: super#self-healing-restart-details
pub fn run_worker_loop<W>(
    mut worker: W,
    tx: broadcast::Sender<RRTEvent<W::Event>>,
    liveness: Arc<RRTLiveness>,
    waker: Arc<Mutex<Option<Box<dyn RRTWaker>>>>,
) where
    W: RRTWorker,
    W::Event: Clone + Send + 'static,
{
    let _guard = TerminationGuard {
        liveness,
        waker: Arc::clone(&waker),
    };

    let policy = W::restart_policy();
    let mut restart_count: u8 = 0;
    let mut current_delay = policy.initial_delay;

    // Clone tx before the closure so it remains available for panic notification.
    let tx_for_panic = tx.clone();

    // Safety: AssertUnwindSafe is sound here. The closure captures &mut worker, &tx,
    // &waker, &policy, &mut restart_count, and &mut current_delay. After catching a panic
    // we don't touch any of the captured loop state - we only send a Shutdown(Panic)
    // notification via the pre-cloned tx_for_panic and then exit. No
    // potentially-corrupted state is observed or reused.
    let result = catch_unwind(AssertUnwindSafe(|| {
        loop {
            match worker.poll_once(&tx) {
                Continuation::Continue => {}

                Continuation::Stop => break,

                Continuation::Restart => {
                    // Inner retry loop: handles both "restart worker" and "W::create()
                    // itself failed" cases.
                    let exhausted = loop {
                        restart_count += 1;
                        if restart_count > policy.max_restarts {
                            drop(tx.send(RRTEvent::Shutdown(
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
                                // Box the concrete waker for type erasure.
                                if let Ok(mut guard) = waker.lock() {
                                    let boxed: Box<dyn RRTWaker> = Box::new(new_waker);
                                    *guard = Some(boxed);
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
        drop(tx_for_panic.send(RRTEvent::Shutdown(ShutdownReason::Panic)));
    }

    // _guard dropped here, clearing waker + marking terminated.
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

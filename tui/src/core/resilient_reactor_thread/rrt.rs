// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words maxfiles taskthreads rrtwaker

//! Thread lifecycle manager and entry point for the Resilient Reactor Thread pattern.
//! See [`RRT`] for details.

use super::{RRTEvent, RRTWaker, RRTWorker, RestartPolicy, ShutdownReason,
            SubscriberGuard};
use crate::{core::common::Continuation, ok};
use std::{panic::{AssertUnwindSafe, catch_unwind},
          sync::{Arc, LazyLock, Mutex,
                 atomic::{AtomicU8, Ordering}},
          time::Duration};
use tokio::sync::broadcast;

pub type SafeSender<E> = broadcast::Sender<RRTEvent<E>>;
pub type SafeWaker = Arc<Mutex<Option<Box<dyn RRTWaker>>>>;

/// Counter for thread generations. Incremented each time a new thread is spawned.
///
/// Wraps naturally from `255` to `0`.
static THREAD_GENERATION: AtomicU8 = AtomicU8::new(0);

/// Returns the next generation number and increments the counter.
fn next_generation() -> u8 {
    THREAD_GENERATION
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1)
}

/// An indication of whether the dedicated thread is running or terminated.
///
/// Used by [`RRT::is_thread_running()`] to provide a self-documenting return type
/// instead of a bare `bool`.
///
/// # Why Not Just `bool`?
///
/// `bool` requires remembering what `true` means. With this enum:
/// - [`LivenessState::Running`] is unambiguous
/// - Pattern matching catches all cases
/// - Code reads like documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    /// The dedicated thread is running and processing events.
    Running,
    /// The dedicated thread has exited or was never started.
    Terminated,
}

/// The entry point for the Resilient Reactor Thread (RRT) framework.
///
/// This struct manages the lifecycle of a single dedicated thread (at most one at a time)
/// with automatic spawn/shutdown/reuse semantics. It is a **static container** with three
/// top-level fields, each using a synchronization primitive that matches its lifetime -
/// see each field's documentation for details:
///
/// - **[`broadcast_sender`]**: [`broadcast channel`] lazily initialized on first access
///   via [`LazyLock`], never replaced.
///   - [`tokio::sync::broadcast::channel()`] is not a [const expression], so it can't
///     live in the `static` [`SINGLETON`] declaration directly.
///   - [`LazyLock`] handles this transparently by deferring creation to first access. The
///     [`broadcast channel`] outlives every thread generation, so old and new subscribers
///     always share the same channel.
///
/// - **[`safe_waker`]**: The [`Arc<Mutex<...>>`] wrapper is also lazily initialized via
///   [`LazyLock`] (same rationale as [`broadcast_sender`]).
///   - The *inner* `Option<Box<dyn RRTWaker>>` is swapped on each [thread relaunch] and
///     cleared to [`None`] when the thread dies.
///   - Because every [`SubscriberGuard`] holds a clone of the same [`Arc`], old and new
///     subscribers always read the *current* waker.
///
/// - **[`generation`]**: Per-thread-generation counter (unlike the singleton-lifetime
///   channel and waker).
///   - Each [thread relaunch] stores a new generation number via [`AtomicU8`].
///   - The [`safe_waker`]'s [`Option`] state serves as the liveness signal: `Some(waker)`
///     = running, `None` = terminated.
///
/// # Thread Lifecycle
///
/// A typical lifetime progresses through these phases:
///
/// 1. **Before first [`subscribe()`]** - channel and waker are uninitialized
///    ([`LazyLock`] defers creation until first access), liveness is `None`.
/// 2. **First [`subscribe()`]** - initializes the channel and waker wrapper, spawns the
///    dedicated thread, and records a new generation.
/// 3. **While running** - [`RRTWorker`] polls in a loop. If it returns
///    [`Continuation::Restart`], the worker is replaced in-place (thread stays alive,
///    subscribers unaffected). See [self-healing restart details].
/// 4. **Thread exits** - when all subscribers drop, the thread exits:
///    [`TerminationGuard`] clears the waker to [`None`] (which IS the terminated signal).
/// 5. **Next [`subscribe()`]** - detects waker is [`None`], spawns a fresh thread,
///    installs a new waker, and records a new generation. The cycle repeats from step 3.
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
/// [`AtomicU8`]: std::sync::atomic::AtomicU8
/// [`Continuation::Restart`]: crate::Continuation::Restart
/// [`LazyLock`]: std::sync::LazyLock
/// [`Option<W::Waker>`]: super::RRTWaker
/// [`Poll`]: mio::Poll
/// [`RRTWaker`]: super::RRTWaker
/// [`RRTWorker`]: super::RRTWorker
/// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
/// [`String`]: std::string::String
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`TerminationGuard`]: TerminationGuard
/// [`Vec<u8>`]: std::vec::Vec
/// [`Waker`]: mio::Waker
/// [`broadcast channel`]: tokio::sync::broadcast::channel
/// [`broadcast_sender`]: field@Self::broadcast_sender
/// [`generation`]: field@Self::generation
/// [`mio::Poll`]: mio::Poll
/// [`safe_waker`]: field@Self::safe_waker
/// [`subscribe()`]: Self::subscribe
/// [`tokio::sync::broadcast::channel()`]: tokio::sync::broadcast::channel
/// [blocking mechanism]: super#understanding-blocking-io
/// [const expression]: #const-expression-vs-const-declaration-vs-static-declaration
/// [self-healing restart details]: super#self-healing-restart-details
/// [thread relaunch]: super#how-it-works
/// [two-phase setup]: super#two-phase-setup
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
    pub broadcast_sender: LazyLock<SafeSender<W::Event>>,

    /// Shared waker wrapper - lazily initialized on first access, inner value swapped
    /// per generation.
    ///
    /// - The [`Arc<Mutex<...>>`] wrapper is created once via [`LazyLock`].
    /// - The inner [`Option<W::Waker>`] is swapped on relaunch (set to
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
    pub safe_waker: LazyLock<SafeWaker>,

    /// Per-thread-generation counter. Incremented each time a new thread is spawned.
    ///
    /// Unlike [`broadcast_sender`] and [`safe_waker`], generation tracking is
    /// per-thread-generation. Each relaunch stores a new generation number via
    /// [`AtomicU8`]. No [`Mutex`] needed - atomic operations are sufficient for a
    /// single counter.
    ///
    /// [`AtomicU8`]: std::sync::atomic::AtomicU8
    /// [`Mutex`]: std::sync::Mutex
    /// [`broadcast_sender`]: field@Self::broadcast_sender
    /// [`safe_waker`]: field@Self::safe_waker
    pub generation: AtomicU8,
}

impl<W: RRTWorker> RRT<W> {
    /// Creates a new uninitialized global state.
    ///
    /// This is a [const expression][const] so it can be used in [static
    /// declarations][const]. See [`SINGLETON`] for a real usage example.
    ///
    /// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
    /// [const]: #const-expression-vs-const-declaration-vs-static-declaration
    #[must_use]
    pub const fn new() -> Self {
        Self {
            broadcast_sender: LazyLock::new(|| broadcast::channel(W::CHANNEL_CAPACITY).0),
            safe_waker: LazyLock::new(|| Arc::new(Mutex::new(None))),
            generation: AtomicU8::new(0),
        }
    }

    /// Allocates a subscription, spawning the dedicated thread if needed.
    ///
    /// # Two Allocation Paths
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
    /// 1. `&*broadcast_sender` - lazily creates channel on first access
    /// 2. `&*safe_waker` - lazily creates wrapper on first access
    /// 3. [`RRTWorker::create()`] - creates fresh worker + waker pair
    /// 4. Swap waker: `*safe_waker.lock() = Some(new_waker)`
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
    /// [`RRTWorker::create()`]: RRTWorker::create
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [race condition]: super#the-inherent-race-condition
    pub fn subscribe(&self) -> Result<SubscriberGuard<W::Event>, SubscribeError> {
        // Lazily initialized on first access - channel created once, never replaced.
        let sender = &*self.broadcast_sender;

        // Lazily initialized on first access - waker wrapper created once. Subsequent
        // thread relaunches swap the *inner* waker value, but the wrapper persists.
        let safe_waker = &*self.safe_waker;

        let mut waker_guard = safe_waker
            .lock()
            .map_err(|_| SubscribeError::MutexPoisoned { which: "waker" })?;

        // FAST PATH: waker is Some -> thread is running -> reuse.
        if waker_guard.is_some() {
            drop(waker_guard);
            let maybe_receiver = Some(sender.subscribe());
            let safe_waker = safe_waker.clone();
            return ok!((maybe_receiver, safe_waker).into());
        }

        // SLOW PATH: waker is None -> thread terminated (or never started).
        // Hold the waker lock for the entire slow path to serialize concurrent
        // subscribe() calls.

        // Create worker and waker atomically.
        // See: "Two-Phase Setup" section in mod.rs.
        let (worker, new_waker) = W::create().map_err(SubscribeError::WorkerCreation)?;

        // Install new waker while we still hold the lock.
        let boxed: Box<dyn RRTWaker> = Box::new(new_waker);
        *waker_guard = Some(boxed);

        // Record generation.
        let generation = next_generation();
        self.generation.store(generation, Ordering::SeqCst);

        // Spawn worker thread.
        let sender_clone = sender.clone();
        let safe_waker_for_thread = safe_waker.clone();
        std::thread::Builder::new()
            .name(format!("rrt-worker-gen-{generation}"))
            .spawn(move || {
                run_worker_loop::<W>(worker, sender_clone, safe_waker_for_thread);
            })
            .map_err(SubscribeError::ThreadSpawn)?;

        // Drop waker lock before creating SubscriberGuard.
        drop(waker_guard);

        let maybe_receiver = Some(sender.subscribe());
        let safe_waker = safe_waker.clone();

        ok!((maybe_receiver, safe_waker).into())
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
        self.safe_waker
            .lock()
            .ok()
            .map(|guard| {
                if guard.is_some() {
                    LivenessState::Running
                } else {
                    LivenessState::Terminated
                }
            })
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
    pub fn get_receiver_count(&self) -> usize { self.broadcast_sender.receiver_count() }

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
    pub fn get_thread_generation(&self) -> u8 { self.generation.load(Ordering::SeqCst) }

    /// Subscribes to events from an existing thread.
    ///
    /// This is a lightweight operation that creates a new subscriber to the existing
    /// broadcast channel. Use this for additional consumers (logging, debugging, etc.)
    /// after an initial allocation.
    ///
    /// [`subscribe()`]: Self::subscribe
    pub fn subscribe_to_existing(&self) -> SubscriberGuard<W::Event> {
        let sender = &*self.broadcast_sender;
        let safe_waker = &*self.safe_waker;

        let maybe_receiver = Some(sender.subscribe());
        let safe_waker = safe_waker.clone();
        SubscriberGuard {
            maybe_receiver,
            safe_waker,
        }
    }
}

/// [RAII] guard that clears the waker to [`None`] when the dedicated thread's work loop
/// exits.
///
/// The waker's [`Option`] state IS the liveness signal: `Some(waker)` means the thread
/// is running, `None` means it has terminated. Clearing it to `None` is the only cleanup
/// needed - [`subscribe()`] checks `is_none()` to detect termination.
///
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`subscribe()`]: RRT::subscribe
#[allow(missing_debug_implementations)]
pub struct TerminationGuard {
    safe_waker: SafeWaker,
}

impl Drop for TerminationGuard {
    fn drop(&mut self) {
        // Clear waker so no subscriber can call stale wake(), and so subscribe()
        // detects termination via is_none().
        if let Ok(mut guard) = self.safe_waker.lock() {
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
/// and two-tier event model.
///
/// When the loop exits, [`TerminationGuard`] clears the waker to [`None`] so the next
/// [`subscribe()`] call detects termination and spawns a new thread.
///
/// [`Continue`]: Continuation::Continue
/// [`RRTEvent::Shutdown(Panic)`]: ShutdownReason::Panic
/// [`Restart`]: Continuation::Restart
/// [`Stop`]: Continuation::Stop
/// [`W::create()`]: RRTWorker::create
/// [`catch_unwind`]: std::panic::catch_unwind
/// [`poll_once()`]: super::RRTWorker::poll_once
/// [`subscribe()`]: RRT::subscribe
/// [self-healing restart details]: super#self-healing-restart-details
pub fn run_worker_loop<W>(
    mut worker: W,
    sender: SafeSender<W::Event>,
    safe_waker: SafeWaker,
) where
    W: RRTWorker,
    W::Event: Clone + Send + 'static,
{
    let _guard = TerminationGuard {
        safe_waker: safe_waker.clone(),
    };

    let policy = W::restart_policy();
    let mut restart_count: u8 = 0;
    let mut current_delay = policy.initial_delay;

    // Clone sender before the closure so it remains available for panic notification.
    let sender_for_panic = sender.clone();

    // Safety: AssertUnwindSafe is sound here. The closure captures &mut worker, &sender,
    // &safe_waker, &policy, &mut restart_count, and &mut current_delay. After catching a
    // panic we don't touch any of the captured loop state - we only send a
    // Shutdown(Panic) notification via the pre-cloned sender_for_panic and then exit.
    // No potentially-corrupted state is observed or reused.
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
                                // Box the concrete waker for type erasure.
                                if let Ok(mut guard) = safe_waker.lock() {
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread-safe global state manager for the Resilient Reactor Thread pattern. See
//! [`RRT`] for details.

use super::{LivenessState, RRTFactory, RRTLiveness, RRTWaker, RRTWorker, SubscriberGuard};
use crate::core::common::Continuation;
use miette::{Context, IntoDiagnostic, Report};
use std::sync::{Arc, Mutex, OnceLock};

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
/// This struct manages the lifecycle of a single dedicated thread (at most one at a
/// time) with automatic spawn/shutdown/reuse semantics. It is a **static container**
/// with three top-level fields, each using the synchronization primitive matching its
/// lifetime:
///
/// - **`broadcast_tx`** ([`OnceLock`]): Created once on first [`subscribe()`], never
///   replaced. The broadcast channel outlives every thread generation, so old and new
///   subscribers always share the same channel.
///
/// - **`waker`** ([`OnceLock<Arc<Mutex<Option<W>>>>`]): The `Arc<Mutex<...>>` wrapper is
///   created once (via [`OnceLock`]). The *inner* `Option<W>` is swapped on each
///   relaunch (set to `Some(new_waker)`) and cleared to `None` when the thread dies.
///   Because every [`SubscriberGuard`] holds a clone of the same [`Arc`], old and new
///   subscribers always read the *current* waker - solving the zombie thread bug where
///   old subscribers would call a stale waker targeting a dead [`mio::Poll`].
///
/// - **`liveness`** ([`Mutex<Option<Arc<RRTLiveness>>>`]): Per-thread-generation.
///   Replaced on each relaunch, cleared when the thread exits.
///
/// # Why [`OnceLock`] for `broadcast_tx` and `waker`?
///
/// [`tokio::sync::broadcast::channel()`] is not a [`const expression`] - it allocates
/// at runtime, so it can't be initialized in the `static` [`SINGLETON`] declaration.
/// [`OnceLock`] bridges this gap: the `static` is initialized with empty [`OnceLock`]s
/// (which *are* const), and the actual channel/waker wrapper is created lazily on the
/// first [`subscribe()`] call.
///
/// # Why [`Mutex<Option<Arc<RRTLiveness>>>`] for `liveness`?
///
/// Unlike the channel and waker wrapper, liveness state is per-thread-generation. Each
/// relaunch creates a fresh [`RRTLiveness`] (with an incremented generation counter).
/// Using [`Mutex<Option<...>>`] allows [`subscribe()`] to atomically check and replace
/// the liveness state.
///
/// # Thread Lifecycle
///
/// Lifecycle states:
/// - **Inert** (all empty) - until first [`subscribe()`] spawns the dedicated thread
/// - **Active** (all populated) - while thread is running
/// - **Dormant** (liveness terminated, waker cleared to `None`) - when all subscribers
///   drop and thread exits
/// - **Reactivates** - on next [`subscribe()`] call (spawns fresh thread, swaps waker,
///   replaces liveness)
///
/// # Usage
///
/// See [`SINGLETON`] for the real implementation used by the terminal input system.
///
/// [`Arc`]: std::sync::Arc
/// [`OnceLock`]: std::sync::OnceLock
/// [`RRTLiveness`]: super::RRTLiveness
/// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
/// [`SubscriberGuard`]: super::SubscriberGuard
/// [`const expression`]: #const-expression-vs-const-declaration-vs-static-declaration
/// [`mio::Poll`]: mio::Poll
/// [`subscribe()`]: Self::subscribe
/// [`tokio::sync::broadcast::channel()`]: tokio::sync::broadcast::channel
///
/// ## `const` Expression vs `const` Declaration vs `static` Declaration
///
/// These are different concepts that share the `const` keyword:
///
/// | Term                     | Meaning                                        | Example                                |
/// | :----------------------- | :--------------------------------------------- | :------------------------------------- |
/// | **`const` expression**   | Value the compiler can compute at compile time | `1 + 2`, `Mutex::new(None)`            |
/// | **`const fn`**           | Function callable in const context             | `const fn new() -> Option<T> { None }` |
/// | **`const` declaration**  | Inlined constant (no fixed address)            | `const PI: f64 = 3.14;`               |
/// | **`static` declaration** | Fixed address, single instance (singleton)     | `static GLOBAL: T = …;`               |
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
/// | Context                 | Syntax Example | Meaning                                             |
/// | :---------------------- | :------------- | :-------------------------------------------------- |
/// | **Lifetime annotation** | `&'static str` | Reference valid for entire program (data in binary) |
/// | **Trait bound**         | `T: 'static`   | Type contains no references shorter than `'static`  |
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
/// For thread spawning, `T: 'static` is required because the spawned thread could
/// outlive the caller - any borrowed data with a shorter lifetime might become invalid.
/// This is why [`RRTWaker`], [`RRTWorker`], and the `E` (event) type parameter all
/// require `'static`.
///
/// # Poll -> Registry -> Waker Chain
///
/// Your [`RRTWaker`] implementation is tightly coupled to its blocking mechanism (e.g.,
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
/// This is why the slow path replaces **both** Poll and Waker together - your
/// [`RRTWaker`] implementation is useless without its parent blocking mechanism.
///
/// [`RRTWaker`]: super::RRTWaker
/// [`RRTWorker`]: super::RRTWorker
/// [`String`]: std::string::String
/// [`Vec<u8>`]: std::vec::Vec
/// [`mio::Poll`]: mio::Poll
/// [process]: https://en.wikipedia.org/wiki/Process_(computing)
#[allow(missing_debug_implementations)]
pub struct RRT<F>
where
    F: RRTFactory,
    F::Waker: RRTWaker,
    F::Event: Clone + Send + Sync + 'static,
{
    /// Broadcast channel sender - created once, never replaced.
    ///
    /// [`OnceLock`] because [`tokio::sync::broadcast::channel()`] is not a const
    /// expression.
    broadcast_tx: OnceLock<tokio::sync::broadcast::Sender<F::Event>>,

    /// Shared waker wrapper - the [`Arc<Mutex<...>>`] is created once via [`OnceLock`].
    /// The inner `Option<F::Waker>` is swapped on relaunch and cleared to `None` when
    /// the thread dies. All [`SubscriberGuard`]s hold a clone of this [`Arc`], so they
    /// always read the current waker.
    ///
    /// [`Arc`]: std::sync::Arc
    /// [`OnceLock`]: std::sync::OnceLock
    /// [`SubscriberGuard`]: super::SubscriberGuard
    waker: OnceLock<Arc<Mutex<Option<F::Waker>>>>,

    /// Per-thread-generation liveness tracking. Replaced on each relaunch.
    liveness: Mutex<Option<Arc<RRTLiveness>>>,
}

impl<F> RRT<F>
where
    F: RRTFactory,
    F::Waker: RRTWaker,
    F::Event: Clone + Send + Sync + 'static,
{
    /// Creates a new uninitialized global state.
    ///
    /// This is a `const fn` so it can be used in `static` declarations.
    /// See [`SINGLETON`] for a real usage example.
    ///
    /// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
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
    /// | Condition                | Path          | What Happens                    |
    /// | ------------------------ | ------------- | ------------------------------- |
    /// | `liveness == Running`    | **Fast path** | Reuse existing thread           |
    /// | `liveness == Terminated` | **Slow path** | Spawn new thread, swap waker    |
    ///
    /// ## Fast Path (Thread Reuse)
    ///
    /// If the thread is still running, we **reuse everything**:
    /// - Same broadcast channel (singleton-lifetime, never replaced)
    /// - Same liveness tracker (still valid for this generation)
    /// - Same thread (continues serving the new subscriber)
    ///
    /// This handles the race condition where a new subscriber appears before the thread
    /// checks [`receiver_count()`].
    ///
    /// ## Slow Path (Thread Restart)
    ///
    /// If the thread has terminated (or never started):
    /// 1. `broadcast_tx.get_or_init(...)` - idempotent channel creation
    /// 2. `waker.get_or_init(...)` - idempotent wrapper creation
    /// 3. [`RRTFactory::create()`] - creates fresh worker + waker pair
    /// 4. Swap waker: `*shared_waker.lock() = Some(new_waker)`
    /// 5. Create fresh [`RRTLiveness`], spawn thread
    ///
    /// The broadcast channel and waker wrapper are **never replaced** - only the
    /// inner waker value and liveness state change on relaunch.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`RRTFactory::create()`] fails (OS resource creation failed)
    /// - The mutex is poisoned (another thread panicked while holding the lock)
    /// - Thread spawning fails (system thread limits)
    ///
    /// [`RRTFactory::create()`]: RRTFactory::create
    /// [`RRTLiveness`]: super::RRTLiveness
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    pub fn subscribe(&self) -> Result<SubscriberGuard<F::Waker, F::Event>, Report> {
        // Initialize channel and waker wrapper idempotently (created once, read forever).
        let tx = self
            .broadcast_tx
            .get_or_init(|| tokio::sync::broadcast::channel(CHANNEL_CAPACITY).0);
        let shared_waker = self
            .waker
            .get_or_init(|| Arc::new(Mutex::new(None)));

        let mut liveness_guard = self
            .liveness
            .lock()
            .map_err(|_| miette::miette!("RRT liveness mutex poisoned"))?;

        // FAST PATH: Reuse existing thread.
        let is_running = liveness_guard
            .as_ref()
            .is_some_and(|liveness| liveness.is_running() == LivenessState::Running);

        // SLOW PATH: Thread terminated (or never started) -> create fresh.
        if !is_running {
            // Explicitly clear stale liveness (if any).
            drop(liveness_guard.take());

            // Create worker and waker atomically (see the "Coupled Resource Creation"
            // problem in mod.rs).
            let (worker, new_waker) =
                F::create().context("Failed to create worker thread resources")?;

            // Swap waker: old subscribers now read the new waker.
            {
                let mut waker_guard = shared_waker
                    .lock()
                    .map_err(|_| miette::miette!("RRT waker mutex poisoned"))?;
                *waker_guard = Some(new_waker);
            }

            // Create fresh liveness for this generation.
            let liveness = Arc::new(RRTLiveness::new());

            // Spawn worker thread.
            let tx_clone = tx.clone();
            let liveness_for_thread = Arc::clone(&liveness);
            let waker_for_thread = Arc::clone(shared_waker);
            std::thread::Builder::new()
                .name(format!(
                    "rrt-worker-gen-{}",
                    liveness.generation
                ))
                .spawn(move || {
                    run_worker_loop(worker, tx_clone, liveness_for_thread, waker_for_thread);
                })
                .into_diagnostic()
                .context("Failed to spawn worker thread")?;

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
    pub fn subscribe_to_existing(&self) -> SubscriberGuard<F::Waker, F::Event> {
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

impl<F> Default for RRT<F>
where
    F: RRTFactory,
    F::Waker: RRTWaker,
    F::Event: Clone + Send + Sync + 'static,
{
    fn default() -> Self { Self::new() }
}

/// RAII guard that clears the waker and calls [`mark_terminated()`] when the dedicated
/// thread's work loop exits.
///
/// **Drop ordering matters**: The waker is cleared to `None` *before* marking
/// terminated. If we marked terminated first, [`subscribe()`] could race in, install a
/// new waker, and our cleanup would clear it.
///
/// [`mark_terminated()`]: super::RRTLiveness::mark_terminated
/// [`subscribe()`]: RRT::subscribe
#[allow(missing_debug_implementations)]
pub struct TerminationGuard<W>
where
    W: RRTWaker,
{
    liveness: Arc<RRTLiveness>,
    waker: Arc<Mutex<Option<W>>>,
}

impl<W> Drop for TerminationGuard<W>
where
    W: RRTWaker,
{
    fn drop(&mut self) {
        // Clear waker FIRST so no subscriber can call stale wake().
        // Order matters: if we mark_terminated() first, subscribe() could
        // race in, install a new waker, and our cleanup would clear it.
        if let Ok(mut guard) = self.waker.lock() {
            *guard = None;
        }
        self.liveness.mark_terminated();
    }
}

/// Runs the poll loop on the dedicated thread until it returns [`Continuation::Stop`].
///
/// Called from the spawned dedicated thread. When the loop exits, [`TerminationGuard`]
/// clears the waker to `None` and calls [`mark_terminated()`] so the next
/// [`subscribe()`] call knows to spawn a new thread.
///
/// [`mark_terminated()`]: super::RRTLiveness::mark_terminated
/// [`subscribe()`]: RRT::subscribe
pub fn run_worker_loop<W, E>(
    mut worker: impl RRTWorker<Event = E>,
    tx: tokio::sync::broadcast::Sender<E>,
    liveness: Arc<RRTLiveness>,
    waker: Arc<Mutex<Option<W>>>,
) where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    let _guard = TerminationGuard { liveness, waker };
    while worker.poll_once(&tx) == Continuation::Continue {}
    // _guard dropped here (or during unwinding), clearing waker + marking terminated.
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread-safe global state manager for the Resilient Reactor Thread pattern. See
//! [`RRT`] for details.

use super::{LivenessState, RRTFactory, RRTState, RRTWaker, RRTWorker, SubscriberGuard};
use crate::core::common::Continuation;
use miette::{Context, IntoDiagnostic, Report};
use std::sync::{Arc, Mutex};

/// The entry point for the Resilient Reactor Thread (RRT) framework.
///
/// This struct manages the lifecycle of a dedicated worker thread with automatic
/// spawn/shutdown/reuse semantics. This is a **static container** that holds an
/// **ephemeral payload**:
///
/// - **Container** ([`Mutex<Option<_>>`]): Lives for [process] lifetime
/// - **Payload** ([`RRTState`]): Created when thread spawns, destroyed when it exits
///
/// The container persists, but the payload comes and goes with the thread lifecycle.
///
/// # Why [`Mutex<Option<Arc<RRTState<W, E>>>>`]?
///
/// **Deferred initialization** — we can't create [`RRTState`] at `static` init time:
///
/// | Operation              | Const? | Why not?                                      |
/// | :--------------------- | :----- | :-------------------------------------------- |
/// | [`mio::Poll::new()`]   | No     | [`Syscall`] (creates [`epoll`]/kqueue [`fd`]) |
/// | [`mio::Waker::new()`]  | No     | Requires Poll's registry (see below)          |
/// | [`Arc::new()`]         | No     | Heap allocation                               |
///
/// ## Why [`syscalls`] Can't Be [`const expressions`]
///
/// In Rust, **all** `static` variables must be initialized with [`const expressions`] —
/// this is a language rule, not a choice. The compiler evaluates these expressions at
/// compile time and embeds the result in the binary. [`Syscalls`] ask the OS to do
/// something (create an [`epoll`] [`fd`], allocate memory), which is impossible during
/// compilation. The OS doesn't exist at compile time, and these operations have side
/// effects that can't be "undone."
///
/// Since [`Mutex::new(None)`] **is** a [`const expression`] (just initializes memory
/// layout), we use [`Option<T>`] to defer the [`syscalls`] until the first
/// [`subscribe()`] call at runtime.
///
/// ## Replacement On Restart
///
/// When the thread terminates and restarts (slow path), we need to replace the entire
/// [`RRTState`] with fresh resources. [`Option::replace()`] makes this clean.
///
/// ## Fallibility Is NOT The Reason
///
/// We panic on [`syscall`] failure anyway. Even if these operations were infallible, we'd
/// still need [`Option<T>`] because they're not [`const expressions`].
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
/// | **`static` declaration** | Fixed address, single instance (singleton)     | `static GLOBAL: T = …;`                |
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
/// [`String`] satisfies `T: 'static` even though it can be dropped at any time — the
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
/// This fails to compile — `&String` has a non-`'static` lifetime:
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
/// the caller — any borrowed data with a shorter lifetime might become invalid. This is
/// why [`RRTWaker`], [`RRTWorker`], and the `E` (event) type parameter all require
/// `'static`.
///
/// # Poll → Registry → Waker Chain
///
/// The waker is tightly coupled to its blocking mechanism (e.g., [`mio::Poll`]):
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
/// This is why the slow path replaces **both** Poll and Waker together — a waker is
/// useless without its parent blocking mechanism.
///
/// # Thread Lifecycle
///
/// Lifecycle states:
/// - **Inert** (`None`) — until first [`subscribe()`] spawns the worker thread
/// - **Active** (`Some`) — while thread is running
/// - **Dormant** (`Some` with terminated liveness) — when all subscribers drop and thread
///   exits
/// - **Reactivates** — on next [`subscribe()`] call (spawns fresh thread, replaces
///   payload)
///
/// # Usage
///
/// See [`SINGLETON`] for the real implementation used by the terminal input system.
///
/// [`Mutex<Option<_>>`]: std::sync::Mutex
/// [`Vec<u8>`]: std::vec::Vec
/// [`Mutex<Option<Arc<RRTState<W, E>>>>`]: super::RRTState
/// [`Arc::new()`]: std::sync::Arc::new
/// [`Mutex::new(None)`]: std::sync::Mutex::new
/// [`Option::replace()`]: std::option::Option::replace
/// [`RRTState`]: super::RRTState
/// [`RRTWaker`]: super::RRTWaker
/// [`RRTWorker`]: super::RRTWorker
/// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
/// [`String`]: std::string::String
/// [`Syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`Syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`const expression`]: #const-expression-vs-const-declaration-vs-static-declaration
/// [`const expressions`]: #const-expression-vs-const-declaration-vs-static-declaration
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
/// [`mio::Poll::new()`]: mio::Poll::new
/// [`mio::Poll`]: mio::Poll
/// [`mio::Waker::new()`]: mio::Waker::new
/// [`subscribe()`]: Self::subscribe
/// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [process]: https://en.wikipedia.org/wiki/Process_(computing)
#[allow(missing_debug_implementations, clippy::type_complexity)]
pub struct RRT<F>
where
    F: RRTFactory,
    F::Waker: RRTWaker,
    F::Event: Clone + Send + Sync + 'static,
{
    pub inner: Mutex<Option<Arc<RRTState<F::Waker, F::Event>>>>,
}

impl<F> RRT<F>
where
    F: RRTFactory,
    F::Waker: RRTWaker,
    F::Event: Clone + Send + Sync + 'static,
{
    /// Creates a new uninitialized global state.u
    ///
    /// This is a `const fn` so it can be used in `static` declarations.
    /// See [`SINGLETON`] for a real usage example.
    ///
    /// [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Allocates a subscription, spawning the worker thread if needed.
    ///
    /// # Two Allocation Paths
    ///
    /// | Condition                | Path          | What Happens                         |
    /// | ------------------------ | ------------- | ------------------------------------ |
    /// | `liveness == Running`    | **Fast path** | Reuse existing thread + [`RRTState`] |
    /// | `liveness == Terminated` | **Slow path** | Replace all, spawn new thread        |
    ///
    /// ## Fast Path (Thread Reuse)
    ///
    /// If the thread is still running, we **reuse everything**:
    /// - Same [`RRTState`] (same broadcast channel, same liveness tracker)
    /// - Same worker resources (still valid)
    /// - Same thread (continues serving the new subscriber)
    ///
    /// This handles the race condition where a new subscriber appears before the thread
    /// checks [`receiver_count()`].
    ///
    /// ## Slow Path (Thread Restart)
    ///
    /// If the thread has terminated, the existing [`RRTState`] is **orphaned** — no
    /// thread is feeding events into its broadcast channel. We must **replace
    /// everything**:
    /// - New [`RRTState`] (fresh broadcast channel + liveness tracker + waker)
    /// - New worker resources (via [`RRTFactory::create()`])
    /// - New thread (spawned to serve the new subscriber)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`RRTFactory::create()`] fails (OS resource creation failed)
    /// - The mutex is poisoned (another thread panicked while holding the lock)
    /// - Thread spawning fails (system thread limits)
    ///
    /// [`RRTFactory::create()`]: RRTFactory::create
    /// [`RRTState`]: super::RRTState
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    pub fn subscribe(&self) -> Result<SubscriberGuard<F::Waker, F::Event>, Report> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| miette::miette!("RRT mutex poisoned"))?;

        // Fast path check: can we reuse the existing thread + RRTState?
        let apply_fast_path_thread_reuse = guard
            .as_ref()
            .is_some_and(|state| state.liveness.is_running() == LivenessState::Running);

        // SLOW PATH: Thread terminated (or never started) → create new everything.
        if !apply_fast_path_thread_reuse {
            // Create worker and waker atomically (see the "Coupled Resource Creation"
            // problem in mod.rs).
            let (worker, waker) =
                F::create().context("Failed to create worker thread resources")?;

            // Create new RRTState with the waker.
            let thread_state = Arc::new(RRTState::new(waker));

            // Spawn worker thread.
            let tx_clone = thread_state.broadcast_tx.clone();
            let liveness_clone = Arc::clone(&thread_state);
            std::thread::Builder::new()
                .name(format!(
                    "rrt-worker-gen-{}",
                    thread_state.liveness.generation
                ))
                .spawn(move || {
                    run_worker_loop(worker, tx_clone, liveness_clone);
                })
                .into_diagnostic()
                .context("Failed to spawn worker thread")?;

            // Replace the old (orphaned) state with the new one.
            guard.replace(thread_state);
        }

        // FAST PATH (or after slow path): Use the current RRTState.
        // Invariant: guard is always Some here because:
        // - Slow path: we just called guard.replace(thread_state)
        // - Fast path: apply_fast_path_thread_reuse was true, so
        //   guard.as_ref().is_some_and() returned true
        let thread_state = guard.as_ref().ok_or_else(|| {
            miette::miette!("Invariant violated: RRTState missing after allocation")
        })?;

        Ok(SubscriberGuard {
            receiver: Some(thread_state.broadcast_tx.subscribe()),
            state: Arc::clone(thread_state),
        })
    }

    /// Checks if the worker thread is currently running.
    ///
    /// Useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// - [`LivenessState::Running`] if the thread is running
    /// - [`LivenessState::Terminated`] if uninitialized or the thread has exited
    #[allow(clippy::redundant_closure_for_method_calls)]
    #[must_use]
    pub fn is_thread_running(&self) -> LivenessState {
        self.inner
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|state| state.liveness.is_running()))
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
        self.inner
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .as_ref()
                    .map(|state| state.broadcast_tx.receiver_count())
            })
            .unwrap_or(0)
    }

    /// Returns the current thread generation number.
    ///
    /// Each time a new worker thread is spawned, the generation increments. This allows
    /// tests to verify whether a thread was reused or relaunched:
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
        self.inner
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|state| state.liveness.generation))
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
    /// - If the mutex is poisoned
    /// - If no thread exists yet (call [`subscribe()`] first)
    ///
    /// [`subscribe()`]: Self::subscribe
    pub fn subscribe_to_existing(&self) -> SubscriberGuard<F::Waker, F::Event> {
        let guard = self.inner.lock().expect(
            "RRT mutex poisoned: another thread panicked while \
             holding this lock.",
        );

        let thread_state = guard.as_ref().expect(
            "subscribe_to_existing() called before subscribe(). \
             Subscribe first to create the thread, then add more subscribers.",
        );

        SubscriberGuard {
            receiver: Some(thread_state.broadcast_tx.subscribe()),
            state: Arc::clone(thread_state),
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

/// RAII guard that calls [`mark_terminated()`] when the worker loop exits.
///
/// [`mark_terminated()`]: super::RRTLiveness::mark_terminated
#[allow(missing_debug_implementations)]
pub struct TerminationGuard<W, E>
where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    state: Arc<RRTState<W, E>>,
}

impl<W, E> Drop for TerminationGuard<W, E>
where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    fn drop(&mut self) { self.state.liveness.mark_terminated(); }
}

/// Runs the worker's poll loop until it returns [`Continuation::Stop`].
///
/// Called from the spawned worker thread. When the loop exits, [`TerminationGuard`]
/// calls [`mark_terminated()`] so the next [`subscribe()`] call knows to spawn a new
/// thread.
///
/// [`mark_terminated()`]: super::RRTLiveness::mark_terminated
/// [`subscribe()`]: RRT::subscribe
pub fn run_worker_loop<W, E>(
    mut worker: impl RRTWorker<Event = E>,
    tx: tokio::sync::broadcast::Sender<E>,
    state: Arc<RRTState<W, E>>,
) where
    W: RRTWaker,
    E: Clone + Send + 'static,
{
    let _guard = TerminationGuard { state };
    while worker.poll_once(&tx) == Continuation::Continue {}
    // _guard dropped here (or during unwinding), calling mark_terminated()
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread-safe global state manager for the Resilient Reactor Thread pattern.
//!
//! [`ThreadSafeGlobalState`] is a **static container** that holds an **ephemeral
//! payload**:
//!
//! - **Container** ([`Mutex<Option<_>>`]): Lives for [process] lifetime
//! - **Payload** ([`ThreadState`]): Created when thread spawns, destroyed when it exits
//!
//! The container persists, but the payload comes and goes with the thread lifecycle.
//!
//! [`Mutex<Option<_>>`]: std::sync::Mutex
//!
//! [`ThreadState`]: super::ThreadState
//! [process]: https://en.wikipedia.org/wiki/Process_(computing)

use super::{LivenessState, SubscriberGuard, ThreadState, ThreadWaker, ThreadWorker,
            ThreadWorkerFactory};
use crate::core::common::Continuation;
use miette::{Context, IntoDiagnostic, Report};
use std::sync::{Arc, Mutex};

/// Thread-safe global state for a Resilient Reactor Thread.
///
/// Manages the lifecycle of a dedicated worker thread with automatic spawn/shutdown/reuse
/// semantics.
///
/// # Why [`Mutex<Option<Arc<ThreadState<W, E>>>>`]?
///
/// **Deferred initialization** — we can't create [`ThreadState`] at `static` init time:
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
/// layout), we use [`Option<T>`] to defer the [`syscalls`] until the first [`allocate()`]
/// call at runtime.
///
/// ## Replacement On Restart
///
/// When the thread terminates and restarts (slow path), we need to replace the entire
/// [`ThreadState`] with fresh resources. [`Option::replace()`] makes this clean.
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
/// | **Trait bound**         | `T: 'static`   | Type contains no non-`'static` references           |
///
/// **Key insight:** [`String`] satisfies `T: 'static` even though a [`String`] can be
/// dropped at any time. The bound means "doesn't contain references that could become
/// invalid," not "lives forever."
///
/// | Type           | Satisfies `T: 'static`? | Why?                                  |
/// | :------------- | :---------------------- | :------------------------------------ |
/// | [`String`]     | Yes                     | Owned, no references                  |
/// | [`Vec<u8>`]    | Yes                     | Owned, no references                  |
/// | `&'static str` | Yes                     | Reference is `'static`                |
/// | `&'a str`      | No                      | Contains non-`'static` reference      |
/// | `Foo<'a>`      | No                      | Lifetime parameter implies references |
///
/// For thread spawning, `T: 'static` is required because the spawned thread could outlive
/// the caller — any borrowed data might become invalid. This is why [`ThreadWaker`],
/// [`ThreadWorker`], and the `E` (event) type parameter all require `'static`.
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
/// - **Inert** (`None`) — until first [`allocate()`] spawns the worker thread
/// - **Active** (`Some`) — while thread is running
/// - **Dormant** (`Some` with terminated liveness) — when all subscribers drop and thread
///   exits
/// - **Reactivates** — on next [`allocate()`] call (spawns fresh thread, replaces
///   payload)
///
/// # Usage
///
/// ```no_run
/// # use r3bl_tui::core::resilient_reactor_thread::*;
/// # use r3bl_tui::Continuation;
/// # use tokio::sync::broadcast::Sender;
/// #
/// # #[derive(Clone)]
/// # struct MyEvent;
/// # #[derive(Debug)]
/// # struct MyError;
/// # impl std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "error") }
/// # }
/// # impl std::error::Error for MyError {}
/// # struct MyWaker;
/// # impl ThreadWaker for MyWaker {
/// #     fn wake(&self) -> std::io::Result<()> { Ok(()) }
/// # }
/// # struct MyWorker;
/// # impl ThreadWorker for MyWorker {
/// #     type Event = MyEvent;
/// #     fn poll_once(&mut self, _tx: &Sender<Self::Event>) -> Continuation { todo!() }
/// # }
/// # struct MyFactory;
/// # impl ThreadWorkerFactory for MyFactory {
/// #     type Event = MyEvent;
/// #     type Worker = MyWorker;
/// #     type Waker = MyWaker;
/// #     type SetupError = MyError;
/// #     fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError> { todo!() }
/// # }
/// // Define a static global state for your RRT implementation
/// static GLOBAL: ThreadSafeGlobalState<MyWaker, MyEvent> =
///     ThreadSafeGlobalState::new();
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Allocate a subscription (spawns thread if needed)
/// let guard = GLOBAL.allocate::<MyFactory>()?;
///
/// // Query state (for testing/debugging)
/// let running = GLOBAL.is_thread_running();
/// let count = GLOBAL.get_receiver_count();
/// let generation = GLOBAL.get_thread_generation();
/// # Ok(())
/// # }
/// ```
///
/// [`Vec<u8>`]: std::vec::Vec
/// [`Mutex<Option<Arc<ThreadState<W, E>>>>`]: super::ThreadState
/// [`Arc::new()`]: std::sync::Arc::new
/// [`Mutex::new(None)`]: std::sync::Mutex::new
/// [`Option::replace()`]: std::option::Option::replace
/// [`String`]: std::string::String
/// [`Syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`Syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`ThreadState`]: super::ThreadState
/// [`ThreadWaker`]: super::ThreadWaker
/// [`ThreadWorker`]: super::ThreadWorker
/// [`allocate()`]: Self::allocate
/// [`const expression`]: #const-expression-vs-const-declaration-vs-static-declaration
/// [`const expressions`]: #const-expression-vs-const-declaration-vs-static-declaration
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`mio::Poll::new()`]: mio::Poll::new
/// [`mio::Poll`]: mio::Poll
/// [`mio::Waker::new()`]: mio::Waker::new
/// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
#[allow(missing_debug_implementations)]
pub struct ThreadSafeGlobalState<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    inner: Mutex<Option<Arc<ThreadState<W, E>>>>,
}

impl<W, E> ThreadSafeGlobalState<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    /// Creates a new uninitialized global state.
    ///
    /// This is a `const fn` so it can be used in `static` declarations:
    ///
    /// ```no_run
    /// # use r3bl_tui::core::resilient_reactor_thread::{ThreadSafeGlobalState, ThreadWaker};
    /// #
    /// # struct MyWaker;
    /// # impl ThreadWaker for MyWaker {
    /// #     fn wake(&self) -> std::io::Result<()> { Ok(()) }
    /// # }
    /// # #[derive(Clone)]
    /// # struct MyEvent;
    /// static GLOBAL: ThreadSafeGlobalState<MyWaker, MyEvent> =
    ///     ThreadSafeGlobalState::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Allocate a subscription, spawning the worker thread if needed.
    ///
    /// # Two Allocation Paths
    ///
    /// | Condition                | Path          | What Happens                              |
    /// | ------------------------ | ------------- | ----------------------------------------- |
    /// | `liveness == Running`    | **Fast path** | Reuse existing thread + [`ThreadState`]   |
    /// | `liveness == Terminated` | **Slow path** | Replace all, spawn new thread             |
    ///
    /// ## Fast Path (Thread Reuse)
    ///
    /// If the thread is still running, we **reuse everything**:
    /// - Same [`ThreadState`] (same broadcast channel, same liveness tracker)
    /// - Same worker resources (still valid)
    /// - Same thread (continues serving the new subscriber)
    ///
    /// This handles the race condition where a new subscriber appears before the thread
    /// checks [`receiver_count()`].
    ///
    /// ## Slow Path (Thread Restart)
    ///
    /// If the thread has terminated, the existing [`ThreadState`] is **orphaned** — no
    /// thread is feeding events into its broadcast channel. We must **replace
    /// everything**:
    /// - New [`ThreadState`] (fresh broadcast channel + liveness tracker + waker)
    /// - New worker resources (via [`ThreadWorkerFactory::setup()`])
    /// - New thread (spawned to serve the new subscriber)
    ///
    /// # Type Parameters
    ///
    /// - `F`: The [`ThreadWorkerFactory`] implementation that creates the worker and
    ///   waker
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`ThreadWorkerFactory::setup()`] fails (OS resource creation failed)
    /// - The mutex is poisoned (another thread panicked while holding the lock)
    /// - Thread spawning fails (system thread limits)
    ///
    /// [`ThreadState`]: super::ThreadState
    /// [`ThreadWorkerFactory::setup()`]: ThreadWorkerFactory::setup
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    pub fn allocate<F>(&self) -> Result<SubscriberGuard<W, E>, Report>
    where
        F: ThreadWorkerFactory<Waker = W, Event = E>,
    {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| miette::miette!("ThreadSafeGlobalState mutex poisoned"))?;

        // Fast path check: can we reuse the existing thread + ThreadState?
        let apply_fast_path_thread_reuse = guard
            .as_ref()
            .is_some_and(|state| state.liveness.is_running() == LivenessState::Running);

        // SLOW PATH: Thread terminated (or never started) → create new everything.
        if !apply_fast_path_thread_reuse {
            // Create worker and waker together (solves chicken-egg problem).
            let (worker, waker) = F::setup()
                .into_diagnostic()
                .context("Failed to setup worker thread resources")?;

            // Create new ThreadState with the waker.
            let thread_state = Arc::new(ThreadState::new(waker));

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

        // FAST PATH (or after slow path): Use the current ThreadState.
        // Invariant: guard is always Some here because:
        // - Slow path: we just called guard.replace(thread_state)
        // - Fast path: apply_fast_path_thread_reuse was true, so
        //   guard.as_ref().is_some_and() returned true
        let thread_state = guard.as_ref().ok_or_else(|| {
            miette::miette!("Invariant violated: ThreadState missing after allocation")
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

    /// Subscribe to events from an existing thread.
    ///
    /// This is a lightweight operation that creates a new subscriber to the existing
    /// broadcast channel. Use this for additional consumers (logging, debugging, etc.)
    /// after an initial allocation.
    ///
    /// # Panics
    ///
    /// - If the mutex is poisoned
    /// - If no thread exists yet (call [`allocate()`] first)
    ///
    /// [`allocate()`]: Self::allocate
    pub fn subscribe_to_existing(&self) -> SubscriberGuard<W, E> {
        let guard = self.inner.lock().expect(
            "ThreadSafeGlobalState mutex poisoned: another thread panicked while \
             holding this lock.",
        );

        let thread_state = guard.as_ref().expect(
            "subscribe_to_existing() called before allocate(). \
             Allocate first, then subscribe.",
        );

        SubscriberGuard {
            receiver: Some(thread_state.broadcast_tx.subscribe()),
            state: Arc::clone(thread_state),
        }
    }
}

impl<W, E> Default for ThreadSafeGlobalState<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    fn default() -> Self { Self::new() }
}

/// Runs the worker's poll loop until it returns [`Continuation::Stop`].
///
/// Called from the spawned worker thread. When the loop exits, marks the thread as
/// terminated so the next [`allocate()`] call knows to spawn a new thread.
///
/// # Panic Safety
///
/// Uses a guard pattern to ensure [`mark_terminated()`] is called even if the worker
/// panics. This is critical for proper thread lifecycle management — if a panic occurs,
/// the next [`allocate()`] call must detect the terminated thread and spawn a new one.
///
/// [`allocate()`]: ThreadSafeGlobalState::allocate
/// [`mark_terminated()`]: super::ThreadLiveness::mark_terminated
fn run_worker_loop<W, E>(
    mut worker: impl ThreadWorker<Event = E>,
    tx: tokio::sync::broadcast::Sender<E>,
    state: Arc<ThreadState<W, E>>,
) where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    // Guard ensures mark_terminated is called even on panic (stack unwinding).
    struct TerminationGuard<W, E>
    where
        W: ThreadWaker,
        E: Clone + Send + 'static,
    {
        state: Arc<ThreadState<W, E>>,
    }

    impl<W, E> Drop for TerminationGuard<W, E>
    where
        W: ThreadWaker,
        E: Clone + Send + 'static,
    {
        fn drop(&mut self) { self.state.liveness.mark_terminated(); }
    }

    let _guard = TerminationGuard { state };

    while worker.poll_once(&tx) == Continuation::Continue {}
    // _guard dropped here (or during unwinding), calling mark_terminated()
}

// SAFETY: ThreadSafeGlobalState uses Mutex internally for synchronization.
// The W and E type parameters have Send/Sync bounds enforced by ThreadWaker trait.
unsafe impl<W, E> Sync for ThreadSafeGlobalState<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
}

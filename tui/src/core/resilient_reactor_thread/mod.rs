// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words epoll kqueue SIGWINCH syscall syscalls SQPOLL IORING eventfd signalfd
// cspell:words pollable Proactor demultiplexing injectables threadwaker IOCP EINVAL
// cspell:words kqueuefd filedescriptor rrtwaker

//! Reusable infrastructure for the Resilient Reactor Thread (RRT) pattern implementation.
//!
//! # What is the RRT Pattern?
//!
//! The **Resilient Reactor Thread** pattern bridges [blocking I/O] with async Rust.
//! Calling a blocking [`syscall`] (like [`read(2)`] on [`stdin`]) from an [async task]
//! in a [`tokio`] [async runtime] executor thread **is not ok**:
//!
//! - The blocking [`syscall`] blocks both the task *and* the executor thread
//! - The task hangs until the [`syscall`] unblocks on its own
//! - Reduces runtime throughput (one less worker thread available)
//! - Can cause issues during executor shutdown (if the [`syscall`] never unblocks)
//!
//! The severity depends on the runtime configuration:
//!
//! 1. 🐢 **Multi-threaded runtime** (default): Reduced throughput but other tasks still
//!    run
//! 2. 🧊 **Single-threaded runtime**: Total blockage — nothing else runs
//!
//! RRT avoids all of this and allows async code to consume events from blocking sources
//! ([`stdin`], [`sockets`], [`signals`]) without blocking the [async runtime].
//! It does this by:
//! 1. Isolating [blocking I/O] in a dedicated thread and creating a bridge to async
//!    consumers via a [`broadcast channel`].
//! 2. Managing the lifecycle of this thread - allowing it to be created, interrupted,
//!    terminated, and started again.
//!
//! So, what's in a name? 😛
//!
//! | Word          | Meaning                                                                                                       |
//! | :------------ | :------------------------------------------------------------------------------------------------------------ |
//! | **Resilient** | Thread can stop or crash and restart with generation tracking; subscribers are not affected                   |
//! | **Reactor**   | Reacts to I/O events ([`stdin`], [`sockets`], [`signals`]) using [`mio`]/[`epoll`] using [OS I/O Primitives]  |
//! | **Thread**    | Dedicated thread for [blocking I/O]; graceful shutdown when consumers disconnect; fully managed lifecycle     |
//!
//! ## Mental Model for Web Developers
//!
//! If you're familiar with [Web Workers], RRT solves the same fundamental problem:
//! **keeping blocking work off the main execution context**. In browsers, blocking
//! the main thread freezes the UI. In async Rust, blocking an executor thread
//! starves other tasks.
//!
//! | Web Pattern         | RRT Equivalent                                    |
//! | :------------------ | :------------------------------------------------ |
//! | `new Worker()`      | [`SINGLETON`].[`subscribe()`]                     |
//! | `postMessage()`     | [`tx.send()`]                                     |
//! | `onmessage`         | [`Receiver::recv()`]                              |
//! | `terminate()`       | Drop [`SubscriberGuard`] (auto-cleanup)           |
//!
//! **Key differences:**
//!
//! - **Work type**: [Web Workers] handle CPU-bound work; RRT handles I/O-bound blocking
//!   ([`syscalls`] like reading from [`stdin`] or waiting for [`signals`])
//! - **Delivery**: [Web Workers] are 1:1 (one worker, one consumer); RRT uses 1:N
//!   [`broadcast`] (all UI components need resize events)
//!
//! # Understanding "blocking I/O"
//!
//! The RRT pattern's core invariant is that the [worker] thread **blocks** while waiting
//! for I/O. But what does "blocking" actually mean? This section clarifies the
//! terminology and establishes why the "bridges blocking I/O with async Rust" claim above
//! holds for various I/O backends, on various OSes.
//!
//! Let's examine handling terminal input on Linux as a concrete example of where and how
//! to use RRT using [`mio`] (see [`mio_poller`] for details). We'll also
//! discuss what it would take to implement on other platforms.
//!
//! <div class="warning">
//!
//! On other OSes, different backends are needed:
//!
//! 1. 🍎 **macOS**: Can't use [`mio`] — its [`kqueue`] backend returns [`EINVAL`] error
//!    code for [`PTY`]/[`tty`] ([known Darwin limitation]). We would need
//!    [`filedescriptor::poll()`] (uses [`select(2)`] internally) instead.
//!
//! 2. 🪟 **Windows**: [`mio`] uses [`IOCP`], which doesn't support console/[`stdin`] —
//!    [`IOCP`] is for file/socket async I/O only. We would need the [Console API] as the
//!    blocking mechanism (no async console I/O exists on Windows).
//!
//! </div>
//!
//! ## [`mio_poller`]: A Concrete RRT Implementation for Linux terminal input handling
//!
//! [`mio_poller`] satisfies RRT's "blocking I/O" invariant by using [`mio`] — a thin
//! Rust wrapper over OS-specific I/O primitives.
//!
//! On 🐧 **Linux**, [`mio`] uses [`epoll`], which works with [`PTY`]/[`tty`]. The thread
//! blocks inside the [`epoll_wait()`] [`syscall`], waiting on one or more [file
//! descriptors] for [readiness] — a notification that an [`fd`] has data available. The
//! thread then performs the actual I/O operation itself:
//!
//! ```text
//!            ┌─────────────────────────────────────────────────────┐
//!            ▼                                                     │
//!   Thread: poll() ──blocks──► [ready] ──► read() ──► process ─────┘
//!                                            ↑            ↑
//!                                       YOU do I/O   YOU broadcast
//!                                       here         events here
//! ```
//!
//! This isn't [busy-waiting] — the kernel puts the thread to sleep, consuming essentially
//! zero CPU. But the thread **cannot do other work** while waiting. That's what makes it
//! "blocking" which is why RRT uses a dedicated thread - to keep this blocking
//! [`syscall`] off [async executor threads].
//!
//! ## I/O Backend Compatibility
//!
//! The "[blocks on I/O]" claim holds for various I/O sources when using [`mio`]:
//!
//! | I/O Backend                                 | Blocks? | Notes                                                                                                                                |
//! | :------------------------------------------ | :------ | :----------------------------------------------------------------------------------------------------------------------------------- |
//! | [`mio`] + [`stdin`]                         | Yes     | See [`mio_poller`]                                                                                                                   |
//! | [`mio`] + [sockets]                         | Yes     | [TCP], [UDP], [Unix domain sockets]                                                                                                  |
//! | [`mio`] + [signals]                         | Yes     | Signals are async interrupts (not [`pollable`]); [`signalfd(2)`] or [`signal-hook`] wraps them as [`fd`]s (see [`mio_poller`])       |
//! | [`mio`] + [`pipe(2)`]/[`fifo(7)`]           | Yes     | [`pollable`] [`fd`]s ie, 1-1 one-way byte streams: [`pipe(2)`] = parent↔child (anonymous), [`fifo(7)`] = via filesystem path (named) |
//!
//! The thread blocks in [`mio::Poll::poll()`] until the kernel signals readiness (via
//! [`epoll`] on Linux). The blocking behavior is identical for all the sources above.
//!
//! ## [`io_uring`] Compatibility
//!
//! For [`io_uring`], we recommend blocking on [`io_uring_enter()`] (see [`io_uring`: An
//! Alternative Model]). This preserves the blocking behavior that RRT depends on, while
//! gaining [`io_uring`]'s performance benefits. [`io_uring`]'s two other non-blocking
//! modes break the RRT assumption and require a different pattern.
//!
//! # Architecture Overview
//!
//! ## Context
//!
//! To get a bird's eye view (from the TUI application's perspective) of how terminal
//! input flows from [`stdin`] through the [worker] thread to your async consumers — see
//! the [RRT section] in the crate documentation.
//!
//! ## Separation of Concerns and [Dependency Injection] (DI)
//!
//! You and the framework have distinct responsibilities:
//!
//! - **The framework** ([`RRT<F>`]) handles all the thread management and lifecycle
//!   boilerplate — spawning threads, reusing running threads, wake signaling, [`broadcast
//!   channel`]s, subscriber tracking, and graceful shutdown.
//! - **You** provide the [`RRTFactory`] trait implementation along with [`Worker`],
//!   [`Waker`], and [`Event`] types. Without your factory concrete type (and the three
//!   other types) to inject ([DI]), the framework has nothing to run.
//!
//!   | Type           | Purpose                                    | Implementation                                          |
//!   | :------------- | :----------------------------------------- | :------------------------------------------------------ |
//!   | [`RRTFactory`] | [`create()`] - both [`Waker`] + [`Worker`] | [Syscalls]: Create [`mio::Poll`], register your [fds]   |
//!   | [`RRTWorker`]  | [`poll_once()`] - the work loop            | Your logic: [`poll()`] → handle events → [`tx.send()`]  |
//!   | [`RRTWaker`]   | [`wake()`] - the interrupt mechanism       | Backend-specific (see [why user-provided?])             |
//!   | [`Event`]      | Domain-specific subscriber event data      | Struct/enum sent via [`tx.send()`] from [`poll_once()`] |
//!
//!   See the [Example] section for details.
//!
//! ## Design Principles
//!
//! 1. [Inversion of control] (IOC / control flow) — the framework owns the loop (`while
//!    poll_once() == Continue {}`) and creates and manages the thread it runs on. You
//!    provide the iteration logic via [`poll_once()`] in your [`RRTWorker`]
//!    implementation.
//!
//! 2. [Dependency Injection] (DI / composition) — you provide trait implementations (the
//!    "injectables"); the framework orchestrates them together. This is **imperative**
//!    (code-based) [DI]: you provide concrete implementations for these traits -
//!    [`RRTFactory`], [`Worker`], [`Waker`], and a concrete type for [`Event`]. It's not
//!    declarative (configuration-based) [DI] where you *declare* wiring/bindings.
//!
//! 3. **Type safety** — the [`RRTFactory`] trait ensures your concrete types implementing
//!    it and [`Worker`], [`Waker`] traits, along with your concrete type for [`Event`]
//!    all match up correctly at compile time.
//!
//! ## Type Hierarchy Diagram
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                    RESILIENT REACTOR THREAD (Generic)                  │
//! │    IoC + DI: you implement traits ─► framework orchestrates & calls    │
//! ├────────────────────────────┬──────────────┬────────────────────────────┤
//! │                            │  YOUR CODE   │                            │
//! │                            └──────────────┘                            │
//! │  SINGLETON:                                                            │
//! │       static SINGLETON: RRT<F> = ...::new();                           │
//! │                                                                        │
//! │  The generic param <F>:                                                │
//! │       F          : RRTFactory          — your factory impl             │
//! │       F::Waker   : RRTWaker            — your waker type (from F)      │
//! │       F::Event   : Clone + Send + Sync — your event type (from F)      │
//! │       F::Worker  : RRTWorker           — your worker type (from F)     │
//! │                                                                        │
//! ├─────────────────────┬────────────────────────────┬─────────────────────┤
//! │                     │ FRAMEWORK → RUNS YOUR CODE │                     │
//! │                     └────────────────────────────┘                     │
//! │  RRT<F>                                                                │
//! │  ├── Mutex<Option<Arc<RRTState<F::Waker, F::Event>>>>                  │
//! │  │   └── RRTState<F::Waker, F::Event>                                  │
//! │  │       ├── broadcast_tx: Sender<F::Event> (event broadcast)          │
//! │  │       ├── liveness:     RRTLiveness   (running state+generation)    │
//! │  │       └── waker:        F::Waker         (interrupt blocked thread) │
//! │  │                                                                     │
//! │  └── subscribe() → SubscriberGuard<F::Waker, F::Event>                 │
//! │      ├── Slow path: F::create() → spawn worker thread                  │
//! │      │   where YOUR CODE F::Worker.poll_once() runs in a loop          │
//! │      └── Fast path: worker thread already running → reuse it           │
//! │                                                                        │
//! │  SubscriberGuard<F::Waker, F::Event>                                   │
//! │  ├── receiver: Receiver<F::Event>    (broadcast subscription)          │
//! │  ├── state:    Arc<RRTState<...>> (for waker access on drop)           │
//! │  └── Drop impl: receiver dropped (decrements count), wakes worker      │
//! │      └── Worker wakes → broadcast_tx.receiver_count() == 0 → exits     │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## The RRT Contract And Benefits
//!
//! 1. **Thread-safe global state** — [`RRT<F>`] is the type you use to declare your own
//!    `static` singleton (initialized with a [`const expression`]). The generic `F:
//!    `[`RRTFactory`] is **the injection point** — when you call [`subscribe()`], the
//!    framework calls [`RRTFactory::create()`] to get your [`Worker`] and [`Waker`], then
//!    spawns a thread running your worker's [`poll_once()`] in a loop:
//!
//!    <!-- It is ok to use ignore here - example of static singleton declaration -->
//!
//!    ```ignore
//!    /// From mio_poller implementation:
//!    static SINGLETON: RRT<MioPollWorkerFactory> =
//!        RRT::new();
//!
//!    let subscriber_guard = SINGLETON.subscribe()?;
//!    ```
//!
//!    The [`'static` trait bound] on `E` means the event type can be held indefinitely
//!    without becoming invalid — it *can* live arbitrarily long, not that it *must*. The
//!    type may contain `'static` references but no shorter-lived ones. See
//!    [`RRT`] for a detailed explanation of `'static` in trait bounds.
//!
//!    This newtype wraps a [`Mutex<Option<Arc<RRTState<W, E>>>>`] because [`syscalls`]
//!    aren't [`const expressions`] — the state must be created at runtime. See
//!    [`RRT`] for a detailed explanation. See [`mio_poller`]'s
//!    [`SINGLETON`] for a concrete example.
//!
//! 2. **State machine** — [`RRTState`] can be created, destroyed, and reused. On spawn,
//!    [`subscribe()`] populates the singleton with a fresh [`RRTState`]. On exit,
//!    [`RRTLiveness`] marks it terminated. On restart, [`subscribe()`] replaces the old
//!    state with fresh resources. Generation tracking distinguishes fresh restarts from
//!    reusing an existing thread.
//!
//! 3. **Contract preservation** — Async consumers never see broken promises; the
//!    [`broadcast channel`] decouples producers from consumers. This unlocks two key
//!    benefits:
//!
//!    - **Lifecycle flexibility** — Multiple async tasks can subscribe independently.
//!      Consumers can come and go without affecting the [worker] thread.
//!
//!    - **Resilience** — The thread itself can crash and restart; services can connect,
//!      disconnect, and reconnect. The TUI app remains unaffected.
//!
//! ## The Coupled Resource Creation Problem
//!
//! Creating the [worker] thread has an ordering conflict:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                       THE ORDERING CONFLICT                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │   To interrupt the thread, SubscriberGuards need a Waker.               │
//! │   To create a Waker, we need mio::Poll's registry.                      │
//! │   But mio::Poll must MOVE to the spawned thread.                        │
//! │                                                                         │
//! │   ┌─────────────────────────────────────────────────────────────────┐   │
//! │   │  PROBLEM: After thread::spawn(), Poll is gone — too late to     │   │
//! │   │           create a Waker from its registry!                     │   │
//! │   └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │   Timeline without solution:                                            │
//! │                                                                         │
//! │     create Poll ──► spawn thread ──► Poll moves ──► ✗ can't create      │
//! │                     (Poll gone!)                       Waker anymore    │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    THE SOLUTION: TWO-PHASE SETUP                        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │   Phase 1: RRTFactory::create() — resources only, no thread spawned     │
//! │   ┌─────────────────────────────────────────────────────────────────┐   │
//! │   │  Creates BOTH from the same mio::Poll registry:                 │   │
//! │   │                                                                 │   │
//! │   │     mio::Poll ──registry──► mio::Waker                          │   │
//! │   │         │                       │                               │   │
//! │   │         ▼                       ▼                               │   │
//! │   │      Worker                   Waker                             │   │
//! │   │    (owns Poll)            (from registry)                       │   │
//! │   └─────────────────────────────────────────────────────────────────┘   │
//! │                    │                       │                            │
//! │                    ▼                       ▼                            │
//! │   Phase 2: Split and distribute                                         │
//! │   ┌────────────────────────┐    ┌───────────────────────────────────┐   │
//! │   │    Spawned Thread      │    │         RRTState                  │   │
//! │   │    ───────────────     │    │         ───────────               │   │
//! │   │    Worker moves here   │    │    Waker stored here              │   │
//! │   │    (owns mio::Poll)    │    │    (shared via Arc)               │   │
//! │   │                        │◄───│    SubscriberGuards call wake()   │   │
//! │   └────────────────────────┘    └───────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The key insight: **atomic creation, then separation**. Both resources are created
//! together from the same [`mio::Poll`] registry, then split — the [waker] stays in
//! [`RRTState`] for subscribers, while the [worker] moves to the spawned thread.
//!
//! ### Why is [`RRTWaker`] User-Provided?
//!
//! The [waker] is **intrinsically coupled** to the [worker]'s blocking mechanism.
//! Different I/O backends need different wake strategies:
//!
//! | Blocking on...          | Wake strategy                                  |
//! | :---------------------- | :--------------------------------------------- |
//! | [`mio::Poll`]           | [`mio::Waker`] (triggers [`epoll`]/[`kqueue`]) |
//! | TCP [`accept()`]        | Connect-to-self pattern                        |
//! | Pipe [`read(2)`]        | Self-pipe trick (write a byte)                 |
//! | [`io_uring`]            | [`eventfd`] or [`IORING_OP_MSG_RING`]          |
//!
//! The framework can't know how you're blocking — it just calls [`poll_once()`] in a
//! loop. Only you know how to interrupt your specific blocking call.
//!
//! The coupling is also at the resource level: a [`mio::Waker`] is created FROM the
//! [`mio::Poll`]'s registry. If the poll is dropped (thread exits), the [waker] becomes
//! useless. That's why [`Factory::create()`] returns both together.
//!
//! This design gives the framework flexibility: it works with [`mio`] today and
//! [`io_uring`] tomorrow without framework changes.
//!
//! ## The Inherent Race Condition
//!
//! There's an unavoidable race window between when a receiver drops and when the
//! thread checks if it should exit:
//!
//! ```text
//! Timeline:
//! ─────────────────────────────────────────────────────────────────►
//!      wake()          kernel         poll()         check
//!      called         schedules       returns     receiver_count
//!         │              │               │              │
//!         └──────────────┴───────────────┴──────────────┘
//!                     RACE WINDOW
//!               (new subscriber can appear here)
//! ```
//!
//! The [kernel] schedules threads independently, so there's no guarantee when the
//! [worker] thread will wake up after [`wake()`] is called. The RRT pattern handles this
//! correctly by checking the **current** [`receiver_count()`] at exit time, not the
//! count when [`wake()`] was called. See [`RRTState::should_self_terminate()`] for
//! details.
//!
//! # How To Use It
//!
//! Your journey begins with [`RRTFactory`] — implement this trait to inject your
//! business logic into the framework.
//!
//! ## Example
//!
//! Implementing the RRT pattern for a new use case:
//!
//! ```no_run
//! # use r3bl_tui::core::resilient_reactor_thread::*;
//! # use r3bl_tui::Continuation;
//! # use tokio::sync::broadcast::Sender;
//! #
//! # // --- Hidden boilerplate: Event type ---
//! # #[derive(Clone)]
//! # struct MyEvent;
//! #
//! // 1. Define your waker (how to interrupt your blocking call)
//! struct MyWaker(mio::Waker);
//!
//! impl RRTWaker for MyWaker {
//!     fn wake(&self) -> std::io::Result<()> {
//!         self.0.wake()
//!     }
//! }
//!
//! // 2. Define your worker (the actual work loop)
//! struct MyWorker { /* resources */ }
//!
//! impl RRTWorker for MyWorker {
//!     type Event = MyEvent;
//!
//!     fn poll_once(&mut self, tx: &Sender<Self::Event>) -> Continuation {
//!         todo!("Do one iteration of work, broadcast events")
//!         // Return Continuation::Stop when receiver_count == 0
//!     }
//! }
//!
//! // 3. Define your factory (creates worker + waker together)
//! struct MyWorkerFactory;
//!
//! impl RRTFactory for MyWorkerFactory {
//!     type Event = MyEvent;
//!     type Worker = MyWorker;
//!     type Waker = MyWaker;
//!
//!     fn create() -> Result<(Self::Worker, Self::Waker), miette::Report> {
//!         todo!("Create coupled worker and waker")
//!     }
//! }
//!
//! // 4. Create a static global state (factory type F bundles all associated types)
//! static GLOBAL: RRT<MyWorkerFactory> =
//!     RRT::new();
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 5. Subscribe to events
//! let subscriber_guard = GLOBAL.subscribe()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Concrete Implementation
//!
//! See [`mio_poller`] for a concrete implementation using this infrastructure for
//! terminal input handling.
//!
//! # Module Contents
//!
//! - **`types`**: Core traits ([`RRTWaker`], [`RRTWorker`], [`RRTFactory`])
//! - **`thread_liveness`**: Thread lifecycle state ([`RRTLiveness`], [`LivenessState`])
//! - **`thread_state`**: Shared state container ([`RRTState`])
//! - **`subscriber_guard`**: RAII subscription guard ([`SubscriberGuard`])
//! - **`thread_safe_global_state`**: Framework entry point ([`RRT`])
//!
//! # [`io_uring`]: An Alternative Model
//!
//! [`io_uring`] (Linux 5.1+) fundamentally changes the I/O model. Instead of waiting
//! for readiness and then doing I/O yourself, you **submit I/O requests** to the kernel
//! and **receive completions** when they finish. The kernel does the actual I/O
//! asynchronously.
//!
//! [`io_uring`] offers several operating modes with different blocking characteristics:
//!
//! | Mode                                    | Thread blocks?                    | Fits RRT? |
//! | :-------------------------------------- | :-------------------------------- | :-------- |
//! | [`io_uring_enter()`] with wait          | Yes (waiting for [completions])   | Yes       |
//! | [`io_uring_enter()`] non-blocking       | No (just checks [CQ])             | No        |
//! | [`SQPOLL`]                              | No ([kernel] thread polls)        | No        |
//!
//! In non-blocking or [`SQPOLL`] modes, the worker could look like this:
//!
//! ```text
//! Thread: submit_io() ──► do_other_work() ──► check_completions() ──► process ──►
//!         (no block)      (thread active)     (non-blocking peek)
//! ```
//!
//! This **breaks the RRT assumption** — there's nothing to interrupt with [`wake()`]
//! because the thread never blocks. You'd need a different pattern entirely.
//!
//! ## Recommendation: Blocking Wait Mode
//!
//! For RRT compatibility, use [`io_uring`] in **blocking-wait mode**. This preserves the
//! simple RRT programming model while gaining [`io_uring`]'s performance benefits:
//!
//! ```text
//! io_uring blocking-wait model:
//! Thread: submit(read) ──► io_uring_enter(wait) ──blocks──► [complete] ──► process
//!                          ↑                                 ↑
//!                     kernel does I/O                   data already
//!                     while you wait                    in buffer!
//! ```
//!
//! Even though you block in both models, **[`io_uring`]'s blocking is more efficient**:
//! the kernel performs the actual I/O during that wait, not just watching for readiness.
//!
//! ## Benefits Over epoll
//!
//! | Benefit                | [`epoll`]                        | [`io_uring`] (blocking wait)          |
//! | :--------------------- | :------------------------------- | :------------------------------------ |
//! | [Syscall] batching     | poll → read → poll → read        | submit N reads, wait once             |
//! | Who does I/O           | You call [`read(2)`] after ready | Kernel already read into your buffer  |
//! | [Registered buffers]   | Not available                    | Pin buffers, avoid copies             |
//! | [Registered FDs]       | FD lookup every op               | Avoid fd table lookup                 |
//! | [Linked operations]    | Not available                    | Chain read→process→write              |
//!
//! ## Implementation Sketch
//!
//! The [`poll_once()`] implementation would change from the current [`epoll`] model:
//!
//! <!-- It is ok to use ignore here - pseudo-code sketch showing epoll readiness-based
//! API pattern -->
//!
//! ```ignore
//! // Current epoll model
//! fn poll_once(&mut self, tx: &Sender<Event>) -> Continuation {
//!     self.poll.poll(&mut events, None)?;  // Block for readiness
//!     for event in &events {
//!         let data = read(fd)?;            // YOU do the I/O
//!         tx.send(data);
//!     }
//! }
//! ```
//!
//! To an [`io_uring`] blocking-wait model:
//!
//! <!-- It is ok to use ignore here - pseudo-code sketch showing io_uring
//! completion-based API pattern -->
//!
//! ```ignore
//! // io_uring blocking-wait model
//! fn poll_once(&mut self, tx: &Sender<Event>) -> Continuation {
//!     // Submit read requests to submission queue
//!     self.ring.submit_read(stdin_fd, &mut buffer)?;
//!
//!     // Block waiting for completions (kernel does I/O during this wait)
//!     self.ring.submit_and_wait(1)?;
//!
//!     // Data already in buffer — just process it
//!     for cqe in self.ring.completion() {
//!         tx.send(process(cqe));           // I/O already done!
//!     }
//! }
//! ```
//!
//! ## Waker Mechanism Adaptation
//!
//! The [`RRTWaker`] implementation would need adjustment for [`io_uring`]. Options:
//!
//! 1. **eventfd registered with [`io_uring`]** — Submit a read on an eventfd, wake by
//!    writing to it
//! 2. **[`IORING_OP_MSG_RING`]** — [`io_uring`]'s native cross-ring messaging (Linux
//!    5.18+)
//! 3. **Cancellation** — Submit [`IORING_OP_ASYNC_CANCEL`] to interrupt pending
//!    operations
//!
//! The RRT's [`RRTWaker`] trait already abstracts this, so the change would be
//! localized to the factory implementation.
//!
//! # Why "RRT" and Not Actor/Reactor/Proactor?
//!
//! RRT shares traits with several classic concurrency patterns but doesn't fit neatly
//! into any single category:
//!
//! | Pattern           | Similarity                          | Key Difference                                                                      |
//! | :---------------- | :---------------------------------- | :---------------------------------------------------------------------------------- |
//! | [Actor]           | Dedicated execution context         | Actors are lightweight (many per thread); RRT is one OS thread that [blocks on I/O] |
//! | [Reactor]         | Event demultiplexing, I/O readiness | Reactor typically runs in the main loop; RRT isolates [blocking I/O] to a worker    |
//! | [Proactor]        | Async I/O, kernel involvement       | Proactor uses completion callbacks; RRT blocks waiting for readiness                |
//! | Producer-Consumer | Thread produces for consumers       | Producer-Consumer uses 1:1 queues; RRT uses 1:N [`broadcast`]                       |
//!
//! **What makes RRT distinct:**
//!
//! 1. **Blocking by design** — The [worker] thread *intentionally* [blocks on I/O]. This
//!    isn't a limitation; it's the feature. Blocking keeps the I/O off async executor
//!    threads.
//!
//! 2. **Broadcast semantics** — Events go to *all* subscribers (1:N), not a single
//!    consumer. When a terminal resize occurs, every UI component needs to know.
//!
//! 3. **Resilience** — Generation tracking enables graceful thread restart without
//!    breaking existing subscribers. The "Resilient" in RRT refers to this recovery
//!    capability.
//!
//! 4. **I/O-centric** — RRT is specialized for OS-level I/O ([`stdin`], [signals],
//!    [sockets]), not general message processing.
//!
//! [`Mutex<Option<Arc<RRTState<W, E>>>>`]: RRTState
//!
//! [Actor]: https://en.wikipedia.org/wiki/Actor_model
//! [CQ]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [Console API]: https://learn.microsoft.com/en-us/windows/console/console-functions
//! [DI]: https://en.wikipedia.org/wiki/Dependency_injection
//! [Dependency Injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [Example]: #example
//! [Linked operations]: https://man7.org/linux/man-pages/man3/io_uring_prep_link.3.html
//! [OS I/O primitives]: #io-backend-compatibility
//! [Proactor]: https://en.wikipedia.org/wiki/Proactor_pattern
//! [RRT section]: crate#resilient-reactor-thread-rrt-pattern
//! [Reactor]: https://en.wikipedia.org/wiki/Reactor_pattern
//! [Registered FDs]: https://man7.org/linux/man-pages/man3/io_uring_register_files.3.html
//! [Registered buffers]: https://man7.org/linux/man-pages/man3/io_uring_register_buffers.3.html
//! [Syscall]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [Syscalls]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [TCP]: https://en.wikipedia.org/wiki/Transmission_Control_Protocol
//! [UDP]: https://en.wikipedia.org/wiki/User_Datagram_Protocol
//! [Unix domain sockets]: https://en.wikipedia.org/wiki/Unix_domain_socket
//! [Web Workers]: https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API
//! [`'static` trait bound]: RRT#static-trait-bound-vs-static-lifetime-annotation
//! [`Arc`]: std::sync::Arc
//! [`Continuation`]: crate::core::common::Continuation
//! [`EINVAL`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`Event`]: RRTWorker::Event
//! [`Factory::create()`]: RRTFactory::create
//! [`IOCP`]: https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports
//! [`IORING_OP_ASYNC_CANCEL`]: https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html
//! [`IORING_OP_MSG_RING`]: https://man7.org/linux/man-pages/man3/io_uring_prep_msg_ring.3.html
//! [`LivenessState`]: crate::core::resilient_reactor_thread::LivenessState
//! [`Mutex`]: std::sync::Mutex
//! [`Option`]: std::option::Option
//! [`PTY`]: https://man7.org/linux/man-pages/man7/pty.7.html
//! [`RRTFactory`]: RRTFactory
//! [`RRTLiveness`]: RRTLiveness
//! [`RRT`]: RRT
//! [`RRTState::should_self_terminate()`]: RRTState::should_self_terminate
//! [`RRTState`]: RRTState
//! [`RRTWaker`]: RRTWaker
//! [`RRTWorker`]: RRTWorker
//! [`Receiver::recv()`]: tokio::sync::broadcast::Receiver::recv
//! [`SINGLETON`]: crate::terminal_lib_backends::global_input_resource::SINGLETON
//! [`SQPOLL`]: https://man7.org/linux/man-pages/man2/io_uring_setup.2.html
//! [`SubscriberGuard`]: SubscriberGuard
//! [`Waker`]: RRTWaker
//! [`Worker`]: RRTWorker
//! [`accept()`]: std::net::TcpListener::accept
//! [`broadcast channel`]: tokio::sync::broadcast
//! [`broadcast`]: tokio::sync::broadcast
//! [`const expression`]: RRT#const-expression-vs-const-declaration-vs-static-declaration
//! [`const expressions`]: RRT#const-expression-vs-const-declaration-vs-static-declaration
//! [`create()`]: RRTFactory::create
//! [`epoll_wait()`]: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
//! [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`fifo(7)`]: https://man7.org/linux/man-pages/man7/fifo.7.html
//! [`file descriptor`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`filedescriptor::poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`io_uring_enter()`]: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html
//! [`io_uring`]: https://kernel.dk/io_uring.pdf
//! [`io_uring`: An Alternative Model]: #io_uring-an-alternative-model
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
//! [`mio::Poll::poll()`]: mio::Poll::poll
//! [`mio::Poll`]: mio::Poll
//! [`mio_poller`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller
//! [`mio`]: mio
//! [`pipe(2)`]: https://man7.org/linux/man-pages/man2/pipe.2.html
//! [`poll()`]: mio::Poll::poll
//! [`poll_once()`]: RRTWorker::poll_once
//! [`pollable`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`read(2)`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
//! [`select(2)`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`signal-hook`]: signal_hook
//! [`signalfd(2)`]: https://man7.org/linux/man-pages/man2/signalfd.2.html
//! [`signals`]: https://en.wikipedia.org/wiki/Signal_(IPC)
//! [`sockets`]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [`stdin`]: std::io::stdin
//! [`subscribe()`]: RRT::subscribe
//! [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [`tx.send()`]: tokio::sync::broadcast::Sender::send
//! [`wake()`]: RRTWaker::wake
//! [async executor threads]: tokio::runtime
//! [async runtime]: tokio::runtime
//! [async task]: tokio::task
//! [blocking I/O]: #understanding-blocking-io
//! [blocks on I/O]: #understanding-blocking-io
//! [busy-waiting]: https://en.wikipedia.org/wiki/Busy_waiting
//! [completions]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [fds]: https://en.wikipedia.org/wiki/File_descriptor
//! [file descriptors]: https://en.wikipedia.org/wiki/File_descriptor
//! [inversion of control]: https://en.wikipedia.org/wiki/Inversion_of_control
//! [kernel]: https://en.wikipedia.org/wiki/Kernel_(operating_system)
//! [known Darwin limitation]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [readiness]: https://man7.org/linux/man-pages/man7/epoll.7.html#DESCRIPTION
//! [signals]: https://en.wikipedia.org/wiki/Signal_(IPC)
//! [sockets]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [system call]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [waker]: RRTWaker
//! [why user-provided?]: #why-is-rrtwaker-user-provided
//! [worker]: RRTWorker

mod rrt_di_traits;
mod rrt_liveness;
mod rrt_safe_global_state;
mod rrt_state;
mod rrt_subscriber_guard;

pub use rrt_di_traits::*;
pub use rrt_liveness::*;
pub use rrt_safe_global_state::*;
pub use rrt_state::*;
pub use rrt_subscriber_guard::*;

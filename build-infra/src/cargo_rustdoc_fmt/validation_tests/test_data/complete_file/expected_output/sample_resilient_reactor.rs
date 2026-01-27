// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words epoll kqueue SIGWINCH syscall syscalls SQPOLL IORING eventfd signalfd
// cspell:words pollable Proactor demultiplexing injectables threadwaker IOCP EINVAL
// cspell:words kqueuefd filedescriptor

//! Generic infrastructure for the Resilient Reactor Thread (RRT) pattern implementation.
//!
//! # What is the RRT Pattern?
//!
//! The **Resilient Reactor Thread** pattern bridges blocking I/O with async Rust.
//!
//! Calling a blocking [`syscall`] (like [`read(2)`] on [`stdin`]) from async code blocks
//! the entire async runtime - **which is not ok**. RRT solves this by isolating [blocking
//! I/O] in a dedicated thread and creating a bridge to async
//! consumers via a [`broadcast channel`]. Thus allowing async code to consume events from
//! blocking sources ([`stdin`], [`sockets`], [`signals`]) without blocking the async
//! runtime.
//!
//! The name reflects its three core properties:
//!
//! | Component     | Meaning                                                                                                       |
//! | :------------ | :------------------------------------------------------------------------------------------------------------ |
//! | **Resilient** | Thread can stop or crash and restart with generation tracking; subscribers are not affected                   |
//! | **Reactor**   | Reacts to I/O events ([`stdin`], [`sockets`], [`signals`]) using [`mio`]/[`epoll`] using [OS I/O Primitives]  |
//! | **Thread**    | Dedicated thread for blocking I/O; graceful shutdown when consumers disconnect; fully managed lifecycle       |
//!
//!
//! # Understanding "Blocking I/O"
//!
//! The RRT pattern's core invariant is that the worker thread **blocks** while waiting
//! for I/O. But what does "blocking" actually mean? This section clarifies the
//! terminology and establishes why the "bridges blocking I/O with async Rust" claim above
//! holds for various I/O backends, on various OSes.
//!
//! Below we examine handling terminal input on Linux using RRT in detail
//! ([`mio_poller`]). We also discuss what it would take to implement terminal input
//! handling using RRT on other platforms.
//!
//! ## [`mio_poller`]: A Concrete RRT Implementation for Linux terminal input handling
//!
//! [`mio_poller`] satisfies RRT's "blocking I/O" invariant by using [`mio`] — a thin
//! Rust wrapper over OS-specific I/O primitives. [`mio`] uses [`epoll`], which works with
//! [`PTY`]/[`tty`]. On other OSes, different backends are needed:
//!
//! 1. **macOS**: Can't use [`mio`] — its [`kqueue`] backend returns [`EINVAL`] for
//!    [`PTY`]/[`tty`] ([known Darwin limitation]). Would need [`filedescriptor::poll()`]
//!    (uses [`select(2)`] internally) instead.
//!
//! 2. **Windows**: [`mio`] uses [`IOCP`], which doesn't support console/stdin — [`IOCP`]
//!    is for file/socket async I/O only. Would need the [Console API] with a dedicated
//!    blocking thread (no async console I/O exists on Windows).
//!
//! With [`epoll`], the thread blocks inside [`epoll_wait()`], waiting on one or more
//! [file descriptors] for **readiness** — a notification that an [`fd`] has data
//! available. The thread then performs the actual I/O operation itself:
//!
//! ```text
//! Thread: poll() ──blocks──► [ready] ──► read() ──► process ──► poll() ──blocks──►
//!                                          ↑
//!                                     YOU do I/O here
//! ```
//!
//! This isn't busy-waiting — the kernel puts the thread to sleep, consuming essentially
//! zero CPU. But the thread **cannot do other work** while waiting. That's what makes it
//! "blocking" which is why RRT uses a dedicated thread - to keep this blocking work off
//! async executor threads.
//!
//! ## I/O Backend Compatibility
//!
//! The "blocks on I/O" claim holds for various I/O sources when using [`mio`]:
//!
//! | I/O Backend                                 | Blocks? | Notes                                                                                                                          |
//! | :------------------------------------------ | :------ | :----------------------------------------------------------------------------------------------------------------------------- |
//! | [`mio`] + [`stdin`]                         | Yes     | See [`mio_poller`]                                                                                                             |
//! | [`mio`] + [sockets]                         | Yes     | TCP, UDP, Unix domain                                                                                                          |
//! | [`mio`] + [signals]                         | Yes     | Signals are async interrupts (not [`pollable`]); [`signalfd(2)`] or [`signal-hook`] wraps them as [`fd`]s (see [`mio_poller`]) |
//! | [`mio`] + [`pipe(2)`]/[`fifo(7)`]           | Yes     | Example of other [`pollable`] [`fd`]s: unidirectional IPC - [`pipe(2)`]=anonymous, [`fifo(7)`]=named                           |
//!
//! The thread blocks in [`mio::Poll::poll()`] until the kernel signals readiness (via
//! [`epoll`] on Linux, [`kqueue`] on macOS). The blocking behavior is identical
//! regardless of **what** you're waiting on:
//! 1. [`stdin`]
//! 2. [`sockets`]
//! 3. [`signals`] via [`signalfd(2)`] or [`signal-hook`]
//! 4. any [`pollable`] [`file descriptor`]
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
//! input flows from [`stdin`] through the worker thread to your async consumers — see the
//! [RRT section] in the crate documentation.
//!
//! ## Separation of Concerns and Dependency Injection (DI)
//!
//! The framework and its user have distinct responsibilities:
//!
//! - The framework ([`ThreadSafeGlobalState<F>`]) handles all the mechanics: spawning
//!   threads, reusing running threads, wake signaling, [`broadcast channel`]s, subscriber
//!   tracking, and graceful shutdown. **The generic `F` is how DI works** — it's your
//!   [`RRTFactory`] impl that the framework calls to get your [`Worker`], [`Waker`], and
//!   [`Event`] types. Without `F`, the framework has no code to run.
//! - The user implements 3 traits and an event type (enum/struct) — see [Example] for
//!   complete implementations:
//!
//!   | Type                    | Purpose                                    | Implementation                                         |
//!   | :---------------------- | :----------------------------------------- | :----------------------------------------------------- |
//!   | [`RRTFactory`]          | Create [`Waker`] + [`Worker`] together     | [Syscalls]: Create [`mio::Poll`], register your [fds]  |
//!   | [`RRTWorker`]           | The work loop - [`poll_once()`] method     | Your logic: [`poll()`] → handle events → [`tx.send()`] |
//!   | [`RRTWaker`]            | Interrupt mechanism - [`wake()`] method    | Backend-specific (see [why user-provided?])            |
//!   | [`Event`] type          | Your event data - `Clone + Send + 'static` | Define your event type                                 |
//!
//! These three design principles make this work:
//!
//! 1. [Inversion of control] (IOC / control flow) — the framework owns the loop (`while
//!    poll_once() == Continue {}`), which runs in the thread that it creates and manages,
//!    you provide the iteration logic ([`poll_once()`]).
//!
//! 2. [Dependency Injection] (DI / composition) — you provide trait implementations (the
//!    "injectables"), the framework orchestrates them together. This is **imperative**
//!    (code-based) DI: you implement traits ([`Factory`], [`Worker`], [`Waker`]), rather
//!    than declarative (configuration-based) DI where you *declare* wiring/bindings.
//!
//! 3. The generic parameter `<F>` makes this **type-safe**: the compiler ensures your
//!    [`Factory`], [`Worker`], [`Waker`], and [`Event`] types all match up at compile
//!    time.
//!
//! ## Type Hierarchy Diagram
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                    RESILIENT REACTOR THREAD (Generic)                  │
//! │    DI + IoC: you implement traits ─► framework orchestrates & calls    │
//! ├──────────────────────────────────────┬─────────────────────────────────┤
//! │                                      │ FRAMEWORK → RUNS YOUR CODE HERE │
//! │  ThreadSafeGlobalState<F>            └─────────────────────────────────┤
//! │  ├── Mutex<Option<Arc<ThreadState<F::Waker, F::Event>>>>               │
//! │  │   └── ThreadState<F::Waker, F::Event>                               │
//! │  │       ├── broadcast_tx: Sender<F::Event> (event broadcast)          │
//! │  │       ├── liveness:     ThreadLiveness   (running state+generation) │
//! │  │       └── waker:        F::Waker         (interrupt blocked thread) │
//! │  │                                                                     │
//! │  └── subscribe() → SubscriberGuard<F::Waker, F::Event>                 │
//! │      ├── Slow path: F::create() → spawn worker thread                  │
//! │      │   where YOUR CODE F::Worker.poll_once() runs in a loop          │
//! │      └── Fast path: worker thread already running → reuse it           │
//! │                                                                        │
//! │  SubscriberGuard<F::Waker, F::Event>                                   │
//! │  ├── receiver: Receiver<F::Event>    (broadcast subscription)          │
//! │  ├── state:    Arc<ThreadState<...>> (for waker access on drop)        │
//! │  └── Drop impl: receiver dropped (decrements count), wakes worker      │
//! │      └── Worker wakes → broadcast_tx.receiver_count() == 0 → exits     │
//! │                                                                        │
//! ├────────────────────────────────────────────┬───────────────────────────┤
//! │                                            │ YOU DEFINE YOUR CODE HERE │
//! │  SINGLETON:                                └───────────────────────────┤
//! │       static SINGLETON: ThreadSafeGlobalState<F> = ...::new();         │
//! │                                                                        │
//! │  The generic param <F>:                                                │
//! │       F          : RRTFactory       — your factory impl                │
//! │       F::Waker   : RRTWaker         — your waker type (from F)         │
//! │       F::Event   : Clone + Send + Sync — your event type (from F)      │
//! │       F::Worker  : RRTWorker        — your worker type (from F)        │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## The RRT Contract And Benefits
//!
//! 1. **Thread-safe global state** — [`ThreadSafeGlobalState<F>`] is the type you use to
//!    declare your own `static` singleton (initialized with a [`const expression`]). The
//!    generic `F: `[`RRTFactory`] is **the injection point** — when you call
//!    [`subscribe()`], the framework calls [`RRTFactory::create()`] to get your
//!    [`Worker`] and [`Waker`], then spawns a thread running your worker's
//!    [`poll_once()`] in a loop:
//!
//!    <!-- It is ok to use ignore here - example of static singleton declaration -->
//!
//!    ```ignore
//!    /// From mio_poller implementation:
//!    static SINGLETON: ThreadSafeGlobalState<MioPollWorkerFactory> =
//!        ThreadSafeGlobalState::new();
//!
//!    let subscriber_guard = SINGLETON.subscribe()?;
//!    ```
//!
//!    The [`'static` trait bound] on `E` means the event type contains no non-`'static`
//!    references — it *can* live arbitrarily long, not that it *must*. See
//!    [`ThreadSafeGlobalState`] for a detailed explanation of `'static` in trait bounds.
//!
//!    This newtype wraps a [`Mutex<Option<Arc<ThreadState<W, E>>>>`] because [`syscalls`]
//!    aren't [`const expressions`] — the state must be created at runtime. See
//!    [`ThreadSafeGlobalState`] for a detailed explanation. See [`mio_poller`]'s
//!    [`SINGLETON`] for a concrete example.
//!
//! 2. **State machine** — [`ThreadState`] can be created, destroyed, and reused. On
//!    spawn, [`subscribe()`] populates the singleton with a fresh [`ThreadState`]. On
//!    exit, [`ThreadLiveness`] marks it terminated. On restart, [`subscribe()`] replaces
//!    the old state with fresh resources. Generation tracking distinguishes fresh
//!    restarts from reusing an existing thread.
//!
//! 3. **Contract preservation** — Async consumers never see broken promises; the
//!    [`broadcast channel`] decouples producers from consumers. This unlocks two key
//!    benefits:
//!
//!    - **Lifecycle flexibility** — Multiple async tasks can subscribe independently.
//!      Consumers can come and go without affecting the worker thread.
//!
//!    - **Resilience** — The thread itself can crash and restart; services can connect,
//!      disconnect, and reconnect. The TUI app remains unaffected.
//!
//! ## The Chicken-Egg Problem
//!
//! Creating a worker thread requires resources that need to be shared. The problem is
//! circular: a [`mio::Waker`] needs the [`mio::Poll`]'s registry to be created, but the
//! [`mio::Poll`] must move to the spawned thread. Meanwhile, the [`ThreadState`] needs
//! the waker stored in it so [`SubscriberGuard`]s can call [`wake()`].
//!
//! The solution is a **two-phase setup** via [`RRTFactory`]:
//! 1. [`Factory::create()`] creates **both** worker and waker together
//! 2. The waker is stored in [`ThreadState`] for subscribers to call [`wake()`]
//! 3. The worker moves to the spawned thread (owns [`mio::Poll`], does the actual work)
//!
//! ### Why is [`RRTWaker`] User-Provided?
//!
//! The waker is **intrinsically coupled** to the worker's blocking mechanism. Different
//! I/O backends need different wake strategies:
//!
//! | Blocking on...          | Wake strategy                              |
//! | :---------------------- | :----------------------------------------- |
//! | [`mio::Poll`]           | [`mio::Waker`] (triggers epoll/kqueue)     |
//! | TCP [`accept()`]        | Connect-to-self pattern                    |
//! | Pipe [`read(2)`]        | Self-pipe trick (write a byte)             |
//! | [`io_uring`]            | eventfd or [`IORING_OP_MSG_RING`]          |
//!
//! The framework can't know how you're blocking — it just calls [`poll_once()`] in a
//! loop. Only you know how to interrupt your specific blocking call.
//!
//! The coupling is also at the resource level: a [`mio::Waker`] is created FROM the
//! [`mio::Poll`]'s registry. If the poll is dropped (thread exits), the waker becomes
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
//! worker thread will wake up after [`wake()`] is called. The RRT pattern handles this
//! correctly by checking the **current** [`receiver_count()`] at exit time, not the
//! count when [`wake()`] was called. See [`ThreadState::should_self_terminate()`] for
//! details.
//!
//! # How To Use It
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
//! static GLOBAL: ThreadSafeGlobalState<MyWorkerFactory> =
//!     ThreadSafeGlobalState::new();
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
//! - **`thread_liveness`**: Thread lifecycle state ([`ThreadLiveness`],
//!   [`LivenessState`])
//! - **`thread_state`**: Shared state container ([`ThreadState`])
//! - **`subscriber_guard`**: RAII subscription guard ([`SubscriberGuard`])
//! - **`thread_safe_global_state`**: Global state manager ([`ThreadSafeGlobalState`])
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
//! | Pattern           | Similarity                          | Key Difference                                                                     |
//! | :---------------- | :---------------------------------- | :--------------------------------------------------------------------------------- |
//! | [Actor]           | Dedicated execution context         | Actors are lightweight (many per thread); RRT is one OS thread that blocks on I/O  |
//! | [Reactor]         | Event demultiplexing, I/O readiness | Reactor typically runs in the main loop; RRT isolates blocking I/O to a worker     |
//! | [Proactor]        | Async I/O, kernel involvement       | Proactor uses completion callbacks; RRT blocks waiting for readiness               |
//! | Producer-Consumer | Thread produces for consumers       | Producer-Consumer uses 1:1 queues; RRT uses 1:N [`broadcast`]                      |
//!
//! **What makes RRT distinct:**
//!
//! 1. **Blocking by design** — The worker thread *intentionally* blocks on I/O. This
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
//! [`Mutex<Option<Arc<ThreadState<W, E>>>>`]: ThreadState
//!
//! [blocking
//! I/O]: #understanding-blocking-io
//!
//! [Actor]: https://en.wikipedia.org/wiki/Actor_model
//! [Blocks on I/O]: #understanding-blocks-on-io
//! [CQ]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [Console API]: https://learn.microsoft.com/en-us/windows/console/console-functions
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
//! [`'static` trait bound]: ThreadSafeGlobalState#static-trait-bound-vs-static-lifetime-annotation
//! [`Arc`]: std::sync::Arc
//! [`Continuation`]: crate::core::common::Continuation
//! [`EINVAL`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`Event`]: RRTWorker::Event
//! [`Factory::create()`]: RRTFactory::create
//! [`Factory`]: RRTFactory
//! [`IOCP`]: https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports
//! [`IORING_OP_ASYNC_CANCEL`]: https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html
//! [`IORING_OP_MSG_RING`]: https://man7.org/linux/man-pages/man3/io_uring_prep_msg_ring.3.html
//! [`LivenessState`]: crate::core::resilient_reactor_thread::LivenessState
//! [`Mutex`]: std::sync::Mutex
//! [`Option`]: std::option::Option
//! [`PTY`]: https://man7.org/linux/man-pages/man7/pty.7.html
//! [`RRTFactory`]: RRTFactory
//! [`RRTWaker`]: RRTWaker
//! [`RRTWorker`]: RRTWorker
//! [`SINGLETON`]: crate::terminal_lib_backends::direct_to_ansi::input::input_device_impl::global_input_resource::SINGLETON
//! [`SQPOLL`]: https://man7.org/linux/man-pages/man2/io_uring_setup.2.html
//! [`SubscriberGuard`]: SubscriberGuard
//! [`ThreadLiveness`]: ThreadLiveness
//! [`ThreadSafeGlobalState`]: ThreadSafeGlobalState
//! [`ThreadState::should_self_terminate()`]: ThreadState::should_self_terminate
//! [`ThreadState`]: ThreadState
//! [`Waker`]: RRTWaker
//! [`Worker`]: RRTWorker
//! [`accept()`]: std::net::TcpListener::accept
//! [`broadcast channel`]: tokio::sync::broadcast
//! [`broadcast`]: tokio::sync::broadcast
//! [`const expression`]: ThreadSafeGlobalState#const-expression-vs-const-declaration-vs-static-declaration
//! [`const expressions`]: ThreadSafeGlobalState#const-expression-vs-const-declaration-vs-static-declaration
//! [`epoll_wait()`]: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`fifo(7)`]: https://man7.org/linux/man-pages/man7/fifo.7.html
//! [`file descriptor`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`filedescriptor::poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`io_uring_enter()`]: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html
//! [`io_uring`]: https://kernel.dk/io_uring.pdf
//! [`io_uring`: An Alternative Model]: #io_uring-an-alternative-model
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
//! [`mio::Poll::poll()`]: mio::Poll::poll
//! [`mio_poller`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller
//! [`mio`]: mio
//! [`pipe(2)`]: https://man7.org/linux/man-pages/man2/pipe.2.html
//! [`poll()`]: https://man7.org/linux/man-pages/man2/poll.2.html
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
//! [`subscribe()`]: ThreadSafeGlobalState::subscribe
//! [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [`tx.send()`]: tokio::sync::broadcast::Sender::send
//! [`wake()`]: RRTWaker::wake
//! [completions]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [fds]: https://en.wikipedia.org/wiki/File_descriptor
//! [file descriptors]: https://en.wikipedia.org/wiki/File_descriptor
//! [inversion of control]: https://en.wikipedia.org/wiki/Inversion_of_control
//! [kernel]: https://en.wikipedia.org/wiki/Kernel_(operating_system)
//! [known Darwin limitation]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [signals]: https://en.wikipedia.org/wiki/Signal_(IPC)
//! [sockets]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [system call]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [why user-provided?]: #why-is-threadwaker-user-provided

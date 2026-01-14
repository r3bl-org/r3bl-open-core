// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words epoll kqueue SIGWINCH syscall syscalls SQPOLL IORING eventfd signalfd
// cspell:words pollable Proactor demultiplexing

//! Generic infrastructure for the Resilient Reactor Thread (RRT) pattern.
//!
//! The RRT pattern manages a dedicated worker thread that:
//! 1. [Blocks on I/O] ([`stdin`], [`sockets`], [`signals`])
//! 2. Broadcasts events to async consumers via [`broadcast`] channel
//! 3. Handles graceful shutdown when all consumers disconnect
//! 4. Supports thread restart/reuse with generation tracking
//!
//! # Understanding "Blocks on I/O"
//!
//! The RRT pattern's core assumption is that the worker thread **blocks** while waiting
//! for I/O. But what does "blocking" actually mean? This section clarifies the
//! terminology and establishes why the "Blocks on I/O" claim above holds for various
//! I/O backends.
//!
//! ## [`epoll`]/[`mio`] ([`mio_poller`] Implementation)
//!
//! [`mio`] is a low-level cross-platform I/O library that provides a unified Rust
//! interface over OS-specific I/O notification mechanisms like [`epoll`] (Linux) and
//! [`kqueue`] (macOS).
//!
//! With these backends, the thread blocks inside a [system call] (like [`poll()`] or
//! [`epoll_wait()`]), waiting on one or more [file descriptors] for **readiness** — a
//! notification that an [`fd`] has data available. The thread then performs the actual
//! I/O operation itself:
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
//! The "blocks on I/O" claim holds for various I/O sources when using [`epoll`]/[`mio`]:
//!
//! | I/O Backend                                  | Blocks? | Notes                                                                                                                          |
//! | :------------------------------------------- | :------ | :----------------------------------------------------------------------------------------------------------------------------- |
//! | [`epoll`]/[`mio`] + [`stdin`]                | Yes     | See [`mio_poller`]                                                                                                             |
//! | [`epoll`]/[`mio`] + [sockets]                | Yes     | TCP, UDP, Unix domain                                                                                                          |
//! | [`epoll`]/[`mio`] + [signals]                | Yes     | Signals are async interrupts (not [`pollable`]); [`signalfd(2)`] or [`signal-hook`] wraps them as [`fd`]s (see [`mio_poller`]) |
//! | [`epoll`]/[`mio`] + [`pipe(2)`]/[`fifo(7)`]  | Yes     | Example of other [`pollable`] [`fd`]s: unidirectional IPC - [`pipe(2)`]=anonymous, [`fifo(7)`]=named                           |
//!
//! The thread blocks in [`epoll_wait()`] until the kernel signals readiness. The blocking
//! behavior is identical regardless of **what** you're waiting on:
//! 1. [`stdin`]
//! 2. [`sockets`]
//! 3. [`signals`] via [`signalfd(2)`] or [`signal-hook`]
//! 4. any [`pollable`] [`file descriptor`]
//!
//! ## `io_uring` Compatibility
//!
//! For [`io_uring`], we recommend blocking on [`io_uring_enter()`] (see [io_uring: An
//! Alternative Model]). This preserves the blocking behavior that RRT depends on, while
//! gaining [`io_uring`]'s performance benefits. [`io_uring`]'s two other non-blocking
//! modes break the RRT assumption and require a different pattern.
//!
//! # Architecture Overview
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                    RESILIENT REACTOR THREAD (Generic)                  │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │                                                                        │
//! │  ThreadSafeGlobalState<W, E>                                           │
//! │  ├── Mutex<Option<Arc<ThreadState<W, E>>>>                             │
//! │  │   └── ThreadState                                                   │
//! │  │       ├── broadcast_tx: Sender<E>    (event broadcast)              │
//! │  │       ├── liveness: ThreadLiveness   (running state + generation)   │
//! │  │       └── waker: W                   (interrupt blocked thread)     │
//! │  │                                                                     │
//! │  └── allocate::<F>() → SubscriberGuard<W, E>                           │
//! │      ├── Fast path: thread running → reuse                             │
//! │      └── Slow path: thread terminated → spawn new                      │
//! │                                                                        │
//! │  SubscriberGuard<W, E>                                                 │
//! │  ├── receiver: Receiver<E>    (broadcast subscription)                 │
//! │  ├── state: Arc<ThreadState>  (for waker access on drop)               │
//! │  └── Drop: decrements receiver_count, wakes thread                     │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## The RRT Contract And Benefits
//!
//! 1. **Thread-safe global state** — [`ThreadSafeGlobalState<W, E>`] is the type you use
//!    to declare your own `static` singleton:
//!
//!    <!-- It is ok to use ignore here - example of static singleton declaration -->
//!
//!    ```ignore
//!    static SINGLETON: ThreadSafeGlobalState<_, _> = ThreadSafeGlobalState::new();
//!    ```
//!
//!    This type wraps a [`Mutex<Option<Arc<ThreadState>>>`] because [`syscalls`] aren't
//!    `const`. The state is created at runtime (see [`ThreadSafeGlobalState`] for
//!    details). See [`mio_poller`]'s [`SINGLETON`] for a concrete example.
//!
//! 2. **State machine** — [`ThreadState`] can be created, destroyed, and reused. On
//!    spawn, [`allocate()`] populates the singleton with a fresh [`ThreadState`]. On
//!    exit, [`ThreadLiveness`] marks it terminated. On restart, [`allocate()`] replaces
//!    the old state with fresh resources. Generation tracking distinguishes fresh
//!    restarts from reusing an existing thread.
//!
//! 3. **Contract preservation** — Async consumers never see broken promises; the
//!    [`broadcast`] channel decouples producers from consumers. This unlocks two key
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
//! `Poll` must move to the spawned thread. Meanwhile, the [`ThreadState`] needs the waker
//! stored in it so [`SubscriberGuard`]s can call [`wake()`].
//!
//! The solution is a **two-phase setup** via [`ThreadWorkerFactory`]:
//! 1. [`Factory::setup()`] creates **both** worker and waker together
//! 2. The waker is stored in [`ThreadState`] for subscribers to call [`wake()`]
//! 3. The worker moves to the spawned thread (owns `Poll`, does the actual work)
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
//! # // --- Hidden boilerplate: Error type with required impls ---
//! # #[derive(Debug)]
//! # struct MyError;
//! # impl std::fmt::Display for MyError {
//! #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//! #         write!(f, "setup error")
//! #     }
//! # }
//! # impl std::error::Error for MyError {}
//! #
//! // 1. Define your waker (how to interrupt your blocking call)
//! struct MyWaker(mio::Waker);
//!
//! impl ThreadWaker for MyWaker {
//!     fn wake(&self) -> std::io::Result<()> {
//!         self.0.wake()
//!     }
//! }
//!
//! // 2. Define your worker (the actual work loop)
//! struct MyWorker { /* resources */ }
//!
//! impl ThreadWorker for MyWorker {
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
//! impl ThreadWorkerFactory for MyWorkerFactory {
//!     type Event = MyEvent;
//!     type Worker = MyWorker;
//!     type Waker = MyWaker;
//!     type SetupError = MyError;
//!
//!     fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError> {
//!         todo!("Create coupled worker and waker")
//!     }
//! }
//!
//! // 4. Create a static global state
//! static GLOBAL: ThreadSafeGlobalState<MyWaker, MyEvent> =
//!     ThreadSafeGlobalState::new();
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 5. Allocate subscriptions
//! let guard = GLOBAL.allocate::<MyWorkerFactory>()?;
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
//! - **`types`**: Core traits ([`ThreadWaker`], [`ThreadWorker`],
//!   [`ThreadWorkerFactory`])
//! - **`thread_liveness`**: Thread lifecycle state ([`ThreadLiveness`],
//!   [`LivenessState`])
//! - **`thread_state`**: Shared state container ([`ThreadState`])
//! - **`subscriber_guard`**: RAII subscription guard ([`SubscriberGuard`])
//! - **`thread_safe_global_state_manager`**: Global state manager
//!   ([`ThreadSafeGlobalState`])
//!
//! # `io_uring`: An Alternative Model
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
//! The [`ThreadWaker`] implementation would need adjustment for [`io_uring`]. Options:
//!
//! 1. **eventfd registered with `io_uring`** — Submit a read on an eventfd, wake by
//!    writing to it
//! 2. **[`IORING_OP_MSG_RING`]** — [`io_uring`]'s native cross-ring messaging (Linux
//!    5.18+)
//! 3. **Cancellation** — Submit [`IORING_OP_ASYNC_CANCEL`] to interrupt pending
//!    operations
//!
//! The RRT's [`ThreadWaker`] trait already abstracts this, so the change would be
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
//! [`Mutex<Option<Arc<ThreadState>>>`]: std::sync::Mutex
//!
//! [Actor]: https://en.wikipedia.org/wiki/Actor_model
//! [Blocks on I/O]: #understanding-blocks-on-io
//! [CQ]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [Linked operations]: https://man7.org/linux/man-pages/man3/io_uring_prep_link.3.html
//! [Proactor]: https://en.wikipedia.org/wiki/Proactor_pattern
//! [Reactor]: https://en.wikipedia.org/wiki/Reactor_pattern
//! [Registered FDs]: https://man7.org/linux/man-pages/man3/io_uring_register_files.3.html
//! [Registered buffers]: https://man7.org/linux/man-pages/man3/io_uring_register_buffers.3.html
//! [Syscall]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`Arc`]: std::sync::Arc
//! [`allocate()`]: ThreadSafeGlobalState::allocate
//! [`Continuation`]: crate::core::common::Continuation
//! [`Factory::setup()`]: ThreadWorkerFactory::setup
//! [`IORING_OP_ASYNC_CANCEL`]: https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html
//! [`IORING_OP_MSG_RING`]: https://man7.org/linux/man-pages/man3/io_uring_prep_msg_ring.3.html
//! [`LivenessState`]: crate::core::resilient_reactor_thread::LivenessState
//! [`Mutex`]: std::sync::Mutex
//! [`Option`]: std::option::Option
//! [`SINGLETON`]: crate::terminal_lib_backends::direct_to_ansi::input::input_device_impl::global_input_resource::SINGLETON
//! [`SQPOLL`]: https://man7.org/linux/man-pages/man2/io_uring_setup.2.html
//! [`SubscriberGuard`]: crate::core::resilient_reactor_thread::SubscriberGuard
//! [`ThreadLiveness`]: crate::core::resilient_reactor_thread::ThreadLiveness
//! [`ThreadSafeGlobalState`]: crate::core::resilient_reactor_thread::ThreadSafeGlobalState
//! [`ThreadState::should_self_terminate()`]: ThreadState::should_self_terminate
//! [`ThreadState`]: crate::core::resilient_reactor_thread::ThreadState
//! [`ThreadWaker`]: crate::core::resilient_reactor_thread::ThreadWaker
//! [`ThreadWorkerFactory`]: crate::core::resilient_reactor_thread::ThreadWorkerFactory
//! [`ThreadWorker`]: crate::core::resilient_reactor_thread::ThreadWorker
//! [`broadcast`]: tokio::sync::broadcast
//! [`epoll_wait()`]: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`fifo(7)`]: https://man7.org/linux/man-pages/man7/fifo.7.html
//! [`file descriptor`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`io_uring_enter()`]: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html
//! [`io_uring_enter()`]: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html
//! [`io_uring`]: https://kernel.dk/io_uring.pdf
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
//! [`mio_poller`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller
//! [`mio_poller`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller
//! [`pipe(2)`]: https://man7.org/linux/man-pages/man2/pipe.2.html
//! [`poll()`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`poll_once()`]: ThreadWorker::poll_once
//! [`pollable`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`read(2)`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
//! [`signal-hook`]: https://docs.rs/signal-hook
//! [`signalfd(2)`]: https://man7.org/linux/man-pages/man2/signalfd.2.html
//! [`signals`]: https://en.wikipedia.org/wiki/Signal_(IPC)
//! [`sockets`]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [`stdin`]: std::io::stdin
//! [`wake()`]: ThreadWaker::wake
//! [completions]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [file descriptors]: https://en.wikipedia.org/wiki/File_descriptor
//! [io_uring: An Alternative Model]: #io_uring-an-alternative-model
//! [kernel]: https://en.wikipedia.org/wiki/Kernel_(operating_system)
//! [signals]: https://en.wikipedia.org/wiki/Signal_(IPC)
//! [sockets]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [system call]: https://man7.org/linux/man-pages/man2/syscalls.2.html

mod subscriber_guard;
mod thread_liveness;
mod thread_safe_global_state_manager;
mod thread_state;
mod types;

pub use subscriber_guard::*;
pub use thread_liveness::*;
pub use thread_safe_global_state_manager::*;
pub use thread_state::*;
pub use types::*;

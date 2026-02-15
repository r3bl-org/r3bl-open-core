// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words epoll kqueue SIGWINCH syscall syscalls SQPOLL IORING eventfd signalfd
// cspell:words pollable Proactor demultiplexing injectables threadwaker IOCP EINVAL
// cspell:words kqueuefd filedescriptor rrtwaker EINTR errno kevent WezTerm

//! Reusable infrastructure for the Resilient Reactor Thread (RRT) pattern implementation.
//!
//! # Blocking Calls and Async Code Don't Mix
//!
//! The **Resilient Reactor Thread** pattern bridges [blocking I/O] with [async Rust].
//! Calling a blocking [`syscall`] (like [`read(2)`] on [`stdin`]) from an [async task]
//! running on an [executor thread in the `tokio` async runtime] is "**not ok**".
//!
//! The blocking [`syscall`] occupies both the task *and* its executor [thread] until it
//! unblocks on its own. There is no guarantee if or when it will unblock, and no way to
//! cancel it. If it never unblocks, this can starve other tasks from running and cause
//! issues during [async runtime] shutdown.
//!
//! How "**not ok**" things can get depends on your [`tokio`] runtime configuration:
//!
//! - 🐢 **[Multi-threaded runtime]**: This is the default runtime. It uses a [thread
//!   pool] of worker [thread]s (typically one per CPU core). When a [blocking call]
//!   stalls one [thread], its [async task] is stuck - until the call unblocks on its own.
//!   Other worker [thread]s can steal queued tasks from the stalled [thread], but latency
//!   is introduced - and if all worker [thread]s are busy, those tasks wait too. This
//!   results in reduced throughput and degraded performance. However, it can be a total
//!   failure if this code runs in the main event loop of your app - in which case the app
//!   becomes unresponsive to user input and appears frozen to the end user.
//!
//! - 🧊 **[Single-threaded runtime]**: Turns the calling [thread] into the executor via
//!   [`block_on()`]. All tasks run on this one [thread], cooperatively yielding at
//!   `.await` points. If any task makes a blocking call (which doesn't unblock on its
//!   own), then it never yields - the runtime hangs and **nothing else runs**.
//!
//! In practice, both runtimes can produce the same result for [TUI] and
//! [`readline_async`] apps - **the app becomes unresponsive** to the end user. The end
//! user types and nothing happens.
//!
//! In the case of the multi-threaded runtime, the fact that it is responsive is cold
//! comfort to the end user - if the input pipeline is dead, it doesn't matter that the
//! [`tokio`] scheduler can work-steal. While the distinction between a runtime-freeze and
//! a UI-freeze is technically important, it is invisible to the end user.
//!
//! To add insult to injury, the end user might not even be able to cleanly exit the
//! frozen / unresponsive app. It's a triple whammy:
//!
//! 1. **App won't exit normally** - The app's own quit mechanism (e.g., pressing `q`) is
//!    blocked because the input pipeline is dead. The app can't read the keystroke.
//! 2. **`Ctrl+C` doesn't work** - The end user escalates with [`SIGINT`], which does
//!    interrupt the blocked [`syscall`] (it returns [`EINTR`]), but the signal handler
//!    can only call [async-signal-safe] functions - it can't run the [async runtime]
//!    shutdown sequence. And with [`SA_RESTART`], the kernel may auto-restart the
//!    [`syscall`], making the interruption invisible to the app entirely.
//! 3. **[`kill -9`] leaves a mess** - The end user is forced to [`kill -9`] the process
//!    from another terminal, which skips all the app's cleanup code. The terminal can get
//!    stuck in [raw mode] (since the code to restore [cooked mode] can't run), forcing
//!    the end user to run [`reset`] or [`stty sane`] before they can use their terminal
//!    again.
//!
//! # What Is the RRT Pattern?
//!
//! This [design pattern] avoids the problems listed above and allows async code to
//! consume events from blocking sources like [`stdin`], [`sockets`], and [`signals`] by:
//!
//! 1. Never blocking the main event loop [thread] - blocking it would cause the
//!    unresponsive-app triple whammy described above. Using the [Multi-threaded runtime]
//!    in this scenario wouldn't help avoid this outcome.
//!
//! 2. Never blocking the [async runtime] - blocking it would hang the [Single-threaded
//!    runtime] so nothing else runs, and degrade performance (reduced throughput,
//!    increased latency) for the [Multi-threaded runtime].
//!
//! ## How It Works
//!
//! RRT manages its own dedicated [thread] to isolate [blocking I/O], and owns a
//! [`broadcast channel`] to decouple the lifecycle of this [thread] from the async
//! consumers (in your [TUI] or [`readline_async`] app).
//!
//! **Async consumer isolation** - This channel (stored in [`RRT`] via [`OnceLock`])
//! completely isolates async consumers from the dedicated [thread]'s lifecycle. It
//! outlives every thread generation and is never replaced - [`RRT`] can reuse, destroy,
//! or relaunch the [thread] without affecting any async consumers.
//!
//! **Thread creation and reuse** - You start by declaring an [`RRT`] [singleton] in your
//! code, and providing your implementation of the [`RRTFactory`] trait. No thread is
//! created when the [singleton] is loaded into memory - only when the first async
//! consumer calls [`subscribe()`] does it create a single dedicated [thread]. Async
//! consumers that call [`subscribe()`] are referred to as **subscribers** throughout
//! these docs. If more subscribers join, they reuse this same thread - there is never
//! more than one at a time.
//!
//! **Cooperative thread shutdown** - Generally speaking, a [thread] can always
//! self-terminate (its code returns). But forcibly terminating it from outside - whether
//! from another [thread] in the same process or from another process (ie, preemptive
//! shutdown) - is unsafe in most OSes ([Linux], [macOS], [Windows]) and Rust doesn't
//! expose it at all ([Rust discussion], [Rust workarounds]). This is why RRT implements
//! cooperative shutdown instead - when an async consumer drops its [`SubscriberGuard`],
//! the guard's [`Drop`] implementation triggers this sequence:
//!
//! 1. Drops the broadcast [`Receiver`] first, atomically decrementing the channel's
//!    [`receiver_count()`].
//!
//! 2. Calls [`wake()`] on your [`RRTWaker`] trait implementation. This causes
//!    [`run_worker_loop()`] to wake up. This is cleaner than using POSIX [signals] to
//!    interrupt a blocking [`syscall`] - signal handlers can only call
//!    [async-signal-safe] functions, and [`SA_RESTART`] can make the interruption
//!    invisible to the app. The [`syscall`] does return [`EINTR`] when interrupted, but
//!    relying on this is fragile.
//!
//! 3. The [thread] wakes up and checks [`receiver_count()`] - if zero, it
//!    self-terminates. If new subscribers have appeared [in the meantime], RRT reuses the
//!    thread and continues running.
//!
//! **Thread cleanup** - After the [thread] self-terminates (step 3 above),
//! [`run_worker_loop()`] returns, and your [`RRTWorker`] trait implementation goes out of
//! scope, triggering [RAII] cleanup via [`Drop`] on the OS resources it owns (like
//! [`fds`]).
//!
//! **Thread relaunch** - When an async consumer calls [`subscribe()`] again on the
//! singleton, this allocates OS resources and starts a new [thread] (with a new
//! [generation]). [`subscribe()`] invokes [`create()`] on your [`RRTFactory`] trait
//! implementation, which returns, among other things, a fresh instance of your
//! [`RRTWorker`] trait implementation that allocates OS resources like [`fds`].
//!
//! **Self-healing thread restart** - When your [`RRTWorker`] trait implementation
//! encounters a recoverable error (e.g., the OS [event mechanism] fails mid-operation),
//! it returns [`Continuation::Restart`] from [`poll_once()`]. The framework then handles
//! the restart sequence automatically using your [`RestartPolicy`]. This differs from
//! **thread relaunch** (above): relaunch happens externally when [`subscribe()`] finds
//! the thread terminated. Self-healing restart happens *within* the running thread - no
//! subscriber action needed, no thread to respawn. See [self-healing restarts] below for
//! the full sequence.
//!
//! ## Self-Healing Restart Details
//!
//! When [`poll_once()`] returns [`Continuation::Restart`], the framework executes the
//! following sequence:
//!
//! 1. Your current [`RRTWorker`] trait implementation is dropped and [RAII] cleanup
//!    releases OS resources.
//! 2. The framework sleeps for the configured delay (see [`RestartPolicy`]).
//! 3. [`RRTFactory::create()`] is called to create a fresh [`RRTWorker`] + [`RRTWaker`]
//!    pair. The new [`RRTWorker`] instance allocates new OS resources, and the
//!    [`RRTWaker`] instance is bound to these resources, and can wake the thread when
//!    needed.
//! 4. The new [`RRTWaker`] replaces the old one in the [shared wrapper] (so existing
//!    subscribers target the new [`RRTWorker`]).
//! 5. The [poll loop] resumes with the fresh [`RRTWorker`].
//!
//! If [`create()`] itself fails (e.g., OS resources exhausted), the framework retries on
//! the pre-existing thread until either [`create()`] succeeds or the [restart budget] is
//! exhausted. [`create()`] only allocates OS resources - it never spawns a thread. Only
//! [`subscribe()`] affects thread lifecycle (spawning or relaunching).
//!
//! **[`RestartPolicy`]** controls the [restart budget]. Override it, for your needs, by
//! implementing [`restart_policy()`] on your [`RRTFactory`]. See
//! [`RestartPolicy::default()`] for the default configuration and [scenario examples].
//!
//! **When the [restart budget] is exhausted**, the framework sends
//! [`RRTEvent::Shutdown(RestartPolicyExhausted)`] to all subscribers, then the [poll
//! loop] exits. [`run_worker_loop()`] creates a [`TerminationGuard`] as a local [RAII]
//! variable at entry, so when the function returns, [`TerminationGuard`]'s [`Drop`] runs
//! automatically - clearing the [`RRTWaker`] and marking [`liveness`] as terminated. A
//! future [`subscribe()`] call can then relaunch a fresh thread.
//!
//! **[`RRTEvent`] - two-tier event model.** The [`broadcast channel`] carries
//! [`RRTEvent<F::Event>`] instead of raw [`F::Event`], cleanly separating domain events
//! from framework infrastructure events:
//!
//! | Tier    | Variant                         | Producer  | Example                                |
//! | :------ | :------------------------------ | :-------- | :------------------------------------- |
//! | Domain  | [`RRTEvent::Worker(E)`]         | Your code | Keyboard input, resize signal          |
//! | Infra   | [`RRTEvent::Shutdown(reason)`]  | Framework | Restart exhausted, [`RRTWorker`] panic |
//!
//! Subscribers should handle both tiers in their event loop:
//!
//! <!-- It is ok to use ignore here - example of handling both tiers of events -->
//!
//! ```ignore
//! match rrt_event {
//!     RRTEvent::Worker(domain_event) => { /* handle normally */ }
//!     RRTEvent::Shutdown(reason)     => { /* graceful degradation */ }
//! }
//! ```
//!
//! ## Thread Termination Paths
//!
//! The dedicated [thread] can exit through three paths:
//!
//! | Path                  | Trigger                                        | Subscribers notified?                                | Recovery                   |
//! | :-------------------- | :--------------------------------------------- | :--------------------------------------------------- | :------------------------- |
//! | **Stop**              | [`poll_once()`] returns [`Continuation::Stop`] | No (they caused it by dropping guards)               | [`subscribe()`] relaunches |
//! | **Restart exhausted** | [`RestartPolicy`] budget depleted              | Yes - [`RRTEvent::Shutdown(RestartPolicyExhausted)`] | [`subscribe()`] relaunches |
//! | **Panic**             | [`poll_once()`] panics                         | Yes - [`RRTEvent::Shutdown(Panic)`]                  | [`subscribe()`] relaunches |
//!
//! **Panics in [`poll_once()`] on the dedicated thread are caught via [`catch_unwind`]**
//! in [`run_worker_loop()`]. No restart is attempted - a panic signals a logic bug, not a
//! transient resource issue that [self-healing restarts] can fix.
//!
//! ## What's in a name? 😛
//!
//! | Word          | Meaning                                                                                                                                                                                    |
//! | :------------ | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
//! | **Resilient** | [Thread] can stop or crash and be relaunched with [generation] tracking; [`RestartPolicy`] for self-healing restarts; [`broadcast channel`] isolates async consumers from thread lifecycle |
//! | **Reactor**   | Reacts to I/O readiness on [`fds`] ([`stdin`], [`sockets`], [`signals`]) via any blocking backend (e.g., [`mio`]/[`epoll`]) with a matching [`RRTWaker`] trait implementation              |
//! | **Thread**    | Dedicated [thread] for [blocking I/O]; cooperative shutdown via [`RRTWaker`] trait implementation & [RAII] cleanup via [`Drop`] on thread exit                                             |
//!
//! ## Mental Model for Web Developers
//!
//! RRT solves the same fundamental problem for [TUI] and [`readline_async`] apps as [Web
//! Workers] does for web browsers - **keeping blocking work off the main execution
//! context**. In [TUI] and [`readline_async`] apps, just like in web browsers, code that
//! blocks the main [thread] freezes the UI and makes it unresponsive to user input.
//!
//! | Web Pattern         | RRT Equivalent                          |
//! | :------------------ | :-------------------------------------- |
//! | `new Worker()`      | [`SINGLETON`].[`subscribe()`]           |
//! | `postMessage()`     | [`Sender::send()`]                      |
//! | `onmessage`         | [`Receiver::recv()`]                    |
//! | `terminate()`       | Drop [`SubscriberGuard`] (auto-cleanup) |
//!
//! There are some key differences though:
//!
//! - **Work type**: [Web Workers] offload CPU-bound work; RRT offloads [blocking I/O] on
//!   [`fds`] ([`stdin`], [`sockets`], [`signals`] via [`signal-hook`]).
//!
//! - **Delivery**: [Web Workers] are 1:1 (one worker, one consumer); RRT uses 1:N
//!   [`broadcast`] (all UI components need resize events).
//!
//! - **Resilience**: [Web Workers] have no built-in self-healing - if a worker crashes,
//!   the consumer must manually create a new one. RRT provides automatic [self-healing
//!   restarts] with a configurable [`RestartPolicy`], and transparent thread relaunch via
//!   [`subscribe()`].
//!
//! - **Lifecycle**: [Web Workers] require manual cleanup (`terminate()` or
//!   `self.close()`); RRT uses [RAII] via [`SubscriberGuard`] - the thread
//!   self-terminates when all guards are dropped.
//!
//! - **Communication**: [Web Workers] use serialized `postMessage`; RRT uses a type-safe
//!   [`broadcast channel`] (no serialization needed).
//!
//! # Understanding "Blocking I/O"
//!
//! The RRT pattern's core invariant is that its dedicated [thread] **blocks** while
//! waiting for I/O. But what does "blocking" actually mean? This section clarifies the
//! terminology and establishes why the "bridges blocking I/O with async Rust" claim above
//! holds for various I/O backends, on various OSes.
//!
//! RRT itself is agnostic to the I/O backend - it just needs *some* blocking mechanism.
//! RRT implementations could block on [`sockets`], [`signals`], or any other [`fd`]-based
//! I/O source.
//!
//! Let's consider a real-world example for RRT - implement async terminal input (blocking
//! [`read(2)`] [`syscall`] on [`fd 0`], aka [`stdin`]). The `crossterm` crate provides
//! this feature, but let's examine what it would take to implement this manually, which
//! this `r3bl_tui` crate does in [`mio_poller`] (for performance, resilience, efficiency,
//! and composability reasons).
//!
//! ## Terminal Input Across OSes
//!
//! Terminal input requires different blocking I/O backends on each OS. This section
//! surveys what it would take to implement this on macOS and Windows, then dives into the
//! Linux implementation using [`mio`] (see [`mio_poller`] for details).
//!
//! 🍎 **macOS**: We can't use [`mio`] to poll [`stdin`]. [`mio`] is a thin wrapper on top
//! of OS polling [`syscalls`]. On [Darwin] it uses [`kqueue`] (the equivalent of Linux's
//! [`epoll`]), which doesn't support terminal [`fds`] (which is a [known Darwin
//! limitation]).
//!
//! - **Background information**: Your [TUI] or [`readline_async`] app's [`stdin`] ([`fd
//!   0`]) is a terminal [`fd`] which is one of the following:
//!
//!   - [`tty`] - A hardware terminal or virtual console (e.g., `Ctrl+Alt+F1` on Linux).
//!     This is the general category for any terminal device. On a virtual console, there
//!     is no windowing system and no terminal emulator process - the kernel directly
//!     handles keyboard input via the keyboard driver, processes it through the [`tty`]
//!     subsystem, and makes the bytes available on your app's [`stdin`].
//!
//!   - [`PTY`] - A pseudoterminal, a software-emulated [`tty`]. When you run your app
//!     inside a terminal emulator (like [WezTerm] or [Alacritty]), the emulator creates a
//!     [`PTY` pair] - it holds the controller end, and your app's [`stdin`] is the
//!     controlled end. The emulator receives OS-level input events from the windowing
//!     system (e.g., [Wayland]), translates them into [terminal escape sequences], and
//!     writes them to the controller end. Those bytes then appear on the controlled end,
//!     for reading, which is your app's [`stdin`].
//!
//! - **The problem**: Each OS has its own I/O polling kernel subsystem and corresponding
//!
//!   | OS       | Kernel subsystem | Syscall          | Supports [`tty`]/[`PTY`] [`fds`]? |
//!   | :------- | :--------------- | :--------------- | :-------------------------------- |
//!   | Linux    | [`epoll`]        | [`epoll_wait()`] | Yes                               |
//!   | [Darwin] | [`kqueue`]       | [`kevent()`]     | No ([`EINVAL`])                   |
//!
//!   **On Linux** [`epoll_wait()`] can poll both [`tty`] and [`PTY`] [`fds`] for
//!   [readiness].
//!
//!   **But on [Darwin]** [`kevent()`] returns [`EINVAL`] (`errno 22, "Invalid argument"`)
//!   when you try to register either type with [`kqueue`]. This is a permanent rejection
//!   - not to be confused with [`EINTR`] (`errno 4, "Interrupted system call"`), where a
//!   POSIX [signal][signals] (like [`SIGINT`] from `Ctrl+C`) interrupts a blocked
//!   [`syscall`] which unblocks and returns [`EINTR`]. In this case, your code can simply
//!   retry the [`syscall`].
//!
//! - **The workaround**: Bypass [`kqueue`] entirely and use [`filedescriptor::poll()`]
//!   instead, which uses [`select(2)`] internally. [`select(2)`] is an older, more
//!   portable polling [`syscall`] that does support [`PTY`]/[`tty`] [`fds`] on macOS.
//!
//! 🪟 **Windows**: [`mio`] uses [`IOCP`], which doesn't support console/[`stdin`] -
//! [`IOCP`] is for file/socket async I/O only. We would need the [Console API] as the
//! blocking mechanism (no async console I/O exists on Windows).
//!
//! ## Terminal Input on Linux - [`mio_poller`]
//!
//! [`mio_poller`] satisfies RRT's "blocking I/O" invariant by using [`mio`] - a thin Rust
//! wrapper over OS-specific I/O primitives.
//!
//! On 🐧 **Linux**, [`mio`] uses [`epoll`], which works with [`PTY`]/[`tty`]. The
//! [thread] blocks inside the [`epoll_wait()`] [`syscall`], waiting on one or more [file
//! descriptors] for [readiness] - a notification that an [`fd`] has data available. The
//! [thread] then performs the actual I/O operation itself:
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
//! This isn't [busy-waiting] - the kernel puts the [thread] to sleep, consuming
//! essentially zero CPU. But the [thread] **cannot do other work** while waiting. That's
//! what makes it "blocking", which is why RRT uses a dedicated [thread] - to keep this
//! blocking [`syscall`] off [executor threads in the async runtime].
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
//! The [thread] blocks in [`mio::Poll::poll()`] until the kernel signals readiness (via
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
//! To get a bird's eye view (from the [TUI] or [`readline_async`] app's perspective) of
//! how terminal input flows from [`stdin`] through the dedicated [thread] to your async
//! consumers - see the [RRT section] in the crate documentation.
//!
//! ## Separation of Concerns and [Dependency Injection] (DI)
//!
//! You and the framework have distinct responsibilities:
//!
//! - **The framework** ([`RRT<F>`]) handles all the [thread] management and lifecycle
//!   boilerplate - spawning the dedicated [thread] (at most one at a time), reusing it if
//!   running, wake signaling, [`broadcast channel`]s, subscriber tracking, and graceful
//!   shutdown.
//! - **You** provide the [`RRTFactory`] trait implementation along with [`RRTWorker`],
//!   [`RRTWaker`], and [`Event`] types. Without your factory concrete type (and the three
//!   other types) to inject ([DI]), the framework has nothing to run.
//!
//!   | Type               | Purpose                                          | Implementation                                          |
//!   | :----------------- | :----------------------------------------------- | :------------------------------------------------------ |
//!   | [`RRTFactory`]     | [`create()`] - both [`RRTWaker`] + [`RRTWorker`] | [Syscalls]: Create [`mio::Poll`], register your [`fds`] |
//!   | [`RRTWorker`]      | [`poll_once()`] - the work loop                  | Your logic: [`poll()`] → handle events → [`tx.send()`]  |
//!   | [`RRTWaker`]       | [`wake()`] - the interrupt mechanism             | Backend-specific (see [why user-provided?])             |
//!   | [`Event`]          | Domain-specific subscriber event data            | Struct/enum sent via [`tx.send()`] from [`poll_once()`] |
//!   | [`RestartPolicy`]  | Config for [self-healing restarts]               | Override [`restart_policy()`] or use [default policy]   |
//!
//!   See the [Example] section for details.
//!
//! ## Design Principles
//!
//! 1. [Inversion of control] (IOC / control flow) - the framework owns the loop (matching
//!    on [`poll_once()`]'s [`Continuation`] return - [`Continue`], [`Stop`], or
//!    [`Restart`]) and creates and manages the [thread] it runs on. You provide the
//!    iteration logic via [`poll_once()`] in your [`RRTWorker`] trait implementation.
//!
//! 2. [Dependency Injection] (DI / composition) - you provide trait implementations (the
//!    "injectables"); the framework orchestrates them together. This is **imperative**
//!    (code-based) [DI]: you provide concrete implementations for these traits -
//!    [`RRTFactory`], [`RRTWorker`], [`RRTWaker`], and a concrete type for [`Event`].
//!    It's not declarative (configuration-based) [DI] where you *declare*
//!    wiring/bindings.
//!
//! 3. **Type safety** - the [`RRTFactory`] trait ensures your concrete types implementing
//!    it and [`RRTWorker`], [`RRTWaker`] traits, along with your concrete type for
//!    [`Event`] all match up correctly at compile time.
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
//! │       F          : RRTFactory          - your factory trait impl       │
//! │       F::Waker   : RRTWaker            - your waker type (from F)      │
//! │       F::Event   : Clone + Send + Sync - your event type (from F)      │
//! │       F::Worker  : RRTWorker           - your worker type (from F)     │
//! │                                                                        │
//! ├─────────────────────┬────────────────────────────┬─────────────────────┤
//! │                     │ FRAMEWORK → RUNS YOUR CODE │                     │
//! │                     └────────────────────────────┘                     │
//! │  RRT<F>  (three top-level fields, each with correct sync primitive)    │
//! │  ├── broadcast_tx: OnceLock<Sender<RRTEvent<F::Event>>>  (once)        │
//! │  ├── waker: OnceLock<Arc<Mutex<Option<F::Waker>>>>  (shared, swapped)  │
//! │  └── liveness: Mutex<Option<Arc<RRTLiveness>>>      (per-generation)   │
//! │                                                                        │
//! │  subscribe() → SubscriberGuard<F::Waker, F::Event>                     │
//! │      ├── Slow path: F::create() → spawn dedicated thread               │
//! │      │   where YOUR CODE F::Worker.poll_once() runs in a loop          │
//! │      └── Fast path: dedicated thread already running → reuse it        │
//! │                                                                        │
//! │  SubscriberGuard<F::Waker, F::Event>                                   │
//! │  ├── receiver: Receiver<RRTEvent<F::Event>> (two-tier events)          │
//! │  ├── waker: Arc<Mutex<Option<F::Waker>>> (always reads current waker)  │
//! │  └── Drop impl: receiver dropped (decrements count), wakes thread      │
//! │      └── Thread wakes → broadcast_tx.receiver_count() == 0 → exits     │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## The RRT Contract and Benefits
//!
//! 1. **Thread-safe global state** - [`RRT<F>`] is the type you use to declare your own
//!    `static` singleton (initialized with a [const expression]). The generic `F:
//!    `[`RRTFactory`] is **the injection point** - when you call [`subscribe()`], the
//!    framework calls [`RRTFactory::create()`] to get your [`RRTWorker`] and [`RRTWaker`]
//!    trait implementations, then spawns a [thread] running your [`RRTWorker`] trait
//!    implementation's [`poll_once()`] in a loop:
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
//!    without becoming invalid - it *can* live arbitrarily long, not that it *must*. The
//!    type may contain `'static` references but no shorter-lived ones. See [`RRT`] for a
//!    detailed explanation of `'static` in trait bounds.
//!
//!    [`RRT`] uses [`OnceLock`] for the broadcast channel and [`RRTWaker`] wrapper
//!    because [`syscalls`] aren't [const expressions] - they must be created at runtime.
//!    See [`RRT`] for a detailed explanation. See [`mio_poller`]'s [`SINGLETON`] for a
//!    concrete example.
//!
//! 2. **State machine** - [`RRT`]'s liveness field tracks thread state. On spawn,
//!    [`subscribe()`] creates fresh [`RRTLiveness`] and swaps in a new [`RRTWaker`]. On
//!    exit, [`TerminationGuard`] clears the [`RRTWaker`] and marks terminated. On
//!    relaunch, [`subscribe()`] replaces the liveness and swaps the [`RRTWaker`].
//!    Generation tracking distinguishes fresh launches from reusing an existing [thread].
//!
//! 3. **Contract preservation** - Async consumers never see broken promises; the
//!    [`broadcast channel`] decouples producers from consumers. This unlocks two key
//!    benefits:
//!
//!   - **Lifecycle flexibility** - Multiple async tasks can subscribe independently.
//!     Consumers can come and go without affecting the dedicated [thread].
//!
//!   - **Resilience** - The [thread] itself can crash and be relaunched; services can
//!     connect, disconnect, and reconnect. The [TUI] or [`readline_async`] app remains
//!     unaffected.
//!
//! ## Two-Phase Setup
//!
//! Creating the dedicated [thread] has an ordering conflict:
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
//! │   │  PROBLEM: After thread::spawn(), Poll is gone - too late to     │   │
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
//! │   Phase 1: RRTFactory::create() - resources only, no thread spawned     │
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
//! │   │    Spawned Thread      │    │     RRT (shared waker wrapper)    │   │
//! │   │    ───────────────     │    │     ─────────────────────────     │   │
//! │   │    Worker moves here   │    │    Waker stored in Arc<Mutex>     │   │
//! │   │    (owns mio::Poll)    │    │    (shared with all subscribers)  │   │
//! │   │                        │◄───│    SubscriberGuards call wake()   │   │
//! │   └────────────────────────┘    └───────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The key insight: **atomic creation, then separation**. Both resources are created
//! together from the same [`mio::Poll`] registry, then split - your [`RRTWaker`] trait
//! implementation is stored in [`RRT`]'s shared [`RRTWaker`] wrapper for subscribers,
//! while your [`RRTWorker`] trait implementation moves to the spawned [thread].
//!
//! ### Why Is [`RRTWaker`] User-Provided?
//!
//! Your [`RRTWaker`] trait implementation is **intrinsically coupled** to your
//! [`RRTWorker`] trait implementation's blocking mechanism. Different I/O backends need
//! different wake strategies:
//!
//! | Blocking on...          | Wake strategy                                  |
//! | :---------------------- | :--------------------------------------------- |
//! | [`mio::Poll`]           | [`mio::Waker`] (triggers [`epoll`]/[`kqueue`]) |
//! | TCP [`accept()`]        | Connect-to-self pattern                        |
//! | Pipe [`read(2)`]        | Self-pipe trick (write a byte)                 |
//! | [`io_uring`]            | [`eventfd`] or [`IORING_OP_MSG_RING`]          |
//!
//! [`RRT`] can't know how you're blocking - it just calls [`poll_once()`] in a loop. Only
//! you know how to interrupt your specific blocking call.
//!
//! The coupling is also at the resource level: a [`mio::Waker`] is created FROM the
//! [`mio::Poll`]'s registry. If the poll is dropped ([thread] exits), your [`RRTWaker`]
//! trait implementation becomes useless. That's why [`Factory::create()`] returns both
//! together.
//!
//! This design gives [`RRT`] flexibility: it works with [`mio`] today and [`io_uring`]
//! tomorrow without [`RRT`] changes.
//!
//! ## The Inherent Race Condition
//!
//! There's an unavoidable race window between when a receiver drops and when the [thread]
//! checks if it should exit:
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
//! The [kernel] schedules [thread]s independently, so there's no guarantee when the
//! dedicated [thread] will wake up after [`wake()`] is called. The RRT pattern handles
//! this correctly by checking the **current** [`receiver_count()`] at exit time, not the
//! count when [`wake()`] was called.
//!
//! # How to Use It
//!
//! Your journey begins with the [`RRT`] struct itself, which requires this generic
//! argument - your implementation of the [`RRTFactory`] trait, where your business logic
//! lives. The factory is the injection point for your code into [`RRT`].
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
//!     fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation {
//!         todo!("Do one iteration of work, broadcast events via RRTEvent::Worker(...)")
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
//! - **`rrt_di_traits`**: Core traits ([`RRTWaker`], [`RRTWorker`], [`RRTFactory`])
//! - **`rrt_event`**: Two-tier event model ([`RRTEvent`], [`ShutdownReason`])
//! - **`rrt_liveness`**: [Thread] lifecycle state ([`RRTLiveness`], [`LivenessState`])
//! - **`rrt_restart_policy`**: Self-healing configuration ([`RestartPolicy`])
//! - **`rrt_subscriber_guard`**: RAII subscription guard ([`SubscriberGuard`])
//! - **`rrt`**: Framework entry point ([`RRT`]), [`TerminationGuard`],
//!   [`run_worker_loop()`]
//!
//! # [`io_uring`]: An Alternative Model
//!
//! [`io_uring`] (Linux 5.1+) fundamentally changes the I/O model. Instead of waiting for
//! readiness and then doing I/O yourself, you **submit I/O requests** to the kernel and
//! **receive completions** when they finish. The kernel does the actual I/O
//! asynchronously.
//!
//! [`io_uring`] offers several operating modes with different blocking characteristics:
//!
//! | Mode                                    | Thread blocks?                    | Fits RRT? |
//! | :-------------------------------------- | :-------------------------------- | :-------- |
//! | [`io_uring_enter()`] with wait          | Yes (waiting for [completions])   | Yes       |
//! | [`io_uring_enter()`] non-blocking       | No (just checks [CQ])             | No        |
//! | [`SQPOLL`]                              | No ([kernel] [thread] polls)      | No        |
//!
//! In non-blocking or [`SQPOLL`] modes, the work loop could look like this:
//!
//! ```text
//! Thread: submit_io() ──► do_other_work() ──► check_completions() ──► process ──►
//!         (no block)      (thread active)     (non-blocking peek)
//! ```
//!
//! This **breaks the RRT assumption** - there's nothing to interrupt with [`wake()`]
//! because the [thread] never blocks. You'd need a different pattern entirely.
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
//!     // Data already in buffer - just process it
//!     for cqe in self.ring.completion() {
//!         tx.send(process(cqe));           // I/O already done!
//!     }
//! }
//! ```
//!
//! ## Waker Mechanism Adaptation
//!
//! The [`RRTWaker`] trait implementation would need adjustment for [`io_uring`], since
//! [`mio::Waker`] targets [`epoll`]/[`kqueue`]. Possible alternatives:
//!
//! 1. **[`eventfd`] registered with [`io_uring`]** - Submit a read on an [`eventfd`],
//!    wake by writing to it.
//! 2. **[`IORING_OP_MSG_RING`]** - [`io_uring`]'s native cross-ring messaging (Linux
//!    5.18+).
//! 3. **Cancellation** - Submit [`IORING_OP_ASYNC_CANCEL`] to interrupt pending
//!    operations.
//!
//! The RRT's [`RRTWaker`] trait already abstracts this, so the change would be localized
//! to your [`RRTFactory`] trait implementation.
//!
//! # Why "RRT" and Not Actor/Reactor/Proactor?
//!
//! RRT shares traits with several classic concurrency patterns but doesn't fit neatly
//! into any single category:
//!
//! | Pattern           | Similarity                          | Key Difference                                                                               |
//! | :---------------- | :---------------------------------- | :------------------------------------------------------------------------------------------- |
//! | [Actor]           | Dedicated execution context         | Actors are lightweight (many per [thread]); RRT is one OS [thread] that [blocks on I/O]      |
//! | [Reactor]         | Event demultiplexing, I/O readiness | Reactor typically runs in the main loop; RRT isolates [blocking I/O] on a dedicated [thread] |
//! | [Proactor]        | Async I/O, kernel involvement       | Proactor uses completion callbacks; RRT blocks waiting for readiness                         |
//! | Producer-Consumer | [Thread] produces for consumers     | Producer-Consumer uses 1:1 queues; RRT uses 1:N [`broadcast`]                                |
//!
//! **What makes RRT distinct:**
//!
//! 1. **Blocking by design** - The dedicated [thread] *intentionally* [blocks on I/O].
//!    This isn't a limitation; it's the feature. Blocking on the owned / managed [thread]
//!    keeps the I/O off [executor threads in the async runtime].
//!
//! 2. **Broadcast semantics** - Events go to *all* subscribers (1:N), not a single
//!    consumer. When a terminal resize occurs, every UI component needs to know.
//!
//! 3. **Resilience** - Generation tracking enables graceful [thread] relaunch without
//!    breaking existing subscribers. The "Resilient" in RRT refers to this recovery
//!    capability.
//!
//! 4. **I/O-centric** - RRT is specialized for OS-level I/O ([`stdin`], [signals],
//!    [sockets]), not general message processing.
//!
//! [Actor]: https://en.wikipedia.org/wiki/Actor_model
//! [Alacritty]: https://alacritty.org/
//! [CQ]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [Console API]: https://learn.microsoft.com/en-us/windows/console/console-functions
//! [DI]: https://en.wikipedia.org/wiki/Dependency_injection
//! [Darwin]: https://en.wikipedia.org/wiki/Darwin_(operating_system)
//! [Dependency Injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [Example]: #example
//! [Linked operations]: https://man7.org/linux/man-pages/man3/io_uring_prep_link.3.html
//! [Linux]: https://man7.org/linux/man-pages/man3/pthread_cancel.3.html
//! [Multi-threaded runtime]: tokio::runtime::Builder::new_multi_thread
//! [Proactor]: https://en.wikipedia.org/wiki/Proactor_pattern
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
//! [RRT section]: crate#resilient-reactor-thread-rrt-pattern
//! [Reactor]: https://en.wikipedia.org/wiki/Reactor_pattern
//! [Registered FDs]: https://man7.org/linux/man-pages/man3/io_uring_register_files.3.html
//! [Registered buffers]: https://man7.org/linux/man-pages/man3/io_uring_register_buffers.3.html
//! [Rust discussion]: https://internals.rust-lang.org/t/thread-cancel-support/3056
//! [Rust workarounds]: https://matklad.github.io/2018/03/03/stopping-a-rust-worker.html
//! [Single-threaded runtime]: tokio::runtime::Builder::new_current_thread
//! [Syscall]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [Syscalls]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [TCP]: https://en.wikipedia.org/wiki/Transmission_Control_Protocol
//! [TUI]: crate::tui::TerminalWindow::main_event_loop
//! [UDP]: https://en.wikipedia.org/wiki/User_Datagram_Protocol
//! [Unix domain sockets]: https://en.wikipedia.org/wiki/Unix_domain_socket
//! [Wayland]: https://wayland.freedesktop.org/
//! [Web Workers]: https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API
//! [WezTerm]: https://wezfurlong.org/wezterm/
//! [Windows]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminatethread
//! [`'static` trait bound]: RRT#static-trait-bound-vs-static-lifetime-annotation
//! [`EINTR`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`EINVAL`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`Event`]: RRTWorker::Event
//! [`Factory::create()`]: RRTFactory::create
//! [`IOCP`]: https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports
//! [`IORING_OP_ASYNC_CANCEL`]: https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html
//! [`IORING_OP_MSG_RING`]: https://man7.org/linux/man-pages/man3/io_uring_prep_msg_ring.3.html
//! [`LivenessState`]: crate::core::resilient_reactor_thread::LivenessState
//! [`OnceLock`]: std::sync::OnceLock
//! [`PTY`]: https://man7.org/linux/man-pages/man7/pty.7.html
//! [`PTY` pair]: crate::core::pty::pty_core::pty_types::PtyPair
//! [`RRTEvent::Shutdown(Panic)`]: ShutdownReason::Panic
//! [`RRTEvent::Shutdown(RestartPolicyExhausted)`]: ShutdownReason::RestartPolicyExhausted
//! [`RRTEvent::Shutdown(reason)`]: RRTEvent::Shutdown
//! [`RRTEvent::Shutdown`]: RRTEvent::Shutdown
//! [`RRTEvent::Worker(...)`]: RRTEvent::Worker
//! [`RRTEvent::Worker(E)`]: RRTEvent::Worker
//! [`RRTEvent<F::Event>`]: RRTEvent
//! [`RRTEvent`]: RRTEvent
//! [`RRTFactory::create()`]: RRTFactory::create
//! [`RRTFactory`]: RRTFactory
//! [`RRTLiveness`]: RRTLiveness
//! [`RRTWaker`]: RRTWaker
//! [`RRTWorker`]: RRTWorker
//! [`RRT`]: RRT
//! [`Receiver::recv()`]: tokio::sync::broadcast::Receiver::recv
//! [`Receiver`]: tokio::sync::broadcast::Receiver
//! [`Restart`]: crate::Continuation::Restart
//! [`RestartPolicy`]: RestartPolicy
//! [`RestartPolicy::default()`]: RestartPolicy#impl-Default-for-RestartPolicy
//! [`SA_RESTART`]: https://man7.org/linux/man-pages/man2/sigaction.2.html
//! [`SIGINT`]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [`SINGLETON`]: #how-to-use-it
//! [`SQPOLL`]: https://man7.org/linux/man-pages/man2/io_uring_setup.2.html
//! [`Sender::send()`]: tokio::sync::broadcast::Sender::send
//! [`ShutdownReason`]: ShutdownReason
//! [`Stop`]: crate::Continuation::Stop
//! [`SubscriberGuard`]: SubscriberGuard
//! [`TIME_WAIT`]: https://en.wikipedia.org/wiki/TCP_TIME-WAIT
//! [`TerminationGuard`]: TerminationGuard
//! [`accept()`]: std::net::TcpListener::accept
//! [`block_on()`]: tokio::runtime::Runtime::block_on
//! [`broadcast channel`]: tokio::sync::broadcast
//! [`catch_unwind`]: std::panic::catch_unwind
//! [`broadcast`]: tokio::sync::broadcast
//! [`Continuation`]: crate::Continuation
//! [`Continuation::Restart`]: crate::Continuation::Restart
//! [`Continuation::Stop`]: crate::Continuation::Stop
//! [`Continue`]: crate::Continuation::Continue
//! [const expression]: RRT#const-expression-vs-const-declaration-vs-static-declaration
//! [const expressions]: RRT#const-expression-vs-const-declaration-vs-static-declaration
//! [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
//! [`create()`]: RRTFactory::create
//! [`epoll_wait()`]: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
//! [`fd 0`]: https://man7.org/linux/man-pages/man3/stdin.3.html
//! [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
//! [`fds`]: https://man7.org/linux/man-pages/man2/open.2.html
//! [`fifo(7)`]: https://man7.org/linux/man-pages/man7/fifo.7.html
//! [`F::Event`]: RRTFactory::Event
//! [`filedescriptor::poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`io_uring_enter()`]: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html
//! [`io_uring`]: https://kernel.dk/io_uring.pdf
//! [`io_uring`: An Alternative Model]: #io_uring-an-alternative-model
//! [`kevent()`]: https://man.freebsd.org/cgi/man.cgi?query=kevent
//! [`kill -9`]: https://man7.org/linux/man-pages/man1/kill.1.html
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
//! [`liveness`]: field@RRT::liveness
//! [`mio::Poll::poll()`]: mio::Poll::poll
//! [`mio::Poll`]: mio::Poll
//! [`mio_poller`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller
//! [`mio`]: mio
//! [`pipe(2)`]: https://man7.org/linux/man-pages/man2/pipe.2.html
//! [`poll()`]: mio::Poll::poll
//! [`poll_once()`]: RRTWorker::poll_once
//! [`pollable`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`read(2)`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
//! [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
//! [`reset`]: https://man7.org/linux/man-pages/man1/reset.1.html
//! [`restart_policy()`]: RRTFactory::restart_policy
//! [`run_worker_loop()`]: run_worker_loop
//! [`select(2)`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`signal-hook`]: signal_hook
//! [`signalfd(2)`]: https://man7.org/linux/man-pages/man2/signalfd.2.html
//! [`signals`]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [`sockets`]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [`stdin`]: std::io::stdin
//! [`stty sane`]: https://man7.org/linux/man-pages/man1/stty.1.html
//! [`subscribe()`]: RRT::subscribe
//! [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [`tx.send()`]: tokio::sync::broadcast::Sender::send
//! [`wake()`]: RRTWaker::wake
//! [async Rust]: https://rust-lang.github.io/async-book/
//! [async runtime]: tokio::runtime
//! [async task]: tokio::task
//! [async-signal-safe]: https://man7.org/linux/man-pages/man7/signal-safety.7.html
//! [blocking I/O]: #understanding-blocking-io
//! [blocking call]: #understanding-blocking-io
//! [blocks on I/O]: #understanding-blocking-io
//! [busy-waiting]: https://en.wikipedia.org/wiki/Busy_waiting
//! [completions]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [cooked mode]: mod@crate::core::ansi::terminal_raw_mode#raw-mode-vs-cooked-mode
//! [design pattern]: https://en.wikipedia.org/wiki/Software_design_pattern
//! [event mechanism]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [executor thread in the `tokio` async runtime]: tokio::runtime
//! [executor threads in the async runtime]: tokio::runtime
//! [file descriptors]: https://man7.org/linux/man-pages/man2/open.2.html
//! [generation]: RRTLiveness#generation-tracking
//! [in the meantime]: #the-inherent-race-condition
//! [inversion of control]: https://en.wikipedia.org/wiki/Inversion_of_control
//! [kernel]: https://en.wikipedia.org/wiki/Kernel_(operating_system)
//! [known Darwin limitation]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [macOS]: https://man7.org/linux/man-pages/man3/pthread_cancel.3.html
//! [poll loop]: run_worker_loop
//! [raw mode]: mod@crate::core::ansi::terminal_raw_mode#raw-mode-vs-cooked-mode
//! [readiness]: https://man7.org/linux/man-pages/man7/epoll.7.html#DESCRIPTION
//! [restart budget]: RestartPolicy
//! [scenario examples]: RestartPolicy#example-scenarios
//! [self-healing restarts]: #self-healing-restart-details
//! [shared wrapper]: field@RRT::waker
//! [signals]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [singleton]: #how-to-use-it
//! [sockets]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [terminal escape sequences]: crate::core::ansi
//! [thread]: https://en.wikipedia.org/wiki/Thread_(computing)
//! [thread pool]: https://en.wikipedia.org/wiki/Thread_pool
//! [why user-provided?]: #why-is-rrtwaker-user-provided

mod rrt;
mod rrt_di_traits;
mod rrt_event;
mod rrt_liveness;
mod rrt_restart_policy;
mod rrt_subscriber_guard;

pub use rrt::*;
pub use rrt_di_traits::*;
pub use rrt_event::*;
pub use rrt_liveness::*;
pub use rrt_restart_policy::*;
pub use rrt_subscriber_guard::*;

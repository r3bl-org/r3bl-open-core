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
//! In practice, both runtimes can produce the same result for [`TUI`] and
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
//! consumers (in your [`TUI`] or [`readline_async`] app).
//!
//! **Async consumer isolation** - This channel (stored in [`RRT`] via [`LazyLock`])
//! completely isolates async consumers from the dedicated [thread]'s lifecycle. It
//! outlives every thread generation and is never replaced - [`RRT`] can reuse, destroy,
//! or relaunch the [thread] without affecting any async consumers. The sender half of the
//! channel is sync (since our dedicated [`std::thread::Thread`] is the sender), and the
//! receiver half is async since our ([`TUI`] or [`readline_async`] app) is async.
//!
//! **Thread creation and reuse** - You start by declaring an [`RRT`] [singleton] in your
//! code, and providing your implementation of the [`RRTWorker`] trait. No thread is
//! created when the [singleton] is loaded into memory - only when the first async
//! consumer calls [`try_subscribe()`] does it create a single dedicated [thread]. Async
//! consumers that call [`try_subscribe()`] are referred to as **subscribers** throughout
//! these docs. If more subscribers join, they reuse this same thread - there is never
//! more than one at a time.
//!
//! **Cooperative thread shutdown** - Generally speaking, a [thread] can always
//! self-terminate (its code returns). But forcibly terminating it from outside - whether
//! from another [thread] in the same process or from another process (ie, preemptive
//! shutdown) - is unsafe in most OSes ([Linux], [macOS], [Windows]) and Rust doesn't
//! expose it at all ([Rust discussion], [Rust workarounds]). This is why RRT implements
//! cooperative shutdown instead - when an async consumer drops its [`SubscriberGuard`],
//! the guard triggers a software-interrupt-check-exit sequence that cleanly shuts down
//! the [thread] when no subscribers remain. This is cleaner than using POSIX [signals] to
//! interrupt a blocking [`syscall`] - signal handlers can only call [async-signal-safe]
//! functions, and [`SA_RESTART`] can make the interruption invisible to the app. The
//! [`syscall`] does return [`EINTR`] when interrupted, but relying on this is fragile.
//! See [`RRT`]'s [Thread Lifecycle] for the step-by-step shutdown sequence.
//!
//! **Thread cleanup** - When the [thread] self-terminates, your [`RRTWorker`] trait
//! implementation goes out of scope, triggering [`RAII`] cleanup via [`Drop`] on the OS
//! resources it owns (like [`fds`]). If new subscribers have appeared [in the meantime],
//! RRT reuses the existing [thread] generation. Then [`TerminationGuard`] transitions the
//! [`ThreadState`] to [`Stopped`] and calls [`notify_all()`] on the [`Condvar`], waking
//! any subscribers that are waiting to spawn a fresh thread. See [`RRT`]'s [Thread
//! Lifecycle] for the full sequence.
//!
//! **Thread relaunch** - When an async consumer calls [`try_subscribe()`] after the
//! [thread] has terminated, [`try_subscribe()`] sees [`ThreadState::Stopped`],
//! transitions to [`Starting`] (or handles an internal [`Restarting`] → [`Running`]
//! transition during self-healing), creates a fresh worker/interrupt pair via
//! [`create_and_register_os_sources()`], installs the interrupt handle in a new
//! [`Running`] variant, and spawns a new [thread] with a new [generation]. See
//! [`try_subscribe()`] for the full state-machine flow.
//!
//! **Self-healing thread restart** - When your [`RRTWorker`] trait implementation
//! encounters a recoverable error (e.g., the OS [event mechanism] fails mid-operation),
//! it returns [`Continuation::Restart`] from [`block_until_ready_then_dispatch()`]. The
//! framework then handles the restart sequence automatically using your
//! [`RestartPolicy`]. This differs from **thread relaunch** (above): relaunch happens
//! externally when [`try_subscribe()`] finds [`ThreadState::Stopped`]. Self-healing
//! happens *within* the running thread - no subscriber action needed, no thread to
//! respawn. See [self-healing restarts] below for the full sequence.
//!
//! # The Distributed State Machine
//!
//! The lifecycle management of the RRT is implemented as a distributed state machine,
//! where different components own specific parts of the state and transition logic:
//!
//! - **[`rrt.rs`] (The External Driver)**: Drives transitions from the "outside" (async
//!   side) via [`try_subscribe()`].
//! - **[`rrt_engine.rs`] (The Internal Engine)**: Drives transitions from the "inside"
//!   (thread side) via [`run_worker_loop()`].
//! - **[`rrt_monitor.rs`] (The Controller)**: Synchronizes access and handles
//!   blocking/interrupting via [`ThreadLifecycleMonitor`].
//! - **[`rrt_thread_state.rs`] (The States)**: Defines the discrete lifecycle phases in
//!   [`ThreadState`].
//! - **[`rrt_worker.rs`] (The Continuation Signal)**: Provides the [`RRTWorker`] trait,
//!   where user logic returns a [`Continuation`] signal.
//!
//! # Static vs. Flexible Usage
//!
//! The [`RRT`] struct supports two primary usage patterns, depending on your needs. For
//! detailed usage patterns and code examples (static singleton vs. local variable), see
//! the [Static vs. Flexible Usage] section in the [`RRT`] struct documentation.
//!
//! ## Self-Healing Restart Details
//!
//! When [`block_until_ready_then_dispatch()`] returns [`Continuation::Restart`], the
//! framework transitions [`ThreadState`] from [`Running`] to [`Restarting`], drops the
//! current [`RRTWorker`] and its [`InterruptHandle`], creates a fresh worker/interrupt
//! pair via [`create_and_register_os_sources()`], installs the new [`InterruptHandle`] in
//! a new [`Running`] variant, and resumes the poll loop - all within the same running
//! [thread]. No subscriber action is needed.
//!
//! **[`RestartPolicy`]** controls the [restart budget] (max retries, backoff delay,
//! multiplier). Override it by implementing [`restart_policy()`] on your [`RRTWorker`].
//! See [`RestartPolicy::default()`] for the default configuration and [scenario
//! examples].
//!
//! **When the [restart budget] is exhausted**, the framework transitions to [`Stopping`],
//! sends [`RRTEvent::Shutdown(RestartPolicyExhausted)`] to all subscribers, then exits
//! the [thread]. [`TerminationGuard`] transitions to [`Stopped`] on drop. A future
//! [`try_subscribe()`] call can relaunch a fresh thread.
//!
//! See [`run_worker_loop()`] for the step-by-step restart sequence, including what
//! happens when [`create_and_register_os_sources()`] itself fails and how the backoff
//! delay advances.
//!
//! **[`RRTEvent`] - two-tier event model.** The [`broadcast channel`] carries
//! [`RRTEvent<W::Event>`] instead of raw [`W::Event`], cleanly separating domain events
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
//! The dedicated [thread] can exit through three paths. All paths end at
//! [`ThreadState::Stopped`] via [`TerminationGuard`]'s [`Drop`] impl. Regardless of the
//! path, calling [`try_subscribe()`] again will relaunch a fresh dedicated [thread].
//!
//! | Path                  | Trigger                                       | State Transition                            | Subscribers Notified?                                |
//! | :-------------------- | :-------------------------------------------- | :------------------------------------------ | :--------------------------------------------------- |
//! | **Normal Stop**       | zero-receivers, [`EOF`], or [`Stop`]          | [`Running`] → [`Stopping`] → [`Stopped`]    | No                                                   |
//! | **Restart Exhausted** | [`RestartPolicy`] budget depleted             | [`Restarting`] → [`Stopping`] → [`Stopped`] | Yes ([`RRTEvent::Shutdown(RestartPolicyExhausted)`]) |
//! | **Panic**             | [`block_until_ready_then_dispatch()`] panics  | Any → [`Stopped`] (direct)                  | Yes ([`RRTEvent::Shutdown(Panic)`])                  |
//!
//! **Panics in [`block_until_ready_then_dispatch()`]** are caught and do not crash the
//! process - no restart is attempted, since a panic signals a logic bug, not a transient
//! resource issue that [self-healing restarts] can fix. [`TerminationGuard`]'s [`Drop`]
//! impl guarantees the state reaches [`Stopped`] and calls [`notify_all()`] on the
//! [`Condvar`], waking any subscribers blocked in [`try_subscribe()`]. See
//! [`run_worker_loop()`] for the catch mechanism.
//!
//! ## What's in a name? 😛
//!
//! | Word          | Meaning                                                                                                                                                                                      |
//! | :------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
//! | **Resilient** | [Thread] can stop or crash and be relaunched with [generation] tracking; [`RestartPolicy`] for self-healing restarts; [`broadcast channel`] isolates async consumers from [Thread Lifecycle] |
//! | **Reactor**   | Reacts to I/O readiness on [`fds`] ([`stdin`], [`sockets`], [`signals`]) via any blocking backend (e.g., [`mio`]/[`epoll`]) with a matching [`RRTSoftwareInterrupt`]                         |
//! | **Thread**    | Dedicated [thread] for [blocking I/O]; cooperative shutdown via [`RRTSoftwareInterrupt`] & [`RAII`] cleanup via [`Drop`] on thread exit                                                      |
//!
//! ## Mental Model for Web Developers
//!
//! RRT solves the same fundamental problem for [`TUI`] and [`readline_async`] apps as
//! [Web Workers] does for web browsers - **keeping blocking work off the main execution
//! context**. In [`TUI`] and [`readline_async`] apps, just like in web browsers, code
//! that blocks the main [thread] freezes the UI and makes it unresponsive to user input.
//!
//! | Web Pattern         | RRT Equivalent                          |
//! | :------------------ | :-------------------------------------- |
//! | `new Worker()`      | [`SINGLETON`].[`try_subscribe()`]       |
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
//!   [`try_subscribe()`].
//!
//! - **Lifecycle**: [Web Workers] require manual cleanup (`terminate()` or
//!   `self.close()`); RRT uses [`RAII`] via [`SubscriberGuard`] - the thread
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
//! - **Background information**: Your [`TUI`] or [`readline_async`] app's [`stdin`] ([`fd
//!   0`]) is a terminal [`fd`] which is one of the following:
//!
//!   - [`tty`] - A hardware terminal or virtual console (e.g., `Ctrl+Alt+F1` on Linux).
//!     This is the general category for any terminal device. On a virtual console, there
//!     is no windowing system and no terminal emulator process - the kernel directly
//!     handles keyboard input via the keyboard driver, processes it through the [`tty`]
//!     subsystem, and makes the bytes available on your app's [`stdin`].
//!
//!   - [`PTY`] - A pseudoterminal, a software-emulated [`tty`]. When you run your app
//!     inside a terminal emulator (like [`WezTerm`] or [`Alacritty`]), the emulator
//!     creates a [`PTY` pair] - it holds the controller end, and your app's [`stdin`] is
//!     the controlled end. The emulator receives OS-level input events from the windowing
//!     system (e.g., [`Wayland`]), translates them into [terminal escape sequences], and
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
//!     POSIX [signal][signals] (like [`SIGINT`] from `Ctrl+C`) interrupts a blocked
//!     [`syscall`] which unblocks and returns [`EINTR`]. In this case, your code can
//!     simply retry the [`syscall`].
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
//! | [`mio`] + [sockets]                         | Yes     | [`TCP`], [`UDP`], [Unix domain sockets]                                                                                              |
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
//! To get a bird's eye view (from the [`TUI`] or [`readline_async`] app's perspective) of
//! how terminal input flows from [`stdin`] through the dedicated [thread] to your async
//! consumers - see the [RRT section] in the crate documentation. For the concise
//! step-by-step spawn/reuse/terminate sequence, see [`RRT`]'s [Thread Lifecycle] section.
//!
//! ## Separation of Concerns and [Dependency Injection] (DI)
//!
//! You and the framework have distinct responsibilities:
//!
//! - **The framework** ([`RRT<W>`]) handles all the [thread] management and lifecycle
//!   boilerplate - spawning the dedicated [thread] (at most one at a time), reusing it if
//!   running, software interrupt signaling, [`broadcast channel`]s, subscriber tracking,
//!   and graceful shutdown.
//! - **You** provide the [`RRTWorker`] trait implementation (with its associated
//!   [`RRTSoftwareInterrupt`] and [`Event`] types). Without your worker concrete type
//!   (and these other pieces) to inject ([`DI`]), the framework has nothing to run.
//!
//!   | Type / Trait             | Purpose                                                                                           | Implementation                                                                    |
//!   | :----------------------- | :------------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------- |
//!   | [`RRTWorker`]            | [`create_and_register_os_sources()`], [`block_until_ready_then_dispatch()`], [`restart_policy()`] | Your logic: create resources, poll, handle events                                 |
//!   | [`RRTSoftwareInterrupt`] | Interrupt mechanism (returned by [`create_and_register_os_sources()`])                            | Backend-specific impl (see [why user-provided?])                                  |
//!   | [`Event`]                | Domain-specific subscriber event data                                                             | Struct/enum sent via [`sender.send()`] from [`block_until_ready_then_dispatch()`] |
//!   | [`RestartPolicy`]        | Config for [self-healing restarts]                                                                | Override [`restart_policy()`] or use [default policy]                             |
//!
//!   See the [Example] section for details.
//!
//! ## Design Principles
//!
//! 1. [Inversion of control] (IOC / control flow) - the framework owns the loop (matching
//!    on [`block_until_ready_then_dispatch()`]'s [`Continuation`] return - [`Continue`],
//!    [`Stop`], or [`Restart`]) and creates and manages the [thread] it runs on. You
//!    provide the iteration logic via [`block_until_ready_then_dispatch()`] in your
//!    [`RRTWorker`] trait implementation.
//!
//! 2. [Dependency Injection] (DI / composition) - you provide a trait implementation (the
//!    "injectable"); the framework orchestrates it. This is **imperative** (code-based)
//!    [`DI`] - you provide a concrete implementation of the [`RRTWorker`] trait (which
//!    includes [`create_and_register_os_sources()`],
//!    [`block_until_ready_then_dispatch()`], and optionally [`restart_policy()`]), along
//!    with a concrete type for [`Event`]. It's not declarative (configuration-based)
//!    [`DI`] where you *declare* wiring/bindings.
//!
//! 3. **Type safety** - the [`RRTWorker`] trait ensures your concrete type's associated
//!    [`Event`] and [`RRTSoftwareInterrupt`] types returned by
//!    [`create_and_register_os_sources()`] all match up correctly at compile time.
//!
//! ## Type Hierarchy Diagram
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────────┐
//! │                    RESILIENT REACTOR THREAD (Generic)                          │
//! │    IoC + DI: you implement trait ─► framework orchestrates & calls             │
//! ├────────────────────────────┬──────────────┬────────────────────────────────────┤
//! │                            │  YOUR CODE   │                                    │
//! │                            └──────────────┘                                    │
//! │  SINGLETON:                                                                    │
//! │       static SINGLETON: RRT<W> = ...::new();                                   │
//! │                                                                                │
//! │  The generic param <W>:                                                        │
//! │       W          : RRTWorker              - your worker trait impl             │
//! │       W::Event   : Clone + Send + Sync    - your event type (from W)           │
//! │       W::create_and_register_os_sources()  - returns (W, W::Interrupt) pair    │
//! │                                                                                │
//! ├─────────────────────┬────────────────────────────┬─────────────────────────────┤
//! │                     │ FRAMEWORK → RUNS YOUR CODE │                             │
//! │                     └────────────────────────────┘                             │
//! │  RRT<W> (two top-level fields, each with correct sync primitive)               │
//! │  ├── sender: LazyLock<Sender<RRTEvent<W::Event>>>                       (once) │
//! │  └── shared_state: LazyLock<Arc<ThreadLifecycleMonitor<W>>>    (state+condvar) │
//! │                                                                                │
//! │  try_subscribe() → SubscriberGuard<W>                                          │
//! │      └── match state {                                                         │
//! │          Stopped  → Starting → spawn thread (YOUR CODE runs in loop)           │
//! │          Running  → reuse existing thread                                      │
//! │          _        → condvar.wait(), retry                                      │
//! │      }                                                                         │
//! │                                                                                │
//! │  SubscriberGuard<W>                                                            │
//! │  ├── receiver: Receiver<RRTEvent<W::Event>> (two-tier events)                  │
//! │  ├── interrupt_on_drop: InterruptOnDrop<W> (interrupts thread via monitor)     │
//! │  └── Drop impl: receiver dropped (decrements count), then interrupts thread    │
//! │      └── Thread interrupted → receiver_count() == 0 → Stopping → exits         │
//! │                                                                                │
//! └────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## The RRT Contract and Benefits
//!
//! 1. **Thread-safe global state** - [`RRT<W>`] is the type you use to declare your own
//!    `static` singleton (initialized with a [const expression]). The generic `W:
//!    `[`RRTWorker`] is **the injection point** - when you call [`try_subscribe()`], the
//!    framework calls [`RRTWorker::create_and_register_os_sources()`] to get your worker
//!    instance and coupled [`RRTSoftwareInterrupt`], then spawns a [thread] running your
//!    [`RRTWorker`] trait implementation's [`block_until_ready_then_dispatch()`] in a
//!    loop:
//!
//!    <!-- It is ok to use ignore here - example of static singleton declaration -->
//!
//!    ```ignore
//!    /// From mio_poller implementation:
//!    static SINGLETON: RRT<MioPollWorker> =
//!        RRT::new();
//!
//!    let subscriber_guard = SINGLETON.try_subscribe()?;
//!    ```
//!
//!    The [`'static` trait bound] on `E` means the event type can be held indefinitely
//!    without becoming invalid - it *can* live arbitrarily long, not that it *must*. The
//!    type may contain `'static` references but no shorter-lived ones. See [`RRT`] for a
//!    detailed explanation of `'static` in trait bounds.
//!
//!    [`RRT`] uses [`LazyLock`] for the broadcast channel and [`ThreadLifecycleMonitor`]
//!    because [`syscalls`] aren't [const expressions] - they must be created at runtime.
//!    See [`RRT`] for a detailed explanation. See [`mio_poller`]'s [`SINGLETON`] for a
//!    concrete example.
//!
//! 2. **State machine** - [`RRT`]'s shared [`ThreadLifecycleMonitor`] holds an explicit
//!    5-variant [`ThreadState`] enum ([`Stopped`], [`Starting`], [`Running`],
//!    [`Stopping`], [`Restarting`]) behind a [`Mutex`], paired with a [`Condvar`]. On
//!    spawn, [`try_subscribe()`] transitions [`Stopped`] → [`Starting`] → [`Running`] and
//!    records a new [generation]. On exit, [`TerminationGuard`] transitions to
//!    [`Stopped`] and wakes blocked subscribers via the [`Condvar`]. On relaunch,
//!    [`try_subscribe()`] blocks on the [`Condvar`] until the state reaches [`Stopped`],
//!    then takes ownership and spawns a fresh thread. [Generation] tracking distinguishes
//!    fresh launches from reusing an existing [thread].
//!
//! 3. **Contract preservation** - Async consumers never see broken promises; the
//!    [`broadcast channel`] decouples producers from consumers. This unlocks two key
//!    benefits:
//!
//!   - **Lifecycle flexibility** - Multiple async tasks can subscribe independently.
//!     Consumers can come and go without affecting the dedicated [thread].
//!
//!   - **Resilience** - The [thread] itself can crash and be relaunched; services can
//!     connect, disconnect, and reconnect. The [`TUI`] or [`readline_async`] app remains
//!     unaffected.
//!
//! ## Two-Phase Setup
//!
//! [`create_and_register_os_sources()`] returns a worker + interrupt pair that must be
//! created together but have different destinations: the interrupt handle is wrapped in
//! an [`InterruptHandle`] and installed in the [`Running`] variant of [`ThreadState`]
//! inside [`ThreadLifecycleMonitor`]; the worker stays on the thread's stack. See
//! [`RRT`]'s [two-phase setup] section for the full ordering conflict explanation,
//! diagrams, and destination details.
//!
//!
//! ### Why Is [`RRTSoftwareInterrupt`] User-Provided?
//!
//! Your [`RRTSoftwareInterrupt`] implementation is **intrinsically coupled** to your
//! [`RRTWorker`] trait implementation's blocking mechanism. Different I/O backends need
//! different interrupt strategies:
//!
//! | Blocking on...          | Interrupt strategy                             |
//! | :---------------------- | :--------------------------------------------- |
//! | [`mio::Poll`]           | [`mio::Waker`] (triggers [`epoll`]/[`kqueue`]) |
//! | TCP [`accept()`]        | Connect-to-self pattern                        |
//! | Pipe [`read(2)`]        | Self-pipe trick (write a byte)                 |
//! | [`io_uring`]            | [`eventfd`] or [`IORING_OP_MSG_RING`]          |
//!
//! [`RRT`] can't know how you're blocking - it just calls
//! [`block_until_ready_then_dispatch()`] in a loop. Only you know how to interrupt your
//! specific blocking call.
//!
//! The coupling is also at the resource level: a [`mio::Waker`] is created FROM the
//! [`mio::Poll`]'s registry. If the poll is dropped ([thread] exits), the
//! [`RRTSoftwareInterrupt`] becomes useless. That's why
//! [`create_and_register_os_sources()`] returns both together.
//!
//! This design gives [`RRT`] flexibility: it works with [`mio`] today and [`io_uring`]
//! tomorrow without [`RRT`] changes.
//!
//! ## Historical Context: Race Conditions Eliminated
//!
//! Earlier RRT designs used `Option::is_some()` on a shared interrupt handle slot
//! (specifically `LazyLock<Arc<Mutex<Option<W::Interrupt>>>>`) as the implicit "is the
//! thread alive?" signal. That model conflated interrupt handle storage with liveness
//! state, which made intermediate lifecycle phases (the moments when a thread is
//! *starting up* or *shutting down*) impossible to represent. The result was a class of
//! timing-sensitive race conditions that required ad-hoc spin-wait workarounds.
//!
//! The current design replaces that implicit model with an explicit 5-variant
//! [`ThreadState`] enum behind a [`Mutex`], paired with a [`Condvar`] inside a
//! [`ThreadLifecycleMonitor`]. With every lifecycle phase having its own variant
//! ([`Stopped`], [`Starting`], [`Running`], [`Stopping`], [`Restarting`]), the races
//! below are **structurally impossible**: the bugs only existed in the gap between "what
//! the interrupt handle slot says" and "what the thread is actually doing", and that gap
//! no longer exists.
//!
//! For more info on monitors, watch this [video].
//!
//! These notes are kept as historical record so it's clear *why* the framework moved to
//! the state machine model - both for future maintainers wondering "why all the
//! ceremony?" and for anyone debugging adjacent timing-related code.
//!
//! ### The Exit Decision Race
//!
//! There **was** an unavoidable race window between when a receiver dropped and when the
//! dedicated [thread] checked whether it should exit:
//!
//! ```text
//! Timeline:
//! ─────────────────────────────────────────────────────────────────►
//! trigger_software_           kernel         poll()      check
//! interrupt() called          schedules      returns     receiver_count
//!    │                           │              │           │
//!    └───────────────────────────┴──────────────┴───────────┘
//!    ▲                                                      ▲
//!    │             new subscriber can appear here           │
//!    window start ───────── RACE WINDOW ─────────  window end
//! ```
//!
//! **How it was mitigated:** A two-part handshake. When the last subscriber disconnected,
//! its `Drop` implementation explicitly interrupted the dedicated thread; the thread then
//! re-checked the **current** `receiver_count()` at exit time (not the count from when
//! the interrupt was triggered) before deciding to exit.
//!
//! **How the state machine eliminates it:** The exit decision now happens *under the
//! [`state`] [`Mutex`]* at the top of [`run_worker_loop()`]. A new subscriber arriving
//! during the same window must acquire the same lock to attach - so either the thread
//! sees the new subscriber and continues running, or the thread takes ownership of the
//! state and transitions to [`Stopping`] before any new subscriber can attach. The race
//! window is closed by serializing both decisions through a single lock.
//!
//! ### The Fast-Path Race
//!
//! Also known as the "Dead Thread, Live Subscriber" race. When a subscriber dropped, the
//! dedicated thread initiated its exit sequence. There **was** a microsecond window where
//! the thread had broken out of its I/O loop but hadn't yet dropped its
//! `TerminationGuard` to clear the interrupt handle slot:
//!
//! ```text
//! Timeline:
//! ─────────────────────────────────────────────────────────────────►
//! thread sees 0        try_subscribe()     TerminationGuard
//! receivers and        called by new       drops, clearing
//! exits loop           subscriber          interrupt handle slot
//!    │                    │                    │
//!    └────────────────────┴────────────────────┘
//!    ▲                                         ▲
//!    │    fast path sees interrupt = Some()     │
//!    window start ──── RACE WINDOW ──── window end
//! ```
//!
//! **How it was mitigated:** The previous [`RRT::try_subscribe()`] detected the dying
//! thread on the fast path (interrupt handle slot still `Some`, but `receiver_count() ==
//! 0`) and called `wait_for_thread_exit()`, which spin-waited (yielding the CPU) for the
//! dying thread to clear the slot before forcing the allocation of a fresh thread.
//! Spin-waiting was correct but inelegant.
//!
//! **How the state machine eliminates it:** The interrupt handle no longer lives in a
//! shared [`Option`], it lives only inside the [`Running`] variant of [`ThreadState`].
//! When the thread starts tearing down, it transitions the state from [`Running`] to
//! [`Stopping`] *before* dropping any OS resources. A new subscriber arriving during the
//! teardown window sees [`Stopping`] (or [`Stopped`]) and blocks on the [`Condvar`] until
//! the dying thread fully exits and the state reaches [`Stopped`], at which point the
//! new subscriber takes ownership and spawns a fresh thread. There is no spin-wait, and
//! no "fast path that sees an interrupt handle for a dying thread."
//!
//! ### The Zombie Interrupt Bug
//!
//! Across thread *generations* (when the framework relaunched a thread after the previous
//! one exited), the new thread allocated a fresh [`RRTSoftwareInterrupt`]. But the old
//! interrupt handle, targeting the now-dead [`mio::Poll`] instance from the previous
//! generation, could still be reached by long-lived subscribers that had been holding a
//! clone of the previous interrupt handle.
//!
//! Calling that stale interrupt handle did nothing useful (the target was a dead
//! [`mio::Poll`]), so the new thread never got interrupted by that subscriber's drop.
//! The subscriber had become a "zombie", holding an interrupt handle that pointed at a
//! corpse.
//!
//! **How it was mitigated:** The framework introduced a `SharedInterruptSlot`,
//! specifically `Arc<Mutex<Option<W::Interrupt>>>` - that *all* subscribers held a clone
//! of, via `InterruptSlotReader`. When the thread relaunched, the framework swapped the
//! inner `Option` to the new interrupt handle. This ensured every subscriber always read
//! the *current* interrupt handle, not the one captured at subscribe time.
//!
//! **How the state machine eliminates it:** The interrupt handle now lives inside the
//! [`Running`] variant of [`ThreadState`], wrapped in an [`InterruptHandle`], a newtype
//! that **deliberately does not implement [`Clone`] or [`Copy`]**. The variant lives
//! inside the [`ThreadLifecycleMonitor`], shared via [`Arc`]. Every subscriber holds a
//! clone of the same `Arc<ThreadLifecycleMonitor>` - never a clone of the interrupt
//! handle itself, because the interrupt handle *cannot be cloned*. Subscribers access
//! the interrupt handle only through [`ThreadLifecycleMonitor::interrupt_if_running()`],
//! which locks the state and matches on the [`Running`] variant on every call. When the
//! framework relaunches the thread (`Restarting → Running`), the new [`InterruptHandle`]
//! is installed in the new [`Running`] variant atomically; the old handle is dropped
//! along with the old variant. Every subscriber's next interrupt call goes through the
//! monitor and sees the *current* interrupt handle, never a stale one.
//!
//! This is the same indirection-through-a-shared-handle solution that
//! `SharedInterruptSlot` provided in the old design - but now with a critical upgrade:
//! the indirection is **enforced by the type system**, not by convention. The old design
//! relied on subscribers correctly choosing to hold an `InterruptSlotReader` (a clone of
//! the slot) instead of a `W::Interrupt` (a clone of the interrupt handle). A future
//! maintainer could have silently reintroduced the bug by "optimizing" to capture the
//! interrupt handle at subscribe time. Under the [`InterruptHandle`] model, that
//! optimization is a compile error.
//!
//! # How to Use It
//!
//! Your journey begins with the [`RRT`] struct itself, which requires this generic
//! argument - your implementation of the [`RRTWorker`] trait, where your business logic
//! lives. The worker is the injection point for your code into [`RRT`].
//!
//! ## Example
//!
//! Implementing the RRT pattern for a new use case:
//!
//! ```no_run
//! # use r3bl_tui::core::resilient_reactor_thread::*;
//! # use r3bl_tui::{Continuation, ok};
//! # use tokio::sync::broadcast::Sender;
//! #
//! # // --- Hidden boilerplate: Event + Interrupt types ---
//! # #[derive(Clone, Debug)]
//! # struct MyEvent;
//! # #[derive(Debug)]
//! # struct MyInterrupt;
//! # impl RRTSoftwareInterrupt for MyInterrupt { fn trigger_software_interrupt(&self) {} }
//! #
//! // 1. Define your worker (creates resources + runs the work loop)
//! #[derive(Debug)]
//! struct MyWorker { /* resources, e.g., mio::Poll */ }
//!
//! impl RRTWorker for MyWorker {
//!     type Event = MyEvent;
//!     type Interrupt = MyInterrupt;
//!
//!     fn create_and_register_os_sources() -> miette::Result<(Self, Self::Interrupt)> {
//!         // Create worker with OS resources and a coupled RRTSoftwareInterrupt.
//!         // e.g., create mio::Poll, register fds, create mio::Waker from registry.
//!         Ok((MyWorker { /* ... */ }, MyInterrupt))
//!     }
//!
//!     fn block_until_ready_then_dispatch(&mut self, sender: &Sender<RRTEvent<Self::Event>>) -> Continuation {
//!         todo!("Do one iteration of work, broadcast events via RRTEvent::Worker(...)")
//!     }
//!
//!     fn restart_policy() -> RestartPolicy {
//!         RestartPolicy::default()
//!     }
//! }
//!
//! // 2. Create a static global state (worker type W provides all associated types)
//! static GLOBAL: RRT<MyWorker> =
//!     RRT::new();
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 3. Subscribe to events
//! let subscriber_guard = GLOBAL.try_subscribe()?;
//! # ok!()
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
//! - **[`rrt_worker`]**: Core traits ([`RRTWorker`], [`RRTSoftwareInterrupt`])
//! - **[`rrt_event`]**: Two-tier event model ([`RRTEvent`], [`ShutdownReason`])
//! - **[`rrt_restart_policy`]**: Self-healing configuration ([`RestartPolicy`])
//! - **[`rrt_interrupt_handle`]**: Non-clonable interrupt handle wrapper ([`InterruptHandle`])
//! - **[`rrt_subscriber_guard`]**: RAII subscription guard ([`SubscriberGuard`])
//! - **[`rrt_termination_guard`]**: RAII thread-exit guard ([`TerminationGuard`])
//! - **[`rrt_monitor`]**: State machine and synchronization ([`ThreadLifecycleMonitor`],
//!   [`ThreadState`])
//! - **[`rrt_types`]**: Public API types ([`SubscribeError`])
//! - **[`rrt`]**: Framework entry point ([`RRT`]), [`run_worker_loop()`]
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
//! | [`io_uring_enter()`] non-blocking       | No (just checks [`CQ`])           | No        |
//! | [`SQPOLL`]                              | No ([kernel] [thread] polls)      | No        |
//!
//! In non-blocking or [`SQPOLL`] modes, the work loop could look like this:
//!
//! ```text
//! Thread: submit_io() ──► do_other_work() ──► check_completions() ──► process ──►
//!         (no block)      (thread active)     (non-blocking peek)
//! ```
//!
//! This **breaks the RRT assumption** - there's nothing to interrupt with the
//! [`RRTSoftwareInterrupt`] because the [thread] never blocks. You'd need a different
//! pattern entirely.
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
//! The [`block_until_ready_then_dispatch()`] implementation would change from the current
//! [`epoll`] model:
//!
//! <!-- It is ok to use ignore here - pseudo-code sketch showing epoll readiness-based
//! API pattern -->
//!
//! ```ignore
//! // Current epoll model
//! fn block_until_ready_then_dispatch(&mut self, sender: &Sender<Event>) -> Continuation {
//!     self.poll.poll(&mut events, None)?;  // Block for readiness
//!     for event in &events {
//!         let data = read(fd)?;            // YOU do the I/O
//!         sender.send(data);
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
//! fn block_until_ready_then_dispatch(&mut self, sender: &Sender<Event>) -> Continuation {
//!     // Submit read requests to submission queue
//!     self.ring.submit_read(stdin_fd, &mut buffer)?;
//!
//!     // Block waiting for completions (kernel does I/O during this wait)
//!     self.ring.submit_and_wait(1)?;
//!
//!     // Data already in buffer - just process it
//!     for cqe in self.ring.completion() {
//!         sender.send(process(cqe));           // I/O already done!
//!     }
//! }
//! ```
//!
//! ## Interrupt Mechanism Adaptation
//!
//! The [`RRTSoftwareInterrupt`] implementation would need adjustment for [`io_uring`],
//! since [`mio::Waker`] targets [`epoll`]/[`kqueue`]. Possible alternatives:
//!
//! 1. **[`eventfd`] registered with [`io_uring`]** - Submit a read on an [`eventfd`],
//!    wake by writing to it.
//! 2. **[`IORING_OP_MSG_RING`]** - [`io_uring`]'s native cross-ring messaging (Linux
//!    5.18+).
//! 3. **Cancellation** - Submit [`IORING_OP_ASYNC_CANCEL`] to interrupt pending
//!    operations.
//!
//! The RRT's [`RRTSoftwareInterrupt`] trait already abstracts this, so the change would
//! be localized to your [`RRTWorker::create_and_register_os_sources()`] implementation.
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
//! [`'static` trait bound]: RRT#static-trait-bound-vs-static-lifetime-annotation
//! [`accept()`]: std::net::TcpListener::accept
//! [`Alacritty`]: https://alacritty.org/
//! [`Arc`]: std::sync::Arc
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`block_on()`]: tokio::runtime::Runtime::block_on
//! [`block_until_ready_then_dispatch()`]: RRTWorker::block_until_ready_then_dispatch
//! [`broadcast channel`]: tokio::sync::broadcast
//! [`broadcast`]: tokio::sync::broadcast
//! [`catch_unwind`]: std::panic::catch_unwind
//! [`Condvar`]: std::sync::Condvar
//! [`Continuation::Restart`]: crate::Continuation::Restart
//! [`Continuation::Stop`]: crate::Continuation::Stop
//! [`Continuation`]: crate::Continuation
//! [`Continue`]: crate::Continuation::Continue
//! [`CQ`]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [`create_and_register_os_sources()`]: RRTWorker::create_and_register_os_sources
//! [`DI`]: https://en.wikipedia.org/wiki/Dependency_injection
//! [`EINTR`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`EINVAL`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`epoll_wait()`]: https://man7.org/linux/man-pages/man2/epoll_wait.2.html
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`Event`]: RRTWorker::Event
//! [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
//! [`fd 0`]: https://man7.org/linux/man-pages/man3/stdin.3.html
//! [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
//! [`fds`]: https://man7.org/linux/man-pages/man2/open.2.html
//! [`fifo(7)`]: https://man7.org/linux/man-pages/man7/fifo.7.html
//! [`filedescriptor::poll()`]:
//!     https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`generation`]: RRT::get_thread_generation
//! [`interrupt_if_running()`]: ThreadLifecycleMonitor::interrupt_if_running
//! [`InterruptHandle`]: InterruptHandle
//! [`io_uring_enter()`]: https://man7.org/linux/man-pages/man2/io_uring_enter.2.html
//! [`io_uring`: An Alternative Model]: #io_uring-an-alternative-model
//! [`io_uring`]: https://kernel.dk/io_uring.pdf
//! [`IOCP`]: https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports
//! [`IORING_OP_ASYNC_CANCEL`]:
//!     https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html
//! [`IORING_OP_MSG_RING`]:
//!     https://man7.org/linux/man-pages/man3/io_uring_prep_msg_ring.3.html
//! [`kevent()`]: https://man.freebsd.org/cgi/man.cgi?query=kevent
//! [`kill -9`]: https://man7.org/linux/man-pages/man1/kill.1.html
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
//! [`LazyLock`]: std::sync::LazyLock
//! [`mio::Poll::poll()`]: mio::Poll::poll
//! [`mio::Poll`]: mio::Poll
//! [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
//! [`mio`]: mio
//! [`Mutex`]: std::sync::Mutex
//! [`notify_all()`]: std::sync::Condvar::notify_all
//! [`pipe(2)`]: https://man7.org/linux/man-pages/man2/pipe.2.html
//! [`poll()`]: mio::Poll::poll
//! [`pollable`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`PTY` pair]: crate::pty_engine::pty_pair::PtyPair
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
//! [`read(2)`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
//! [`Receiver::recv()`]: tokio::sync::broadcast::Receiver::recv
//! [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
//! [`Receiver`]: tokio::sync::broadcast::Receiver
//! [`reset`]: https://man7.org/linux/man-pages/man1/reset.1.html
//! [`restart_policy()`]: RRTWorker::restart_policy
//! [`Restart`]: crate::Continuation::Restart
//! [`Restarting`]: ThreadState::Restarting
//! [`RestartPolicy::default()`]: RestartPolicy#impl-Default-for-RestartPolicy
//! [`RestartPolicy`]: RestartPolicy
//! [`rrt.rs`]: mod@rrt
//! [`rrt_engine.rs`]: mod@rrt_engine
//! [`rrt_event`]: mod@rrt_event
//! [`rrt_interrupt_handle`]: mod@rrt_interrupt_handle
//! [`rrt_monitor.rs`]: mod@rrt_monitor
//! [`rrt_monitor`]: mod@rrt_monitor
//! [`rrt_restart_policy`]: mod@rrt_restart_policy
//! [`rrt_subscriber_guard`]: mod@rrt_subscriber_guard
//! [`rrt_termination_guard`]: mod@rrt_termination_guard
//! [`rrt_thread_state.rs`]: mod@rrt_thread_state
//! [`rrt_types`]: mod@rrt_types
//! [`rrt_worker.rs`]: mod@rrt_worker
//! [`rrt_worker`]: mod@rrt_worker
//! [`rrt`]: mod@rrt
//! [`RRT`]: RRT
//! [`RRTEvent::Shutdown(Panic)`]: ShutdownReason::Panic
//! [`RRTEvent::Shutdown(reason)`]: RRTEvent::Shutdown
//! [`RRTEvent::Shutdown(RestartPolicyExhausted)`]: ShutdownReason::RestartPolicyExhausted
//! [`RRTEvent::Shutdown`]: RRTEvent::Shutdown
//! [`RRTEvent::Worker(...)`]: RRTEvent::Worker
//! [`RRTEvent::Worker(E)`]: RRTEvent::Worker
//! [`RRTEvent<W::Event>`]: RRTEvent
//! [`RRTEvent`]: RRTEvent
//! [`RRTSoftwareInterrupt`]: RRTSoftwareInterrupt
//! [`RRTWorker::create_and_register_os_sources()`]:
//!     RRTWorker::create_and_register_os_sources
//! [`RRTWorker`]: RRTWorker
//! [`run_worker_loop()`]: run_worker_loop
//! [`Running`]: ThreadState::Running
//! [`SA_RESTART`]: https://man7.org/linux/man-pages/man2/sigaction.2.html
//! [`select(2)`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`sender.send()`]: tokio::sync::broadcast::Sender::send
//! [`Sender::send()`]: tokio::sync::broadcast::Sender::send
//! [`ShutdownReason`]: ShutdownReason
//! [`SIGINT`]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [`signal-hook`]: signal_hook
//! [`signalfd(2)`]: https://man7.org/linux/man-pages/man2/signalfd.2.html
//! [`signals`]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [`SINGLETON`]: #how-to-use-it
//! [`sockets`]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [`SQPOLL`]: https://man7.org/linux/man-pages/man2/io_uring_setup.2.html
//! [`Starting`]: ThreadState::Starting
//! [`state`]: ThreadLifecycleMonitor::lock()
//! [`stdin`]: std::io::stdin
//! [`Stop`]: crate::Continuation::Stop
//! [`Stopped`]: ThreadState::Stopped
//! [`Stopping`]: ThreadState::Stopping
//! [`stty sane`]: https://man7.org/linux/man-pages/man1/stty.1.html
//! [`SubscriberGuard`]: SubscriberGuard
//! [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`syscalls`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`TCP`]: https://en.wikipedia.org/wiki/Transmission_Control_Protocol
//! [`TerminationGuard`]: TerminationGuard
//! [`ThreadLifecycleMonitor`]: ThreadLifecycleMonitor
//! [`ThreadState::Stopped`]: ThreadState::Stopped
//! [`ThreadState`]: ThreadState
//! [`TIME_WAIT`]:
//!     https://en.wikipedia.org/wiki/Transmission_Control_Protocol#Connection_termination
//! [`tokio`]: tokio
//! [`try_subscribe()`]: RRT::try_subscribe
//! [`tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [`TUI`]: crate::tui::TerminalWindow::main_event_loop
//! [`UDP`]: https://en.wikipedia.org/wiki/User_Datagram_Protocol
//! [`W::Event`]: RRTWorker::Event
//! [`Wayland`]: https://wayland.freedesktop.org/
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [Actor]: https://en.wikipedia.org/wiki/Actor_model
//! [async runtime]: tokio::runtime
//! [async Rust]: https://rust-lang.github.io/async-book/
//! [async task]: tokio::task
//! [async-signal-safe]: https://man7.org/linux/man-pages/man7/signal-safety.7.html
//! [blocking call]: #understanding-blocking-io
//! [blocking I/O]: #understanding-blocking-io
//! [blocks on I/O]: #understanding-blocking-io
//! [busy-waiting]: https://en.wikipedia.org/wiki/Busy_waiting
//! [completions]: https://man7.org/linux/man-pages/man7/io_uring.7.html
//! [Console API]: https://learn.microsoft.com/en-us/windows/console/console-functions
//! [const expression]: RRT#const-expression-vs-const-declaration-vs-static-declaration
//! [const expressions]: RRT#const-expression-vs-const-declaration-vs-static-declaration
//! [cooked mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
//! [Darwin]: https://en.wikipedia.org/wiki/Darwin_(operating_system)
//! [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
//! [Dependency Injection]: https://en.wikipedia.org/wiki/Dependency_injection
//! [design pattern]: https://en.wikipedia.org/wiki/Software_design_pattern
//! [Drop implementation]: SubscriberGuard#method.drop
//! [event mechanism]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [Example]: #example
//! [executor thread in the `tokio` async runtime]: tokio::runtime
//! [executor threads in the async runtime]: tokio::runtime
//! [file descriptors]: https://man7.org/linux/man-pages/man2/open.2.html
//! [generation]: RRT::get_thread_generation
//! [in the meantime]: #the-exit-decision-race
//! [inversion of control]: https://en.wikipedia.org/wiki/Inversion_of_control
//! [kernel]: https://en.wikipedia.org/wiki/Kernel_(operating_system)
//! [known Darwin limitation]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [Linked operations]: https://man7.org/linux/man-pages/man3/io_uring_prep_link.3.html
//! [Linux]: https://man7.org/linux/man-pages/man3/pthread_cancel.3.html
//! [macOS]: https://man7.org/linux/man-pages/man3/pthread_cancel.3.html
//! [Multi-threaded runtime]: tokio::runtime::Builder::new_multi_thread
//! [poll loop]: run_worker_loop
//! [Proactor]: https://en.wikipedia.org/wiki/Proactor_pattern
//! [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
//! [Reactor]: https://en.wikipedia.org/wiki/Reactor_pattern
//! [readiness]: https://man7.org/linux/man-pages/man7/epoll.7.html#DESCRIPTION
//! [Registered buffers]:
//!     https://man7.org/linux/man-pages/man3/io_uring_register_buffers.3.html
//! [Registered FDs]: https://man7.org/linux/man-pages/man3/io_uring_register_files.3.html
//! [restart budget]: RestartPolicy
//! [RRT section]: crate#resilient-reactor-thread-rrt-pattern
//! [Rust discussion]: https://internals.rust-lang.org/t/thread-cancel-support/3056
//! [Rust workarounds]: https://matklad.github.io/2018/03/03/stopping-a-rust-worker.html
//! [scenario examples]: RestartPolicy#example-scenarios
//! [self-healing restarts]: #self-healing-restart-details
//! [signals]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [Single-threaded runtime]: tokio::runtime::Builder::new_current_thread
//! [singleton]: #how-to-use-it
//! [sockets]: https://man7.org/linux/man-pages/man7/socket.7.html
//! [Static vs. Flexible Usage]: RRT#static-vs-flexible-usage
//! [Syscall]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [Syscalls]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [terminal escape sequences]: crate::core::ansi
//! [Thread Lifecycle]: RRT#thread-lifecycle
//! [thread pool]: https://en.wikipedia.org/wiki/Thread_pool
//! [thread]: https://en.wikipedia.org/wiki/Thread_(computing)
//! [two-phase setup]: RRT#two-phase-setup
//! [Unix domain sockets]: https://en.wikipedia.org/wiki/Unix_domain_socket
//! [video]: https://www.youtube.com/watch?v=HvCptpU5r_4
//! [Web Workers]: https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API
//! [why user-provided?]: #why-is-rrtsoftwareinterrupt-user-provided
//! [Windows]:
//!     https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminatethread

#![rustfmt::skip]

// Attach.
#[cfg(any(test, doc))]
pub mod rrt;
#[cfg(not(any(test, doc)))]
mod rrt;

#[cfg(any(test, doc))]
pub mod rrt_engine;
#[cfg(not(any(test, doc)))]
mod rrt_engine;

#[cfg(any(test, doc))]
pub mod rrt_event;
#[cfg(not(any(test, doc)))]
mod rrt_event;

#[cfg(any(test, doc))]
pub mod rrt_monitor;
#[cfg(not(any(test, doc)))]
mod rrt_monitor;

#[cfg(any(test, doc))]
pub mod rrt_restart_policy;
#[cfg(not(any(test, doc)))]
mod rrt_restart_policy;

#[cfg(any(test, doc))]
pub mod rrt_subscriber_guard;
#[cfg(not(any(test, doc)))]
mod rrt_subscriber_guard;

#[cfg(any(test, doc))]
pub mod rrt_termination_guard;
#[cfg(not(any(test, doc)))]
mod rrt_termination_guard;

#[cfg(any(test, doc))]
pub mod rrt_thread_state;
#[cfg(not(any(test, doc)))]
mod rrt_thread_state;

#[cfg(any(test, doc))]
pub mod rrt_types;
#[cfg(not(any(test, doc)))]
mod rrt_types;

#[cfg(any(test, doc))]
pub mod rrt_interrupt_handle;
#[cfg(not(any(test, doc)))]
mod rrt_interrupt_handle;

#[cfg(any(test, doc))]
pub mod rrt_worker;
#[cfg(not(any(test, doc)))]
mod rrt_worker;

// Re-export.

// --- Public API (What you use) ---
pub use rrt::*;
pub use rrt_event::*;
pub use rrt_restart_policy::*;
pub use rrt_subscriber_guard::*;
pub use rrt_types::*;
pub use rrt_worker::*;

// --- Internal Implementation (How it works) ---
pub use rrt_engine::*;
pub use rrt_monitor::*;
pub use rrt_termination_guard::*;
pub use rrt_thread_state::*;
pub use rrt_interrupt_handle::*;

// Tests.
#[cfg(any(test, doc))]
pub mod rrt_integration_tests;
#[cfg(any(test, doc))]
pub mod process_isolated_tests;
#[cfg(any(test, doc))]
pub mod unit_tests;

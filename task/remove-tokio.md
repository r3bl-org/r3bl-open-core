# Task: Remove Tokio from `r3bl_tui`

<!-- BEGIN mktoc -->

- [Task: Remove Tokio from `r3bl_tui`](#task-remove-tokio-from-r3bl_tui)
- [Overview](#overview)
    - [Motivation](#motivation)
    - [Roadmap Alignment](#roadmap-alignment)
    - [Guiding Principles](#guiding-principles)
- [Core Async Adapter Algorithms](#core-async-adapter-algorithms)
    - [Runtime-Agnostic Primitives](#runtime-agnostic-primitives)
        - [Why `ScopedMutex`](#why-scopedmutex)
    - [Algorithm 1: One-Shot Thread Future](#algorithm-1-one-shot-thread-future)
        - [Session vs. Event Stream Boundary](#session-vs-event-stream-boundary)
        - [`ThreadJoinFuture<T>` Design](#threadjoinfuturet-design)
        - [Algorithm 1 Lifecycle](#algorithm-1-lifecycle)
    - [Algorithm 2: Broadcast Channel Future](#algorithm-2-broadcast-channel-future)
        - [`SubscriberEntry<T>` Design](#subscriberentryt-design)
        - [Double-Check Poll Algorithm](#double-check-poll-algorithm)
        - [Algorithm 2 Lifecycle](#algorithm-2-lifecycle)
    - [Sync vs. Async Callers](#sync-vs-async-callers)
- [Architecture & Design](#architecture--design)
    - [1. RRT Broadcast Channel](#1-rrt-broadcast-channel)
    - [2. `InputDevice` Unification](#2-inputdevice-unification)
    - [3. `TerminalWindow` Event Loop](#3-terminalwindow-event-loop)
    - [4. `StdThreadRunner`](#4-stdthreadrunner)
    - [5. Remaining `tui` Modules](#5-remaining-tui-modules)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: RRT Broadcast Channel](#phase-1-rrt-broadcast-channel)
    - [Phase 2: `InputDevice` & Backends](#phase-2-inputdevice--backends)
    - [Phase 3: `TerminalWindow` Event Loop](#phase-3-terminalwindow-event-loop)
    - [Phase 4: `StdThreadRunner`](#phase-4-stdthreadrunner)
    - [Phase 5: Remaining `tui` Modules](#phase-5-remaining-tui-modules)
    - [Phase 6: Downstream Crates](#phase-6-downstream-crates)
    - [Phase 7: Tests & Documentation](#phase-7-tests--documentation)
- [Success Criteria](#success-criteria)
- [Future-Proofing: Multi-Socket Servers](#future-proofing-multi-socket-servers)

<!-- END mktoc -->

---

## Overview

### Motivation

This task transitions `r3bl_tui` and its downstream workspace crates from an async/Tokio
execution model to a pure synchronous architecture with runtime-agnostic
`std::future::Future` adapters.

The core of `r3bl_tui` (the rendering pipeline, diff engine, layout engine, ANSI parser,
and component/app traits) is **already 100% synchronous**. Async Rust in `r3bl_tui` was
introduced historically for terminal input polling, before our Linux-native
[`direct_to_ansi`] backend and [`ResilientReactorThread` (RRT)] were built.

Today, having Tokio in `tui` introduces substantial incidental complexity:

1. **Async "function coloring"**: Callers must be `async`, requiring `#[tokio::main]` in
   binaries and `block_on` in test harnesses.
2. **Pinned Boxed Futures (`>16 KiB`)**: Functions like `main_event_loop_impl` and
   `choose` require `Pin<Box<dyn Future>>` to avoid stack blowouts and clippy's
   `large_futures` warning.
3. **Nested Runtime Hazards**: Caused the creation of [`std_thread_tokio_runner.rs`] (400
   lines of complex runtime-isolation machinery) just to avoid
   `"Cannot start a runtime from within a runtime"` panics when sync code calls async
   helpers.
4. **Binary & Compilation Overhead**: Pulling Tokio with `features = ["full", "tracing"]`
   adds significant compilation latency and binary bloat to every consumer of `r3bl_tui`.

### Roadmap Alignment

This task represents **Step 3** of the **Unify Rendering Roadmap** in
[`prepare-v0.8.0-meta-task.md:117-128`](prepare-v0.8.0-meta-task.md#L117-L128):

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. task_unify_cli_and_styled_text.md                            │
│    (Extends TuiStyledText with builder methods for RenderOps)   │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│ 2. task_refactor_interactive_apis_to_alternate_screen.md        │
│    (Consolidates choose & readline onto TerminalWindow/App/     │
│     Component primitives; deletes legacy cursor engines)        │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│ 3. remove-tokio.md (THIS TASK)                                  │
│    (Switches TerminalWindow & InputDevice to sync, dropping     │
│     Tokio from tui in one clean, uninterrupted pass!)           │
└─────────────────────────────────────────────────────────────────┘
```

**Why This 3-Step Sequence Is Optimal:**

- In Step 2, `choose` and `readline` are consolidated to run as standard `Component` /
  `App` primitives through [`TerminalWindow::main_event_loop`].
- The bespoke cursor-positioning engines in `choose_impl` and `readline_async_impl` are
  deleted.
- **Therefore, 100% of all UI in `r3bl_tui` now runs through
  `TerminalWindow::main_event_loop` and `InputDevice`.**
- In this task (Step 3), making `TerminalWindow::main_event_loop` and `InputDevice`
  synchronous **automatically makes all UI in the crate (including `choose`, `readline`,
  `edi`, and full apps) synchronous with zero duplicate effort!**

### Guiding Principles

1. **Clean Architectural Sweep**: With all UI unified under `TerminalWindow`,
   synchronizing the event pump synchronizes everything.
2. **Zero Compromise on Responsiveness**: The UI remains fully responsive; blocking I/O
   (such as `read(2)` or `epoll_wait(2)`) remains isolated to dedicated background threads
   (via RRT).
3. **100% Pure `std` Primitives (Zero External Async/Channel Crates)**: Replace `tokio`
   entirely using standard library primitives (`std::sync::mpsc`, `std::thread`,
   `std::future::Future`, and `std::task::Waker`). Do **not** add `crossbeam-channel` or
   any optional `tokio` feature flag. (Since Rust 1.67+, `std::sync::mpsc` is already
   powered by `crossbeam-channel` under the hood in `libstd`, and since Rust 1.72+,
   `std::sync::mpsc::Sender<T>` is `Clone + Send + Sync`.)
4. **Runtime-Agnostic Async Compatibility (`std::future::Future` Adapters)**: Downstream
   customers using _any_ async runtime (`tokio`, `smol`, `async-std`, `futures`) can still
   `.await` TUI entry points (`TerminalWindow`, `choose`, `readline`), `.await`
   `InputDevice` events, and send [`TerminalWindowMainThreadSignal`]s from spawned async
   tasks, all via pure-`std` `Future` and `Waker` adapters with zero runtime lock-in.
5. **Incremental Compilation**: Each phase can be compiled and passes local checks
   (`./check.fish --check`).

---

## Core Async Adapter Algorithms

This section documents the exact algorithms and lifecycle state machines used to keep
`r3bl_tui` 100% synchronous internally while providing runtime-agnostic pure-`std`
`Future` and `Waker` adapters backed by `ScopedMutex` that work in any executor (`tokio`,
`smol`, `async-std`, `futures`) with zero external dependencies.

### Runtime-Agnostic Primitives

The core mechanism combines `std::future::Future`, `std::task::Waker`, and `ScopedMutex`.
An async runtime (`tokio`, `smol`, etc.) does not require futures to be created by that
runtime. Every `.await` in Rust compiles down to calling `std::future::Future::poll`:

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output>;
}
```

Inside `cx` (`std::task::Context`) is a **`std::task::Waker`** (`cx.waker()`), which is a
thread-safe (`Clone + Send + Sync + 'static`) callback handle provided by whatever runtime
the caller is using:

- When our `Future::poll()` returns **`Poll::Pending`**, the caller's async runtime
  suspends that task and **frees the async worker thread** to do other work.
- Later, when our synchronous OS thread finishes its work (or pushes a message into a
  `std::sync::mpsc` channel), that OS thread calls **`waker.wake()`**.
- Calling `waker.wake()` from _any_ OS thread tells the caller's async runtime: _"This
  future is ready: put the task back on your run queue and call `poll()` again!"_

#### Why `ScopedMutex`

We use [`ScopedMutex`] (and never raw `std::sync::Mutex`) to guard shared `Future`/`Waker`
state for two reasons:

1. **Structural Prevention of "Wake-Under-Lock" Deadlocks**: When waking a
   `std::task::Waker`, you must **never** call `waker.wake()` while still holding the
   `MutexGuard` that `poll()` locks, because single-threaded or inline executors may
   immediately re-poll the `Future` inside `waker.wake()` on the same thread, which would
   deadlock if the lock is still held. With [`ScopedMutex`], the `.write(|state| ...)`
   closure returns `Option<Waker>`, **guaranteeing the lock is released at the end of the
   closure** before `waker.wake()` is called:

    ```rust
    let maybe_waker = thread_state.write(|state| {
        state.outcome = Some(outcome);
        state.waker.take()
    }); // <-- Lock is guaranteed to be released right here!

    if let Some(waker) = maybe_waker {
        waker.wake(); // <-- Always called outside the lock!
    }
    ```

2. **Fits Our Concurrency Safety Taxonomy**: Per
   `.agents/skills/concurrency-safety/patterns.md`, `Monitor<S>` (Chain of Custody) is
   reserved for state machines that require a `Condvar`. Neither Algorithm 1 nor Algorithm
   2 uses a `Condvar` (both are short-lived state updates), making
   `ScopedMutex<S, { DeadlockPreventionPolicy::PanicOnSpecificLockNesting }>`
   (`scoped_mutex!(SPEC, ...)`) the exact fit.

---

### Algorithm 1: One-Shot Thread Future

Algorithm 1 (`StdThreadRunner::run_async` + `ThreadJoinFuture<T>`) bridges a dedicated OS
thread or full interactive TUI session completion into a `std::future::Future` for
awaiting `TerminalWindow::main_event_loop_async`, `choose_async`, `readline_async`, or
`Spinner::await_shutdown_async`.

#### Session vs. Event Stream Boundary

Here is how the inside and outside views of `TerminalWindow::main_event_loop` relate:

- **Inside `run_main_event_loop` (Continuous Stream):** The loop continuously reads
  `InputEvent`s from `InputDevice` and `TerminalWindowMainThreadSignal`s from
  `GlobalData::main_thread_channel_sender`, invoking `App::app_handle_input_event` and
  `App::app_handle_signal` on every frame.
- **Outside at the Call Site (e.g., `cmdr/src/edi/launcher.rs`):** From the caller's
  perspective, `main_event_loop` runs the entire interactive TUI session and returns
  **once** when the user exits the app (`Ctrl+Q` / `ExitMainEventLoop`), yielding
  `CommonResult<(GlobalData<S, AS>, InputDevice, OutputDevice)>`.
- Meanwhile, while the TUI is running, any async task the customer spawned via
  `tokio::spawn` communicates with the running TUI by sending signals through
  `global_data.main_thread_channel_sender.send(TerminalWindowMainThreadSignal::ApplyAppSignal(...))`
  (which is a non-blocking `std::sync::mpsc::Sender`).

#### `ThreadJoinFuture<T>` Design

`ThreadJoinFuture<T>` is a custom pure-`std` type we build in `r3bl_tui`. Because
`std::thread::JoinHandle<T>` in the standard library only provides a blocking `.join()`
(`pthread_join`) and does **not** implement `std::future::Future`, we define
`ThreadJoinFuture<T>` in `tui/src/core/common/std_thread_runner.rs`:

```rust
pub struct ThreadJoinFuture<T> {
    shared_state: Arc<
        ScopedMutex<
            ThreadFutureState<T>,
            { DeadlockPreventionPolicy::PanicOnSpecificLockNesting },
        >,
    >,
}

struct ThreadFutureState<T> {
    outcome: Option<ThreadRunnerOutcome<T>>,
    waker: Option<std::task::Waker>,
}

impl<T: Send + 'static> StdThreadRunner {
    pub fn run_async<F>(work_fn: F) -> ThreadJoinFuture<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let shared_state = Arc::new(scoped_mutex!(
            SPEC,
            ThreadFutureState {
                outcome: None,
                waker: None,
            }
        ));
        let thread_state = Arc::clone(&shared_state);

        let spawn_res = std::thread::Builder::new()
            .name("r3bl-async-bridge".into())
            .spawn(move || {
                let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work_fn)) {
                    Ok(val) => ThreadRunnerOutcome::Completed(val),
                    Err(panic_payload) => ThreadRunnerOutcome::Panicked(panic_payload),
                };
                let maybe_waker = thread_state.write(|state| {
                    state.outcome = Some(outcome);
                    state.waker.take()
                }); // Lock released before waking!
                if let Some(waker) = maybe_waker {
                    waker.wake();
                }
            });

        if let Err(io_err) = spawn_res {
            shared_state.write(|state| {
                state.outcome = Some(ThreadRunnerOutcome::FailedToStart(io_err.into()));
            });
        }

        ThreadJoinFuture { shared_state }
    }
}
```

#### Algorithm 1 Lifecycle

The diagram below traces the complete lifecycle sequence for Algorithm 1 from two
perspectives:

- **Left Side (Caller / Async Executor Thread):** An async customer inside
  `#[tokio::main]` calling
  `TerminalWindow::main_event_loop_async(app, exit_keys, state).await`.
- **Right Side (Dedicated TUI OS Thread):** The `std::thread` spawned by
  `StdThreadRunner::run_async` that runs the synchronous
  `TerminalWindow::main_event_loop`.

```text
ASYNC CALLER THREAD (e.g., Tokio Worker)          DEDICATED TUI OS THREAD (std::thread)
(Customer's `#[tokio::main]` async fn)            (Spawned by `StdThreadRunner::run_async`)
──────────────────────────────────────            ─────────────────────────────────────────
[PHASE 1: Startup & Spawning the OS Thread]
1. Caller calls:
   `let fut = TerminalWindow::main_event_loop_async(app, exit_keys, state);`
   ├─ Allocates shared mailbox:
   │  `shared_state = Arc::new(scoped_mutex!(SPEC, {`
   │  `    outcome: None,`
   │  `    waker:   None,`
   │  `}))`
   ├─ Spawns OS thread (`std::thread::spawn`) ──► 2. Dedicated OS Thread starts!
   │  moving `app`, `state`, and                     ├─ Enters raw mode & alternate screen
   │  `Arc::clone(&shared_state)` into it.           ├─ Calls `app.app_init_components()`
   └─ Returns `ThreadJoinFuture { shared_state }`    ├─ Calls `app.app_start_background_services(&mut global_data)`:
      immediately to the Caller.                     │  (If the customer wants background async tasks,
                                                     │   they can clone `global_data.main_thread_channel_sender`
                                                     │   (`std::sync::mpsc::Sender`) and `tokio::spawn` tasks!)
                                                     ├─ Renders first frame to terminal
                                                     └─ Enters synchronous `run_main_event_loop`:
                                                        Blocks on `main_thread_channel_receiver.recv()`
                                                        (waiting for keystrokes OR app signals at 0% CPU)

[PHASE 2: Caller `.await`s `ThreadJoinFuture`]
3. Caller executes `fut.await`:
   ├─ Tokio calls `ThreadJoinFuture::poll(cx)`
   ├─ `shared_state.write(|state| ...)`:
   │  • Checks `state.outcome`: still `None`
   │  • Registers doorbell: `state.waker = Some(cx.waker().clone())`
   │  • Returns `Poll::Pending` (ScopedMutex unlocked!)
   └─ Tokio puts this task to sleep.
      *** The Tokio worker thread is now 100% FREE ***
      to run the customer's background HTTP/timer tasks!

[PHASE 3: While the TUI Is Running (Minutes / Hours)]
4. Customer's other Tokio tasks run freely on
   Tokio worker threads!
   ├─ Say a `tokio::spawn` task finishes an HTTP fetch:
   └─ It calls `main_thread_channel_sender.send(`
      `   ApplyAppSignal(MySignal::DataLoaded(data))`
      `)` (non-blocking `std::sync::mpsc::Sender`!) ──► 5. Dedicated OS Thread wakes from `mpsc::recv()`!
                                                           ├─ Dispatches `app.app_handle_signal()`
                                                           ├─ Or dispatches `InputEvent` from `InputDevice`
                                                           ├─ Re-renders diff to terminal
                                                           └─ Loops back to `mpsc::recv()`
                                                           (Note: `ThreadJoinFuture` is NOT woken here;
                                                            the TUI session is still running!)

[PHASE 4: User Exits TUI (`Ctrl+Q` / `ExitMainEventLoop`)]
                                                  6. User presses `Ctrl+Q` (or App returns `ExitMainEventLoop`)!
                                                     ├─ `run_main_event_loop` breaks out of `while` loop
                                                     ├─ Drops RAII guards (restores terminal from
                                                     │  alternate screen & raw mode back to cooked mode)
                                                     ├─ `let maybe_waker = shared_state.write(|state| {`
                                                     │  `    state.outcome = Some(Completed(res));`
                                                     │  `    state.waker.take()`
                                                     │  `});` (ScopedMutex unlocked!)
                                                     ├─ Calls `waker.wake()`! ─────────────┐
                                                     └─ Dedicated OS Thread exits cleanly! │
                                                                                           │
[PHASE 5: Tokio Wakes Up & Resolves `.await`]                                              │
7. Tokio receives `waker.wake()` notification ◄────────────────────────────────────────────┘
   ├─ Schedules Caller's task & calls `ThreadJoinFuture::poll(cx)`
   ├─ `shared_state.write(|state| state.outcome.take())` -> `Some(Completed(res))`
   ├─ Returns `Poll::Ready(Completed(res))`!
   └─ Caller's `fut.await` finishes and receives
      `(GlobalData<S, AS>, InputDevice, OutputDevice)`!
```

---

### Algorithm 2: Broadcast Channel Future

Algorithm 2 (`SyncBroadcastSender` / `SyncBroadcastReceiver` + `SubscriberEntry<T>`)
bridges a continuous broadcast event stream into a `std::future::Future`. It is used by
`ResilientReactorThread` (`RRT`) and `InputDevice::next_async()` so async consumers can
`.await` continuous terminal events inside `tokio::select!` without spawning a thread per
event.

#### `SubscriberEntry<T>` Design

`SubscriberEntry<T>` enables 1-to-many broadcast over `std::sync::mpsc`. In `RRT`, **1
Producer Thread** (`rrt-worker-gen-1`) broadcasts every `RRTEvent` to **many Subscribers**
(`Subscriber #1`, `Subscriber #2`, ...). However, `std::sync::mpsc::channel()` is
**Single-Consumer** (`Receiver<T>` cannot be cloned). To build a **1-to-Many Broadcast
Channel** out of `std::sync::mpsc`:

- Every time a Subscriber calls `subscribe()`, we create a **dedicated
  `std::sync::mpsc::channel()` pair** just for that Subscriber.
- The **Subscriber** keeps the `Receiver<T>` (and a shared `ScopedMutex<Option<Waker>>`).
- The **Producer** stores that Subscriber's `Sender<T>` and
  `Arc<ScopedMutex<Option<Waker>>>` as a **`SubscriberEntry<T>`** inside its `Vec` of
  active subscribers:

```rust
/// Held inside the Producer's `Vec<SubscriberEntry<T>>`, one entry per active subscriber.
pub struct SubscriberEntry<T> {
    /// Pipe to push cloned events into this specific subscriber's `mpsc::Receiver<T>`.
    sender: std::sync::mpsc::Sender<T>,
    /// Shared doorbell in case this subscriber is parked in `.recv_async().await`.
    waker: Arc<
        ScopedMutex<
            Option<std::task::Waker>,
            { DeadlockPreventionPolicy::PanicOnSpecificLockNesting },
        >,
    >,
}
```

#### Double-Check Poll Algorithm

The double-check `try_recv()` poll algorithm guarantees race-free and cancellation-safe
async event reception:

1. **When the Producer (RRT thread) broadcasts an event:**

    ```rust
    let mut wakers_to_wake = SmallVec::<[std::task::Waker; 4]>::new();
    self.subscribers.write(|entries| {
        entries.retain(|entry| {
            if entry.sender.send(event.clone()).is_ok() {
                if let Some(waker) = entry.waker.write(|w| w.take()) {
                    wakers_to_wake.push(waker);
                }
                true
            } else {
                false // Subscriber dropped its Receiver; prune disconnected entry!
            }
        });
    }); // Both ScopedMutexes are unlocked here!

    for waker in wakers_to_wake {
        waker.wake();
    }
    ```

2. **When an Async Subscriber calls `receiver.recv_async().await` (`poll(cx)`):** Uses the
   **double-check `try_recv()`** pattern so an event can never be missed between checking
   the `mpsc` queue and registering the `Waker`:

    ```rust
    // Step A: Fast path: is an event already queued in mpsc?
    match self.receiver.try_recv() {
        Ok(event) => return Poll::Ready(Ok(event)),
        Err(TryRecvError::Disconnected) => return Poll::Ready(Err(RecvError)),
        Err(TryRecvError::Empty) => {} // Queue is empty; proceed to register waker
    }

    // Step B: Register our Waker via ScopedMutex so the producer thread can wake us:
    self.waker.write(|w| *w = Some(cx.waker().clone()));

    // Step C: Double-check! In case the producer pushed an event between Step A and Step B:
    match self.receiver.try_recv() {
        Ok(event) => {
            self.waker.write(|w| w.take());
            Poll::Ready(Ok(event))
        }
        Err(TryRecvError::Disconnected) => Poll::Ready(Err(RecvError)),
        Err(TryRecvError::Empty) => Poll::Pending, // Safely parked!
    }
    ```

3. **Why This Is 100% Cancellation-Safe in `tokio::select!`:** Events remain buffered
   inside `std::sync::mpsc::Receiver<T>` until `try_recv()` pops one and returns
   `Poll::Ready`. If another branch of `tokio::select!` wins and drops the `recv_async()`
   future while it is `Poll::Pending`, **zero events are lost**: the event stays in
   `self.receiver` for the next iteration of the `select!` loop.

#### Algorithm 2 Lifecycle

The diagram below traces the complete lifecycle sequence for Algorithm 2 from two
perspectives:

- **Left Side (Subscriber / Caller Thread):** e.g., `DirectToAnsiInputDevice::new()`
  calling `RRT::try_subscribe()`, and then consuming events via sync `.recv()` or async
  `.recv_async().await`.
- **Right Side (Producer / RRT Worker Thread):** `rrt-worker-gen-1`, running
  `run_worker_loop()` and blocking on `mio::Poll::poll()`.

```text
SUBSCRIBER / CALLER THREAD                        PRODUCER (RRT WORKER THREAD)
(e.g., App / DirectToAnsiInputDevice)             (rrt-worker-gen-1)
─────────────────────────────────────             ────────────────────────────
[PHASE 1: Cold Start & Registration]
1. Caller calls `RRT::try_subscribe()`
   ├─ Locks `ThreadLifecycleMonitor`
   ├─ Sees state is `Stopped` (SlowPath)
   ├─ Calls `W::create_and_register_os_sources()`
   │  (creates `mio::Poll` + `MioSoftwareInterrupt`)
   ├─ Calls `broadcast_sender.subscribe()`:
   │  • Creates `std::sync::mpsc::channel()`
   │  • Creates `waker = Arc::new(scoped_mutex!(SPEC, None))`
   │  • Pushes `SubscriberEntry { tx, waker }`
   │    into Producer's `subscribers: ScopedMutex<Vec>`
   │  • Increments `receiver_count` (0 -> 1)
   ├─ Spawns `rrt-worker-gen-1` OS thread ──────► 2. RRT Worker Thread starts
   └─ Returns `SubscriberGuard { rx, waker }`        └─ Enters `run_worker_loop()`
                                                     └─ Blocks in `worker.block_until_ready_then_dispatch()`
                                                        (e.g., `mio::Poll::poll()` at 0% CPU)

[PHASE 2: Consuming Events (Sync OR Async)]
3A. If Caller is SYNC (`rx.recv()`):
   └─ Calls `self.receiver.recv()`
      (OS thread sleeps inside `mpsc::recv()`)

3B. If Caller is ASYNC (`rx.recv_async().await`):
   ├─ Executor calls `poll(cx)`
   ├─ `self.receiver.try_recv()` -> `Empty`
   ├─ `self.waker.write(|w| *w = Some(cx.waker().clone()))`
   ├─ Double-checks `self.receiver.try_recv()` -> `Empty`
   └─ Returns `Poll::Pending` (Executor thread freed!)

                                                  4. User presses a key in terminal!
                                                     ├─ `mio::Poll::poll()` wakes up
                                                     ├─ Reads stdin bytes & parses `InputEvent`
                                                     ├─ Calls `broadcast_sender.send(event)`:
                                                     │  Iterates `subscribers` via `ScopedMutex::write`:
                                                     │  • Calls `entry.sender.send(event.clone())`
                                                     │    (If `Err`, subscriber dropped -> remove entry!)
                                                     │  • Checks `entry.waker.write(|w| w.take())`:
                                                     │    - For 3A (Sync): `waker` is `None`
                                                     │      (`mpsc::Sender::send` already woke 3A's `recv()`!)
                                                     │    - For 3B (Async): `waker` is `Some(w)` ->
                                                     │      calls `w.wake()` outside lock! ┐
                                                     └─ Loops back to `mio::Poll`          │
                                                                                           │
5A. Sync Caller (3A) wakes from `mpsc::recv()`                                             │
    with `Ok(event)`!                                                                      │
                                                                                           │
5B. Async Caller (3B) is re-polled by Tokio/smol ◄─────────────────────────────────────────┘
    ├─ Calls `self.receiver.try_recv()`
    └─ Pops `Ok(event)` -> returns `Poll::Ready(Ok(event))`!

[PHASE 3: Teardown When Subscriber Drops]
6. Caller drops `SubscriberGuard`:
   ├─ Field 1 drops (`SyncBroadcastReceiver`):
   │  • Drops `mpsc::Receiver<T>`
   │  • Atomically decrements `receiver_count` (1 -> 0)
   └─ Field 2 drops (`InterruptOnDrop`):
      • Calls `interrupt.trigger_software_interrupt()`
        (`mio::Waker::wake()`) ─────────────────► 7. `mio::Poll::poll()` wakes up immediately!
                                                     ├─ Checks `sender.receiver_count() == 0`
                                                     ├─ Transitions state `Running -> Stopping -> Stopped`
                                                     └─ RRT Worker Thread exits cleanly!
```

---

### Sync vs. Async Callers

The table below summarizes how synchronous and asynchronous callers interact with the same
synchronous core:

| Operation                           | Sync Caller (`fn main`)                          | Async Caller (`#[tokio::main]`, `smol`, etc.)      | Underlying Mechanism                                     |
| :---------------------------------- | :----------------------------------------------- | :------------------------------------------------- | :------------------------------------------------------- |
| **Run TUI / `choose` / `readline`** | `TerminalWindow::main_event_loop(...)`           | `TerminalWindow::main_event_loop_async(...).await` | **Algorithm 1** (`ThreadJoinFuture` + `ScopedMutex`)     |
| **Read next `InputDevice` event**   | `input_device.next()` (`mpsc::Receiver::recv()`) | `input_device.next_async().await`                  | **Algorithm 2** (`try_recv()` + `Waker` + `ScopedMutex`) |
| **Send app signal to TUI**          | `sender.send(signal)`                            | `sender.send(signal)` _(non-blocking!)_            | `std::sync::mpsc::Sender::send`                          |

---

## Architecture & Design

### 1. RRT Broadcast Channel

This section specifies the `ResilientReactorThread` (RRT) synchronization layer using
`std::sync::mpsc`, `std::task::Waker`, and `ScopedMutex`.

**Target Files:**

- `tui/src/core/resilient_reactor_thread/rrt.rs`
- `tui/src/core/resilient_reactor_thread/rrt_engine.rs`
- `tui/src/core/resilient_reactor_thread/rrt_types.rs`
- `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs`

Replace `tokio::sync::broadcast` with a 100% pure-`std` multi-subscriber broadcast channel
built on `std::sync::mpsc` and [`ScopedMutex`] with optional `std::task::Waker` support
for async consumers:

```rust
pub struct SubscriberEntry<T> {
    sender: std::sync::mpsc::Sender<T>,
    waker: Arc<ScopedMutex<Option<std::task::Waker>, { DeadlockPreventionPolicy::PanicOnSpecificLockNesting }>>,
}

pub struct SyncBroadcastSender<T: Clone> {
    subscribers: Arc<ScopedMutex<Vec<SubscriberEntry<T>>, { DeadlockPreventionPolicy::PanicOnSpecificLockNesting }>>,
    receiver_count: Arc<AtomicUsize>,
}

pub struct SyncBroadcastReceiver<T: Clone> {
    receiver: std::sync::mpsc::Receiver<T>,
    waker: Arc<ScopedMutex<Option<std::task::Waker>, { DeadlockPreventionPolicy::PanicOnSpecificLockNesting }>>,
    receiver_count: Arc<AtomicUsize>,
}
```

- **Synchronous Reception (`recv`)**: `SubscriberGuard` holds
  `SyncBroadcastReceiver<RRTEvent<W::Output>>` and implements blocking
  `recv(&self) -> Result<RRTEvent<W::Output>, std::sync::mpsc::RecvError>`.
- **Runtime-Agnostic Async Reception (`recv_async`)**: Also exposes
  `recv_async(&mut self) -> impl std::future::Future<Output = Result<RRTEvent<W::Output>, std::sync::mpsc::RecvError>>`
  that checks `self.receiver.try_recv()` and registers `cx.waker().clone()` in
  `self.waker` when `Pending`. When `SyncBroadcastSender::send` broadcasts an event, it
  sends on each subscriber's `std::sync::mpsc::Sender` (pruning disconnected slots) and
  wakes any registered `Waker`.
- **Drop Order Preservation (RFC 1857)**: Dropping `SyncBroadcastReceiver` atomically
  decrements `receiver_count` before `SubscriberGuard::interrupt_on_drop` fires.

### 2. `InputDevice` Unification

This section unifies `InputDevice` and its backends (`direct_to_ansi`, `crossterm`, and
`MockInputDevice`) around a synchronous `next()` method and a pure-`std` `next_async()`
adapter.

**Target File:** `tui/src/core/terminal_io/input_device.rs`

Change `InputDevice` primary API from `async fn next` to synchronous `fn next`, plus a
pure-`std` `next_async` adapter for customers writing custom async `select!` loops:

```rust
impl InputDevice {
    /// Blocking synchronous read of the next terminal input event.
    pub fn next(&mut self) -> Option<InputEvent>;

    /// Runtime-agnostic `std::future::Future` adapter for async callers (`tokio`, `smol`, etc.).
    pub fn next_async(&mut self) -> impl std::future::Future<Output = Option<InputEvent>> + '_;
}
```

- **`DirectToAnsiInputDevice`**:
    - `next(&mut self)` calls `self.subscriber_guard.receiver.recv()`.
    - `next_async(&mut self)` awaits `self.subscriber_guard.receiver.recv_async()`.
- **`CrosstermInputDevice`**:
    - `next(&mut self)` calls `crossterm::event::read()` (with proper error mapping to
      `InputEvent`), dropping `crossterm::event::EventStream` and `futures_util`.
- **`MockInputDevice`**: Pops from internal buffer synchronously (and sleeps via
  `std::thread::sleep` if a synthetic delay is configured).

### 3. `TerminalWindow` Event Loop

This section converts the full TUI main event loop (`TerminalWindow::main_event_loop`) to
a synchronous loop driven by `std::sync::mpsc`.

**Target Files:**

- `tui/src/tui/terminal_window/main_event_loop.rs`
- `tui/src/tui/terminal_window/terminal_window_api.rs`
- `tui/src/tui/terminal_window/shared_global_data.rs`

Replace `tokio::select!` and `tokio::sync::mpsc` with a **single unified
`std::sync::mpsc::channel`** (no `crossbeam-channel` needed):

1. **Unified Event Channel (`std::sync::mpsc`)**:
    - `GlobalData::main_thread_channel_sender` becomes
      `std::sync::mpsc::Sender<TerminalWindowMainThreadSignal<AS>>`.
    - Add `TerminalWindowMainThreadSignal::Input(InputEvent)` (or an internal
      `EventLoopMessage<AS>` enum with `Signal(TerminalWindowMainThreadSignal<AS>)` and
      `Input(InputEvent)`) fed into the same `std::sync::mpsc` queue.
    - Because `std::sync::mpsc::Sender::send` is non-blocking and `Sender` is
      `Clone + Send + Sync`, **both** standard OS threads (`std::thread::spawn`) **and**
      downstream async tasks (`tokio::spawn`, `smol::spawn`) inside
      `App::app_start_background_services` can call
      `main_thread_channel_sender.send(TerminalWindowMainThreadSignal::ApplyAppSignal(...))`
      directly without `.await`!
    - `request_exit_by_sending_signal` simplifies to a synchronous 1-liner:
      `channel_sender.send(TerminalWindowMainThreadSignal::Exit).ok();` (deleting
      `tokio::spawn`).

2. **Synchronous `run_main_event_loop`**:

```rust
pub fn run_main_event_loop<S, AS>(
    mut event_loop_state: EventLoopState<S, AS>,
    mut app: BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    mut input_device: InputDevice,
    output_device: OutputDevice,
) -> CommonResult<(GlobalData<S, AS>, InputDevice, OutputDevice)> {
    while let Ok(signal_or_input) = event_loop_state.main_thread_channel_receiver.recv() {
        match signal_or_input {
            TerminalWindowMainThreadSignal::Input(input_event) => {
                handle_input_event(
                    input_event,
                    &mut event_loop_state,
                    &mut app,
                    exit_keys,
                    &output_device,
                );
            }
            signal => {
                if handle_main_thread_signal(
                    signal,
                    &mut event_loop_state,
                    &mut app,
                    exit_keys,
                    &output_device,
                )? == Continuation::Stop
                {
                    break;
                }
            }
        }
        event_loop_state.log_telemetry_info();
    }
    event_loop_state.log_shutdown_info();
    Ok((event_loop_state.global_data, input_device, output_device))
}
```

- `TerminalWindow::main_event_loop` runs synchronously and returns
  `TuiAvailability<CommonResult<(GlobalData<S, AS>, InputDevice, OutputDevice)>>`
  directly, deleting `MainEventLoopFuture` and `Pin<Box<dyn Future>>`.
- `TerminalWindow::main_event_loop_async` (and `choose_async` / `readline_async`) wraps
  the synchronous call using `StdThreadRunner::run_async` (below) so async customers can
  `.await` it on any executor without blocking their async worker threads.

### 4. `StdThreadRunner`

This section transforms `std_thread_tokio_runner` into a pure-`std` `StdThreadRunner` with
a `ThreadJoinFuture<T>` async adapter.

**Target File:** `tui/src/core/common/std_thread_tokio_runner.rs` (rename to
`std_thread_runner.rs`)

Transform `StdThreadTokioRunner` into `StdThreadRunner` with **three** execution modes
using 100% `std`:

1. **`StdThreadRunner::run_blocking(work_fn: F) -> ThreadRunnerOutcome<T>`**
    - Spawns a named scoped OS thread (`std::thread::scope`) and joins immediately.
2. **`StdThreadRunner::run_non_blocking(work_fn: F) -> std::io::Result<StdThreadJoinHandle<T>>`**
    - Spawns a named unscoped OS thread (`std::thread::Builder::spawn`) and returns a
      handle that can be `.join()`ed synchronously later.
3. **`StdThreadRunner::run_async(work_fn: F) -> ThreadJoinFuture<T>` (Pure-`std` Async
   Adapter)**
    - Spawns a named unscoped OS thread that executes synchronous `work_fn()`, stores the
      resulting `ThreadRunnerOutcome<T>` into a [`ScopedMutex`]-guarded shared state, and
      wakes a `std::task::Waker` **after** the closure releases the lock:

```rust
pub struct ThreadJoinFuture<T> {
    shared_state: Arc<ScopedMutex<ThreadFutureState<T>, { DeadlockPreventionPolicy::PanicOnSpecificLockNesting }>>,
}

struct ThreadFutureState<T> {
    outcome: Option<ThreadRunnerOutcome<T>>,
    waker: Option<std::task::Waker>,
}

impl<T> std::future::Future for ThreadJoinFuture<T> {
    type Output = ThreadRunnerOutcome<T>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.shared_state.write(|state| {
            if let Some(outcome) = state.outcome.take() {
                std::task::Poll::Ready(outcome)
            } else {
                state.waker = Some(cx.waker().clone());
                std::task::Poll::Pending
            }
        })
    }
}
```

- Also implement `IntoFuture` on `StdThreadJoinHandle<T>` (or provide `.into_future()`) so
  callers can either `.join()` synchronously **or** `.await` it in any async runtime
  (`tokio`, `smol`, `async-std`, etc.)!

### 5. Remaining `tui` Modules

This section covers deleting `AsyncPtySession` and synchronizing `Spinner`,
`core::script`, and `network_io`.

1. **Delete `AsyncPtySession` (`tui/src/core/pty/pty_session/`)**:
    - [`PtySession`](tui/src/core/pty/pty_session/session.rs) is already 100% synchronous
      (`std::sync::mpsc::SyncSender` / `Receiver` + `std::thread::JoinHandle`), and
      [`PTYMux`](tui/src/core/pty/pty_mux/process_manager.rs) already uses `PtySession`
      exclusively.
    - The only caller of `PtySessionBuilder::start_async()` in the workspace was
      `cmdr/src/self_upgrade/upgrade/build.rs`, which switches to synchronous
      `PtySessionBuilder::start()`.
    - Delete `AsyncPtySession` from `session.rs`, `PtySessionBuilder::start_async()` from
      `builder.rs`, and `AsyncPtyOrchestratorHandle` / `AsyncOutputEventReceiverHalf` /
      `AsyncInputEventSenderHalf` from `type_aliases.rs`. (External async callers can use
      `StdThreadRunner::run_async` if needed.)
2. **`Spinner` (`tui/src/readline_async/spinner.rs`)**:
    - Replace `tokio::time::interval` and `tokio::sync::broadcast` with a dedicated
      `std::thread` loop that waits on a `std::sync::mpsc::Receiver<()>` via
      `rx.recv_timeout(tick_interval)`:
        - On `RecvTimeoutError::Timeout`: renders the next spinner tick frame.
        - On `Ok(())` or `RecvTimeoutError::Disconnected`: prints the final message,
          restores terminal state, and exits cleanly.
    - `Spinner::await_shutdown()` joins the spinner thread synchronously (and provides
      `await_shutdown_async()` returning `ThreadJoinFuture<()>` for async consumers).
3. **`core::script` (`tui/src/core/script/`)**:
    - Convert `command_impl.rs`, `git.rs`, `env_source.rs`, `download.rs`,
      `crates_api.rs`, `github_api.rs`, and `http_client.rs` from
      `tokio::process::Command` / `tokio::fs` / async `reqwest` to
      `std::process::Command`, `std::fs`, and a pure-sync HTTP client (e.g., `ureq` with
      `rustls` and `json`, removing transitive `tokio` from `reqwest`).
4. **`network_io` (`tui/src/network_io/`)**:
    - Convert `length_prefix_protocol.rs` and `compress_block.rs` from
      `tokio::io::{AsyncRead, AsyncWrite}` to `std::io::{Read, Write}`.

---

## Implementation Plan

### Phase 1: RRT Broadcast Channel

Replace `tokio::sync::broadcast` in the `ResilientReactorThread` (RRT) layer with a
synchronous `std::sync::mpsc` + `Waker` broadcast channel.

- [ ] **Step 1.1: Design and Implement `std::sync::mpsc` Broadcast Channel with `Waker`**
    - Implement `SyncBroadcastSender` and `SyncBroadcastReceiver` in `rrt_types.rs` using
      `std::sync::mpsc::channel()`, `Arc<AtomicUsize>` for `receiver_count()`, and
      `Arc<ScopedMutex<Option<std::task::Waker>, { DeadlockPreventionPolicy::PanicOnSpecificLockNesting }>>`
      for async notification.
    - Support `receiver_count()`, `subscribe()`, blocking `recv()`, and runtime-agnostic
      `recv_async()`.
- [ ] **Step 1.2: Refactor RRT Worker & Engine to use Sync Channel**
    - Update `rrt.rs`, `rrt_engine.rs`, `rrt_input_sender.rs`, `rrt_subscriber_guard.rs`
      to use `SyncBroadcastSender` / `SyncBroadcastReceiver`.
    - Preserve RFC 1857 drop order in `SubscriberGuard`.
- [ ] **Step 1.3: Update RRT Unit & Integration Tests**
    - Update `rrt_integration_tests` to test both synchronous `recv()` and `recv_async()`.
- [ ] **Step 1.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/core/resilient_reactor_thread/rrt.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_engine.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_types.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_worker.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_input_sender.rs`

---

### Phase 2: `InputDevice` & Backends

Convert `InputDevice` and its `direct_to_ansi`, `crossterm`, and `MockInputDevice`
backends to synchronous `next()` with a pure-`std` `next_async()` adapter.

- [ ] **Step 2.1: Update `DirectToAnsiInputDevice`**
    - Change `pub fn next(&mut self) -> Option<InputEvent>` to call blocking
      `self.subscriber_guard.receiver.recv()`.
    - Add `pub fn next_async(&mut self)` backed by
      `self.subscriber_guard.receiver.recv_async()`.
- [ ] **Step 2.2: Update `CrosstermInputDevice`**
    - Remove `crossterm::event::EventStream`.
    - Implement `pub fn next(&mut self) -> Option<InputEvent>` using
      `crossterm::event::read()`.
- [ ] **Step 2.3: Update `MockInputDevice` and `InputDevice`**
    - Update `InputDevice::next(&mut self)` to be synchronous and provide
      `InputDevice::next_async(&mut self)` via pure-`std` `Future`.
    - Update `MockInputDevice::next(&mut self)` to be synchronous.
- [ ] **Step 2.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/input_device_impl.rs`
    - [ ] `tui/src/core/terminal_io/input_device.rs`
    - [ ] `tui/src/core/terminal_io/mock_input_device.rs`

---

### Phase 3: `TerminalWindow` Event Loop

Synchronize the full TUI event loop (`TerminalWindow::main_event_loop`) and global data
signal channels.

- [ ] **Step 3.1: Synchronize `run_main_event_loop` with Unified `std::sync::mpsc`**
    - Replace `tokio::select!` with a unified `std::sync::mpsc::Receiver` loop that
      processes both `TerminalWindowMainThreadSignal` and `InputEvent`.
    - Convert `run_main_event_loop` from `async fn` to synchronous `fn`.
    - Replace `tokio::spawn` in `request_exit_by_sending_signal` and `send_signal!` with
      direct non-blocking `std::sync::mpsc::Sender::send`.
- [ ] **Step 3.2: Update `main_event_loop_impl` & `TerminalWindow::main_event_loop`**
    - Remove `MainEventLoopFuture` (`Pin<Box<dyn Future>>`).
    - Return `TuiAvailability<CommonResult<...>>` synchronously from
      `TerminalWindow::main_event_loop`.
    - Provide `TerminalWindow::main_event_loop_async` (and async adapter helpers for
      `choose` / `readline`) using `StdThreadRunner::run_async` for async callers.
- [ ] **Step 3.3: Update Global Data Channels**
    - Convert `main_thread_channel` from `tokio::sync::mpsc` to `std::sync::mpsc`.
- [ ] **Step 3.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_api.rs`
    - [ ] `tui/src/tui/terminal_window/shared_global_data.rs`

---

### Phase 4: `StdThreadRunner`

Transform `std_thread_tokio_runner` into `std_thread_runner` and implement the pure-`std`
`ThreadJoinFuture<T>` adapter.

- [ ] **Step 4.1: Refactor `std_thread_tokio_runner.rs` to `std_thread_runner.rs`**
    - Replace `AsyncFnOnce() -> T` with `FnOnce() -> T`.
    - Remove all Tokio runtime building logic
      (`tokio::runtime::Builder::new_current_thread()`).
    - Simplify `run_blocking` and `run_non_blocking` to pure `std::thread` execution.
    - Implement `ThreadJoinFuture<T>` (`std::future::Future` + `std::task::Waker`) and
      `StdThreadRunner::run_async` so any async runtime (`tokio`, `smol`, `async-std`) can
      `.await` background or TUI threads without blocking executor threads.
- [ ] **Step 4.2: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/core/common/std_thread_tokio_runner.rs`

---

### Phase 5: Remaining `tui` Modules

Delete `AsyncPtySession`, synchronize `Spinner`, `core::script`, and `network_io`, and
move `tokio` from production `[dependencies]` to `[dev-dependencies]` in `tui/Cargo.toml`.

- [ ] **Step 5.1: Delete `AsyncPtySession` and `PtySessionBuilder::start_async()`**
    - Remove `AsyncPtySession`, `PtySessionBuilder::start_async()`, and the `Async*` type
      aliases from `tui/src/core/pty/pty_session/type_aliases.rs`, `session.rs`, and
      `builder.rs`.
    - Update `tui/src/core/pty/mod.rs` and `tui/src/core/pty/pty_session/mod.rs` rustdoc
      to remove `AsyncPtySession` references.
- [ ] **Step 5.2: Convert `Spinner` to `std::thread` + `recv_timeout`**
    - Replace `tokio::time::interval` and `tokio::sync::broadcast` in
      `tui/src/readline_async/spinner.rs` with `std::thread` and
      `std::sync::mpsc::Receiver::recv_timeout(interval)`.
- [ ] **Step 5.3: Synchronize `core::script` and `network_io`**
    - Convert `tui/src/core/script/` (`git.rs`, `env_source.rs`, `download.rs`,
      `crates_api.rs`, `github_api.rs`, `http_client.rs`, `package_manager.rs`,
      `permissions.rs`) to use `std::process::Command`, `std::fs`, and a pure-sync HTTP
      client (`ureq` with `rustls` and `json`).
    - Convert `length_prefix_protocol.rs` to use `std::io::{Read, Write}` instead of
      `tokio::io`.
- [ ] **Step 5.4: Move `tokio` to `[dev-dependencies]` & Remove Async Crates from
      `[dependencies]` in `tui/Cargo.toml`**
    - Remove `tokio`, `futures-util`, `futures-core`, `async-stream`, and `reqwest` from
      production `[dependencies]`, and drop the `"event-stream"` feature from `crossterm`.
    - Add a minimal `tokio` entry to `[dev-dependencies]` in `tui/Cargo.toml`:
        ```toml
        tokio = { version = "1.48.0", default-features = false, features = ["rt", "rt-multi-thread", "macros"] }
        ```
        Using `default-features = false` with only `["rt", "rt-multi-thread", "macros"]`
        (instead of `"full"`) keeps test compilation fast while allowing integration tests
        and doctests to empirically verify our pure-`std` async adapters
        (`ThreadJoinFuture`, `recv_async()`, `InputDevice::next_async()`) against a real
        Tokio runtime (`tokio::select!`, `tokio::spawn`, `current_thread`, and
        `multi_thread`). (`[dev-dependencies]` are never downloaded or compiled by
        downstream consumers of `r3bl_tui`.)
    - Verify no external channel crate (`crossbeam-channel`) is added.
- [ ] **Step 5.5: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/core/pty/pty_session/type_aliases.rs`
    - [ ] `tui/src/core/pty/pty_session/session.rs`
    - [ ] `tui/src/core/pty/pty_session/builder.rs`
    - [ ] `tui/src/readline_async/spinner.rs`
    - [ ] `tui/src/core/script/mod.rs` (and submodules)
    - [ ] `tui/src/network_io/length_prefix_protocol.rs`
    - [ ] `tui/Cargo.toml`

---

### Phase 6: Downstream Crates

Update downstream workspace crates (`cmdr` and `build-infra`, including `edi`, `giti`,
`self_upgrade`, and `cargo-rustdoc-fmt`) to call the synchronous `r3bl_tui` APIs and drop
`tokio` from their `Cargo.toml` files.

- [ ] **Step 6.1: Update `build-infra` (`cargo-rustdoc-fmt`) & Drop Tokio**
    - Remove `#[tokio::main]` and `.await` calls on `try_is_git_repo()` /
      `try_get_changed_files_by_ext()` in `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
    - Remove `tokio` from `build-infra/Cargo.toml` and run
      `cargo install --path build-infra --force`.
- [ ] **Step 6.2: Update `edi` Editor Launcher**
    - Convert `cmdr/src/edi/launcher.rs` and `cmdr/src/bin/edi.rs` from `async fn` to sync
      `fn`.
    - Remove `future.await` on `TerminalWindow::main_event_loop`.
- [ ] **Step 6.3: Update `giti` Branch & Commands**
    - Convert `cmdr/src/bin/giti.rs` and `cmdr/src/giti/` commands from `async fn` to
      synchronous `fn`, removing `.await` on `choose()` and git script helpers.
- [ ] **Step 6.4: Update `self_upgrade` & `analytics_client` and Drop Tokio from
      `cmdr/Cargo.toml`**
    - Update `cmdr/src/self_upgrade/upgrade/build.rs` to use synchronous
      `PtySessionBuilder::start()` instead of `start_async()`.
    - Update `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs` and
      `cmdr/src/analytics_client/` to use synchronous `StdThreadRunner` and pure-sync HTTP
      calls.
    - Remove `tokio`, `futures-util`, and `reqwest` from `cmdr/Cargo.toml`.
- [ ] **Step 6.5: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/Cargo.toml`
    - [ ] `cmdr/src/bin/edi.rs`
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/edi/launcher.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/build.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs`
    - [ ] `cmdr/Cargo.toml`

---

### Phase 7: Tests & Documentation

Migrate unit and integration tests, add `[dev-dependencies]` Tokio adapter tests, and
update crate documentation.

- [ ] **Step 7.1: Migrate Sync Integration Tests & Add Real Tokio `[dev-dependencies]`
      Adapter Tests**
    - Convert synchronous TUI/RRT tests from `#[tokio::test]` to standard `#[test]`.
    - Using `tokio` in `[dev-dependencies]`, add dedicated async adapter integration tests
      (`#[tokio::test]` in both `current_thread` and `multi_thread` flavors) verifying
      that `StdThreadRunner::run_async` (`ThreadJoinFuture`),
      `SyncBroadcastReceiver::recv_async()`, `InputDevice::next_async()`, and
      `main_thread_channel_sender` work inside real `tokio::select!` and `tokio::spawn`
      workflows.
- [ ] **Step 7.2: Documentation & Link Updates**
    - Update rustdoc comments referencing Tokio in `tui/src/lib.rs`, `tui/README.md`, and
      `cmdr/README.md`, and document how async customers (`tokio`, `smol`, `async-std`)
      use the pure-`std` `Future` adapters and `main_thread_channel_sender` (with runnable
      doctests using `[dev-dependencies]` `tokio`).
    - Run `cargo rustdoc-fmt`.
- [ ] **Step 7.3: Full Quality Check**
    - Run `./check.fish --fmt`
    - Run `./check.fish --clippy`
    - Run `./check.fish --test`
- [ ] **Mandatory manual review:**
    - [ ] All modified test files
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/README.md`
    - [ ] `cmdr/README.md`

---

## Success Criteria

1. **Zero Production Tokio Across All Workspace Crates (`tui`, `cmdr`, `build-infra`,
   `rust-analyzer-mcp-server`)**: `tokio`, `futures-util`, `futures-core`, and
   `async-stream` are completely removed from production `[dependencies]` in all
   `Cargo.toml` files, and all internal channels/futures use 100% pure `std`
   (`std::sync::mpsc`, `std::thread`, `std::future::Future`, `std::task::Waker`). (`tokio`
   remains **only** in `tui`'s `[dev-dependencies]` to test the async adapters against a
   real Tokio runtime.)
2. **Runtime-Agnostic Async Support**: Customers using `tokio`, `smol`, `async-std`, or
   any `std::future::Future` executor can `.await` TUI entry points (`ThreadJoinFuture`),
   `.await` input events (`next_async`), and send `TerminalWindowMainThreadSignal`s from
   async tasks without blocking executor worker threads.
3. **Simplified Type Signatures**: No `MainEventLoopFuture`, `ChooseFuture`,
   `AsyncPtySession`, `AsyncFnOnce`, or `Pin<Box<dyn Future>>` across primary TUI entry
   points.
4. **Parity**: All interactive behavior in `edi`, `giti`, `choose`, `readline`, and
   `Spinner` operates with identical responsiveness and zero-latency ESC key handling.
5. **Clean Quality Gate**: `./check.fish --check`, `./check.fish --clippy`, and all
   unit/PTY tests pass.

---

## Future-Proofing: Multi-Socket Servers

This section documents how future multi-socket servers (`Unix`, `TCP`, `UDP`) can be
multiplexed on a single OS thread via `RRT` + `mio` with zero Tokio dependency.

If future R3BL applications or TUI hosts need to embed in-process servers (exposing Unix
domain sockets, TCP listeners/streams, or UDP sockets) **without tying up 1 OS thread per
socket**, we **still do not need Tokio** outside of `[dev-dependencies]`:

1. **What `std::net` Provides Alone**:
    - `std::net::{TcpListener, TcpStream, UdpSocket}` and
      `std::os::unix::net::{UnixListener, UnixStream}` support `.set_nonblocking(true)`,
      but `std` does not expose an OS readiness selector (`epoll` / `kqueue` / `IOCP`).
    - For 1-5 local IPC connections (e.g., an MCP server or local CLI daemon), standard
      blocking `std::net` with 1 OS thread per connection is already tiny (~8 KB kernel
      stack while sleeping in `read(2)`).

2. **How `RRT` + `mio` Multiplexes 10 to 10,000+ Sockets on 1 Thread**:
    - `r3bl_tui` **already** depends on [`mio`] (`mio::Poll`, `mio::Registry`,
      `mio::Waker`), which is the foundational synchronous wrapper over `epoll(7)`
      (Linux), `kqueue(2)` (macOS), and `IOCP` (Windows) that powers
      [`ResilientReactorThread` (RRT)].
    - By enabling `mio`'s `"net"` feature
      (`mio::net::{TcpListener, TcpStream, UdpSocket, UnixListener, UnixStream}`), a
      **single** [`RRTWorker`] thread can multiplex a listener socket and thousands of
      connected client sockets simultaneously at **0% idle CPU**:

```text
                       ONE RRT OS Thread
┌─────────────────────────────────────────────────────────────┐
│ mio::Poll::poll(&mut events, None)   (blocks at 0% CPU)     │
│   ├─ Token(0): MioSoftwareInterrupt  (waker / shutdown)     │
│   ├─ Token(1): UnixListener / TcpListener (accepts clients) │
│   ├─ Token(2): UdpSocket                                    │
│   ├─ Token(3): Client #1 (UnixStream / TcpStream)           │
│   ├─ Token(4): Client #2 (UnixStream / TcpStream)           │
│   └─ Token(N): Client #N ...                                │
└──────────────────────────────┬──────────────────────────────┘
                               │ broadcasts parsed messages
                               ▼
              SyncBroadcastSender (std::sync::mpsc)
                               │
                               ▼
                 TerminalWindow::main_event_loop
```

- **Workflow inside the `RRTWorker`**:
    1.  `poll.poll(&mut events, None)` sleeps in the kernel until _any_ registered socket
        or the `MioSoftwareInterrupt` waker is ready.
    2.  When `Token(1)` (the listener) is ready, `.accept()` yields a non-blocking
        `UnixStream` / `TcpStream`, assigns it `Token(next_id)`, and registers it on the
        **same** `poll.registry()`.
    3.  When `Token(client_id)` is ready, available bytes are read into that client's
        frame buffer, decoded (e.g., via `length_prefix_protocol.rs`), and broadcast to
        the TUI via `SyncBroadcastSender` (`std::sync::mpsc`).
    4.  Dropping the `SubscriberGuard` triggers `MioSoftwareInterrupt` (`mio::Waker`),
        waking `poll.poll()` immediately for clean, deterministic RAII teardown.

**Conclusion**: Between `std::net` (for simple 1:1 pipes), `RRT` + `mio` (for 1:N
non-blocking socket multiplexing on a single OS thread), and `std::future::Future` +
`std::task::Waker` (for downstream async callers), **there is never a need to re-introduce
Tokio as a production dependency.**

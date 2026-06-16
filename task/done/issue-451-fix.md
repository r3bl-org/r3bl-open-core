# Task: RRT Per-Worker Startup Config and Inbound Message Passing

## Overview

This task addresses [Issue 451](https://github.com/r3bl-org/r3bl-open-core/issues/451),
which identifies two patterns in the Resilient Reactor Thread (RRT) framework that
currently require user-space boilerplate and static globals (`OnceLock`).

## Why do we need this?

## Per-Worker Startup Configuration

- _Eliminate Global State & Boilerplate_: Currently,
  `RRTWorker::create_and_register_os_sources` takes no arguments, forcing users to store
  worker config (like directory watch paths or LSP configurations) in process-global
  static variables (`OnceLock`). This introduces dangerous global state and makes the code
  difficult to reason about.
- _Enable Independent Worker Instances_: With startup configs tied to global statics, you
  cannot instantiate multiple workers of the same type with different settings.
- _Example & Contrast_:
  - **Contrast (MioPollWorker)**: The existing `MioPollWorker` handles terminal input by
    polling `stdin` and `SIGWINCH` signals. Since `stdin` is a fixed, process-global
    resource, `MioPollWorker` needs no dynamic startup configuration.
  - **Example (FileWatcherWorker)**: If we want to implement a worker that monitors files
    for changes using `mio`, it needs to know _which_ directory to watch (e.g.,
    `/path/to/project_a`). Without per-worker config, we cannot run two separate watchers
    watching different paths concurrently because they would have to read from the same
    global static path.

## Inbound Message Passing

- _Support UI-to-Worker Control Flow_: Async UI code (running on Tokio tasks) needs a
  first-class way to send commands (e.g., `"request semantic tokens for file X"`) to
  blocking workers (running on dedicated RRT threads).
- _Ensure Stable Senders on Restart_: RRT handles worker crashes by dropping the old
  worker and creating a new one on restart. If a communication channel is created inside
  the worker, the sender held by the UI goes stale on restart. The framework must handle
  swapping or rewiring these channel senders transparently.
- _Example & Contrast_:
  - **Contrast (MioPollWorker)**: `MioPollWorker` only has one-way outbound data flow (it
    reads from `stdin` and pushes key/mouse events _out_ to the TUI). The TUI never needs
    to send messages back to the input poller.
  - **Example (SubprocessWorker)**: If we want to run a background shell/subprocess (like
    a terminal multiplexer session or LSP server) inside RRT, we need bidirectional flow.
    The worker sends subprocess stdout _out_ to the TUI (via `W::Output`), and the TUI
    needs to send keystrokes _in_ to the subprocess (via an inbound channel). If the
    subprocess crashes, RRT restarts the worker. The user-facing keystroke sender must not
    go stale/disconnected when this restart occurs.

## What do we need?

1. **Per-worker startup configuration**: Currently,
   `RRTWorker::create_and_register_os_sources` is a static method with no parameters. Any
   construction-time configuration (e.g., watch paths, LSP configs) must be injected via
   process-global statics.
2. **External/inbound message passing**: External async code (like the TUI) needs a way to
   send messages to the RRT worker. If a channel is created inside the worker, it is
   dropped on worker restart, making the sender end stale. If created outside, the sender
   end must be manually updated in a static slot on every restart.

The goal is to provide a first-class, framework-managed way to handle startup
configuration and inbound message passing that survives worker restarts.

---

## Problem Space & Constraints

### 1. The RRT Lifecycle and Thread Spawning

The RRT framework manages the lifecycle of a single dedicated thread. The thread is
spawned on demand when a subscriber calls `try_subscribe()`. When a worker encounters a
restart condition (returning `Continuation::Restart`), the RRT engine drops the current
worker instance, applies a delay/backoff, and calls `W::create_and_register_os_sources()`
again to instantiate a fresh worker.

```
[Subscriber] ──try_subscribe()──► [RRT Engine]
                                       │
                              (spawn thread loop)
                                       │
                                       ▼
                       W::create_and_register_os_sources()
                                       │
                                       ▼
                       [run_worker_loop (blocking)]
                                       │
                       (Continuation::Restart returned)
                                       │
                                       ▼
                            (drop current worker)
                                       │
                                       ▼
                       W::create_and_register_os_sources()
```

### 2. Constraints on Config and Messaging

- **Static Singleton Compatibility**: The `RRT<W>` struct is declared as a `static`
  singleton. Its fields must be initialized at compile time via `const fn` (typically
  deferred via `LazyLock`). Therefore, RRT cannot hold a dynamic config at construction
  time.
- **Worker Recreation on Restart**: Since the worker is dropped and recreated, any
  configuration or inbound channel receiver held by the worker is also dropped. The
  framework must be able to re-supply the startup config and re-wire/provide a fresh
  channel/sender on restart.
- **No Extra Dependencies/Low Overhead**: The design must not enforce a specific channel
  implementation if it can be avoided, or it should use standard/already available
  dependencies (like `tokio::sync::mpsc`). When no config or inbound channel is needed,
  the overhead should be zero.

---

## Design Alternatives

### Alternative A: Framework-Managed Tokio Channel via Associated Types

Define `Config` and `Input` as associated types on `RRTWorker`. To keep the naming
perfectly orthogonal, we will also rename the existing `Event` type to `Output`. The
framework handles the creation of a `tokio::sync::broadcast` channel.

#### The "Input Sender" Concept

Instead of giving the async consumer a raw `tokio::sync::broadcast::Sender` that goes dead
when the worker restarts, the framework will provide an `InputSender<W>` wrapper. If
`input_sender.send(msg)` returns an error (disconnected), the `InputSender` will
automatically query the `ThreadState` for the newly created underlying channel and retry
the send, making restarts completely transparent to the user.

This retry logic inherently protects against race conditions where the worker might crash
multiple times in rapid succession, as the `InputSender` loops and re-verifies the
`ThreadState` lock until the message is successfully delivered or the thread permanently
stops.

#### API Changes

##### 1. `RRTWorker` Trait

Add associated types and update the method signature:

```rust
// this is a snippet from: tui/src/core/resilient_reactor_thread/rrt_worker.rs
pub trait RRTWorker: Send + Debug + 'static {
    /// Capacity of the `tokio::sync::broadcast` channel for inbound commands or data sent to the worker.
    /// Default is `1_024`. Override this if your worker requires a different inbound buffer size.
    const INPUT_CHANNEL_CAPACITY: usize = 1_024;

    /// Capacity of the `tokio::sync::broadcast` channel for outbound events or data produced by the worker.
    /// Default is `4_096`. Override this if your worker is high-volume (e.g., `stdout` streaming).
    const OUTPUT_CHANNEL_CAPACITY: usize = 4_096;

    /// The type containing startup configuration passed to `create_and_register_os_sources`.
    /// Set to `()` if your worker does not require dynamic configuration.
    type Config: Clone + Send + 'static;

    /// The type containing domain-specific commands or data sent into your worker from
    /// async code. Set to `()` if your worker does not require inbound messaging.
    type Input: Clone + Send + 'static;

    fn create_and_register_os_sources(
        config: Self::Config,
        receiver: tokio::sync::broadcast::Receiver<Self::Input>,
    ) -> miette::Result<(Self, Self::Interrupt)>
    where
        Self: Sized;

    // ...
}
```

##### 2. `ThreadState` Enum

Update the `Running` variant to hold the `broadcast::Sender`:

```rust
pub enum ThreadState<W: RRTWorker> {
    Stopped,
    Starting,
    Running(InterruptHandle<W::Interrupt>, tokio::sync::broadcast::Sender<W::Input>),
    Stopping(StopReason),
    Restarting,
}
```

##### Design Decision: Wait Mechanism (Notify vs Watch)

When `InputSender` encounters a restart, it needs a way to await the thread state changing
back to `Running` without polling. We evaluated two Tokio primitives for this:

- **`tokio::sync::Notify` (Chosen)**: The RRT engine manages a blocking OS thread, which
  natively relies on blocking synchronization (like its current `std::sync::Condvar`). A
  `Notify` can simply be triggered alongside the `Condvar` without changing the core
  locking architecture, bridging the blocking and async worlds cleanly.
- **`tokio::sync::watch` (Rejected)**: While `watch` channels are the canonical way for
  async state observation (offering lock-free reads and no race conditions), they don't
  fit RRT's design or reason for existing, which is to manage a blocking resource.
  Replacing the `Condvar` by rewriting RRT to use them would force the blocking OS thread
  to manage its lifecycle using Tokio's async channels (clunky async bridges), which is an
  architectural mismatch.

##### 3. `ThreadLifecycleMonitor`

Add the `tokio::sync::Notify` to the monitor so async consumers can await state changes
without polling.

```rust
pub struct ThreadLifecycleMonitor<W: RRTWorker> {
    pub state: std::sync::Mutex<ThreadState<W>>,
    pub condvar: std::sync::Condvar,
    pub input_sender_notify: tokio::sync::Notify, // NEW
}

impl<W: RRTWorker> ThreadLifecycleMonitor<W> {
    pub fn set_state<'a>(
        &'a self,
        mut guard: std::sync::MutexGuard<'a, ThreadState<W>>,
        new_state: ThreadState<W>,
    ) -> std::sync::MutexGuard<'a, ThreadState<W>> {
        *guard = new_state;
        // Trigger Tokio tasks waiting on InputSender
        self.input_sender_notify.notify_waiters();
        guard
    }
}
```

##### 4. `RRT` and `SubscriberGuard` Methods

Add `try_subscribe` parameter and a method to get the active sender:

````rust
// this is a snippet from: tui/src/core/resilient_reactor_thread/rrt.rs
impl<W: RRTWorker> RRT<W> {
    pub fn try_subscribe(
        &self,
        config: W::Config,
    ) -> Result<SubscriberGuard<W>, SubscribeError> {
        try_subscribe::<W>(&self.sender, &self.shared_state, config)
    }

    /// Returns a [`InputSender`] that allows you to send `W::Input` messages into the
    /// blocking worker thread.
    ///
    /// Unlike a raw `tokio::sync::broadcast::Sender`, this `InputSender` is aware of the
    /// RRT lifecycle. If the worker encounters an error and is restarted by the framework,
    /// the underlying channel is dropped and recreated. The `InputSender` handles this
    /// transparently by intercepting disconnect errors, waiting for the new worker to spin up,
    /// and retrying the send operation on the new channel.
    ///
    /// # Optional Usage for Worker Authors
    ///
    /// Inbound messages are functionally optional. If you are writing a worker that doesn't
    /// care about inbound messages from the UI (e.g. `MioPollWorker`), you just "opt out" at
    /// the type level:
    ///
    /// 1. You define `type Input = ();` in your `impl RRTWorker` block.
    /// 2. You ignore the argument in your implementation by prefixing it with an underscore:
    ///
    /// ```rust
    /// impl RRTWorker for MioPollWorker {
    ///     type Config = ();
    ///     type Input = ();
    ///     // ... other associated types ...
    ///
    ///     fn create_and_register_os_sources(
    ///         _config: Self::Config,
    ///         _receiver: tokio::sync::broadcast::Receiver<Self::Input>,
    ///     ) -> miette::Result<(Self, Self::Interrupt)> {
    ///         // ... implementation ...
    ///     }
    ///     // Provides the smart sender
    ///     pub fn get_input_sender(&self) -> InputSender<W> {
    ///         InputSender {
    ///             shared_state: self.shared_state.clone(),
    ///         }
    ///     }
    ///     // ...
    /// }
    /// ```
    pub fn get_input_sender(&self) -> InputSender<W> {
        InputSender {
            shared_state: self.shared_state.clone(),
        }
    }
}
````

##### 5. `InputSender` Implementation

This is a new struct that we will add in
`tui/src/core/resilient_reactor_thread/rrt_input_sender.rs`.

```rust
use tokio::time::sleep;
use std::time::Duration;
use crate::ok;

pub struct InputSender<W: RRTWorker> {
    pub shared_state: Arc<ThreadLifecycleMonitor<W>>,
}

impl<W: RRTWorker> InputSender<W> {
    /// Sends a message into the blocking worker thread, transparently handling
    /// worker lifecycle events.
    ///
    /// If the worker is currently restarting, this method will asynchronously
    /// yield and wait until the new worker is ready before attempting the send.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread state is [`Stopping`] or [`Stopped`], meaning
    /// the worker has permanently shut down and can no longer receive messages.
    ///
    /// # Implementation Details
    ///
    /// This method uses a retry loop. If [`input_sender_notify`] wakes this task up,
    /// but the worker crashes *again* before we acquire the [`Mutex`] lock, the inner
    /// match arm observes the transient state, drops the lock, and goes back to sleep.
    ///
    /// [`input_sender_notify`]: ThreadLifecycleMonitor::input_sender_notify
    pub async fn send(&self, msg: W::Input) -> Result<(), miette::Report> {
        loop {
            // 1. Get the current sender from ThreadState
            let sender = {
                let guard = self.shared_state.lock();
                match &*guard.state {
                    ThreadState::Running(_, tx) => tx.clone(),
                    ThreadState::Starting | ThreadState::Restarting => {
                        // Transient states: wait for it to become Running.
                        // RACE CONDITION HANDLING: This match arm inherently handles the
                        // edge case where `Notify` wakes us up, but the worker crashes
                        // again *before* we acquire the Mutex lock. We simply observe
                        // that it is dead again, drop the lock, and go back to sleep.
                        drop(guard); // release lock before awaiting
                        self.shared_state.input_sender_notify.notified().await;
                        continue;
                    }
                    ThreadState::Stopping(_) | ThreadState::Stopped => {
                        // Permanent states: abort!
                        return Err(miette::miette!("Worker thread is permanently stopped"));
                    }
                }
            };

            // 2. Try to send
            match sender.send(msg.clone()) {
                Ok(_) => return Ok(()),
                Err(_) => {
                    // Channel disconnected (worker restarting).
                    // Loop around to fetch the NEW sender from state!
                    // We do not wait for notify here, because by the time the
                    // channel disconnected, the state likely already changed,
                    // so we just loop and let the match statement above handle the waiting.
                    continue;
                }
            }
        }
    }
}
```

##### 6. Engine loop `run_worker_loop`

Update `run_worker_loop` and `perform_restart_retry_loop` to keep and reuse the config,
and instantiate the `tokio::sync::broadcast::channel`:

```rust
pub fn run_worker_loop<W: RRTWorker>(
    worker: W,
    config: W::Config,
    sender: BroadcastSender<W::Output>,
    shared_state: Arc<ThreadLifecycleMonitor<W>>,
) {
    // ...
}
```

- **Pros**:
  - Fully managed by the framework.
  - Very clean API.
- **Cons**:
  - Forces the use of `tokio::sync::broadcast::channel`. The worker might want to block
    synchronously on a standard library `std::sync::mpsc::Receiver` or use a different
    channel library.
  - Requires the framework to handle channel recreation and forwarding.

---

### Alternative B: Associated Types for Config and Inbound Sender (Highly Flexible)

Define `Config` and `InboundSender` as associated types. The worker itself instantiates
the channel and returns the sender. The framework stores the sender and makes it
queryable.

```rust
pub trait RRTWorker: Send + Debug + 'static {
    type Config: Clone + Send + 'static;
    type InboundSender: Clone + Send + 'static;
    type Output: Clone + Send + Sync + 'static;
    type Interrupt: RRTSoftwareInterrupt;

    fn create_and_register_os_sources(
        config: Self::Config,
    ) -> miette::Result<(Self, Self::Interrupt, Self::InboundSender)>
    where
        Self: Sized;
    // ...
}
```

- **How it works**:
  - `try_subscribe(config: W::Config)` moves/clones the config into the thread loop:
    ```rust
    impl<W: RRTWorker> RRT<W> {
        pub fn try_subscribe(
            &self,
            config: W::Config,
        ) -> Result<SubscriberGuard<W>, SubscribeError> {
            try_subscribe::<W>(&self.sender, &self.shared_state, config)
        }
    }
    ```
  - The worker creates whatever channel it wants inside `create_and_register_os_sources`
    and returns the sender.
  - The framework stores this `InboundSender` inside the `ThreadState::Running` variant:
    ```rust
    pub enum ThreadState<W: RRTWorker> {
        Stopped,
        Starting,
        Running(InterruptHandle<W::Interrupt>, W::InboundSender),
        Stopping(StopReason),
        Restarting,
    }
    ```
  - The subscriber queries the current sender using `SubscriberGuard::get_input_sender()`:
    ```rust
    impl<W: RRTWorker> SubscriberGuard<W> {
        }
    }
    ```
  - If a restart happens, a new channel is created by `create_and_register_os_sources`,
    and RRT automatically swaps the new sender into the `Running` state.
- **Pros**:
  - Complete control: the worker decides which channel type to use (std, tokio, crossbeam,
    etc.).
  - Zero framework overhead when not used: if a worker doesn't need configuration or
    inbound messaging, it can define `type Config = ()` and `type InboundSender = ()`.
  - Swapping the sender on restart is handled automatically by the typestate/liveness
    machine.
- **Cons**:
  - Old senders held by subscribers become disconnected on restart. Subscribers must query
    `input_sender()` again if they receive a send error.

#### Why Alternative B was Rejected

While Alternative B offers ultimate channel flexibility, it violates the core design
philosophy of "low cognitive load" by forcing either the async consumer or the worker
author to handle restart boilerplate.

Because `InboundSender` is an opaque type, the framework does not know how to invoke
`.send()` on it. To fix this and provide a seamless retry-on-disconnect experience, we
would have to choose between three flawed approaches:

1. **The "Smart Getter"**: The framework provides `get_sender().await` which waits for the
   thread to be healthy. The consumer still has to write a manual retry loop every time
   they send a message:
   `loop { match get_sender().await.send(msg) { Ok => break, Err => continue } }`.
2. **The Closure Wrapper**: The framework provides `smart_send(|tx| tx.send(msg))`. This
   requires complex trait bounds to standardize disconnect errors across different channel
   libraries.
3. **The Universal `RRTChannelSender` Trait**: The framework defines an async sender
   trait. This shifts the burden to the _worker author_, who must now write a wrapper
   struct around their custom channel, implement the async trait (reconciling sync vs.
   async send signatures), and map their specific channel's error types into a
   standardized framework error enum.

To keep the API simple and "batteries included," we rejected Alternative B. We trade the
flexibility of choosing a custom channel for the zero-boilerplate guarantee of Alternative
A's framework-managed `InputSender`.

---

## Proposed Design (Chosen: Alternative A)

We choose **Alternative A** because it provides a superior developer experience through an
"Input Sender." Even though it forces a specific channel type (`tokio::sync::broadcast`),
this trade-off is worth it because the framework can fully hide worker restarts from the
async consumer.

---

## Implementation Plan

### Phase 1: RRT API Refactoring

> **CRITICAL**: Do NOT generate new Rustdocs from scratch. You MUST copy the exact code
> snippets and their accompanying Rustdocs directly from the `Alternative A` section above
> into the source code. These snippets were explicitly designed to be the final
> implementation.

- [x] Update `RRTWorker` trait to define `Config` and `Input` associated types. Rename
      existing `Event` type to `Output`. Rename `CHANNEL_CAPACITY` to
      `OUTPUT_CHANNEL_CAPACITY` and add `INPUT_CHANNEL_CAPACITY`.
- [x] Update `RRTWorker::create_and_register_os_sources` signature to accept `config` and
      `receiver`, and return the tuple.
- [x] Update `ThreadState` enum to store the `tokio::sync::broadcast::Sender` in the
      `Running` variant.
- [x] Add `state_change_notify: AsyncNotify` to `ThreadLifecycleMonitor` and trigger it
      inside `set_state()`.
- [x] Implement the `InputSender<W>` struct which wraps `shared_state` and handles
      retry-on-disconnect logic using `state_change_notify`.
- [x] Add the `get_input_sender()` method to `RRT` and `SubscriberGuard`.
- [x] **Mandatory manual review:** Verify API signatures.
  - [x] `tui/src/core/resilient_reactor_thread/rrt_worker.rs`
  - [x] `tui/src/core/resilient_reactor_thread/rrt.rs`
  - [x] `tui/src/core/resilient_reactor_thread/rrt_thread_state.rs`
  - [x] `tui/src/core/resilient_reactor_thread/rrt_monitor.rs`
  - [x] `tui/src/core/resilient_reactor_thread/rrt_input_sender.rs`

### Phase 2: Engine & Spawning Refactoring

- [x] Update `try_subscribe` function in `rrt.rs` to take `config`, create the initial
      `tokio::sync::broadcast` channel, and store the sender.
- [x] Update `run_worker_loop` and `perform_restart_retry_loop` in `rrt_engine.rs` to hold
      the config, and recreate the channel upon restart.

### Phase 3: Adapting Existing Workers & Tests

- [x] Update `MioPollWorker` and other internal RRT workers to use `type Config = ()` and
      `type Input = ()` (or appropriate mock types), and rename `Event` to `Output`.
- [x] Fix all compile errors in the RRT tests.
- [x] Run RRT tests to ensure liveness, restart policy, and channel behavior still pass.

### Phase 4: Mandatory manual review

- [x] Verify RRT engine loop changes.
  - [x] `tui/src/core/resilient_reactor_thread/rrt.rs`
  - [x] `tui/src/core/resilient_reactor_thread/rrt_engine.rs`
- [x] Verify all tests and adapted workers compile and run successfully.
  - [x] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/`
  - [x] `tui/src/core/resilient_reactor_thread/process_isolated_tests/`

## Test coverage

### Phase 5: Test Coverage for Config & Smart Sender

#### Phase 5.1: Unit Test for `InputSender` State Transitions

- [x] Create `tui/src/core/resilient_reactor_thread/unit_tests/group_c_input_sender.rs`.
- [x] Test the asynchronous retry loop of `InputSender` in isolation:
  - [x] Manually instantiate a `ThreadLifecycleMonitor` and an `InputSender`.
  - [x] Set the state to `ThreadState::Restarting`.
  - [x] Spawn a Tokio task that calls `input_sender.send("msg")`. It should yield and
        wait.
  - [x] From the main thread, transition the state to `ThreadState::Running` with a fresh
        channel and trigger `input_sender_notify`.
  - [x] Assert the blocked Tokio task wakes up, successfully delivers the message to the
        new channel, and it is received correctly.

#### Phase 5.2: End-to-End Integration Test

- [x] Create
      `tui/src/core/resilient_reactor_thread/rrt_integration_tests/smart_sender_test.rs`.
- [x] Verify the end-to-end behavior of `Config` preservation and `InputSender` retry
      within the actual running engine:
  - [x] Define `SmartWorker` with `type Config = String;` and `type Input = String;`.
  - [x] Ensure `create_and_register_os_sources` asserts that `config` equals expected
        value.
  - [x] The `block_until_ready_then_dispatch` will await the `Input` channel.
  - [x] Start `RRT<SmartWorker>` with config `"test_config_value"`.
  - [x] Grab the `input_sender`.
  - [x] Send `"crash"`. Worker deliberately returns `Continuation::Restart`.
  - [x] Immediately `await` sending the next message `"hello after crash"`. The
        `InputSender` should seamlessly handle channel disconnect, wait for recreation,
        and deliver it.
  - [x] Verify the new worker received the preserved config, and received
        `"hello after crash"`.

#### Phase 5.3: Modernize Existing Process-Isolated Fixtures

- [x] Refactor `TestWorker` in
      `tui/src/core/resilient_reactor_thread/process_isolated_tests/fixtures.rs`.
  - [x] Change `TestWorker` to use `type Input = u8;`.
  - [x] Remove the manual `cmd_receiver: mpsc::Receiver<u8>` from `TestWorker`.
  - [x] Remove `cmd_sender` from the global `TEST_FACTORY_STATE`.
- [x] Update all process-isolated tests (e.g., `group_b_run_worker_loop.rs` and
      `group_c_rrt_integration.rs`) to retrieve the `InputSender` from the
      `SubscriberGuard` and use it to drive tests.

#### Phase 5.4: Mandatory manual review

- [x] Verify newly added unit and integration tests compile and pass.
- [x] Verify the modernized process isolated tests still run successfully and use the
      `InputSender`.
- [x] Make sure this task md file is up to date with the progress made.
- [x] Move this task file to task/done.
- [x] Make a commit.

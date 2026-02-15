<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Problem: Stranded Subscribers When Thread Dies](#problem-stranded-subscribers-when-thread-dies)
  - [Two Types of Restart](#two-types-of-restart)
  - [Two-Tier Event Model](#two-tier-event-model)
  - [Expensive Syscall Scenarios](#expensive-syscall-scenarios)
  - [Design Decisions Summary](#design-decisions-summary)
  - [Dependency on Previous Work](#dependency-on-previous-work)
- [Implementation Plan](#implementation-plan)
  - [Step 0: Add New Types](#step-0-add-new-types)
    - [Step 0.0: Add `Continuation::Restart` Variant](#step-00-add-continuationrelaunch-variant)
    - [Step 0.1: Add `RRTEvent<E>` and `ShutdownReason`](#step-01-add-rrtevente-and-shutdownreason)
    - [Step 0.2: Add `RestartPolicy` Struct](#step-02-add-restartpolicy-struct)
    - [Step 0.3: Add `restart_policy()` to `RRTFactory` Trait](#step-03-add-restart_policy-to-rrtfactory-trait)
    - [Step 0.4: Update `RRTWorker::poll_once` Signature](#step-04-update-rrtworkerpoll_once-signature)
    - [Step 0.5: Update `mod.rs` Exports](#step-05-update-modrs-exports)
  - [Step 1: Update Framework Core](#step-1-update-framework-core)
    - [Step 1.0: Update `RRT` Broadcast Channel Type](#step-10-update-rrt-broadcast-channel-type)
    - [Step 1.1: Rewrite `run_worker_loop` with Restart Policy](#step-11-rewrite-run_worker_loop-with-restart-policy)
    - [Step 1.2: Update `SubscriberGuard` Receiver Type](#step-12-update-subscriberguard-receiver-type)
  - [Step 2: Update Worker Implementation](#step-2-update-worker-implementation)
    - [Step 2.0: Update `MioPollWorker::poll_once`](#step-20-update-miopollworkerpoll_once)
    - [Step 2.1: Update Handler Functions](#step-21-update-handler-functions)
    - [Step 2.2: Update `MioPollWorkerFactory::restart_policy`](#step-22-update-miopollworkerfactoryrestart_policy)
  - [Step 3: Update Subscriber Side](#step-3-update-subscriber-side)
    - [Step 3.0: Update `InputSubscriberGuard` Type Alias](#step-30-update-inputsubscriberguard-type-alias)
    - [Step 3.1: Update Subscriber Event Loop](#step-31-update-subscriber-event-loop)
  - [Step 4: Documentation](#step-4-documentation)
    - [Step 4.0: Update Module Docs](#step-40-update-module-docs)
    - [Step 4.1: Update DI Trait Docs](#step-41-update-di-trait-docs)
    - [Step 4.2: Update Input Device Docs](#step-42-update-input-device-docs)
  - [Step 5: Verification](#step-5-verification)
    - [Step 5.0: Run Full Checks](#step-50-run-full-checks)
    - [Step 5.1: Review Existing Tests](#step-51-review-existing-tests)
  - [Files Changed (estimated)](#files-changed-estimated)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Problem: Stranded Subscribers When Thread Dies

After the structural refactoring (dissolving `RRTState`, promoting fields to `RRT`), there is a gap:
when the dedicated thread dies while subscribers exist, those subscribers are **silently stranded
forever**.

Why? The `broadcast_tx` lives in an `OnceLock` (never dropped), so `recv().await` never returns
`RecvError::Closed`. Subscribers hang indefinitely with no indication that the thread died. There is
no restart, no error notification, nothing.

## Two Types of Restart

The worker returns a `Continuation` variant that tells the framework what to do:

| Variant    | Meaning                                      | Framework response                                  |
| :--------- | :------------------------------------------- | :-------------------------------------------------- |
| `Continue` | Keep polling                                 | Call `poll_once()` again                            |
| `Stop`     | Worker is done                               | Thread exits. Always respected, never overridden    |
| `Restart` | OS resources corrupted but process is viable | Drop worker, call `F::create()` per `RestartPolicy` |

**`Stop` vs `Restart`**: The worker is the source of truth. `Stop` means the worker decided to exit
(e.g., zero receivers, stdin EOF). `Restart` means the worker's OS resources are broken but a fresh
`F::create()` might succeed (e.g., epoll fd corrupted, file descriptor limit hit temporarily).

**Panics**: Caught via `catch_unwind(AssertUnwindSafe(...))` and reported to subscribers as
`Shutdown(Panic)`. No restart is attempted - a panic signals a logic bug, not a transient resource
issue. Subscribers can call `subscribe()` to relaunch a fresh thread if appropriate.

## Two-Tier Event Model

Events have two producers with clean separation:

| Producer                   | Event                        | Example                                  |
| :------------------------- | :--------------------------- | :--------------------------------------- |
| Worker (domain)            | `RRTEvent::Worker(E)`        | `PollerEvent::Stdin(StdinEvent::Eof)`    |
| Framework (infrastructure) | `RRTEvent::Shutdown(reason)` | `ShutdownReason::RestartPolicyExhausted`, `ShutdownReason::Panic` |

The worker never sends `Shutdown`. The framework never sends domain events. Each tier owns its own
signals.

## Expensive Syscall Scenarios

`F::create()` may involve expensive or failure-prone syscalls. The `RestartPolicy` with backoff
gives the system time to recover between attempts:

| Scenario                 | What `F::create()` allocates      | Why backoff matters                                                  |
| :----------------------- | :-------------------------------- | :------------------------------------------------------------------- |
| Terminal input (current) | epoll fd, eventfd, signal handler | File descriptor limit - need time for other processes to release fds |
| Network server           | socket + bind + listen            | Port in TIME_WAIT - needs kernel timeout to expire                   |
| Serial/hardware          | `open("/dev/ttyUSB0")` + ioctl    | Device busy - other process must release it                          |
| GPU compute              | ioctl + mmap for device memory    | Scarce resource - need time for other workloads to finish            |
| IPC channel              | shared memory + semaphore         | System V IPC limits - need admin intervention or other cleanup       |

## Design Decisions Summary

| Aspect                    | Decision                                                                          |
| :------------------------ | :-------------------------------------------------------------------------------- |
| `Stop`                    | Always respected. Thread exits. No "soft restart"                                 |
| `Restart`                | Framework applies user-provided `RestartPolicy` via `F::create()`                 |
| `RestartPolicy`           | Struct with max_restarts, optional delay, optional backoff, optional max_delay    |
| Policy source             | `RRTFactory::restart_policy()` (defaulted trait method, DI pattern)               |
| Policy exhaustion         | Send `RRTEvent::Shutdown` to subscribers, then exit thread cleanly                |
| Thread exit on exhaustion | Clean exit (not panic). `subscribe()` slow path can try again later               |
| Panics                    | `catch_unwind` + `Shutdown(Panic)` notification. No restart attempted             |
| `RRTEvent<E>`             | Two variants: `Worker(E)` and `Shutdown(ShutdownReason)`. No `#[non_exhaustive]`  |
| `poll_once` signature     | `&Sender<RRTEvent<Self::Event>>` - worker wraps events in `RRTEvent::Worker(...)` |
| Liveness                  | Keep `Mutex<Option<Arc<RRTLiveness>>>` (unchanged from current)                   |
| `run_worker_loop`         | Takes `F: RRTFactory` (was `impl RRTWorker`) for hard restart via `F::create()`   |

## Dependency on Previous Work

This plan builds on the completed refactoring from `task/fix-rrt.md`:

- `RRTState` has been dissolved (fields promoted to `RRT`)
- Broadcast channel and waker wrapper use `OnceLock` (singleton-lifetime)
- `TerminationGuard` clears waker before marking terminated
- All subscribers share the same waker via `Arc<Mutex<Option<W>>>`

# Implementation Plan

## Step 0: Add New Types [COMPLETE]

Add the new types that the rest of the implementation depends on.

### Step 0.0: Add `Continuation::Restart` Variant [COMPLETE]

File: `tui/src/core/common/continuation.rs` (or wherever `Continuation` is defined)

Add `Restart` variant to the existing enum:

```rust
pub enum Continuation {
    Continue,
    Stop,
    Restart,  // NEW
}
```

### Step 0.1: Add `RRTEvent<E>` and `ShutdownReason` [COMPLETE]

File: new file in `tui/src/core/resilient_reactor_thread/` (e.g., `rrt_event.rs`)

```rust
pub enum RRTEvent<E> {
    /// Domain event produced by your [`RRTWorker`] implementation.
    Worker(E),
    /// The framework is shutting down the thread. Subscribers should take
    /// corrective action.
    Shutdown(ShutdownReason),
}

pub enum ShutdownReason {
    /// Worker returned [`Continuation::Restart`] more times than the
    /// [`RestartPolicy`] allows.
    RestartPolicyExhausted { attempts: u8 },
    /// Worker panicked. Caught via `catch_unwind`, reported to subscribers.
    Panic,
}
```

### Step 0.2: Add `RestartPolicy` Struct [COMPLETE]

File: new file in `tui/src/core/resilient_reactor_thread/` (e.g., `rrt_restart_policy.rs`)

```rust
pub struct RestartPolicy {
    /// Maximum restart attempts before giving up. `0` means never restart
    /// (policy exhaustion on first `Restart`).
    pub max_restarts: u8,
    /// Delay before the first restart attempt. [`None`] means no delay.
    pub initial_delay: Option<Duration>,
    /// Multiplier applied to the delay after each restart attempt.
    /// [`None`] means constant delay (no growth).
    pub backoff_multiplier: Option<f64>,
    /// Cap on delay growth. [`None`] means unbounded growth.
    pub max_delay: Option<Duration>,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            initial_delay: Some(Duration::from_millis(100)),
            backoff_multiplier: Some(2.0),
            max_delay: Some(Duration::from_secs(5)),
        }
    }
}
```

### Step 0.3: Add `restart_policy()` to `RRTFactory` Trait [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/rrt_di_traits.rs`

```rust
pub trait RRTFactory {
    type Event;
    type Worker: RRTWorker<Event = Self::Event>;
    type Waker: RRTWaker;

    fn create() -> Result<(Self::Worker, Self::Waker), Report>;

    /// Restart policy for this factory. Override to customize.
    fn restart_policy() -> RestartPolicy {
        RestartPolicy::default()
    }
}
```

### Step 0.4: Update `RRTWorker::poll_once` Signature [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/rrt_di_traits.rs`

```rust
pub trait RRTWorker: Send + 'static {
    type Event: Clone + Send + 'static;
    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation;
}
```

### Step 0.5: Update `mod.rs` Exports [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/mod.rs`

Add `mod rrt_event`, `mod rrt_restart_policy`, and corresponding `pub use` re-exports for
`RRTEvent`, `ShutdownReason`, `RestartPolicy`.

## Step 1: Update Framework Core [COMPLETE]

### Step 1.0: Update `RRT` Broadcast Channel Type [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/rrt.rs`

Change `broadcast_tx` from `OnceLock<Sender<F::Event>>` to `OnceLock<Sender<RRTEvent<F::Event>>>`.

Update `subscribe()` and `subscribe_to_existing()` to return `SubscriberGuard<F::Waker, F::Event>`
where the receiver is `Receiver<RRTEvent<F::Event>>`.

### Step 1.1: Rewrite `run_worker_loop` with Restart Policy [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/rrt.rs`

Change signature from `impl RRTWorker<Event = E>` to `F::Worker` (needs `F: RRTFactory` to call
`F::create()` and `F::restart_policy()`):

```rust
pub fn run_worker_loop<F: RRTFactory>(
    mut worker: F::Worker,
    tx: Sender<RRTEvent<F::Event>>,
    liveness: Arc<RRTLiveness>,
    waker: Arc<Mutex<Option<F::Waker>>>,
) where
    F::Waker: RRTWaker,
    F::Event: Clone + Send + 'static,
{
    let _guard = TerminationGuard {
        liveness,
        waker: Arc::clone(&waker),
    };

    let policy = F::restart_policy();
    let mut restart_count: u8 = 0;
    let mut current_delay = policy.initial_delay;

    let tx_for_panic = tx.clone();

    // Safety: AssertUnwindSafe is sound here. After catching a panic we
    // don't touch any of the captured loop state - we only send a
    // Shutdown(Panic) notification via the pre-cloned tx_for_panic.
    let result = catch_unwind(AssertUnwindSafe(|| {
        loop {
            match worker.poll_once(&tx) {
                Continuation::Continue => {}
                Continuation::Stop => break,
                Continuation::Restart => {
                    // Inner retry loop with backoff...
                    // (see actual implementation in rrt.rs)
                }
            }
        }
    }));

    if result.is_err() {
        drop(tx_for_panic.send(RRTEvent::Shutdown(ShutdownReason::Panic)));
    }
    // _guard dropped here: clears waker, marks terminated.
}
```

### Step 1.2: Update `SubscriberGuard` Receiver Type [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs`

Change `receiver: Option<Receiver<E>>` to `receiver: Option<Receiver<RRTEvent<E>>>`. The `Drop` impl
is unchanged (it drops the receiver and wakes the thread).

## Step 2: Update Worker Implementation [COMPLETE]

### Step 2.0: Update `MioPollWorker::poll_once` [COMPLETE]

File: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`

Change signature to `&Sender<RRTEvent<Self::Event>>`.

Change the fatal poll error (non-EINTR) from `Stop` to `Restart`:

```rust
// Before:
drop(tx.send(PollerEvent::Stdin(StdinEvent::Error)));
return Continuation::Stop;

// After:
drop(tx.send(RRTEvent::Worker(PollerEvent::Stdin(StdinEvent::Error))));
return Continuation::Restart;
```

### Step 2.1: Update Handler Functions [COMPLETE]

All handler functions receive `&Sender<PollerEvent>` and need to change to
`&Sender<RRTEvent<PollerEvent>>`. All `tx.send(event)` calls wrap in `RRTEvent::Worker(event)`.

Files and specific send sites:

**`handler_stdin.rs`**:

- Line 54: `tx.send(PollerEvent::Stdin(StdinEvent::Eof))` - wrap in `RRTEvent::Worker(...)`
- Line 78: `tx.send(PollerEvent::Stdin(StdinEvent::Error))` - wrap in `RRTEvent::Worker(...)`
- Line 110: `tx.send(PollerEvent::Stdin(StdinEvent::Input(...)))` - wrap in `RRTEvent::Worker(...)`

**`handler_signals.rs`**:

- Line 53: `tx.send(PollerEvent::Signal(SignalEvent::Resize(...)))` - wrap in
  `RRTEvent::Worker(...)`

**`handler_receiver_drop.rs`**:

- No `tx.send()` calls to change. Only checks `tx.receiver_count()`.

**`dispatcher.rs`**:

- Update `dispatch_with_tx` parameter type.

### Step 2.2: Update `MioPollWorkerFactory::restart_policy` [COMPLETE]

File: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`

Add explicit `restart_policy()` override to `MioPollWorkerFactory` (or rely on the default - decide
during implementation based on whether the defaults are appropriate for terminal input polling).

## Step 3: Update Subscriber Side [COMPLETE]

### Step 3.0: Update `InputSubscriberGuard` Type Alias [COMPLETE]

File: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs`

The type alias `InputSubscriberGuard = SubscriberGuard<MioPollWaker, PollerEvent>` should still work
because `SubscriberGuard<W, E>` now internally holds `Receiver<RRTEvent<E>>`. The `E` parameter
stays as `PollerEvent`.

Verify this compiles correctly.

### Step 3.1: Update Subscriber Event Loop [COMPLETE]

File: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`

Update wherever `recv().await` is called to handle `RRTEvent`:

```rust
match rx.recv().await {
    Ok(RRTEvent::Worker(poller_event)) => {
        // Handle normal input event.
    }
    Ok(RRTEvent::Shutdown(reason)) => {
        // Thread gave up. Take corrective action.
    }
    Err(RecvError::Lagged(n)) => { /* missed events */ }
    Err(RecvError::Closed) => { break; }
}
```

## Step 4: Documentation [COMPLETE]

### Step 4.0: Update Module Docs [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/mod.rs`

Updated module-level documentation:

- Design Principles: Updated loop description from `while poll_once() == Continue {}` to three-variant `Continuation` match
- DI table: Added `RestartPolicy` row
- Type Hierarchy Diagram: Updated `broadcast_tx` to `Sender<RRTEvent<F::Event>>` and receiver to `Receiver<RRTEvent<F::Event>>`
- Module Contents: Added `rrt_event` and `rrt_restart_policy` entries
- New "Self-healing via restart" section under "How It Works" describing the full restart lifecycle
- New "Self-Healing Restart Details" section with `RestartPolicy` backoff sequence diagram, two-tier event model table, and subscriber event loop pattern
- Added all missing link definitions (`RRTEvent`, `RestartPolicy`, `Continuation` variants, `ShutdownReason`, etc.)
- Updated "What's in a name?" Resilient row to mention `RestartPolicy`

### Step 4.1: Update DI Trait Docs [COMPLETE]

File: `tui/src/core/resilient_reactor_thread/rrt_di_traits.rs`

- `RRTFactory` trait: Added paragraph about `restart_policy()` and `create()` being called during restart, with cross-link to mod.rs
- `RRTFactory::create()`: Added note about being called during self-healing restart
- `RRTFactory::restart_policy()`: Added "See also" link to mod.rs restart details
- `RRTWorker` trait: Added `Continuation::Restart` to the trait-level description with cross-link

File: `tui/src/core/resilient_reactor_thread/rrt.rs`

- `RRT` struct Thread Lifecycle: Added "Restarting" state with cross-link
- `run_worker_loop()`: Added "See also" link to mod.rs restart details

Files: `rrt_event.rs`, `rrt_restart_policy.rs`

- Both module docs and struct docs now cross-link to mod.rs restart details

### Step 4.2: Update Input Device Docs [COMPLETE]

File: `input_device_public_api.rs`

- `next()` method: Added documentation about `RRTEvent` two-tier model (Worker/Shutdown) with intra-doc links

## Step 5: Verification [COMPLETE]

### Step 5.0: Run Full Checks [COMPLETE]

```bash
./check.fish --full
```

Typecheck, build, clippy, tests, doctests, docs.

### Step 5.1: Review Existing Tests [COMPLETE]

Existing PTY integration tests use the public API and should still work, but the event matching will
need to handle `RRTEvent::Worker(...)` wrapping.

Check if any test directly matches on `PollerEvent` from the receiver - those will need updating.

## Files Changed (estimated)

| File                                                 | Change Type                                                               |
| :--------------------------------------------------- | :------------------------------------------------------------------------ |
| `continuation.rs` (or wherever `Continuation` lives) | Add `Restart` variant                                                    |
| `rrt_event.rs`                                       | **New** - `RRTEvent<E>`, `ShutdownReason`                                 |
| `rrt_restart_policy.rs`                              | **New** - `RestartPolicy` struct                                          |
| `rrt_di_traits.rs`                                   | `poll_once` signature, `restart_policy()` method on `RRTFactory`          |
| `rrt.rs`                                             | `broadcast_tx` type, `run_worker_loop` rewrite, `subscribe()` return type |
| `rrt_subscriber_guard.rs`                            | `Receiver<E>` -> `Receiver<RRTEvent<E>>`                                  |
| `mod.rs`                                             | Module declarations, re-exports, doc updates                              |
| `mio_poll_worker.rs`                                 | `poll_once` signature, fatal error -> `Restart`, wrap sends              |
| `handler_stdin.rs`                                   | Sender type, wrap sends in `RRTEvent::Worker(...)`                        |
| `handler_signals.rs`                                 | Sender type, wrap sends in `RRTEvent::Worker(...)`                        |
| `handler_receiver_drop.rs`                           | Sender type (no send changes)                                             |
| `dispatcher.rs`                                      | Sender type                                                               |
| `input_device_impl.rs`                               | Verify type alias, doc updates                                            |
| `input_device_public_api.rs`                         | Handle `RRTEvent` in recv loop, doc updates                               |

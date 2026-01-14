# Resilient Reactor Thread (RRT) Pattern

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Origin Story](#origin-story)
  - [Core Invariants](#core-invariants)
  - [Goals](#goals)
  - [Architecture Overview](#architecture-overview)
    - [Component Relationships](#component-relationships)
    - [The Chicken-Egg Problem & Solution](#the-chicken-egg-problem--solution)
    - [Cleanup Chain (No Leaks!)](#cleanup-chain-no-leaks)
    - [Understanding `'static` Bounds](#understanding-static-bounds)
  - [Module Structure](#module-structure)
    - [New Generic Module](#new-generic-module)
    - [Refactored mio_poller Module](#refactored-mio_poller-module)
  - [Design Decisions](#design-decisions)
    - [Generics vs `Box<dyn ThreadWaker>`](#generics-vs-boxdyn-threadwaker)
    - [`poll_once() → Continuation` vs `run()`](#poll_once-%E2%86%92-continuation-vs-run)
    - [Error Handling](#error-handling)
- [Implementation Plan](#implementation-plan)
  - [Phase 1: Create Generic RRT Module [COMPLETE]](#phase-1-create-generic-rrt-module-complete)
    - [Step 1.0: Create module structure [COMPLETE]](#step-10-create-module-structure-complete)
    - [Step 1.1: Implement ThreadWaker trait [COMPLETE]](#step-11-implement-threadwaker-trait-complete)
    - [Step 1.2: Implement ThreadWorker and ThreadWorkerFactory traits [COMPLETE]](#step-12-implement-threadworker-and-threadworkerfactory-traits-complete)
    - [Step 1.3: Implement ThreadLiveness [COMPLETE]](#step-13-implement-threadliveness-complete)
    - [Step 1.4: Implement ThreadState [COMPLETE]](#step-14-implement-threadstate-complete)
    - [Step 1.5: Implement SubscriberGuard [COMPLETE]](#step-15-implement-subscriberguard-complete)
    - [Step 1.6: Implement ThreadSafeGlobalState [COMPLETE]](#step-16-implement-threadsafeglobalstate-complete)
  - [Phase 2: Refactor mio_poller to Use RRT [COMPLETE]](#phase-2-refactor-mio_poller-to-use-rrt-complete)
    - [Step 2.0: Create MioPollWaker [COMPLETE]](#step-20-create-miopollwaker-complete)
    - [Step 2.1: Create MioPollWorker and Factory [COMPLETE]](#step-21-create-miopollworker-and-factory-complete)
    - [Step 2.2: Rename source files [DEFERRED]](#step-22-rename-source-files-deferred)
    - [Step 2.3: Update input_device_impl.rs [DEFERRED]](#step-23-update-input_device_implrs-deferred)
    - [Step 2.4: Update all imports and doc links [COMPLETE]](#step-24-update-all-imports-and-doc-links-complete)
  - [Phase 3: Testing & Documentation [COMPLETE]](#phase-3-testing--documentation-complete)
    - [Step 3.0: Verify existing tests pass [COMPLETE]](#step-30-verify-existing-tests-pass-complete)
    - [Step 3.1: Add unit tests for generic module [DEFERRED]](#step-31-add-unit-tests-for-generic-module-deferred)
    - [Step 3.2: Write comprehensive module documentation [COMPLETE]](#step-32-write-comprehensive-module-documentation-complete)
    - [Step 3.3: Run full quality checks [COMPLETE]](#step-33-run-full-quality-checks-complete)
  - [Phase 4: Future Preparation (Optional) [PENDING]](#phase-4-future-preparation-optional-pending)
    - [Step 4.0: Document extension points [PENDING]](#step-40-document-extension-points-pending)
    - [Step 4.1: Create example implementations [PENDING]](#step-41-create-example-implementations-pending)
  - [Success Criteria](#success-criteria)
  - [Estimated Effort](#estimated-effort)
  - [References](#references)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Origin Story

This pattern emerged from the `DirectToAnsiInputDevice` and `mio_poller` implementation, where we
needed to manage a dedicated thread for blocking I/O (stdin, signals) while providing a clean async
interface to consumers. The architecture proved to be highly generalizable.

```text
InputDevice
    │
    └── THREAD-SAFE GLOBAL STATE - static global
        static Mutex<Option<T>>
              │
              ▼
        ThreadState  ←── created/destroyed/reused
              │
              ▼
        WorkerThread
```

**Key Insight**: "Resilient Reactor Thread Pattern → can be reused in many other places, e.g.,
servers inside the TUI process to manage network services while having resilience in thread
lifecycle changes tied to OS resources / sys calls (fallible), network failures, etc."

## Core Invariants

The RRT pattern captures three fundamental invariants:

1. **Thread-safe global state lifecycle** — Static global with lazy initialization, thread-safe
   access via `Mutex<Option<Arc<ThreadState>>>`

2. **State machine** — Created/destroyed/reused with generation tracking to detect thread restarts
   vs thread reuse

3. **Contract preservation** — Async consumers never see broken promises; the broadcast channel
   decouples producers from consumers

## Goals

1. **Extract reusable infrastructure** from `mio_poller` into a generic `resilient_reactor_thread`
   module
2. **Preserve all existing behavior** — No regression in `DirectToAnsiInputDevice`
3. **Enable future use cases** — Chi remote control (mDNS discovery, TLS connections), other network
   services
4. **Maintain type safety** — Use generics to make illegal states unrepresentable
5. **Document the pattern** — Comprehensive rustdoc explaining the architecture

## Architecture Overview

### Component Relationships

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    RESILIENT REACTOR THREAD (Generic)                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  trait ThreadWaker: Send + Sync + 'static                             │  │
│  │  ───────────────────────────────────────────────────────────────────  │  │
│  │  fn wake(&self) → io::Result<()>                                      │  │
│  │                                                                       │  │
│  │  Implementations:                                                     │  │
│  │    • MioWaker (wraps mio::Waker)                                      │  │
│  │    • PipeWaker (write to pipe to interrupt select/poll)               │  │
│  │    • SocketWaker (connect-to-self pattern for TCP accept)             │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  trait ThreadWorkerFactory                                            │  │
│  │  ───────────────────────────────────────────────────────────────────  │  │
│  │  type Event: Clone + Send                                             │  │
│  │  type Worker: ThreadWorker<Event = Self::Event>                       │  │
│  │  type Waker: ThreadWaker                                              │  │
│  │                                                                       │  │
│  │  fn setup() → Result<(Self::Worker, Self::Waker), SetupError>         │  │
│  │         ↑                   ↑            ↑                            │  │
│  │         │                   │            └── Goes to thread-safe      │  │
│  │         │                   │                global state             │  │
│  │         │                   └────────────── Moves to thread           │  │
│  │         └────────────────────────────────── Solves chicken-egg!       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  trait ThreadWorker                                                   │  │
│  │  ───────────────────────────────────────────────────────────────────  │  │
│  │  type Event: Clone + Send                                             │  │
│  │                                                                       │  │
│  │  fn poll_once(&mut self, tx: &Sender<Self::Event>) → Continuation     │  │
│  │                                                                       │  │
│  │  // Called in loop until Continuation::Stop                           │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  ThreadState<W: ThreadWaker, E: Clone + Send>                         │  │
│  │  ───────────────────────────────────────────────────────────────────  │  │
│  │  • broadcast_tx: Sender<E>                                            │  │
│  │  • liveness: ThreadLiveness                                           │  │
│  │  • waker: W                                                           │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  SubscriberGuard<W, E>                                                │  │
│  │  ───────────────────────────────────────────────────────────────────  │  │
│  │  • receiver: Option<Receiver<E>>                                      │  │
│  │  • state: Arc<ThreadState<W, E>>                                      │  │
│  │  • Drop: wake thread to check receiver_count                          │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### The Chicken-Egg Problem & Solution

The `ThreadWorkerFactory::setup()` pattern solves a fundamental coupling issue:

```text
Problem:
  Waker needs Poll's registry → but Poll goes to thread
  Thread-safe global state needs Waker → but Waker created from Poll

Solution (Two-Phase Setup):
  1. Factory::setup() creates BOTH worker and waker together
  2. Waker → thread-safe global state (for SubscriberGuard to call wake())
  3. Worker → spawned thread (owns Poll, does the work)
```

### Cleanup Chain (No Leaks!)

```text
                    ┌─────────────────────────────────────────────────────────┐
                    │              CLEANUP CHAIN                              │
                    └─────────────────────────────────────────────────────────┘

    ┌──────────────────┐         ┌──────────────────┐
    │ ThreadSafeGlobal │         │ SubscriberGuard  │
    │      State       │         │     (user)       │
    └────────┬─────────┘         └────────┬─────────┘
             │                            │
             │ Arc<ThreadState>           │ Arc<ThreadState>
             │     (ref 1)                │     (ref 2)
             └────────────┬───────────────┘
                          │
                          ▼
               ┌──────────────────────┐
               │ ThreadState<W, E>    │
               │ ┌──────────────────┐ │
               │ │ waker: W         │ │  ← Dropped when Arc refcount = 0
               │ │ broadcast_tx     │ │
               │ │ liveness         │ │
               │ └──────────────────┘ │
               └──────────────────────┘

    SCENARIO: User drops SubscriberGuard
    ───────────────────────────────────────
    1. SubscriberGuard::drop() → calls waker.wake(), refcount = 1
    2. Thread-safe global state still holds Arc → waker NOT dropped yet
    3. On next allocate() slow path → global state replaces Arc
    4. Old Arc refcount = 0 → ThreadState dropped → waker dropped ✅
```

### Understanding `'static` Bounds

The `'static` bound means **"contains no borrowed references"**, NOT "lives forever":

```rust
// These are ALL 'static types (they own their data):
String           // owns heap data
Vec<u8>          // owns heap data
mio::Waker       // owns OS handle
Arc<T>           // owns shared pointer

// A 'static type CAN be dropped! The bound just ensures self-contained data.
```

Why needed:

- `Arc<T>` requires `T: 'static` (Arc could outlive borrowed data)
- `std::thread::spawn` requires `'static` closure (thread could outlive caller)

## Module Structure

### New Generic Module

```text
tui/src/core/
└── resilient_reactor_thread/             # NEW: Generic RRT infrastructure
    ├── mod.rs                            # Public exports + module-level docs
    ├── types.rs                          # ThreadWaker, ThreadWorker, ThreadWorkerFactory traits
    ├── thread_state.rs                   # ThreadState<W, E>
    ├── subscriber_guard.rs               # SubscriberGuard<W, E>
    ├── thread_liveness.rs                # ThreadLiveness, LivenessState, ShutdownDecision
    └── thread_safe_global_state_manager.rs  # ThreadSafeGlobalState<W, E>
```

### Refactored mio_poller Module

```text
tui/src/tui/terminal_lib_backends/direct_to_ansi/input/
└── mio_poller/                           # REFACTORED: Uses resilient_reactor_thread
    ├── mod.rs                            # Re-exports, module docs
    ├── mio_poll_worker.rs                # MioPollWorker + MioPollWorkerFactory
    ├── mio_poll_waker.rs                 # MioPollWaker impl ThreadWaker
    ├── mio_poll_sources.rs               # SourceRegistry (stdin, signals)
    └── mio_poll_handlers/                # Event handlers
        ├── mod.rs
        ├── stdin.rs
        ├── signals.rs
        └── receiver_drop.rs
```

## Design Decisions

### Generics vs `Box<dyn ThreadWaker>`

**Decision: Use Generics**

```rust
// Generics (chosen)
pub struct ThreadState<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    pub waker: W,  // Zero-cost, inlined wake() calls
    // ...
}

// Type aliases for ergonomics
pub type MioThreadState = ThreadState<MioPollWaker, PollerEvent>;
pub type MioSubscriberGuard = SubscriberGuard<MioPollWaker, PollerEvent>;
```

Rationale:

- Zero-cost abstraction (no vtable lookup for `wake()`)
- Full type safety (can't mix MioPollWaker with TlsWaker)
- Type parameter "infection" is contained via type aliases

### `poll_once() → Continuation` vs `run()`

**Decision: `poll_once() → Continuation`**

```rust
pub trait ThreadWorker: Send + 'static {
    type Event: Clone + Send + 'static;

    /// Run one iteration of the work loop.
    fn poll_once(&mut self, tx: &broadcast::Sender<Self::Event>) -> Continuation;
}
```

Rationale:

- Framework controls the loop (can inject logging, metrics)
- Single responsibility (worker handles events, framework handles lifecycle)
- Testability (can unit test `poll_once` in isolation)

### Error Handling

**Decision: Use `miette::Report`**

```rust
pub trait ThreadWorkerFactory: Send + 'static {
    type SetupError: Into<miette::Report>;
    // ...
}
```

---

# Implementation Plan

## Phase 1: Create Generic RRT Module [COMPLETE]

### Step 1.0: Create module structure [COMPLETE]

Create the new `resilient_reactor_thread` module under `tui/src/core/`.

**Tasks:**

- [x] Create `tui/src/core/resilient_reactor_thread/mod.rs` with module-level documentation
- [x] Create `tui/src/core/resilient_reactor_thread/types.rs` with trait definitions
- [x] Create `tui/src/core/resilient_reactor_thread/thread_state.rs`
- [x] Create `tui/src/core/resilient_reactor_thread/subscriber_guard.rs`
- [x] Create `tui/src/core/resilient_reactor_thread/thread_liveness.rs`
- [x] Create `tui/src/core/resilient_reactor_thread/thread_safe_global_state_manager.rs`
- [x] Add `pub mod resilient_reactor_thread;` to `tui/src/core/mod.rs`

### Step 1.1: Implement ThreadWaker trait [COMPLETE]

**File:** `types.rs`

```rust
/// Waker abstraction for interrupting a blocking thread.
///
/// Each RRT implementation provides its own waker that knows how to
/// interrupt its specific blocking mechanism (mio::Poll, TCP accept, etc.).
pub trait ThreadWaker: Send + Sync + 'static {
    /// Wake the thread so it can check if it should exit.
    ///
    /// Called by [`SubscriberGuard::drop()`] to signal the thread.
    /// The thread then checks [`receiver_count()`] to decide whether to exit.
    ///
    /// [`SubscriberGuard::drop()`]: super::subscriber_guard::SubscriberGuard
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn wake(&self) -> std::io::Result<()>;
}
```

### Step 1.2: Implement ThreadWorker and ThreadWorkerFactory traits [COMPLETE]

**File:** `types.rs`

```rust
/// Factory that creates coupled worker and waker together.
///
/// Solves the chicken-egg problem where waker creation depends on
/// resources that the worker owns.
pub trait ThreadWorkerFactory: Send + 'static {
    /// Event type broadcast to subscribers.
    type Event: Clone + Send + 'static;

    /// Worker type that runs on the thread.
    type Worker: ThreadWorker<Event = Self::Event>;

    /// Waker type for interrupting the worker.
    type Waker: ThreadWaker;

    /// Error type for setup failures.
    type SetupError: Into<miette::Report>;

    /// Create worker and waker together.
    ///
    /// - Worker → moves to spawned thread
    /// - Waker → stored in thread-safe global state for SubscriberGuards
    fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError>;
}

/// Worker that runs on a dedicated thread.
///
/// Implements the actual work loop logic. Called repeatedly by the
/// framework until [`Continuation::Stop`] is returned.
pub trait ThreadWorker: Send + 'static {
    /// Event type this worker produces.
    type Event: Clone + Send + 'static;

    /// Run one iteration of the work loop.
    ///
    /// Called in a loop by the framework. Return [`Continuation::Continue`]
    /// to keep running, or [`Continuation::Stop`] to exit the thread.
    fn poll_once(
        &mut self,
        tx: &tokio::sync::broadcast::Sender<Self::Event>,
    ) -> Continuation;
}
```

### Step 1.3: Implement ThreadLiveness [COMPLETE]

**File:** `thread_liveness.rs`

Move and generalize from `mio_poller/poller_thread_state.rs`:

- `ThreadLiveness` struct
- `LivenessState` enum
- `ShutdownDecision` enum
- `THREAD_GENERATION` static counter

### Step 1.4: Implement ThreadState [COMPLETE]

**File:** `thread_state.rs`

```rust
/// Shared state between thread-safe global state and worker thread.
///
/// Centralizes thread lifecycle, event broadcasting, and waker in one place.
/// Shared via [`Arc`] between the global state and SubscriberGuards.
pub struct ThreadState<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    /// Broadcasts events to async subscribers.
    pub broadcast_tx: tokio::sync::broadcast::Sender<E>,

    /// Thread liveness and generation tracking.
    pub liveness: ThreadLiveness,

    /// Waker to signal thread for shutdown check.
    pub waker: W,
}
```

### Step 1.5: Implement SubscriberGuard [COMPLETE]

**File:** `subscriber_guard.rs`

```rust
/// RAII guard that wakes the thread on drop.
///
/// When dropped:
/// 1. Drops the broadcast receiver (decrements receiver_count)
/// 2. Calls `waker.wake()` to interrupt the thread
/// 3. Thread checks receiver_count and exits if 0
pub struct SubscriberGuard<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    /// Broadcast receiver for events. Taken on drop.
    pub receiver: Option<tokio::sync::broadcast::Receiver<E>>,

    /// Shared thread state (for waker access).
    pub state: Arc<ThreadState<W, E>>,
}

impl<W, E> Drop for SubscriberGuard<W, E>
where
    W: ThreadWaker,
    E: Clone + Send + 'static,
{
    fn drop(&mut self) {
        drop(self.receiver.take());  // Decrement receiver_count
        let _ = self.state.waker.wake();  // Signal thread to check
    }
}
```

### Step 1.6: Implement ThreadSafeGlobalState [COMPLETE]

**File:** `thread_safe_global_state_manager.rs`

```rust
/// Thread-safe global state for a Resilient Reactor Thread.
///
/// Manages the lifecycle of a dedicated worker thread with automatic
/// spawn/shutdown/reuse semantics.
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
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Allocate a subscription, spawning thread if needed.
    ///
    /// - Fast path: thread running → subscribe to existing
    /// - Slow path: thread terminated → spawn new
    pub fn allocate<F>(&self) -> Result<SubscriberGuard<W, E>, miette::Report>
    where
        F: ThreadWorkerFactory<Waker = W, Event = E>,
    {
        // ... implementation
    }

    // Testing helpers
    pub fn is_thread_running(&self) -> LivenessState { ... }
    pub fn get_receiver_count(&self) -> usize { ... }
    pub fn get_thread_generation(&self) -> u8 { ... }
}
```

---

## Phase 2: Refactor mio_poller to Use RRT [COMPLETE]

### Step 2.0: Create MioPollWaker [COMPLETE]

**File:** `mio_poll_waker.rs`

```rust
use crate::core::resilient_reactor_thread::ThreadWaker;

/// mio-specific waker implementation.
pub struct MioPollWaker(pub mio::Waker);

impl ThreadWaker for MioPollWaker {
    fn wake(&self) -> std::io::Result<()> {
        self.0.wake()
    }
}
```

### Step 2.1: Create MioPollWorker and Factory [COMPLETE]

**File:** `mio_poll_worker.rs`

Extract the current `MioPollerThread` into:

- `MioPollWorker` — implements `ThreadWorker`
- `MioPollWorkerFactory` — implements `ThreadWorkerFactory`

```rust
pub struct MioPollWorker {
    pub poll_handle: Poll,
    pub ready_events_buffer: Events,
    pub sources: SourceRegistry,
    pub stdin_unparsed_byte_buffer: [u8; STDIN_READ_BUFFER_SIZE],
    pub vt_100_input_seq_parser: StatefulInputParser,
    pub paste_collection_state: PasteCollectionState,
    // Note: NO thread_state here - that's in the generic layer
}

impl ThreadWorker for MioPollWorker {
    type Event = PollerEvent;

    fn poll_once(&mut self, tx: &broadcast::Sender<Self::Event>) -> Continuation {
        // Current start() loop body, but just one iteration
    }
}

pub struct MioPollWorkerFactory;

impl ThreadWorkerFactory for MioPollWorkerFactory {
    type Event = PollerEvent;
    type Worker = MioPollWorker;
    type Waker = MioPollWaker;
    type SetupError = MioSetupError;

    fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError> {
        let poll = Poll::new()?;
        let waker = MioPollWaker(Waker::new(poll.registry())?);
        let worker = MioPollWorker::new(poll)?;
        Ok((worker, waker))
    }
}
```

### Step 2.2: Rename source files [DEFERRED]

Decided to keep existing file structure. The new generic RRT module lives alongside the existing
mio_poller implementation. Created new files instead:

- `mio_poll_waker.rs` — MioPollWaker implementing ThreadWaker
- `mio_poll_worker.rs` — MioPollWorker and MioPollWorkerFactory

### Step 2.3: Update input_device_impl.rs [DEFERRED]

The existing global state pattern continues to work. The generic RRT infrastructure is now
available for future implementations (Chi remote control, etc.). Migration of the existing
mio_poller to fully use ThreadSafeGlobalState can be done in a future iteration if desired.

### Step 2.4: Update all imports and doc links [COMPLETE]

Added `_with_tx` variants to handlers for use with the generic ThreadWorker pattern:

- `handler_receiver_drop.rs` — `handle_receiver_drop_waker_with_tx()`
- `handler_signals.rs` — `consume_pending_signals_with_tx()`
- `handler_stdin.rs` — `consume_pending_stdin_events_with_tx()`
- `dispatcher.rs` — `dispatch_with_tx()`

---

## Phase 3: Testing & Documentation [COMPLETE]

### Step 3.0: Verify existing tests pass [COMPLETE]

All existing tests continue to pass. Ran `cargo test -p r3bl_tui` — 305 tests passed.

### Step 3.1: Add unit tests for generic module [DEFERRED]

The generic RRT module is validated through the existing mio_poller integration tests. The
`MioPollWorker` and `MioPollWorkerFactory` implementations prove the traits work correctly.
Additional unit tests with mock wakers can be added when implementing new RRT consumers
(Chi remote control, etc.).

### Step 3.2: Write comprehensive module documentation [COMPLETE]

**File:** `resilient_reactor_thread/mod.rs`

Contains:

- Pattern overview with ASCII diagrams showing thread lifecycle
- Core invariants (global state lifecycle, state machine, contract preservation)
- The chicken-egg problem and solution via `ThreadWorkerFactory::setup()`
- Race condition handling explanation
- Cleanup chain documentation (no leaks!)
- Understanding `'static` bounds
- Complete usage example
- Links to concrete implementation (`mio_poller`)

### Step 3.3: Run full quality checks [COMPLETE]

All quality checks pass:

- `cargo check -p r3bl_tui --all-targets` ✓
- `cargo doc -p r3bl_tui --no-deps` ✓ (no warnings)
- `cargo clippy -p r3bl_tui --all-targets -- -D warnings` ✓
- `cargo test -p r3bl_tui` ✓ (305 tests passed)

---

## Phase 4: Future Preparation (Optional) [PENDING]

### Step 4.0: Document extension points [PENDING]

Add documentation for how to implement RRT for:

- mDNS discovery (chi remote control)
- TLS connections (chi remote control)
- Other network services

### Step 4.1: Create example implementations [PENDING]

Stub implementations showing the pattern:

- `PipeWaker` — for self-pipe pattern
- `SocketWaker` — for TCP accept interruption

---

## Success Criteria

1. **Zero regression** — All existing tests pass
2. **Clean separation** — Generic infrastructure in `resilient_reactor_thread`, mio-specific in
   `mio_poller`
3. **Type safety** — Generics prevent mixing incompatible wakers/events
4. **Documentation** — Comprehensive rustdoc for the pattern
5. **Extensibility** — Clear path for future implementations

## Estimated Effort

- **Phase 1**: 2-3 days (create generic module)
- **Phase 2**: 2-3 days (refactor mio_poller)
- **Phase 3**: 1-2 days (testing & documentation)
- **Phase 4**: Optional, as needed

**Total**: 5-8 days

## References

- Current implementation: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/`
- Race condition documentation: `mio_poller/poller_thread_state.rs`
- Integration tests: `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/`
- Future use case: `task/pending/prd_chi.md` (Remote Control Mode)
- **Applications of RRT**: `task/pending/rrt-applications.md` — Design exploration for mDNS
  discovery, Unix/TCP IPC, clipboard sync, CRDTs, and P2P mesh architectures that will use the RRT
  infrastructure

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Fix: Thread Liveness Tracking for Mio-Poller Thread Relaunch [COMPLETE]](#fix-thread-liveness-tracking-for-mio-poller-thread-relaunch-complete)
  - [The Problem](#the-problem)
  - [Solution: Thread Liveness Tracking with `Drop` + `mio::Waker`](#solution-thread-liveness-tracking-with-drop--miowaker)
    - [Key Design Decisions](#key-design-decisions)
  - [Implementation](#implementation)
    - [1. `types.rs` - WakingReceiver](#1-typesrs---wakingreceiver)
    - [2. `global_input_resource.rs`](#2-global_input_resourcers)
    - [3. `mio_poller/poller_thread.rs`](#3-mio_pollerpoller_threadrs)
    - [4. `mio_poller/handler_receiver_drop.rs` - Waker Handler](#4-mio_pollerhandler_receiver_droprs---waker-handler)
    - [5. `mio_poller/dispatcher.rs` - Dispatch Routing](#5-mio_pollerdispatcherrs---dispatch-routing)
    - [6. `mio_poller/sources.rs` - ReceiverDropWaker Token](#6-mio_pollersourcesrs---receiverdropwaker-token)
  - [Race Condition and Correctness](#race-condition-and-correctness)
  - [Data Flow After Fix](#data-flow-after-fix)
  - [Why `Drop` Instead of Manual `store(false)`?](#why-drop-instead-of-manual-storefalse)
  - [Alternatives Considered But Rejected](#alternatives-considered-but-rejected)
    - [Alternative 1: Reset `INPUT_RESOURCE` to `None` on thread exit](#alternative-1-reset-input_resource-to-none-on-thread-exit)
    - [Alternative 2: Use `tx.receiver_count() == 0` as liveness check](#alternative-2-use-txreceiver_count--0-as-liveness-check)
    - [Alternative 3: Store `JoinHandle` and check `is_finished()`](#alternative-3-store-joinhandle-and-check-is_finished)
    - [Alternative 4: Poll timeout instead of mio::Waker](#alternative-4-poll-timeout-instead-of-miowaker)
    - [Comparison Table](#comparison-table)
  - [Integration Tests](#integration-tests)
    - [1. Lifecycle Test (`pty_mio_poller_thread_lifecycle_test.rs`)](#1-lifecycle-test-pty_mio_poller_thread_lifecycle_testrs)
    - [2. Reuse Test (`pty_mio_poller_thread_reuse_test.rs`)](#2-reuse-test-pty_mio_poller_thread_reuse_testrs)
    - [Test Implementation](#test-implementation)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Fix: Thread Liveness Tracking for Mio-Poller Thread Relaunch [COMPLETE]

## The Problem

When the mio-poller thread exits (e.g., all receivers dropped, EOF, or error), the global
`INPUT_RESOURCE` remains `Some(tx)`. Subsequent calls to `subscribe_to_input_events()` return a
**dead receiver**:

```text
Timeline:
1. First TUI app starts → subscribe_to_input_events() → INPUT_RESOURCE = Some(tx), thread spawned
2. TUI app exits → receiver dropped → thread detects 0 receivers → thread exits
3. INPUT_RESOURCE still = Some(tx)  ← Problem!
4. Second TUI app starts → subscribe_to_input_events() → sees Some(tx) → skips init → returns dead rx
5. Second app's rx.recv() hangs forever (no sender)
```

**Root cause**: The `is_none()` check in `subscribe_to_input_events()` only runs once. After
initialization, `INPUT_RESOURCE` is always `Some(tx)`, even if the thread is dead.

## Solution: Thread Liveness Tracking with `Drop` + `mio::Waker`

Two mechanisms work together:

1. **`Arc<AtomicBool>` liveness flag** - thread sets to `false` via `Drop` when exiting
2. **`mio::Waker`** - immediately wakes thread when receiver drops (no polling delay)

### Key Design Decisions

1. **`Arc<AtomicBool>` for liveness tracking** - explicit, low overhead, panic-safe
2. **`Drop` impl sets the flag** - guarantees flag is set even if `start()` panics
3. **`mio::Waker` for immediate exit** - thread exits immediately when last receiver drops
4. **`WakingReceiver` wrapper** - `Drop` impl calls `waker.wake()` to interrupt poll
5. **`InputResourceState` is `Clone`** - three `Arc`s (tx, thread_alive, waker), cloning is cheap
6. **Poll created before spawn** - `initialize_input_resource()` creates Poll so Waker can be shared

## Implementation

### 1. `types.rs` - WakingReceiver

**`WakingReceiver`** wrapper that wakes thread on drop:

```rust
pub struct WakingReceiver {
    inner: Option<InputEventReceiver>,
    waker: Arc<mio::Waker>,
}

impl Drop for WakingReceiver {
    fn drop(&mut self) {
        // Drop inner first so receiver_count decrements.
        self.inner.take();
        // Wake thread to check if it should exit.
        if let Err(err) = self.waker.wake() {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "WakingReceiver::drop: failed to wake mio-poller thread",
                    error = ?err
                );
            });
        }
    }
}
```

### 2. `global_input_resource.rs`

**`InputResourceState` struct** with `Clone` (three `Arc`s):

```rust
#[derive(Clone)]
pub struct InputResourceState {
    pub tx: InputEventSender,
    pub thread_alive: Arc<AtomicBool>,
    pub waker: Arc<mio::Waker>,
}
```

**`subscribe_to_input_events()`** returns `WakingReceiver`:

```rust
pub fn subscribe_to_input_events() -> WakingReceiver {
    let mut guard = INPUT_RESOURCE.lock().expect("...");

    let needs_init = match guard.as_ref() {
        None => true,
        Some(state) => !state.thread_alive.load(Ordering::SeqCst),
    };

    if needs_init {
        initialize_input_resource(&mut guard);
    }

    let state = guard.as_ref().expect("...");
    WakingReceiver::new(state.tx.subscribe(), Arc::clone(&state.waker))
}
```

**`initialize_input_resource()`** creates Poll and Waker before spawning:

```rust
fn initialize_input_resource(guard: &mut Option<InputResourceState>) {
    // Poll created HERE so we can get the registry for the Waker.
    let poll = Poll::new().expect("...");
    let waker = Waker::new(poll.registry(), SourceKindReady::ReceiverDropWaker.to_token()).expect("...");

    let state = InputResourceState {
        tx: tokio::sync::broadcast::channel(CHANNEL_CAPACITY).0,
        thread_alive: Arc::new(AtomicBool::new(true)),
        waker: Arc::new(waker),
    };

    MioPollerThread::spawn_thread(poll, state.clone());
    guard.replace(state);
}
```

### 3. `mio_poller/poller_thread.rs`

**`MioPollerThread` struct** stores the liveness flag:

```rust
/// Thread liveness flag shared with [`INPUT_RESOURCE`].
///
/// Set to `false` by the [`Drop`] impl when this struct is dropped. This allows
/// [`subscribe_to_input_events()`] to detect a dead thread and spawn a new one.
///
/// Using [`Drop`] ensures panic-safety: even if [`start()`] panics, the flag is set
/// during stack unwinding, so [`subscribe_to_input_events()`] will correctly detect
/// the dead thread and reinitialize.
pub thread_alive: Arc<AtomicBool>,
```

**`Drop` impl** for panic-safe liveness tracking:

```rust
impl Drop for MioPollerThread {
    /// Sets the liveness flag to `false` when the struct is dropped.
    ///
    /// This is panic-safe: even if [`start()`] panics, the flag is set during stack
    /// unwinding, enabling [`subscribe_to_input_events()`] to detect the dead thread.
    fn drop(&mut self) {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: dropping, setting thread_alive = false");
        });
        self.thread_alive.store(false, Ordering::SeqCst);
    }
}
```

**`spawn_thread(poll, state)`** receives Poll (with Waker pre-registered) and state:

```rust
pub fn spawn_thread(poll: Poll, state: InputResourceState) {
    let _unused = std::thread::Builder::new()
        .name("mio-poller".into())
        .spawn(move || {
            let mut mio_poller = Self::setup(poll, state);
            mio_poller.start();
            // Drop impl sets thread_alive = false (panic-safe).
        })
        .expect("...");
}
```

**`setup(poll, state)`** uses provided Poll (Waker already registered):

```rust
pub fn setup(poll: Poll, state: InputResourceState) -> Self {
    let InputResourceState {
        tx: tx_parsed_input_events,
        thread_alive,
        waker: _, // Already registered with Poll
    } = state;
    let poll_handle = poll;
    // ... register stdin and signals ...
}
```

### 4. `mio_poller/handler_receiver_drop.rs` - Waker Handler

**Handler function** follows the same pattern as `handler_stdin.rs` and `handler_signals.rs`:

```rust
//! Handler for [`ReceiverDropWaker`] events (thread exit check).

use super::poller_thread::MioPollerThread;
use crate::tui::{DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                 terminal_lib_backends::direct_to_ansi::input::types::ThreadLoopContinuation};

/// Handles [`ReceiverDropWaker`] event - check if thread should exit.
///
/// Called when [`WakingReceiver::drop()`] wakes the thread via [`mio::Waker::wake()`].
/// Checks if all receivers have been dropped (i.e., `receiver_count() == 0`).
pub fn handle_receiver_drop_waker(poller: &mut MioPollerThread) -> ThreadLoopContinuation {
    let receiver_count = poller.tx_parsed_input_events.receiver_count();
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "mio-poller-thread: receiver drop waker triggered",
            receiver_count = receiver_count
        );
    });

    if receiver_count == 0 {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: no receivers left, exiting");
        });
        ThreadLoopContinuation::Return
    } else {
        ThreadLoopContinuation::Continue
    }
}
```

**Naming convention**: Uses `handle_` prefix rather than `consume_` because there's no data to
consume - the waker is just a notification. Compare with `consume_stdin_input()` and
`consume_pending_signals()` which actually read/drain data.

### 5. `mio_poller/dispatcher.rs` - Dispatch Routing

**Dispatcher** routes tokens to appropriate handlers:

```rust
pub fn dispatch(
    source_kind: SourceKindReady,
    poller: &mut MioPollerThread,
    token: Token,
) -> ThreadLoopContinuation {
    match source_kind {
        SourceKindReady::Stdin => consume_stdin_input(poller),
        SourceKindReady::Signals => consume_pending_signals(poller),
        SourceKindReady::ReceiverDropWaker => handle_receiver_drop_waker(poller),
        SourceKindReady::Unknown => { /* warn and continue */ }
    }
}
```

### 6. `mio_poller/sources.rs` - ReceiverDropWaker Token

**`SourceKindReady`** includes ReceiverDropWaker variant:

```rust
pub enum SourceKindReady {
    Stdin,              // Token(0)
    Signals,            // Token(1)
    ReceiverDropWaker,  // Token(2) - wakeup from WakingReceiver::drop()
    Unknown,
}
```

## Race Condition and Correctness

The combination of liveness flag + `mio::Waker` handles a subtle race condition:

**The race**: What if device A drops, waker fires, thread checks `receiver_count()` = 0, but
_before_ the thread exits, device B subscribes?

**Why it's handled correctly**:

1. Thread checks `receiver_count()` while still alive (`thread_alive = true`)
2. If device B subscribes before thread checks: `receiver_count() = 1`, thread continues
3. If device B subscribes after thread checks but before exit: thread is still alive
   (`thread_alive = true`), so device B reuses the existing thread
4. If device B subscribes after thread exits: `thread_alive = false`, new thread spawns

The key insight: as long as `thread_alive = true`, the thread can handle new subscribers. The waker
just accelerates the "check if I should exit" path.

## Data Flow After Fix

```text
subscribe_to_input_events()
         │
         ▼
┌────────────────────────┐
│ INPUT_RESOURCE.lock()  │
└────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ state.thread_alive.load(SeqCst)?  │
│   true  → return state.tx.subscribe()
│   false → reinitialize (new state, new thread, flag = true)
└────────────────────────────────────┘
```

## Why `Drop` Instead of Manual `store(false)`?

The original plan had the closure manually call `thread_alive.store(false, ...)` after `start()`
returns. This has a **panic-safety bug**: if `start()` panics, the flag is never set to `false`, and
`subscribe_to_input_events()` will think the thread is still alive.

Using `Drop`:

- **Guaranteed execution**: `Drop::drop()` runs during stack unwinding
- **RAII principle**: The struct owns responsibility for setting the flag
- **No dead code**: The field is actually read by `Drop`

## Alternatives Considered But Rejected

### Alternative 1: Reset `INPUT_RESOURCE` to `None` on thread exit

**Rejected because**:

1. **Coupling**: Thread would need to know about `INPUT_RESOURCE` global static
2. **Mutex contention at exit**: Lock contention during thread teardown
3. **Panic safety**: Mutex poisoning cascades to all future subscribe calls

### Alternative 2: Use `tx.receiver_count() == 0` as liveness check

**Rejected because**:

1. **False positive**: `receiver_count() == 0` is normal before `subscribe()`
2. **Doesn't detect dead thread**: Only detects dead receivers, not thread exit
3. **Race condition**: Count can change between check and subscribe

### Alternative 3: Store `JoinHandle` and check `is_finished()`

**Rejected because**:

1. **What to do with old handle?** `join()` blocks; dropping loses panic info
2. **Ownership complexity**: `JoinHandle` can't be cloned
3. **No benefit**: Essentially same as `Arc<AtomicBool>` with worse ergonomics

### Alternative 4: Poll timeout instead of mio::Waker

Use a fixed poll timeout (e.g., 100ms) to periodically check `receiver_count()`:

```rust
poll.poll(&mut events, Some(Duration::from_millis(100)))?;
if self.tx.receiver_count() == 0 {
    return; // Exit thread
}
```

**Rejected because**:

1. **Delayed exit**: Thread takes up to 100ms to notice last receiver dropped
2. **Unnecessary wakeups**: Thread wakes every 100ms even when no input
3. **CPU/power waste**: Bad for laptops and mobile devices
4. **Latency tradeoff**: Shorter timeout = faster exit but more CPU; longer = slower exit

`mio::Waker` is zero-cost when not triggered and instant when receivers drop.

### Comparison Table

| Criterion                   | Reset to None | receiver_count | JoinHandle | Arc<AtomicBool> |
| :-------------------------- | :------------ | :------------- | :--------- | :-------------- |
| No mutex contention at exit | ❌            | ✅             | ✅         | ✅              |
| Panic-safe (with Drop)      | ❌            | ✅             | ⚠️         | ✅              |
| Detects dead thread         | ✅            | ❌             | ✅         | ✅              |
| Simple ownership model      | ✅            | ✅             | ❌         | ✅              |
| No blocking operations      | ✅            | ✅             | ⚠️         | ✅              |

## Integration Tests

Two PTY-based integration tests verify the implementation works correctly:

### 1. Lifecycle Test (`pty_mio_poller_thread_lifecycle_test.rs`)

Verifies thread relaunch works:

```text
📍 Step 1: Initial state: thread_alive=false, receiver_count=0
📍 Step 2: Create device A → thread_alive=true, receiver_count=1
📍 Step 3: Drop device A → thread exits in 1ms! ← mio::Waker works!
           After drop: thread_alive=false, receiver_count=0
📍 Step 4: Create device B → NEW thread spawns, thread_alive=true
```

**Key result**: Thread exits in **1ms** after receiver drop (verified by polling loop).

### 2. Reuse Test (`pty_mio_poller_thread_reuse_test.rs`)

Verifies race condition is handled correctly:

```text
📍 Step 1: Create device A → thread_alive=true, receiver_count=1
📍 Step 2: Drop A and IMMEDIATELY create B (race condition scenario)
           → Thread still alive, receiver_count=1 (thread reused!)
```

**Key result**: When device B subscribes before thread exits, the existing thread is reused rather
than spawning a new one.

### Test Implementation

Both tests use the PTY (pseudo-terminal) pattern:

1. **Controller**: Spawns a "controlled" process with `R3BL_PTY_TEST_CONTROLLED=1`
2. **Controlled**: Runs inside PTY, uses real terminal I/O
3. **Communication**: Controller sends keystrokes via PTY master fd

This allows testing real terminal input handling in an automated test environment.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Simplify Thread Lifecycle Orchestration](#simplify-thread-lifecycle-orchestration)
  - [Status: Partially Implemented](#status-partially-implemented)
  - [Context](#context)
  - [Current Architecture: Two Mechanisms](#current-architecture-two-mechanisms)
  - [Race Condition Diagram](#race-condition-diagram)
  - [Why This Complexity Exists](#why-this-complexity-exists)
  - [Implemented: API Ergonomics](#implemented-api-ergonomics)
    - [Singleton Device Pattern](#singleton-device-pattern)
    - [Type Rename](#type-rename)
    - [Multiple Subscribers via `subscribe()`](#multiple-subscribers-via-subscribe)
  - [Future Work: Thread Lifecycle Simplification](#future-work-thread-lifecycle-simplification)
    - [Option 1: Keep Thread Alive Forever](#option-1-keep-thread-alive-forever)
    - [Option 2: Polling with Timeout (Eliminate Waker)](#option-2-polling-with-timeout-eliminate-waker)
    - [Option 3: Channel-Based Shutdown (Eliminate Liveness Flag)](#option-3-channel-based-shutdown-eliminate-liveness-flag)
  - [Integration Tests That Validate Current Behavior](#integration-tests-that-validate-current-behavior)
  - [Documentation Completed](#documentation-completed)
    - [API Ergonomics Changes (commit `f78e1a0`)](#api-ergonomics-changes-commit-f78e1a0)
  - [Next Steps](#next-steps)
  - [Related Files](#related-files)
    - [Implementation](#implementation)
    - [Integration Tests](#integration-tests)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Simplify Thread Lifecycle Orchestration

## Status: Partially Implemented

- **Implemented**: API ergonomics (singleton pattern, `subscribe()` method)
- **Future Work**: Thread lifecycle simplification options

## Context

After documenting the thread lifecycle in detail, we identified that the current design has
significant complexity due to two separate signaling mechanisms that must coordinate.

## Current Architecture: Two Mechanisms

```
Mechanism 1: Waker (tells thread to check)
──────────────────────────────────────────
InputDeviceResourceHandle::drop()
  → receiver_count -= 1
  → mio_poller_thread_waker.wake() ► thread wakes from poll()

Mechanism 2: Liveness flag (tells subscribers thread is dead)
─────────────────────────────────────────────────────────────
MioPollerThread::drop()
  → thread_alive = false
  → subscribe_to_input_events() checks this to spawn new thread
```

## Race Condition Diagram

When a device drops and a new one is created quickly, there's a race window where the new device can
"save" the thread from exiting. Note: Creating a device via `new()` internally subscribes to the
broadcast channel (receiver_count += 1).

```
Main Thread                                mio_poller Thread
───────────                                ─────────────────
1. Old device drops                        │ blocked in poll()
   a. DirectToAnsiInputDevice::drop()      │
      - single_device_gate::clear()        │
   b. Rust drops field `resource_handle`   │
      (InputDeviceResourceHandle::drop)    │
      - maybe_stdin_rx.take()              │
        (receiver_count → 0)               │
      - mio_poller_thread_waker.wake() ────► kernel sees eventfd readable
                                           │
   ┌───────────────────────────────────────┼───────────────────────────────┐
   │        RACE WINDOW                    │ kernel schedules thread       │
   │                                       │ thread returns from syscall   │
   │  New device created via new()         │ ...                           │
   │  → allocate_or_get_existing_thread()  │                               │
   │  → receiver_count = 1                 │ finally checks receiver_count │
   └───────────────────────────────────────┼───────────────────────────────┘
                                           ▼
                                           if receiver_count == 0 { exit }
                                           else { continue! }  ← thread reused
```

The delay exists because:

1. **Kernel scheduling** - Thread is blocked in a syscall; kernel must schedule it
2. **Context switch** - CPU saves/restores thread state
3. **Syscall return** - `poll()` must return through kernel → userspace
4. **Code execution** - Thread iterates events, dispatches, _then_ checks count

## Why This Complexity Exists

The fundamental tension: we _want_ the thread to block efficiently (via `poll()`), but that same
blocking makes it hard to tell the thread "time to check if you should exit." The waker solves the
blocking problem, but introduces the race condition.

## Implemented: API Ergonomics

These changes simplify the public API while the underlying thread lifecycle complexity remains as
documented above.

### Singleton Device Pattern

`DirectToAnsiInputDevice::new()` now enforces singleton semantics:

- **Only ONE device** can exist at a time (panics if called twice)
- Uses `single_device_gate` module with `AtomicBool` for enforcement
- `Drop` clears the gate, allowing recreation after the device is dropped

**Rationale**: There's only one `stdin`, so having multiple "devices" is semantically incorrect.

```rust
// CORRECT: One device, use subscribe() for additional receivers
let device = DirectToAnsiInputDevice::new();
let subscriber = device.subscribe();

// WRONG: Panics!
let device1 = DirectToAnsiInputDevice::new();
let device2 = DirectToAnsiInputDevice::new(); // panic!
```

### Type Rename

`InputEventReceiverHandle` → `InputDeviceResourceHandle`

The new name better reflects its role as a resource handle containing:

- The broadcast channel receiver (`maybe_stdin_rx`)
- The mio waker for thread lifecycle signaling (`mio_poller_thread_waker`)

### Multiple Subscribers via `subscribe()`

For additional event consumers (logging, debugging, multiple concurrent readers):

```rust
let device = DirectToAnsiInputDevice::new();
let subscriber = device.subscribe();  // Returns InputDeviceResourceHandle

// Both receive the same events independently
```

Each subscriber:

- Independently receives all input events via broadcast channel
- When dropped, notifies the mio_poller thread to check if it should exit

---

## Future Work: Thread Lifecycle Simplification

The options below are still under discussion. They address the underlying thread lifecycle
complexity (two-mechanism coordination) rather than the public API.

### Option 1: Keep Thread Alive Forever

**Question**: Do we even need the thread to exit? If apps are sequential, could one long-lived
thread serve all apps forever?

**Pros**:

- Eliminates all exit/restart complexity
- No race conditions
- No waker needed for shutdown signaling

**Cons**:

- Thread lives for entire process lifetime
- May hold resources unnecessarily
- Need to handle the case where thread dies unexpectedly (panic, etc.)

### Option 2: Polling with Timeout (Eliminate Waker)

**Question**: What if `poll()` had a timeout, and the thread periodically checked `receiver_count`?

```rust
loop {
    poll.poll(&mut events, Some(Duration::from_millis(100)))?;
    if receiver_count() == 0 {
        break; // Exit gracefully
    }
    // ... handle events ...
}
```

**Pros**:

- Eliminates waker complexity
- Simpler mental model
- No race condition (thread checks on its own schedule)

**Cons**:

- Less efficient (wakes up even when nothing happened)
- 100ms latency on shutdown (or whatever timeout chosen)
- Tradeoff between efficiency and shutdown latency

### Option 3: Channel-Based Shutdown (Eliminate Liveness Flag)

**Question**: What if `receiver_count > 0` was the only signal to keep running, and
`subscribe_to_input_events()` always just tried to send a "ping" to see if the channel works?

**Pros**:

- Single source of truth (receiver_count)
- No separate liveness flag to maintain

**Cons**:

- Still need waker to unblock poll()
- May not actually reduce complexity

## Integration Tests That Validate Current Behavior

- [`pty_mio_poller_singleton_test`] - Singleton semantics (panics on second `new()`)
- [`pty_mio_poller_subscribe_test`] - Multiple subscriber broadcast semantics
- [`pty_mio_poller_thread_lifecycle_test`] - Full cycle: spawn → exit → respawn (with delay)
- [`pty_mio_poller_thread_reuse_test`] - Race condition: fast subscriber reuses thread

[`pty_mio_poller_singleton_test`]:
  crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_singleton_test
[`pty_mio_poller_subscribe_test`]:
  crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_subscribe_test
[`pty_mio_poller_thread_lifecycle_test`]:
  crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_thread_lifecycle_test
[`pty_mio_poller_thread_reuse_test`]:
  crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_thread_reuse_test

## Documentation Completed

Before this discussion, we completed comprehensive documentation:

| File                       | Change                                                                        |
| :------------------------- | :---------------------------------------------------------------------------- |
| `mio_poller/mod.rs`        | Added race condition diagram, delay explanation, and integration test links   |
| `input_device.rs`          | Added "Drop Behavior" section explaining the lifecycle trigger                |
| `poller_thread.rs`         | Enhanced `MioPollerThread::drop()` docs explaining the two-part exit protocol |
| `global_input_resource.rs` | Fixed protection layers table ordering (kernel buffer first)                  |

### API Ergonomics Changes (commit `f78e1a0`)

| File                            | Change                                                           |
| :------------------------------ | :--------------------------------------------------------------- |
| `input_device.rs`               | Added `single_device_gate` module for singleton enforcement      |
| `input_device.rs`               | Added `subscribe()` method for additional receivers              |
| `input_device.rs`               | Renamed `InputEventReceiverHandle` → `InputDeviceResourceHandle` |
| `types.rs` → `channel_types.rs` | File renamed for clarity; cleaned up type definitions            |

## Next Steps

1. Decide which simplification option (if any) to pursue
2. Evaluate tradeoffs for the specific use case
3. If Option 1 (keep alive forever): refactor to remove exit logic
4. If Option 2 (timeout): implement and benchmark efficiency impact
5. Update tests to match new behavior

## Related Files

### Implementation

- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device.rs` - Device + singleton gate
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/global_input_resource.rs` - Global
  resource
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/channel_types.rs` - Shared types
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mod.rs` - Thread lifecycle docs
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/poller_thread.rs` - Thread impl

### Integration Tests

- `pty_mio_poller_singleton_test.rs` - Singleton semantics validation
- `pty_mio_poller_subscribe_test.rs` - Multiple subscriber broadcast semantics
- `pty_mio_poller_thread_lifecycle_test.rs` - Full cycle: spawn → exit → respawn
- `pty_mio_poller_thread_reuse_test.rs` - Race condition: fast subscriber reuses thread

# Simplify Thread Lifecycle Orchestration

## Status: COMPLETE

The thread lifecycle orchestration is now fully implemented with proper race condition handling:

- **Implemented**: API ergonomics (singleton pattern, `subscribe()` method)
- **Implemented**: Race condition handling via slow/fast path in `PollerThreadState`
- **Decision**: Options 1-3 (keep alive forever, polling timeout, channel-based) are NOT needed —
  the current two-mechanism design works correctly and is well-documented

## Context

After documenting the thread lifecycle in detail, we identified that the current design has
two separate signaling mechanisms that must coordinate. This complexity is **intentional and
correct** — it handles the inherent race condition between thread shutdown and new subscriber
creation.

## Final Architecture

```
PollerThreadState (consolidated from InputResource + PollerBridge):
├── broadcast_tx: PollerEventSender      ← channel for input events
├── thread_liveness: ThreadLiveness      ← tracks running/terminated state
└── waker: mio::Waker                    ← wakes thread from poll() for shutdown check

allocate() function:
├── Fast path: thread running → subscribe to existing channel
└── Slow path: thread terminated → create new Poll + Waker + PollerThreadState + spawn thread
```

## Race Condition Handling

The race condition is **correctly handled** by:

1. **Waker registered with mio Poll** — `ReceiverDropWaker` token wakes thread
2. **`receiver_count()` check** — thread only exits if no subscribers remain
3. **Fast/slow path in `allocate()`** — new subscriber can "save" thread from exiting

```
Main Thread                                mio_poller Thread
───────────                                ─────────────────
1. Old device drops                        │ blocked in poll()
   - receiver_count → 0                    │
   - waker.wake() ─────────────────────────► kernel sees eventfd readable
                                           │
   ┌───────────────────────────────────────┼───────────────────────────────┐
   │        RACE WINDOW                    │ kernel schedules thread       │
   │                                       │ thread returns from syscall   │
   │  New device created via new()         │ ...                           │
   │  → allocate() fast path               │                               │
   │  → receiver_count = 1                 │ finally checks receiver_count │
   └───────────────────────────────────────┼───────────────────────────────┘
                                           ▼
                                           if receiver_count == 0 { exit }
                                           else { continue! }  ← thread reused
```

## Why Alternative Options Were Not Needed

### Option 1: Keep Thread Alive Forever — NOT NEEDED

The current design already efficiently reuses threads when possible, and properly cleans up
resources when all subscribers are gone.

### Option 2: Polling with Timeout — NOT NEEDED

The waker-based approach is more efficient (no busy-waiting) and the race condition is
handled correctly.

### Option 3: Channel-Based Shutdown — NOT NEEDED

The current `receiver_count()` + `ThreadLiveness` approach provides clear separation of
concerns and is well-documented.

## Integration Tests That Validate Behavior

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

| File                            | Content                                                         |
| :------------------------------ | :-------------------------------------------------------------- |
| `mio_poller/mod.rs`             | Race condition diagram, delay explanation, test links           |
| `mio_poller/poller_thread_state.rs` | Thread lifecycle, race condition handling, why thread reuse is safe |
| `mio_poller/poller_thread.rs`   | `MioPollerThread::drop()` docs, two-part exit protocol          |
| `input_device_impl.rs`          | Container/payload pattern, `allocate()` fast/slow path          |
| `input_device_public_api.rs`    | Singleton device pattern, `subscribe()` method                  |

## Related Files

### Implementation

- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs` - Singleton + allocate()
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs` - Public API
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/poller_thread_state.rs` - Thread state
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/poller_thread.rs` - Thread impl
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_receiver_drop.rs` - Shutdown check

### Integration Tests

- `pty_mio_poller_singleton_test.rs` - Singleton semantics validation
- `pty_mio_poller_subscribe_test.rs` - Multiple subscriber broadcast semantics
- `pty_mio_poller_thread_lifecycle_test.rs` - Full cycle: spawn → exit → respawn
- `pty_mio_poller_thread_reuse_test.rs` - Race condition: fast subscriber reuses thread

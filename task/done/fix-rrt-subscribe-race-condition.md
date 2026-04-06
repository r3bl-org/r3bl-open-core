# Task: Fix RRT subscribe() Race Condition [COMPLETE]

## Overview

There was a race condition in `RRT::subscribe()` that caused input to freeze when a
subscriber was rapidly dropped and re-created (e.g., the `tui_apps` example loop that
creates/drops `ReadlineAsyncContext` between examples).

## Root Cause

The race window was between `Continuation::Stop` being returned from the worker loop and
`TerminationGuard::drop()` clearing the waker to `None`:

1. Old subscriber drops -> `receiver_count` becomes 0
2. Mio poller: `handle_receiver_drop_waker_with_sender` returns `Continuation::Stop`
3. `run_worker_loop`: breaks out of loop, but `TerminationGuard` hasn't dropped yet
4. New `subscribe()` call: checks `waker_guard.is_some()` -> true (thread "running")
5. Takes fast path: returns subscriber without spawning thread
6. `TerminationGuard::drop()` clears waker -> thread exits
7. Dead thread, live subscriber. No stdin reader. Input frozen.

## Implementation

### Phase 1: Fix the race in subscribe() [COMPLETE]

Used a scope-based approach in `subscribe()` (`rrt.rs`). A `needs_new_thread` bool is
computed inside a block where the `MutexGuard` lives — it drops naturally at the block
boundary. If the thread is dying (`receiver_count == 0`), `wait_for_thread_exit()` is
called to yield the CPU until the dying thread's `TerminationGuard` clears the waker
slot, then the slow path spawns a fresh thread.

Extracted `RRT::wait_for_thread_exit()` as a public method with rustdocs linking to the
module-level race condition documentation.

Files changed:
- `tui/src/core/resilient_reactor_thread/rrt.rs` - `subscribe()` refactored, `wait_for_thread_exit()` added

### Phase 2: Update RRT Rustdocs [COMPLETE]

Restructured module-level docs to cover both race conditions:
- Renamed `## The Inherent Race Condition` to `## Race Conditions Handled`
- Added sub-section `### 1. The Exit Decision Race (Kernel Scheduling)` (existing content)
- Added sub-section `### 2. The "Dead Thread, Live Subscriber" Race (Fast Path)` (new content with timeline diagram)
- Updated all 6 fragment links from `#the-inherent-race-condition` to `#race-conditions-handled`

Files changed:
- `tui/src/core/resilient_reactor_thread/mod.rs` - heading rename, new docs
- `tui/src/core/resilient_reactor_thread/rrt.rs` - 2 fragment links
- `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs` - 1 fragment link
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs` - 2 fragment links

### Phase 3: Restore tracing in shutdown path [COMPLETE]

Restored the `tracing::debug!()` call in `handler_receiver_drop.rs` inside the
`receiver_count == 0` branch. Previously removed to narrow the race window — no longer
needed since `wait_for_thread_exit()` handles the race properly.

### Phase 4: Verify [COMPLETE]

- Set `DEBUG_TUI_SHOW_MIO_POLLER = true` - all examples work
- Tested `tui_apps` repeatedly with example selection and exit cycles

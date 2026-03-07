<!-- cspell:words openpty tcgetwinsize -->

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Fix flaky test: test_pty_mio_poller_thread_reuse

## Status: COMPLETE

## Symptom

`test_pty_mio_poller_thread_reuse` fails intermittently in `check.fish --full` but passes
when run in isolation. The failure manifests as either:

- **Controlled panic at line 194**: `"Timeout reading from device B"` -- device_b.next()
  times out after 5s because the mio-poller thread exited and the new thread doesn't
  deliver the keystroke
- **Controlled panic at line 86**: `"EOF before receiving {signal}"` -- PTY closes
  before expected signal arrives (consequence of the above panic killing the controlled
  process)
- **Controller panic at line 86**: `"EOF before receiving {signal}"` -- controller sees
  EOF because the controlled process exited due to the above panic

## What the test does

The test validates the **fast-path thread reuse** scenario in the RRT (Resilient Reactor
Thread) pattern:

```text
1. Create device A -> spawns mio-poller thread (generation N)
2. Read one keystroke from device A (proves thread works)
3. Drop device A -> waker fires, thread wakes up
4. IMMEDIATELY create device B (no sleep!) -> calls SINGLETON.subscribe()
5. Read one keystroke from device B
6. Assert generation unchanged (thread reused, not relaunched)
```

The test intentionally races the thread's `receiver_count()` check against device B's
`subscribe()` call. The expected outcome is that device B subscribes before the thread
checks `receiver_count()`, so the thread continues (same generation).

## Root cause analysis

### The race timeline

```text
Controlled process (single-threaded tokio)     mio-poller thread
----------------------------------------------  ---------------------
device_a.next() returns                         blocked in poll()
drop(device_a)
  -> receiver drops (count: 1 -> 0)
  -> WakeOnDrop fires waker                     waker fires, poll() returns
                                                handle_receiver_drop_waker_with_sender()
  +-- RACE WINDOW --+                            sender.receiver_count()
  | device_b = new() |                            -> if 0: return Stop -> thread exits
  | .subscribe()     |                            -> if > 0: return Continue -> thread lives
  +------------------+
```

### Why the race is lost

The `drop(device_a)` line does two things in sequence:

1. **Drops the `BroadcastReceiver`** (inside `SubscriberGuard`) -- this decrements the
   broadcast channel's `receiver_count` from 1 to 0
2. **Drops the `WakeOnDrop`** -- this calls `mio::Waker::wake()`, waking the thread

Both happen synchronously. The waker fires **after** the count has already been
decremented.

Meanwhile, the controlled process runs on a **single-threaded tokio runtime**. After
`drop(device_a)`, the very next statement is `DirectToAnsiInputDevice::new()` which calls
`SINGLETON.subscribe()`. This is also synchronous -- no `.await` points between the drop
and the subscribe.

**The race depends on OS thread scheduling.** After `mio::Waker::wake()`, the
mio-poller thread becomes runnable. Whether it actually runs before the controlled
process's next statement depends on:

- CPU load (parallel test execution)
- OS scheduler decisions
- Whether the mio-poller thread was on-CPU or sleeping
- `mio::Poll::poll()` wake latency

On a **lightly loaded machine**, the controlled process completes `subscribe()` before
the mio-poller thread runs -- test passes. On a **heavily loaded machine** (parallel test
execution during `check.fish --full`), the mio-poller thread may run first, see
`receiver_count() == 0`, and exit -- test fails.

### Why the lifecycle test is NOT flaky

The companion test `test_pty_mio_poller_thread_lifecycle` does the opposite: it **wants**
the thread to exit after device_a drops. It polls `is_thread_running()` in a 100-iteration
loop with 1ms sleeps, giving the thread up to 100ms to exit. This generous window makes
the test timing-insensitive.

# Implementation plan

## Step 0: Choose the fix approach [COMPLETE]

### Rejected options

**Option B (synchronization barrier)**: Insert `yield_now()` + `sleep(1ms)` between drop
and subscribe. Still timing-dependent, may still fail under extreme load.

**Option C (accept either outcome)**: Assert both reuse and relaunch are valid. Doesn't
validate the specific fast-path behavior. Could mask real bugs.

**Option A (subscribe before dropping)**: Create device B before dropping device A.
Blocked by `at_most_one_instance_assert` which panics if two
`DirectToAnsiInputDevice` instances coexist.

### Chosen approach: Hybrid of Options A and D

Use `SINGLETON.subscribe_to_existing()` to create a **temporary subscriber guard** that
overlaps with device A, preventing `receiver_count` from ever reaching 0.

Key properties:

- **Deterministic**: `receiver_count` never reaches 0, so the thread always continues.
- **Still tests thread reuse**: Same generation is verified -- thread was reused, not
  relaunched.
- **No production code changes**: Only test code is modified.
- **No encapsulation violation**: Uses existing public API (`subscribe_to_existing()`).

Revised test flow:

```text
1. Create device A -> thread spawns (generation N, receiver_count = 1)
2. Read keystroke from device A (proves thread works)
3. Create temp_guard = SINGLETON.subscribe_to_existing() -> receiver_count = 2
4. Drop device A -> receiver_count = 2->1, waker fires, thread sees count=1, continues
5. Create device B via DirectToAnsiInputDevice::new() -> receiver_count = 2
6. Drop temp_guard -> receiver_count = 2->1
7. Read keystroke from device B (proves thread serves new device)
8. Assert generation unchanged (thread reused, not relaunched)
```

## Step 1: Fix the race condition in the test [COMPLETE]

File:
`tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`

### Step 1.0: Update module-level doc comment

- Update the description to reflect the new strategy (overlapping subscriptions instead
  of racing)
- Update the "Test Flow" ASCII diagram to show the temp_guard overlap
- Add a cross-reference link to the companion lifecycle test

### Step 1.1: Fix controlled_entry_point

Replace the race-prone `drop -> immediately new` sequence with the deterministic overlap
strategy using `SINGLETON.subscribe_to_existing()`.

### Step 1.2: Update inline comments and eprintln diagnostics

The step comments (Step 1, Step 2, Step 3) and eprintln diagnostics need updating to
describe the overlap strategy instead of "racing the thread."

## Step 2: Add cross-references between companion tests [COMPLETE]

### Step 2.0: Add cross-reference from reuse test to lifecycle test

In the reuse test's module doc, add a reference to the companion lifecycle test.

### Step 2.1: Add cross-reference from lifecycle test to reuse test

File:
`tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`

In the lifecycle test's module doc, add a reference to the companion reuse test.

## Step 3: Verify the fix

### Step 3.0: Run the test in isolation

```bash
cargo test -p r3bl_tui --lib test_pty_mio_poller_thread_reuse -- --nocapture
```

### Step 3.1: Run it 100 times in a loop

```bash
for i in $(seq 1 100); do
  cargo test -p r3bl_tui --lib test_pty_mio_poller_thread_reuse -- --nocapture 2>/dev/null
  if [ $? -ne 0 ]; then echo "FAILED on iteration $i"; break; fi
done
```

### Step 3.2: Verify rustdoc links resolve

```bash
./check.fish --doc
```

## Files to change

- [x] `task/fix-flaky-test-test_pty_mio_poller_thread_reuse.md` -- Update with
  implementation plan
- [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
  -- Fix the race condition + update docs
- [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
  -- Add cross-reference to reuse test

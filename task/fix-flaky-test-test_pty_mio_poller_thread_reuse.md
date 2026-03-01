<!-- cspell:words openpty tcgetwinsize -->

# Fix flaky test: test_pty_mio_poller_thread_reuse

## Status: TODO

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
1. Create device A → spawns mio-poller thread (generation N)
2. Read one keystroke from device A (proves thread works)
3. Drop device A → waker fires, thread wakes up
4. IMMEDIATELY create device B (no sleep!) → calls SINGLETON.subscribe()
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
──────────────────────────────────────────      ─────────────────────────
device_a.next() returns                         blocked in poll()
drop(device_a)
  → receiver drops (count: 1 → 0)
  → WakeOnDrop fires waker                     waker fires, poll() returns
                                                handle_receiver_drop_waker_with_sender()
  ┌── RACE WINDOW ──┐                            sender.receiver_count()
  │ device_b = new() │                            → if 0: return Stop → thread exits
  │ .subscribe()     │                            → if > 0: return Continue → thread lives
  └──────────────────┘
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

## Fix options

### Option A: Subscribe BEFORE dropping device A (recommended)

The test's goal is to verify that the thread continues when a new subscriber appears
before the thread checks `receiver_count()`. We can guarantee this by making device B's
subscription happen **while device A still exists**:

```rust
// Step 2: Subscribe device B WHILE device A is still alive, then drop device A.
// This guarantees receiver_count never hits 0.
let mut device_b = DirectToAnsiInputDevice::new();  // count: 1 → 2
drop(device_a);                                       // count: 2 → 1 (never 0!)
```

**Pros**: Deterministic. receiver_count never reaches 0, so thread always continues.
**Cons**: Changes what the test validates. It no longer tests the "race condition where
subscriber appears after count hits 0 but before thread checks." It instead tests the
simpler "thread continues when count > 0" scenario.

**Assessment**: This is still a valid and useful test -- it confirms the thread handles
overlapping subscribers correctly. The race condition described in the docs is inherently
timing-dependent and cannot be deterministically tested without injecting synchronization
into production code.

### Option B: Add a synchronization barrier in the test

Insert a yield point between drop and subscribe to let the test's tokio runtime process
pending work, then verify the thread is still alive before proceeding:

```rust
drop(device_a);
// Let tokio process pending work (waker fire, etc.)
tokio::task::yield_now().await;
// Small window for thread to wake and check receiver_count
tokio::time::sleep(Duration::from_millis(1)).await;

// Now subscribe. The thread may or may not have checked yet.
let mut device_b = DirectToAnsiInputDevice::new();
```

**Pros**: Closer to the original intent.
**Cons**: Still timing-dependent -- the sleep is a heuristic. May still fail under
extreme load. Also, a 1ms sleep makes the test slower and doesn't guarantee the thread
hasn't already exited.

### Option C: Accept the race and assert either outcome

The test currently asserts `generation_before == generation_after`. Instead, accept that
the race can go either way and assert both outcomes are valid:

```rust
let generation_after = SINGLETON.get_thread_generation();
if generation_before == generation_after {
    eprintln!("  ✓ Fast path: thread reused (same generation)");
} else {
    eprintln!("  ✓ Slow path: thread relaunched (new generation)");
    // Verify thread IS running (it was relaunched, not stuck).
    assert_eq!(SINGLETON.is_thread_running(), LivenessState::Running);
}
```

**Pros**: Never flaky. Tests that both outcomes are correct.
**Cons**: Doesn't validate the specific fast-path behavior. Could mask real bugs where
the thread fails to relaunch.

### Option D: Use `at_most_one_instance_assert` ordering to overlap

`DirectToAnsiInputDevice::new()` has `at_most_one_instance_assert` which prevents two
devices from existing simultaneously. This blocks Option A as written. However, we could
temporarily bypass it by calling `SINGLETON.subscribe()` directly (it's `pub`):

```rust
// Subscribe via SINGLETON directly to avoid at_most_one_instance_assert.
let guard_b = SINGLETON.subscribe().unwrap();
drop(device_a);
// Now wrap guard_b into a DirectToAnsiInputDevice... (may not be possible cleanly)
```

**Pros**: Tests the exact fast-path scenario.
**Cons**: Breaks encapsulation. May not be feasible without API changes.

## Recommended approach: Option A

Option A is the simplest and most robust fix. The key insight is:

1. The race condition documented in `SubscriberGuard` is **by design** -- it's an
   inherent property of the system, not a bug to be tested
2. What the test should validate is that **the thread correctly serves a second
   subscriber** (proving the fast path works)
3. Whether `receiver_count` briefly hits 0 is an OS scheduling artifact, not observable
   application behavior

However, `at_most_one_instance_assert` in `DirectToAnsiInputDevice::new()` may prevent
two devices from coexisting. We need to check if this is a hard constraint or if it can
be relaxed for testing.

## Investigation needed before implementing

1. **Check `at_most_one_instance_assert` behavior**: Can two `DirectToAnsiInputDevice`
   instances coexist briefly? If not, Option A needs `SINGLETON.subscribe()` directly.
   - File: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`
   - Look for the assert implementation and whether it panics or is debug-only

2. **Check if `SINGLETON.subscribe_to_existing()` works for this**: There's a
   `subscribe_to_existing()` method that might let us subscribe without the full
   `DirectToAnsiInputDevice` construction.

3. **Verify the lifecycle test is stable**: Confirm
   `test_pty_mio_poller_thread_lifecycle` never fails, validating that the thread exit
   path is correct.

## Files to change

- [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
  -- Fix the race condition in the test

## Verification

1. Run the test 100 times in a loop to verify stability:
   ```bash
   for i in $(seq 1 100); do
     cargo test -p r3bl_tui --lib test_pty_mio_poller_thread_reuse -- --nocapture 2>/dev/null
     if [ $? -ne 0 ]; then echo "FAILED on iteration $i"; break; fi
   done
   ```

2. Run `check.fish --full` multiple times to verify no flakiness under parallel test load

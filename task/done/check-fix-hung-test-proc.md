<!-- cspell:words ETXTBSY  -->

# Fix Hung PTY Test Processes and Orphan Cleanup

## Context

When `check.fish --full` runs, the test phase sometimes hangs indefinitely. When the user
kills check.fish with Ctrl+C, the test processes survive as orphans (because GNU `timeout`
creates a separate process group). These orphans hold the test binary open, causing subsequent
`cargo test` to fail with `ETXTBSY` from the `wild` linker.

Three independent problems:

1. **PTY tests deadlock**: Controller calls bare `child.wait()` without draining the PTY buffer
2. **No safety net**: If a future test forgets to drain, it hangs forever with no timeout
3. **Orphans survive Ctrl+C**: `check.fish` doesn't ensure child cleanup on signal

---

## Part 1: Fix the PTY Buffer Deadlock (Root Cause)

### Problem

Controller functions call bare `child.wait()` after finishing their assertions. The deadlock:

1. Controller finishes assertions, **stops reading** from PTY
2. Controller drops writer, calls `child.wait()` — blocks waiting for child to exit
3. Child's 2-second inactivity timeout fires
4. Child calls `std::process::exit(0)` which **flushes** buffered `eprintln!()` output
5. PTY buffer is **full** — nobody is reading the controller side anymore
6. Child's `flush()` **blocks** waiting for buffer space
7. **Deadlock**: parent waits for child, child waits for buffer space

This is exactly what `drain_pty_and_wait()` prevents — it drains the buffer before waiting.
But these controllers use bare `child.wait()` instead.

### Fix: `GuardedChild` newtype — make bare `child.wait()` impossible

Wrap `ControlledChild` in a newtype that does NOT expose `.wait()`. The only way to finish
is through `drain_and_wait()`, which drains the PTY buffer first.

**New file: `tui/src/core/test_fixtures/pty_test_fixtures/guarded_child.rs`**

```rust
/// Wraps [`ControlledChild`] to enforce correct PTY cleanup.
///
/// Does NOT expose `.wait()`. The only exit path is [`drain_and_wait`],
/// which drains the PTY buffer first — preventing the PTY buffer deadlock
/// where parent waits for child exit while child blocks on a full buffer.
///
/// [`drain_and_wait`]: Self::drain_and_wait
pub struct GuardedChild {
    child: ControlledChild,
}

impl GuardedChild {
    pub fn new(child: ControlledChild) -> Self { Self { child } }

    /// Clone a kill handle (for the watchdog timer).
    pub fn clone_killer(&self) -> ControlledChildTerminationHandle {
        self.child.clone_killer()
    }

    /// Drain the PTY buffer, then wait for the child to exit.
    /// Consumes self — after this call, the guard is gone.
    pub fn drain_and_wait(
        mut self,
        buf_reader: BufReader<ControllerReader>,
        pty_pair: PtyPair,
    ) {
        drain_pty_and_wait(buf_reader, pty_pair, &mut self.child);
    }
}
```

No `Deref<Target = dyn Child>`, no `.wait()` method — the only way to finish is through
`drain_and_wait()`.

**Modify: `tui/src/core/test_fixtures/pty_test_fixtures/mod.rs`**

Add `mod guarded_child; pub use guarded_child::*;`

**Modify: `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`**

Wrap the child before passing to controller:

```rust
let child = pty_pair.controlled().spawn_command(cmd).expect("...");
let child = $crate::GuardedChild::new(child);  // wrap in newtype
```

Controller signature changes from `(PtyPair, ControlledChild)` to `(PtyPair, GuardedChild)`.

### Affected controller functions (all change `child.wait()` → `child.drain_and_wait()`):

- `tui/src/readline_async/.../pty_ctrl_w_test.rs`
- `tui/src/readline_async/.../pty_ctrl_d_eof_test.rs`
- `tui/src/readline_async/.../pty_ctrl_d_delete_test.rs`
- `tui/src/readline_async/.../pty_ctrl_u_test.rs`
- `tui/src/readline_async/.../pty_ctrl_navigation_test.rs`
- `tui/src/readline_async/.../pty_alt_navigation_test.rs`
- `tui/src/readline_async/.../pty_alt_kill_test.rs`
- `tui/src/readline_async/.../pty_multiline_output_test.rs`
- `tui/src/readline_async/.../pty_shared_writer_no_blank_line_test.rs`
- `tui/src/core/ansi/.../pty_terminal_events_test.rs`
- `tui/src/core/ansi/.../pty_mio_poller_singleton_test.rs`
- `tui/src/core/ansi/.../pty_mio_poller_thread_lifecycle_test.rs`

Pattern change in each:
```rust
// Before (deadlocks — bare child.wait() now impossible):
fn controller(pty_pair: PtyPair, mut child: ControlledChild) {
    // ... test work ...
    drop(writer);
    match child.wait() { ... }
}

// After (GuardedChild forces drain):
fn controller(pty_pair: PtyPair, child: GuardedChild) {
    // ... test work ...
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);
}
```

Each controller keeps `buf_reader` and `pty_pair` alive until the final `drain_and_wait()`.

### Tests that currently use `read_lines_and_drain()`

2 tests call `read_lines_and_drain(pty_pair, &mut child, ...)` which combines reading
and draining. With `GuardedChild`, split into two steps:

1. Make `read_until_marker()` public (currently private in `read_lines_and_drain.rs:183`)
2. Tests call `read_until_marker()` for reading, then `child.drain_and_wait()` for cleanup

```rust
// Before:
let result = read_lines_and_drain(pty_pair, &mut child, "CONTROLLED_DONE", filter);

// After:
let reader = pty_pair.controller().try_clone_reader().unwrap();
let mut buf_reader = BufReader::new(reader);
let (lines, found_marker) = read_until_marker(&mut buf_reader, "CONTROLLED_DONE", &filter);
child.drain_and_wait(buf_reader, pty_pair);
let result = ReadLinesResult { lines, found_marker };
```

Affected tests:
- `tui/src/readline_async/.../pty_multiline_output_test.rs`
- `tui/src/readline_async/.../pty_shared_writer_no_blank_line_test.rs`

`GuardedChild` stays with a single method: `drain_and_wait()`.

---

## Part 2: PTY Test Watchdog (Safety Net)

### Purpose

Defense-in-depth: `GuardedChild` makes the deadlock impossible for normal code paths.
The watchdog catches edge cases (e.g., controller panics before calling `drain_and_wait`,
or a blocking read loop that never reaches cleanup). After 30 seconds, the watchdog kills
the child, converting an indefinite hang into a test failure.

### Files to modify

**New file: `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_watchdog.rs`**

```rust
pub const PTY_TEST_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);

pub struct PtyTestWatchdog {
    cancelled: Arc<AtomicU8>,  // 0 = active, 1 = cancelled. Uses AtomicU8Ext.
}
```

- `new(killer: Box<dyn ChildKiller>, timeout: Duration)` - Spawns watchdog thread
- Thread does a single `thread::sleep(timeout)`, then checks `cancelled` flag
- If not cancelled (0): calls `killer.kill()` (test hung, unblock reads)
- If cancelled (1): exits immediately without calling kill (test completed normally)
- `Drop` impl sets `cancelled` to 1 — no join, no blocking
- Thread wakes after timeout, sees the flag, exits cleanly

**Modify: `tui/src/core/test_fixtures/pty_test_fixtures/mod.rs`**

Add `mod pty_test_watchdog; pub use pty_test_watchdog::*;`

**Modify: `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`**

After line 258 (child spawned), before line 261 (controller called), add:

```rust
let child = $crate::GuardedChild::new(child);
let killer = child.clone_killer();
let _watchdog = $crate::PtyTestWatchdog::new(killer, $crate::PTY_TEST_WATCHDOG_TIMEOUT);
$controller_fn(pty_pair, child);
// _watchdog dropped here -> sets cancelled flag, thread exits cleanly on wake
```

### Optional: configurable timeout per test

Add a second macro arm with `timeout: $timeout:expr` parameter for tests needing
longer/shorter timeouts. Default arm uses `PTY_TEST_WATCHDOG_TIMEOUT`.

---

## Part 3: check.fish Process Cleanup (Fish)

### Problem

GNU `timeout` (without `--foreground`) creates a new process group for its children. When
Ctrl+C sends SIGINT to the terminal's foreground process group, `timeout` itself receives it
and exits, but cargo test and its children (in the separate group) survive as orphans.

### Approach: `--foreground` flag + ETXTBSY recovery

**Primary fix**: Add `--foreground` to all `timeout` invocations. This keeps cargo and all its
children in the shell's process group, so Ctrl+C SIGINT reaches them directly.

With `--foreground`:
- **Ctrl+C**: SIGINT goes to entire foreground group -> all processes die
- **Timer expires**: SIGTERM goes to cargo only (not group), cargo propagates to children

**Safety net**: Add `detect_text_file_busy()` to `check_detection.fish` so that if ETXTBSY
occurs (e.g., from `kill -9` bypass), check.fish can detect it, kill orphans, and retry.

### Files to modify

**Modify: `check_cargo.fish`** (lines 13, 18, 23, 28, 33, 38, 45, 52)

Change all 8 `timeout` invocations from:
```fish
ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo ...
```
to:
```fish
ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo ...
```

**Modify: `check_detection.fish`** - Add after `detect_linker_failure`:

```fish
function detect_text_file_busy
    set -l temp_output $argv[1]
    if grep -qE "Text file busy" $temp_output 2>/dev/null
        return 0
    end
    return 1
end
```

**Modify: `check_orchestrators.fish`** - Add ETXTBSY detection block after linker failure
detection (after line 166), with recovery action that kills orphaned test processes before
retry:

```fish
if detect_text_file_busy $temp_output
    # Kill orphaned test processes holding the binary open
    pkill -f "r3bl_tui.*--quiet" 2>/dev/null
    sleep 1
    return 2  # Recoverable -> retry
end
```

---

## Verification

1. **Drain fix**: Run `./check.fish --test` — all existing tests pass, no hangs
2. **Watchdog**: Temporarily break one test (remove drain) — watchdog kills after 30s, test
   fails (not hangs)
3. **Ctrl+C cleanup**: Run `./check.fish --test`, press Ctrl+C, verify no orphans with
   `pgrep -f r3bl_tui.*quiet`
4. **ETXTBSY recovery**: If orphans somehow survive, verify check.fish detects "Text file
   busy", kills orphans, and retries

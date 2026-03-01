<!-- cspell:words SIGINT SIGTERM ETXTBSY eprintln openpt grantpt unlockpt errno -->

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [PTY Slave fd Leak in `generate_pty_test!` Macro — Audit & Fix](#pty-slave-fd-leak-in-generate_pty_test-macro--audit--fix)
  - [Overview](#overview)
    - [The bug in one sentence](#the-bug-in-one-sentence)
    - [Impact](#impact)
  - [The PTY File Descriptor Model](#the-pty-file-descriptor-model)
  - [The Bug: Step by Step](#the-bug-step-by-step)
    - [Before the fix: what `generate_pty_test!` did](#before-the-fix-what-generate_pty_test-did)
    - [The deadlock scenario (child panics under load)](#the-deadlock-scenario-child-panics-under-load)
    - [Why it only manifested in the full suite](#why-it-only-manifested-in-the-full-suite)
    - [Why `drain_and_wait` didn't save us](#why-drain_and_wait-didnt-save-us)
  - [The Fix](#the-fix)
    - [Step 1: Restructure `PtyPair` to support early slave close](#step-1-restructure-ptypair-to-support-early-slave-close)
    - [Step 2: Close slave immediately after spawn in the macro](#step-2-close-slave-immediately-after-spawn-in-the-macro)
    - [Step 3: Consolidate `drain_pty_and_wait` into `single_thread_safe_controlled_child.rs`](#step-3-consolidate-drain_pty_and_wait-into-single_thread_safe_controlled_childrs)
    - [Step 4: Update `drain_pty_and_wait` docs](#step-4-update-drain_pty_and_wait-docs)
  - [Why This Is Standard Unix PTY Hygiene](#why-this-is-standard-unix-pty-hygiene)
  - [Verification](#verification)
  - [Files Changed](#files-changed)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# PTY Slave fd Leak in `generate_pty_test!` Macro — Audit & Fix

## Overview

During a routine audit of the `SingleThreadSafeControlledChild` documentation, we discovered that
`test_pty_mio_poller_thread_reuse` would **hang indefinitely** when run as part of the full test
suite (`./check.fish --test`), while passing instantly in isolation. Investigation revealed this was
**not** a timing issue but a real **file descriptor leak** in the `generate_pty_test!` macro that
made PTY read loops unable to receive EOF after the child process exited.

### The bug in one sentence

The parent process held the slave (controlled) side of the PTY open for the entire test, so even
after the child exited, the kernel never delivered EOF to the master (controller) reader — any read
loop that waited for child output would hang forever.

### Impact

- **Any** controller function that reads from the PTY in a loop and relies on EOF to detect child
  exit was vulnerable.
- The `SingleThreadSafeControlledChild` / `drain_and_wait` mechanism masked this for the **happy
  path** (child exits normally) because it happened to drain quickly. But if the child **panicked**
  or exited abnormally, the controller's read loop would deadlock permanently.
- The 30-second `PtyTestWatchdog` would kill the child process — but the child was already dead (a
  zombie). Killing a zombie doesn't close the **parent's** copy of the slave fd. So the watchdog
  couldn't save us either.

---

## The PTY File Descriptor Model

To understand the bug, you need to understand how PTY file descriptors work in Unix:

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                           Kernel PTY Layer                                │
│                                                                           │
│   Master fd ◄──── buffer (~1KB macOS, ~4KB Linux) ────► Slave fd          │
│                                                                           │
│   EOF rule: master reader gets EOF only when ALL slave fds are closed.    │
│   If parent AND child both hold slave fds, BOTH must close for EOF.       │
└────────────────────────────────────────────────────────────────────────────┘
```

When `openpty()` is called, the kernel creates a master/slave pair. The **PtyPair** struct holds
both sides. When `spawn_command()` is called, `portable_pty` duplicates the slave fd into the
child's stdin/stdout/stderr. Now **two processes** hold slave fds:

```text
Parent process:
  - controller fd  (via pty_pair.controller())
  - controlled fd  (via pty_pair.controlled())     ← THIS IS THE LEAK

Child process:
  - controlled fd  (stdin, stdout, stderr — duplicated by spawn_command)
```

The critical kernel rule: **the master reader receives EOF only when ALL slave fds are closed.** If
the parent still holds a slave fd, the master reader will block on `read()` forever, even after the
child exits and closes its slave fds.

---

## The Bug: Step by Step

### Before the fix: what `generate_pty_test!` did

```rust
let pty_pair = PtyPair::from(raw_pty_pair);  // holds master + slave

let child = pty_pair.controlled()            // borrows slave
    .spawn_command(cmd)                       // child gets its own slave fds
    .expect("...");
                                              // parent STILL holds slave fd in pty_pair!

let child = SingleThreadSafeControlledChild::new(child);
let _watchdog = PtyTestWatchdog::new(...);

$controller_fn(pty_pair, child);             // controller gets pty_pair with slave still open
```

### The deadlock scenario (child panics under load)

`test_pty_mio_poller_thread_reuse` tests a tight race condition. Under full-suite load (~2690 tests
running concurrently), the race sometimes goes the wrong way:

1. **Controlled subprocess panics** — assertion fails because the mio poller thread was relaunched
   instead of reused.
2. **Child exits abnormally** — panic unwinds, child process terminates. The child's slave fds
   (stdin/stdout/stderr) are closed by the OS.
3. **But the parent's slave fd is still open** in `pty_pair`.
4. **Controller's `read_line()` loop blocks forever** — it's waiting for the `REUSE_TEST_PASSED`
   marker, but the child is dead. Because the parent holds a slave fd, the master reader never gets
   EOF.
5. **Watchdog fires after 30 seconds** — calls `kill()` on the child. But the child is already a
   zombie. Killing a zombie is a no-op. The parent's slave fd remains open.
6. **Permanent deadlock** — the controller thread is stuck in `read_line()` forever. The test hangs
   until the entire `check.fish` process is killed externally.

### Why it only manifested in the full suite

In isolation, the race condition always goes the right way (the thread is reused). Under load, OS
scheduling delays cause the mio poller thread to check `receiver_count` before device B subscribes,
so it exits. Then device B triggers a new thread (new generation). The controlled process sees
`generation_before != generation_after` and panics.

### Why `drain_and_wait` didn't save us

`drain_and_wait()` is called AFTER the controller function returns. But the controller function
never returns — it's stuck in `read_line()`. The drain code was unreachable.

---

## The Fix

### Step 1: Restructure `PtyPair` to support early slave close

**File: `tui/src/core/pty/pty_core/pty_types.rs`**

Changed `PtyPair` from wrapping an opaque `portable_pty::PtyPair` to storing the two halves
separately:

```rust
// Before:
pub struct PtyPair {
    inner: portable_pty::PtyPair,
}

// After:
pub struct PtyPair {
    controller: Controller,
    maybe_controlled: Option<Controlled>,  // Option enables early close
}
```

Added `close_controlled()`:

```rust
pub fn close_controlled(&mut self) {
    drop(self.maybe_controlled.take());
}
```

The `Option` pattern makes the state transition explicit: after `close_controlled()`, the
`maybe_controlled` field is `None`, and any attempt to access it panics with a clear message.
Removed unused `into_inner()`.

### Step 2: Close slave immediately after spawn in the macro

**File: `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`**

```rust
let child = pty_pair.controlled()
    .spawn_command(cmd)
    .expect("...");

// NEW: Close the slave side immediately after spawning.
pty_pair.close_controlled();

let child = SingleThreadSafeControlledChild::new(child);
```

Now the fd ownership is correct:

```text
Parent process:
  - controller fd  (via pty_pair.controller())
  - controlled fd  CLOSED via close_controlled() ✓

Child process:
  - controlled fd  (stdin, stdout, stderr)
```

When the child exits (normally or via panic), its slave fds close, and the kernel delivers EOF to
the master reader. The controller's `read_line()` loop can now detect child death and fail with a
clear message instead of hanging.

### Step 3: Consolidate `drain_pty_and_wait` into `single_thread_safe_controlled_child.rs`

**Deleted: `drain_pty_and_wait.rs`**

Moved the function into `single_thread_safe_controlled_child.rs` since it's the only caller (via
`SingleThreadSafeControlledChild::drain_and_wait`). Updated docs to reflect that the slave is now
closed before the read loop, not during drain.

### Step 4: Update `drain_pty_and_wait` docs

The function's step 1 previously said "Drop `pty_pair` — closes the parent's handle to the
controlled fd." This was misleading after the fix. Updated to:

> Drop `pty_pair` — closes the parent's controller (master) fd. The controlled (slave) fd must
> already be closed by the caller (via `PtyPair::close_controlled`) before the controller's read
> loop begins.

---

## Why This Is Standard Unix PTY Hygiene

This pattern — **close the slave fd in the parent immediately after fork/spawn** — is standard Unix
practice. From the POSIX documentation and every PTY tutorial:

1. `openpty()` or `posix_openpt()` + `grantpt()` + `unlockpt()` creates master + slave
2. `fork()`
3. **In the child**: close master, set up slave as stdin/stdout/stderr
4. **In the parent**: **close slave**, communicate via master only

Step 4 is what was missing. The `portable_pty` library handles the child side correctly
(`spawn_command` sets up the slave in the child), but it doesn't close the parent's copy of the
slave. That's the caller's responsibility, and `generate_pty_test!` wasn't doing it.

---

## Verification

After the fix:

- `./check.fish --test` passes in ~13 seconds with zero hangs
- `test_pty_mio_poller_thread_reuse` completes normally even when the race goes either way
- `./check.fish --quick-doc` passes with zero warnings
- All intra-doc links resolve correctly

---

## Files Changed

| File                                     | Change                                                                                                |
| :--------------------------------------- | :---------------------------------------------------------------------------------------------------- |
| `tui/src/core/pty/pty_core/pty_types.rs` | `PtyPair` restructured to `Option<Controlled>`, added `close_controlled()`, removed `into_inner()`    |
| `generate_pty_test.rs`                   | Added `pty_pair.close_controlled()` after spawn, added orchestration docs                             |
| `single_thread_safe_controlled_child.rs` | Absorbed `drain_pty_and_wait` function, updated docs                                                  |
| `drain_pty_and_wait.rs`                  | Deleted (moved into `single_thread_safe_controlled_child.rs`)                                         |
| `mod.rs`                                 | Removed `drain_pty_and_wait` module, renamed `guarded_child` to `single_thread_safe_controlled_child` |
| `task/done/check-fix-hung-test-proc.md`  | Updated plan to reflect `SingleThreadSafeControlledChild` naming and `clone_termination_handle`       |

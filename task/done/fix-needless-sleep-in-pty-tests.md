<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Three antipatterns fixed](#three-antipatterns-fixed)
  - [Scope](#scope)
  - [Reference implementation](#reference-implementation)
- [Implementation plan](#implementation-plan)
  - [Step 0: Fix readline\_async PTY tests (6 files) \[COMPLETE\]](#step-0-fix-readline_async-pty-tests-6-files-complete)
  - [Step 1: Fix vt\_100\_terminal\_input\_parser PTY tests (8 files) \[COMPLETE\]](#step-1-fix-vt_100_terminal_input_parser-pty-tests-8-files-complete)
    - [Step 1.0: Remove sleeps and bump watchdogs \[COMPLETE\]](#step-10-remove-sleeps-and-bump-watchdogs-complete)
    - [Step 1.1: Move CONTROLLED\_READY after device creation \[COMPLETE\]](#step-11-move-controlled_ready-after-device-creation-complete)
  - [Step 2: Fix backend\_compat\_input\_test.rs \[COMPLETE\]](#step-2-fix-backend_compat_input_testrs-complete)
  - [Step 3: Fix channel-based tests (2 files) \[COMPLETE\]](#step-3-fix-channel-based-tests-2-files-complete)
  - [Step 4: Fix premature READY in readline\_async controlled processes (7 files) \[COMPLETE\]](#step-4-fix-premature-ready-in-readline_async-controlled-processes-7-files-complete)
  - [Step 5: Update stale rustdoc and comments \[COMPLETE\]](#step-5-update-stale-rustdoc-and-comments-complete)
    - [Step 5.0: Fix stale "waits ~200ms" protocol docs (6 files) \[COMPLETE\]](#step-50-fix-stale-waits-200ms-protocol-docs-6-files-complete)
    - [Step 5.1: Improve READY signal inline comments (15 files) \[COMPLETE\]](#step-51-improve-ready-signal-inline-comments-15-files-complete)
    - [Step 5.2: Fix "raw mode enabled" log messages \[COMPLETE\]](#step-52-fix-raw-mode-enabled-log-messages-complete)
  - [Step 6: Codebase audit \[COMPLETE\]](#step-6-codebase-audit-complete)
  - [Step 7: Final verification \[COMPLETE\]](#step-7-final-verification-complete)
- [Summary of all changes](#summary-of-all-changes)
  - [readline\_async tests (7 files)](#readline_async-tests-7-files)
  - [vt\_100\_terminal\_input\_parser tests (8 files)](#vt_100_terminal_input_parser-tests-8-files)
  - [Other tests (3 files)](#other-tests-3-files)
- [Out of scope](#out-of-scope)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

Codebase-wide fix for three antipatterns causing flaky PTY integration tests. Initially scoped to 6
`readline_async` files, then extended to cover the entire codebase: 18 test files across 3 modules.

## Three antipatterns fixed

**Antipattern 1: Needless `std::thread::sleep()` between flush and blocking read.**
The controller's `read_line()` / `read_line_state()` already blocks until a response arrives. The
sleep is pure dead time that slows tests and makes them fragile under CPU load (too short = watchdog
fires, too long = wasted CI time).

**Why the sleeps are unnecessary:** The controller-controlled protocol is request-response:

```text
Controller: write_all(input) + flush()  →  PTY buffer  →  Controlled: event loop reads input
Controller: read_line_state() blocks    ←  PTY buffer  ←  Controlled: println!("Line: ...")
```

The `read_line_state()` closure calls `buf_reader.read_line()` which blocks until a complete line
arrives. The sleep between flush and read is dead time — it doesn't synchronize anything because
`read_line()` already waits. The controlled process's `DebouncedState` (10ms debounce) batches rapid
input and prints a single line state, which the controller then reads.

**Antipattern 2: Inactivity watchdog set to 2 seconds.**
Under parallel test execution (2651 tests, ~15 PTY tests running simultaneously), CPU scheduling
delays can cause input events to arrive late. A 2s watchdog is too aggressive; bumped to 5s for
headroom.

**Antipattern 3: `CONTROLLED_READY` emitted before `DirectToAnsiInputDevice::new()`.**
The controller waits for `CONTROLLED_READY` before sending input. If the signal is emitted before
the input device exists, the controller can send input that arrives before the mio poller thread is
watching stdin. Under CPU load, the input is lost and the controlled process times out.

The fix: create the device first, then emit `CONTROLLED_READY`. The mio poller thread must already
be watching stdin before the controller sends any input through the PTY.

```text
WRONG (race condition):                  CORRECT (no race):
  println!(CONTROLLED_READY)               DirectToAnsiInputDevice::new()
  flush()                                  // mio poller now watching stdin
  // controller starts sending!            println!(CONTROLLED_READY)
  DirectToAnsiInputDevice::new()           flush()
  // device missed the input!              // controller starts sending safely
```

## Scope

**In scope:** All PTY test files using the controller/controlled request-response pattern across:

- `tui/src/readline_async/readline_async_impl/integration_tests/` (7 files)
- `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/` (8 files)
- `tui/src/core/terminal_io/backend_compat_tests/` (1 file)
- `tui/src/readline_async/readline_async_impl/integration_tests/` channel-based tests (2 files)

## Reference implementation

`pty_ctrl_d_delete_test.rs` (commit `37f9e396`) was the initial reference for how the fix should
look.

**IMPORTANT: New Standard Handshake Pattern**
As of March 2026, the **[`wait_for_ready`]** shared fixture is the preferred way to handle PTY
handshakes. It is more robust than manual loops because it returns `leftover` bytes that may have
been batched by the kernel during the `READY` signal. Use it with `std::io::Chain` to ensure zero
data loss.

Key properties of robust PTY tests:
- **Zero sleeps** in controller between write/read cycles
- **Watchdog = 5s** (not 2s) for parallel execution headroom
- **Waits for `CONTROLLED_READY`** via `wait_for_ready` shared fixture.
- **`CONTROLLED_READY` emitted after `DirectToAnsiInputDevice::new()`** so the mio poller thread is
  already watching stdin before the controller sends any input

[`wait_for_ready`]: crate::wait_for_ready

# Implementation plan

## Step 0: Fix readline_async PTY tests (6 files) [COMPLETE]

Applied the three-part fix (remove sleeps, bump watchdog, add READY signal) to 6 files:

- `pty_ctrl_w_test.rs` — 5 sleeps removed, READY signal added, watchdog 2s → 5s
- `pty_ctrl_u_test.rs` — 3 sleeps removed, watchdog 2s → 5s
- `pty_alt_navigation_test.rs` — 4 sleeps removed, READY signal added, watchdog 2s → 5s
- `pty_alt_kill_test.rs` — 8 sleeps removed, READY signal added, watchdog 2s → 5s
- `pty_ctrl_d_eof_test.rs` — 1 sleep removed, watchdog 2s → 5s
- `pty_ctrl_navigation_test.rs` — 4 sleeps removed, READY signal added, watchdog 2s → 5s

## Step 1: Fix vt_100_terminal_input_parser PTY tests (8 files) [COMPLETE]

These tests follow the same request-response pattern but are in a different module. The controlled
processes had `CONTROLLED_READY` emitted before the Tokio runtime was created and before device
creation — a much larger race window than the readline_async tests.

### Step 1.0: Remove sleeps and bump watchdogs [COMPLETE]

Removed 1 `std::thread::sleep()` from each controller and bumped watchdog from 2s to 5s in:

- `pty_input_device_test.rs`
- `pty_terminal_events_test.rs`
- `pty_keyboard_modifiers_test.rs`
- `pty_mouse_events_test.rs`
- `pty_new_keyboard_features_test.rs`
- `pty_utf8_text_test.rs`
- `pty_bracketed_paste_test.rs`
- `pty_sigwinch_test.rs` (already had 5s watchdog, only sleep removed)

Preserved legitimate 10ms sleeps in `WouldBlock` retry loops (pty_utf8_text_test.rs,
pty_mouse_events_test.rs) and the 200ms sleep in pty_sigwinch_test.rs (signal handler setup).

### Step 1.1: Move CONTROLLED_READY after device creation [COMPLETE]

In all 8 files, the controlled entry point was restructured:

- Added `CONTROLLED_STARTING` constant and signal (emitted before runtime creation)
- Moved `CONTROLLED_READY` to after `DirectToAnsiInputDevice::new()` inside
  `runtime.block_on(async { ... })`
- Updated controller wait loop to handle both `CONTROLLED_STARTING` and `CONTROLLED_READY`

This was the critical fix for `test_pty_terminal_events` which was failing ~5% of the time because
the controller sent a Window Resize sequence before the controlled's input device existed.

## Step 2: Fix backend_compat_input_test.rs [COMPLETE]

- Removed 1 `std::thread::sleep(Duration::from_millis(50))` from the controller loop
- Bumped the `run_event_loop!` macro's inactivity timeout from 3s to 5s
- Removed unused `Duration` and `time::Duration` imports

## Step 3: Fix channel-based tests (2 files) [COMPLETE]

These tests use `tokio::sync::mpsc::channel` (not the PTY request-response pattern). The `writeln!`
calls happen synchronously before `block_on()`, so messages are already in the channel buffer when
`try_recv()` runs.

- `pty_multiline_output_test.rs` — removed `tokio::time::sleep(Duration::from_millis(50)).await`
- `pty_shared_writer_no_blank_line_test.rs` — removed same sleep, removed unused `Duration` import

## Step 4: Fix premature READY in readline_async controlled processes (7 files) [COMPLETE]

The readline_async tests had `CONTROLLED_READY` emitted before `DirectToAnsiInputDevice::new()`.
While the race window was small (both within the same async block), it was inconsistent with the
vt100 tests and still a race condition in principle. Fixed by swapping the order in all 7 files:

```rust
// BEFORE:
println!("{CONTROLLED_READY}");
std::io::stdout().flush();
let mut input_device = DirectToAnsiInputDevice::new();

// AFTER:
let mut input_device = DirectToAnsiInputDevice::new();
println!("{CONTROLLED_READY}");
std::io::stdout().flush();
```

Files: `pty_ctrl_d_delete_test.rs`, `pty_ctrl_w_test.rs`, `pty_ctrl_u_test.rs`,
`pty_alt_navigation_test.rs`, `pty_alt_kill_test.rs`, `pty_ctrl_d_eof_test.rs`,
`pty_ctrl_navigation_test.rs`.

## Step 5: Update stale rustdoc and comments [COMPLETE]

### Step 5.0: Fix stale "waits ~200ms" protocol docs (6 files) [COMPLETE]

The `## Test Protocol` rustdoc sections in 6 readline_async tests still described the old
sleep-based protocol ("Controller flushes and waits ~200ms"). Updated to match the reference
implementation: "Controller flushes and blocks reading controlled stdout until it sees 'Line: ...'".

### Step 5.1: Improve READY signal inline comments (15 files) [COMPLETE]

Changed the inline comment from the *what* ("so device exists before controller starts sending") to
the *why* ("so the mio poller thread is already watching stdin before the controller sends any input
through the PTY"). Applied consistently across all 15 PTY test files.

### Step 5.2: Fix "raw mode enabled" log messages [COMPLETE]

Removed "raw mode enabled" from the `eprintln!` messages in controller wait loops. Raw mode is
handled by the `generate_pty_test!` macro, not by the controlled process's `CONTROLLED_READY`
signal. The message now just says "input device created".

## Step 6: Codebase audit [COMPLETE]

Full codebase audit confirmed no remaining instances of:

- Needless `std::thread::sleep()` between flush and blocking read in any PTY test
- `Duration::from_secs(2)` watchdog timers in any PTY test
- `CONTROLLED_READY` emitted before `DirectToAnsiInputDevice::new()` in any PTY test

Remaining legitimate sleeps:

- `pty_utf8_text_test.rs:138` — 10ms WouldBlock backoff
- `pty_mouse_events_test.rs:176` — 10ms WouldBlock backoff
- `pty_sigwinch_test.rs:128` — 200ms signal handler setup
- `pty_mio_poller_thread_lifecycle_test.rs:203` — 1ms thread status polling
- `pty_mio_poller_subscribe_test.rs:221,253` — 10ms retry loops

## Step 7: Final verification [COMPLETE]

All PTY tests pass 20/20 when run together. The full test suite (`cargo test --lib -p r3bl_tui`)
passes with only occasional failures from unrelated PTY resource contention
(`test_backend_compat_output_compare`, `test_pty_resize`).

# Summary of all changes

## readline_async tests (7 files)

| File                          | Sleeps removed | Watchdog | READY signal       |
| :---------------------------- | :------------- | :------- | :----------------- |
| `pty_ctrl_d_delete_test.rs`   | 0 (reference)  | 5s       | Moved after device |
| `pty_ctrl_w_test.rs`          | 5              | 2s → 5s  | Added + moved      |
| `pty_ctrl_u_test.rs`          | 3              | 2s → 5s  | Moved after device |
| `pty_alt_navigation_test.rs`  | 4              | 2s → 5s  | Added + moved      |
| `pty_alt_kill_test.rs`        | 8              | 2s → 5s  | Added + moved      |
| `pty_ctrl_d_eof_test.rs`      | 1              | 2s → 5s  | Moved after device |
| `pty_ctrl_navigation_test.rs` | 4              | 2s → 5s  | Added + moved      |

## vt_100_terminal_input_parser tests (8 files)

| File                                | Sleeps removed | Watchdog | READY signal       |
| :---------------------------------- | :------------- | :------- | :----------------- |
| `pty_input_device_test.rs`          | 1              | 2s → 5s  | Moved after device |
| `pty_terminal_events_test.rs`       | 1              | 2s → 5s  | Moved after device |
| `pty_keyboard_modifiers_test.rs`    | 1              | 2s → 5s  | Moved after device |
| `pty_mouse_events_test.rs`          | 1              | 2s → 5s  | Moved after device |
| `pty_new_keyboard_features_test.rs` | 1              | 2s → 5s  | Moved after device |
| `pty_utf8_text_test.rs`             | 1              | 2s → 5s  | Moved after device |
| `pty_bracketed_paste_test.rs`       | 1              | 2s → 5s  | Moved after device |
| `pty_sigwinch_test.rs`              | 1              | 5s       | Moved after device |

## Other tests (3 files)

| File                                      | Change                             |
| :---------------------------------------- | :--------------------------------- |
| `backend_compat_input_test.rs`            | 1 sleep removed, watchdog 3s → 5s  |
| `pty_multiline_output_test.rs`            | 1 tokio sleep removed              |
| `pty_shared_writer_no_blank_line_test.rs` | 1 tokio sleep removed              |

# Out of scope

- `test_backend_compat_output_compare` — output rendering test, residual flakiness from PTY
  resource contention (already fixed in commit d525ad33 for the main byte-loss issue).
- `test_pty_resize` — sporadic "No such file or directory" from PTY device exhaustion during
  parallel execution. Would need concurrency limiting (`#[serial]` or semaphore) to fix.
- `terminal_raw_mode` integration tests — not PTY session tests.
- `backend_compat_input_test.rs` controlled processes — `signal_ready()` is called before raw mode
  and device creation by necessity (println doesn't work after raw mode). The controller's blocking
  read provides enough latency for the device to be created before input arrives.

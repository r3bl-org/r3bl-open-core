<!-- cspell:words setsid TIOCSCTTY cmdbuilder -->
# Task: Fix PTYMux UI Freeze (stdout Backpressure)

## Overview

Investigate and fix two primary issues identified during the debugging session of the `PTYMux`
terminal multiplexer (`pty_mux_example`). The symptoms include an unresponsive UI (frozen input)
while background output processing continues, and potential conflicts with the terminal backend.

## Approach A: Rendering Bypass & PTY Session Theft

### Hypotheses and Findings

#### 1. Architectural Bypass of Terminal Backend — ELIMINATED

~~The `PTYMux` output renderer (`tui/src/core/pty/pty_mux/output_renderer.rs`) is directly
instantiating and calling `paint()`/`paint_diff()` on `OffscreenBufferPaintImplCrossterm`.~~

**Hypothesis was:** This bypasses the `TERMINAL_LIB_BACKEND` configuration (which defaults to `DirectToAnsi`
on Linux) and forces execution through the Crossterm-specific backend executor, leading to raw
mode conflicts.

**Finding:** Verified by tracing `OffscreenBufferPaintImpl::paint()` and `paint_diff()`.
- `OffscreenBufferPaintImpl` is completely backend-agnostic.
- Its `paint()` and `paint_diff()` methods create a `RenderOpOutputVec` and call `render_ops.execute_all()`.
- `execute_all()` maps to `RenderOpOutputVec::route_paint_render_op_output_to_backend`.
- Inside this dispatcher, the code explicitly checks `match TERMINAL_LIB_BACKEND` and routes to either `PaintRenderOpImplCrossterm` or `RenderOpPaintImplDirectToAnsi`.

**Conclusion:** `PTYMux` is NOT bypassing the configuration. It respects the backend correctly. No architectural bypass exists.

#### 2. The `/dev/tty` Theft Hypothesis (Input Starvation) — ELIMINATED

~~The `mio_poller` input thread (a robust global singleton used successfully across the framework) is
starving. The key difference in `PTYMux` is that it spawns interactive child processes (`claude`,
`htop`, `bash`).~~

**Hypothesis was:** If `portable_pty` does not call `setsid()` + `TIOCSCTTY`, the child inherits
the parent's controlling terminal, and interactive TUI children opening `/dev/tty` would steal
bytes from the parent's input.

**Finding:** Verified by reading `portable_pty` source (`wezterm/pty/src/unix.rs:240-283` and
`wezterm/pty/src/cmdbuilder.rs:222,234,261`):

- All `CommandBuilder` constructors default `controlling_tty: true`.
- At spawn time, the child calls `setsid()` then `ioctl(0, TIOCSCTTY, 0)`, making the PTY slave
  the child's controlling terminal.
- Our code (`pty_engine/pty_pair.rs`) never calls `set_controlling_tty(false)`.
- Therefore the child's `/dev/tty` resolves to its own PTY slave, fully isolated from the parent.

**Conclusion:** `/dev/tty` theft is not the cause of the input starvation.

### Implementation Plan

#### Phase 1: ~~Fix Architectural Rendering Bypass~~ — ELIMINATED

- [x] Verified `OffscreenBufferPaintImpl` logic in `tui/src/tui/terminal_lib_backends/offscreen_buffer/paint_impl.rs`.
- [x] Confirmed it delegates to `TERMINAL_LIB_BACKEND` via `execute_all()`.
- No fix needed — `PTYMux` correctly respects the backend configuration.

#### Phase 2: ~~Investigate and Fix Child Process PTY Association~~ — ELIMINATED

- [x] Examine `tui/src/core/pty/pty_engine/pty_pair.rs` and `portable_pty` source.
- [x] Verified `portable_pty` calls `setsid()` + `TIOCSCTTY` by default (`controlling_tty: true`).
- [x] Confirmed our code never overrides this. Child processes are properly session-isolated.
- No fix needed — `/dev/tty` theft is not the cause.

## Approach B: Synchronous stdout Backpressure Blocking the Async Runtime

### Hypothesis

The UI freeze is caused by synchronous `write_all()`/`flush()` calls to stdout blocking the tokio
main event loop thread. On Linux, the active backend is `DirectToAnsi` (not Crossterm), so the
blocking call chain is:

```
mux.rs: output_poll_interval.tick()
  -> output_renderer.render_from_active_buffer()
    -> paint_buffer()
      -> OffscreenBufferPaintImpl::paint() / paint_diff()
        -> execute_all() dispatches to RenderOpPaintImplDirectToAnsi
          -> helpers::flush() -> locked_output_device.flush()  [BLOCKING]
```

The chain of events:

1. **Firehose**: Multiple child processes (claude, htop, gitui, bash) produce continuous terminal
   output.
2. **100 FPS trigger**: `output_poll_interval` ticks every 10ms, and `active_had_output` becomes
   true up to 100 times per second.
3. **Synchronous rendering**: Each tick with output calls `paint()`, which does blocking
   `write_all()` and `flush()` to stdout via the `DirectToAnsi` backend.
4. **Terminal bottleneck**: The terminal emulator can't parse and render ANSI data as fast as we
   write it. The OS pipe buffer for stdout fills up.
5. **Runtime blocked**: The synchronous `flush()` blocks the thread. The entire async runtime halts
   — it cannot loop back around to poll `input_device.next()`.
6. **Trickle effect**: The terminal eventually renders a frame and drains some buffer. The OS
   unblocks the thread, tokio writes the next chunk, logs the action, and immediately blocks on
   stdout again. This explains why the log file keeps generating output while input is dead.

The `mio_poller` input thread (on its own OS thread) is healthy and queuing keystrokes into the
broadcast channel, but the tokio event loop is perpetually parked in a kernel syscall and never
reads them.

**Why `biased;` + `MissedTickBehavior::Skip` didn't help**: The problem isn't which select branch
wins — it's that the entire tokio task is stuck inside a synchronous write. No amount of async
priority tuning helps when the runtime thread itself is blocked.

### Verification Plan

Before making code changes, manually verify the hypothesis by proving two things:

#### Phase 0: Isolate `mio_poller` Logging (Completed)
- [x] Added `DEBUG_TUI_SHOW_MIO_POLLER` flag to `tui/src/tui/mod.rs`.
- [x] Updated the four `mio_poller` handlers (`dispatcher.rs`, `handler_receiver_drop.rs`, `handler_signals.rs`, `handler_stdin.rs`) to use this specific flag instead of the generic `DEBUG_TUI_SHOW_TERMINAL_BACKEND`.
This allows us to cleanly verify `mio_poller` health without being flooded by terminal rendering logs.

#### Phase 0.1: Isolate `DirectToAnsi` Logging (Completed)
- [x] Added `DEBUG_TUI_SHOW_DIRECT_TO_ANSI` flag to `tui/src/tui/mod.rs`.
- [x] Updated `direct_to_ansi/output/direct_to_ansi_paint_render_op_impl.rs` (8 sites) and
      `direct_to_ansi/input/input_device_public_api.rs` (1 site) to use this flag instead of the
      generic `DEBUG_TUI_SHOW_TERMINAL_BACKEND`.
This allows observing DirectToAnsi flush/write timing without noise from other subsystems.

#### Verification Results (Completed)

- [x] **mio-poller is healthy and receiving keystrokes during the freeze.**
  Ran `pty_mux_example`, triggered the freeze, and mashed keys. Analyzed the stripped logs using `ansifilter`.
  Found numerous `mio_poller thread: read bytes` entries accompanied by the exact parsed key events (e.g., `Keyboard(Plain { key: SpecialKey(Up) })`).
  This proves the `mio-poller` OS thread is actively reading and parsing keystrokes and sending them to the Tokio broadcast channel.

- [x] **The tokio thread is blocked in the stdout flush syscall.**
  Despite the input events being successfully forwarded to the PTY channel, the UI did not react.
  This proves the Tokio event loop is blocked on synchronous `stdout` writes and cannot process
  the channel's receive end.

- [x] **DirectToAnsi flush logging confirms burst-then-gap blocking pattern.**
  After fixing the flush dispatch (Phase 3) and enabling `DEBUG_TUI_SHOW_DIRECT_TO_ANSI`, the log
  shows 241 `"direct_to_ansi: ✅ Succeeded"` flush entries with a clear blocking pattern:
  - **Bursts**: rapid clusters of flushes spaced 5-11 lines apart (stdout buffer has room).
  - **Gaps**: 34-97 line gaps between clusters (flush blocks on saturated stdout pipe).
  - **During gaps**: keyboard input events arrive and get logged by mio-poller, but the tokio
    event loop is parked in `flush()` and cannot consume them.
  - **Smoking gun**: a 79-line gap (lines 2536-2615) contains multiple
    `"Received input event: Keyboard(...)"` entries sandwiched between flush calls.

**Conclusion:** Hypothesis validated. We must proceed to implement the Render Budget.

### Implementation Plan

#### Phase 1: Decouple PTY Polling from Screen Rendering

The key insight: keep polling PTYs every 10ms so OffscreenBuffers stay fresh (critical for instant
switching), but throttle actual screen painting to ~30 FPS.

- [ ] Add a `last_render_time: Instant` field to track when the screen was last painted.
- [ ] In the output poll handler, continue calling `poll_all_processes()` on every tick.
- [ ] Gate `render_from_active_buffer()` behind a render budget: only paint when `active_had_output`
      is true AND at least ~33ms have elapsed since the last render.
- [ ] This eliminates stdout flooding while preserving buffer freshness for instant process
      switching.

#### Phase 2: Fix Status Bar Index-Out-of-Bounds Panic

- [x] `OutputRenderer::composite_status_bar_into_buffer()` used `self.terminal_size` to index into
      the `OffscreenBuffer`, but the buffer is `STATUS_BAR_HEIGHT` rows shorter (reserved by
      `Process::new()`). This caused a panic when the column or row index exceeded the buffer's
      actual dimensions.
- [x] Fixed by using `ofs_buf.window_size` instead of `self.terminal_size` in
      `composite_status_bar_into_buffer()` and `paint_buffer()`.

#### Phase 3: Fix Flush/Clear Dispatch Bypassing `TERMINAL_LIB_BACKEND`

- [x] `OffscreenBufferPaintImpl::paint()` and `paint_diff()` in
      `offscreen_buffer/paint_impl.rs` hardcoded `PaintRenderOpImplCrossterm.flush()` and
      `.clear_before_flush()`, bypassing the `TERMINAL_LIB_BACKEND` dispatch. On Linux
      (where `DirectToAnsi` is selected), the render ops were correctly dispatched via
      `execute_all()` but the final flush always went through the Crossterm path.
- [x] Fixed by adding `match TERMINAL_LIB_BACKEND` dispatch to all three call sites
      (`clear_before_flush` in `paint()`, `flush` in `paint()`, `flush` in `paint_diff()`),
      routing to `RenderOpPaintImplDirectToAnsi` when appropriate.

#### Phase 4: Rename Misleading Type

- [x] Rename `OffscreenBufferPaintImplCrossterm` to `OffscreenBufferPaintImpl` across the codebase,
      since it is the sole backend-agnostic implementation of the `OffscreenBufferPaint` trait (the
      actual backend dispatch happens inside `execute_all()` via `TERMINAL_LIB_BACKEND`).

#### Phase 5: Non-blocking Notifications Behind Debug Flag (Completed)

- [x] Add `DEBUG_TUI_SHOW_PTY_MUX_NOTIFICATIONS` const (default `true`) to `tui/src/tui/mod.rs`.
- [x] Gate `show_notification()` in `notification.rs` behind the flag.
- [x] Wrap the `Notification::new()...show()` call in `std::thread::spawn` so it doesn't block
      the caller (synchronous D-Bus IPC currently blocks the tokio event loop on every keystroke).
- [x] Renamed `show_notification` to `show_notification_non_blocking` to clearly indicate to callers
      that the function will not block the async runtime.

#### Phase 6: Remove Unnecessary Screen Clear on Process Switch

- [ ] Remove the `clear_screen()` + `cursor_position(0,0)` + `flush()` block in
      `input_router.rs` (~lines 80-94) before `process_manager.switch_to()`. The new process's
      buffer will be fully painted by `render_from_active_buffer()` on the next output poll tick.
      The clear just adds flicker.

#### Phase 7: Remove `process::exit(0)` in Cleanup

- [ ] Remove `std::process::exit(0)` in `mux.rs` (~line 403). Keep the logging/warning for slow
      cleanup but let the process exit normally through Drop impls so resources are cleaned up
      properly.

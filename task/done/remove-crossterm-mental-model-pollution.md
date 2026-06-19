# Task: Remove Crossterm Mental Model Pollution from RenderOps

## Overview

- **Goal**: Decouple Terminal Lifecycle/Global State management from Terminal Rendering
  operations.
- **Problem**: The `RenderOpCommon` enum contains variants like `EnterRawMode`,
  `ExitRawMode`, `ShowCursor`, `HideCursor`, `EnterAlternateScreen`, etc. This is a
  vestigial remain from when the project relied heavily on Crossterm's `Command` queue
  model.
  - The Compositor has to explicitly ignore these variants because they don't apply to the
    2D layout domain.
  - Lifecycle operations like `EnterRawMode` trigger OS-level syscalls
    (`tcgetattr`/`tcsetattr`) which shouldn't happen midway through a buffered rendering
    flush loop.
  - UI components shouldn't be responsible for emitting global environment changes.

### Proposed Architecture

**1. Clean up `RenderOpCommon`:** Strip all non-drawing operations from the enum. Remove
`EnterRawMode`, `ExitRawMode`, `EnterAlternateScreen`, `ExitAlternateScreen`,
`EnableMouseTracking`, `DisableMouseTracking`, `ShowCursor`, `HideCursor`,
`EnableBracketedPaste`, `DisableBracketedPaste`.

**2. Introduce `TerminalModeController` Trait:** Create a new `TerminalModeController`
trait and implement it for `OutputDevice`. This provides an ergonomic API to explicitly
control global terminal states.

- For OS-level syscalls (like raw mode), this trait will delegate to the existing
  `terminal_raw_mode` low-level API (which preserves the panic-safety of `RawModeGuard`).
- For ANSI mode changes (like hiding the cursor or switching to alternate screen), it will
  write directly to the output using `crossterm` commands or `AnsiSequenceGenerator`
  bytes.

**3. Refactor `pty_mux`**: Currently, `pty_mux` explicitly injects
`RenderOpCommon::HideCursor` into its render stream _every single frame_. Since `pty_mux`
manages its own virtual block cursor directly in its `OffscreenBuffer`, the native
terminal cursor is unnecessary. Under the new model, we will remove this frame-by-frame
injection and instead just rely on the global startup sequence (or `pty_mux`
initialization) to hide the native cursor once using the new `TerminalModeController` API.

## Implementation plan

### Phase 1: Define `TerminalModeController` API

- [x] Create the `TerminalModeController` trait in
      `tui/src/core/terminal_io/output_device.rs`.
- [x] Implement `TerminalModeController` for `OutputDevice`, delegating raw mode calls to
      `crate::terminal_raw_mode` and implementing ANSI sequence writing for other modes.
- [x] Create a PTY integration test
      (`tui/src/core/terminal_io/backend_compat_tests/terminal_mode_pty_test.rs`) to
      verify `TerminalModeController` methods.
  - Test will run in an isolated PTY process.
  - Test will execute mode methods on `OutputDevice`.
  - Test will capture raw ANSI bytes and assert they match exactly the expected sequences
    (e.g. `\x1b[?1049h` for alternate screen).
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/terminal_io/output_device.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/pty_terminal_mode_test.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/mod.rs`

### Phase 2: Purge `RenderOpCommon`

- [x] Delete the 10 defunct terminal mode variants from `RenderOpCommon`.
- [x] Remove the defunct variants from `RenderOpIR` and `RenderOpOutput` if applicable.
- [x] Remove the matching arms for these variants from `PaintRenderOpImplCrossterm` and
      `RenderOpPaintImplDirectToAnsi`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/tui/terminal_lib_backends/render_op/render_op_common.rs`
  - [x] `tui/src/tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs`
  - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_paint_render_op_impl.rs`
  - [x] `tui/src/tui/terminal_lib_backends/render_op/render_op_common_ext.rs`
  - [x] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
  - [x] `tui/src/tui/terminal_lib_backends/raw_mode_backend.rs`
  - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/debug.rs`
  - [x] `tui/src/tui/terminal_lib_backends/crossterm_backend/debug.rs`

### Phase 3: Refactor Multiplexer & App Startup

- [x] Update `pty_mux` (`output_renderer.rs`) to remove `RenderOpCommon::HideCursor` from
      its render queue.
- [x] Ensure app startup/shutdown sequences (`TerminalWindow::main_event_loop`, etc.) use
      the new `TerminalModeController` API instead of emitting `RenderOp`s.
  - Built `FullScreenTuiModeGuard` for panic-safe RAII teardown.
  - Refactored `OutputDevice::setup_full_screen_tui` and `teardown_full_screen_tui`.
- [x] Update `tui/src/lib.rs` documentation:
  - [x] Replace `enable_raw_mode()` code examples with `output_device.enter_raw_mode()`.
  - [x] Add a sub-section on `TerminalModeController` under "Rendering and painting".
  - [x] Verify the `RenderOpCommon` doc-link and its rustdocs still make sense.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/pty/pty_mux/output_renderer.rs`
  - [x] `tui/src/core/terminal_io/output_device.rs`
  - [x] `tui/src/tui/terminal_window/main_event_loop.rs`
  - [x] `tui/src/lib.rs`
  - [x] `task/remove-crossterm-mental-model-pollution.md` (Update task file itself if
        needed)

### Phase 4: Code Quality & Final Verification

- [x] Run comprehensive code quality checks (`./check.fish --full`) to ensure full
      cross-platform compatibility.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.

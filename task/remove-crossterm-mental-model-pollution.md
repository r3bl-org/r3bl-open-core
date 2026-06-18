# Task: Remove Crossterm Mental Model Pollution from RenderOps

## 1. Overview

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
  - **Historical Context**: The compositor used to explicitly track these states (see code
    below in `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`), but
    this was removed during the `OfsBufVT100` refactor, leaving behind empty match arms.
    This proves these variants no longer belong in the rendering layer.
    ```rust
    // Old code (removed):
    // ...
    // These operations update the OffscreenBuffer's terminal mode state while also
    // being executed by the terminal backend to affect actual terminal behavior.
    RenderOpCommon::EnterRawMode => {
        ofs_buf.terminal_mode.raw_mode = RawModeState::Enabled;
    }
    RenderOpCommon::ExitRawMode => {
        ofs_buf.terminal_mode.raw_mode = RawModeState::Disabled;
    }
    RenderOpCommon::EnterAlternateScreen => {
        ofs_buf.terminal_mode.alternate_screen = AlternateScreenState::Active;
    }
    RenderOpCommon::ExitAlternateScreen => {
        ofs_buf.terminal_mode.alternate_screen = AlternateScreenState::Inactive;
    }
    RenderOpCommon::EnableMouseTracking => {
        ofs_buf.terminal_mode.mouse_tracking = MouseTrackingState::Enabled;
    }
    RenderOpCommon::DisableMouseTracking => {
        ofs_buf.terminal_mode.mouse_tracking = MouseTrackingState::Disabled;
    }
    RenderOpCommon::EnableBracketedPaste => {
        ofs_buf.terminal_mode.bracketed_paste = BracketedPasteState::Enabled;
    }
    RenderOpCommon::DisableBracketedPaste => {
        ofs_buf.terminal_mode.bracketed_paste = BracketedPasteState::Disabled;
    }
    // ...
    ```

## 2. Proposed Architecture

### Clean up `RenderOpCommon`

Strip all non-drawing operations from the enum.

- **Remove**: `EnterRawMode`, `ExitRawMode`, `EnterAlternateScreen`,
  `ExitAlternateScreen`, `EnableMouseTracking`, `DisableMouseTracking`, `ShowCursor`,
  `HideCursor`, `EnableBracketedPaste`, `DisableBracketedPaste`.
- **Keep**: Pure visual instructions only (e.g., `PrintText`, `SetFgColor`, `SetBgColor`,
  `MoveCursorPosition`, `ClearScreen`).

### Introduce a Terminal Lifecycle API

Move these state changes to explicit, synchronous function calls on the backend or output
device (e.g., a `TerminalState` trait).

```rust
// Application Startup
backend.enter_raw_mode()?;
backend.enter_alternate_screen()?;
backend.hide_cursor()?;

// ... Event loop and purely visual RenderOps happen here ...

// Application Shutdown
backend.show_cursor()?;
backend.exit_alternate_screen()?;
backend.exit_raw_mode()?;
```

### Refactor Multiplexer (`pty_mux`)

Currently, `pty_mux` explicitly injects `RenderOpCommon::HideCursor` into the render
stream to suppress the native cursor. Under the new model, it should simply call
`backend.hide_cursor()` upon initialization, as it inherently takes over the global
terminal environment.

## 3. Scope of Work (Checklist)

- [ ] Audit `RenderOpCommon` and identify all lifecycle/state variants to be removed.
- [ ] Create a new API surface (e.g. `TerminalLifecycle` or methods on
      `OutputDevice`/Backend) for explicit terminal state control.
- [ ] Implement the new synchronous syscall/ANSI sequence triggers for `direct_to_ansi`
      and `crossterm` backends.
- [ ] Remove the defunct variants from `RenderOpCommon`, `RenderOpIR`, and
      `RenderOpOutput`.
- [ ] Update `pty_mux` to use the new synchronous cursor hiding API.
- [ ] Update app startup/shutdown sequences across the workspace to use the new lifecycle
      API instead of emitting `RenderOp`s.
- [ ] Run comprehensive code quality checks (`./check.fish --full`) to ensure full
      cross-platform compatibility.

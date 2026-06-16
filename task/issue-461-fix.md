# Task: Fix Cursor Display Issues in Component Pipeline (Issue #461)

# 1. Overview

- **Goal**: Fix the bug where the native terminal cursor is incorrectly and permanently
  shown in all TUI applications using the component pipeline, while preserving virtual
  cursors for `pty_mux`.
- **Context**: [Issue #461](https://github.com/r3bl-org/r3bl-open-core/issues/461),
  introduced by commit `f33c6a3c`.

# 2. Problem Space & Constraints

The original author of `f33c6a3c` wanted to ensure `pty_mux` forcefully hid the terminal
cursor. To achieve this, they modified the global rendering engine (`paint_impl.rs`) to
accept a visibility parameter and injected visibility commands into the render stream.

This inadvertently broke the global component pipeline (`paint.rs`), which also calls
`paint_impl.rs`. To fix the resulting compile error, the author lazily extracted
`OffscreenBuffer::ansi_parser_support.cursor_visibility` and passed it down. Because the
VT100 standard dictates this defaults to `Visible`, the global pipeline began spamming
`ShowCursor` on every frame, overriding the apps' initial startup commands to hide the
cursor.

## The Exact Origin of the Bug

Here are the exact files and lines where the bug was born:

### 1. The Extraction: `tui/src/tui/terminal_lib_backends/paint.rs`

The author modified the orchestration functions to reach into the `OffscreenBuffer` and
extract the default visibility state:

- In `perform_diff_paint` (Lines 65-66):

  ```rust
  fn perform_diff_paint(ofs_buf: &OffscreenBuffer, /* ... */) {
      let cursor_visibility = ofs_buf.ansi_parser_support.cursor_visibility;
  ```

- In `perform_full_paint` (Lines 112-113):
  ```rust
  fn perform_full_paint(ofs_buf: &OffscreenBuffer, /* ... */) {
      let cursor_visibility = ofs_buf.ansi_parser_support.cursor_visibility;
  ```

(They then passed this extracted state down into the backend calls below).

### 2. The Injection: `tui/src/tui/terminal_lib_backends/offscreen_buffer/paint_impl.rs`

The author added code to the final rendering step to dynamically push `ShowCursor` /
`HideCursor` commands to the terminal based on the state passed down from `paint.rs`.

- In `paint()` (Lines 111-114):

  ```rust
  // Apply cursor visibility state.
  match cursor_visibility {
      CursorVisibilityState::Visible => render_ops.push(RenderOpCommon::ShowCursor),
      CursorVisibilityState::Hidden => render_ops.push(RenderOpCommon::HideCursor),
  }
  ```

- In `paint_diff()` (Lines 157-160):
  ```rust
  // Apply cursor visibility state.
  match cursor_visibility {
      CursorVisibilityState::Visible => render_ops.push(RenderOpCommon::ShowCursor),
      CursorVisibilityState::Hidden => render_ops.push(RenderOpCommon::HideCursor),
  }
  ```

By extracting the `Visible` default in `paint.rs` and injecting the `ShowCursor` command
into the stream in `paint_impl.rs`, they unknowingly forced the cursor visible on every
frame for the entire component pipeline.

## Hard Constraints & Core Intent

1. **`pty_mux` must continue to work**: The terminal emulator relies on hiding the real
   terminal cursor so it can draw multiple _virtual cursors_ via styling.
2. **Component pipeline must be decoupled**: Standard TUI applications do not need
   continuous cursor visibility injection. They hide the cursor once at initialization and
   expect the render pipeline to leave it alone.
3. **No needless compositor changes**: Modifying `compositor_render_ops_to_ofs_buf.rs` is
   a non-starter and a distraction.

# 3. Proposed Design: Clean Reversion and Localized Injection

Instead of hacking around the damage done by `f33c6a3c`, we will completely revert its
architectural mistake. We will remove cursor visibility logic from the shared rendering
engine and isolate it entirely within `pty_mux`.

- **How it works**:
  1. Remove the `cursor_visibility` argument and injection logic from `paint_impl.rs`.
  2. Remove the state extraction from `paint.rs`.
  3. Modify `pty_mux`'s custom renderer (`output_renderer.rs`) to manually push a
     `RenderOpCommon::HideCursor` into its _own_ stream before calling the paint engine.
- **Why it's correct**: It completely decouples the component pipeline from PTY parser
  state, returning it to the flawless state it was in before the bug existed, while
  cleanly satisfying `pty_mux`'s requirement to hide the physical cursor.

# 4. Implementation Plan

## Phase 1: Reverting the Global Pipeline

- [x] In `tui/src/tui/terminal_lib_backends/offscreen_buffer/paint_impl.rs`:
  - Remove the `cursor_visibility` argument from `paint()` and `paint_diff()`.
  - Remove the `match cursor_visibility` blocks that push `ShowCursor`/`HideCursor`.
- [x] In `tui/src/tui/terminal_lib_backends/paint.rs`:
  - Remove `let cursor_visibility = ofs_buf.ansi_parser_support.cursor_visibility;` from
    `perform_full_paint` and `perform_diff_paint`.
  - Remove the `cursor_visibility` argument passed to the backend calls.
- [x] **Mandatory manual review**:
  - [x] `paint_impl.rs`
  - [x] `paint.rs`

## Phase 2: Localizing the Fix to PTY Mux

- [x] In `tui/src/core/pty/pty_mux/output_renderer.rs`:
  - In `paint_buffer()`, change `let render_ops` to `let mut render_ops`.
  - Push `RenderOpCommon::HideCursor` into `render_ops` before calling
    `ofs_buf_paint_impl.paint()`.
  - Remove the `CursorVisibilityState::Hidden` argument from the `paint()` call.
- [x] **Mandatory manual review**:
  - [x] `output_renderer.rs`

## Phase 3: Integration & Testing

- [x] Run `cargo run --example tui_apps` and select `ex_app_no_layout` to verify the
      terminal cursor remains hidden.
- [x] Run `./check.fish --check` to verify no compilation errors.
- [x] Run `./check.fish --test` to ensure tests pass.
- [x] **Mandatory manual review**:
  - [x] Compile and Test success visually verified.

## Phase 4: How to fix the underlying problem

The underlying architectural problem is **Dual-Use**. Currently, `OffscreenBuffer` serves
two completely different masters:

1. **The Component Pipeline**: Uses it as a simple 2D pixel canvas. `ansi_parser_support`
   is useless dead weight.
2. **The PTY Multiplexer**: Uses it as a full VT100 emulator screen where parsing state
   must be tracked alongside the pixels.

To permanently fix this, we will cleanly separate the ANSI parser state from the generic
UI canvas using a Composition (Structural Split) approach.

### The Composition Approach

Physically break `OffscreenBuffer` into two distinct structs:

1. **`OffscreenBuffer`**: Keep the existing name, but strip it down to a pure struct
   containing _only_ the 2D grid (`PixelCharLines`), `cursor_pos`, and `window_size`.
2. **`VT100TerminalState`**: A new struct containing an `OffscreenBuffer` **PLUS** the
   `ansi_parser_support` and `alt_screen_support`.

- Standard TUI components and `paint.rs` continue using `OffscreenBuffer` (meaning minimal
  renaming is needed in the component pipeline).
- The ANSI parser and `pty_mux` would operate on `VT100TerminalState`. When `pty_mux` is
  ready to paint, it simply extracts its internal `OffscreenBuffer` and hands it to the
  generic `paint.rs` layer. **Pros**: Purity. UI components stop carrying heavy VT100
  baggage (like an entire alternate screen memory allocation). The compiler naturally
  enforces the boundary because `OffscreenBuffer` physically lacks the parser state.
  Keeping the name `OffscreenBuffer` drastically reduces the refactoring scope.
- **Future-Proofing (Scrollback Buffers)**: Scrollback memory is inherently a Terminal
  Emulator feature, not a generic UI feature. If a `scroll_back_buffer` was added to the
  monolithic `OffscreenBuffer`, every single UI component (like a simple `Button` or
  `DialogBox`) would wastefully allocate memory for thousands of lines of hidden history.
  By using this split, `VT100TerminalState` becomes the perfect, dedicated home for
  historical scrollback data, while `OffscreenBuffer` stays small and fast representing
  _only_ the currently visible screen pixels.

### Codebase Audit for Structural Split

By keeping the name `OffscreenBuffer` for the core grid, the entire standard TUI engine
(Layouts, Dialogs, Editors, Buttons) remains completely untouched, but drops all the
memory overhead. Here is the exact list of places we would need to migrate to
`VT100TerminalState`:

1. **The Struct Definitions (`ofs_buf_core.rs`)**
   - Remove `ansi_parser_support` and `alt_screen_support` from `OffscreenBuffer`.
   - Create `VT100TerminalState` containing an `OffscreenBuffer` plus the two VT100
     fields.

2. **The VT100 Implementations (`vt_100_ansi_impl/` directory)**
   - Move `impl Vt100TerminalOps for OffscreenBuffer` to
     `impl Vt100TerminalOps for VT100TerminalState`.
   - When the implementation needs to mutate pixels, it will access `self.ofs_buf.buffer`
     instead of `self.buffer`.

3. **The ANSI Parser (`performer.rs` & `ansi_parser_public_api.rs`)**
   - `AnsiToOfsBufPerformer` (the engine that interprets ANSI escape codes) currently
     holds an `OffscreenBuffer`.
   - Update it to hold a `VT100TerminalState` instead.

4. **The PTY Multiplexer (`pty_mux/` module)**
   - `ProcessManager`: Currently tracks an `OffscreenBuffer` for each running shell.
     Update it to track a `VT100TerminalState`.
   - `OutputRenderer`: Will accept the `VT100TerminalState`, do its virtual cursor
     compositing, and then pass the inner `OffscreenBuffer` to the generic
     `paint_buffer()` engine.

# Task: Improve Immature VT100 Performer (Resolve Visual Artifacts)

## Overview

The terminal multiplexer currently displays significant visual artifacts, overlapping
text, flickering, and remnant states from exiting applications.

While the underlying byte-to-token parsing is powered by the production-grade **`vte`**
crate (used by Alacritty and WezTerm), our performer implementation
(`AnsiToOfsBufPerformer` in `tui/src/core/ansi/vt_100_pty_output_parser/performer.rs`) and
the associated `OffscreenBuffer` are highly basic shims that explicitly ignore or lack
support for critical terminal control sequences.

This task involves maturing the performer to correctly process display control, alternate
buffers, and cursor visibility events.

---

## Technical Gaps

### 1. Erase Display (`ED`) and Erase Line (`EL`) are Ignored

- **Location**: `tui/src/core/ansi/vt_100_pty_output_parser/performer.rs` (around line
  530)
- **Current Behavior**: Discards `ED_ERASE_DISPLAY` (`CSI J`) and `EL_ERASE_LINE` (`CSI
  K`), erroneously assuming that TUI applications will simply repaint themselves over
  unchanged areas.
- **The Issue**: Any text from previous frames or long command prompts remains visible
  under shorter strings, causing overlapping text and visual corruption.
- **Solution**:
  - Implement `EL_ERASE_LINE` to clear cells relative to the active cursor position (e.g.,
    from the cursor to the end of the line, start of the line, or the entire line).
  - Implement `ED_ERASE_DISPLAY` to clear sections of the buffer (above/below the cursor
    or the entire screen).

### 2. Alternate Screen Buffer (`?1049h` / `?1049l`) is Unhandled

- **Location**:
  `tui/src/core/ansi/vt_100_pty_output_parser/operations/vt_100_shim_mode_ops.rs`
- **Current Behavior**: Ignores private modes other than `AutoWrap`.
- **The Issue**: Full-screen applications (`htop`, `gitui`, `hx`, `less`) run directly on
  the primary screen buffer. Upon exiting, the full-screen terminal paint ruins the `bash`
  prompt scrollback context instead of disappearing cleanly.
- **Solution**:
  - Add support for `?1049h` and `?1049l` in `vt_100_shim_mode_ops.rs`.
  - Maintain a dual-buffer system (primary and secondary/alternate) inside
    `OffscreenBuffer`. When alternate screen mode is active, paint operations should route
    to the alternate buffer, and switching back should restore the primary buffer.

### 3. Cursor Visibility (`?25h` / `?25l`) is Unhandled

- **Location**:
  `tui/src/core/ansi/vt_100_pty_output_parser/operations/vt_100_shim_mode_ops.rs`
- **Current Behavior**: Ignores private mode `25`.
- **The Issue**: TUI apps hide the cursor while redrawing to prevent cursor-drift
  flickering. Failing to honor this causes the cursor to flicker visibly and jump around
  the grid.
- **Solution**:
  - Add support for toggling cursor visibility inside `OffscreenBuffer`.

---

## Implementation Checklist

### Phase 1: Clear Display (`CSI J`) and Clear Line (`CSI K`) Support

- [x] **Research Existing Buffer APIs**: Check what clear/erase APIs are already exposed
      on `OffscreenBuffer` (e.g., filling regions with spaces).
- [x] **Implement Erase Line (`CSI K`)**:
  - Update `performer.rs` to dispatch `EL_ERASE_LINE` to a handling operation.
  - Implement clearing cells in the active row:
    - `0` (or default): Clear from cursor to end of line.
    - `1`: Clear from beginning of line to cursor.
    - `2`: Clear entire line.
- [x] **Implement Erase Display (`CSI J`)**:
  - Update `performer.rs` to dispatch `ED_ERASE_DISPLAY` to a handling operation.
  - Implement clearing rows relative to the cursor:
    - `0` (or default): Clear from cursor to end of screen.
    - `1`: Clear from beginning of screen to cursor.
    - `2`: Clear entire screen.
- [x] **Write Unit/Integration Tests**: Validate that clears cleanly replace target cells
      with empty space chars while retaining styles.
- [x] **Remove redundant rustdocs**: The same architecture diagram appeared in all the
      `operations/*ops.rs` files. Consolidate them into `mod.rs` and replace the redundant
      ASCII diagrams w/ a link pointing to the exact section in `mod.rs` w/ the diagram.
- [x] **Fix/Update Outdated Integration Tests**: Update
      `test_paint_after_clear_sequence_rendered` in `screen_operations_rendered.rs` to
      expect cells to be cleared (e.g. 'X' is cleared) instead of ignoring the clear
      operation.
- [x] **Use bounds_check module**: Replace all usize and code that might have off by one
      errors with the robust bounds_check module.
  - [x] Add new `RangeExt` trait to make range iteration ergonomic (and not have to "fall
        out" of our `bounds_check` domain into usize). Update rustdocs and
        `.agents/skills/check-bounds-safety/`
- [x] Make sure proper constants are used in this code, and not magic strings / hard coded
      literals.
- [x] **Implement End-to-End Conformance Tests**: Create `vt_100_test_clear_ops.rs` in
      `vt_100_pty_output_conformance_tests/tests/` and register it in `mod.rs` to verify
      the full pipeline from raw ANSI bytes (`CSI J` and `CSI K` sequences) to offscreen
      buffer state: - Verify parameter defaults (missing arguments default to 0). - Verify
      erase display (ED) modes 0, 1, 2. - Verify erase line (EL) modes 0, 1, 2.
- [x] **Mandatory manual review**: Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x]
    `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/mod.rs`
  - [x]
    `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_clear_ops.rs`

### Phase 2: Dual Screen Buffers (Alternate Screen)

- [ ] **Refactor `OffscreenBuffer` to support Alternate Buffer**:
  - Introduce an `active_buffer` flag and a secondary grid buffer (`alt_buffer:
    Option<PixelCharLines>`) inside `OffscreenBuffer`.
  - Ensure cursor states are stored independently for both primary and alternate buffers.
- [ ] **Implement Mode Toggle**:
  - Update `vt_100_shim_mode_ops.rs` to process `1049` mode.
  - Toggle the active buffer on `SM ?1049h` (Set Mode) and `RM ?1049l` (Reset Mode).
- [ ] **Write Alternate Buffer Tests**: Verify switching buffers preserves primary buffer
      scrollback and isolates alternate buffer changes.

### Phase 3: Cursor Visibility & Secondary Private Modes

- [ ] **Cursor Visibility toggle (`?25h` / `?25l`)**:
  - Add an `is_cursor_visible` boolean flag to `AnsiParserSupport` (not the main buffer
    grid).
  - Feed cursor visibility state into the main rendering engine: update
    `OffscreenBufferPaintImpl::paint_diff` (Stage 4) to emit `RenderOpCommon::HideCursor`
    or `ShowCursor` based on this state.
- [ ] **Handle other critical modern TUI private modes**:
  - Add fallback warning suppression or shims for mouse events (`?1000h`/`?1006h`) and
    bracketed paste (`?2004h`).

### Phase 4: Integration & Manual Verification

- [ ] **Run Multiplexer Example**: Execute `cargo run --example pty_mux_example` with
      `hx`, `less`, `htop`, and `gitui`.
- [ ] **Verify Visual Quality**:
  - Confirm tab switching, editor scrolling, and process exits are clean and leave zero
    visual artifacts.
  - Check the output logs with `DEBUG_TUI_PTY_MUX=true` to verify no more warning entries
    exist for `CSI J`, `CSI K`, or `CSI ?1049h/l`.

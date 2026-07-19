<!-- cspell:words ofsbuf memmoves -->

_Architecture Plan: Continuous 2D Buffer for Panning & Variable Line Widths_

# 1. The Problem and Use Cases

When building a terminal multiplexer, we must handle two fundamentally different types of
terminal applications:

1. **Alternate Screen Apps (aka full TUI):** Apps like `vim`, `htop`, or `less` use the
   "alternate screen" (`CSI ? 1049 h`). They draw a fixed-size UI that exactly matches the
   dimensions of the terminal window.
    - By design, there is no scrollback history; when content shifts, old data is
      permanently destroyed.
    - This "full TUI" use case is exactly why we have a fixed grid `OfsBuf` backed by a
      `Flat2DArray`. It allows for extremely fast, SIMD-optimized memory shifting
      (`copy_within_rows`) without heap allocations.
2. **Primary Screen Apps (CLI):** Standard command-line tools like `cat`, `grep`, `ls`, or
   just running a shell, via the virtual terminal emulator or `pty_mux` module. These apps
   print continuous streams of text. When output hits the bottom of the screen, the
   terminal must scroll down, preserving the old output in a "scrollback history" that the
   user can scroll up to view later.
    - **The Pre-Refactor Relationship:** `OfsBufVT100` is the VT-100 emulator/state
      machine which currently contains two separate data structures to handle CLI
      scrollback:
        1. `OfsBuf` (which acts purely as the fixed-size grid for the active screen) and
        2. a separate `ScrollbackBuffer` (which acts as the history).
    - Because they are two physically separate structures inside the emulator, whenever
      text scrolls off the top of the active `OfsBuf` grid, the system must manually
      extract those lines and push them into the `ScrollbackBuffer`.
    - Consequently, because these history lines are extracted directly from the fixed-size
      `OfsBuf`, the entire scrollback implementation is strictly bound by a fixed width
      (column size). This means long lines are permanently hard-wrapped or truncated to
      the viewport width the moment they scroll off the screen.

The `r3bl_tui` crate needs to support both of these domains flawlessly:

1. **Full TUI Domain (`TerminalWindow::main_event_loop()`):** Where we manipulate the
   entire screen natively in alternate mode. Note that these are separate from the other
   two entry points: `readline_async` and `choose`.
2. **`pty_mux` Domain (`PTYMuxBuilder::build()`):** Where we allow other programs to run
   in a virtual terminal emulator environment (e.g., one process running in a virtual
   tab).

However, for the `pty_mux` domain specifically, we also want to introduce a powerful new
feature: **Infinite 2D Panning**. If a CLI app prints lines that are wider than the
terminal window (e.g., a wide `ls -l` or a long `grep` match), we want users to be able to
pan horizontally to read the truncated text, rather than relying on awkward line wrapping.
This requires the terminal to natively support variable line widths.

## 1.1. The Current Scrollback Implementation & The New Goal

Currently, the `r3bl_tui` crate does not support both use cases cleanly, and panning is
not impossible (but it isn't possible on any existing terminal emulator or multiplexer
software).

We rely on a single fixed-size grid buffer called `OfsBuf`, backed by a 1D `Flat2DArray`.
This is perfect for Alternate Screen apps, but terrible for Primary Screen apps (in the
`pty_mux` context). To support CLI scrollback history, we bolt on a completely separate
struct called `ScrollbackBuffer` (which wraps a `VecDeque<PixelCharLine>`) in
`OfsBufVT100` struct (see details above).

This dual-state system causes massive friction:

- Because the active grid (`OfsBuf`) and the history (`ScrollbackBuffer`) are physically
  separated, whenever a CLI app prints a new line, the system must awkwardly extract the
  top row from `OfsBuf` and manually push it into the `ScrollbackBuffer`.
- To render the screen, the `OutputRenderer` must dynamically stitch the
  `ScrollbackBuffer` lines and the `OfsBuf` lines together on the fly.
- Most importantly, because `OfsBuf` is a rigid 1D `Flat2DArray`, **every single line is
  forced to exactly match the width of the viewport**. Variable line widths are
  impossible, making horizontal panning structurally impossible. This is not a `r3bl_tui`
  limitation - this is how all terminal emulators work.

## 1.2. The Design Approach: A Unified Mental Model

To unlock panning and variable line widths, we draft a **new concept in the world of
terminal emulators** - allowing primary mode applications to be able to pan (scroll
vertically and horizontally) through a **Continuous 2D Buffer**. Instead of splitting the
active screen and the scrollback history into two separate pieces, they should be one
unified, continuous list of lines.

This relies on what we call the **Canvas and Viewport concept**. As a CLI app (in a
`pty_mux` context) prints text, it simply appends lines to the bottom of the list. This
continuous list of lines acts as an infinite **canvas**. The "visible screen" is no longer
a physical 2D array; it is merely a **viewport** (a rectangular window) defined by a
`Size` and an `(x, y)` offset that slides over this canvas.

- **Vertical Panning** is just changing the `y` offset (scrolling up through history).
- **Horizontal Panning** is just changing the `x` offset (sliding right to view long
  lines).

However, introducing this new growable buffer (`OfsBufGrowable`) would have created a
massive code duplication problem. Both the new Growable buffer and the old Fixed buffer
represent a 2D grid with a cursor, and both must implement the exact same 20+ complex
VT-100 operations (e.g., `insert_lines`, `print_char`, `erase_in_display`).

# 2. The Solution: Dependency Injection and Generics

To avoid duplicating logic between the fixed and growable buffers, we can keep the
existing `OfsBuf` struct with its existing methods and logic, but instead of hardcoding
the backing store, we use DI to inject the backing store.

We need to introduce a new trait that abstracts the low-level memory operations (e.g.,
`get_row_mut()`, `shift_lines_up()`, etc.). Then we can implement this trait for both the
fixed-size `Flat2DArray` struct and the _new_ history-preserving `GrowableBuffer` struct.
Finally, we make `OfsBuf` generic over types with this trait bound
(`OfsBuf<S: OfsBufStorage>`).

Because this abstraction boundary is so low, `OfsBuf` absorbs 100% of the complex 2D grid
mathematics into itself rather than duplicating it across backends. It implements the
logic exactly once for converting (x,y) coordinates into safe memory bounds checks (e.g.,
`set_char()`, `get_char()`), calculating index offsets for splicing/shifting characters
within a line (e.g., `copy_chars_within_line()`, `fill_char_range()`), and generating
optimized pixel-by-pixel screen diffs (`diff()`).

```rust
// The Solution: Generics & Dependency Injection
pub struct OfsBuf<S: OfsBufStorage> {
    pub inner: S,           // The low-level memory operations
    pub cursor_pos: Pos,    // State lives alongside it natively!
}

impl<S: OfsBufStorage> OfsBuf<S> {
    fn apply_vt100_command(&mut self) {
        // NO BORROW CHECKER CONFLICTS:
        // Because this is a concrete struct, the compiler can perform
        // "Split Borrows". We can mutate `self.inner` (the grid memory)
        // while simultaneously reading from `self.cursor_pos`.
        self.inner.get_row_mut(self.cursor_pos.row_index);
    }
}
```

The `r3bl_tui` crate will now instantiate two specialized variants of `OfsBuf` using the
exact same VT-100 logic:

1. **`alternate_buffer: OfsBuf<Flat2DArray>`**: Used for TUI apps (`vim`, `htop`). Uses a
   contiguous 1D slice. Fast, fixed-size, with SIMD-optimized memory shifts
   (`copy_within_rows`), but no scrollback.
2. **`primary_buffer: OfsBuf<GrowableBuffer>`**: Used for CLI apps (`cat`, `ls`). Uses a
   `VecDeque` of allocated lines. As output pushes past the bottom of the screen, the
   `GrowableBuffer` simply increments its internal viewport offset, preserving the old
   lines in history. It natively supports variable line widths and 2D panning.

# 3. Implementation Steps

Here is the exact architecture to achieve zero duplication:

## Step A: The `OfsBufStorage` Trait

Extract the raw memory operations into a trait. This only requires operations that differ
based on the backing store.

```rust
pub trait OfsBufStorage: GetMemSize {
    fn get_width(&self) -> ColWidth;
    fn get_height(&self) -> RowHeight; // Height of the viewport

    // Viewport-relative indexing (row 0 is the top of the visible screen)
    fn get_row(&self, row: RowIndex) -> Option<&[PixelChar]>;
    fn get_row_mut(&mut self, row: RowIndex) -> Option<&mut [PixelChar]>;

    // Viewport panning operations for horizontal and vertical scrolling
    fn set_viewport_col_offset(&mut self, offset: ColIndex);
    fn set_viewport_row_offset(&mut self, offset: RowIndex);
    fn get_viewport_col_offset(&self) -> ColIndex;
    fn get_viewport_row_offset(&self) -> RowIndex;

    // -------------------------------------------------------------------------
    // VT-100 Line Shifting (In-memory data destruction)
    // Used by `IL` (Insert Line) and `DL` (Delete Line) or Margin Scrolls.
    // - Flat2DArray: Uses fast 1D SIMD copy_within_rows.
    // - GrowableBuffer: Rotates elements in place (O(N) pointer swaps).
    // -------------------------------------------------------------------------
    fn shift_lines_up(&mut self, row_range: Range<RowIndex>, amount: Length, empty_char: PixelChar);
    fn shift_lines_down(&mut self, row_range: Range<RowIndex>, amount: Length, empty_char: PixelChar);

    // -------------------------------------------------------------------------
    // Terminal Scrolling (Viewport panning)
    // Triggered by `\n` at the bottom of the screen (unrestricted scroll).
    // - Flat2DArray (Alternate Screen): There is NO SCROLL in the alternate screen.
    //   This just degrades to `shift_lines_up(0..height)`.
    // - GrowableBuffer (Primary Screen): Appends a new line to the VecDeque and
    //   pans the viewport down, natively preserving the old top line in history!
    // -------------------------------------------------------------------------
    fn scroll_up(&mut self, amount: Length, empty_char: PixelChar);

    fn fill_all(&mut self, empty_char: PixelChar);
}
```

## Step B: The Generic `OfsBuf`

`OfsBuf` becomes generic over the `OfsBufStorage`. We provide a default generic so we
don't break existing UI compositor code (`OfsBufPaint`).

```rust
pub struct OfsBuf<S: OfsBufStorage = Flat2DArray<PixelChar>> {
    pub store: S,
    pub cursor_pos: Pos,
}
```

## Step C: Implement `Canvas` ONCE

Instead of writing `ofs_buf_impl.rs` and `ofs_buf_growable_impl.rs`, we implement `Canvas`
generically. All VT-100 math is written only one time.

```rust
impl<S: OfsBufStorage> Canvas for OfsBuf<S> {
    fn move_cursor_up(&mut self, how_many: Length) {
        self.cursor_pos.row_index = self.cursor_pos.row_index.saturating_sub(how_many);
    }

    fn insert_lines(&mut self, how_many: Length, scroll_region: Range<RowIndex>) {
        self.store.shift_lines_down(scroll_region, how_many, PixelChar::Spacer);
    }
    // ... all other Canvas methods
}
```

## Step D: The `OfsBufVT100` State Machine

The parser holds both the primary and alternate buffers simultaneously. This is critical
because switching to the alternate screen must not destroy the primary screen and its
scrollback history. Instead of `dyn Canvas`, we use a lightweight enum `ActiveBufferMut`
to abstract over the generic `OfsBuf<S>` variants.

```rust
pub enum ActiveBufferMut<'a> {
    Primary(&'a mut OfsBuf<GrowableBuffer>),
    Alternate(&'a mut OfsBuf<Flat2DArray<PixelChar>>),
}

// ActiveBufferMut implements the common OfsBuf methods (try_set, get_width, shift_lines_up)
// and forwards them to the active variant.

impl OfsBufVT100 {
    pub fn active_buf_mut(&mut self) -> ActiveBufferMut<'_> {
        match self.terminal_mode.active_screen_buffer {
            ActiveScreenBuffer::Primary => ActiveBufferMut::Primary(&mut self.primary_buffer),
            ActiveScreenBuffer::Alternate => ActiveBufferMut::Alternate(&mut self.alternate_buffer),
        }
    }
}
```

## Step E: Rendering and Compositor Stitching

Currently, `OutputRenderer` explicitly stitches `ScrollbackBuffer` and `OfsBuf`. With
`GrowableBuffer` natively handling history, `OfsBufVT100` no longer needs a separate
`ScrollbackBuffer`.

- `OutputRenderer` will be updated to read viewport-relative rows directly from the active
  buffer.
- `OfsBuf::diff` is highly optimized with SIMD `copy_within_rows` for `Flat2DArray`. We
  must scope `diff` to `impl OfsBuf<Flat2DArray<PixelChar>>` so it doesn't break. The
  compositor UI pipeline (`OfsBufPaint`) will continue to composite _into_ a generic
  `OfsBuf<Flat2DArray<PixelChar>>` so rendering works seamlessly without changes to
  `diff`.

## Step F: Dims (Viewports and Composite Geometry)

Currently, `OfsBufStorage` handles 4 scattered getters for viewport management
(`get_width`, `get_height`, `get_viewport_col_offset`, `get_viewport_row_offset`). These
are semantically confusing (e.g. `get_width` returning the viewport width, not the
underlying buffer's physical allocation).

We will introduce a `Dims` primitive in `tui/src/core/coordinates/buffer_coords/dims.rs`:

```rust
pub struct Dims {
    pub size: Size,
    pub pos: Pos,
}
```

This enables ergonomic viewport queries and math (e.g. `let d = dims(pos + size)` via
`Add` trait overloading). `OfsBufStorage` will be simplified to
`fn get_viewport(&self) -> Dims`.

## Step G: Flat2DArray SIMD Refactoring and Cleanup

Removed dead API surface and regressed loops from `Flat2DArray` to re-align with SIMD
optimization patterns:

- Restored `shift_rows_up` and `shift_rows_down` to `Flat1DSimdMut` using SIMD block
  rotations (`copy_within`), resolving a regression in
  `OfsBufStorage::shift_lines_in_range` that was using slow manual row-by-row iteration.
- Deleted `range_validation.rs` and unused scalar getter methods (`get_height`, `try_get`,
  `try_get_mut`, `try_set`) that were cluttering the API.
- Enforced indexing traits (`Index` and `IndexMut`) as the singular path for single-cell
  coordinate access.

## Step H: Clean up the API

### Task: OfsBuf Cleanup, Simplification, and Encapsulation

#### Overview

This task merges three main efforts to improve the `OfsBufVT100` struct:

1. **Phase 1: Completing Option A**: Deleting `new_empty` from `OfsBuf` and instantiating
   via `OfsBuf::new(Flat2DArray::new_empty(...))` by resolving compile errors in tests and
   backend files.
2. **Phase 2: Simplifying OfsBufVT100**: Removing redundant delegating methods that simply
   wrap `self.active_buf()` or `self.active_buf_mut()`.
3. **Phase 3: Encapsulation**: Ensuring that internal fields like `primary_buffer`,
   `alternate_buffer`, `parser_global_state`, and `terminal_mode` are strictly private,
   providing safe getter/setter methods.

#### Phase 1: Completing Option A (Compile Fixes)

- [x] Add `use crate::Flat2DArray;` imports to:
    - `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
    - `tui/src/tui/terminal_lib_backends/ofs_buf/pixel_char.rs`
    - `tui/src/tui/terminal_lib_backends/ofs_buf/paint_impl.rs`
    - `tui/src/tui/terminal_lib_backends/ofs_buf/test_fixtures_ofs_buf.rs`
- [x] In `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs` (tests), update
      calls to `state.get_row_with_scrollback(...)` to
      `state.active_buf().get_row_with_scrollback(...)`.
- [x] Verify the codebase compiles and all tests pass.

#### Phase 2: Simplify `OfsBufVT100`

- [x] Add `get_char` method to the `OfsBufOpsVT100` trait (in `ofs_buf_vt_100.rs`) with a
      default implementation mapping to `get_row`.
- [x] Implement `get_char` for `OfsBuf<GrowableBuffer>` and
      `OfsBuf<Flat2DArray<PixelChar>>` in `ofs_buf_vt_100.rs`.
- [x] Remove redundant/intermediary methods on `OfsBufVT100` in `ofs_buf_vt_100.rs`:
    - `get_window_size`
    - `get_height`
    - `get_row`
    - `get_row_mut`
    - `get_char`
    - `get_cursor_pos`
    - `set_cursor_pos`
    - `update_cursor_pos`
- [x] Update call sites in
      `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/*.rs` (e.g.,
      `vt_100_impl_cursor_ops.rs`, `vt_100_impl_scroll_ops.rs`, etc.) to call these
      methods on `active_buf()` or `active_buf_mut()` instead of `self`.
- [x] Update call sites in other files (such as `output_renderer.rs`,
      `keyboard_command.rs`, etc.) to call these methods on `active_buf()` /
      `active_buf_mut()`.
- [x] Update any tests in `ofs_buf_vt_100.rs` and other test files.
- [x] Run `cargo check --tests` and `cargo test` to ensure everything is correct.

#### Phase 3: Encapsulate `OfsBufVT100`

The `OfsBufVT100` struct previously exposed `primary_buffer`, `alternate_buffer`,
`parser_global_state`, and `terminal_mode` as `pub`. Direct access to these fields is
unsafe because the struct has special handling for normal vs. alternate screens (routing
through `active_buf_mut()`).

- [x] Change visibility of these fields from `pub` to strictly private.
- [x] Provide `pub` accessors for sibling modules and tests:
    - `primary_buffer()` / `primary_buffer_mut()`
    - `alternate_buffer()` / `alternate_buffer_mut()`
    - `parser_global_state()` / `parser_global_state_mut()`
    - `terminal_mode()` / `terminal_mode_mut()`
- [x] Migrate external callers (e.g., `mouse_command.rs`, `output_renderer.rs`) to use the
      new public accessors instead of direct field access.
- [x] Update core operations implementations (`ops_impl_ofs_buf/`) and shims to use the
      proper getters/setters instead of raw field mutation.
- [x] Update tests (e.g., `vt_100_test_clear_ops.rs`, `vt_100_test_mode_ops.rs`) to use
      accessors.
- [x] Add rustdoc explaining the `ED 3` design decision in `vt_100_impl_clear_ops.rs` (it
      blindly clears the primary buffer's scrollback by convention, matching `xterm` /
      `WezTerm`).

#### Verification Plan

##### Automated Tests

- [x] Run `./check.fish --full` to verify tests, linting (clippy), and formatting.
      (Successfully completed).

##### Manual Verification

None required. This is a refactor to enforce invariants.

#### Mandatory Manual Review

- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_ansi_scroll_helper.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_control_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_cursor_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_dsr_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_line_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_mode_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_sgr_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_char_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_line_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_scroll_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_terminal_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/test_fixtures_ofs_buf_vt_100.rs`
- [x] `tui/src/core/pty/pty_mux/input_router/keyboard_command.rs`
- [x] `tui/src/core/pty/pty_mux/input_router/mouse_command.rs`
- [x] `tui/src/core/pty/pty_mux/output_renderer.rs`
- [x] `tui/src/core/pty/pty_mux/process_manager.rs`
- [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/render_to_ansi.rs`
- [x] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
- [x] `tui/src/core/pty/pty_mux/scrollback_amount.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_scroll_ops.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_system_performer_lifecycle.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_integration_real_world.rs`
- [x] `tui/src/tui/terminal_lib_backends/ofs_buf/mod.rs`

## Step I: Canvas and Viewport aware API

- [ ] inline `task/cleanup_viewport.md` into this section & remove the old file.

# Summary of Benefits

1. **Zero Duplicate VT-100 Logic:** We don't need a `Canvas` trait (which is the OOP
   mindset). The existing VT-100 logic in `ops_impl_ofs_buf/` remains exactly where it is
   (`impl OfsBufVT100`). It simply accesses the buffer via `self.active_buf_mut()` instead
   of `self.ofs_buf`.
2. **True Dependency Injection:** `OfsBuf` doesn't care if it's backed by a 1D slice or a
   VecDeque. It just calls `.shift_lines_up()` and the store handles its own business
   logic (like preserving history).
3. **No Breaking Changes to Compositor:** By using
   `OfsBuf<S: OfsBufStorage = Flat2DArray<PixelChar>>`, the UI rendering system
   (`OfsBufPaint`) can continue using `OfsBuf` without knowing about the generics or the
   growable scrollback buffer.

# Execution Plan

- [x] 1. Design the `OfsBufStorage` Trait (done above).
- [x] 2. Define the `OfsBufStorage` trait in
      `tui/src/tui/terminal_lib_backends/ofs_buf/buffer_storage/types.rs`.
- [x] 3. Create and implement `GrowableBuffer` (in a new file
      `tui/src/tui/terminal_lib_backends/ofs_buf/buffer_storage/growable_buffer.rs`):
    - Back it with a `VecDeque<PixelCharLine>`.
    - Implement `GetMemSize`.
    - Handle `set_viewport_col_offset` and `set_viewport_row_offset`.
- [x] 4. Implement `OfsBufStorage` for `Flat2DArray<PixelChar>`.
- [x] 5. Remove the flawed `Canvas` trait abstraction from
      `tui/src/tui/terminal_lib_backends/ofs_buf/buffer_storage/types.rs` (leaving only
      the `OfsBufStorage` trait and its documentation).
- [x] 6. Scope `OfsBuf::diff` to `impl OfsBuf<Flat2DArray<PixelChar>>` to ensure SIMD
      array requirements are met for the default store type.
- [x] 7. Define `ActiveBufferMut` enum in `ofs_buf_vt_100_core.rs` and update all VT100
      shim implementations in `ops_impl_ofs_buf/` to use `self.active_buf_mut().foo()`
      instead of `self.ofs_buf.foo()`.
- [x] 8. Refactor `OfsBufVT100` struct to replace `ofs_buf`, `scrollback_buffer`, and
      `hidden_screen_state` with:
    - `primary_buffer: OfsBuf<GrowableBuffer>`
    - `alternate_buffer: OfsBuf<Flat2DArray<PixelChar>>`
    - Also, delete `hidden_screen_state.rs` entirely (move the `ActiveScreenBuffer` enum
      to `terminal_mode.rs` or similar). Update `OfsBufVT100Config` initializers
      accordingly.
- [x] 9. Refactor `OutputRenderer::render_from_active_buffer` to remove explicit
      `ScrollbackBuffer` stitching and pull rows cleanly from the active buffer (handling
      viewport offset).
- [x] 10. Enforce `scrollback_limit` in `GrowableBuffer` during full screen scrolls.
- [x] 11. Handle the Alternate Screen (`CSI ? 1049 h`) by switching
      `terminal_mode.active_screen_buffer` and correctly routing all `get_active_canvas()`
      calls to `self.alternate_buffer`.
- [x] 12. Fix the test suite (`vt_100_pty_output_conformance_tests`) which directly
      inspects `.ofs_buf` state to work with the new `primary_buffer` and
      `alternate_buffer`.
- [x] 13. Update UI input handling (e.g. in `pty_mux_example.rs`) to intercept Mouse Wheel
      Left/Right and `Shift+Scroll` to increment/decrement `viewport.start.col`
      (Horizontal Panning) by delegating to `set_viewport_col_offset`.
- [x] 14. Create `tui/examples/pty_2d_panning_example.rs` as a dedicated showcase for the
      infinite 2D canvas and panning features.
- [x] 15. Document this overarching "Continuous 2D Buffer" architectural design as the
      struct-level rustdoc for `OfsBufVT100`.
- [x] 16. Implement the `Dims` primitive (struct and constructors).
- [x] 17. Refactor `OfsBufStorage` to use `get_viewport() -> Dims` instead of individual
      getters.
- [x] 18. Update `OfsBuf` and callers to use `get_viewport()`.
- [x] 19. Mandatory manual review (Verify tests run, `cargo check` passes, and the
      architecture matches the plan).
    - [x] `tui/src/core/common/flat_2d_array/array_1d_simd_access.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impl_flat_2d_array.rs`
    - [x] `tui/src/core/common/flat_2d_array/mod.rs`
    - [x] `tui/src/core/common/flat_2d_array/array_2d_access.rs`
    - [x] `tui/src/core/common/flat_2d_array/core.rs`
    - [x] `task/ofsbuf_trait_growable_impl.md`
    - [x] `tui/src/core/coordinates/buffer_coords/dims.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/core.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/paint_impl.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/viewport.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
    - [x] `tui/src/core/pty/pty_mux/output_renderer.rs`
    - [x] `tui/src/core/pty/pty_mux/mod.rs`
    - [x] `tui/src/core/pty/pty_mux/input_router/mouse_command.rs`
    - [x] `tui/src/core/pty/pty_mux/input_router/router.rs`
    - [x] `tui/src/core/pty/pty_mux/scrollback_amount.rs`
    - [x] `tui/examples/pty_2d_panning_example.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/core.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/active_buffer_routing.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/accessors.rs`
- [x] 20. Manual Testing:
    - [x] Run and test `tui/examples/pty_2d_panning_example.rs` to verify left/right
          panning.
    - [x] Run and test all other examples to ensure panning (left/right) works correctly.
    - [x] Test `pty_mux` to verify horizontal (left/right) panning functionality.

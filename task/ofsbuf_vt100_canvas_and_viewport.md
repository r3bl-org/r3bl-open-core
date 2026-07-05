# Task: Rearchitect the OfsBufVT100 to use a Trait-based Canvas Architecture

This outlines a structural refactor to replace the current `OffscreenBuffer` (Canvas) and
`ScrollbackBuffer` (History) paradigm with a unified, trait-driven `Canvas` architecture.
This task also incorporates the migration of the fixed-grid buffer to a highly optimized
1D array (SIMD-friendly) backing store.

## The Core Architecture

We will introduce a `Canvas` trait that abstracts VT100 drawing operations. The
`OfsBufVT100` parser will interact exclusively with this trait, decoupling the parsing
logic from the underlying storage mechanism.

```rust
// File: tui/src/core/common/flat_2d_array.rs

/// A generic, highly optimized 1D array backing store that provides a safe 2D grid API.
/// Uses bounds-checking newtypes (`RowHeight`, `ColWidth`, `RowIndex`, `ColIndex`) instead of `usize`.
pub struct Flat2DArray<T> {
    pub data: Box<[T]>,
    pub width: ColWidth,
    pub height: RowHeight,
}
impl<T> Flat2DArray<T> {
    // Contains `get`, `set`, `copy_within` (zero-allocation scrolling), and `fill` methods.
}

// File: tui/src/core/ansi/vt_100_pty_output_parser/canvas.rs

/// The fundamental VT100 abstraction. The `OfsBufVT100` parser uses this trait via an accessor
/// `get_active_canvas() -> &mut dyn Canvas`, allowing the VT100 shim operations to interact
/// directly with the trait object without duplicating all these methods on `OfsBufVT100` itself.
pub trait Canvas {
    // --- Cursor Operations ---
    fn get_cursor_pos(&self) -> Pos;
    fn set_cursor_pos(&mut self, pos: Pos);
    fn cursor_up(&mut self, how_many: Length);
    fn cursor_down(&mut self, how_many: Length);
    fn cursor_forward(&mut self, how_many: Length);
    fn cursor_backward(&mut self, how_many: Length);

    // --- Character Operations ---
    fn print_char(&mut self, ch: char);
    fn insert_chars_at_cursor(&mut self, how_many: Length);
    fn delete_chars_at_cursor(&mut self, how_many: Length);
    fn erase_chars_at_cursor(&mut self, how_many: Length);

    // --- Line Operations ---
    fn insert_lines(&mut self, how_many: Length, scroll_region: Range<RowIndex>);
    fn delete_lines(&mut self, how_many: Length, scroll_region: Range<RowIndex>);

    // --- Clear Operations ---
    fn erase_in_display(&mut self, mode: EraseDisplayMode);
    fn erase_in_line(&mut self, mode: EraseLineMode);

    // --- Scroll Operations ---
    fn scroll_up(&mut self, how_many: Length, scroll_region: Range<RowIndex>);
    fn scroll_down(&mut self, how_many: Length, scroll_region: Range<RowIndex>);

    // --- Memory ---
    fn get_mem_size(&self) -> usize;
}

// File: tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs
/// Fixed size, SIMD-optimized 1D array backing (For Alternate Screen / TUIs)
pub struct OffscreenBuffer {
    /// Uses the generic `Flat2DArray<T>` to safely encapsulate the 2D math,
    /// while retaining maximum cache locality and SIMD vectorization.
    pub grid: Flat2DArray<PixelChar>,
    pub cursor_pos: Pos,
}

// File: tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_growable.rs

/// Dynamically growing buffer (For Primary Screen / Standard CLI apps)
pub struct OffscreenBufferGrowable {
    /// 2D contiguous canvas. `PixelCharLine` is not clamped to viewport width,
    /// enabling unbounded horizontal panning.
    pub lines: VecDeque<PixelCharLine>,
    pub cursor_pos: Pos,
}

// Module: tui/src/core/ansi/vt_100_pty_output_parser/canvas_impl/
//   ├── mod.rs
//   ├── ofs_buf_impl.rs
//   └── ofs_buf_growable_impl.rs

/// Extension traits: Implements `Canvas` for the backend types here in the parser module,
/// strictly adhering to the Orphan Rule and keeping the backend files pure from VT100 semantics.
/// `ofs_buf_impl.rs` contains:
impl Canvas for OffscreenBuffer { /* ... */ }
/// `ofs_buf_growable_impl.rs` contains:
impl Canvas for OffscreenBufferGrowable { /* ... */ }

pub struct Viewport {
    pub start: Pos,
    pub size: Size,
}

pub enum BufferState {
    Primary {
        canvas: OffscreenBufferGrowable,
        viewport: Viewport,
        scrollback_limit: ScrollbackBufferLimit,
    },
    Alternate {
        canvas: OffscreenBuffer,
        viewport: Viewport,
    }
}

pub struct OfsBufVT100 {
    pub active_buffer: BufferState,
    pub parser_global_state: ParserGlobalState,
    pub terminal_mode: TerminalModeState,
}

impl OfsBufVT100 {
    /// Returns the active canvas trait object. The VT100 shim operations will call
    /// this accessor and interact with the `dyn Canvas` directly, rather than having
    /// `OfsBufVT100` duplicate all the trait methods as passthroughs.
    pub fn get_active_canvas(&mut self) -> &mut dyn Canvas {
        match &mut self.active_buffer {
            BufferState::Primary { canvas, .. } => canvas,
            BufferState::Alternate { canvas, .. } => canvas,
        }
    }
}
```

## Architectural Benefits

1. **Raw Speed for TUIs**: The Alternate Screen (which runs `vim`, `htop`, etc.) uses the
   1D array `OffscreenBuffer`. Since TUI apps redraw constantly, this provides maximum
   performance, SIMD auto-vectorization, and cache locality (e.g., using `copy_within` for
   scrolling, `fill` for clearing).
2. **Unbounded UX for Standard CLI**: The Primary Screen (which runs `cat`, `ls`, etc.)
   uses `OffscreenBufferGrowable` (backed by `VecDeque`). This gives us unbounded
   horizontal panning across long lines (like JSON logs) and infinite scrollback.
3. **Clean Parser Abstraction**: The VT100 parser just delegates to the `Canvas` trait
   interface, completely oblivious to the storage complexity beneath it.

## Implementation Checklist

- [ ] 1. Define the `Canvas` trait in
      `tui/src/core/ansi/vt_100_pty_output_parser/canvas.rs`. This should cover all
      necessary VT100 mutation and rendering operations exactly as outlined in the core
      architecture above.
- [ ] 2. Create the generic `Flat2DArray<T>` struct (in
      `tui/src/core/common/flat_2d_array.rs`):
  - Back it with a 1D `Box<[T]>` and define its dimensions using the bounds-checking
    newtypes `RowHeight` and `ColWidth` (avoid raw `usize`).
  - Implement 2D coordinate accessors like `get(row: RowIndex, col: ColIndex)` and
    `set(...)`, strictly using the type-safe indices.
  - Implement zero-allocation scrolling via `copy_within()`.
  - Implement SIMD-optimized clearing via `slice::fill()`.
  - Export it in `tui/src/core/common/mod.rs` so it is reusable across the codebase.
  - **Crucial**: Write comprehensive unit tests in `flat_2d_array.rs` to verify the
    2D-to-1D index mapping, bounds checking, scrolling, and clearing.
- [ ] 3. Refactor the `OffscreenBuffer` (in
      `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs`) to use the new
      `Flat2DArray<PixelChar>`.
- [ ] 4. Create and implement `OffscreenBufferGrowable` (in a new file
      `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_growable.rs`):
  - Back it with a `VecDeque<PixelCharLine>`.
  - Handle clear operations (e.g., `CSI 2 J` or `CSI K`) by wiping the _entire_ infinite
    line in memory, not just the visible viewport width.
- [ ] 5. Implement `Canvas` for both structures in a dedicated parser submodule at
      `tui/src/core/ansi/vt_100_pty_output_parser/canvas_impl/` to adhere to the Extension
      Trait pattern and keep VT100 logic out of the backend storage files.
  - Create `mod.rs` to coordinate the implementations.
  - Create `ofs_buf_impl.rs` for the 1D Array implementation.
  - Create `ofs_buf_growable_impl.rs` for the `VecDeque` implementation.
- [ ] 6. Refactor the VT100 shim methods (e.g. `vt_100_shim_char_ops.rs`) to use the
      `ofs_buf_vt_100.get_active_canvas()` accessor instead of expecting inherent methods
      on `OfsBufVT100`.
- [ ] 7. Refactor `OfsBufVT100` struct to use the `BufferState` enum. Ensure
      `parser_global_state` and `terminal_mode` are preserved.
- [ ] 8. Implement `GetMemSize` for `OfsBufVT100`. The fixed `OffscreenBuffer` can use
      cached O(1) calculations, while `OffscreenBufferGrowable` must calculate or smartly
      cache its size dynamically.
- [ ] 9. Update all internal coordinate math (VT100 parser) to use the active buffer and
      offset operations by `viewport.start.row` and `viewport.start.col` when in the
      Primary state.
- [ ] 10. Enforce `scrollback_limit` in `OffscreenBufferGrowable` during full screen
      scrolls: pop old lines from the front of the `VecDeque` when
      `lines.len() > viewport.size.row_height + scrollback_limit`.
- [ ] 11. Handle the Alternate Screen (`CSI ? 1049 h`) by switching the `active_buffer`
      enum variant to `Alternate` (which uses the 1D array `OffscreenBuffer`).
- [ ] 12. Update UI input handling (e.g. in `pty_mux_example.rs`) to intercept Mouse Wheel
      Left/Right and `Shift+Scroll` to increment/decrement `viewport.start.col`
      (Horizontal Panning).
- [ ] 13. Create `tui/examples/pty_2d_panning_example.rs` as a dedicated showcase for the
      unbounded 2D panning capability.
- [ ] 14. Document this overarching "Continuous 2D Buffer" architectural design as the
      struct-level rustdoc for `OfsBufVT100` (following the inverted pyramid pattern). Add
      a new section in `src/tui/lib.rs` (which becomes `tui/README.md`) highlighting the
      groundbreaking ability to horizontally pan standard CLI apps (like `cat`, `tail`)
      over SSH.

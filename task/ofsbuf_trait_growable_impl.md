# Architecture Plan: Unifying `OfsBuf` and `OfsBufGrowable`

<!-- cspell:words ofsbuf memmoves -->

To address questions about overlap, traits, DI, and avoiding duplicate work, here is a
high-performance, deduplicated architectural design.

## 1. What is the overlap between the two?

**Overlap (The "What"):** Both buffers act as a 2D grid of `PixelChar` with a viewport.
Both require a cursor position (`Pos`). Both need to support all 20+ VT-100 operations
(e.g., `insert_lines`, `print_char`, `move_cursor_up`, `erase_in_display`).

**Difference (The "How"):** The fundamental difference is their memory layout and how they
handle VT-100 line shifting vs. Terminal Scrollback:

- **Alternate Screen (Fixed Canvas):** Backed by `Flat2DArray` (a contiguous 1D slice).
  There is **no terminal scrollback** in the alternate screen. When VT-100 line-shifting
  operations occur (e.g., `IL`, `DL`, `SU`, `SD`), it shifts the lines in memory (using
  SIMD `copy_within_rows`), and permanently destroys whatever lines fall outside the
  active region.
- **Primary Screen (Growable Canvas):** Backed by a `VecDeque` of lines (e.g.
  `VecDeque<Box<[PixelChar]>>`). This screen supports **terminal scrollback history**.
  When the terminal scrolls (e.g. output pushes past the bottom of the screen), it pushes
  a new line to the bottom and _preserves_ the old top line in the scrollback history,
  rather than destroying it.

## 2. Can we extract a common trait and use Generics/DI?

**Yes! This is the perfect use case for Dependency Injection via Generics.**

If we build two separate structs (`OfsBuf` and `OfsBufGrowable`), we would have to
implement the `Canvas` trait (and all its VT-100 cursor math) **twice**, which is massive
duplication.

Instead, we can extract the backing store into a `BufferStorage` trait. `OfsBuf` becomes a
generic shell that handles all VT-100 logic, and delegates raw memory operations to the
injected store.

## 3. The Proposed Design

Here is the exact architecture to achieve zero duplication:

### Step A: The `BufferStorage` Trait

Extract the raw memory operations into a trait. This only requires operations that differ
based on the backing store.

```rust
pub trait BufferStorage {
    fn get_width(&self) -> ColWidth;
    fn get_height(&self) -> RowHeight; // Height of the viewport

    // Viewport-relative indexing (row 0 is the top of the visible screen)
    fn get_row(&self, row: RowIndex) -> Option<&[PixelChar]>;
    fn get_row_mut(&mut self, row: RowIndex) -> Option<&mut [PixelChar]>;

    // -------------------------------------------------------------------------
    // VT-100 Line Shifting (In-memory data destruction)
    // Used by `IL` (Insert Line) and `DL` (Delete Line) or Margin Scrolls.
    // - Flat2DArray: Uses fast 1D SIMD copy_within_rows.
    // - GrowableStore: Rotates elements in place (O(N) pointer swaps).
    // -------------------------------------------------------------------------
    fn shift_lines_up(&mut self, row_range: Range<RowIndex>, amount: Length, empty_char: PixelChar);
    fn shift_lines_down(&mut self, row_range: Range<RowIndex>, amount: Length, empty_char: PixelChar);

    // -------------------------------------------------------------------------
    // Terminal Scrolling (Viewport panning)
    // Triggered by `\n` at the bottom of the screen (unrestricted scroll).
    // - Flat2DArray (Alternate Screen): There is NO SCROLL in the alternate screen.
    //   This just degrades to `shift_lines_up(0..height)`.
    // - GrowableStore (Primary Screen): Appends a new line to the VecDeque and
    //   pans the viewport down, natively preserving the old top line in history!
    // -------------------------------------------------------------------------
    fn scroll_up(&mut self, amount: Length, empty_char: PixelChar);

    fn fill_all(&mut self, empty_char: PixelChar);
}
```

### Step B: The Generic `OfsBuf`

`OfsBuf` becomes generic over the `BufferStorage`. We provide a default generic so we
don't break existing UI compositor code (`OfsBufPaint`).

```rust
pub struct OfsBuf<S: BufferStorage = Flat2DArray<PixelChar>> {
    pub store: S,
    pub cursor_pos: Pos,
}
```

### Step C: Implement `Canvas` ONCE

Instead of writing `ofs_buf_impl.rs` and `ofs_buf_growable_impl.rs`, we implement `Canvas`
generically. All VT-100 math is written only one time.

```rust
impl<S: BufferStorage> Canvas for OfsBuf<S> {
    fn move_cursor_up(&mut self, how_many: Length) {
        self.cursor_pos.row_index = self.cursor_pos.row_index.saturating_sub(how_many);
    }

    fn insert_lines(&mut self, how_many: Length, scroll_region: Range<RowIndex>) {
        self.store.shift_lines_down(scroll_region, how_many, PixelChar::Spacer);
    }
    // ... all other Canvas methods
}
```

### Step D: The `OfsBufVT100` State Machine

The parser holds the `BufferState` enum, switching between the two stores at runtime. This
uses Trait Objects (`&mut dyn Canvas`) so the VT-100 parser itself doesn't become generic
(preventing compile-time bloat).

```rust
pub enum BufferState {
    Primary {
        canvas: OfsBuf<GrowableStore>,
    },
    Alternate {
        canvas: OfsBuf<Flat2DArray<PixelChar>>,
    }
}

impl OfsBufVT100 {
    pub fn get_active_canvas(&mut self) -> &mut dyn Canvas {
        match &mut self.active_buffer {
            BufferState::Primary { canvas, .. } => canvas,
            BufferState::Alternate { canvas, .. } => canvas,
        }
    }
}
```

# Summary of Benefits

1. **Zero Duplicate VT-100 Logic:** We implement `move_cursor_up`, `insert_chars`, and
   `erase_in_display` exactly once.
2. **True Dependency Injection:** `OfsBuf` doesn't care if it's backed by a 1D slice or a
   VecDeque. It just calls `.shift_lines_up()` and the store handles its own business
   logic (like preserving history).
3. **No Breaking Changes to Compositor:** By using
   `OfsBuf<S: BufferStorage = Flat2DArray<PixelChar>>`, the UI rendering engine
   (`OfsBufPaint`) can continue using `OfsBuf` without knowing about the generics or the
   growable scrollback buffer.

# Execution Plan

- [x] 1. Design the `BufferStorage` Trait (done above).
- [ ] 2. Define the `BufferStorage` trait in
      `tui/src/core/ansi/vt_100_pty_output_parser/canvas.rs`.
- [ ] 3. Create and implement `GrowableStore` (in a new file
      `tui/src/tui/terminal_lib_backends/ofs_buf/ofs_buf_growable.rs`):
    - Back it with a `VecDeque<PixelCharLine>`.
- [ ] 4. Implement `BufferStorage` for `Flat2DArray<PixelChar>`.
- [ ] 5. Implement `Canvas` generically for `OfsBuf<S: BufferStorage>` in
      `tui/src/core/ansi/vt_100_pty_output_parser/canvas_impl/`.
- [ ] 6. Refactor the VT100 shim methods (e.g. `vt_100_shim_char_ops.rs`) to use the
      `ofs_buf_vt_100.get_active_canvas()` accessor.
- [ ] 7. Refactor `OfsBufVT100` struct to use the `BufferState` enum. Ensure
      `parser_global_state` and `terminal_mode` are preserved.
- [ ] 8. Enforce `scrollback_limit` in `GrowableStore` during full screen scrolls.
- [ ] 9. Handle the Alternate Screen (`CSI ? 1049 h`) by switching the `active_buffer`
      enum variant to `Alternate` (which uses the 1D array `Flat2DArray`).
- [ ] 10. Update UI input handling (e.g. in `pty_mux_example.rs`) to intercept Mouse Wheel
      Left/Right and `Shift+Scroll` to increment/decrement `viewport.start.col`
      (Horizontal Panning).
- [ ] 11. Create `tui/examples/pty_2d_panning_example.rs` as a dedicated showcase for the
      unbounded 2D panning capability.
- [ ] 12. Document this overarching "Continuous 2D Buffer" architectural design as the
      struct-level rustdoc for `OfsBufVT100`.

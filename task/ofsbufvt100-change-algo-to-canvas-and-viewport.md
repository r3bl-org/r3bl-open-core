# Task: Rearchitect the OfsBufVT100 to use "Canvas and Viewport" architecture

This outlines a structural refactor to replace the current `OffscreenBuffer` (Canvas) and
`ScrollbackBuffer` (History) paradigm with a unified "Continuous 2D Buffer" (Tape &
Viewport) architecture.

## The Core Architecture

We will eliminate the `OffscreenBuffer` abstraction entirely and have `OfsBufVT100`
directly own the state as a continuous tape with a sliding viewport window:

```rust
pub struct Viewport {
    /// The top-left anchor of the active viewport in the 2D canvas.
    /// `Pos.col` is the horizontal offset (X).
    /// `Pos.row` is the vertical offset (Y).
    pub start: Pos,

    /// The visible dimensions of the terminal window (height and width).
    pub size: Size,
}

pub struct OfsBufVT100 {
    /// The 2D contiguous tape.
    /// Crucially, `PixelCharLine` is no longer clamped to `viewport.size.width`.
    /// It can grow dynamically to any length.
    pub tape: VecDeque<PixelCharLine>,

    /// The sliding window over the tape.
    pub viewport: Viewport,

    /// Limits how much history we retain before `viewport.start.row`.
    /// Reuses the existing `ScrollbackBufferLimit` (Fixed or Unlimited).
    pub scrollback_limit: ScrollbackBufferLimit,
}
```

## Architectural Benefits

1. **O(1) Rendering Complexity**: The `OutputRenderer` no longer needs to stitch lines
   from a separate `ScrollbackBuffer` and `OffscreenBuffer`. It simply slices the `tape`
   using `viewport.start`.
2. **Elegant Resizing**: Resizing the terminal height becomes simple math. Expanding
   height just involves decrementing `viewport.start.row` to pull lines from history.
   Shrinking height pushes the top lines into history.
3. **2D Panning Superpower**: Because `PixelCharLine` can grow infinitely wide (when
   auto-wrap is disabled), we unlock **horizontal scrolling**.
   - `viewport.start.col` can be incremented/decremented to pan left and right across
     massively wide lines (like JSON logs) without wrapping destroying the layout.

## Implementation Checklist

- [ ] 1. Refactor `OfsBufVT100` struct to use `Viewport` and the continuous `tape`.
- [ ] 2. Update all internal coordinate math (VT100 parser) to offset operations by
      `viewport.start.row` and `viewport.start.col`.
- [ ] 3. Bifurcate scroll logic:
  - "Full screen scroll" simply pushes a new line to the `tape` and slides
    `viewport.start.row`.
  - "Margin scroll" (partial scrolling regions) performs an in-place block copy within the
    viewport bounds.
- [ ] 4. Handle the Alternate Screen (`CSI ? 1049 h`): Create an enum (e.g.
      `BufferState { Primary(Tape), Alternate(FixedGrid) }`) so that apps like `vim` don't
      pollute the scrollback history with their UI artifacts.
- [ ] 5. Update UI input handling (e.g. in `pty_mux_example.rs`) to intercept Mouse Wheel
      Left/Right and `Shift+Scroll` to increment/decrement `viewport.start.col`.
- [ ] 6. Create `tui/examples/pty_2d_panning_example.rs` as a dedicated showcase for the
      unbounded 2D panning capability.
- [ ] 7. Document this overarching "Continuous 2D Buffer" architectural design as the
      struct-level rustdoc for `OfsBufVT100` (following the inverted pyramid pattern).

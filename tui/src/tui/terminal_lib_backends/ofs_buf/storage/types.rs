// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words SSOT

use crate::{GetMemSize, PixelChar, RangeExclusive, VPLength, VPRow, Viewport,
            ViewportPanValidity};

/// Abstract 2D storage engine backing an [`OfsBuf`].
///
/// # Why This Trait Exists
///
/// We evolved the storage design behind [`OfsBuf`] through several iterations to balance
/// raw CPU performance with features like scrollback and 2D panning (both Up/Down AND
/// Left/Right).
///
/// Up/Down panning is a common feature in terminal emulators, but left/right panning is
/// not. Our [virtual terminal] architecture allows us to implement Left/Right panning for
/// terminal apps that use normal mode, like `cat`, `grep`, `head`, `tail`, etc. We call
/// this the [Canvas and Viewport Concept], which is documented on [`GrowableBuffer`].
///
/// Originally, [`OfsBuf`] stored rows as `Vec<Vec<PixelChar>>`. While simple, chasing
/// heap pointers across separate row vectors hurt CPU cache locality. To fix this, we
/// flattened storage into [`Flat2DArray`] — a single, contiguous 1D memory array. This
/// layout delivered massive L1 cache hits and enabled SIMD vector operations for fast row
/// shifting and clearing.
///
/// While [`Flat2DArray`] ran blazingly fast on fixed-size screens (like the alternate
/// screen mode in `vim` or `htop`), it is not designed to handle this use case -
/// scrolling a line off the top (which is what normal screen apps expect terminal
/// emulators to do). We initially added vertical scrollback by pairing [`OfsBuf`] with an
/// external helper struct. However, spreading scrollback logic across multiple components
/// fragmented state management and violated our Single Source of Truth principles (SSOT).
///
/// We introduced [`CanvasStorage`] to establish clean SSOT boundaries. This trait
/// decouples high-level buffer logic (drawing, styling, diffing) from physical memory
/// layout, letting us plug in [`GrowableBuffer`]. [`GrowableBuffer`] uses a
/// [`VecDeque`]-backed engine that unifies vertical scrollback history with 2D (Up/Down
/// and Left/Right) viewport panning. Horizontal panning across long lines gives our
/// [`pty_mux`] [virtual terminal] primitive a novel capability that standard terminal
/// emulators lack when running normal-mode commands like  `cat`, `grep`, `head`, `tail`,
/// etc.
///
/// With [`CanvasStorage`], [`OfsBuf`] stays completely agnostic of whether it draws onto
/// a fixed [SIMD]-accelerated array or a 2D scrollback canvas. It is polymorphic over the
/// underlying storage engine, which can be swapped out for different memory layouts
/// without affecting the high-level buffer logic. Leaving it open to future storage
/// engines that may be optimized for specific use cases, like a GPU-backed canvas or a
/// memory-mapped file.
///
/// # Implementations
///
/// - [`Flat2DArray`]: Fixed-size contiguous slice built for maximum speed on screens
///   without scrollback (e.g., alternate screen buffer).
/// - [`GrowableBuffer`]: Dynamic [`VecDeque`] canvas supporting scrollback history and 2D
///   (Up/Down and Left/Right) viewport panning (e.g., primary screen buffer).
///
/// # Vertical Scrollback vs. Horizontal Panning
///
/// Moving the visible window involves two distinct mechanics to protect the [`VT-100`]
/// parser state:
///
/// 1. **Vertical (Up/Down) Scrollback:**
///    - Handled via `get_row_with_scrollback(row, scrollback_amt)` and the underlying
///      [`VecDeque`] history.
///    - **Important Nuance:** Vertical scrolling is *not* done by mutating
///      [`try_pan_viewport_to`]'s vertical `row_index`. If vertical panning were mutated
///      natively in the storage buffer, it would detach the [`VT-100`] parser's active
///      cursor from the live bottom line! Instead, vertical scrollback is applied
///      externally during rendering using [`ScrollbackAmount`].
///
/// 2. **Horizontal (Left/Right) Panning:**
///    - Handled natively via [`try_pan_viewport_to`] by changing `origin_pos.col_index`.
///    - Shifting the horizontal column offset is completely safe because moving left or
///      right across columns does not affect the [`VT-100`] parser's active line
///      appending.
///
/// # Architecture & Documentation Map
///
/// For a complete understanding of offscreen buffers, storage, and [virtual terminal]s,
/// refer to:
///
/// 1. [`CanvasStorage`] ([`types.rs`]): Trait Level — *The "Why"* (Architectural
///    evolution, storage abstraction, and motivation for 2D viewport panning).
/// 2. [`GrowableBuffer`] ([`growable_buffer.rs`]): Implementation Level — *The "How It's
///    Stored"* (Canvas and Viewport concept, [`VecDeque`] history storage, and 2D grid
///    mechanics).
/// 3. [`pty_mux`] ([`mod.rs`]): UX & Parser Level — *The "How It's Triggered"* (Viewport
///    mechanics, mouse scroll vs horizontal pan, and [`VT-100`] parser cursor anchoring).
///
/// [`Flat2DArray`]: crate::core::common::flat_2d_array::Flat2DArray
/// [`get_viewport`]: Self::get_viewport
/// [`growable_buffer.rs`]: crate::tui::GrowableBuffer
/// [`GrowableBuffer`]: crate::tui::GrowableBuffer
/// [`mod.rs`]: crate::core::pty::pty_mux
/// [`OfsBuf`]: crate::tui::OfsBuf
/// [`pty_mux`]: crate::core::pty::pty_mux
/// [`ScrollbackAmount`]: crate::ScrollbackAmount
/// [`try_pan_viewport_to`]: Self::try_pan_viewport_to
/// [`types.rs`]: Self
/// [`VecDeque`]: std::collections::VecDeque
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [SIMD]: https://en.wikipedia.org/wiki/SIMD
/// [virtual terminal]: crate::core::pty::pty_mux#virtual-terminal-architecture
pub trait CanvasStorage: GetMemSize {
    /// Returns the dimensions (size and offset) of the active viewport.
    ///
    /// See [Canvas and Viewport Concept] for details on how this coordinates with the
    /// underlying canvas.
    ///
    /// **Implementation Note**: For fixed-size buffers (like [`Flat2DArray`]), the
    /// viewport is permanently locked to `(0, 0)` to perfectly match the viewport screen
    /// size. For dynamically growing buffers [`GrowableBuffer`], the viewport can move
    /// across the infinite canvas.
    ///
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    /// [`GrowableBuffer`]: crate::tui::GrowableBuffer
    /// [Canvas and Viewport Concept]: crate::tui::GrowableBuffer#canvas-and-viewport-concept
    fn get_viewport(&self) -> Viewport;

    /// Sets the offset for the active viewport.
    ///
    /// Used for panning the visible screen area over the underlying data. See [Canvas and
    /// Viewport Concept] for details on the underlying 2D grid mechanics.
    ///
    /// **Implementation Note**: For fixed-size buffers (like [`Flat2DArray`]), this
    /// method is a no-op since the viewport is permanently locked to `(0, 0)` to
    /// perfectly match the viewport screen size. For dynamically growing buffers
    /// [`GrowableBuffer`], this moves the viewport across the infinite canvas.
    ///
    /// For an explanation of how panning is utilized in [`pty_mux`] without corrupting
    /// parser state see [Viewport Mechanics (Scroll vs Pan)]. It covers why:
    /// - We avoid vertical panning in favor of external scrolling.
    /// - Horizontal panning is safe.
    ///
    /// Attempts to pan the viewport to the requested origin position.
    ///
    /// # Errors
    /// Returns `Err(ViewportPanValidity::InvalidVerticalPan { .. })` if vertical panning
    /// is attempted.
    ///
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    /// [`GrowableBuffer`]: crate::tui::GrowableBuffer
    /// [`pty_mux`]: crate::core::pty::pty_mux
    /// [Canvas and Viewport Concept]: crate::tui::GrowableBuffer#canvas-and-viewport-concept
    /// [Viewport Mechanics (Scroll vs Pan)]:
    ///     crate::core::pty::pty_mux#viewport-mechanics-scroll-vs-pan
    fn try_pan_viewport_to(
        &mut self,
        origin_pos: crate::CPos,
    ) -> Result<(), ViewportPanValidity>;

    /// Gets a read-only reference to a line in the buffer at the specified row.
    ///
    /// # Viewport Interactions
    /// - **Vertically**: The `row` argument is relative to the viewport (row 0 is the top
    ///   of the visible screen). For fixed-size buffers, the viewport origin is always
    ///   `0`, meaning the relative row perfectly matches the absolute canvas row.
    /// - **Horizontally**: The returned slice is **not** affected by horizontal panning.
    ///   It returns the entire line starting from column 0. The caller must manually
    ///   apply the viewport's column offset if they want the visible slice.
    ///
    /// Returns [`None`] if the row is out of bounds.
    ///
    /// See [Canvas and Viewport Concept] for more details.
    ///
    /// [Canvas and Viewport Concept]: crate::tui::GrowableBuffer#canvas-and-viewport-concept
    fn get_row(&self, row: VPRow) -> Option<&[PixelChar]>;

    /// Gets a mutable reference to a line in the buffer at the specified row.
    ///
    /// # Viewport Interactions
    /// - **Vertically**: The `row` argument is relative to the viewport (row 0 is the top
    ///   of the visible screen). For fixed-size buffers, the viewport origin is always
    ///   `0`, meaning the relative row perfectly matches the absolute canvas row.
    /// - **Horizontally**: The returned slice is **not** affected by horizontal panning.
    ///   It returns the entire line starting from column 0. The caller must manually
    ///   apply the viewport's column offset if they want the visible slice.
    ///
    /// Returns [`None`] if the row is out of bounds.
    ///
    /// See [Canvas and Viewport Concept] for more details.
    ///
    /// [Canvas and Viewport Concept]: crate::tui::GrowableBuffer#canvas-and-viewport-concept
    fn get_row_mut(&mut self, row: VPRow) -> Option<&mut [PixelChar]>;

    /// Shifts a range of lines (destructively) upward or downward by the specified
    /// amount.
    ///
    /// This is a **destructive** operation. It destroys the data at the edge of the range
    /// being shifted towards, and fills the vacated lines at the opposite edge with `it`.
    ///
    /// # Use Case
    ///
    /// This is critical for terminal applications (like Vim, `tmux`, or `less`) that
    /// divide the screen into separate panes or use scrolling margins. For example, if a
    /// user deletes a line in the middle of a Vim split window, the terminal needs to
    /// shift only the lines within that specific split upward to close the gap. This
    /// destroys the deleted line and inserts a blank line at the bottom of the split
    /// window.
    ///
    /// Used by [`IL`] (Insert Line) and [`DL`] (Delete Line) operations within specific
    /// margin scrolls, as well as [`SU`] (Scroll Up) and [`SD`] (Scroll Down).
    ///
    /// - **Upwards**: Destroys data at the top, fills at the bottom. Used by [`DL`] and
    ///   [`SU`].
    /// - **Downwards**: Destroys data at the bottom, fills at the top. Used by [`IL`] and
    ///   [`SD`].
    ///
    /// For scrolling the entire viewport (which may preserve history depending on the
    /// storage implementation), see [`allocate_new_lines_at_bottom`].
    ///
    /// [`allocate_new_lines_at_bottom`]: Self::allocate_new_lines_at_bottom
    /// [`DL`]: https://vt100.net/docs/vt510-rm/DL.html
    /// [`IL`]: https://vt100.net/docs/vt510-rm/IL.html
    /// [`SD`]: https://vt100.net/docs/vt510-rm/SD.html
    /// [`SU`]: https://vt100.net/docs/vt510-rm/SU.html
    fn shift_lines_in_range(
        &mut self,
        direction: ShiftLinesDirection,
        row_index_range: RangeExclusive<VPRow>,
        amount: VPLength,
        fill_char: PixelChar,
    );

    /// Allocates new lines at the bottom of the buffer to make room for new content.
    ///
    /// New lines introduced at the bottom are filled with `fill_char`.
    ///
    /// **Implementation Note**: Depending on the storage implementation (like
    /// [`GrowableBuffer`]), lines scrolled off the top may be preserved in the scrollback
    /// history rather than destroyed. For fixed-size buffers (like [`Flat2DArray`]),
    /// lines scrolled off the top are permanently lost.
    ///
    /// For a scroll operation that operates on a specific subset of the screen (useful
    /// for [`VT-100`] margins), see [`shift_lines_in_range`].
    ///
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    /// [`GrowableBuffer`]: crate::tui::GrowableBuffer
    /// [`shift_lines_in_range`]: Self::shift_lines_in_range
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    fn allocate_new_lines_at_bottom(
        &mut self,
        arg_amount: impl Into<VPLength>,
        fill_char: PixelChar,
    );

    /// Fills the entire visible buffer with the specified `fill_char`.
    fn clear_viewport_with(&mut self, fill_char: PixelChar);

    /// Fills a range of viewport-relative rows with `fill_char`.
    fn fill_row_range(
        &mut self,
        row_index_range: RangeExclusive<VPRow>,
        fill_char: PixelChar,
    );

    /// Swaps the contents of two viewport-relative rows.
    ///
    /// # Errors
    ///
    /// Returns an error if either row is out of bounds.
    fn swap_lines(
        &mut self,
        row_index_1: VPRow,
        row_index_2: VPRow,
    ) -> miette::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShiftLinesDirection {
    Up,
    Down,
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::CanvasStorage;
use crate::{CPos, CRow, CanvasRangeExt, GetMemSize, PixelChar, PixelCharLine,
            RangeBoundsResult, RangeExclusive, RangeValidityStatus, ScrollbackAmount,
            ShiftLinesDirection, StorageLineLimit, VPLength, VPRow, VPSize, Viewport,
            ViewportToCanvasExt, ok, scrollback_amount, vp_height};
use std::{cmp::{max, min},
          collections::VecDeque};

/// A dynamically growing storage backend for an [`OfsBuf`] that serves as an infinite
/// canvas.
///
/// It retains all output lines (up to an optional [`StorageLineLimit`]), allowing
/// for vertical scrolling through history without losing output that has rolled off the
/// viewport.
///
/// # Viewport Boundaries
///
/// While the underlying storage contains the entire scrollback history, all viewport
/// operations defined by [`CanvasStorage`] (such as read/write access via [`get_row`] and
/// [`get_row_mut`], line shifting, and line swapping) are strictly restricted to the
/// active visible viewport region. Row indices exceeding the viewport bounds will result
/// in errors or return [`None`].
///
/// # Canvas and Viewport Concept
///
/// For visual diagrams, taxonomy, and mathematical definition of the Canvas and
/// Viewport concept, see [`canvas`].
///
/// All storage operations operate explicitly in either:
/// - **Viewport Coordinates (Viewport-Relative)** (visible screen window space), or
/// - **Canvas Coordinates (Canvas-Absolute)** (absolute storage buffer space).
///
/// This continuous canvas approach enables the following key properties:
///
/// - **[`Viewport`]**
///   - Returned by [`get_viewport`], it defines the visible window's origin [`VPPos`]
///     within the canvas and its viewport [`VPSize`].
/// - **Viewport-Relative Math**
///   - Methods like [`get_row`] operate in **Viewport Coordinates (Viewport-Relative)**.
///     Requesting row 0 returns the top line of the visible screen, transparently
///     translating to **Canvas Coordinates (Canvas-Absolute)** space.
///
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
/// [`canvas`]: mod@crate::core::coordinates::canvas
/// [`CanvasStorage`]: crate::CanvasStorage
/// [`get_row_mut`]: Self::get_row_mut
/// [`get_row`]: Self::get_row
/// [`get_viewport`]: crate::CanvasStorage::get_viewport
/// [`growable_buffer.rs`]: Self
/// [`GrowableBuffer`]: Self
/// [`mod.rs`]: crate::core::pty::pty_mux
/// [`OfsBuf`]: crate::tui::OfsBuf
/// [`pty_mux`]: crate::core::pty::pty_mux
/// [`ScrollbackAmount`]: crate::ScrollbackAmount
/// [`try_pan_viewport_to`]: crate::CanvasStorage::try_pan_viewport_to
/// [`types.rs`]: crate::CanvasStorage
/// [`VecDeque`]: std::collections::VecDeque
/// [`Viewport`]: crate::Viewport
/// [`VPPos`]: crate::core::VPPos
/// [`VPSize`]: crate::VPSize
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [Canvas and Viewport Concept]: #canvas-and-viewport-concept
/// [virtual terminal]: crate::core::pty::pty_mux#virtual-terminal-architecture
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrowableBuffer {
    lines: VecDeque<PixelCharLine>,
    viewport: Viewport,
    storage_line_limit: StorageLineLimit,
}

impl GrowableBuffer {
    /// Creates a new, empty [`GrowableBuffer`] with the specified viewport dimensions and
    /// a capacity constraint on how many lines it can retain.
    ///
    /// Unlike fixed-size backends (like [`Flat2DArray`]) which have no concept of
    /// scrollback history, [`GrowableBuffer`] requires a [`StorageLineLimit`] to
    /// prevent unbounded memory growth as new lines are allocated at the bottom.
    ///
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    #[must_use]
    pub fn new_empty(
        arg_size: impl Into<VPSize>,
        storage_line_limit: StorageLineLimit,
        default_val: PixelChar,
    ) -> Self {
        let VPSize {
            col_width: width,
            row_height: height,
        } = arg_size.into();

        // Initialize with viewport height. `vec![...]` avoids ring buffer overhead during
        // initialization, and `.into()` is an O(1) cast with zero allocations.
        let lines = {
            let empty_row = PixelCharLine::new_empty(width, default_val);
            let num_of_rows = height.as_usize();
            let vec_of_rows = vec![empty_row; num_of_rows];
            vec_of_rows.into()
        };

        Self {
            lines,
            viewport: (crate::c_row(0usize), crate::c_col(0usize), width, height).into(),
            storage_line_limit,
        }
    }

    /// Wipes all scrollback history, retaining only the active viewport lines.
    ///
    /// This method is specific to [`GrowableBuffer`] and is not part of the
    /// [`CanvasStorage`] trait. The trait is an abstraction for *any* 2D buffer backend,
    /// including fixed-size grids like [`Flat2DArray`] which inherently have no concept
    /// of scrollback history.
    ///
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    pub fn clear_scrollback(&mut self) {
        self.lines.drain(0..self.viewport.get_history_len());
        self.viewport.reset_history_len();
    }

    /// Retrieves a read-only reference to a line in the buffer, taking into account an
    /// explicit scrollback offset.
    ///
    /// Suppose we have 5 lines of history and a 3-line active screen:
    ///
    /// ```text
    /// [ Line 0 ]  <-- History (scrolled off top)
    /// [ Line 1 ]  <-- History
    /// [ Line 2 ]  <-- History
    /// [ Line 3 ]  <-- History
    /// [ Line 4 ]  <-- History (history_len = 5)
    /// ======================================== (Top of active screen)
    /// [ Line 5 ]  <-- Active Viewport Row 0
    /// [ Line 6 ]  <-- Active Viewport Row 1
    /// [ Line 7 ]  <-- Active Viewport Row 2
    /// ```
    ///
    /// - Calling `get_row(vp_row(0))` with `scrollback_amt = 0` returns Line 5 (Active
    ///   Viewport Line 0)
    /// - Calling `get_row_with_scrollback(vp_row(0), scrollback_amount(2))` shifts back
    ///   by 2 lines and returns Line 3.
    ///
    /// This method is specific to [`GrowableBuffer`] and is not part of the
    /// [`CanvasStorage`] trait. The trait is an abstraction for *any* 2D buffer backend,
    /// including fixed-size grids like [`Flat2DArray`] which inherently have no concept
    /// of scrollback history.
    ///
    /// [`CanvasStorage`]: crate::CanvasStorage
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    #[must_use]
    pub fn get_row_with_scrollback(
        &self,
        row_index: VPRow,
        scrollback_amt: ScrollbackAmount,
    ) -> Option<&[PixelChar]> {
        let abs_row_index = scrollback_amt.to_c_row(&self.viewport, row_index);
        let line = self.lines.get(abs_row_index.as_usize())?;
        Some(line.pixel_chars.as_slice())
    }

    /// Retrieves a mutable reference to a line in the buffer, taking into account an
    /// explicit scrollback offset.
    ///
    /// See [`Self::get_row_with_scrollback()`] for details on the mental model.
    pub fn get_row_with_scrollback_mut(
        &mut self,
        row_index: VPRow,
        scrollback_amt: ScrollbackAmount,
    ) -> Option<&mut [PixelChar]> {
        let abs_row_index = scrollback_amt.to_c_row(&self.viewport, row_index);
        let line_mut = self.lines.get_mut(abs_row_index.as_usize())?;
        Some(&mut line_mut.pixel_chars)
    }

    /// Enforce the storage line limit by popping lines from the top if necessary.
    fn trim_lines_to_storage_line_limit(&mut self) {
        if let Some(max_lines) = self
            .storage_line_limit
            .calc_max_line_count(vp_height(self.viewport.get_height()))
        {
            while self.lines.len() > max_lines {
                self.lines.pop_front();
                self.viewport.decrement_history_len();
            }
        }
    }
}

impl GetMemSize for GrowableBuffer {
    fn get_mem_size(&self) -> usize {
        let mut total = 0;
        for line in &self.lines {
            total += line.get_mem_size();
        }
        total
    }
}

impl CanvasStorage for GrowableBuffer {
    #[inline]
    fn get_viewport(&self) -> Viewport { self.viewport }

    fn get_row(&self, row_index: VPRow) -> Option<&[PixelChar]> {
        if self.viewport.contains_row(row_index) != RangeBoundsResult::Within {
            return None;
        }
        self.get_row_with_scrollback(row_index, scrollback_amount(0))
    }

    fn get_row_mut(&mut self, row_index: VPRow) -> Option<&mut [PixelChar]> {
        if self.viewport.contains_row(row_index) != RangeBoundsResult::Within {
            return None;
        }
        self.get_row_with_scrollback_mut(row_index, scrollback_amount(0))
    }

    /// Changes the viewport offset over the underlying data.
    ///
    /// Note that this buffer natively refuses to pan vertically (the `row_index` cannot
    /// be changed here). For a high-level explanation of how panning is utilized in
    /// [`pty_mux`] without corrupting parser state see
    /// - Why horizontal panning is safe.
    ///
    /// [`pty_mux`]: crate::core::pty::pty_mux
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    /// [Viewport Mechanics (Scroll vs Pan)]:
    ///     crate::core::pty::pty_mux#viewport-mechanics-scroll-vs-pan
    /// [Viewport Mechanics (Scroll vs Pan)]:
    /// - Why we avoid vertical panning in favor of external scrolling.
    fn try_pan_viewport_to(
        &mut self,
        origin_pos: crate::CPos,
    ) -> Result<(), ViewportPanValidity> {
        let validity = check_pan_validity(&self.viewport, origin_pos);
        let ViewportPanValidity::ValidHorizontalOnly = validity else {
            return Err(validity);
        };

        self.viewport.set_origin_pos(|pos| *pos = origin_pos);

        ok!()
    }

    fn shift_lines_in_range(
        &mut self,
        direction: ShiftLinesDirection,
        row_index_range: RangeExclusive<VPRow>,
        amount: VPLength,
        fill_char: PixelChar,
    ) {
        if self.viewport.contains_range(&row_index_range) != RangeValidityStatus::Valid {
            return;
        }

        if amount.is_empty() {
            return;
        }

        let abs_range = self.viewport.to_canvas(row_index_range);
        let start = abs_range.start.as_usize();
        let end = abs_range.end.as_usize();
        let amount = amount.as_usize();

        // Acquire a contiguous slice of the target row range from the underlying VecDeque
        // to shift rows in a single batch operation via slice rotation (`rotate_left` /
        // `rotate_right`). This avoids costly element-by-element bubble-up swaps across
        // line boundaries.
        let slice = &mut self.lines.make_contiguous()[start..end];
        let rotate_by = min(amount, slice.len());

        match direction {
            ShiftLinesDirection::Up => {
                slice.rotate_left(rotate_by);

                // Fill the bottom with empty lines
                let fill_start = max(start, end.saturating_sub(amount));
                for row_idx in fill_start..end {
                    if let Some(line) = self.lines.get_mut(row_idx) {
                        line.pixel_chars.fill(fill_char);
                    }
                }
            }
            ShiftLinesDirection::Down => {
                slice.rotate_right(rotate_by);

                // Fill the top with empty lines
                let fill_end = min(start + amount, end);
                for row_idx in start..fill_end {
                    if let Some(line) = self.lines.get_mut(row_idx) {
                        line.pixel_chars.fill(fill_char);
                    }
                }
            }
        }
    }

    /// Trims the oldest lines from the top of the buffer to respect the configured limit
    /// after new empty lines are pushed to the bottom of the canvas.
    ///
    /// This is the exact point where scrolling becomes a **destructive** operation for
    /// a [`GrowableBuffer`]: if the total buffer capacity exceeds the user's configured
    /// limits, the oldest lines at the top are permanently dropped.
    ///
    /// ```text
    ///  Max Capacity = limit + size.row_height
    ///
    ///             1         2         3         4         5
    ///   01234567890123456789012345678901234567890123456789012
    ///  ┌─────────────────────────────────────────────────────┐  ← Top of buffer
    /// 0│ POP ←   Oldest line if buffer capacity exceeded     │  ▲
    /// 1│                                                     │  │ scrollback_limit
    /// 2│ vp.           (Scrollback History)                  │  │
    /// 3│ history_                                            │  ▼
    /// 4│ len()   → ┌───────────────────────────┐             │  ← Viewport Top
    /// 5│           │ ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ │             │  ▲
    /// 6│           │ ▒▒▒▒▒▒Viewport▒▒▒▒▒▒▒▒▒▒▒ │             │  │ vp.size
    /// 7│           │ ▒▒▒▒▒▒Visible Screen▒▒▒▒▒ │             │  │ .row_height
    /// 8│           │ ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ │             │  ▼
    /// 9│           └───────────────────────────┘             │  ← Bottom of buffer
    ///  └─────────────────────────────────────────────────────┘
    /// ```
    ///
    /// The `scrollback_limit` configuration dictates how many lines of history we are
    /// allowed to keep *above* the active screen. Therefore, the absolute maximum
    /// capacity of the `lines` buffer is the `scrollback_limit` plus the viewport height
    /// of the terminal window itself.
    ///
    /// Whenever a line is popped from the top, we must concurrently call
    /// [`Viewport::decrement_history_len`] so that the viewport stays correctly
    /// synchronized with the new canvas size.
    fn allocate_new_lines_at_bottom(
        &mut self,
        arg_amount: impl Into<VPLength>,
        fill_char: PixelChar,
    ) {
        // Add new lines at the bottom of the buffer, filling them with `fill_char`.
        let amount: VPLength = arg_amount.into();
        let amount = amount.as_usize();

        // Reserve capacity to avoid multiple reallocations during push_back.
        self.lines.reserve(amount);

        // Push new empty lines to the bottom of the buffer and increment the viewport's
        // history_len.
        for _ in 0..amount {
            let new_line =
                PixelCharLine::new_empty(*self.viewport.get_width(), fill_char);
            self.lines.push_back(new_line);
            self.viewport.increment_history_len();
        }

        // Enforce the storage line limit by popping lines from the top if necessary.
        self.trim_lines_to_storage_line_limit();
    }

    fn clear_viewport_with(&mut self, fill_char: PixelChar) {
        self.fill_row_range(self.viewport.get_viewport_row_range(), fill_char);
    }

    fn fill_row_range(
        &mut self,
        row_index_range: RangeExclusive<VPRow>,
        fill_char: PixelChar,
    ) {
        if self.viewport.contains_range(&row_index_range) != RangeValidityStatus::Valid {
            return;
        }

        let abs_range = self.viewport.to_canvas(row_index_range);

        for line in self.lines.range_mut(abs_range.to_raw()) {
            line.pixel_chars.fill(fill_char);
        }
    }

    fn swap_lines(
        &mut self,
        row_index_1: VPRow,
        row_index_2: VPRow,
    ) -> miette::Result<()> {
        if self.viewport.contains_row(row_index_1) != RangeBoundsResult::Within
            || self.viewport.contains_row(row_index_2) != RangeBoundsResult::Within
        {
            return Err(miette::miette!("Row index out of bounds"));
        }

        let abs_row_1 = self.viewport.to_canvas(row_index_1);
        let abs_row_2 = self.viewport.to_canvas(row_index_2);
        self.lines.swap(abs_row_1.as_usize(), abs_row_2.as_usize());

        ok!()
    }
}

/// Status indicating whether a proposed pan operation on an [`CanvasStorage`]
/// [`Viewport`] is strictly horizontal or attempts an invalid vertical pan.
///
/// [`CanvasStorage`]: crate::CanvasStorage
/// [`Viewport`]: crate::Viewport
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportPanValidity {
    ValidHorizontalOnly,
    InvalidVerticalPan {
        expected_row: CRow,
        requested_row: CRow,
    },
}

/// Validates whether a proposed pan operation on an [`CanvasStorage`] viewport is
/// strictly horizontal (i.e., the requested origin's `row_index` matches the viewport's
/// current origin `row_index`).
///
/// [`CanvasStorage`]: crate::CanvasStorage
#[must_use]
fn check_pan_validity(
    viewport: &Viewport,
    requested_origin_pos: CPos,
) -> ViewportPanValidity {
    let expected_row = viewport.get_origin_pos().row_index;
    if expected_row == requested_origin_pos.row_index {
        ViewportPanValidity::ValidHorizontalOnly
    } else {
        ViewportPanValidity::InvalidVerticalPan {
            expected_row,
            requested_row: requested_origin_pos.row_index,
        }
    }
}

#[cfg(any(test, doc))]
pub mod test_fixture_growable_buffer_for_conformance_tests {
    use super::*;
    use std::collections::VecDeque;

    pub trait TestGrowableBufferExt {
        fn get_lines(&self) -> &VecDeque<PixelCharLine>;
        fn get_lines_mut(&mut self) -> &mut VecDeque<PixelCharLine>;
        fn get_viewport(&self) -> &Viewport;
        fn get_viewport_mut(&mut self) -> &mut Viewport;
        fn get_storage_line_limit(&self) -> StorageLineLimit;
    }

    impl TestGrowableBufferExt for GrowableBuffer {
        fn get_lines(&self) -> &VecDeque<PixelCharLine> { &self.lines }
        fn get_lines_mut(&mut self) -> &mut VecDeque<PixelCharLine> { &mut self.lines }
        fn get_viewport(&self) -> &Viewport { &self.viewport }
        fn get_viewport_mut(&mut self) -> &mut Viewport { &mut self.viewport }
        fn get_storage_line_limit(&self) -> StorageLineLimit { self.storage_line_limit }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{c_pos, vp_col, vp_height, vp_len, vp_row, vp_width};

    fn create_viewport() -> Viewport {
        Viewport::from((c_pos(0, 5), vp_width(80) + vp_height(24)))
    }

    #[test]
    fn test_viewport_pan_validity() {
        let vp = create_viewport(); // origin is (c_col(0), c_row(5))

        // Valid horizontal pan (same row index)
        let horiz_pan = crate::c_pos(15, 5);
        assert_eq!(
            check_pan_validity(&vp, horiz_pan),
            ViewportPanValidity::ValidHorizontalOnly
        );

        // Invalid vertical pan (different row index)
        let vert_pan = crate::c_pos(0, 10);
        assert_eq!(
            check_pan_validity(&vp, vert_pan),
            ViewportPanValidity::InvalidVerticalPan {
                expected_row: crate::c_row(5),
                requested_row: crate::c_row(10),
            }
        );
    }

    #[test]
    fn test_growable_buffer_bounds_check_get_row() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        // Before scroll, bounds [0, 10) are valid
        assert!(buffer.get_row(vp_row(0)).is_some());
        assert!(buffer.get_row(vp_row(9)).is_some());
        assert!(buffer.get_row(vp_row(10)).is_none());

        // Perform an allocation to create scrollback history (now length = 11, viewport
        // row_index = 1)
        buffer.allocate_new_lines_at_bottom(vp_len(1), PixelChar::default());

        // Viewport relative indices are still [0, 10)
        assert!(buffer.get_row(vp_row(0)).is_some());
        assert!(buffer.get_row(vp_row(9)).is_some());

        // Out-of-viewport relative indices (e.g. index 10) must return None, even though
        // absolute index 10 exists in the underlying buffer.
        assert!(buffer.get_row(vp_row(10)).is_none());
    }

    #[test]
    fn test_growable_buffer_bounds_check_swap_lines() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        // Valid swap within viewport
        assert!(buffer.swap_lines(vp_row(0), vp_row(9)).is_ok());

        // Out of bounds swaps
        assert!(buffer.swap_lines(vp_row(0), vp_row(10)).is_err());
        assert!(buffer.swap_lines(vp_row(10), vp_row(11)).is_err());
    }

    #[test]
    fn test_growable_buffer_bounds_check_shift_lines() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        // Valid shift within range
        buffer.shift_lines_in_range(
            ShiftLinesDirection::Up,
            vp_row(0)..vp_row(10),
            vp_len(1),
            PixelChar::default(),
        );

        // Out of bounds range should be silently ignored (verify by checking it does not
        // panic or corrupt)
        buffer.shift_lines_in_range(
            ShiftLinesDirection::Up,
            vp_row(0)..vp_row(11),
            vp_len(1),
            PixelChar::default(),
        );
    }

    #[test]
    fn test_growable_buffer_clear_scrollback() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );
        buffer.allocate_new_lines_at_bottom(vp_len(5), PixelChar::default());

        assert_eq!(buffer.lines.len(), 15);
        assert_eq!(buffer.viewport.get_history_len(), 5);

        buffer.clear_scrollback();

        assert_eq!(buffer.lines.len(), 10);
        assert_eq!(buffer.viewport.get_history_len(), 0);
    }

    #[test]
    fn test_growable_buffer_enforce_scrollback_limit() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(5), // Viewport height is 5
            StorageLineLimit::Fixed(10), // History limit is 10
            PixelChar::Spacer,
        );

        // Initial state: buffer has 5 lines (viewport only)
        assert_eq!(buffer.lines.len(), 5);
        assert_eq!(buffer.viewport.get_history_len(), 0);

        // Scroll up by 8 lines (less than limit of 10)
        buffer.allocate_new_lines_at_bottom(vp_len(8), PixelChar::default());

        // Total lines should be 8 (history) + 5 (viewport) = 13.
        assert_eq!(buffer.lines.len(), 13);
        assert_eq!(buffer.viewport.get_history_len(), 8);

        // Scroll up by 5 more lines (total 13 history lines, exceeds limit of 10!)
        buffer.allocate_new_lines_at_bottom(vp_len(5), PixelChar::default());

        // Max lines = 10 (limit) + 5 (viewport height) = 15. The buffer should have
        // popped 3 lines from the top.
        assert_eq!(buffer.lines.len(), 15);

        // History length should be exactly the limit (10).
        assert_eq!(buffer.viewport.get_history_len(), 10);
    }

    #[test]
    fn test_growable_buffer_trim_lines_to_storage_line_limit_direct() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(5),
            StorageLineLimit::Fixed(2),
            PixelChar::Spacer,
        );

        // Allocate 5 history lines (exceeds limit of 2)
        buffer.allocate_new_lines_at_bottom(vp_len(5), PixelChar::default());
        assert_eq!(buffer.viewport.get_history_len(), 2);
        assert_eq!(buffer.lines.len(), 7);

        // Explicitly calling trim when already within limit is a no-op
        buffer.trim_lines_to_storage_line_limit();
        assert_eq!(buffer.viewport.get_history_len(), 2);
        assert_eq!(buffer.lines.len(), 7);
    }

    fn set_row_char(buffer: &mut GrowableBuffer, row_idx: VPRow, ch: char) {
        let style = crate::TuiStyle::default();
        if let Some(row_data) = buffer.get_row_mut(row_idx) {
            row_data.fill(PixelChar::PlainText {
                display_char: ch,
                style,
            });
        }
    }

    fn check_row_char(buffer: &mut GrowableBuffer, row_idx: VPRow, ch: char) -> bool {
        let style = crate::TuiStyle::default();
        let expected = PixelChar::PlainText {
            display_char: ch,
            style,
        };
        if let Some(row_data) = buffer.get_row(row_idx) {
            row_data.iter().all(|&c| c == expected)
        } else {
            false
        }
    }

    #[test]
    fn test_growable_buffer_get_row_mut() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        // Out of bounds mutation should return None
        assert!(buffer.get_row_mut(vp_row(10)).is_none());

        // In bounds mutation should work
        assert!(buffer.get_row_mut(vp_row(5)).is_some());

        set_row_char(&mut buffer, vp_row(5), 'A');
        assert!(check_row_char(&mut buffer, vp_row(5), 'A'));
    }

    #[test]
    fn test_growable_buffer_get_row_with_scrollback() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        set_row_char(&mut buffer, vp_row(0), 'A');

        // Scroll up by 2 lines
        buffer.allocate_new_lines_at_bottom(vp_len(2), PixelChar::default());
        set_row_char(&mut buffer, vp_row(0), 'B');

        // Now 'A' is at scrollback offset 2
        let style = crate::TuiStyle::default();
        let expected_a = PixelChar::PlainText {
            display_char: 'A',
            style,
        };
        let expected_b = PixelChar::PlainText {
            display_char: 'B',
            style,
        };

        let row_a = buffer
            .get_row_with_scrollback(vp_row(0), scrollback_amount(2))
            .expect("conversion error");
        assert!(row_a.iter().all(|&c| c == expected_a));

        let row_b = buffer
            .get_row_with_scrollback(vp_row(0), scrollback_amount(0))
            .expect("conversion error");
        assert!(row_b.iter().all(|&c| c == expected_b));

        // Clamping check: passing an excessively large scrollback amount clamps to
        // history_len (2)
        let row_clamped = buffer
            .get_row_with_scrollback(vp_row(0), scrollback_amount(100))
            .expect("conversion error");
        assert!(row_clamped.iter().all(|&c| c == expected_a));
    }

    #[test]
    fn test_growable_buffer_get_row_with_scrollback_mut() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        set_row_char(&mut buffer, vp_row(0), 'A');
        buffer.allocate_new_lines_at_bottom(vp_len(2), PixelChar::default());
        set_row_char(&mut buffer, vp_row(0), 'B');

        // Mutate line at scrollback offset 2 (originally 'A')
        let style = crate::TuiStyle::default();
        let modified_char = PixelChar::PlainText {
            display_char: 'X',
            style,
        };

        if let Some(row_mut) =
            buffer.get_row_with_scrollback_mut(vp_row(0), scrollback_amount(2))
        {
            row_mut[0] = modified_char;
        }

        // Verify mutation persisted in scrollback history
        let row_a = buffer
            .get_row_with_scrollback(vp_row(0), scrollback_amount(2))
            .expect("conversion error");
        assert_eq!(row_a[0], modified_char);
    }

    #[test]
    fn test_growable_buffer_clear_viewport_with() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        set_row_char(&mut buffer, vp_row(0), 'A');
        set_row_char(&mut buffer, vp_row(9), 'B');

        let style = crate::TuiStyle::default();
        let clear_char = PixelChar::PlainText {
            display_char: 'C',
            style,
        };
        buffer.clear_viewport_with(clear_char);

        assert!(check_row_char(&mut buffer, vp_row(0), 'C'));
        assert!(check_row_char(&mut buffer, vp_row(9), 'C'));
    }

    #[test]
    fn test_growable_buffer_swap_lines_data_validation() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        set_row_char(&mut buffer, vp_row(2), 'A');
        set_row_char(&mut buffer, vp_row(5), 'B');

        buffer
            .swap_lines(vp_row(2), vp_row(5))
            .expect("conversion error");

        assert!(check_row_char(&mut buffer, vp_row(2), 'B'));
        assert!(check_row_char(&mut buffer, vp_row(5), 'A'));
    }

    #[test]
    fn test_growable_buffer_shift_lines_data_validation() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        set_row_char(&mut buffer, vp_row(1), 'A');
        set_row_char(&mut buffer, vp_row(2), 'B');
        set_row_char(&mut buffer, vp_row(3), 'C');

        let style = crate::TuiStyle::default();
        let empty_char = PixelChar::PlainText {
            display_char: 'E',
            style,
        };

        // Shift Up by 1
        buffer.shift_lines_in_range(
            ShiftLinesDirection::Up,
            vp_row(1)..vp_row(4),
            vp_len(1),
            empty_char,
        );

        // Old row 2 ('B') -> row 1
        // Old row 3 ('C') -> row 2
        // row 3 -> empty
        assert!(check_row_char(&mut buffer, vp_row(1), 'B'));
        assert!(check_row_char(&mut buffer, vp_row(2), 'C'));
        assert!(check_row_char(&mut buffer, vp_row(3), 'E'));

        // Reset
        set_row_char(&mut buffer, vp_row(5), 'A');
        set_row_char(&mut buffer, vp_row(6), 'B');
        set_row_char(&mut buffer, vp_row(7), 'C');

        // Shift Down by 1
        buffer.shift_lines_in_range(
            ShiftLinesDirection::Down,
            vp_row(5)..vp_row(8),
            vp_len(1),
            empty_char,
        );

        // Old row 5 ('A') -> row 6
        // Old row 6 ('B') -> row 7
        // row 5 -> empty
        assert!(check_row_char(&mut buffer, vp_row(5), 'E'));
        assert!(check_row_char(&mut buffer, vp_row(6), 'A'));
        assert!(check_row_char(&mut buffer, vp_row(7), 'B'));

        // Shift by 0 should do nothing
        buffer.shift_lines_in_range(
            ShiftLinesDirection::Down,
            vp_row(5)..vp_row(8),
            vp_len(0),
            empty_char,
        );
        assert!(check_row_char(&mut buffer, vp_row(5), 'E'));
        assert!(check_row_char(&mut buffer, vp_row(6), 'A'));
        assert!(check_row_char(&mut buffer, vp_row(7), 'B'));
    }

    #[test]
    fn test_growable_buffer_unlimited_scrollback() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(5),
            StorageLineLimit::Unlimited,
            PixelChar::Spacer,
        );

        // Allocate 1000 lines
        buffer.allocate_new_lines_at_bottom(vp_len(1000), PixelChar::default());

        // Buffer should have 1005 lines and 1000 history
        assert_eq!(buffer.lines.len(), 1005);
        assert_eq!(buffer.viewport.get_history_len(), 1000);
    }

    #[test]
    fn test_growable_buffer_get_mem_size() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        // A newly allocated buffer should have a memory size > 0.
        let initial_size = buffer.get_mem_size();
        assert!(initial_size > 0);

        // Allocating more lines should increase the memory size.
        buffer.allocate_new_lines_at_bottom(vp_len(5), PixelChar::default());
        let new_size = buffer.get_mem_size();
        assert!(new_size > initial_size);
    }

    #[test]
    fn test_growable_buffer_pan_viewport_to() {
        let mut buffer = GrowableBuffer::new_empty(
            vp_width(80) + vp_height(10),
            StorageLineLimit::Fixed(100),
            PixelChar::Spacer,
        );

        // Initial viewport origin is (0,0)
        assert_eq!(buffer.viewport.get_origin_pos(), crate::c_pos(0, 0));

        // Valid horizontal pan succeeds
        let valid_pos = vp_col(15) + vp_row(0);
        assert_eq!(
            buffer.try_pan_viewport_to(crate::c_pos(
                valid_pos.col_index.as_u16(),
                valid_pos.row_index.as_u16()
            )),
            Ok(())
        );
        assert_eq!(
            buffer.viewport.get_origin_pos(),
            crate::c_pos(valid_pos.col_index.as_u16(), valid_pos.row_index.as_u16())
        );

        // Invalid vertical pan fails and leaves viewport unchanged
        let invalid_pos = vp_col(15) + vp_row(5);
        assert_eq!(
            buffer.try_pan_viewport_to(crate::c_pos(
                invalid_pos.col_index.as_u16(),
                invalid_pos.row_index.as_u16()
            )),
            Err(ViewportPanValidity::InvalidVerticalPan {
                expected_row: crate::c_row(0),
                requested_row: crate::c_row(5),
            })
        );
        assert_eq!(
            buffer.viewport.get_origin_pos(),
            crate::c_pos(valid_pos.col_index.as_u16(), valid_pos.row_index.as_u16())
        );
    }
}

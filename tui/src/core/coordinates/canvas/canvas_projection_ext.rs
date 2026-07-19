// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: Method overloading pattern

//! Type-safe coordinate projection operations for [`Viewport`].
//!
//! To provide a unified API for 1D row ([`VPRow`], [`CRow`]), 1D column ([`VPCol`],
//! [`CCol`]), 2D position ([`VPPos`], [`CPos`]), and 1D range projections, this module
//! uses trait-based method overloading so single methods on [`Viewport`] seamlessly
//! handle all coordinate and range types.
//!
//! # Method Overloading Pattern
//!
//! Rust does not support traditional method overloading like C++ or Java. Instead, Rust
//! uses Generic Trait Parameterization (Ad-hoc Polymorphism) to enable method
//! overloading:
//!
//! 1. This is done by parameterizing generic extension traits [`ViewportToCanvasExt`] and
//!    [`CanvasToViewportExt`] over a type parameter `TargetCoord`. The overloaded methods
//!    in these extension traits accept arguments of this `TargetCoord` type.
//! 2. Which allows [`ViewportToCanvasExt<TargetCoord>`] and
//!    [`CanvasToViewportExt<TargetCoord>`] to be implemented for multiple `TargetCoord`
//!    types on the same struct [`Viewport`].
//! 3. For each `TargetCoord` type, we have a separate implementation of [`to_canvas`] or
//!    [`to_viewport`].
//! 4. Thus enabling method overloading for [`to_canvas`] and [`to_viewport`], which
//!    allows these methods to be called on [`Viewport`] with different coordinate and
//!    range types, each of which has its own implementation of the method. The call sites
//!    are clean, ergonomic, and type-safe, and the compiler resolves the correct
//!    implementation based on the type of the argument passed in.
//!
//! # Performance & Zero Cost
//!
//! Calls to [`to_canvas`] and [`to_viewport`] use **static dispatch
//! (monomorphization)**. The compiler resolves the target coordinate type at compile time
//! and generates direct function calls with zero vtable runtime overhead.
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::{
//!     CanvasToViewportExt, Viewport, ViewportToCanvasExt, VPSize, c_col, c_pos, c_row,
//!     vp_col, vp_height, vp_pos, vp_row, vp_width,
//! };
//!
//! let viewport = Viewport::new(c_pos(10, 20), VPSize::new((vp_width(80), vp_height(24))));
//!
//! // 1D Viewport -> Canvas Coordinate Projection
//! assert_eq!(viewport.to_canvas(vp_row(5)), c_row(25));
//! assert_eq!(viewport.to_canvas(vp_col(10)), c_col(20));
//! assert_eq!(viewport.to_canvas(vp_pos(10, 5)), c_pos(20, 25));
//!
//! // 1D Canvas -> Viewport Coordinate Projection
//! assert_eq!(viewport.to_viewport(c_row(25)), Some(vp_row(5)));
//! assert_eq!(viewport.to_viewport(c_col(20)), Some(vp_col(10)));
//! assert_eq!(viewport.to_viewport(c_pos(20, 25)), Some(vp_pos(10, 5)));
//!
//! // Coordinates outside viewport bounds return None
//! assert_eq!(viewport.to_viewport(c_row(10)), None); // In history
//! assert_eq!(viewport.to_viewport(c_col(5)), None);  // Off screen left
//!
//! // Range Projections
//! assert_eq!(viewport.to_canvas(vp_row(0)..vp_row(10)), c_row(20)..c_row(30));
//! assert_eq!(
//!     viewport.to_viewport(c_row(20)..c_row(30)),
//!     Some(vp_row(0)..vp_row(10))
//! );
//! ```
//!
//! [`to_canvas`]: ViewportToCanvasExt::to_canvas
//! [`to_viewport`]: CanvasToViewportExt::to_viewport

use crate::{ArrayBoundsCheck, ArrayOverflowResult, CCol, CPos, CRow, NarrowingCastToU16,
            RangeExclusive, VPCol, VPPos, VPRow, Viewport, c_col, c_pos, c_row, vp_col,
            vp_pos, vp_row};

/// Extension trait for projecting Viewport coordinates and ranges into Canvas
/// coordinates.
///
/// Implemented on [`Viewport`] for [`VPRow`], [`VPCol`], [`VPPos`],
/// [`RangeExclusive<VPRow>`], and [`RangeExclusive<VPCol>`].
pub trait ViewportToCanvasExt<TargetCoord> {
    type CanvasResult;

    /// Projects a [`Viewport`] coordinate or range into a [`Canvas`] coordinate or
    /// range.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    /// [`Viewport`]: crate::Viewport
    fn to_canvas(&self, target: TargetCoord) -> Self::CanvasResult;
}

/// Extension trait for projecting Canvas coordinates and ranges into Viewport
/// coordinates.
///
/// Implemented on [`Viewport`] for [`CRow`], [`CCol`], [`CPos`],
/// [`RangeExclusive<CRow>`], and [`RangeExclusive<CCol>`].
pub trait CanvasToViewportExt<TargetCoord> {
    type ViewportResult;

    /// Projects a [`Canvas`] coordinate or range into a [`Viewport`] coordinate or
    /// range.
    ///
    /// Returns [`None`] if the target coordinate falls outside the visible viewport
    /// window.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    /// [`Viewport`]: crate::Viewport
    fn to_viewport(&self, target: TargetCoord) -> Option<Self::ViewportResult>;
}

mod impl_viewport_to_canvas_ext {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl ViewportToCanvasExt<VPRow> for Viewport {
        type CanvasResult = CRow;

        /// Translates a row index in **Viewport Coordinates (Viewport-Relative)** to an
        /// absolute row index in **Canvas Coordinates (Canvas-Absolute)**.
        ///
        /// **Row Address Calculation**
        ///
        /// In a terminal buffer layout, lines from 0 up to `history_len` represent
        /// scrollback history above the screen.
        ///
        /// Translating a viewport relative row index to an absolute canvas row index adds
        /// the history length (`history_len`). For example, with `history_len = 4` (4
        /// lines of scrollback history), [`VPRow(0)`] maps directly to [`CRow(4)`].
        ///
        /// ```text
        ///    Canvas Storage Buffer                              Row Address Calculation
        ///   ┌────────────────────────────────────────────────┐
        ///  0│ (History Line 0)                               │  ← Canvas Row 0
        ///  1│ (History Line 1)                               │  ▲
        ///  2│ (History Line 2)                               │  │ history_len = 4
        ///  3│ (History Line 3)                               │  ▼
        ///   ├────────────────────────────────────────────────┤  ← Canvas Row 4 (history_len)
        ///  4│ Viewport Row 0 ◄───── Target (Canvas Row 4)    │  ▲
        ///  5│ Viewport Row 1        [relative_row_index = 0] │  │
        ///  6│ Viewport Row 2                                 │  │ Viewport Height = 4
        ///  7│ Viewport Row 3                                 │  ▼
        ///   └────────────────────────────────────────────────┘  ← Viewport Bottom
        ///
        ///   Target Canvas Row = history_len (4) + relative_row_index (0)
        ///                      = CRow(4)
        /// ```
        ///
        /// [`CRow(4)`]: crate::CRow
        /// [`VPRow(0)`]: crate::VPRow
        #[inline]
        fn to_canvas(&self, target: VPRow) -> Self::CanvasResult {
            let history_len = self.get_history_len();
            let abs_row_index = history_len + target.as_usize();
            c_row(abs_row_index)
        }
    }

    impl ViewportToCanvasExt<VPCol> for Viewport {
        type CanvasResult = CCol;

        /// Translates a col index in **Viewport Coordinates (Viewport-Relative)** to an
        /// absolute col index in **Canvas Coordinates (Canvas-Absolute)**.
        ///
        /// **Col Address Calculation**
        ///
        /// When the viewport is panned horizontally, columns from 0 up to `col_offset`
        /// sit off screen to the left.
        ///
        /// Translating a viewport relative column index to an absolute canvas column
        /// index adds the horizontal origin offset (`col_offset`). For example, with:
        /// - `col_offset = 8` (panned right by 8 columns),
        /// - [`VPCol(0)`] maps directly to [`CCol(8)`].
        ///
        /// ```text
        /// Canvas Storage Buffer (Horizontal Columns)
        ///
        ///         ▼  1         2
        ///  0123467│89012345678901234567
        /// ┌───────┼────────────────────┐
        /// │Panned │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
        /// │Offset │  Viewport Window   │
        /// └───────┴────────────────────┘
        ///  ▲       ▲                    ▲
        ///  │       │                    │
        ///  │       Canvas Col 8         Canvas Col 28 (Exclusive End)
        ///  │       (col_offset = 8)     [Last Col = 27]
        ///  │       Viewport Col 0       (Right Edge)
        ///  │       ▲
        ///  │       │
        ///  │       Target: Canvas Col 8
        ///  │       [relative_col_index = 0]
        ///  │
        ///  Canvas Col 0
        ///  (Left Edge)
        ///
        /// Target Canvas Col = col_offset (8) + relative_col_index (0)
        ///                   = CCol(8)
        /// ```
        ///
        /// [`CCol(8)`]: crate::CCol
        /// [`VPCol(0)`]: crate::VPCol
        #[inline]
        fn to_canvas(&self, target: VPCol) -> Self::CanvasResult {
            let col_offset = self.get_origin_pos().col_index.as_usize();
            let abs_col_index = col_offset + target.as_usize();
            c_col(abs_col_index)
        }
    }

    impl ViewportToCanvasExt<VPPos> for Viewport {
        type CanvasResult = CPos;

        /// Translates a 2D position in **Viewport Coordinates (Viewport-Relative)** to an
        /// absolute 2D position in **Canvas Coordinates (Canvas-Absolute)**.
        #[inline]
        fn to_canvas(&self, target: VPPos) -> Self::CanvasResult {
            c_pos(
                self.to_canvas(target.col_index),
                self.to_canvas(target.row_index),
            )
        }
    }

    impl ViewportToCanvasExt<RangeExclusive<VPRow>> for Viewport {
        type CanvasResult = RangeExclusive<CRow>;

        /// Translates a row range in **Viewport Coordinates (Viewport-Relative)** to an
        /// absolute row range in **Canvas Coordinates (Canvas-Absolute)**.
        #[inline]
        fn to_canvas(&self, target: RangeExclusive<VPRow>) -> Self::CanvasResult {
            let start_idx = self.to_canvas(target.start);
            let end_idx = self.to_canvas(target.end);
            start_idx..end_idx
        }
    }

    impl ViewportToCanvasExt<&RangeExclusive<VPRow>> for Viewport {
        type CanvasResult = RangeExclusive<CRow>;

        /// Translates a row range reference in **Viewport Coordinates
        /// (Viewport-Relative)** to an absolute row range in **Canvas Coordinates
        /// (Canvas-Absolute)**.
        #[inline]
        fn to_canvas(&self, target: &RangeExclusive<VPRow>) -> Self::CanvasResult {
            self.to_canvas(target.start..target.end)
        }
    }

    impl ViewportToCanvasExt<RangeExclusive<VPCol>> for Viewport {
        type CanvasResult = RangeExclusive<CCol>;

        /// Translates a col range in **Viewport Coordinates (Viewport-Relative)** to an
        /// absolute col range in **Canvas Coordinates (Canvas-Absolute)**.
        #[inline]
        fn to_canvas(&self, target: RangeExclusive<VPCol>) -> Self::CanvasResult {
            let start_idx = self.to_canvas(target.start);
            let end_idx = self.to_canvas(target.end);
            start_idx..end_idx
        }
    }

    impl ViewportToCanvasExt<&RangeExclusive<VPCol>> for Viewport {
        type CanvasResult = RangeExclusive<CCol>;

        /// Translates a col range reference in **Viewport Coordinates
        /// (Viewport-Relative)** to an absolute col range in **Canvas Coordinates
        /// (Canvas-Absolute)**.
        #[inline]
        fn to_canvas(&self, target: &RangeExclusive<VPCol>) -> Self::CanvasResult {
            self.to_canvas(target.start..target.end)
        }
    }
}

mod impl_canvas_to_viewport_ext {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CanvasToViewportExt<CRow> for Viewport {
        type ViewportResult = VPRow;

        /// Translates an absolute row index in **Canvas Coordinates (Canvas-Absolute)**
        /// to a relative row index in **Viewport Coordinates (Viewport-Relative)**.
        ///
        /// **Row Address Calculation**
        ///
        /// Translating an absolute canvas row index to a relative viewport row index
        /// subtracts the history length (`history_len`). It requires the target canvas
        /// row to fall within the visible window bounds (between `history_len` and
        /// `history_len + height`).
        ///
        /// For example, with `history_len = 4` and `height = 4`:
        /// - [`CRow(3)`] returns [`None`] (it is in scrollback history).
        /// - [`CRow(5)`] maps directly to [`VPRow(1)`].
        ///
        /// ```text
        ///    Canvas Storage Buffer                              Row Address Calculation
        ///   ┌────────────────────────────────────────────────┐
        ///  0│ (History Line 0)                               │  ← Canvas Row 0
        ///  1│ (History Line 1)                               │  ▲
        ///  2│ (History Line 2)                               │  │ history_len = 4
        ///  3│ (History Line 3) ◄─── Target (None, in history)│  ▼
        ///   ├────────────────────────────────────────────────┤  ← Canvas Row 4 (history_len)
        ///  4│ Viewport Row 0                                 │  ▲
        ///  5│ Viewport Row 1 ◄───── Target (Canvas Row 5)    │  │
        ///  6│ Viewport Row 2        [relative_row_index = 1] │  │ Viewport Height = 4
        ///  7│ Viewport Row 3                                 │  ▼
        ///   └────────────────────────────────────────────────┘  ← Viewport Bottom
        ///
        ///   Target Viewport Row = Canvas Row (5) - history_len (4)
        ///                       = VPRow(1)
        /// ```
        ///
        /// Returns [`Some(VPRow)`] if the canvas row falls within the visible
        /// viewport window, or [`None`] if it is in scrollback history or below the
        /// visible window.
        ///
        /// [`CRow(3)`]: crate::CRow
        /// [`CRow(5)`]: crate::CRow
        /// [`Some(VPRow)`]: crate::VPRow
        /// [`VPRow(1)`]: crate::VPRow
        #[inline]
        fn to_viewport(&self, target: CRow) -> Option<Self::ViewportResult> {
            let history_len = self.get_history_len();
            let rel_row_index_usize = target.as_usize().checked_sub(history_len)?;
            let rel_row_index = vp_row(rel_row_index_usize.as_u16_narrowing());
            if rel_row_index.overflows(self.get_height()) == ArrayOverflowResult::Within {
                Some(rel_row_index)
            } else {
                None
            }
        }
    }

    impl CanvasToViewportExt<CCol> for Viewport {
        type ViewportResult = VPCol;

        /// Translates an absolute col index in **Canvas Coordinates (Canvas-Absolute)**
        /// to a relative col index in **Viewport Coordinates (Viewport-Relative)**.
        ///
        /// **Col Address Calculation**
        ///
        /// Translating an absolute canvas column index to a relative viewport column
        /// index subtracts the horizontal origin offset (`col_offset`). It requires the
        /// target canvas column to fall within the visible window (between `col_offset`
        /// and `col_offset + width`).
        ///
        /// For example, with `col_offset = 8` and `width = 20`:
        /// - `CCol(5)` returns `None` (it is panned off-screen to the left).
        /// - `CCol(10)` maps directly to `VPCol(2)`.
        ///
        /// ```text
        /// Canvas Storage Buffer (Horizontal Columns)
        ///
        ///         ▼  1         2
        ///  0123467│89012345678901234567
        /// ┌───────┼────────────────────┐
        /// │Panned │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
        /// │Offset │  Viewport Window   │
        /// └───────┴────────────────────┘
        ///  ▲ ▲     ▲ ▲                  ▲
        ///  │ │     │ │                  │
        ///  │ │     │ │                  Canvas Col 28 (Exclusive End)
        ///  │ │     │ Target: Canvas Col 10
        ///  │ │     │ [relative_col_index = 2]
        ///  │ │     │
        ///  │ │     Canvas Col 8 (col_offset = 8)
        ///  │ │     Viewport Col 0
        ///  │ │
        ///  │ Target: Canvas Col 5 (None, off-screen left)
        ///  │
        ///  Canvas Col 0
        ///  (Left Edge)
        ///
        /// Target Viewport Col = Canvas Col (10) - col_offset (8)
        ///                     = VPCol(2)
        /// ```
        ///
        /// Returns `Some(VPCol)` if the canvas col falls within the visible viewport
        /// window, or `None` if it is to the left or right of the visible window.
        #[inline]
        fn to_viewport(&self, target: CCol) -> Option<Self::ViewportResult> {
            let origin_col_index = self.get_origin_pos().col_index.as_usize();
            let rel_col_index_usize = target.as_usize().checked_sub(origin_col_index)?;
            let rel_col_index = vp_col(rel_col_index_usize.as_u16_narrowing());
            if rel_col_index.overflows(self.get_width()) == ArrayOverflowResult::Within {
                Some(rel_col_index)
            } else {
                None
            }
        }
    }

    impl CanvasToViewportExt<CPos> for Viewport {
        type ViewportResult = VPPos;

        /// Translates an absolute 2D position in **Canvas Coordinates (Canvas-Absolute)**
        /// to a relative 2D position in **Viewport Coordinates (Viewport-Relative)**.
        ///
        /// Returns `Some(VPPos)` if both row and column coordinates fall within the
        /// visible viewport window, or `None` otherwise.
        #[inline]
        fn to_viewport(&self, target: CPos) -> Option<Self::ViewportResult> {
            let col_index = self.to_viewport(target.col_index)?;
            let row_index = self.to_viewport(target.row_index)?;
            Some(vp_pos(col_index, row_index))
        }
    }

    impl CanvasToViewportExt<RangeExclusive<CRow>> for Viewport {
        type ViewportResult = RangeExclusive<VPRow>;

        /// Translates an absolute row range in **Canvas Coordinates (Canvas-Absolute)**
        /// to a relative row range in **Viewport Coordinates (Viewport-Relative)**.
        ///
        /// Returns `Some(RangeExclusive<VPRow>)` if both start and end fall within the
        /// visible viewport window, or `None` otherwise.
        #[inline]
        fn to_viewport(
            &self,
            target: RangeExclusive<CRow>,
        ) -> Option<Self::ViewportResult> {
            let start_vp = self.to_viewport(target.start)?;
            let end_vp = self.to_viewport(target.end)?;
            Some(start_vp..end_vp)
        }
    }

    impl CanvasToViewportExt<&RangeExclusive<CRow>> for Viewport {
        type ViewportResult = RangeExclusive<VPRow>;

        /// Translates an absolute row range reference in **Canvas Coordinates
        /// (Canvas-Absolute)** to a relative row range in **Viewport Coordinates
        /// (Viewport-Relative)**.
        ///
        /// Returns `Some(RangeExclusive<VPRow>)` if both start and end fall within the
        /// visible viewport window, or `None` otherwise.
        #[inline]
        fn to_viewport(
            &self,
            target: &RangeExclusive<CRow>,
        ) -> Option<Self::ViewportResult> {
            self.to_viewport(target.start..target.end)
        }
    }

    impl CanvasToViewportExt<RangeExclusive<CCol>> for Viewport {
        type ViewportResult = RangeExclusive<VPCol>;

        /// Translates an absolute col range in **Canvas Coordinates (Canvas-Absolute)**
        /// to a relative col range in **Viewport Coordinates (Viewport-Relative)**.
        ///
        /// Returns `Some(RangeExclusive<VPCol>)` if both start and end fall within the
        /// visible viewport window, or `None` otherwise.
        #[inline]
        fn to_viewport(
            &self,
            target: RangeExclusive<CCol>,
        ) -> Option<Self::ViewportResult> {
            let start_vp = self.to_viewport(target.start)?;
            let end_vp = self.to_viewport(target.end)?;
            Some(start_vp..end_vp)
        }
    }

    impl CanvasToViewportExt<&RangeExclusive<CCol>> for Viewport {
        type ViewportResult = RangeExclusive<VPCol>;

        /// Translates an absolute col range reference in **Canvas Coordinates
        /// (Canvas-Absolute)** to a relative col range in **Viewport Coordinates
        /// (Viewport-Relative)**.
        ///
        /// Returns `Some(RangeExclusive<VPCol>)` if both start and end fall within the
        /// visible viewport window, or `None` otherwise.
        #[inline]
        fn to_viewport(
            &self,
            target: &RangeExclusive<CCol>,
        ) -> Option<Self::ViewportResult> {
            self.to_viewport(target.start..target.end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VPSize, c_col, c_pos, c_row, vp_col, vp_height, vp_pos, vp_row, vp_width};

    #[test]
    fn test_to_canvas_1d_row() {
        let viewport =
            Viewport::new(c_pos(0, 10), VPSize::new((vp_width(80), vp_height(24))));
        assert_eq!(viewport.to_canvas(vp_row(0)), c_row(10));
        assert_eq!(viewport.to_canvas(vp_row(5)), c_row(15));
    }

    #[test]
    fn test_to_canvas_1d_col() {
        let viewport =
            Viewport::new(c_pos(10, 0), VPSize::new((vp_width(80), vp_height(24))));
        assert_eq!(viewport.to_canvas(vp_col(0)), c_col(10));
        assert_eq!(viewport.to_canvas(vp_col(5)), c_col(15));
    }

    #[test]
    fn test_to_canvas_2d_pos() {
        let viewport =
            Viewport::new(c_pos(10, 20), VPSize::new((vp_width(80), vp_height(24))));
        assert_eq!(viewport.to_canvas(vp_pos(0, 0)), c_pos(10, 20));
        assert_eq!(viewport.to_canvas(vp_pos(5, 5)), c_pos(15, 25));
    }

    #[test]
    fn test_to_canvas_ranges() {
        let viewport =
            Viewport::new(c_pos(10, 20), VPSize::new((vp_width(80), vp_height(24))));

        // By value
        assert_eq!(
            viewport.to_canvas(vp_row(0)..vp_row(5)),
            c_row(20)..c_row(25)
        );
        assert_eq!(
            viewport.to_canvas(vp_col(0)..vp_col(5)),
            c_col(10)..c_col(15)
        );

        // By reference
        let row_range = vp_row(2)..vp_row(7);
        let col_range = vp_col(3)..vp_col(8);
        assert_eq!(viewport.to_canvas(&row_range), c_row(22)..c_row(27));
        assert_eq!(viewport.to_canvas(&col_range), c_col(13)..c_col(18));
    }

    #[test]
    fn test_to_viewport_1d_row() {
        let viewport =
            Viewport::new(c_pos(0, 10), VPSize::new((vp_width(80), vp_height(24))));
        assert_eq!(viewport.to_viewport(c_row(10)), Some(vp_row(0)));
        assert_eq!(viewport.to_viewport(c_row(15)), Some(vp_row(5)));
        assert_eq!(viewport.to_viewport(c_row(9)), None); // In history
        assert_eq!(viewport.to_viewport(c_row(34)), None); // Below visible window
    }

    #[test]
    fn test_to_viewport_1d_col() {
        let viewport =
            Viewport::new(c_pos(10, 0), VPSize::new((vp_width(80), vp_height(24))));
        assert_eq!(viewport.to_viewport(c_col(10)), Some(vp_col(0)));
        assert_eq!(viewport.to_viewport(c_col(15)), Some(vp_col(5)));
        assert_eq!(viewport.to_viewport(c_col(9)), None); // Left of viewport
        assert_eq!(viewport.to_viewport(c_col(90)), None); // Right of viewport
    }

    #[test]
    fn test_to_viewport_2d_pos() {
        let viewport =
            Viewport::new(c_pos(10, 20), VPSize::new((vp_width(80), vp_height(24))));
        assert_eq!(viewport.to_viewport(c_pos(10, 20)), Some(vp_pos(0, 0)));
        assert_eq!(viewport.to_viewport(c_pos(15, 25)), Some(vp_pos(5, 5)));
        assert_eq!(viewport.to_viewport(c_pos(5, 20)), None);
        assert_eq!(viewport.to_viewport(c_pos(10, 10)), None);
    }

    #[test]
    fn test_to_viewport_ranges() {
        let viewport =
            Viewport::new(c_pos(10, 20), VPSize::new((vp_width(80), vp_height(24))));

        // Valid ranges (by value & by ref)
        assert_eq!(
            viewport.to_viewport(c_row(20)..c_row(25)),
            Some(vp_row(0)..vp_row(5))
        );
        assert_eq!(
            viewport.to_viewport(c_col(10)..c_col(15)),
            Some(vp_col(0)..vp_col(5))
        );

        let row_range = c_row(22)..c_row(27);
        let col_range = c_col(13)..c_col(18);
        assert_eq!(viewport.to_viewport(&row_range), Some(vp_row(2)..vp_row(7)));
        assert_eq!(viewport.to_viewport(&col_range), Some(vp_col(3)..vp_col(8)));

        // Invalid ranges (out of bounds)
        assert_eq!(viewport.to_viewport(c_row(15)..c_row(25)), None);
        assert_eq!(viewport.to_viewport(c_col(5)..c_col(15)), None);
    }
}

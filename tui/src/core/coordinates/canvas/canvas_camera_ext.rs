// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: Method overloading pattern

//! Type-safe camera operations for [`Viewport`] panning and coordinate projection.
//!
//! To provide a unified API for 1D row ([`CRow`]), 1D column ([`CCol`]), and 2D position
//! ([`CPos`]) coordinates, this module uses method overloading so a single method on
//! [`Viewport`] seamlessly handles all three coordinate types.
//!
//! # Method Overloading Pattern
//!
//! Rust does not support traditional method overloading like C++ or Java. Instead, Rust
//! uses Generic Trait Parameterization (Ad-hoc Polymorphism) in order to allow method
//! overloading:
//!
//! 1. This is done by parameterizing a generic extension trait [`CanvasCameraExt`] over a
//!    type parameter `InputCoord`. The overloaded methods in this extension trait
//!    **must** accept arguments of this `InputCoord` type. If they don't then the
//!    compiler won't be able to disambiguate which implementation to use.
//! 2. Which allows [`CanvasCameraExt<InputCoord>`] to be implemented for multiple
//!    `InputCoord` types on the same struct [`Viewport`].
//! 3. For each `InputCoord` type ([`CRow`], [`CCol`], [`CPos`]) we must have a separate
//!    implementation of the trait methods [`pan_to_keep_coord_in_view`] and [`to_vp`].
//! 4. Thus enabling method overloading for [`pan_to_keep_coord_in_view`] and [`to_vp`]
//!    which allows these methods to be called on [`Viewport`] with different coordinate
//!    types, each of which has its own implementation of the method. However the call
//!    sites are the same (clean and ergonomic) and the compiler resolves the correct
//!    implementation based on the type of the argument passed in.
//!
//! By doing this, we get a single, unified API on [`Viewport`] for both panning and
//! projection.
//!
//! ```
//! use r3bl_tui::{
//!     CanvasCameraExt, Viewport, VPSize, c_col, c_pos, c_row,
//!     vp_col, vp_height, vp_pos, vp_row, vp_width,
//! };
//!
//! let mut viewport = Viewport::new(c_pos(0, 0), VPSize::new((vp_width(80), vp_height(24))));
//!
//! // 1D Horizontal Panning (CCol -> VPCol)
//! let target_col = c_col(10);
//! viewport.pan_to_keep_coord_in_view(target_col);
//! let res_col = viewport.to_vp(target_col);
//! assert_eq!(res_col, vp_col(10));
//!
//! // 1D Vertical Panning (CRow -> VPRow)
//! let target_row = c_row(5);
//! viewport.pan_to_keep_coord_in_view(target_row);
//! let res_row = viewport.to_vp(target_row);
//! assert_eq!(res_row, vp_row(5));
//!
//! // 2D Position Panning (CPos -> VPPos)
//! let target_pos = c_pos(15, 10);
//! viewport.pan_to_keep_coord_in_view(target_pos);
//! let res_pos = viewport.to_vp(target_pos);
//! assert_eq!(res_pos, vp_pos(15, 10));
//! ```
//!
//! # Performance & Zero Cost
//!
//! Calls to [`pan_to_keep_coord_in_view`] and [`to_vp`] use **static dispatch
//! (monomorphization)**. The compiler resolves the target coordinate type at compile time
//! and generates direct function calls with zero vtable runtime overhead.
//!
//! [`pan_to_keep_coord_in_view`]: CanvasCameraExt::pan_to_keep_coord_in_view
//! [`to_vp`]: CanvasCameraExt::to_vp

use crate::{CCol, CPos, CRow, NarrowingCastToU16, RangeBoundsResult, VPCol, VPPos,
            VPRow, Viewport, ViewportBoundsCheck, vp_col, vp_pos, vp_row};

/// Extension trait for Viewport camera operations (panning & projection).
///
/// Implemented on [`Viewport`] for [`CRow`], [`CCol`], and [`CPos`] canvas coordinates.
pub trait CanvasCameraExt<InputCoord> {
    type ViewportResult;

    /// Pans the camera (viewport) by adjusting its origin coordinate on the [`Canvas`]
    /// just enough to ensure the `coord` position is included within the visible
    /// frame.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    fn pan_to_keep_coord_in_view(&mut self, coord: InputCoord);

    /// Projects a [`Canvas`] coordinate into a [`Viewport`] coordinate.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    /// [`Viewport`]: crate::Viewport
    fn to_vp(&self, coord: InputCoord) -> Self::ViewportResult;
}

impl CanvasCameraExt<CRow> for Viewport {
    type ViewportResult = VPRow;

    /// Handles vertical panning by mutating `self` to ensure a target row remains
    /// visible.
    ///
    /// Checks whether the `coord` row is within the visible viewport bounds:
    /// - If above the viewport: adjusts `self` (the origin) to exactly match the `coord`.
    /// - If below the viewport: adjusts `self` so the `coord` sits exactly on the
    ///   bottom-most visible edge (`coord - vp_height + 1`).
    /// - If within the viewport: does nothing.
    ///
    /// ```text
    ///                         0
    ///                       0 ┌───────────────────┐
    ///                         │                   │
    ///                         │  above viewport   │ ← coord
    ///                         │                   │   (< vp_origin)
    /// vp_origin             → ├───────────────────┤ ┬
    ///                         │         ▲         │ │
    ///                         │         │         │ │
    ///                         │    within vp      │ │ vp height (row_height)
    ///                         │         │         │ │
    ///                         │         ▼         │ │
    /// vp_origin             → ├───────────────────┤ ┴
    /// + vp height             │                   │
    ///                         │  below viewport   │ ← coord
    ///                         │                   │   (>= vp_origin + vp height)
    ///                         └───────────────────┘
    /// ```
    fn pan_to_keep_coord_in_view(&mut self, coord: CRow) {
        let current_origin_row = self.get_origin_pos().row_index;
        let vp_height = self.get_height();
        let new_origin_row =
            match coord.check_viewport_bounds(current_origin_row, vp_height) {
                RangeBoundsResult::Underflowed => coord,
                RangeBoundsResult::Within => current_origin_row,
                RangeBoundsResult::Overflowed => {
                    let max_visible_offset = vp_height - 1;
                    coord - max_visible_offset
                }
            };
        self.set_origin_pos(|pos| pos.row_index = new_origin_row);
    }

    fn to_vp(&self, coord: CRow) -> Self::ViewportResult {
        let origin = self.get_origin_pos().row_index;
        let distance = coord.as_usize().saturating_sub(origin.as_usize());

        debug_assert!(
            u16::try_from(distance).is_ok(),
            "CRow distance exceeds Viewport limits (u16::MAX). Was pan_to_keep_coord_in_view called?"
        );

        vp_row(distance.as_u16_narrowing())
    }
}

impl CanvasCameraExt<CCol> for Viewport {
    type ViewportResult = VPCol;

    /// Handles horizontal panning by mutating `self` to ensure a target column remains
    /// visible.
    ///
    /// Checks whether the `coord` column is within the visible viewport bounds:
    /// - If left of the viewport: adjusts `self` (the origin) to exactly match the
    ///   `coord`.
    /// - If right of the viewport: adjusts `self` so the `coord` sits exactly on the
    ///   rightmost visible edge (`coord - vp_width + 1`).
    /// - If within the viewport: does nothing.
    ///
    /// ```text
    ///           ╭─── vp width ───╮
    /// ╭0────────┼────────────────┼─────────→
    /// 0         │                │
    /// │ left of │←  within vp   →│ right of
    /// │         │                │
    /// ╰─────────┴────────────────┴─────────→
    ///           ↑                ↑
    ///        vp_origin     vp_origin + vp width
    /// ```
    fn pan_to_keep_coord_in_view(&mut self, coord: CCol) {
        let current_origin_col = self.get_origin_pos().col_index;
        let vp_width = self.get_width();
        let new_origin_col =
            match coord.check_viewport_bounds(current_origin_col, vp_width) {
                RangeBoundsResult::Underflowed => coord,
                RangeBoundsResult::Within => current_origin_col,
                RangeBoundsResult::Overflowed => {
                    let max_visible_offset = vp_width - 1;
                    coord - max_visible_offset
                }
            };
        self.set_origin_pos(|pos| pos.col_index = new_origin_col);
    }

    fn to_vp(&self, coord: CCol) -> Self::ViewportResult {
        let origin = self.get_origin_pos().col_index;
        let distance = coord.as_usize().saturating_sub(origin.as_usize());

        debug_assert!(
            u16::try_from(distance).is_ok(),
            "CCol distance exceeds Viewport limits (u16::MAX). Was pan_to_keep_coord_in_view called?"
        );

        vp_col(distance.as_u16_narrowing())
    }
}

impl CanvasCameraExt<CPos> for Viewport {
    type ViewportResult = VPPos;

    /// Convenience method to pan both the vertical and horizontal camera origin
    /// simultaneously by mutating `self` to ensure the `coord` position remains
    /// visible.
    fn pan_to_keep_coord_in_view(&mut self, coord: CPos) {
        self.pan_to_keep_coord_in_view(coord.row_index);
        self.pan_to_keep_coord_in_view(coord.col_index);
    }

    fn to_vp(&self, coord: CPos) -> Self::ViewportResult {
        vp_pos(self.to_vp(coord.col_index), self.to_vp(coord.row_index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VPSize, c_col, c_pos, c_row, vp_col, vp_height, vp_pos, vp_row, vp_width};

    #[test]
    fn test_viewport_canvas_camera_ext_row() {
        let mut vp =
            Viewport::from((c_pos(0, 10), VPSize::new((vp_width(5), vp_height(5)))));

        // 1. Target within visible viewport (10..15)
        vp.pan_to_keep_coord_in_view(c_row(12));
        assert_eq!(vp.get_origin_pos().row_index, c_row(10));
        assert_eq!(vp.to_vp(c_row(12)), vp_row(2));

        // Top edge
        vp.pan_to_keep_coord_in_view(c_row(10));
        assert_eq!(vp.get_origin_pos().row_index, c_row(10));
        assert_eq!(vp.to_vp(c_row(10)), vp_row(0));

        // Bottom edge (14)
        vp.pan_to_keep_coord_in_view(c_row(14));
        assert_eq!(vp.get_origin_pos().row_index, c_row(10));
        assert_eq!(vp.to_vp(c_row(14)), vp_row(4));

        // 2. Target above viewport -> pan up
        vp.pan_to_keep_coord_in_view(c_row(8));
        assert_eq!(vp.get_origin_pos().row_index, c_row(8));
        assert_eq!(vp.to_vp(c_row(8)), vp_row(0));

        // 3. Target below viewport -> pan down
        vp.pan_to_keep_coord_in_view(c_row(20));
        assert_eq!(vp.get_origin_pos().row_index, c_row(16));
        assert_eq!(vp.to_vp(c_row(20)), vp_row(4));
    }

    #[test]
    fn test_viewport_canvas_camera_ext_col() {
        let mut vp =
            Viewport::from((c_pos(10, 0), VPSize::new((vp_width(5), vp_height(5)))));

        // 1. Target within visible viewport (10..15)
        vp.pan_to_keep_coord_in_view(c_col(12));
        assert_eq!(vp.get_origin_pos().col_index, c_col(10));
        assert_eq!(vp.to_vp(c_col(12)), vp_col(2));

        // 2. Target left of viewport -> pan left
        vp.pan_to_keep_coord_in_view(c_col(5));
        assert_eq!(vp.get_origin_pos().col_index, c_col(5));
        assert_eq!(vp.to_vp(c_col(5)), vp_col(0));

        // 3. Target right of viewport -> pan right
        vp.pan_to_keep_coord_in_view(c_col(20));
        assert_eq!(vp.get_origin_pos().col_index, c_col(16));
        assert_eq!(vp.to_vp(c_col(20)), vp_col(4));
    }

    #[test]
    fn test_viewport_canvas_camera_ext_pos() {
        let mut vp =
            Viewport::from((c_pos(10, 10), VPSize::new((vp_width(5), vp_height(5)))));

        vp.pan_to_keep_coord_in_view(c_pos(12, 12));
        assert_eq!(vp.get_origin_pos(), c_pos(10, 10));
        assert_eq!(vp.to_vp(c_pos(12, 12)), vp_pos(2, 2));

        vp.pan_to_keep_coord_in_view(c_pos(5, 20));
        assert_eq!(vp.get_origin_pos(), c_pos(5, 16));
        assert_eq!(vp.to_vp(c_pos(5, 20)), vp_pos(0, 4));
    }

    #[test]
    fn test_viewport_canvas_camera_ext_target_behind_origin() {
        let vp =
            Viewport::from((c_pos(10, 10), VPSize::new((vp_width(5), vp_height(5)))));

        // When target is behind current origin without panning first, to_vp saturates to
        // 0
        assert_eq!(vp.to_vp(c_row(5)), vp_row(0));
        assert_eq!(vp.to_vp(c_col(5)), vp_col(0));
        assert_eq!(vp.to_vp(c_pos(5, 5)), vp_pos(0, 0));
    }
}

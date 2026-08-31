// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CCol, CPos, CRow, CanvasRangeExt, RangeBoundsExt, RangeBoundsResult,
            RangeConstructExt, RangeExclusive, RangeValidityStatus, VPCol, VPHeight,
            VPRow, VPSize, VPWidth, c_pos, c_row, vp_col, vp_height, vp_row, vp_width};

/// Represents the visible 2D screen window scrolling or sliding or panning over an
/// underlying canvas (like a terminal scrollback buffer).
///
/// This is the 2D projection of the canvas that is currently visible:
/// - The visible projection is bound by [`u16`] dimensions, which is the maximum size of
///   a terminal window.
/// - The canvas however can be much larger than the viewport and is only bounded by
///   [`usize`] dimensions.
///
/// Coordinate conversions between Viewport-relative and Canvas-absolute coordinates can
/// be performed using inherent methods or trait-based method overloading via
/// [`ViewportToCanvasExt`] and [`CanvasToViewportExt`] (see also [`CanvasCameraExt`] for
/// camera panning operations).
///
/// See the [Canvas and Viewport concept] for details on how this coordinates with the
/// underlying [`Canvas`]. The separation between Canvas coordinates ([`usize`]) and
/// Viewport coordinates ([`u16`]) implements the "Parse, don't validate" principle and
/// the Newtype and Typestate patterns, guaranteeing runtime faultlessness and
/// invalidation of illegal states with zero performance penalty. For more information,
/// see [academic research on type safety at scale] ([theoretical foundations], [empirical
/// benchmarks]).
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [`CanvasCameraExt`]: crate::CanvasCameraExt
/// [`CanvasToViewportExt`]: crate::CanvasToViewportExt
/// [`ViewportToCanvasExt`]: crate::ViewportToCanvasExt
/// [academic research on type safety at scale]:
///     mod@crate::core::coordinates::canvas#academic-research-on-type-safety-at-scale
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
/// [empirical benchmarks]: mod@crate::core::coordinates::canvas#empirical-benchmarks
/// [theoretical foundations]: mod@crate::core::coordinates::canvas#theoretical-foundations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, Default)]
pub struct Viewport {
    origin_pos: CPos,
    size: VPSize,
}

/// Viewport origin and dimensions.
impl Viewport {
    /// Creates a new [`Viewport`] with the specified origin and size.
    #[inline]
    #[must_use]
    pub fn new(origin_pos: CPos, size: VPSize) -> Self { Self { origin_pos, size } }

    /// Returns the height of the visible viewport as a strongly-typed
    /// [`VPHeight`].
    ///
    /// [`VPHeight`]: crate::VPHeight
    #[inline]
    #[must_use]
    pub fn get_height(&self) -> VPHeight { vp_height(self.size.row_height) }

    /// Returns the width of the visible viewport as a strongly-typed
    /// [`VPWidth`].
    ///
    /// [`VPWidth`]: crate::VPWidth
    #[inline]
    #[must_use]
    pub fn get_width(&self) -> VPWidth { vp_width(self.size.col_width) }

    /// Returns the origin coordinate of the viewport on the canvas.
    ///
    /// The `row_index` of the returned position acts as the history length or offset,
    /// while the `col_index` indicates the current horizontal panning offset.
    #[inline]
    #[must_use]
    pub fn get_origin_pos(&self) -> CPos { self.origin_pos }

    /// Mutates the origin coordinate of the viewport on the canvas using a closure.
    ///
    /// This handles origin coordinate translations for actions like horizontal panning
    /// or synchronized offset updates.
    #[inline]
    pub fn set_origin_pos(&mut self, mut fn_mut: impl FnMut(&mut CPos)) {
        fn_mut(&mut self.origin_pos);
    }

    /// Returns the dimensions ([`VPSize`]) of the visible viewport.
    #[inline]
    #[must_use]
    pub fn get_size(&self) -> VPSize { self.size }

    /// Mutates the dimensions ([`VPSize`]) of the visible viewport using a closure.
    #[inline]
    pub fn set_size(&mut self, mut fn_mut: impl FnMut(&mut VPSize)) {
        fn_mut(&mut self.size);
    }
}

/// History length management.
impl Viewport {
    /// Returns the number of canvas rows that exist above the viewport. In a terminal
    /// context, this is equivalent to the scrollback history length.
    ///
    /// Because the viewport natively stays anchored to the bottom of the canvas, its top
    /// edge (`pos.row_index`) accurately counts the number of lines above it.
    #[inline]
    #[must_use]
    pub fn get_history_len(&self) -> usize { self.get_origin_pos().row_index.as_usize() }

    /// Decreases the history length of the viewport.
    ///
    /// This should be called when lines are pushed out of the scrollback buffer from the
    /// top of the canvas. This effectively anchors the viewport to the newly shifted
    /// canvas, keeping the view stationary relative to the bottom.
    #[inline]
    pub fn decrement_history_len(&mut self) {
        self.set_origin_pos(|pos| pos.row_index -= 1);
    }

    /// Increases the history length of the viewport.
    ///
    /// This should be called when new lines are rendered at the bottom of the canvas,
    /// pushing the viewport further down. This effectively anchors the viewport to the
    /// bottom of the newly expanded canvas.
    #[inline]
    pub fn increment_history_len(&mut self) {
        self.set_origin_pos(|pos| pos.row_index += 1);
    }

    /// Resets the history length of the viewport to zero.
    ///
    /// This should be called when the terminal scrollback is completely erased,
    /// effectively pulling the viewport to the absolute top of the canvas.
    #[inline]
    pub fn reset_history_len(&mut self) {
        self.set_origin_pos(|pos| pos.row_index = c_row(0usize));
    }
}

/// Range validation and 2D index bounds.
impl Viewport {
    /// Validates whether a row index in **Viewport Coordinates (Viewport-Relative)**
    /// falls within the viewport height.
    #[inline]
    #[must_use]
    pub fn contains_row(&self, row_index: VPRow) -> RangeBoundsResult {
        self.get_viewport_row_range()
            .to_raw()
            .check_index_is_within(row_index)
    }

    /// Validates whether a col index in **Viewport Coordinates (Viewport-Relative)**
    /// falls within the viewport width.
    #[inline]
    #[must_use]
    pub fn contains_col(&self, col_index: VPCol) -> RangeBoundsResult {
        self.get_viewport_col_range()
            .to_raw()
            .check_index_is_within(col_index)
    }

    /// Validates whether a row index range in **Viewport Coordinates
    /// (Viewport-Relative)** is valid for the viewport height.
    #[inline]
    #[must_use]
    pub fn contains_row_range(
        &self,
        row_index_range: &RangeExclusive<VPRow>,
    ) -> RangeValidityStatus {
        row_index_range
            .to_raw()
            .check_range_is_valid_for_length(*self.get_height())
    }

    /// Validates whether a col index range in **Viewport Coordinates
    /// (Viewport-Relative)** is valid for the viewport width.
    #[inline]
    #[must_use]
    pub fn contains_col_range(
        &self,
        col_index_range: &RangeExclusive<VPCol>,
    ) -> RangeValidityStatus {
        col_index_range
            .to_raw()
            .check_range_is_valid_for_length(*self.get_width())
    }

    /// Validates whether a row index range in **Viewport Coordinates
    /// (Viewport-Relative)** is valid for the viewport height.
    ///
    /// Alias for [`Self::contains_row_range`].
    #[inline]
    #[must_use]
    pub fn contains_range(
        &self,
        row_index_range: &RangeExclusive<VPRow>,
    ) -> RangeValidityStatus {
        self.contains_row_range(row_index_range)
    }

    /// Returns the full range of row indices in **Viewport Coordinates
    /// (Viewport-Relative)** (`0..height`).
    #[inline]
    #[must_use]
    pub fn get_viewport_row_range(&self) -> RangeExclusive<VPRow> {
        (vp_row(0), self.get_height()).to_exclusive_range()
    }

    /// Returns the full range of col indices in **Viewport Coordinates
    /// (Viewport-Relative)** (`0..width`).
    #[inline]
    #[must_use]
    pub fn get_viewport_col_range(&self) -> RangeExclusive<VPCol> {
        (vp_col(0), self.get_width()).to_exclusive_range()
    }
}

mod impl_viewport_from {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<(VPSize, CPos)> for Viewport {
        #[inline]
        fn from((size, origin_pos): (VPSize, CPos)) -> Viewport {
            Viewport { origin_pos, size }
        }
    }

    impl From<(CPos, VPSize)> for Viewport {
        #[inline]
        fn from((origin_pos, size): (CPos, VPSize)) -> Viewport {
            Viewport { origin_pos, size }
        }
    }

    impl From<(CRow, CCol, VPWidth, VPHeight)> for Viewport {
        #[inline]
        fn from(
            (row_index, col_index, col_width, row_height): (
                CRow,
                CCol,
                VPWidth,
                VPHeight,
            ),
        ) -> Viewport {
            let origin = c_pos(col_index, row_index);
            let size = col_width + row_height;
            (origin, size).into()
        }
    }
}

#[cfg(test)]
pub mod test_fixture_viewport {
    use super::*;

    pub trait TestViewportExt {
        fn set_history_len(&mut self, len: usize);
    }

    impl TestViewportExt for Viewport {
        fn set_history_len(&mut self, len: usize) {
            self.set_origin_pos(|pos| pos.row_index = c_row(len));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CanvasToViewportExt, ScrollbackAmount, ViewportToCanvasExt, c_col,
                vp_height, vp_pos, vp_width};
    use test_fixture_viewport::TestViewportExt;

    fn create_viewport() -> Viewport {
        Viewport::from((c_pos(0, 5), VPSize::new((vp_width(80), vp_height(24)))))
    }

    #[test]
    fn test_viewport_creation_and_accessors() {
        let vp = create_viewport();

        assert_eq!(vp.get_history_len(), 5);
        assert_eq!(vp.get_width(), vp_width(80));
        assert_eq!(vp.get_height(), vp_height(24));
        assert_eq!(vp.get_size(), VPSize::new((vp_width(80), vp_height(24))));

        let origin = vp.get_origin_pos();
        assert_eq!(origin.col_index, c_col(0));
        assert_eq!(origin.row_index, c_row(5));

        // Test Viewport::new constructor
        let new_vp = Viewport::new(origin, VPSize::new((vp_width(80), vp_height(24))));
        assert_eq!(new_vp, vp);
        assert_eq!(new_vp.get_height(), vp_height(24));
        assert_eq!(new_vp.get_width(), vp_width(80));

        // Test set_size
        let mut vp_mut = vp;
        let new_size = VPSize::new((vp_width(100), vp_height(40)));
        vp_mut.set_size(|size| *size = new_size);
        assert_eq!(vp_mut.get_size(), new_size);
    }

    #[test]
    fn test_viewport_from_conversions() {
        let size = VPSize::new((vp_width(80), vp_height(24)));
        let pos = c_pos(0, 5);

        // Test From<(Size, CPos)>
        let vp1: Viewport = (size, pos).into();
        assert_eq!(vp1.get_width(), vp_width(80));
        assert_eq!(vp1.get_height(), vp_height(24));
        assert_eq!(vp1.get_origin_pos(), pos);

        // Test From<(CPos, Size)>
        let vp2: Viewport = (pos, size).into();
        assert_eq!(vp2.get_width(), vp_width(80));
        assert_eq!(vp2.get_height(), vp_height(24));
        assert_eq!(vp2.get_origin_pos(), pos);

        // Test From<(CRow, CCol, VPWidth, VPHeight)>
        let vp3: Viewport = (c_row(5), c_col(0), vp_width(80), vp_height(24)).into();
        assert_eq!(vp3.get_width(), vp_width(80));
        assert_eq!(vp3.get_height(), vp_height(24));
        assert_eq!(vp3.get_origin_pos(), pos);
    }

    #[test]
    fn test_increment_and_decrement_history_len() {
        let mut vp = create_viewport();

        vp.increment_history_len();
        assert_eq!(vp.get_history_len(), 6);

        vp.decrement_history_len();
        assert_eq!(vp.get_history_len(), 5);

        // Decrement down to 0
        vp.decrement_history_len();
        vp.decrement_history_len();
        vp.decrement_history_len();
        vp.decrement_history_len();
        vp.decrement_history_len();
        assert_eq!(vp.get_history_len(), 0);

        // Decrement below 0 should have no effect
        vp.decrement_history_len();
        assert_eq!(vp.get_history_len(), 0);
    }

    #[test]
    fn test_reset_history_len() {
        let mut vp = create_viewport();

        assert_eq!(vp.get_history_len(), 5);
        vp.reset_history_len();
        assert_eq!(vp.get_history_len(), 0);
    }

    #[test]
    fn test_set_history_len() {
        let mut vp = create_viewport();

        vp.set_history_len(10);
        assert_eq!(vp.get_history_len(), 10);
    }

    #[test]
    fn test_set_origin_pos() {
        let mut vp = create_viewport();

        let new_origin = c_pos(10, 20);
        vp.set_origin_pos(|pos| *pos = new_origin);

        assert_eq!(vp.get_origin_pos(), new_origin);
        assert_eq!(vp.get_history_len(), 20); // History length is derived from row_index
    }

    #[test]
    fn test_contains_row() {
        let vp = create_viewport(); // height is 24 (row 0 to 23 inclusive)

        assert_eq!(vp.contains_row(vp_row(0)), RangeBoundsResult::Within);
        assert_eq!(vp.contains_row(vp_row(23)), RangeBoundsResult::Within);
        assert_eq!(vp.contains_row(vp_row(24)), RangeBoundsResult::Overflowed);
        assert_eq!(vp.contains_row(vp_row(100)), RangeBoundsResult::Overflowed);
    }

    #[test]
    fn test_contains_col() {
        let vp = create_viewport(); // width is 80 (col 0 to 79 inclusive)

        assert_eq!(vp.contains_col(vp_col(0)), RangeBoundsResult::Within);
        assert_eq!(vp.contains_col(vp_col(79)), RangeBoundsResult::Within);
        assert_eq!(vp.contains_col(vp_col(80)), RangeBoundsResult::Overflowed);
    }

    #[test]
    fn test_contains_range() {
        let vp = create_viewport(); // height is 24

        // Valid ranges
        assert_eq!(
            vp.contains_row_range(&(vp_row(0)..vp_row(24))),
            RangeValidityStatus::Valid
        );
        assert_eq!(
            vp.contains_range(&(vp_row(5)..vp_row(10))),
            RangeValidityStatus::Valid
        );

        // Invalid ranges (exceeding height or inverted)
        assert_eq!(
            vp.contains_row_range(&(vp_row(0)..vp_row(25))),
            RangeValidityStatus::EndOutOfBounds
        );
        assert_eq!(
            vp.contains_row_range(&(vp_row(10)..vp_row(5))),
            RangeValidityStatus::Inverted
        );
    }

    #[test]
    fn test_contains_col_range() {
        let vp = create_viewport(); // width is 80

        // Valid ranges
        assert_eq!(
            vp.contains_col_range(&(vp_col(0)..vp_col(80))),
            RangeValidityStatus::Valid
        );
        assert_eq!(
            vp.contains_col_range(&(vp_col(10)..vp_col(50))),
            RangeValidityStatus::Valid
        );

        // Invalid ranges (exceeding width or inverted)
        assert_eq!(
            vp.contains_col_range(&(vp_col(0)..vp_col(81))),
            RangeValidityStatus::EndOutOfBounds
        );
        assert_eq!(
            vp.contains_col_range(&(vp_col(50)..vp_col(10))),
            RangeValidityStatus::Inverted
        );
    }

    #[test]
    fn test_viewport_row_and_col_ranges() {
        let vp = create_viewport(); // height = 24, width = 80

        assert_eq!(vp.get_viewport_row_range(), vp_row(0)..vp_row(24));
        assert_eq!(vp.get_viewport_col_range(), vp_col(0)..vp_col(80));
    }

    #[test]
    fn test_2d_row_address_translation() {
        let vp = create_viewport(); // history_len = 5

        assert_eq!(vp.to_canvas(vp_row(0)), c_row(5));
        assert_eq!(vp.to_canvas(vp_row(3)), c_row(8));

        assert_eq!(vp.to_canvas(vp_row(0)..vp_row(3)), c_row(5)..c_row(8));
    }

    #[test]
    fn test_2d_col_address_translation() {
        let vp =
            Viewport::from((c_pos(10, 5), VPSize::new((vp_width(80), vp_height(24))))); // col_offset = 10, history_len = 5

        assert_eq!(vp.to_canvas(vp_col(0)), c_col(10));
        assert_eq!(vp.to_canvas(vp_col(5)), c_col(15));

        assert_eq!(vp.to_canvas(vp_col(0)..vp_col(5)), c_col(10)..c_col(15));
    }

    #[test]
    fn test_row_address_translation_with_scrollback() {
        let vp = create_viewport(); // history_len = 5

        // Zero scrollback
        assert_eq!(
            ScrollbackAmount::from(0u16).to_c_row(&vp, vp_row(0)),
            c_row(5)
        );
        // Scroll back by 2 lines
        assert_eq!(
            ScrollbackAmount::from(2u16).to_c_row(&vp, vp_row(0)),
            c_row(3)
        );
        // Scroll back exceeding history_len (e.g. 10) is clamped to history_len (5)
        assert_eq!(
            ScrollbackAmount::from(10u16).to_c_row(&vp, vp_row(0)),
            c_row(0usize)
        );
    }

    #[test]
    fn test_canvas_to_viewport_translation() {
        let vp =
            Viewport::from((c_pos(10, 5), VPSize::new((vp_width(80), vp_height(24))))); // history_len = 5, col_offset = 10, height = 24, width = 80

        // On-screen row conversions (c_row 5..29)
        assert_eq!(vp.to_viewport(c_row(5)), Some(vp_row(0)));
        assert_eq!(vp.to_viewport(c_row(8)), Some(vp_row(3)));
        assert_eq!(vp.to_viewport(c_row(28)), Some(vp_row(23)));

        // Off-screen row conversions (history or below window)
        assert_eq!(vp.to_viewport(c_row(4)), None); // In history
        assert_eq!(vp.to_viewport(c_row(29)), None); // Below screen

        // Row range conversion
        assert_eq!(
            vp.to_viewport(c_row(5)..c_row(8)),
            Some(vp_row(0)..vp_row(3))
        );
        assert_eq!(vp.to_viewport(c_row(4)..c_row(8)), None);

        // Col conversions (origin col is 10, width is 80 -> c_col 10..90)
        assert_eq!(vp.to_viewport(c_col(10)), Some(vp_col(0)));
        assert_eq!(vp.to_viewport(c_col(89)), Some(vp_col(79)));
        assert_eq!(vp.to_viewport(c_col(90)), None);
        assert_eq!(vp.to_viewport(c_col(9)), None); // Left of viewport

        // Col range conversion
        assert_eq!(
            vp.to_viewport(c_col(10)..c_col(15)),
            Some(vp_col(0)..vp_col(5))
        );
        assert_eq!(vp.to_viewport(c_col(5)..c_col(15)), None);

        // Scrollback reverse conversion
        assert_eq!(
            ScrollbackAmount::from(2u16).to_viewport_row(&vp, c_row(3)),
            Some(vp_row(0))
        );
        assert_eq!(
            ScrollbackAmount::from(2u16).to_viewport_row(&vp, c_row(2)),
            None
        );
    }

    #[test]
    fn test_2d_canvas_and_viewport_pos_translation() {
        let vp =
            Viewport::from((c_pos(10, 5), VPSize::new((vp_width(80), vp_height(24)))));

        let vp_pos_val = vp_pos(5, 3);
        let c_pos_val = vp.to_canvas(vp_pos_val);
        assert_eq!(c_pos_val, c_pos(15usize, 8usize));

        let recovered_vp_pos = vp.to_viewport(c_pos_val);
        assert_eq!(recovered_vp_pos, Some(vp_pos_val));

        // Position outside of viewport bounds
        let offscreen_pos = c_pos(5usize, 2usize);
        assert_eq!(vp.to_viewport(offscreen_pos), None);
    }
}

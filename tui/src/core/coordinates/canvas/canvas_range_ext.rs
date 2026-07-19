// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CCol, CRow, RangeConvertExt, VPCol, VPRow, c_col, c_row};
use std::ops::{Range, RangeInclusive};

/// Extension trait to convert ranges of coordinate wrapper types (decorators and
/// newtypes) into ranges of inner primitives.
///
/// See [`canvas`] for details on coordinate wrapper types and mental models.
/// See the [`decorator pattern`] docs for more information on the decorator pattern vs
/// newtype pattern.
///
/// [`canvas`]: mod@crate::core::coordinates::canvas
/// [`decorator pattern`]:
///     mod@crate::core::canvas#design-decision-decorator-vs-newtype-pattern-rationale
pub trait CanvasRangeExt {
    type OutputRange;

    /// Converts a range of wrapper indices (e.g. [`Range<VPRow>`]) into a
    /// range of inner raw indices ([`Range<RowIndex>`]).
    fn to_raw(&self) -> Self::OutputRange;
}

impl CanvasRangeExt for Range<VPRow> {
    type OutputRange = Range<VPRow>;

    fn to_raw(&self) -> Range<VPRow> { self.start..self.end }
}

impl CanvasRangeExt for Range<VPCol> {
    type OutputRange = Range<VPCol>;

    fn to_raw(&self) -> Range<VPCol> { self.start..self.end }
}

impl CanvasRangeExt for Range<CRow> {
    type OutputRange = Range<usize>;

    fn to_raw(&self) -> Range<usize> { self.start.as_usize()..self.end.as_usize() }
}

impl CanvasRangeExt for Range<CCol> {
    type OutputRange = Range<usize>;

    fn to_raw(&self) -> Range<usize> { self.start.as_usize()..self.end.as_usize() }
}

impl CanvasRangeExt for RangeInclusive<VPRow> {
    type OutputRange = RangeInclusive<VPRow>;

    fn to_raw(&self) -> RangeInclusive<VPRow> { *self.start()..=*self.end() }
}

impl CanvasRangeExt for RangeInclusive<VPCol> {
    type OutputRange = RangeInclusive<VPCol>;

    fn to_raw(&self) -> RangeInclusive<VPCol> { *self.start()..=*self.end() }
}

impl CanvasRangeExt for RangeInclusive<CRow> {
    type OutputRange = RangeInclusive<usize>;

    fn to_raw(&self) -> RangeInclusive<usize> {
        self.start().as_usize()..=self.end().as_usize()
    }
}

impl CanvasRangeExt for RangeInclusive<CCol> {
    type OutputRange = RangeInclusive<usize>;

    fn to_raw(&self) -> RangeInclusive<usize> {
        self.start().as_usize()..=self.end().as_usize()
    }
}

impl RangeConvertExt for RangeInclusive<CRow> {
    type IndexType = CRow;

    fn to_exclusive(self) -> Range<CRow> {
        let start = *self.start();
        let end = c_row(self.end().as_usize() + 1);
        start..end
    }
}

impl RangeConvertExt for RangeInclusive<CCol> {
    type IndexType = CCol;

    fn to_exclusive(self) -> Range<CCol> {
        let start = *self.start();
        let end = c_col(self.end().as_usize() + 1);
        start..end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vp_col, vp_row};

    #[test]
    fn test_viewport_row_range_to_raw() {
        let range = vp_row(2)..vp_row(8);
        assert_eq!(range.to_raw(), vp_row(2)..vp_row(8));

        let range_inc = vp_row(2)..=vp_row(8);
        assert_eq!(range_inc.to_raw(), vp_row(2)..=vp_row(8));
    }

    #[test]
    fn test_viewport_col_range_to_raw() {
        let range = vp_col(5)..vp_col(15);
        assert_eq!(range.to_raw(), vp_col(5)..vp_col(15));

        let range_inc = vp_col(5)..=vp_col(15);
        assert_eq!(range_inc.to_raw(), vp_col(5)..=vp_col(15));
    }

    #[test]
    fn test_canvas_row_range_to_raw() {
        let range = c_row(10)..c_row(50);
        assert_eq!(range.to_raw(), 10..50);

        let range_inc = c_row(10)..=c_row(50);
        assert_eq!(range_inc.to_raw(), 10..=50);
    }

    #[test]
    fn test_canvas_col_range_to_raw() {
        let range = c_col(0)..c_col(100);
        assert_eq!(range.to_raw(), 0..100);

        let range_inc = c_col(0)..=c_col(100);
        assert_eq!(range_inc.to_raw(), 0..=100);
    }

    #[test]
    fn test_wrapper_range_to_exclusive() {
        let vp_inc = vp_row(0)..=vp_row(23);
        assert_eq!(vp_inc.to_exclusive(), vp_row(0)..vp_row(24));

        let vp_col_inc = vp_col(0)..=vp_col(79);
        assert_eq!(vp_col_inc.to_exclusive(), vp_col(0)..vp_col(80));

        let c_row_inc = c_row(5)..=c_row(99);
        assert_eq!(c_row_inc.to_exclusive(), c_row(5)..c_row(100));

        let c_col_inc = c_col(0)..=c_col(49);
        assert_eq!(c_col_inc.to_exclusive(), c_col(0)..c_col(50));
    }
}

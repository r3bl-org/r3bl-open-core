// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! A caret represents the insertion point or cursor in a text buffer.
//!
//! There are two kinds of carets in this coordinate system:
//! 1. [`VPCaret`]: The 0-based ([`u16`]) cursor position INSIDE the visible screen
//!    window, without adjustments for scrolling (or panning).
//! 2. [`CCaret`]: The 0-based ([`usize`]) cursor position on the continuous document
//!    canvas, adjusted for scrolling (or panning).
//!
//! # Mental Model & Conversions
//!
//! When calculating cursor positions, the viewport's top-left origin position [`CPos`]
//! (retrieved via [`Viewport::get_origin_pos()`] on [`EditorContent`] /
//! [`EditorBufferMut`]) represents where the visible screen window sits on the document
//! canvas:
//! - [`VPCaret`] + [`CPos`] = [`CCaret`]:
//!   - Adding the viewport origin to the relative viewport cursor yields the
//!     canvas-absolute position.
//! - [`CCaret::to_viewport_caret`]:
//!   - Safely converts a canvas-absolute position back to visible viewport coordinates,
//!     if it is within the viewport, given a viewport origin and size. If the
//!     canvas-absolute position is outside the viewport, it returns [`None`].
//!
//! # Creation
//!
//! Construct carets using [`vp_caret()`] or [`c_caret()`], or using arithmetic
//! operators (`+`) directly between [`VPCaret`] and [`CPos`].
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::{
//!     ch,
//!     VPPos, VPSize, CPos, VPCaret, CCaret,
//!     vp_col, vp_row, vp_size, vp_caret, c_col, c_row, c_caret, c_pos
//! };
//!
//! let vp_origin_1: CPos = c_pos(3, 2);
//!
//! //
//! // Directly using VPCaret and CCaret.
//! //
//!
//! let vp_caret_1: VPCaret = vp_caret(vp_col(5) + vp_row(5));
//! let c_caret_1: CCaret = c_caret(c_col(7) + c_row(8));
//!
//! assert_eq!(vp_col(5) + vp_row(5), *vp_caret_1);
//! assert_eq!(c_col(7) + c_row(8), *c_caret_1);
//!
//! //
//! // Using Caret (and viewport origin CPos).
//! //
//!
//! // Convert CCaret (and CPos viewport origin) to VPCaret.
//! let vp_size: VPSize = vp_size(10, 10);
//! let caret_1 = c_caret_1.to_viewport_caret(vp_origin_1, vp_size).unwrap();
//! let expected_1 = vp_col(4) + vp_row(6);
//! assert_eq!(expected_1, *caret_1);
//!
//! // Convert VPCaret (and CPos viewport origin) to CCaret.
//! let caret_3 = vp_caret_1 + vp_origin_1;
//! let caret_4 = vp_origin_1 + vp_caret_1;
//! let expected_2 = c_col(8) + c_row(7);
//! assert_eq!(expected_2, *caret_3);
//! assert_eq!(expected_2, *caret_4);
//! ```
//!
//! [`EditorBufferMut`]: crate::EditorBufferMut
//! [`EditorContent`]: crate::EditorContent
//! [`Viewport::get_origin_pos()`]: crate::Viewport::get_origin_pos
use crate::{
    c_pos, CBoundingBox, CPos, NarrowingCastToU16, VPPos, VPSize, vp_col, vp_pos,
    vp_row,
};
use std::ops::{Add, Deref, DerefMut};

pub fn vp_caret(arg_vp_caret: impl Into<VPCaret>) -> VPCaret { arg_vp_caret.into() }

pub fn c_caret(arg_c_caret: impl Into<CCaret>) -> CCaret { arg_c_caret.into() }

/// The viewport-relative position is the `col_index` and `row_index` of the caret INSIDE
/// the viewport, without making any adjustments for scrolling (or panning).
/// - It does not take into account the amount of scrolling (or panning) that is currently
///   active.
/// - When scrolling (or panning) is active, this position will be different from the
///   canvas-absolute position.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct VPCaret(pub VPPos);

mod impl_viewport_caret {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl VPCaret {
        pub fn new(arg_vp_caret: impl Into<VPCaret>) -> Self { arg_vp_caret.into() }
    }

    impl Deref for VPCaret {
        type Target = VPPos;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for VPCaret {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<VPPos> for VPCaret {
        fn from(position: VPPos) -> VPCaret { VPCaret(position) }
    }

    impl From<VPCaret> for VPPos {
        fn from(c: VPCaret) -> VPPos { c.0 }
    }
}

/// The canvas-absolute position is the `col_index` and `row_index` of the caret OUTSIDE
/// the viewport, after making adjustments for scrolling (or panning).
/// - It takes into account the amount of scrolling (or panning) that is currently active.
/// - When scrolling (or panning) is active, this position will be different from the
///   viewport-relative position.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct CCaret(pub CPos);

mod impl_c_caret {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CCaret {
        pub fn new(arg_c_caret: impl Into<CCaret>) -> Self { arg_c_caret.into() }

        #[must_use]
        pub fn to_viewport_caret(
            &self,
            vp_origin: CPos,
            vp_size: VPSize,
        ) -> Option<VPCaret> {
            let vp_bounds_on_canvas = CBoundingBox::new(vp_origin, vp_size.into());

            if vp_bounds_on_canvas.contains_pos(self.0) {
                let rel_col = self.0.col_index - vp_origin.col_index;
                let rel_row = self.0.row_index - vp_origin.row_index;

                let vp_col = vp_col(rel_col.as_u16_narrowing());
                let vp_row = vp_row(rel_row.as_u16_narrowing());

                Some(VPCaret::from(vp_pos(vp_col, vp_row)))
            } else {
                None
            }
        }
    }

    impl Deref for CCaret {
        type Target = CPos;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for CCaret {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<(VPCaret, CPos)> for CCaret {
        fn from((vp_caret, vp_origin): (VPCaret, CPos)) -> Self {
            CCaret(c_pos(
                vp_origin.col_index + vp_caret.col_index.as_usize(),
                vp_origin.row_index + vp_caret.row_index.as_usize(),
            ))
        }
    }

    impl From<(CPos, VPCaret)> for CCaret {
        fn from((vp_origin, vp_caret): (CPos, VPCaret)) -> Self {
            (vp_caret, vp_origin).into()
        }
    }

    impl From<CPos> for CCaret {
        fn from(position: CPos) -> CCaret { CCaret(position) }
    }

    // VPCaret + CPos = CCaret
    impl Add<CPos> for VPCaret {
        type Output = CCaret;

        fn add(self, rhs: CPos) -> Self::Output { (self, rhs).into() }
    }

    // CPos + VPCaret = CCaret
    impl Add<VPCaret> for CPos {
        type Output = CCaret;

        fn add(self, rhs: VPCaret) -> Self::Output { (rhs, self).into() }
    }
}

#[cfg(test)]
mod tests_caret {
    use super::*;
    use crate::{c_pos, vp_col, vp_row, vp_size};

    #[test]
    fn test_viewport_caret_constructors() {
        let pos_1 = vp_row(10) + vp_col(20);

        // vp_caret constructor fn and new.
        let cv_1 = vp_caret(pos_1);
        let cv_2 = VPCaret::new(pos_1);
        assert_eq!(*cv_1, pos_1);
        assert_eq!(cv_1, cv_2);
    }

    #[test]
    fn test_c_caret_constructors() {
        let pos_1 = c_pos(20, 10);

        // c_caret constructor fn and new.
        let cc_1 = c_caret(pos_1);
        let cc_2 = CCaret::new(pos_1);
        assert_eq!(*cc_1, pos_1);
        assert_eq!(cc_1, cc_2);
    }

    #[test]
    fn test_caret_math_and_conversions() {
        let pos_1 = vp_row(5) + vp_col(5);
        let vp_origin = c_pos(2, 3);

        let cv = vp_caret(pos_1);
        let cc = c_caret(c_pos(7, 8));

        // VPCaret + CPos -> CCaret.
        let cc_from_add_1 = cv + vp_origin;
        let cc_from_add_2 = vp_origin + cv;
        assert_eq!(*cc_from_add_1, *cc);
        assert_eq!(*cc_from_add_2, *cc);

        // CCaret::to_viewport_caret.
        let cv_from_to_vp = cc.to_viewport_caret(vp_origin, vp_size(10, 10)).unwrap();
        assert_eq!(*cv_from_to_vp, *cv);
    }

    #[test]
    fn test_deref_mut() {
        let mut cv = vp_caret(vp_row(5) + vp_col(5));
        *cv = vp_row(10) + vp_col(20);
        assert_eq!(*cv, vp_row(10) + vp_col(20));

        let mut cc = c_caret(c_pos(7, 8));
        *cc = c_pos(15, 25);
        assert_eq!(*cc, c_pos(15, 25));
    }

    #[test]
    fn test_tuple_conversions() {
        let cv = vp_caret(vp_row(5) + vp_col(5));
        let vp_origin = c_pos(2, 3);

        let cc_1: CCaret = (cv, vp_origin).into();
        let cc_2: CCaret = (vp_origin, cv).into();

        assert_eq!(cc_1, cc_2);
        assert_eq!(*cc_1, c_pos(7, 8));
    }

    #[test]
    fn test_to_viewport_caret_bounds_and_edges() {
        let vp_origin = c_pos(10, 20);
        let size = vp_size(5, 5);

        // 1. Top-left origin boundary (inside)
        let cc_origin = c_caret(c_pos(10, 20));
        assert_eq!(
            cc_origin.to_viewport_caret(vp_origin, size),
            Some(vp_caret(vp_row(0) + vp_col(0)))
        );

        // 2. Bottom-right inside edge
        let cc_bottom_right = c_caret(c_pos(14, 24));
        assert_eq!(
            cc_bottom_right.to_viewport_caret(vp_origin, size),
            Some(vp_caret(vp_row(4) + vp_col(4)))
        );

        // 3. Above origin (row < 20) -> None
        let cc_above = c_caret(c_pos(10, 19));
        assert_eq!(cc_above.to_viewport_caret(vp_origin, size), None);

        // 4. Left of origin (col < 10) -> None
        let cc_left = c_caret(c_pos(9, 20));
        assert_eq!(cc_left.to_viewport_caret(vp_origin, size), None);

        // 5. Right of boundary (col >= 15) -> None
        let cc_right = c_caret(c_pos(15, 20));
        assert_eq!(cc_right.to_viewport_caret(vp_origin, size), None);

        // 6. Below boundary (row >= 25) -> None
        let cc_below = c_caret(c_pos(10, 25));
        assert_eq!(cc_below.to_viewport_caret(vp_origin, size), None);
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CCol, CHeight, CPos, c_height};
use crate::{VPHeight, generate_canvas_index_type_impl};
use std::ops::Add;

/// Absolute 0-based row index in the continuous storage buffer space (64-bit [`Canvas`]
/// domain).
///
/// Addresses an absolute line/row position in document space (from row index 0 up to
/// total row count minus 1), decoupling buffer storage capacity from 16-bit Viewport
/// limits.
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CRow(pub usize);
generate_canvas_index_type_impl!(CRow, CHeight, VPHeight, c_row, c_height);

impl Add<CCol> for CRow {
    type Output = CPos;

    fn add(self, rhs: CCol) -> Self::Output {
        CPos {
            col_index: rhs,
            row_index: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrayBoundsCheck, ArrayOverflowResult, NarrowingCastToU16, c_col, c_pos,
                vp_height};

    #[test]
    fn test_get_and_set_methods() {
        let mut r = c_row(5);
        assert_eq!(r.get(), c_row(5));
        r.set(c_row(10));
        assert_eq!(r.get(), c_row(10));
    }

    #[test]
    fn test_canvas_row_index_constructors_and_conversions() {
        let r1 = c_row(5usize);
        assert_eq!(r1.0, 5);
        assert_eq!(r1.as_usize(), 5);

        let r2: CRow = 5u16.into();
        assert_eq!(r1, r2);

        let r3: CRow = 5i32.into();
        assert_eq!(r1, r3);

        let r4: CRow = c_row(5usize);
        assert_eq!(r1, r4);

        let u_val: usize = r1.into();
        assert_eq!(u_val, 5);

        let mut r_mut = c_row(5usize);
        r_mut.0 = 10;
        assert_eq!(r_mut.0, 10);
    }

    #[test]
    fn test_canvas_row_index_math_ops() {
        let r = c_row(10usize);

        // Arithmetic with usize.
        assert_eq!(r + 5usize, c_row(15usize));
        assert_eq!(r - 3usize, c_row(7usize));
        assert_eq!(r - 15usize, c_row(0usize)); // Saturating underflow.

        let mut r_assign = c_row(10usize);
        r_assign += 5usize;
        assert_eq!(r_assign, c_row(15usize));
        r_assign -= 20usize;
        assert_eq!(r_assign, c_row(0usize)); // Saturating underflow.

        // Arithmetic with i32.
        assert_eq!(r + 5i32, c_row(15usize));
        assert_eq!(r + (-3i32), c_row(7usize));
        assert_eq!(r + (-15i32), c_row(0usize));

        assert_eq!(r - 3i32, c_row(7usize));
        assert_eq!(r - (-5i32), c_row(15usize));
        assert_eq!(r - 20i32, c_row(0usize));

        let mut r_assign_i32 = c_row(10usize);
        r_assign_i32 += 5i32;
        assert_eq!(r_assign_i32, c_row(15usize));
        r_assign_i32 -= -5i32;
        assert_eq!(r_assign_i32, c_row(20usize));

        // Self arithmetic.
        assert_eq!(r + c_row(5usize), c_row(15usize));
        assert_eq!(r - c_row(3usize), c_height(7usize));
        assert_eq!(r - c_row(15usize), c_height(0usize));

        // Add CCol -> CPos.
        let pos = r + c_col(20usize);
        assert_eq!(pos, c_pos(20usize, 10usize));

        // Arithmetic with associated CHeight.
        assert_eq!(r + c_height(5usize), c_row(15usize));
        assert_eq!(r - c_height(3usize), c_row(7usize));
        assert_eq!(r - c_height(20usize), c_row(0usize)); // Saturating underflow.

        let mut r_assign_h = c_row(10usize);
        r_assign_h += c_height(5usize);
        assert_eq!(r_assign_h, c_row(15usize));
        r_assign_h -= c_height(20usize);
        assert_eq!(r_assign_h, c_row(0usize));

        // Arithmetic with associated VPHeight.
        assert_eq!(r + vp_height(5), c_row(15usize));
        assert_eq!(r - vp_height(3), c_row(7usize));
        assert_eq!(r - vp_height(20), c_row(0usize));

        let mut r_assign_vph = c_row(10usize);
        r_assign_vph += vp_height(5);
        assert_eq!(r_assign_vph, c_row(15usize));
        r_assign_vph -= vp_height(20);
        assert_eq!(r_assign_vph, c_row(0usize));

        // Narrowing cast to u16.
        let r_cast = c_row(42usize);
        assert_eq!(r_cast.as_u16_narrowing(), 42u16);

        // ArrayBoundsCheck.
        assert_eq!(
            c_row(5usize).overflows(c_height(10usize)),
            ArrayOverflowResult::Within
        );
        assert_eq!(
            c_row(10usize).overflows(c_height(10usize)),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            c_row(15usize).overflows(c_height(10usize)),
            ArrayOverflowResult::Overflowed
        );
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CPos, CRow, CWidth, c_width};
use crate::{VPWidth, generate_canvas_index_type_impl};
use std::ops::Add;

/// Absolute 0-based column index in the continuous storage buffer space (64-bit
/// [`Canvas`] domain).
///
/// Addresses an absolute display column position on a line in document space (from column
/// index 0 up to total line display width minus 1), decoupling buffer line capacity from
/// 16-bit Viewport limits.
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CCol(pub usize);
generate_canvas_index_type_impl!(CCol, CWidth, VPWidth, c_col, c_width);

impl Add<CRow> for CCol {
    type Output = CPos;

    fn add(self, rhs: CRow) -> Self::Output {
        CPos {
            col_index: self,
            row_index: rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrayBoundsCheck, ArrayOverflowResult, NarrowingCastToU16, c_col, c_pos,
                c_row, vp_width};

    #[test]
    fn test_get_and_set_methods() {
        let mut c = c_col(8);
        assert_eq!(c.get(), c_col(8));
        c.set(c_col(20));
        assert_eq!(c.get(), c_col(20));
    }

    #[test]
    fn test_canvas_col_index_constructors_and_conversions() {
        let c1 = c_col(10usize);
        assert_eq!(c1.0, 10);
        assert_eq!(c1.as_usize(), 10);

        let c2: CCol = 10u16.into();
        assert_eq!(c1, c2);

        let c3: CCol = 10i32.into();
        assert_eq!(c1, c3);

        let c4: CCol = c_col(10usize);
        assert_eq!(c1, c4);

        let u_val: usize = c1.into();
        assert_eq!(u_val, 10);

        let mut c_mut = c_col(10usize);
        c_mut.0 = 20;
        assert_eq!(c_mut.0, 20);
    }

    #[test]
    fn test_canvas_col_index_math_ops() {
        let c = c_col(10usize);

        // Arithmetic with usize.
        assert_eq!(c + 5usize, c_col(15usize));
        assert_eq!(c - 3usize, c_col(7usize));
        assert_eq!(c - 15usize, c_col(0usize)); // Saturating underflow.

        let mut c_assign = c_col(10usize);
        c_assign += 5usize;
        assert_eq!(c_assign, c_col(15usize));
        c_assign -= 20usize;
        assert_eq!(c_assign, c_col(0usize)); // Saturating underflow.

        // Arithmetic with i32.
        assert_eq!(c + 5i32, c_col(15usize));
        assert_eq!(c + (-3i32), c_col(7usize));
        assert_eq!(c + (-15i32), c_col(0usize));

        assert_eq!(c - 3i32, c_col(7usize));
        assert_eq!(c - (-5i32), c_col(15usize));
        assert_eq!(c - 20i32, c_col(0usize));

        let mut c_assign_i32 = c_col(10usize);
        c_assign_i32 += 5i32;
        assert_eq!(c_assign_i32, c_col(15usize));
        c_assign_i32 -= -5i32;
        assert_eq!(c_assign_i32, c_col(20usize));

        // Self arithmetic.
        assert_eq!(c + c_col(5usize), c_col(15usize));
        assert_eq!(c - c_col(3usize), c_width(7usize));
        assert_eq!(c - c_col(15usize), c_width(0usize));

        // Add CRow -> CPos.
        let pos = c + c_row(5usize);
        assert_eq!(pos, c_pos(10usize, 5usize));

        // Arithmetic with associated CWidth.
        assert_eq!(c + c_width(5usize), c_col(15usize));
        assert_eq!(c - c_width(3usize), c_col(7usize));
        assert_eq!(c - c_width(20usize), c_col(0usize)); // Saturating underflow.

        let mut c_assign_w = c_col(10usize);
        c_assign_w += c_width(5usize);
        assert_eq!(c_assign_w, c_col(15usize));
        c_assign_w -= c_width(20usize);
        assert_eq!(c_assign_w, c_col(0usize));

        // Arithmetic with associated VPWidth.
        assert_eq!(c + vp_width(5), c_col(15usize));
        assert_eq!(c - vp_width(3), c_col(7usize));
        assert_eq!(c - vp_width(20), c_col(0usize));

        let mut c_assign_vpw = c_col(10usize);
        c_assign_vpw += vp_width(5);
        assert_eq!(c_assign_vpw, c_col(15usize));
        c_assign_vpw -= vp_width(20);
        assert_eq!(c_assign_vpw, c_col(0usize));

        // Narrowing cast to u16.
        let c_cast = c_col(42usize);
        assert_eq!(c_cast.as_u16_narrowing(), 42u16);

        // ArrayBoundsCheck.
        assert_eq!(
            c_col(5usize).overflows(c_width(10usize)),
            ArrayOverflowResult::Within
        );
        assert_eq!(
            c_col(10usize).overflows(c_width(10usize)),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            c_col(15usize).overflows(c_width(10usize)),
            ArrayOverflowResult::Overflowed
        );
    }
}

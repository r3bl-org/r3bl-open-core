// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CCol, CHeight, CSize};
use crate::{VPWidth, generate_canvas_length_type_impl};
use std::ops::Add;

/// 1-based horizontal width or column count in the continuous storage buffer space
/// (64-bit [`Canvas`] domain).
///
/// Represents the total display width or column extent on a line in document space.
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CWidth(pub usize);
generate_canvas_length_type_impl!(CWidth, CCol, VPWidth, c_width, c_col);

impl Add<CHeight> for CWidth {
    type Output = CSize;

    fn add(self, rhs: CHeight) -> Self::Output {
        CSize {
            col_width: self,
            row_height: rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, NarrowingCastToU16, c_col, c_height, c_size, vp_width};

    #[test]
    fn test_get_and_set_methods() {
        let mut w = c_width(5);
        assert_eq!(w.get(), c_width(5));
        w.set(c_width(10));
        assert_eq!(w.get(), c_width(10));
    }

    #[test]
    fn test_canvas_width_constructors_and_conversions() {
        let w1 = c_width(80usize);
        assert_eq!(w1.0, 80);
        assert_eq!(w1.as_usize(), 80);

        let w2: CWidth = 80u16.into();
        assert_eq!(w1, w2);

        let w3: CWidth = 80i32.into();
        assert_eq!(w1, w3);

        let w4: CWidth = vp_width(80).into();
        assert_eq!(w1, w4);

        let u_val: usize = w1.into();
        assert_eq!(u_val, 80);

        let mut w_mut = c_width(5usize);
        w_mut.0 = 10;
        assert_eq!(w_mut.0, 10);

        let w_cast = c_width(42usize);
        assert_eq!(w_cast.as_u16_narrowing(), 42u16);
    }

    #[test]
    fn test_canvas_width_ops() {
        let w = c_width(80usize);
        assert_eq!(w.as_usize(), 80);
        assert_eq!(w.convert_to_index(), c_col(79usize));
        assert_eq!(c_width(0usize).convert_to_index(), c_col(0usize));
        assert!(!w.is_empty());
        assert!(c_width(0usize).is_empty());

        // Self arithmetic.
        assert_eq!(w + c_width(10usize), c_width(90usize));
        assert_eq!(w - c_width(20usize), c_width(60usize));
        assert_eq!(w - c_width(100usize), c_width(0usize));

        let mut w_assign = c_width(40usize);
        w_assign += c_width(10usize);
        assert_eq!(w_assign, c_width(50usize));
        w_assign -= c_width(20usize);
        assert_eq!(w_assign, c_width(30usize));

        // Arithmetic with usize.
        assert_eq!(w + 10usize, c_width(90usize));
        assert_eq!(w - 20usize, c_width(60usize));
        assert_eq!(w - 100usize, c_width(0usize));

        let mut w_assign_u = c_width(40usize);
        w_assign_u += 10usize;
        assert_eq!(w_assign_u, c_width(50usize));
        w_assign_u -= 60usize;
        assert_eq!(w_assign_u, c_width(0usize));

        // Arithmetic with i32.
        assert_eq!(w + 10i32, c_width(90usize));
        assert_eq!(w + (-20i32), c_width(60usize));
        assert_eq!(w + (-100i32), c_width(0usize));
        assert_eq!(w - 20i32, c_width(60usize));
        assert_eq!(w - (-10i32), c_width(90usize));
        assert_eq!(w - 100i32, c_width(0usize));

        let mut w_assign_i32 = c_width(40usize);
        w_assign_i32 += 10i32;
        assert_eq!(w_assign_i32, c_width(50usize));
        w_assign_i32 -= -10i32;
        assert_eq!(w_assign_i32, c_width(60usize));

        // Multiplication and Division.
        assert_eq!(w * 2usize, c_width(160usize));
        assert_eq!(w / 2usize, c_width(40usize));
        assert_eq!(w / 0usize, c_width(0usize)); // Div by zero edge case.

        let h = c_height(24usize);
        assert_eq!(w + h, c_size(80usize, 24usize));
    }
}

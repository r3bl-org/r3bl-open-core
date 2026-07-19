// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CRow, CSize, CWidth};
use crate::{VPHeight, generate_canvas_length_type_impl};
use std::ops::Add;

/// 1-based vertical height or row count in the continuous storage buffer space (64-bit
/// [`Canvas`] domain).
///
/// Represents the total number of rows or vertical extent in document space.
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CHeight(pub usize);
generate_canvas_length_type_impl!(CHeight, CRow, VPHeight, c_height, c_row);

impl Add<CWidth> for CHeight {
    type Output = CSize;

    fn add(self, rhs: CWidth) -> Self::Output {
        CSize {
            col_width: rhs,
            row_height: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, NarrowingCastToU16, c_row, c_size, c_width, vp_height};

    #[test]
    fn test_get_and_set_methods() {
        let mut h = c_height(5);
        assert_eq!(h.get(), c_height(5));
        h.set(c_height(10));
        assert_eq!(h.get(), c_height(10));
    }

    #[test]
    fn test_canvas_height_constructors_and_conversions() {
        let h1 = c_height(24usize);
        assert_eq!(h1.0, 24);
        assert_eq!(h1.as_usize(), 24);

        let h2: CHeight = 24u16.into();
        assert_eq!(h1, h2);

        let h3: CHeight = 24i32.into();
        assert_eq!(h1, h3);

        let h4: CHeight = vp_height(24).into();
        assert_eq!(h1, h4);

        let u_val: usize = h1.into();
        assert_eq!(u_val, 24);

        let mut h_mut = c_height(5usize);
        h_mut.0 = 10;
        assert_eq!(h_mut.0, 10);

        let h_cast = c_height(42usize);
        assert_eq!(h_cast.as_u16_narrowing(), 42u16);
    }

    #[test]
    fn test_canvas_height_ops() {
        let h = c_height(24usize);
        assert_eq!(h.as_usize(), 24);
        assert_eq!(h.convert_to_index(), c_row(23usize));
        assert_eq!(c_height(0usize).convert_to_index(), c_row(0usize));
        assert!(!h.is_empty());
        assert!(c_height(0usize).is_empty());

        // Self arithmetic.
        assert_eq!(h + c_height(10usize), c_height(34usize));
        assert_eq!(h - c_height(4usize), c_height(20usize));
        assert_eq!(h - c_height(30usize), c_height(0usize));

        let mut h_assign = c_height(20usize);
        h_assign += c_height(5usize);
        assert_eq!(h_assign, c_height(25usize));
        h_assign -= c_height(10usize);
        assert_eq!(h_assign, c_height(15usize));

        // Arithmetic with usize.
        assert_eq!(h + 6usize, c_height(30usize));
        assert_eq!(h - 4usize, c_height(20usize));
        assert_eq!(h - 30usize, c_height(0usize));

        let mut h_assign_u = c_height(20usize);
        h_assign_u += 5usize;
        assert_eq!(h_assign_u, c_height(25usize));
        h_assign_u -= 30usize;
        assert_eq!(h_assign_u, c_height(0usize));

        // Arithmetic with i32.
        assert_eq!(h + 5i32, c_height(29usize));
        assert_eq!(h + (-4i32), c_height(20usize));
        assert_eq!(h + (-30i32), c_height(0usize));
        assert_eq!(h - 4i32, c_height(20usize));
        assert_eq!(h - (-5i32), c_height(29usize));
        assert_eq!(h - 30i32, c_height(0usize));

        let mut h_assign_i32 = c_height(20usize);
        h_assign_i32 += 5i32;
        assert_eq!(h_assign_i32, c_height(25usize));
        h_assign_i32 -= -5i32;
        assert_eq!(h_assign_i32, c_height(30usize));

        // Multiplication and Division.
        assert_eq!(h * 2usize, c_height(48usize));
        assert_eq!(h / 2usize, c_height(12usize));
        assert_eq!(h / 0usize, c_height(0usize)); // Div by zero edge case.

        let w = c_width(80usize);
        assert_eq!(h + w, c_size(80usize, 24usize));
    }
}

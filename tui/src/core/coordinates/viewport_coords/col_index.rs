// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{VPIndex, VPLength, VPWidth, vp_width};
use crate::{ChUnit, generate_index_type_impl};
use std::{hash::Hash,
          ops::{Add, AddAssign, Mul, Sub, SubAssign}};

/// The horizontal index in a grid of characters, starting at 0, which is the first
/// column.
///
/// This is one part of a [`VPPos`] (position), and is different from [`VPWidth`], which
/// is one part of a [`VPSize`]. You can use the [`vp_col()`] to create a new instance.
///
/// # Examples
/// ```
/// use r3bl_tui::{VPCol, vp_col};
/// let col = vp_col(5);
/// let col = VPCol::new(5);
/// ```
///
/// [`vp_col()`]: crate::vp_col
/// [`VPPos`]: crate::core::VPPos
/// [`VPSize`]: crate::VPSize
/// [`VPWidth`]: crate::VPWidth
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPCol(ChUnit);
generate_index_type_impl!(
    VPCol,    // Add impl for this type
    VPWidth,  // Use this associated type
    vp_col,   // Make this constructor fn
    vp_width  // Use this constructor fn
);

impl From<VPIndex> for VPCol {
    fn from(index: VPIndex) -> VPCol { VPCol(*index) }
}

impl Sub<VPLength> for VPCol {
    type Output = VPCol;
    fn sub(self, rhs: VPLength) -> Self::Output {
        let mut self_copy = self;
        self_copy.0 -= *rhs;
        self_copy
    }
}

impl SubAssign<VPLength> for VPCol {
    fn sub_assign(&mut self, rhs: VPLength) { self.0 -= *rhs; }
}

impl Add<VPLength> for VPCol {
    type Output = VPCol;
    fn add(self, rhs: VPLength) -> Self::Output {
        let mut self_copy = self;
        self_copy.0 += *rhs;
        self_copy
    }
}

impl AddAssign<VPLength> for VPCol {
    fn add_assign(&mut self, rhs: VPLength) { self.0 += *rhs; }
}

impl Mul<VPLength> for VPCol {
    type Output = VPCol;
    fn mul(self, rhs: VPLength) -> Self::Output {
        let mut self_copy = self;
        self_copy.0 *= *rhs;
        self_copy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ch;

    #[test]
    fn test_deref_and_deref_mut() {
        let mut col = VPCol::new(5);
        assert_eq!(*col, ch(5));
        *col = ch(10);
        assert_eq!(*col, ch(10));
    }

    #[test]
    fn test_col_index_add() {
        // Add.
        {
            let col1 = VPCol::from(ch(5));
            let col2 = VPCol::new(3);
            let result = col1 + col2;
            assert_eq!(result, VPCol::new(8));
        }
        // AddAssign.
        {
            let mut col1 = VPCol::from(ch(5));
            let col2 = VPCol::new(3);
            col1 += col2;
            assert_eq!(col1, VPCol::new(8));
        }
    }

    #[test]
    fn test_col_index_sub() {
        // Sub.
        {
            let col1 = vp_col(5);
            let col2 = vp_col(3);
            let result = col1 - col2;
            assert_eq!(result, vp_col(2));
        }
        // SubAssign.
        {
            let mut col1 = vp_col(5);
            let col2 = vp_col(3);
            col1 -= col2;
            assert_eq!(col1, vp_col(2));
        }
    }

    #[test]
    fn test_width_sub() {
        // Sub.
        {
            let col_idx = VPCol::new(5);
            let wid = vp_width(3);
            let res = col_idx - wid;
            assert_eq!(res, vp_col(2));
            assert_eq!(*res, ch(2));
        }
        // SubAssign.
        {
            let col = VPCol::new(5);
            let width = VPWidth::new(3u16);
            let mut col = col;
            col -= width;
            assert_eq!(col, VPCol::new(2));
        }
    }

    #[test]
    fn test_width_add() {
        // Add.
        {
            let col = VPCol::new(5);
            let width = VPWidth::new(3u16);
            let result = col + width;
            assert_eq!(result, VPCol::new(8));
        }
        // AddAssign.
        {
            let col = VPCol::new(5);
            let width = VPWidth::new(3u16);
            let mut col = col;
            col += width;
            assert_eq!(col, VPCol::new(8));
        }
    }

    #[test]
    fn test_width_mul() {
        let col = VPCol::new(5);
        let width = VPWidth::new(3u16);
        let result = col * width;
        assert_eq!(result, VPCol::new(15));
    }

    #[test]
    fn test_as_usize() {
        let col = VPCol::new(5);
        assert_eq!(col.as_usize(), 5);
    }

    #[test]
    fn test_as_u16() {
        let col = VPCol::new(5);
        assert_eq!(col.as_u16(), 5);
    }

    #[test]
    fn test_convert_to_length() {
        let col = VPCol::new(5);
        assert_eq!(col.convert_to_length(), vp_width(6));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(VPCol::from(5), vp_col(5));
    }

    #[test]
    fn test_col_index_add_i32() {
        // Add positive i32.
        {
            let col_idx = vp_col(5);
            let result = col_idx + 3i32;
            assert_eq!(result, vp_col(8));
        }
        // Add negative i32 (should be treated as 0).
        {
            let col_idx = vp_col(5);
            let result = col_idx + -3i32;
            assert_eq!(result, vp_col(5)); // -3 becomes 0
        }
        // Add zero.
        {
            let col_idx = vp_col(5);
            let result = col_idx + 0i32;
            assert_eq!(result, vp_col(5));
        }
    }

    #[test]
    fn test_col_index_sub_i32() {
        // Subtract positive i32.
        {
            let col_idx = vp_col(10);
            let result = col_idx - 3i32;
            assert_eq!(result, vp_col(7));
        }
        // Subtract larger value (should saturate to 0).
        {
            let col_idx = vp_col(5);
            let result = col_idx - 10i32;
            assert_eq!(result, vp_col(0));
        }
        // Subtract negative i32 (should be treated as 0, no change).
        {
            let col_idx = vp_col(5);
            let result = col_idx - -3i32;
            assert_eq!(result, vp_col(5)); // -3 becomes 0
        }
        // Subtract zero.
        {
            let col_idx = vp_col(5);
            let result = col_idx - 0i32;
            assert_eq!(result, vp_col(5));
        }
    }

    #[test]
    fn test_col_index_add_assign_i32() {
        // AddAssign positive i32.
        {
            let mut col_idx = vp_col(5);
            col_idx += 3i32;
            assert_eq!(col_idx, vp_col(8));
        }
        // AddAssign negative i32 (should be treated as 0).
        {
            let mut col_idx = vp_col(5);
            col_idx += -3i32;
            assert_eq!(col_idx, vp_col(5)); // -3 becomes 0
        }
    }

    #[test]
    fn test_col_index_sub_assign_i32() {
        // SubAssign positive i32.
        {
            let mut col_idx = vp_col(10);
            col_idx -= 3i32;
            assert_eq!(col_idx, vp_col(7));
        }
        // SubAssign larger value (should saturate to 0).
        {
            let mut col_idx = vp_col(5);
            col_idx -= 10i32;
            assert_eq!(col_idx, vp_col(0));
        }
        // SubAssign negative i32 (should be treated as 0, no change).
        {
            let mut col_idx = vp_col(5);
            col_idx -= -3i32;
            assert_eq!(col_idx, vp_col(5)); // -3 becomes 0
        }
    }
}

#[cfg(test)]
mod tests_length_arithmetic {
    use super::*;
    use crate::vp_len;

    #[test]
    fn test_add_length() {
        let col_idx = vp_col(5);
        let length = vp_len(3);
        let result = col_idx + length;
        assert_eq!(result, vp_col(8));
    }

    #[test]
    fn test_sub_length() {
        let col_idx = vp_col(10);
        let length = vp_len(3);
        let result = col_idx - length;
        assert_eq!(result, vp_col(7));
    }

    #[test]
    fn test_mul_length() {
        let col_idx = vp_col(4);
        let length = vp_len(3);
        let result = col_idx * length;
        assert_eq!(result, vp_col(12));
    }

    #[test]
    fn test_add_assign_length() {
        let mut col_idx = vp_col(5);
        let length = vp_len(3);
        col_idx += length;
        assert_eq!(col_idx, vp_col(8));
    }

    #[test]
    fn test_sub_assign_length() {
        let mut col_idx = vp_col(10);
        let length = vp_len(3);
        col_idx -= length;
        assert_eq!(col_idx, vp_col(7));
    }

    #[test]
    fn test_sub_length_narrowing() {
        // Test subtraction that would go below zero (should saturate to 0).
        let col_idx = vp_col(5);
        let length = vp_len(10);
        let result = col_idx - length;
        assert_eq!(result, vp_col(0));
    }

    #[test]
    fn test_sub_assign_length_narrowing() {
        let mut col_idx = vp_col(3);
        let length = vp_len(10);
        col_idx -= length;
        assert_eq!(col_idx, vp_col(0));
    }

    #[test]
    fn test_length_zero_operations() {
        let col_idx = vp_col(5);
        let zero_length = vp_len(0);

        // Adding zero should not change value.
        assert_eq!(col_idx + zero_length, vp_col(5));

        // Subtracting zero should not change value.
        assert_eq!(col_idx - zero_length, vp_col(5));

        // Multiplying by zero should result in zero.
        assert_eq!(col_idx * zero_length, vp_col(0));
    }

    #[test]
    fn test_length_operations_consistency() {
        // Verify operations work consistently with direct ChUnit operations.
        let col_idx = vp_col(7);
        let length = vp_len(4);

        // Test that Length operations give same result as ChUnit operations.
        assert_eq!(col_idx + length, vp_col(*col_idx + *length));
        assert_eq!(col_idx - length, vp_col(*col_idx - *length));
        assert_eq!(col_idx * length, vp_col(*col_idx * *length));
    }
}

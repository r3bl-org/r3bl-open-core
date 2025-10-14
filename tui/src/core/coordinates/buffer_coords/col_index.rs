// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{ColWidth, width};
use crate::{ChUnit, Index, Length, generate_index_type_impl};
use std::{hash::Hash,
          ops::{Add, AddAssign, Mul, Sub, SubAssign}};

/// The horizontal index in a grid of characters, starting at 0, which is the first
/// column.
///
/// This is one part of a [`Pos`] (position), and is different from [`ColWidth`], which
/// is one part of a [`Size`]. You can use the [`col()`] to create a new instance.
///
/// # Examples
/// ```
/// use r3bl_tui::{ColIndex, col};
/// let col = col(5);
/// let col = ColIndex::new(5);
/// ```
///
/// [`Pos`]: crate::Pos
/// [`ColWidth`]: crate::ColWidth
/// [`Size`]: crate::Size
/// [`col()`]: crate::col
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct ColIndex(pub ChUnit);
generate_index_type_impl!(
    /* Add impl for this type */ ColIndex,
    /* Use this associated type */ ColWidth,
    /* Make this constructor fn */ col, /* Use this constructor fn */ width
);

impl From<Index> for ColIndex {
    fn from(index: Index) -> Self { ColIndex(index.0) }
}

impl Sub<Length> for ColIndex {
    type Output = ColIndex;
    fn sub(self, rhs: Length) -> Self::Output {
        let mut self_copy = self;
        self_copy.0 -= rhs.0;
        self_copy
    }
}

impl SubAssign<Length> for ColIndex {
    fn sub_assign(&mut self, rhs: Length) { self.0 -= rhs.0; }
}

impl Add<Length> for ColIndex {
    type Output = ColIndex;
    fn add(self, rhs: Length) -> Self::Output {
        let mut self_copy = self;
        self_copy.0 += rhs.0;
        self_copy
    }
}

impl AddAssign<Length> for ColIndex {
    fn add_assign(&mut self, rhs: Length) { self.0 += rhs.0; }
}

impl Mul<Length> for ColIndex {
    type Output = ColIndex;
    fn mul(self, rhs: Length) -> Self::Output {
        let mut self_copy = self;
        self_copy.0 *= rhs.0;
        self_copy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ch;

    #[test]
    fn test_deref_and_deref_mut() {
        let mut col = ColIndex::new(5);
        assert_eq!(*col, ch(5));
        *col = ch(10);
        assert_eq!(*col, ch(10));
    }

    #[test]
    fn test_col_index_add() {
        // Add.
        {
            let col1 = ColIndex::from(ch(5));
            let col2 = ColIndex::new(3);
            let result = col1 + col2;
            assert_eq!(result, ColIndex::new(8));
        }
        // AddAssign.
        {
            let mut col1 = ColIndex::from(ch(5));
            let col2 = ColIndex::new(3);
            col1 += col2;
            assert_eq!(col1, ColIndex::new(8));
        }
    }

    #[test]
    fn test_col_index_sub() {
        // Sub.
        {
            let col1 = col(5);
            let col2 = col(3);
            let result = col1 - col2;
            assert_eq!(result, col(2));
        }
        // SubAssign.
        {
            let mut col1 = col(5);
            let col2 = col(3);
            col1 -= col2;
            assert_eq!(col1, col(2));
        }
    }

    #[test]
    fn test_width_sub() {
        // Sub.
        {
            let col_idx = ColIndex::new(5);
            let wid = width(3);
            let res = col_idx - wid;
            assert_eq!(res, col(2));
            assert_eq!(*res, ch(2));
        }
        // SubAssign.
        {
            let col = ColIndex::new(5);
            let width = ColWidth::new(3);
            let mut col = col;
            col -= width;
            assert_eq!(col, ColIndex::new(2));
        }
    }

    #[test]
    fn test_width_add() {
        // Add.
        {
            let col = ColIndex::new(5);
            let width = ColWidth::new(3);
            let result = col + width;
            assert_eq!(result, ColIndex::new(8));
        }
        // AddAssign.
        {
            let col = ColIndex::new(5);
            let width = ColWidth::new(3);
            let mut col = col;
            col += width;
            assert_eq!(col, ColIndex::new(8));
        }
    }

    #[test]
    fn test_width_mul() {
        let col = ColIndex::new(5);
        let width = ColWidth::new(3);
        let result = col * width;
        assert_eq!(result, ColIndex::new(15));
    }

    #[test]
    fn test_as_usize() {
        let col = ColIndex::new(5);
        assert_eq!(col.as_usize(), 5);
    }

    #[test]
    fn test_as_u16() {
        let col = ColIndex::new(5);
        assert_eq!(col.as_u16(), 5);
    }

    #[test]
    fn test_convert_to_length() {
        let col = ColIndex::new(5);
        assert_eq!(col.convert_to_length(), width(6));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(ColIndex::from(5usize), col(5));
    }

    #[test]
    fn test_col_index_add_i32() {
        // Add positive i32.
        {
            let col_idx = col(5);
            let result = col_idx + 3i32;
            assert_eq!(result, col(8));
        }
        // Add negative i32 (should be treated as 0).
        {
            let col_idx = col(5);
            let result = col_idx + -3i32;
            assert_eq!(result, col(5)); // -3 becomes 0
        }
        // Add zero.
        {
            let col_idx = col(5);
            let result = col_idx + 0i32;
            assert_eq!(result, col(5));
        }
    }

    #[test]
    fn test_col_index_sub_i32() {
        // Subtract positive i32.
        {
            let col_idx = col(10);
            let result = col_idx - 3i32;
            assert_eq!(result, col(7));
        }
        // Subtract larger value (should saturate to 0).
        {
            let col_idx = col(5);
            let result = col_idx - 10i32;
            assert_eq!(result, col(0));
        }
        // Subtract negative i32 (should be treated as 0, no change).
        {
            let col_idx = col(5);
            let result = col_idx - -3i32;
            assert_eq!(result, col(5)); // -3 becomes 0
        }
        // Subtract zero.
        {
            let col_idx = col(5);
            let result = col_idx - 0i32;
            assert_eq!(result, col(5));
        }
    }

    #[test]
    fn test_col_index_add_assign_i32() {
        // AddAssign positive i32.
        {
            let mut col_idx = col(5);
            col_idx += 3i32;
            assert_eq!(col_idx, col(8));
        }
        // AddAssign negative i32 (should be treated as 0).
        {
            let mut col_idx = col(5);
            col_idx += -3i32;
            assert_eq!(col_idx, col(5)); // -3 becomes 0
        }
    }

    #[test]
    fn test_col_index_sub_assign_i32() {
        // SubAssign positive i32.
        {
            let mut col_idx = col(10);
            col_idx -= 3i32;
            assert_eq!(col_idx, col(7));
        }
        // SubAssign larger value (should saturate to 0).
        {
            let mut col_idx = col(5);
            col_idx -= 10i32;
            assert_eq!(col_idx, col(0));
        }
        // SubAssign negative i32 (should be treated as 0, no change).
        {
            let mut col_idx = col(5);
            col_idx -= -3i32;
            assert_eq!(col_idx, col(5)); // -3 becomes 0
        }
    }
}

#[cfg(test)]
mod tests_length_arithmetic {
    use super::*;
    use crate::len;

    #[test]
    fn test_add_length() {
        let col_idx = col(5);
        let length = len(3);
        let result = col_idx + length;
        assert_eq!(result, col(8));
    }

    #[test]
    fn test_sub_length() {
        let col_idx = col(10);
        let length = len(3);
        let result = col_idx - length;
        assert_eq!(result, col(7));
    }

    #[test]
    fn test_mul_length() {
        let col_idx = col(4);
        let length = len(3);
        let result = col_idx * length;
        assert_eq!(result, col(12));
    }

    #[test]
    fn test_add_assign_length() {
        let mut col_idx = col(5);
        let length = len(3);
        col_idx += length;
        assert_eq!(col_idx, col(8));
    }

    #[test]
    fn test_sub_assign_length() {
        let mut col_idx = col(10);
        let length = len(3);
        col_idx -= length;
        assert_eq!(col_idx, col(7));
    }

    #[test]
    fn test_sub_length_saturating() {
        // Test subtraction that would go below zero (should saturate to 0).
        let col_idx = col(5);
        let length = len(10);
        let result = col_idx - length;
        assert_eq!(result, col(0));
    }

    #[test]
    fn test_sub_assign_length_saturating() {
        let mut col_idx = col(3);
        let length = len(10);
        col_idx -= length;
        assert_eq!(col_idx, col(0));
    }

    #[test]
    fn test_length_zero_operations() {
        let col_idx = col(5);
        let zero_length = len(0);

        // Adding zero should not change value.
        assert_eq!(col_idx + zero_length, col(5));

        // Subtracting zero should not change value.
        assert_eq!(col_idx - zero_length, col(5));

        // Multiplying by zero should result in zero.
        assert_eq!(col_idx * zero_length, col(0));
    }

    #[test]
    fn test_length_operations_consistency() {
        // Verify operations work consistently with direct ChUnit operations.
        let col_idx = col(7);
        let length = len(4);

        // Test that Length operations give same result as ChUnit operations.
        assert_eq!(col_idx + length, col(*col_idx + *length));
        assert_eq!(col_idx - length, col(*col_idx - *length));
        assert_eq!(col_idx * length, col(*col_idx * *length));
    }
}

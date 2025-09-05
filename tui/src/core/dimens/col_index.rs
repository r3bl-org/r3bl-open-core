// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{fmt::Debug,
          ops::{Add, AddAssign, Deref, DerefMut, Mul, Sub, SubAssign}};

use crate::{ChUnit, ColWidth, IndexMarker, UnitCompare,
            create_numeric_arithmetic_operators, usize, width};

/// The horizontal index in a grid of characters, starting at 0, which is the first
/// column.
/// - This is one part of a [`crate::Pos`] (position), and is different from
///   [`crate::ColWidth`], which is one part of a [`crate::Size`].
/// - You can use the [`crate::col()`] to create a new instance.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{ColIndex, col};
/// let col = col(5);
/// let col = ColIndex::new(5);
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct ColIndex(pub ChUnit);

impl Debug for ColIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ColIndex({:?})", self.0)
    }
}

/// Creates a new [`ColIndex`] from any type that can be converted into it.
///
/// This is a convenience function that provides a shorter way to create
/// column indices.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{ColIndex, col};
/// let col = col(5_usize);
/// assert_eq!(col, ColIndex::new(5));
/// ```
pub fn col(arg_col_index: impl Into<ColIndex>) -> ColIndex { arg_col_index.into() }

mod impl_core {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    impl ColIndex {
        pub fn new(arg_col_index: impl Into<ColIndex>) -> Self { arg_col_index.into() }

        #[must_use]
        pub fn as_usize(&self) -> usize { usize(self.0) }

        /// This is for use with [crossterm] crate.
        #[must_use]
        pub fn as_u16(&self) -> u16 { self.0.into() }

        /// Add 1 to the index to convert it to a width. The intention of this function is
        /// to meaningfully convert a [`ColIndex`] to a [`ColWidth`]. This is useful in
        /// situations where you need to find what the width is at this row index.
        #[must_use]
        pub fn convert_to_width(&self) -> ColWidth { width(self.0 + 1) }
    }
}

mod impl_from_numeric {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    impl From<ChUnit> for ColIndex {
        fn from(ch_unit: ChUnit) -> Self { ColIndex(ch_unit) }
    }

    impl From<usize> for ColIndex {
        fn from(val: usize) -> Self { ColIndex(val.into()) }
    }

    impl From<ColIndex> for usize {
        fn from(col: ColIndex) -> Self { col.as_usize() }
    }

    impl From<u16> for ColIndex {
        fn from(val: u16) -> Self { ColIndex(val.into()) }
    }

    impl From<i32> for ColIndex {
        fn from(val: i32) -> Self { ColIndex(val.into()) }
    }

    impl From<ColIndex> for u16 {
        fn from(col: ColIndex) -> Self { col.as_u16() }
    }
}

mod impl_deref {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for ColIndex {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for ColIndex {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod dimension_arithmetic_operators {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    impl Sub<ColIndex> for ColIndex {
        type Output = ColIndex;

        fn sub(self, rhs: ColIndex) -> Self::Output { col(*self - *rhs) }
    }

    impl SubAssign<ColIndex> for ColIndex {
        /// This simply subtracts the value of the RHS [`ColIndex`] instance from the LHS
        /// [`ColIndex`].
        fn sub_assign(&mut self, rhs: ColIndex) {
            let diff = **self - *rhs;
            *self = col(diff);
        }
    }

    impl Add<ColIndex> for ColIndex {
        type Output = ColIndex;

        fn add(self, rhs: ColIndex) -> Self::Output {
            let mut self_copy = self;
            *self_copy += *rhs;
            self_copy
        }
    }

    impl AddAssign<ColIndex> for ColIndex {
        fn add_assign(&mut self, rhs: ColIndex) { *self = *self + rhs; }
    }

    impl Sub<ColWidth> for ColIndex {
        type Output = ColIndex;

        fn sub(self, rhs: ColWidth) -> Self::Output {
            let mut self_copy = self;
            *self_copy -= *rhs;
            self_copy
        }
    }

    impl SubAssign<ColWidth> for ColIndex {
        fn sub_assign(&mut self, rhs: ColWidth) { **self -= *rhs; }
    }

    impl Add<ColWidth> for ColIndex {
        type Output = ColIndex;

        fn add(self, rhs: ColWidth) -> Self::Output {
            let mut self_copy = self;
            *self_copy += *rhs;
            self_copy
        }
    }

    impl AddAssign<ColWidth> for ColIndex {
        fn add_assign(&mut self, rhs: ColWidth) { *self = *self + rhs; }
    }

    impl Mul<ColWidth> for ColIndex {
        type Output = ColIndex;

        fn mul(self, rhs: ColWidth) -> Self::Output {
            let mut self_copy = self;
            *self_copy *= *rhs;
            self_copy
        }
    }
}

mod numeric_arithmetic_operators {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    // Generate numeric operations using macro
    create_numeric_arithmetic_operators!(ColIndex, col, [usize, u16, i32]);
}

mod bounds_check_trait_impls {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl UnitCompare for ColIndex {
        fn as_usize(&self) -> usize { self.0.into() }

        fn as_u16(&self) -> u16 { self.0.into() }
    }

    impl IndexMarker for ColIndex {}
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
    fn test_convert_to_width() {
        let col = ColIndex::new(5);
        assert_eq!(col.convert_to_width(), width(6));
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

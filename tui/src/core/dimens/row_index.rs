/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{fmt::Debug,
          ops::{Add, AddAssign, Deref, DerefMut, Mul, Sub, SubAssign}};

use crate::{ChUnit, RowHeight, height, usize};

/// The vertical index in a grid of characters, starting at 0, which is the first row.
/// This is one part of a [`crate::Pos`] position and is different from
/// [`crate::RowHeight`], which is one part of a [`crate::Size`].
///
/// You can use the [`crate::row()`] to create a new instance.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{RowIndex, row};
/// let row = row(5);
/// let row = RowIndex::new(5);
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct RowIndex(pub ChUnit);

impl Debug for RowIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RowIndex({:?})", self.0)
    }
}

pub fn row(arg_row_index: impl Into<RowIndex>) -> RowIndex { arg_row_index.into() }

mod constructor {
    use super::{ChUnit, RowHeight, RowIndex, height, usize};

    impl RowIndex {
        pub fn new(arg_row_index: impl Into<RowIndex>) -> Self { arg_row_index.into() }

        #[must_use]
        pub fn as_usize(&self) -> usize { usize(self.0) }

        /// This is for use with [crossterm] crate.
        #[must_use]
        pub fn as_u16(&self) -> u16 { self.0.into() }

        /// Add 1 to the index to convert it to a height. The intention of this function
        /// is to meaningfully convert a [`RowIndex`] to a [`RowHeight`]. This is useful
        /// in situations where you need to find what the height is at this row
        /// index.
        #[must_use]
        pub fn convert_to_height(&self) -> RowHeight { height(self.0 + 1) }
    }

    impl From<ChUnit> for RowIndex {
        fn from(ch_unit: ChUnit) -> Self { RowIndex(ch_unit) }
    }

    impl From<usize> for RowIndex {
        fn from(val: usize) -> Self { RowIndex(val.into()) }
    }

    impl From<RowIndex> for usize {
        fn from(row: RowIndex) -> Self { row.as_usize() }
    }

    impl From<u16> for RowIndex {
        fn from(val: u16) -> Self { RowIndex(val.into()) }
    }

    impl From<i32> for RowIndex {
        fn from(val: i32) -> Self { RowIndex(val.into()) }
    }
}

mod ops {
    use super::{Add, AddAssign, ChUnit, Deref, DerefMut, Mul, RowHeight, RowIndex, Sub,
                SubAssign};

    impl Deref for RowIndex {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for RowIndex {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl Sub<RowIndex> for RowIndex {
        type Output = RowIndex;

        fn sub(self, rhs: RowIndex) -> Self::Output {
            let mut self_copy = self;
            *self_copy -= *rhs;
            self_copy
        }
    }

    impl SubAssign<RowIndex> for RowIndex {
        fn sub_assign(&mut self, rhs: RowIndex) { **self -= *rhs; }
    }

    impl Add<RowIndex> for RowIndex {
        type Output = RowIndex;

        fn add(self, rhs: RowIndex) -> Self::Output {
            let mut self_copy = self;
            *self_copy += *rhs;
            self_copy
        }
    }

    impl AddAssign<RowIndex> for RowIndex {
        fn add_assign(&mut self, rhs: RowIndex) { *self = *self + rhs; }
    }

    impl Sub<RowHeight> for RowIndex {
        type Output = RowIndex;

        fn sub(self, rhs: RowHeight) -> Self::Output {
            let mut self_copy = self;
            *self_copy -= *rhs;
            self_copy
        }
    }

    impl SubAssign<RowHeight> for RowIndex {
        fn sub_assign(&mut self, rhs: RowHeight) { **self -= *rhs; }
    }

    impl Add<RowHeight> for RowIndex {
        type Output = RowIndex;

        fn add(self, rhs: RowHeight) -> Self::Output {
            let mut self_copy = self;
            *self_copy += *rhs;
            self_copy
        }
    }

    impl AddAssign<RowHeight> for RowIndex {
        fn add_assign(&mut self, rhs: RowHeight) { *self = *self + rhs; }
    }

    impl Mul<RowHeight> for RowIndex {
        type Output = RowIndex;

        fn mul(self, rhs: RowHeight) -> Self::Output {
            let mut self_copy = self;
            *self_copy *= *rhs;
            self_copy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ch;

    #[test]
    fn test_row_index_add() {
        let row1 = RowIndex::from(ch(5));
        let row2 = RowIndex::new(3);
        let result = row1 + row2;
        assert_eq!(result, RowIndex(ch(8)));
        assert_eq!(*result, ch(8));
    }

    #[test]
    fn test_row_index_sub() {
        let row1 = RowIndex::from(ch(5));
        let row2 = RowIndex::new(3);
        let result = row1 - row2;
        assert_eq!(result, RowIndex::new(2));
        assert_eq!(*result, ch(2));
    }

    #[test]
    fn test_row_index_sub_assign_add_assign() {
        let mut row0 = row(5);
        let row2 = row(3);

        row0 -= row2;
        assert_eq!(row0, row(2));
        assert_eq!(*row0, ch(2));

        row0 += row2;
        assert_eq!(row0, row(5));
        assert_eq!(*row0, ch(5));
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut row = RowIndex::new(5);
        assert_eq!(*row, ch(5));
        *row = ch(10);
        assert_eq!(*row, ch(10));
    }

    #[test]
    fn test_height_mul() {
        let row = RowIndex::new(5);
        let height = RowHeight::new(3);
        let result = row * height;
        assert_eq!(result, RowIndex::new(15));
        assert_eq!(*result, ch(15));
    }

    #[test]
    fn test_height_add() {
        // Add.
        {
            let row = RowIndex::new(5);
            let height = RowHeight::new(3);
            let result = row + height;
            assert_eq!(result, RowIndex::new(8));
            assert_eq!(*result, ch(8));
        }
        // AddAssign.
        {
            let mut row = RowIndex::new(5);
            let height = RowHeight::new(3);
            row += height;
            assert_eq!(row, RowIndex::new(8));
            assert_eq!(*row, ch(8));
        }
    }

    #[test]
    fn test_height_sub() {
        // Sub.
        {
            let row_idx = RowIndex::new(5);
            let ht = RowHeight::new(3);
            let res = row_idx - ht;
            assert_eq!(res, row(2));
            assert_eq!(*res, ch(2));
        }
        // SubAssign.
        {
            let mut row = RowIndex::new(5);
            let height = RowHeight::new(3);
            row -= height;
            assert_eq!(row, RowIndex::new(2));
            assert_eq!(*row, ch(2));
        }
    }

    #[test]
    fn test_as_usize() {
        let row = RowIndex::new(5);
        assert_eq!(row.as_usize(), 5);
    }

    #[test]
    fn test_convert_to_height() {
        let row = RowIndex::new(5);
        let ht = row.convert_to_height();
        assert_eq!(ht, height(6));
        assert_eq!(*ht, ch(6));
    }

    #[test]
    fn test_as_u16() {
        let row = RowIndex::new(5);
        assert_eq!(row.as_u16(), 5);
    }

    #[test]
    fn test_from_usize() {
        assert_eq!(RowIndex::from(5usize), row(5));
    }
}

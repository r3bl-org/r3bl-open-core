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
          ops::{Add, Deref, DerefMut, Div, Sub, SubAssign}};

use crate::{ChUnit, RowIndex, ch, row};

/// Height is row count, ie the number of rows that a UI component occupies. This is one
/// part of a [crate::Size], and is not the same as the [crate::RowIndex]
/// (position). You can simply use the [crate::height()] to create a new instance.
///
/// # Working with row index
///
/// You can't safely add or subtract a [crate::RowIndex] from this `Height`; since without
/// knowing your specific use case ahead of time, it isn't posable to provide a default
/// implementation without leading to unintended consequences. You can do the reverse
/// safely.
///
/// In order to add or subtract a [crate::RowIndex] from this `Height` you can call
/// [Self::convert_to_row_index()], and apply whatever logic makes sense for your use
/// case.
///
/// There is a special case for scrolling vertically, and clipping rendering output to max
/// display rows which is handled by
/// `r3bl_tui::caret_scroll_index::scroll_row_index_for_height()`.
///
/// # Examples
///
/// ```rust
/// use r3bl_core::{RowHeight, height};
/// let height = height(5);
/// let height = RowHeight::new(5);
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct RowHeight(pub ChUnit);

impl Debug for RowHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RowHeight({:?})", self.0)
    }
}

pub fn height(arg_row_height: impl Into<RowHeight>) -> RowHeight { arg_row_height.into() }

mod constructor {
    use super::*;

    impl RowHeight {
        pub fn new(arg_row_height: impl Into<RowHeight>) -> Self { arg_row_height.into() }

        /// Subtract 1 from row index to get the height. I.e.: `row index = height - 1`.
        /// row index = height - 1
        ///
        /// The following are equivalent:
        /// - row index >= height
        /// - row index > height - 1 (which is this function)
        ///
        /// The following holds true:
        /// - last row index == height - 1 (which is this function)
        pub fn convert_to_row_index(&self) -> RowIndex { row(self.0 - ch(1)) }
    }

    impl From<ChUnit> for RowHeight {
        fn from(ch_unit: ChUnit) -> Self { RowHeight(ch_unit) }
    }

    impl From<usize> for RowHeight {
        fn from(height: usize) -> Self { RowHeight(ch(height)) }
    }

    impl From<u16> for RowHeight {
        fn from(val: u16) -> Self { RowHeight(val.into()) }
    }

    impl From<i32> for RowHeight {
        fn from(val: i32) -> Self { RowHeight(val.into()) }
    }

    impl From<u8> for RowHeight {
        fn from(val: u8) -> Self { RowHeight(val.into()) }
    }
}

mod ops {
    use super::*;

    impl Deref for RowHeight {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for RowHeight {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl Add<RowHeight> for RowHeight {
        type Output = RowHeight;

        fn add(self, rhs: RowHeight) -> Self::Output {
            let mut self_copy = self;
            *self_copy += *rhs;
            self_copy
        }
    }

    impl Sub<RowHeight> for RowHeight {
        type Output = RowHeight;

        fn sub(self, rhs: RowHeight) -> Self::Output {
            let mut self_copy = self;
            *self_copy -= *rhs;
            self_copy
        }
    }

    impl SubAssign<RowHeight> for RowHeight {
        fn sub_assign(&mut self, rhs: RowHeight) { **self -= *rhs; }
    }

    impl Div<ChUnit> for RowHeight {
        type Output = RowHeight;

        fn div(self, rhs: ChUnit) -> Self::Output {
            let value = *self / rhs;
            height(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ch, row};

    #[test]
    fn test_height_new() {
        let height = RowHeight::new(10);
        assert_eq!(height, RowHeight(10.into()));
        assert_eq!(*height, ch(10));
    }

    #[test]
    fn test_height_add() {
        let height1 = RowHeight(10.into());
        let height2 = RowHeight(4.into());
        let result = height1 + height2;
        assert_eq!(result, RowHeight(14.into()));
        assert_eq!(*result, ch(14));
    }

    #[test]
    fn test_height_sub() {
        // Sub. This returns a Height as expected, and not a RowIndex.
        {
            let height1 = height(10);
            let height2 = height(4);
            let result = height1 - height2;
            assert_eq!(result, height(6));
            assert_eq!(*result, ch(6));
        }

        // SubAssign.
        {
            let mut height1 = height(10);
            let height2 = height(4);
            height1 -= height2;
            assert_eq!(height1, height(6));
            assert_eq!(*height1, ch(6));
        }
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut height = RowHeight(10.into());
        assert_eq!(*height, ch(10));
        *height = ch(20);
        assert_eq!(*height, ch(20));
    }

    #[test]
    fn test_div_ch_unit() {
        assert_eq!(height(10) / ch(2), height(5));
    }

    #[test]
    fn test_convert_to_row_index() {
        assert_eq!(height(10).convert_to_row_index(), row(9));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(RowHeight::from(10usize), height(10));
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ChUnit, LengthOps, NumericConversions, NumericValue, RowIndex, ch,
            create_numeric_arithmetic_operators};
use std::{fmt::{Debug, Formatter},
          ops::{Add, Deref, DerefMut, Div, Sub, SubAssign}};

/// `RowHeight` is row count, i.e., the number of rows that a UI component occupies.
///
/// This is one part of a [`Size`] and is different from the [`RowIndex`] (position).
///
/// You can use the [`height()`] to create a new instance.
///
/// # Working with row index
/// You can't safely add or subtract a [`RowIndex`] from this [`RowHeight`]; since without
/// knowing your specific use case ahead of time, it isn't possible to provide a default
/// implementation without leading to unintended consequences. You can do the reverse
/// safely.
///
/// To add or subtract a [`RowIndex`] from this [`RowHeight`], you can call
/// [`Self::convert_to_index()`] and apply whatever logic makes sense for your use
/// case.
///
/// There is a special case for scrolling vertically and clips rendering output to max
/// display rows which is handled by the [`CursorBoundsCheck`] trait method
/// [`eol_cursor_position()`].
///
/// # Examples
/// ```
/// use r3bl_tui::{RowHeight, height};
/// let height = height(5);
/// let height = RowHeight::new(5);
/// ```
///
/// [`RowHeight`]: crate::RowHeight
/// [`RowIndex`]: crate::RowIndex
/// [`Size`]: crate::Size
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`eol_cursor_position()`]: crate::CursorBoundsCheck::eol_cursor_position
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct RowHeight(pub ChUnit);

impl Debug for RowHeight {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RowHeight({:?})", self.0)
    }
}

pub fn height(arg_row_height: impl Into<RowHeight>) -> RowHeight { arg_row_height.into() }

mod impl_core {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl RowHeight {
        pub fn new(arg_row_height: impl Into<RowHeight>) -> Self { arg_row_height.into() }

        #[must_use]
        pub fn as_u16(&self) -> u16 { self.0.into() }

        #[must_use]
        pub fn as_usize(&self) -> usize { self.0.into() }
    }
}

mod impl_from_numeric {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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

    impl From<RowHeight> for u16 {
        fn from(row_height: RowHeight) -> Self { row_height.0.into() }
    }
}

mod impl_deref {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for RowHeight {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for RowHeight {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod dimension_arithmetic_operators {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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

mod numeric_arithmetic_operators {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    // Generate numeric operations using macro.
    create_numeric_arithmetic_operators!(RowHeight, height, [usize, u16, i32]);
}

mod bounds_check_trait_impls {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NumericConversions for RowHeight {
        fn as_usize(&self) -> usize { self.0.as_usize() }

        fn as_u16(&self) -> u16 { self.0.as_u16() }
    }

    impl NumericValue for RowHeight {}

    impl LengthOps for RowHeight {
        type IndexType = RowIndex;

        fn convert_to_index(&self) -> Self::IndexType {
            if self.0.value == 0 {
                RowIndex::new(0)
            } else {
                RowIndex::new(self.0.value - 1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row;

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
        // Sub. This returns a RowHeight as expected, and not a RowIndex.
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
    fn test_convert_to_index() {
        assert_eq!(height(10).convert_to_index(), row(9));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(RowHeight::from(10usize), height(10));
    }
}

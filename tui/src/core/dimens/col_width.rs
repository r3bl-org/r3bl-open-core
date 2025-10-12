// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ChUnit, ColIndex, LengthOps, NumericConversions, NumericValue, ch,
            generate_numeric_arithmetic_ops_impl};
use std::{fmt::Debug,
          ops::{Add, AddAssign, Deref, DerefMut, Div, Sub, SubAssign}};

/// `ColWidth` is column count, i.e., the number of columns that a UI component occupies.
///
/// This is one part of a [`Size`] and is different from the [`ColIndex`] (position).
///
/// You can use the [`width()`] to create a new instance.
///
/// # Working with col index
/// You can't safely add or subtract a [`ColIndex`] from this `ColWidth`; since without
/// knowing your specific use case ahead of time, it isn't possible to provide a default
/// implementation without leading to unintended consequences. You can do the reverse
/// safely.
///
/// To add or subtract a [`ColIndex`] from this [`ColWidth`], you can call
/// [`Self::convert_to_index()`] and apply whatever logic makes sense for your use
/// case.
///
/// There is a special case for scrolling horizontally, and creates a selection range,
/// which is handled by the [`CursorBoundsCheck`] trait method [`eol_cursor_position()`].
///
/// # Examples
/// ```
/// use r3bl_tui::{ColWidth, width};
/// let width = width(5);
/// let width = ColWidth::new(5);
/// ```
///
/// [`ColWidth`]: crate::ColWidth
/// [`RowHeight`]: crate::RowHeight
/// [`RowIndex`]: crate::RowIndex
/// [`Size`]: crate::Size
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`eol_cursor_position()`]: crate::CursorBoundsCheck::eol_cursor_position
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct ColWidth(pub ChUnit);

impl Debug for ColWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ColWidth({:?})", self.0)
    }
}

pub fn width(arg_col_width: impl Into<ColWidth>) -> ColWidth { arg_col_width.into() }

mod impl_core {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl ColWidth {
        pub fn new(arg_col_width: impl Into<ColWidth>) -> Self { arg_col_width.into() }

        #[must_use]
        pub fn as_u16(&self) -> u16 { self.0.into() }

        #[must_use]
        pub fn as_usize(&self) -> usize { self.0.into() }
    }
}

mod impl_from_numeric {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<ChUnit> for ColWidth {
        fn from(ch_unit: ChUnit) -> Self { ColWidth(ch_unit) }
    }

    impl From<usize> for ColWidth {
        fn from(width: usize) -> Self { ColWidth(ch(width)) }
    }

    impl From<u16> for ColWidth {
        fn from(val: u16) -> Self { ColWidth(val.into()) }
    }

    impl From<i32> for ColWidth {
        fn from(val: i32) -> Self { ColWidth(val.into()) }
    }

    impl From<u8> for ColWidth {
        fn from(val: u8) -> Self { ColWidth(val.into()) }
    }

    impl From<ColWidth> for u16 {
        fn from(col_width: ColWidth) -> Self { col_width.0.into() }
    }
}

mod impl_deref {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for ColWidth {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for ColWidth {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod dimension_arithmetic_operators {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Add<ColWidth> for ColWidth {
        type Output = ColWidth;

        fn add(self, rhs: ColWidth) -> Self::Output {
            let mut self_copy = self;
            *self_copy += *rhs;
            self_copy
        }
    }

    impl AddAssign<ColWidth> for ColWidth {
        fn add_assign(&mut self, rhs: ColWidth) { **self += *rhs; }
    }

    impl Sub<ColWidth> for ColWidth {
        type Output = ColWidth;

        fn sub(self, rhs: ColWidth) -> Self::Output {
            let mut self_copy = self;
            *self_copy -= *rhs;
            self_copy
        }
    }

    impl SubAssign<ColWidth> for ColWidth {
        fn sub_assign(&mut self, rhs: ColWidth) { **self -= *rhs; }
    }

    impl Div<ChUnit> for ColWidth {
        type Output = ColWidth;

        fn div(self, rhs: ChUnit) -> Self::Output {
            let value = *self / rhs;
            width(value)
        }
    }
}

mod numeric_arithmetic_operators {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    // Generate numeric operations using macro.
    generate_numeric_arithmetic_ops_impl!(ColWidth, width, [usize, u16, i32]);
}

mod bounds_check_trait_impls {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NumericConversions for ColWidth {
        fn as_usize(&self) -> usize { self.0.as_usize() }

        fn as_u16(&self) -> u16 { self.0.as_u16() }
    }

    impl NumericValue for ColWidth {}

    impl LengthOps for ColWidth {
        type IndexType = ColIndex;

        fn convert_to_index(&self) -> Self::IndexType {
            if self.0.value == 0 {
                ColIndex::new(0)
            } else {
                ColIndex::new(self.0.value - 1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::col;

    #[test]
    fn test_width_new() {
        let it = ColWidth::new(5);
        assert_eq!(it, width(5));
        assert_eq!(*it, ch(5));
    }

    #[test]
    fn test_width_add() {
        // Add.
        {
            let width1 = ColWidth(5.into());
            let width2 = ColWidth(3.into());
            let result = width1 + width2;
            assert_eq!(result, ColWidth(8.into()));
            assert_eq!(*result, ch(8));
        }
        // AddAssign.
        {
            let mut width1 = ColWidth(5.into());
            let width2 = ColWidth(3.into());
            width1 += width2;
            assert_eq!(width1, ColWidth(8.into()));
            assert_eq!(*width1, ch(8));
        }
    }

    #[test]
    fn test_width_sub() {
        // Sub. This returns a ColWidth as expected, and not a ColIndex.
        {
            let width1 = width(5);
            let width2 = width(3);
            let result = width1 - width2;
            assert_eq!(result, width(2));
            assert_eq!(*result, ch(2));
        }
        // SubAssign.
        {
            let mut width1 = width(5);
            let width2 = width(3);
            width1 -= width2;
            assert_eq!(width1, width(2));
            assert_eq!(*width1, ch(2));
        }
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut width = ColWidth(5.into());
        assert_eq!(*width, ch(5));
        *width = ch(10);
        assert_eq!(*width, ch(10));
    }

    #[test]
    fn test_div_ch_unit() {
        assert_eq!(width(10) / ch(2), width(5));
    }

    #[test]
    fn test_convert_to_index() {
        assert_eq!(width(5).convert_to_index(), col(4));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(ColWidth::from(5usize), width(5));
    }
}

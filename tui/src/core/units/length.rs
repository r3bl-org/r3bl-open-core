// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! One-based character size measurements for terminal UI - see [`Length`] type.

use std::{fmt::Debug,
          hash::Hash,
          ops::{Add, AddAssign, Deref, DerefMut, Div, Sub, SubAssign}};

use super::{ChUnit, Index, ch, idx};
use crate::{ColWidth, LengthOps, NumericValue, RowHeight,
            create_numeric_arithmetic_operators};

/// Represents a length measurement in character units.
///
/// A `Length` is a 1-based measurement (as opposed to 0-based indices) that represents
/// the size or extent of something in the terminal UI, such as the width or height
/// of a component. It wraps a [`ChUnit`] value.
///
/// `Length` values can be created using the [`Length::new`] method, the [len] function,
/// or by converting from various numeric types.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{Length, len, ch};
///
/// // Create a Length using the new method
/// let length1 = Length::new(10);
///
/// // Create a Length using the len function
/// let length2 = len(10);
///
/// // Convert from a ChUnit
/// let length3 = Length::from(ch(10));
///
/// assert_eq!(length1, length2);
/// assert_eq!(length2, length3);
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct Length(pub ChUnit);

/// Creates a new [Length] from a value that can be converted into a Length.
///
/// This is a convenience function that is equivalent to calling [`Length::new`].
///
/// # Examples
///
/// ```
/// use r3bl_tui::{Length, len};
///
/// let length = len(10);
/// assert_eq!(length, Length::new(10));
/// ```
pub fn len(arg_length: impl Into<Length>) -> Length { arg_length.into() }

impl Debug for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Length({:?})", self.0)
    }
}

mod impl_core {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Length {
        pub fn new(arg_length: impl Into<Length>) -> Self { arg_length.into() }

        #[must_use]
        pub fn as_usize(&self) -> usize { self.0.into() }

        /// This is for use with [crossterm] crate.
        #[must_use]
        pub fn as_u16(&self) -> u16 { self.0.into() }

        /// Subtract 1 from length to get the last index. I.e.: `length = last index + 1`.
        ///
        /// The following are equivalent:
        /// - index >= length
        /// - index > length - 1 (which is this function)
        ///
        /// The following holds true:
        /// - last index == length - 1 (which is this function)
        #[must_use]
        pub fn convert_to_index(&self) -> Index {
            let it = self.0 - ch(1);
            idx(it)
        }
    }
}

mod impl_from_numeric {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    impl From<ChUnit> for Length {
        fn from(ch_unit: ChUnit) -> Self { Length(ch_unit) }
    }

    impl From<usize> for Length {
        fn from(width: usize) -> Self { Length(ch(width)) }
    }

    impl From<u16> for Length {
        fn from(val: u16) -> Self { Length(val.into()) }
    }

    impl From<i32> for Length {
        fn from(val: i32) -> Self { Length(val.into()) }
    }

    impl From<u8> for Length {
        fn from(val: u8) -> Self { Length(val.into()) }
    }

    impl From<Length> for ColWidth {
        fn from(val: Length) -> Self { val.0.into() }
    }

    impl From<ColWidth> for Length {
        fn from(val: ColWidth) -> Self { val.0.into() }
    }

    impl From<Length> for RowHeight {
        fn from(val: Length) -> Self { val.0.into() }
    }

    impl From<RowHeight> for Length {
        fn from(val: RowHeight) -> Self { val.0.into() }
    }
}

mod impl_deref {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for Length {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Length {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod dimension_arithmetic_operators {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Add<Length> for Length {
        type Output = Length;

        fn add(self, rhs: Length) -> Self::Output { Length(self.0 + rhs.0) }
    }

    impl AddAssign<Length> for Length {
        fn add_assign(&mut self, rhs: Length) { *self = *self + rhs; }
    }

    impl Sub<Length> for Length {
        type Output = Length;

        fn sub(self, rhs: Length) -> Self::Output { Length(self.0 - rhs.0) }
    }

    impl SubAssign<Length> for Length {
        fn sub_assign(&mut self, rhs: Length) { *self = *self - rhs; }
    }

    impl Div<Length> for Length {
        type Output = Length;

        fn div(self, rhs: Length) -> Self::Output { Length(self.0 / rhs.0) }
    }
}

mod numeric_arithmetic_operators {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    // Generate numeric operations using macro.
    create_numeric_arithmetic_operators!(Length, len, [usize, u16, i32]);
}

mod bounds_check_trait_impls {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NumericValue for Length {
        fn as_usize(&self) -> usize { self.0.as_usize() }

        fn as_u16(&self) -> u16 { self.0.as_u16() }
    }

    impl LengthOps for Length {
        type IndexType = Index;

        fn convert_to_index(&self) -> Self::IndexType {
            if self.0.value == 0 {
                Index::new(0)
            } else {
                Index::new(self.0.value - 1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_creation() {
        let length1 = Length::new(10);
        let length2 = Length::from(20);
        assert_eq!(length1.0, ch(10));
        assert_eq!(length2.0, ch(20));
    }

    #[test]
    fn test_length_conversion() {
        let length = Length::new(10);
        let index = length.convert_to_index();
        assert_eq!(index.0, ch(9));
    }

    #[test]
    fn test_length_operators() {
        let length1 = Length::new(10);
        let length2 = Length::new(20);

        // Add
        let length3 = length1 + length2;
        assert_eq!(length3.0, ch(30));

        // AddAssign
        let mut length4 = Length::new(10);
        length4 += length2;
        assert_eq!(length4.0, ch(30));

        // Sub
        let length5 = length2 - length1;
        assert_eq!(length5.0, ch(10));

        // SubAssign
        let mut length6 = Length::new(20);
        length6 -= length1;
        assert_eq!(length6.0, ch(10));

        // Div
        let length7 = length2 / length1;
        assert_eq!(length7.0, ch(2));
    }

    #[test]
    fn test_length_deref() {
        let length = Length::new(10);
        let value = *length;
        assert_eq!(value, ch(10));
    }

    #[test]
    fn test_length_deref_mut() {
        let mut length = Length::new(10);
        *length = ch(20);
        assert_eq!(length.0, ch(20));
    }

    #[test]
    fn test_length_from_various_types() {
        let length1 = Length::from(10_usize);
        let length2 = Length::from(20_u16);
        let length3 = Length::from(30_i32);
        let length4 = Length::from(40_u8);

        assert_eq!(length1.0, ch(10));
        assert_eq!(length2.0, ch(20));
        assert_eq!(length3.0, ch(30));
        assert_eq!(length4.0, ch(40));
    }

    #[test]
    fn test_length_partial_eq() {
        let length1 = Length::new(10);
        let length2 = Length::new(10);
        let length3 = Length::new(20);

        assert_eq!(length1, length2);
        assert_ne!(length1, length3);
    }

    #[test]
    fn test_length_partial_ord() {
        let length1 = Length::new(10);
        let length2 = Length::new(20);

        assert!(length1 < length2);
        assert!(length2 > length1);
        assert!(length1 <= length2);
        assert!(length2 >= length1);
    }

    #[test]
    fn test_len_fn() {
        let length1 = len(10);
        assert_eq!(length1.0, ch(10));

        let length2 = len(Length::new(20));
        assert_eq!(length2.0, ch(20));
    }

    #[test]
    fn test_debug_fmt() {
        let length = Length::new(10);
        assert_eq!(format!("{length:?}"), "Length(10)");
    }

    #[test]
    fn test_length_max_value() {
        // Test with maximum u16 value.
        let max_length = Length::new(u16::MAX);
        assert_eq!(max_length.as_u16(), u16::MAX);
    }

    #[test]
    fn test_length_zero() {
        // Test with zero
        let zero_length = Length::new(0);
        assert_eq!(zero_length.0, ch(0));

        // Converting zero length to index.
        let index = zero_length.convert_to_index();
        assert_eq!(index.0, ch(0)); // Should be 0 since we don't go below 0
    }

    #[test]
    fn test_length_interop_with_index() {
        // Test interoperability with Index.
        let length = Length::new(10);
        let index = idx(5);

        // Index + Length
        let new_index = index + length;
        assert_eq!(new_index, idx(15));

        // Index - Length
        let new_index = idx(20) - length;
        assert_eq!(new_index, idx(10));
    }

    #[test]
    fn test_length_arithmetic_edge_cases() {
        // Test addition near maximum value.
        let max_length = Length::new(u16::MAX - 5);
        let small_length = Length::new(5);
        let result = max_length + small_length;
        assert_eq!(result, Length::new(u16::MAX));

        // Test subtraction with zero.
        let length = Length::new(5);
        let result = length - Length::new(5);
        assert_eq!(result, Length::new(0));

        // Test subtraction below zero (should clamp to zero due to unsigned type)
        let length = Length::new(5);
        let result = length - Length::new(10);
        assert_eq!(result, Length::new(0));
    }
}

#[cfg(test)]
mod tests_col_width_conversion {
    use super::*;
    use crate::width;

    #[test]
    fn test_length_to_col_width() {
        let length = Length::new(10);
        let col_width: ColWidth = length.into();
        assert_eq!(col_width, width(10));
    }

    #[test]
    fn test_col_width_to_length() {
        let col_width = width(15);
        let length: Length = col_width.into();
        assert_eq!(length, len(15));
    }

    #[test]
    fn test_round_trip_conversion() {
        let original_length = len(42);
        let col_width: ColWidth = original_length.into();
        let back_to_length: Length = col_width.into();
        assert_eq!(original_length, back_to_length);

        let original_width = width(37);
        let length: Length = original_width.into();
        let back_to_width: ColWidth = length.into();
        assert_eq!(original_width, back_to_width);
    }

    #[test]
    fn test_zero_conversion() {
        let zero_length = len(0);
        let zero_width: ColWidth = zero_length.into();
        assert_eq!(zero_width, width(0));

        let zero_width = width(0);
        let zero_length: Length = zero_width.into();
        assert_eq!(zero_length, len(0));
    }

    #[test]
    fn test_max_value_conversion() {
        let max_length = len(u16::MAX);
        let max_width: ColWidth = max_length.into();
        assert_eq!(max_width.as_usize(), u16::MAX as usize);

        let max_width = width(u16::MAX);
        let max_length: Length = max_width.into();
        assert_eq!(max_length.as_usize(), u16::MAX as usize);
    }

    #[test]
    fn test_conversion_preserves_underlying_chunit() {
        let length = len(25);
        let width: ColWidth = length.into();

        // Both should have the same underlying ChUnit value.
        assert_eq!(length.0, width.0);
    }
}

#[cfg(test)]
mod tests_row_height_conversion {
    use super::*;
    use crate::height;

    #[test]
    fn test_length_to_row_height() {
        let length = Length::new(8);
        let row_height: RowHeight = length.into();
        assert_eq!(row_height, height(8));
    }

    #[test]
    fn test_row_height_to_length() {
        let row_height = height(12);
        let length: Length = row_height.into();
        assert_eq!(length, len(12));
    }

    #[test]
    fn test_round_trip_conversion() {
        let original_length = len(33);
        let row_height: RowHeight = original_length.into();
        let back_to_length: Length = row_height.into();
        assert_eq!(original_length, back_to_length);

        let original_height = height(29);
        let length: Length = original_height.into();
        let back_to_height: RowHeight = length.into();
        assert_eq!(original_height, back_to_height);
    }

    #[test]
    fn test_zero_conversion() {
        let zero_length = len(0);
        let zero_height: RowHeight = zero_length.into();
        assert_eq!(zero_height, height(0));

        let zero_height = height(0);
        let zero_length: Length = zero_height.into();
        assert_eq!(zero_length, len(0));
    }

    #[test]
    fn test_max_value_conversion() {
        let max_length = len(u16::MAX);
        let max_height: RowHeight = max_length.into();
        assert_eq!(max_height.as_usize(), u16::MAX as usize);

        let max_height = height(u16::MAX);
        let max_length: Length = max_height.into();
        assert_eq!(max_length.as_usize(), u16::MAX as usize);
    }

    #[test]
    fn test_conversion_preserves_underlying_chunit() {
        let length = len(18);
        let height: RowHeight = length.into();

        // Both should have the same underlying ChUnit value.
        assert_eq!(length.0, height.0);
    }

    #[test]
    fn test_different_conversions_independence() {
        // Verify that ColWidth and RowHeight conversions work independently.
        let length = len(50);

        let width: ColWidth = length.into();
        let height: RowHeight = length.into();

        // All three should have the same underlying value.
        assert_eq!(length.0, width.0);
        assert_eq!(length.0, height.0);
        assert_eq!(width.0, height.0);
    }
}

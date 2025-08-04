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
          hash::Hash,
          ops::{Add, AddAssign, Deref, DerefMut, Div, Sub, SubAssign}};

use super::{ChUnit, Index, idx};
use crate::ch;

/// Represents a length measurement in character units.
///
/// A Length is a 1-based measurement (as opposed to 0-based indices) that represents
/// the size or extent of something in the terminal UI, such as the width or height
/// of a component. It wraps a [`ChUnit`] value.
///
/// Length values can be created using the [`Length::new`] method, the [len] function,
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

mod construct {
    use super::{ChUnit, Index, Length, ch, idx};

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
}

mod ops {
    use super::{Add, AddAssign, ChUnit, Deref, DerefMut, Div, Length, Sub, SubAssign};

    impl Deref for Length {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Length {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

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
        // Test with maximum u16 value
        let max_length = Length::new(u16::MAX);
        assert_eq!(max_length.as_u16(), u16::MAX);
    }

    #[test]
    fn test_length_zero() {
        // Test with zero
        let zero_length = Length::new(0);
        assert_eq!(zero_length.0, ch(0));

        // Converting zero length to index
        let index = zero_length.convert_to_index();
        assert_eq!(index.0, ch(0)); // Should be 0 since we don't go below 0
    }

    #[test]
    fn test_length_interop_with_index() {
        // Test interoperability with Index
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
        // Test addition near maximum value
        let max_length = Length::new(u16::MAX - 5);
        let small_length = Length::new(5);
        let result = max_length + small_length;
        assert_eq!(result, Length::new(u16::MAX));

        // Test subtraction with zero
        let length = Length::new(5);
        let result = length - Length::new(5);
        assert_eq!(result, Length::new(0));

        // Test subtraction below zero (should clamp to zero due to unsigned type)
        let length = Length::new(5);
        let result = length - Length::new(10);
        assert_eq!(result, Length::new(0));
    }
}

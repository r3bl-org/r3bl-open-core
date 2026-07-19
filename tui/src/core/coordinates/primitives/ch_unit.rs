// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Base character unit for monospace terminal measurements - see [`ChUnit`] type.

use crate::{LossyConvertToByte, NarrowingCastToI16, NarrowingCastToIsize,
            NarrowingCastToU16, NumericConversions, NumericValue, ScreenCoordinate,
            WideningCastToI32, WideningCastToU16, WideningCastToU32, WideningCastToUsize};
use std::{fmt::{Debug, Formatter},
          ops::{Add, AddAssign, Deref, Div, Mul, MulAssign, Sub, SubAssign}};

/// Represents a character unit or `ch` unit.
///
/// - This is a unit of measurement that is used to represent the width or height of a
///   character in a monospace font.
/// - The [terminal emulator] or [kernel virtual console] running (and displaying the UI)
///   of the Rust binary build using this crate will ultimately determine the actual width
///   and height of a character.
/// - In order to create values of `ch` unit, use [ch].
/// - The underlying primitive type for [`ChUnit`] is [`prim@u16`].
/// - This unit is unsigned and supports basic arithmetic operations with arguments that
///   have negative values.
/// - It has extensive support for conversion to and from other types.
///
/// [kernel virtual console]: https://en.wikipedia.org/wiki/Virtual_console
/// [terminal emulator]: https://en.wikipedia.org/wiki/Terminal_emulator
#[derive(Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ChUnit {
    pub value: u16,
}

impl ChUnit {
    #[must_use]
    pub fn new(value: u16) -> Self { Self { value } }

    #[must_use]
    pub fn as_usize(&self) -> usize { usize(*self) }

    #[must_use]
    pub fn as_u16(&self) -> u16 { self.value }

    #[must_use]
    pub fn as_u32(&self) -> u32 { u32(*self) }
}

/// ```
/// use r3bl_tui::{ch, ChUnit};
///
/// let it_usize: usize = 12;
/// let it_ch: ChUnit = ch(it_usize);
/// ```
pub fn ch(arg_num: impl Into<ChUnit>) -> ChUnit { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, usize};
///
/// let it_ch: ChUnit = ch(12);
/// let it_usize: usize = usize(it_ch);
/// ```
pub fn usize(arg_num: impl Into<usize>) -> usize { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, u32};
///
/// let it_ch: ChUnit = ch(12);
/// let it_u32: u32 = u32(it_ch);
/// ```
pub fn u32(arg_num: impl Into<u32>) -> u32 { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, isize};
///
/// let it_ch: ChUnit = ch(12);
/// let it_isize: isize = isize(it_ch);
/// ```
pub fn isize(arg_num: impl Into<isize>) -> isize { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, i32};
///
/// let it_ch: ChUnit = ch(12);
/// let it_i32: i32 = i32(it_ch);
/// ```
pub fn i32(arg_num: impl Into<i32>) -> i32 { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, i16};
///
/// let it_ch: ChUnit = ch(12);
/// let it_i16: i16 = i16(it_ch);
/// ```
pub fn i16(arg_num: impl Into<i16>) -> i16 { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, f64};
///
/// let it_ch: ChUnit = ch(12);
/// let it_f64: f64 = f64(it_ch);
/// ```
pub fn f64(arg_num: impl Into<f64>) -> f64 { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, u8};
///
/// let it_usize: usize = 12;
/// let it_ch: ChUnit = ch(it_usize);
/// let it_u8: u8 = u8(it_ch);
/// ```
pub fn u8(arg_num: impl Into<u8>) -> u8 { arg_num.into() }

/// ```
/// use r3bl_tui::{ch, ChUnit, u16};
///
/// let it_ch: ChUnit = ch(12);
/// let it_u16: u16 = u16(it_ch);
/// ```
pub fn u16(arg_num: impl Into<u16>) -> u16 { arg_num.into() }

impl Debug for ChUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Deref for ChUnit {
    type Target = u16;

    fn deref(&self) -> &Self::Target { &self.value }
}

pub mod ch_unit_math_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl MulAssign<ChUnit> for ChUnit {
        fn mul_assign(&mut self, rhs: Self) {
            self.value = self.value.saturating_mul(rhs.value);
        }
    }

    impl Add for ChUnit {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            ch(self.value.saturating_add(rhs.value))
        }
    }

    impl Add<u16> for ChUnit {
        type Output = Self;

        fn add(self, rhs: u16) -> Self::Output { ch(self.value.saturating_add(rhs)) }
    }

    impl AddAssign for ChUnit {
        fn add_assign(&mut self, rhs: Self) {
            self.value = self.value.saturating_add(rhs.value);
        }
    }

    impl AddAssign<u16> for ChUnit {
        fn add_assign(&mut self, rhs: u16) {
            self.value = self.value.saturating_add(rhs);
        }
    }

    impl Sub for ChUnit {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self::Output {
            ch(self.value.saturating_sub(rhs.value))
        }
    }

    impl Sub<u16> for ChUnit {
        type Output = Self;

        fn sub(self, rhs: u16) -> Self::Output { ch(self.value.saturating_sub(rhs)) }
    }

    impl SubAssign for ChUnit {
        fn sub_assign(&mut self, rhs: Self) {
            self.value = self.value.saturating_sub(rhs.value);
        }
    }

    impl SubAssign<u16> for ChUnit {
        fn sub_assign(&mut self, rhs: u16) {
            self.value = self.value.saturating_sub(rhs);
        }
    }

    impl Mul for ChUnit {
        type Output = Self;

        fn mul(self, rhs: Self) -> Self::Output {
            ch(self.value.saturating_mul(rhs.value))
        }
    }

    impl Mul<u16> for ChUnit {
        type Output = Self;

        fn mul(self, rhs: u16) -> Self::Output { ch(self.value.saturating_mul(rhs)) }
    }

    impl Div<u16> for ChUnit {
        type Output = Self;

        fn div(self, rhs: u16) -> Self::Output { ch(self.value / rhs) }
    }

    impl Div<ChUnit> for ChUnit {
        type Output = Self;

        fn div(self, rhs: ChUnit) -> Self::Output { ch(self.value / rhs.value) }
    }
}

/// Converts to other types [prim@f64], [prim@isize], [prim@usize], etc. from [`ChUnit`].
pub mod convert_to_other_types_from_ch {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<ChUnit> for u16 {
        fn from(arg: ChUnit) -> u16 { arg.value }
    }

    impl From<ChUnit> for u8 {
        fn from(arg: ChUnit) -> u8 { arg.value.to_u8_lossy() }
    }

    impl From<ChUnit> for f64 {
        fn from(arg: ChUnit) -> f64 { f64::from(arg.value) }
    }

    impl From<ChUnit> for i32 {
        fn from(arg: ChUnit) -> i32 { arg.value.as_i32_widening() }
    }

    impl From<ChUnit> for u32 {
        fn from(arg: ChUnit) -> u32 { arg.value.as_u32_widening() }
    }

    impl From<ChUnit> for usize {
        fn from(arg: ChUnit) -> usize { arg.value.as_usize_widening() }
    }

    impl From<ChUnit> for i16 {
        fn from(arg: ChUnit) -> i16 { arg.value.as_i16_narrowing() }
    }

    impl From<ChUnit> for isize {
        fn from(arg: ChUnit) -> isize { arg.value.as_isize_narrowing() }
    }
}

/// Converts from other types [prim@f64], [prim@isize], [prim@usize], etc. to [`ChUnit`].
pub mod convert_from_other_types_to_ch {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Safely convert the [`f64`] to [`u16`] by rounding it. The
    /// conversion can fail if the value is out of range of [`u16`] (negative number or
    /// greater than max [`u16`] capacity).
    ///
    /// This is what happens if an error occurs:
    /// - Generate a tracing error if the conversion fails.
    /// - Even if it fails, return `0` and consume the error.
    fn f64_to_u16(value: f64) -> Result<u16, String> {
        let value = value.round(); // Remove the fractional part by rounding up or down.
        if value < 0.0 || value > f64::from(u16::MAX) {
            return Err(format!("Failed to convert {value} to u16: out of range"));
        }
        // Convert the f64 to u16, which is safe now since we checked the range.
        // The value is guaranteed to be in the range [0, 65535].
        // The `as` keyword is the designated tool for primitive, potentially lossy
        // conversions. This trait provides a consistent interface for converting
        // various numeric types to [`u8`] with appropriate bounds checking where needed.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::as_conversions
        )]
        Ok(value as u16)
    }

    impl From<f64> for ChUnit {
        fn from(value: f64) -> ChUnit {
            let int_value: u16 = match f64_to_u16(value) {
                Ok(it) => it,
                Err(err) => {
                    // % is Display, ? is Debug.
                    tracing::error!(message = "Problem converting f64 to u16", err = err);
                    0
                }
            };

            ChUnit { value: int_value }
        }
    }

    impl From<f32> for ChUnit {
        fn from(value: f32) -> ChUnit {
            let int_value: u16 = match f64_to_u16(f64::from(value)) {
                Ok(it) => it,
                Err(err) => {
                    // % is Display, ? is Debug.
                    tracing::error!(message = "Problem converting f32 to u16", err = err);
                    0
                }
            };

            ChUnit { value: int_value }
        }
    }

    impl From<isize> for ChUnit {
        fn from(value: isize) -> ChUnit {
            ChUnit {
                // The `as` keyword is the designated tool for primitive, potentially
                // lossy conversions. This trait provides a consistent
                // interface for converting various numeric types to
                // [`u16`] with appropriate bounds checking where
                // needed.
                value: value.as_u16_narrowing(),
            }
        }
    }

    impl From<u8> for ChUnit {
        fn from(it: u8) -> ChUnit {
            let value = it.as_u16_widening();
            ChUnit { value }
        }
    }

    impl From<u16> for ChUnit {
        fn from(value: u16) -> ChUnit { ChUnit { value } }
    }

    impl From<usize> for ChUnit {
        fn from(value: usize) -> ChUnit {
            ChUnit {
                // The `as` keyword is the designated tool for primitive, potentially
                // lossy conversions. This trait provides a consistent
                // interface for converting various numeric types to
                // [`u16`] with appropriate bounds checking where
                // needed.
                value: value.as_u16_narrowing(),
            }
        }
    }

    impl From<i32> for ChUnit {
        fn from(value: i32) -> ChUnit {
            ChUnit {
                // The `as` keyword is the designated tool for primitive, potentially
                // lossy conversions. This trait provides a consistent
                // interface for converting various numeric types to
                // [`u16`] with appropriate bounds checking where
                // needed.
                value: value.as_u16_narrowing(),
            }
        }
    }

    impl From<u32> for ChUnit {
        fn from(value: u32) -> ChUnit {
            ChUnit {
                // The `as` keyword is the designated tool for primitive, potentially
                // lossy conversions. This trait provides a consistent
                // interface for converting various numeric types to
                // [`u16`] with appropriate bounds checking where
                // needed.
                value: value.as_u16_narrowing(),
            }
        }
    }

    impl From<i16> for ChUnit {
        fn from(value: i16) -> ChUnit {
            ChUnit {
                // The `as` keyword is the designated tool for primitive, potentially
                // lossy conversions. This trait provides a consistent
                // interface for converting various numeric types to
                // [`u16`] with appropriate bounds checking where
                // needed.
                value: value.as_u16_narrowing(),
            }
        }
    }
}

/// Implementation of [`NumericValue`] trait for [`ChUnit`].
///
/// This enables `ChUnit` to participate in the bounds checking type system.
/// All wrapper types (`RowIndex`, `ColWidth`, etc.) delegate to this implementation.
///
/// [`NumericValue`]: crate::NumericValue
mod bounds_check_trait_impls {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NumericConversions for ChUnit {
        fn as_usize(&self) -> usize { self.value.as_usize_widening() }
    }

    impl NumericValue for ChUnit {}

    impl ScreenCoordinate for ChUnit {
        fn as_u16(&self) -> u16 { self.value }
    }
}

#[cfg(test)]
mod tests_convert {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_as_usize() {
        let ch_1: ChUnit = ch(1);
        assert_eq2!(ch_1.as_usize(), 1);
    }

    #[test]
    fn test_from_whatever_into_ch() {
        let ch_1: ChUnit = ch(1);
        assert_eq2!(*ch_1, 1);

        let ch_2: ChUnit = ch(1) + ch(1);
        assert_eq2!(*ch_2, 2);

        let ch_3: ChUnit = ch(1) - ch(1);
        assert_eq2!(*ch_3, 0);

        let ch_4: ChUnit = ch(0) - ch(1);
        assert_eq2!(*ch_4, 0);
    }

    #[test]
    fn test_from_ch_into_usize() {
        let usize_1: usize = usize(ch(1));
        assert_eq2!(usize_1, 1);

        let usize_2: usize = usize(ch(1) + ch(1));
        assert_eq2!(usize_2, 2);

        let usize_3: usize = usize(ch(1) - ch(1));
        assert_eq2!(usize_3, 0);

        let usize_4: usize = usize(ch(0) - ch(1));
        assert_eq2!(usize_4, 0);
    }

    #[test]
    fn test_from_ch_into_u16() {
        let u16_1: u16 = u16(ch(1));
        assert_eq2!(u16_1, 1);

        let u16_2: u16 = u16(ch(1) + ch(1));
        assert_eq2!(u16_2, 2);

        let u16_3: u16 = u16(ch(1) - ch(1));
        assert_eq2!(u16_3, 0);

        let u16_4: u16 = u16(ch(0) - ch(1));
        assert_eq2!(u16_4, 0);
    }

    #[test]
    fn test_from_ch_into_i16() {
        let i16_1: i16 = i16::from(ch(1));
        assert_eq2!(i16_1, 1i16);

        let i16_max: i16 = i16::from(ch(32767));
        assert_eq2!(i16_max, 32767i16);

        // Test saturating narrowing cast when ChUnit > i16::MAX
        let i16_overflow: i16 = i16::from(ch(40000));
        assert_eq2!(i16_overflow, i16::MAX);
    }

    #[test]
    fn test_from_ch_into_isize() {
        let isize_1: isize = isize::from(ch(1));
        assert_eq2!(isize_1, 1isize);

        let isize_val: isize = isize::from(ch(65535));
        assert_eq2!(isize_val, 65535isize);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_from_ch_into_i32_u32_u8_f64() {
        assert_eq2!(i32::from(ch(100)), 100i32);
        assert_eq2!(u32::from(ch(100)), 100u32);
        assert_eq2!(u8::from(ch(100)), 100u8);
        assert_eq2!(u8::from(ch(300)), 44u8);
        assert_eq2!(f64::from(ch(100)), 100.0f64);
    }
}

#[cfg(test)]
mod tests_ch_unit_math_ops {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_add_ch_units() {
        let ch_1: ChUnit = ch(1);
        let ch_2: ChUnit = ch(2);
        let result: ChUnit = ch_1 + ch_2;
        assert_eq2!(*result, 3);
    }

    #[test]
    fn test_add_assign_ch_units() {
        let mut ch_1: ChUnit = ch(1);
        let ch_2: ChUnit = ch(2);
        ch_1 += ch_2;
        assert_eq2!(*ch_1, 3);
    }

    #[test]
    fn test_sub_ch_units() {
        let ch_1: ChUnit = ch(3);
        let ch_2: ChUnit = ch(1);
        let result: ChUnit = ch_1 - ch_2;
        assert_eq2!(*result, 2);
    }

    #[test]
    fn test_sub_assign_ch_units() {
        let mut ch_1: ChUnit = ch(3);
        let ch_2: ChUnit = ch(1);
        ch_1 -= ch_2;
        assert_eq2!(*ch_1, 2);
    }

    #[test]
    fn test_mul_ch_units() {
        let ch_1: ChUnit = ch(2);
        let ch_2: ChUnit = ch(3);
        let result: ChUnit = ch_1 * ch_2;
        assert_eq2!(*result, 6);
    }

    #[test]
    fn test_mul_assign_ch_units() {
        let mut ch_1: ChUnit = ch(2);
        let ch_2: ChUnit = ch(3);
        ch_1 *= ch_2;
        assert_eq2!(*ch_1, 6);
    }

    #[test]
    fn test_div_ch_units() {
        let ch_1: ChUnit = ch(6);
        let result: ChUnit = ch_1 / 2;
        assert_eq2!(*result, 3);
    }
}

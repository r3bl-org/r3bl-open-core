/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use std::{fmt::{Debug, Formatter},
          ops::{Add, AddAssign, Deref, Div, Mul, MulAssign, Sub, SubAssign}};

use crate::{add_unsigned, mul_unsigned, sub_unsigned};

/// The backing field that is used to represent a [ChUnit] in memory.
pub type ChUnitPrimitiveType = u16;

/// Represents a character unit or `ch` unit.
///
/// - This is a unit of measurement that is used to represent the width or height of a
///   character in a monospace font.
/// - The terminal displaying the Rust binary build using the tui library will ultimately
///   determine the actual width and height of a character.
/// - In order to create values of `ch` unit, use [ch].
/// - The underlying primitive type for [ChUnit] is [ChUnitPrimitiveType]. The use of the
///   type alias allows for this to be changed in the future if needed.
/// - This unit is unsigned and supports basic arithmetic operations with arguments that
///   have negative values.
/// - It has extensive support for conversion to and from other types.
#[derive(Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ChUnit {
    pub value: ChUnitPrimitiveType,
}

impl ChUnit {
    pub fn new(value: ChUnitPrimitiveType) -> Self { Self { value } }

    pub fn as_usize(&self) -> usize { usize(*self) }

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
    type Target = ChUnitPrimitiveType;

    fn deref(&self) -> &Self::Target { &self.value }
}

pub mod ch_unit_math_ops {
    use super::*;

    impl MulAssign<ChUnit> for ChUnit {
        fn mul_assign(&mut self, rhs: Self) {
            self.value = mul_unsigned!(self.value, rhs.value);
        }
    }

    impl Add for ChUnit {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            ch(add_unsigned!(self.value, rhs.value))
        }
    }

    impl Add<u16> for ChUnit {
        type Output = Self;

        fn add(self, rhs: u16) -> Self::Output { ch(add_unsigned!(self.value, rhs)) }
    }

    impl AddAssign for ChUnit {
        fn add_assign(&mut self, rhs: Self) {
            self.value = add_unsigned!(self.value, rhs.value);
        }
    }

    impl AddAssign<u16> for ChUnit {
        fn add_assign(&mut self, rhs: u16) {
            self.value = add_unsigned!(self.value, rhs);
        }
    }

    impl Sub for ChUnit {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self::Output {
            ch(sub_unsigned!(self.value, rhs.value))
        }
    }

    impl Sub<u16> for ChUnit {
        type Output = Self;

        fn sub(self, rhs: u16) -> Self::Output { ch(sub_unsigned!(self.value, rhs)) }
    }

    impl SubAssign for ChUnit {
        fn sub_assign(&mut self, rhs: Self) {
            self.value = sub_unsigned!(self.value, rhs.value);
        }
    }

    impl SubAssign<u16> for ChUnit {
        fn sub_assign(&mut self, rhs: u16) {
            self.value = sub_unsigned!(self.value, rhs);
        }
    }

    impl Mul for ChUnit {
        type Output = Self;

        fn mul(self, rhs: Self) -> Self::Output {
            ch(mul_unsigned!(self.value, rhs.value))
        }
    }

    impl Mul<u16> for ChUnit {
        type Output = Self;

        fn mul(self, rhs: u16) -> Self::Output { ch(mul_unsigned!(self.value, rhs)) }
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

/// Convert to other types [prim@f64], [prim@isize], [prim@usize], etc. from [ChUnit].
pub mod convert_to_other_types_from_ch {
    use super::*;

    impl From<ChUnit> for u8 {
        fn from(arg: ChUnit) -> Self { arg.value as u8 }
    }

    impl From<ChUnit> for f64 {
        fn from(arg: ChUnit) -> Self { arg.value as f64 }
    }

    impl From<ChUnit> for isize {
        fn from(arg: ChUnit) -> Self { arg.value as isize }
    }

    impl From<ChUnit> for i32 {
        fn from(arg: ChUnit) -> Self { arg.value as i32 }
    }

    impl From<ChUnit> for u32 {
        fn from(arg: ChUnit) -> Self { arg.value as u32 }
    }

    impl From<ChUnit> for i16 {
        fn from(arg: ChUnit) -> Self { arg.value as i16 }
    }

    impl From<ChUnit> for usize {
        fn from(arg: ChUnit) -> Self { arg.value as usize }
    }

    impl From<ChUnit> for ChUnitPrimitiveType {
        fn from(arg: ChUnit) -> Self { arg.value }
    }
}

/// Convert from other types [prim@f64], [prim@isize], [prim@usize], etc. to [ChUnit].
pub mod convert_from_other_types_to_ch {
    use super::*;

    /// Safely convert the f64 to u16 by rounding it. The conversion can fail if the value
    /// is out of range of u16 (negative number or greater than max u16 capacity).
    ///
    /// This is what happens if an error occurs:
    /// - Generate a tracing error if the conversion fails.
    /// - Even if it fails, return `0` and consume the error.
    fn f64_to_u16(value: f64) -> Result<u16, String> {
        let value = value.round(); // Remove the fractional part by rounding up or down.
        if value < 0.0 || value > u16::MAX as f64 {
            return Err(format!("Failed to convert {value} to u16: out of range"));
        }
        Ok(value as u16)
    }

    impl From<f64> for ChUnit {
        fn from(value: f64) -> Self {
            let int_value: u16 = match f64_to_u16(value) {
                Ok(it) => it,
                Err(err) => {
                    // % is Display, ? is Debug.
                    tracing::error!(message = "Problem converting f64 to u16", err = err);
                    0
                }
            };

            Self {
                value: int_value as ChUnitPrimitiveType,
            }
        }
    }

    impl From<f32> for ChUnit {
        fn from(value: f32) -> Self {
            let int_value: u16 = match f64_to_u16(f64::from(value)) {
                Ok(it) => it,
                Err(err) => {
                    // % is Display, ? is Debug.
                    tracing::error!(message = "Problem converting f32 to u16", err = err);
                    0
                }
            };

            Self {
                value: int_value as ChUnitPrimitiveType,
            }
        }
    }

    impl From<isize> for ChUnit {
        fn from(value: isize) -> Self {
            Self {
                value: value.try_into().unwrap_or(value as ChUnitPrimitiveType),
            }
        }
    }

    impl From<u8> for ChUnit {
        fn from(it: u8) -> Self {
            let value = it as ChUnitPrimitiveType;
            Self { value }
        }
    }

    impl From<ChUnitPrimitiveType> for ChUnit {
        fn from(value: ChUnitPrimitiveType) -> Self { Self { value } }
    }

    impl From<usize> for ChUnit {
        fn from(value: usize) -> Self {
            Self {
                value: value.try_into().unwrap_or(value as ChUnitPrimitiveType),
            }
        }
    }

    impl From<i32> for ChUnit {
        fn from(value: i32) -> Self {
            Self {
                value: value.try_into().unwrap_or(value as ChUnitPrimitiveType),
            }
        }
    }

    impl From<u32> for ChUnit {
        fn from(value: u32) -> Self {
            Self {
                value: value.try_into().unwrap_or(value as ChUnitPrimitiveType),
            }
        }
    }

    impl From<i16> for ChUnit {
        fn from(value: i16) -> Self {
            Self {
                value: value.try_into().unwrap_or(value as ChUnitPrimitiveType),
            }
        }
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

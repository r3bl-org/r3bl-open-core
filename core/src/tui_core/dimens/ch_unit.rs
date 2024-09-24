/*
 *   Copyright (c) 2022 R3BL LLC
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

use std::{fmt::{Debug, Display, Formatter},
          ops::Deref};

use serde::{Deserialize, Serialize};

use crate::{add_unsigned, mul_unsigned, sub_unsigned};

/// The backing field that is used to represent a [ChUnit] in memory.
pub type ChUnitPrimitiveType = u16;

/// Represents a character unit or "ch" unit.
///
/// - This is a unit of measurement that is used to represent the width or height of a
///   character in a monospace font.
/// - The terminal displaying the Rust binary build using the tui library will ultimately
///   determine the actual width and height of a character.
/// - In order to create amounts of ch units, use the [ch!] macro.
#[derive(
    Copy,
    Clone,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    Eq,
    Hash,
    size_of::SizeOf,
)]
pub struct ChUnit {
    pub value: ChUnitPrimitiveType,
}

impl ChUnit {
    pub fn new(value: ChUnitPrimitiveType) -> Self { Self { value } }
}

/// Creates a new [ChUnit] amount.
///
/// ```rust
/// use r3bl_rs_utils_core::*;
/// let width_col = ch!(10);
/// let height_row = ch!(5, @inc);
/// let height_row = ch!(5, @inc);
/// ```
///
/// You can also convert a [ChUnit] amount into a [usize] primitive.
///
/// ```rust
/// use r3bl_rs_utils_core::*;
/// let width_col = ch!(10);
/// let width_col_usize: usize = ch!(@to_usize width_col);
/// let width_col_usize: usize = ch!(@to_usize width_col, @inc);
/// let width_col_usize: usize = ch!(@to_usize width_col, @dec);
/// ```
#[macro_export]
macro_rules! ch {
    // Returns ChUnit.
    ($arg: expr) => {{
        let ch_value: $crate::ChUnit = $arg.into();
        ch_value
    }};
    // Returns ChUnit +=1.
    ($arg: expr, @inc) => {{
        let mut ch_value: $crate::ChUnit = $arg.into();
        ch_value += 1;
        ch_value
    }};
    // Returns ChUnit -=1.
    ($arg: expr, @dec) => {{
        let mut ch_value: $crate::ChUnit = $arg.into();
        ch_value -= 1;
        ch_value
    }};
    // Returns isize.
    (@to_isize $arg: expr) => {{
        let isize_value: isize = $arg.into();
        isize_value
    }};
    // Returns usize.
    (@to_usize $arg: expr) => {{
        let usize_value: usize = $arg.into();
        usize_value
    }};
    // Returns usize +=1.
    (@to_usize $arg: expr, @inc) => {{
        let mut ch_value_copy = $arg.clone();
        ch_value_copy += 1;
        let usize_value: usize = ch_value_copy.into();
        usize_value
    }};
    // Returns usize -=1.
    (@to_usize $arg: expr, @dec) => {{
        let mut ch_value_copy = $arg.clone();
        ch_value_copy -= 1;
        let usize_value: usize = ch_value_copy.into();
        usize_value
    }};
    // Returns u16.
    (@to_u16 $arg: expr) => {{
        let u16_value: u16 = *$arg;
        u16_value
    }};
    // Returns u16 +=1.
    (@to_u16 $arg: expr, @inc) => {{
        let mut ch_value_copy = $arg.clone();
        ch_value_copy += 1;
        let u16_value: u16 = *ch_value_copy;
        u16_value
    }};
    // Returns u16 -=1.
    (@to_u16 $arg: expr, @dec) => {{
        let mut ch_value_copy = $arg.clone();
        ch_value_copy -= 1;
        let u16_value: u16 = *ch_value_copy;
        u16_value
    }};
}

impl Debug for ChUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Display for ChUnit {
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

    impl std::ops::Add for ChUnit {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            ch!(add_unsigned!(self.value, rhs.value))
        }
    }

    impl std::ops::Add<u16> for ChUnit {
        type Output = Self;

        fn add(self, rhs: u16) -> Self::Output { ch!(add_unsigned!(self.value, rhs)) }
    }

    impl std::ops::AddAssign for ChUnit {
        fn add_assign(&mut self, rhs: Self) {
            self.value = add_unsigned!(self.value, rhs.value);
        }
    }

    impl std::ops::AddAssign<u16> for ChUnit {
        fn add_assign(&mut self, rhs: u16) {
            self.value = add_unsigned!(self.value, rhs);
        }
    }

    impl std::ops::Sub for ChUnit {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self::Output {
            ch!(sub_unsigned!(self.value, rhs.value))
        }
    }

    impl std::ops::Sub<u16> for ChUnit {
        type Output = Self;

        fn sub(self, rhs: u16) -> Self::Output { ch!(sub_unsigned!(self.value, rhs)) }
    }

    impl std::ops::SubAssign for ChUnit {
        fn sub_assign(&mut self, rhs: Self) {
            self.value = sub_unsigned!(self.value, rhs.value);
        }
    }

    impl std::ops::SubAssign<u16> for ChUnit {
        fn sub_assign(&mut self, rhs: u16) {
            self.value = sub_unsigned!(self.value, rhs);
        }
    }

    impl std::ops::Mul for ChUnit {
        type Output = Self;

        fn mul(self, rhs: Self) -> Self::Output {
            ch!(mul_unsigned!(self.value, rhs.value))
        }
    }

    impl std::ops::Mul<u16> for ChUnit {
        type Output = Self;

        fn mul(self, rhs: u16) -> Self::Output { ch!(mul_unsigned!(self.value, rhs)) }
    }

    impl std::ops::Div<u16> for ChUnit {
        type Output = Self;

        fn div(self, rhs: u16) -> Self::Output { ch!(self.value / rhs) }
    }
}

pub mod convert_to_number {
    use super::*;

    impl From<ChUnit> for isize {
        fn from(arg: ChUnit) -> Self { arg.value as isize }
    }

    impl From<ChUnit> for usize {
        fn from(arg: ChUnit) -> Self { arg.value as usize }
    }

    impl From<ChUnit> for ChUnitPrimitiveType {
        fn from(arg: ChUnit) -> Self { arg.value }
    }
}

pub mod convert_from_number {
    use super::*;

    impl From<isize> for ChUnit {
        fn from(value: isize) -> Self {
            Self {
                value: value.try_into().unwrap_or(value as ChUnitPrimitiveType),
            }
        }
    }

    impl From<u8> for ChUnit {
        fn from(it: u8) -> Self { Self { value: it.into() } }
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
}

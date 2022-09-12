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

/// Safely subtracts two unsigned numbers and returns the result. Does not panic.
///
/// ```rust
/// use r3bl_rs_utils_core::*;
/// let a: u16 = 10;
/// let b: u16 = 15;
/// let c: u16 = sub_unsigned!(a, b);
/// assert_eq!(c, 0);
/// ```
#[macro_export]
macro_rules! sub_unsigned {
  ($arg_lhs: expr, $arg_rhs: expr) => {
    if $arg_lhs > $arg_rhs {
      $arg_lhs - $arg_rhs
    } else {
      0
    }
  };
}

/// Safely adds two unsigned numbers and returns the result. Does not panic.
///
/// ```rust
/// use r3bl_rs_utils_core::*;
/// let a: u16 = 10;
/// let b: u16 = 15;
/// let c: u16 = add_unsigned!(a, b);
/// assert_eq!(c, 25);
/// ```
///
/// More info: <https://rust-lang.github.io/rust-clippy/master/index.html#absurd_extreme_comparisons>
#[macro_export]
macro_rules! add_unsigned {
  ($arg_lhs: expr, $arg_rhs: expr) => {{
    type BigInt = u64;
    let lhs_big: BigInt = $arg_lhs as BigInt;
    let rhs_big: BigInt = $arg_rhs as BigInt;
    let sum_big: BigInt = lhs_big + rhs_big;

    if sum_big > ChUnitPrimitiveType::MAX as BigInt {
      ChUnitPrimitiveType::MAX
    } else {
      $arg_lhs + $arg_rhs
    }
  }};
}

#[macro_export]
macro_rules! mul_unsigned {
  ($arg_lhs: expr, $arg_rhs: expr) => {{
    type BigInt = u64;
    let lhs_big: BigInt = $arg_lhs as BigInt;
    let rhs_big: BigInt = $arg_rhs as BigInt;
    let mul_big: BigInt = lhs_big + rhs_big;

    if mul_big > ChUnitPrimitiveType::MAX as BigInt {
      ChUnitPrimitiveType::MAX
    } else {
      $arg_lhs * $arg_rhs
    }
  }};
}

/// Safely increments an unsigned number. Does not panic.
///
/// ```rust
/// use r3bl_rs_utils_core::*;
/// let mut my_u16: u16 = 0;
///
/// inc_unsigned!(my_u16);
/// assert_eq!(my_u16, 1);
///
/// inc_unsigned!(my_u16, by: 10);
/// assert_eq!(my_u16, 11);
///
/// inc_unsigned!(my_u16, max: 10);
/// assert_eq!(my_u16, 10);
/// ```
#[macro_export]
macro_rules! inc_unsigned {
  ($arg_lhs: expr) => {
    $arg_lhs = add_unsigned!($arg_lhs, 1);
  };
  ($arg_lhs: expr, by: $arg_amount: expr) => {
    $arg_lhs = add_unsigned!($arg_lhs, $arg_amount);
  };
  ($arg_lhs: expr, max: $arg_max: expr) => {
    if $arg_lhs >= $arg_max {
      $arg_lhs = $arg_max;
    } else {
      $arg_lhs = add_unsigned!($arg_lhs, 1);
    }
  };
  ($arg_lhs: expr, by: $arg_amount:expr, max: $arg_max: expr) => {
    if add_unsigned!($arg_lhs, $arg_amount) >= $arg_max {
      $arg_lhs = $arg_max;
    } else {
      $arg_lhs = add_unsigned!($arg_lhs, $arg_amount);
    }
  };
}

/// Safely decrements an unsigned number. Does not panic.
///
/// ```rust
/// use r3bl_rs_utils_core::*;
/// let mut my_u16: u16 = 10;
///
/// dec_unsigned!(my_u16);
/// assert_eq!(my_u16, 9);
///
/// dec_unsigned!(my_u16, by: 10);
/// assert_eq!(my_u16, 0);
/// ```
#[macro_export]
macro_rules! dec_unsigned {
  ($arg_lhs: expr) => {
    if $arg_lhs > 1 {
      $arg_lhs = sub_unsigned!($arg_lhs, 1);
    } else {
      $arg_lhs = 0;
    }
  };
  ($arg_lhs: expr, by: $arg_amount: expr) => {
    $arg_lhs = sub_unsigned!($arg_lhs, $arg_amount);
  };
}

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

use std::fmt::Write;

use crate::StringStorage;

/// Safely subtracts two unsigned numbers and returns the result. Does not panic.
///
/// ```rust
/// use r3bl_core::*;
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
/// use r3bl_core::*;
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
/// use r3bl_core::*;
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
/// use r3bl_core::*;
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

/// Format the given number of bytes as kilobytes with commas. If the number of bytes is
/// less than 1024, it will be formatted as bytes.
pub fn format_as_kilobytes_with_commas(bytes_size: usize) -> StringStorage {
    if bytes_size < 1024 {
        let mut acc = format_with_commas(bytes_size);
        _ = write!(acc, " B");
        acc
    } else {
        let kilobytes = bytes_size / 1024;
        let mut acc = format_with_commas(kilobytes);
        _ = write!(acc, " KB");
        acc
    }
}

#[test]
fn test_format_as_kilobytes_with_commas() {
    // Test with a number of bytes less than 1024.
    {
        let bytes_size = 512;
        let result = format_as_kilobytes_with_commas(bytes_size);
        assert_eq!(result, "512 B");
    }

    // Test with a number of bytes equal to 1024.
    {
        let bytes_size = 1024;
        let result = format_as_kilobytes_with_commas(bytes_size);
        assert_eq!(result, "1 KB");
    }

    // Test with a number of bytes greater than 1024.
    {
        let bytes_size = 2048;
        let result = format_as_kilobytes_with_commas(bytes_size);
        assert_eq!(result, "2 KB");
    }
}

/// Format a number with commas.
pub fn format_with_commas(num: usize) -> StringStorage {
    let num_str = num.to_string();

    let ir = {
        let mut acc = StringStorage::with_capacity(num_str.len());
        for (count, ch) in num_str.chars().rev().enumerate() {
            if count % 3 == 0 && count != 0 {
                acc.push(',');
            }
            acc.push(ch);
        }
        acc
    };

    {
        let mut acc = StringStorage::with_capacity(ir.len());
        for ch in ir.chars().rev() {
            acc.push(ch);
        }
        acc
    }
}

#[test]
fn test_format_with_commas() {
    // Test with a single-digit number.
    {
        let num = 5;
        let result = format_with_commas(num);
        assert_eq!(result, "5");
    }

    // Test with a two-digit number.
    {
        let num = 12;
        let result = format_with_commas(num);
        assert_eq!(result, "12");
    }

    // Test with a three-digit number.
    {
        let num = 123;
        let result = format_with_commas(num);
        assert_eq!(result, "123");
    }

    // Test with a six-digit number.
    {
        let num = 987654;
        let result = format_with_commas(num);
        assert_eq!(result, "987,654");
    }

    // Test with a nine-digit number.
    {
        let num = 123456789;
        let result = format_with_commas(num);
        assert_eq!(result, "123,456,789");
    }
}

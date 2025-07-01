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

use std::fmt::Write;

use crate::{constants::COMMA_CHAR, InlineString};

/// Safely subtracts two unsigned numbers and returns the result. Does not panic.
///
/// ```
/// use r3bl_tui::*;
/// let a: u16 = 10;
/// let b: u16 = 15;
/// let c: u16 = sub_unsigned!(a, b);
/// assert_eq!(c, 0);
/// ```
#[macro_export]
macro_rules! sub_unsigned {
    ($arg_lhs: expr, $arg_rhs: expr) => {
        $arg_lhs.saturating_sub($arg_rhs)
    };
}

/// Safely adds two unsigned numbers and returns the result. Does not panic.
///
/// ```
/// use r3bl_tui::*;
/// let a: u16 = 10;
/// let b: u16 = 15;
/// let c: u16 = add_unsigned!(a, b);
/// assert_eq!(c, 25);
/// ```
///
/// More info: <https://rust-lang.github.io/rust-clippy/master/index.html#absurd_extreme_comparisons>
#[macro_export]
macro_rules! add_unsigned {
    ($arg_lhs: expr, $arg_rhs: expr) => {
        $arg_lhs.saturating_add($arg_rhs)
    };
}

#[macro_export]
macro_rules! mul_unsigned {
    ($arg_lhs: expr, $arg_rhs: expr) => {
        $arg_lhs.saturating_mul($arg_rhs)
    };
}

/// Safely increments an unsigned number. Does not panic.
///
/// ```
/// use r3bl_tui::*;
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
        $arg_lhs = $arg_lhs.saturating_add(1);
    };
    ($arg_lhs: expr, by: $arg_amount: expr) => {
        $arg_lhs = $arg_lhs.saturating_add($arg_amount);
    };
    ($arg_lhs: expr, max: $arg_max: expr) => {
        $arg_lhs = std::cmp::min($arg_lhs.saturating_add(1), $arg_max);
    };
    ($arg_lhs: expr, by: $arg_amount:expr, max: $arg_max: expr) => {
        $arg_lhs = std::cmp::min($arg_lhs.saturating_add($arg_amount), $arg_max);
    };
}

/// Safely decrements an unsigned number. Does not panic.
///
/// ```
/// use r3bl_tui::*;
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
        $arg_lhs = $arg_lhs.saturating_sub(1);
    };
    ($arg_lhs: expr, by: $arg_amount: expr) => {
        $arg_lhs = $arg_lhs.saturating_sub($arg_amount);
    };
}

/// Format the given number of bytes as kilobytes with commas. If the number of bytes is
/// less than 1024, it will be formatted as bytes.
#[must_use]
pub fn format_as_kilobytes_with_commas(bytes_size: usize) -> InlineString {
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
#[must_use]
pub fn format_with_commas(num: usize) -> InlineString {
    let num_str = num.to_string();

    let ir = {
        let mut acc = InlineString::with_capacity(num_str.len());
        for (digit_position, ch) in num_str.chars().rev().enumerate() {
            // Skip the first digit (position 0), then add comma every 3 digits.
            let should_add_comma = digit_position > 0 && digit_position.is_multiple_of(3);
            if should_add_comma {
                acc.push(COMMA_CHAR);
            }
            acc.push(ch);
        }
        acc
    };

    {
        let mut acc = InlineString::with_capacity(ir.len());
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
        let num = 987_654;
        let result = format_with_commas(num);
        assert_eq!(result, "987,654");
    }

    // Test with a nine-digit number.
    {
        let num = 123_456_789;
        let result = format_with_commas(num);
        assert_eq!(result, "123,456,789");
    }
}

/// Trait for performing potentially lossy conversions from primitive types to `u8`. Avoid
/// triggering warnings from:
/// - `clippy::cast_sign_loss`
/// - `clippy::cast_lossless`
/// - `clippy::cast_possible_truncation`
///
/// The `as` keyword is the designated tool for primitive, potentially lossy
/// conversions. This trait provides a consistent interface for converting
/// various numeric types to [`u8`] with appropriate bounds checking where needed.
///
/// See also:
/// - [`crate::ChUnitPrimitiveType`]: This is the type used for unit values in the TUI
///   library.
/// - [`crate::ChUnit`]: This is the type used for unit values and conversions in the TUI
///   library.
pub trait LossyConvertToByte {
    /// Intentionally converts the value to a [`u8`] with direct casting, potentially
    /// losing precision or clamping values. Values outside the valid range may
    /// produce unexpected results.
    #[must_use]
    fn to_u8_lossy(self) -> u8;
}

impl LossyConvertToByte for f64 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for f32 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for i32 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for u32 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for usize {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for u64 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for u16 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for i16 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

impl LossyConvertToByte for i8 {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn to_u8_lossy(self) -> u8 { self as u8 }
}

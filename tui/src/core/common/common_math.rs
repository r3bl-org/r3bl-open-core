// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{InlineString, constants::COMMA_CHAR};
use std::fmt::Write;

/// Format the given number of bytes as kilobytes with commas. If the number of bytes is
/// less than 1024, it will be formatted as bytes.
#[must_use]
pub fn format_as_kilobytes_with_commas(bytes_size: usize) -> InlineString {
    if bytes_size < 1024 {
        let mut acc = format_with_commas(bytes_size);
        // We don't care about the result of this operation.
        write!(acc, " B").ok();
        acc
    } else {
        let kilobytes = bytes_size / 1024;
        let mut acc = format_with_commas(kilobytes);
        // We don't care about the result of this operation.
        write!(acc, " KB").ok();
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

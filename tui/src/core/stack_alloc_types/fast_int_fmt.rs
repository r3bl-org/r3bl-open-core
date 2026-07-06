// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Fast, zero-allocation integer-to-string conversion using fixed-size stack arrays.
//!
//! See [`macro@crate::convert_u16_to_ascii_str_slice`] and
//! [`macro@crate::convert_usize_to_ascii_str_slice`] for the primary public API.

use std::debug_assert_matches;

/// A highly optimized macro that takes a [`u16`] and returns a temporary `&str` slice
/// pointing to its [`ASCII`] representation without any heap allocations.
///
/// ```text
///                                                            "4" "2"
/// ┌──────────────┐     u16_to_u8_array()   ┌──────────────────▲───▲───┐
/// │ u16 (e.g 42) │ ──────────────────────► │ Stack: [0, 0, 0, 52, 50] │
/// └──────────────┘                         └────────────┬─────────────┘
///                                                       │ convert_u16_to_str_slice()
///                                          ┌────────────▼─────────────┐
///                                          │ &str: "42" (points to    │
///                                          │       stack memory)      │
///                                          └──────────────────────────┘
/// ```
///
/// # Examples
///
/// ```
/// use r3bl_tui::convert_u16_to_ascii_str_slice;
///
/// let mut acc = String::new();
/// acc.push_str(convert_u16_to_ascii_str_slice!(42_u16));
/// assert_eq!(acc, "42");
/// ```
///
/// **Note on lifetimes:** Due to Rust's temporary lifetime extension rules, you can pass
/// this macro directly into functions like [`String::push_str(...)`] and the temporary
/// stack array will live precisely until the end of the statement.
///
/// Do *not* try to assign its result to a local `let` binding, or you will get a
/// "temporary value dropped while borrowed" compiler error.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`String::push_str(...)`]: String::push_str
#[macro_export]
macro_rules! convert_u16_to_ascii_str_slice {
    ($num:expr) => {
        $crate::core::stack_alloc_types::u16_impl::convert_u16_to_str_slice(
            &$crate::core::stack_alloc_types::u16_impl::u16_to_u8_array($num),
        )
    };
}

/// A highly optimized macro that takes a [`usize`] and returns a temporary `&str` slice
/// pointing to its [`ASCII`] representation without any heap allocations.
///
/// ```text
///                                                               "4" "2"
/// ┌────────────────┐   usize_to_u8_array() ┌─────────────────────▲───▲───┐
/// │ usize (e.g 42) │ ────────────────────► │ Stack: [0, ... 0, 0, 52, 50]│
/// └────────────────┘                       └───────────────┬─────────────┘
///                                                          │
///                                                          │ convert_usize_to_str_slice()
///                                             ┌────────────▼─────────────┐
///                                             │ &str: "42" (points to    │
///                                             │       stack memory)      │
///                                             └──────────────────────────┘
/// ```
///
/// **Note on lifetimes:** Due to Rust's temporary lifetime extension rules, you can pass
/// this macro directly into functions like `String::push_str(...)` and the temporary
/// stack array will live precisely until the end of the statement.
///
/// Do *not* try to assign its result to a local `let` binding, or you will get a
/// "temporary value dropped while borrowed" compiler error.
///
/// # Examples
///
/// ```
/// use r3bl_tui::convert_usize_to_ascii_str_slice;
///
/// let mut acc = String::new();
/// acc.push_str(convert_usize_to_ascii_str_slice!(1234567890_usize));
/// assert_eq!(acc, "1234567890");
/// ```
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
#[macro_export]
macro_rules! convert_usize_to_ascii_str_slice {
    ($num:expr) => {
        $crate::core::stack_alloc_types::usize_impl::convert_usize_to_str_slice(
            &$crate::core::stack_alloc_types::usize_impl::usize_to_u8_array($num),
        )
    };
}

pub mod usize_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[must_use]
    pub fn usize_to_u8_array(num: usize) -> [u8; USIZE_FMT_MAX_DIGITS] {
        debug_assert_usize_fits_20_digits();

        let mut num_copy = num;

        let mut result = [0; USIZE_FMT_MAX_DIGITS]; // Initialize with zeros
        let mut index = USIZE_FMT_MAX_DIGITS - 1;

        if num_copy == 0 {
            result[index] = b'0';
            return result;
        }

        while num_copy > 0 && index > 0 {
            let digit = u8::try_from(num_copy % 10).unwrap_or(0);
            result[index] = b'0' + digit; // Convert digit to ASCII character
            num_copy /= 10;
            index -= 1;
        }

        result
    }

    /// This function converts a [u8] array to a string slice by removing leading zeros.
    /// It also trims the string slice to remove any trailing zeros. The call to
    /// `unwrap()` in this function should never panic because the input is a valid [u8]
    /// array.
    ///
    /// # Panics
    ///
    /// This function will panic if the input [u8] array is not a valid [`UTF-8`]
    /// sequence. This should never happen because the input is a fixed size [u8]
    /// array that is guaranteed to contain only [`ASCII`] digits (0-9) and null bytes
    /// (0).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn convert_usize_to_str_slice(arg: &[u8; USIZE_FMT_MAX_DIGITS]) -> &str {
        let result_str = std::str::from_utf8(arg);
        debug_assert_matches!(
            result_str,
            Ok(_),
            "Failed to convert u8 array to string slice" // Should never happen!
        );

        #[allow(
            clippy::unwrap_used,
            reason = "Numbers are mathematically guaranteed to be valid UTF-8"
        )]
        result_str.unwrap().trim_start_matches(char::from(0))
    }

    /// This is just a sanity check done in the debug release to makes sure that the
    /// maximum value of [usize] can be represented with [`USIZE_FMT_MAX_DIGITS`] decimal
    /// digits.
    fn debug_assert_usize_fits_20_digits() {
        // Calculate the maximum value of usize.
        let max_usize = usize::MAX;

        // Calculate the number of digits needed.
        let mut num_digits = 1; // At least one digit for 0
        let mut temp = max_usize;
        while temp >= 10 {
            temp /= 10;
            num_digits += 1;
        }

        // Assert that the number of digits does not exceed 20.
        debug_assert!(
            num_digits <= 20,
            "usize on this platform requires more than 20 decimal digits"
        );
    }
}

pub mod u16_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Converts a [u16] to a fixed-size [u8] array without heap allocation.
    ///
    /// This is optimized for terminal coordinates (rows/columns) which are typically
    /// [u16] values. The array is smaller (5 bytes vs 20) and conversion is faster
    /// than the [usize] version.
    ///
    /// # Performance
    ///
    /// - **Zero heap allocations** - Uses stack array
    /// - **Max 5 loop iterations** - vs 20 for usize
    /// - **Cache-friendly** - 5-byte array fits in single cache line
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::stack_alloc_types::u16_impl::{
    ///     u16_to_u8_array, convert_u16_to_str_slice
    /// };
    ///
    /// // Terminal coordinate (row 42)
    /// let row = 42_u16;
    /// let result = u16_to_u8_array(row);
    /// let result_str = convert_u16_to_str_slice(&result);
    /// assert_eq!(result_str, "42");
    ///
    /// // Maximum terminal size
    /// let max_coord = 65535_u16;
    /// let result = u16_to_u8_array(max_coord);
    /// let result_str = convert_u16_to_str_slice(&result);
    /// assert_eq!(result_str, "65535");
    /// ```
    #[must_use]
    pub fn u16_to_u8_array(num: u16) -> [u8; U16_FMT_MAX_DIGITS] {
        let mut num_copy = num;
        let mut result = [0; U16_FMT_MAX_DIGITS]; // Initialize with zeros
        let mut index = U16_FMT_MAX_DIGITS - 1;

        if num_copy == 0 {
            result[index] = b'0';
            return result;
        }

        while num_copy > 0 {
            let digit = (num_copy % 10) as u8;
            result[index] = b'0' + digit; // Convert digit to ASCII character
            num_copy /= 10;
            index = index.saturating_sub(1);
        }

        result
    }

    /// Converts a [u16] [u8] array to a string slice by removing leading zeros.
    ///
    /// This is the companion function to [`u16_to_u8_array`]. It trims the leading null
    /// bytes from the array to produce a valid string slice.
    ///
    /// # Panics
    ///
    /// This function will panic if the input [u8] array is not a valid [`UTF-8`]
    /// sequence. This should never happen because the input is a fixed-size [u8]
    /// array that is guaranteed to contain only [`ASCII`] digits (0-9) and null bytes
    /// (0).
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::stack_alloc_types::u16_impl::{u16_to_u8_array, convert_u16_to_str_slice};
    ///
    /// let num = 123_u16;
    /// let array = u16_to_u8_array(num);
    /// let result = convert_u16_to_str_slice(&array);
    /// assert_eq!(result, "123");
    /// ```
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn convert_u16_to_str_slice(arg: &[u8; U16_FMT_MAX_DIGITS]) -> &str {
        let result_str = std::str::from_utf8(arg);
        debug_assert_matches!(
            result_str,
            Ok(_),
            "Failed to convert u16 u8 array to string slice" // Should never happen!
        );

        #[allow(
            clippy::unwrap_used,
            reason = "Numbers are mathematically guaranteed to be valid UTF-8"
        )]
        result_str.unwrap().trim_start_matches(char::from(0))
    }
}

/// Maximum number of decimal digits needed to represent a [usize] / [u64].
///
/// This is 20 because `u64::MAX` is `18_446_744_073_709_551_615` (20 digits).
/// - Even if [usize] is [u32] on some platforms, we just want to safely format an
///   [`ANSI`] sequence. 20 is needed for [std].
/// - <https://doc.rust-lang.org/std/primitive.u64.html>
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
pub const USIZE_FMT_MAX_DIGITS: usize = 20;

/// Maximum number of decimal digits needed to represent a [u16].
///
/// This is 5 because [`u16::MAX`] is 65535 (5 digits).
/// - <https://doc.rust-lang.org/std/primitive.u16.html>
///
/// # Use Case
///
/// Terminal coordinates are typically [u16] values (max terminal size ~65K rows/cols).
/// Using this smaller array vs [`USIZE_FMT_MAX_DIGITS`] provides:
/// - 75% less stack space (5 bytes vs 20)
/// - Fewer loop iterations (max 5 vs 20)
/// - Better cache locality
/// - ~20% faster conversion
pub const U16_FMT_MAX_DIGITS: usize = 5;

#[cfg(test)]
mod tests {
    use super::{u16_impl::*, usize_impl::*};

    #[test]
    fn test_usize_to_u8_array() {
        let num = 1_234_567_890;
        let result = usize_to_u8_array(num);
        let result_str = convert_usize_to_str_slice(&result);
        assert_eq!(result_str, "1234567890");

        let num = 0;
        let result = usize_to_u8_array(num);
        let result_str = convert_usize_to_str_slice(&result);
        assert_eq!(result_str, "0");

        let num = 12;
        let result = usize_to_u8_array(num);
        let result_str = convert_usize_to_str_slice(&result);
        assert_eq!(result_str, "12");
    }

    #[test]
    fn test_u16_to_u8_array_zero() {
        let num = 0_u16;
        let result = u16_to_u8_array(num);
        let result_str = convert_u16_to_str_slice(&result);
        assert_eq!(result_str, "0");
    }

    #[test]
    fn test_u16_to_u8_array_single_digit() {
        let num = 5_u16;
        let result = u16_to_u8_array(num);
        let result_str = convert_u16_to_str_slice(&result);
        assert_eq!(result_str, "5");
    }

    #[test]
    fn test_u16_to_u8_array_two_digits() {
        let num = 42_u16;
        let result = u16_to_u8_array(num);
        let result_str = convert_u16_to_str_slice(&result);
        assert_eq!(result_str, "42");
    }

    #[test]
    fn test_u16_to_u8_array_three_digits() {
        let num = 123_u16;
        let result = u16_to_u8_array(num);
        let result_str = convert_u16_to_str_slice(&result);
        assert_eq!(result_str, "123");
    }

    #[test]
    fn test_u16_to_u8_array_four_digits() {
        let num = 9999_u16;
        let result = u16_to_u8_array(num);
        let result_str = convert_u16_to_str_slice(&result);
        assert_eq!(result_str, "9999");
    }

    #[test]
    fn test_u16_to_u8_array_max_value() {
        let num = u16::MAX; // 65535
        let result = u16_to_u8_array(num);
        let result_str = convert_u16_to_str_slice(&result);
        assert_eq!(result_str, "65535");
    }

    #[test]
    fn test_u16_terminal_coordinates() {
        // Typical terminal coordinates
        let row = 24_u16;
        let col = 80_u16;

        let row_result = u16_to_u8_array(row);
        let col_result = u16_to_u8_array(col);

        assert_eq!(convert_u16_to_str_slice(&row_result), "24");
        assert_eq!(convert_u16_to_str_slice(&col_result), "80");
    }
}

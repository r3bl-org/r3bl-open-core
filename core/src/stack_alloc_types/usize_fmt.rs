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

//! This module handles conversion of [usize] to a fixed size [u8] array. This is useful
//! when you want to convert a [usize] to a string slice without heap allocation. The
//! maximum number of digits that a [usize] can have is 20.
//! - This is because the maximum value of [usize] is [std::usize::MAX] which is 2^64 - 1.
//! - The number of digits needed to represent this number is 20.
//!
//! # Constants
//!
//! - `USIZE_FMT_MAX_DIGITS`: The maximum number of digits needed to represent a [usize].
//!
//! # Functions
//!
//! - `usize_to_u8_array`: Converts a [usize] to a fixed size [u8] array.
//! - `convert_to_string_slice`: Converts a [u8] array to a string slice by removing leading zeros.
//! - `debug_assert_usize_fits_20_digits`: Sanity check to ensure that the maximum value of [usize] can be represented with 20 decimal digits.
//!
//! # Examples
//!
//! ```rust
//! use r3bl_core::stack_alloc_types::usize_fmt::{usize_to_u8_array, convert_to_string_slice};
//!
//! let num = 1234567890;
//! let result = usize_to_u8_array(num);
//! let result_str = convert_to_string_slice(&result);
//! assert_eq!(result_str, "1234567890");
//!
//! let num = 0;
//! let result = usize_to_u8_array(num);
//! let result_str = convert_to_string_slice(&result);
//! assert_eq!(result_str, "0");
//!
//! let num = 12;
//! let result = usize_to_u8_array(num);
//! let result_str = convert_to_string_slice(&result);
//! assert_eq!(result_str, "12");
//! ```
//!
//! # Panics
//!
//! - `convert_to_string_slice` will panic if the input [u8] array is not a valid UTF-8 sequence.

/// 20 is needed for [std].
/// - <https://doc.rust-lang.org/std/primitive.u64.html>
pub const USIZE_FMT_MAX_DIGITS: usize = 20;

pub fn usize_to_u8_array(mut num: usize) -> [u8; USIZE_FMT_MAX_DIGITS] {
    debug_assert_usize_fits_20_digits();

    let mut result = [0; USIZE_FMT_MAX_DIGITS]; // Initialize with zeros
    let mut index = USIZE_FMT_MAX_DIGITS - 1;

    if num == 0 {
        result[index] = b'0';
        return result;
    }

    while num > 0 && index > 0 {
        let digit = (num % 10) as u8;
        result[index] = b'0' + digit; // Convert digit to ASCII character
        num /= 10;
        index -= 1;
    }

    result
}

/// This function converts a [u8] array to a string slice by removing leading zeros.
/// It also trims the string slice to remove any trailing zeros. The call to
/// `unwrap()` in this function should never panic because the input is a valid [u8]
/// array.
pub fn convert_to_string_slice(arg: &[u8; USIZE_FMT_MAX_DIGITS]) -> &str {
    let result_str = std::str::from_utf8(arg);
    debug_assert!(
        result_str.is_ok(),
        // This should never happen!
        "Failed to convert u8 array to string slice"
    );
    let result_str = result_str.unwrap().trim_start_matches(char::from(0));
    result_str
}

/// This is just a sanity check done in the debug release to makes sure that the
/// maximum value of [usize] can be represented with [USIZE_FMT_MAX_DIGITS] decimal
/// digits.
fn debug_assert_usize_fits_20_digits() {
    // Calculate the maximum value of usize
    let max_usize = usize::MAX;

    // Calculate the number of digits needed
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

#[cfg(test)]
mod tests_usize_fmt {
    use super::*;

    #[test]
    fn test_usize_to_u8_array() {
        let num = 1234567890;
        let result = usize_to_u8_array(num);
        let result_str = convert_to_string_slice(&result);
        assert_eq!(result_str, "1234567890");

        let num = 0;
        let result = usize_to_u8_array(num);
        let result_str = convert_to_string_slice(&result);
        assert_eq!(result_str, "0");

        let num = 12;
        let result = usize_to_u8_array(num);
        let result_str = convert_to_string_slice(&result);
        assert_eq!(result_str, "12");
    }
}

/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

//! This module contains a parser that parses a hex color string into a [`RgbValue`]
//! struct. The hex color string can be in the following format: `#RRGGBB`, eg: `#FF0000`
//! for red.

use std::num::ParseIntError;

use nom::{bytes::complete::{tag, take_while_m_n},
          combinator::map_res,
          IResult, Parser};

use super::RgbValue;

/// Parse function that generate an [`RgbValue`] struct from a valid hex color string.
///
/// # Errors
///
/// Returns an error if:
/// - The input doesn't start with '#'
/// - The hex digits are invalid or not exactly 6 digits (RRGGBB)
/// - The hex values cannot be parsed as valid u8 integers
pub fn parse_hex_color(input: &str) -> IResult<&str, RgbValue> {
    let (input, _) = tag("#")(input)?;
    let (input, (red, green, blue)) =
        (hex_primary::parse, hex_primary::parse, hex_primary::parse).parse(input)?;
    Ok((input, RgbValue { red, green, blue }))
}

mod hex_primary {
    use super::{map_res, take_while_m_n, IResult, ParseIntError, Parser};

    #[allow(clippy::missing_errors_doc)]
    pub fn parse(input: &str) -> IResult<&str, u8> {
        map_res(take_while_m_n(2, 2, is_hex_digit), from_hex).parse(input)
    }

    #[allow(clippy::missing_errors_doc)]
    fn from_hex(input: &str) -> Result<u8, ParseIntError> {
        u8::from_str_radix(input, 16)
    }

    fn is_hex_digit(c: char) -> bool { c.is_ascii_hexdigit() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_color() {
        assert_eq!(
            parse_hex_color("#2F14DF"),
            Ok((
                "",
                RgbValue {
                    red: 47,
                    green: 20,
                    blue: 223,
                }
            ))
        );
    }

    #[test]
    fn parse_valid_color_with_rem() {
        let mut input = String::new();
        input.push_str("#2F14DF");
        input.push('🔅');

        let result = dbg!(parse_hex_color(&input));

        let Ok((remainder, color)) = result else {
            panic!();
        };
        assert_eq!(remainder, "🔅");
        assert_eq!(color, RgbValue::from_u8(47, 20, 223));
    }

    #[test]
    fn parse_invalid_color() {
        let result = dbg!(parse_hex_color("🔅#2F14DF"));
        assert!(result.is_err());
    }
}

// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module contains a parser that parses a hex color string into a [`RgbValue`]
//! struct. The hex color string can be in the following format: `#RRGGBB`, eg: `#FF0000`
//! for red.

use super::RgbValue;
use nom::{IResult, Parser,
          bytes::complete::{tag, take_while_m_n},
          combinator::map_res};
use std::num::ParseIntError;

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
    use super::{IResult, ParseIntError, Parser, map_res, take_while_m_n};

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

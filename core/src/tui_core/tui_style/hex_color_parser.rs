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

//! This module contains a parser that parses a hex color string into a [RgbValue] struct.
//! The hex color string can be in the following format: `#RRGGBB`, eg: `#FF0000` for red.

use std::num::ParseIntError;

use nom::{IResult,
          Parser,
          bytes::complete::{tag, take_while_m_n},
          combinator::map_res,
          error::{FromExternalError, ParseError},
          sequence::tuple};

use crate::RgbValue;

/// Parse function that generate an [RgbValue] struct from a valid hex color string.
pub fn parse_hex_color(input: &str) -> IResult<&str, RgbValue> {
    // This tuple contains 3 ways to do the same thing.
    let it = (
        helper_fns::parse_hex_seg, // This is preferred.
        intermediate_parsers::gen_hex_seg_parser_fn(),
        map_res(
            take_while_m_n(2, 2, helper_fns::match_is_hex_digit),
            helper_fns::parse_str_to_hex_num,
        ),
    );
    let (input, _) = tag("#")(input)?;
    let (input, (red, green, blue)) = tuple(it)(input)?; // same as `it.parse(input)?`
    Ok((input, RgbValue { red, green, blue }))
}

/// Helper functions to match and parse hex digits. These are not [Parser] implementations.
mod helper_fns {
    use super::*;

    /// This function is used by [map_res] and it returns a [Result], not [IResult].
    pub fn parse_str_to_hex_num(input: &str) -> Result<u8, std::num::ParseIntError> {
        u8::from_str_radix(input, 16)
    }

    /// This function is used by [take_while_m_n] and as long as it returns `true` items will be
    /// taken from the input.
    pub fn match_is_hex_digit(c: char) -> bool { c.is_ascii_hexdigit() }

    pub fn parse_hex_seg(input: &str) -> IResult<&str, u8> {
        map_res(
            take_while_m_n(2, 2, match_is_hex_digit),
            parse_str_to_hex_num,
        )(input)
    }
}

/// These are [Parser] implementations that are used by [parse_hex_color].
mod intermediate_parsers {
    use super::*;

    /// Call this to return function that implements the [Parser] trait.
    pub fn gen_hex_seg_parser_fn<'input, E>() -> impl Parser<&'input str, u8, E>
    where
        E: FromExternalError<&'input str, ParseIntError> + ParseError<&'input str>,
    {
        map_res(
            take_while_m_n(2, 2, helper_fns::match_is_hex_digit),
            helper_fns::parse_str_to_hex_num,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_color() {
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

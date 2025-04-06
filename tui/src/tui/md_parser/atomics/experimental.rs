/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

//! All the code in this file is experimental.
//!
//! - It is not used in the main codebase. It is used to test out new ideas and approaches
//!   with `nom` parsing.
//! - When some of these ideas graduate, they get moved to the main codebase.

use nom::{character::complete::anychar,
          combinator::recognize,
          multi::many0,
          IResult,
          Parser};

use crate::md_parser::constants::NEW_LINE_CHAR;

#[allow(dead_code)]
mod common_batch {
    /// For use with [nom::Parser::and_then]. If any of the input characters are in
    /// `denied_chars`, an error is returned.
    pub fn not_denied_chars<'input>(
        denied_chars: &'input [char],
    ) -> impl Fn(char) -> Result<(char, ()), nom::Err<nom::error::Error<&'input str>>>
    {
        |it| match denied_chars.contains(&it) {
            true => Err(nom::Err::Error(nom::error::Error::new(
                "Found denied character",
                nom::error::ErrorKind::Fail,
            ))),
            false => Ok((it, ())),
        }
    }
}

#[allow(dead_code)]
mod exp_batch_1 {
    use common_batch::not_denied_chars;

    use super::*;

    /// Approach 1 (for and_then), using [not_new_line].
    #[rustfmt::skip]
    pub fn parse_opt_eol(input: &str) -> IResult<&str, &str> {
        let (rem, output) = recognize(
            many0(
                anychar.and_then(not_new_line)
            )
        )(input)?;
        Ok((rem, output))
    }

    pub fn not_new_line<'input>(
        input: char,
    ) -> Result<(char, ()), nom::Err<nom::error::Error<&'input str>>> {
        match input == NEW_LINE_CHAR {
            true => Err(nom::Err::Error(nom::error::Error::new(
                "Found newline.",
                nom::error::ErrorKind::Fail,
            ))),
            false => Ok((input, ())),
        }
    }

    /// Approach 2 (for and_then), using [not_denied_chars].
    #[rustfmt::skip]
    pub fn parse_opt_eol_2(input: &str) -> IResult<&str, &str> {
        let (rem, output) = recognize(
            many0(
                anychar.and_then(not_denied_chars(&[NEW_LINE_CHAR]))
            )
        )(input)?;
        Ok((rem, output))
    }

    #[cfg(test)]
    pub mod new_nom_parser_code {
        use r3bl_core::{assert_eq2, bold, rgb_value};

        use super::*;

        #[test]
        fn test_eol_or_not_behavior() {
            println!(
                "\n\n{}",
                format_args!(
                    "{}",
                    bold("parse_opt_eol()")
                        .fg_rgb_color(rgb_value!(lizard_green))
                        .bg_dark_grey()
                )
            );

            // With EOL.
            {
                let input = "@tags: tag1, tag2, tag3\n";
                let (rem, output) = parse_opt_eol(input).unwrap();
                println!("{:8}: {:?}", "input", input);
                println!("{:8}: {:?}", "rem", rem);
                println!("{:8}: {:?}", "output", output);
                assert_eq2!(output, "@tags: tag1, tag2, tag3");
                assert_eq2!(rem, "\n");
            }

            // Without EOL.
            {
                let input = "@tags: tag1, tag2, tag3";
                let (rem, output) = parse_opt_eol(input).unwrap();
                println!("\n{:8}: {:?}", "input", input);
                println!("{:8}: {:?}", "rem", rem);
                println!("{:8}: {:?}", "output", output);
                assert_eq2!(output, "@tags: tag1, tag2, tag3");
                assert_eq2!(rem, "");
            }

            println!(
                "{}",
                format_args!(
                    "\n\n{}",
                    bold("parse_opt_eol_2()")
                        .fg_rgb_color(rgb_value!(lizard_green))
                        .bg_dark_grey()
                )
            );

            // With EOL.
            {
                let input = "@tags: tag1, tag2, tag3\n";
                let (rem, output) = parse_opt_eol_2(input).unwrap();
                println!("{:8}: {:?}", "input", input);
                println!("{:8}: {:?}", "rem", rem);
                println!("{:8}: {:?}", "output", output);
                assert_eq2!(output, "@tags: tag1, tag2, tag3");
                assert_eq2!(rem, "\n");
            }

            // Without EOL.
            {
                let input = "@tags: tag1, tag2, tag3";
                let (rem, output) = parse_opt_eol_2(input).unwrap();
                println!("\n{:8}: {:?}", "input", input);
                println!("{:8}: {:?}", "rem", rem);
                println!("{:8}: {:?}", "output", output);
                assert_eq2!(output, "@tags: tag1, tag2, tag3");
                assert_eq2!(rem, "");
            }
        }
    }
}

#[allow(dead_code)]
mod exp_batch_2 {
    use common_batch::not_denied_chars;

    use super::*;

    /// Take text until an optional EOL character is found, or end of input is reached. If an
    /// EOL character is found:
    /// - The EOL character is not included in the output.
    /// - The EOL character is not consumed, and is part of the remainder.
    /// - The EOL character is defined by any char in `denied_chars` (which is typically
    ///   &[NEW_LINE_CHAR]).
    ///
    /// Here are some examples:
    ///
    /// | input               | rem       |  output           |
    /// | ------------------- | --------- | ----------------- |
    /// | `"Hello, world!\n"` | `"\n"`    | `"Hello, world!"` |
    /// | `"Hello, world!"`   | `""`      | `"Hello, world!"` |
    pub fn take_text_until_eol_or_end<'input>(
        denied_chars: &'input [char],
    ) -> impl FnMut(
        &'input str,
    ) -> Result<
        (&'input str, &'input str),
        nom::Err<nom::error::Error<&'input str>>,
    > {
        recognize(many0(anychar.and_then(not_denied_chars(denied_chars))))
    }

    #[cfg(test)]
    mod test_text_until_opt_eol {
        use r3bl_core::{assert_eq2, bold, rgb_value};

        use super::*;

        #[test]
        fn test_text_until_opt_eol() {
            println!(
                "\n\n{}",
                format_args!(
                    "{}",
                    bold("test_text_until_opt_eol()")
                        .fg_rgb_color(rgb_value!(lizard_green))
                        .bg_dark_grey()
                )
            );

            let denied_chars = &[NEW_LINE_CHAR];

            // With EOL.
            {
                let input = "Hello, world!\n";
                let (rem, output) =
                    take_text_until_eol_or_end(denied_chars)(input).unwrap();
                println!("{:8}: {:?}", "input", input);
                println!("{:8}: {:?}", "rem", rem);
                println!("{:8}: {:?}", "output", output);
                assert_eq2!(output, "Hello, world!");
                assert_eq2!(rem, "\n");
            }

            // Without EOL.
            {
                let input = "Hello, world!";
                let (rem, output) =
                    take_text_until_eol_or_end(denied_chars)(input).unwrap();
                println!("\n{:8}: {:?}", "input", input);
                println!("{:8}: {:?}", "rem", rem);
                println!("{:8}: {:?}", "output", output);
                assert_eq2!(output, "Hello, world!");
                assert_eq2!(rem, "");
            }
        }
    }
}

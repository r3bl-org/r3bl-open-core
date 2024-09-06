/*
 *   Copyright (c) 2024 R3BL LLC
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

//! This is the lowest priority parser called by
//! [parse_inline_fragments_until_eol_or_eoi()].
//!
//! It matches anything that is not a special character. This is used to parse plain text.
//! It works with the specialized parsers in [parse_inline_fragments_until_eol_or_eoi()]
//! such as:
//! - [parse_fragment_starts_with_underscore_err_on_new_line()],
//! - [parse_fragment_starts_with_star_err_on_new_line()],
//! - [parse_fragment_starts_with_backtick_err_on_new_line()], etc.
//!
//! It also works hand in hand with
//! [specialized_parser_delim_matchers::take_starts_with_delim_no_new_line()] which is
//! used by the specialized parsers.
//!
//! To see this in action, set the [DEBUG_MD_PARSER_STDOUT] to true, and run all the tests
//! in [parse_fragments_in_a_line].

use constants::*;
use crossterm::style::Stylize;
use nom::{branch::*,
          bytes::complete::*,
          character::complete::*,
          combinator::*,
          multi::*,
          sequence::*,
          IResult};
use r3bl_rs_utils_core::call_if_true;

use crate::*;

// BOOKM: Lowest priority parser for "plain text" Markdown fragment

/// This is the lowest priority parser called by
/// [parse_inline_fragments_until_eol_or_eoi()], which itself is called:
/// 1. Repeatedly in a loop by [parse_block_markdown_text_with_or_without_new_line()].
/// 2. And by [parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line()].
///
/// It will match anything that is not a special character. This is used to parse plain
/// text.
///
/// However, when it encounters a special character, it will break the input at that
/// character and split the input into two parts, and return them:
/// 1. The plain text.
/// 2. And the remainder (after the special character).
///
/// This gives the other more specialized parsers a chance to address these special
/// characters (like italic, bold, links, etc.), when this function is called repeatedly:
/// - By [parse_block_markdown_text_with_or_without_new_line()],
/// - Which repeatedly calls [parse_inline_fragments_until_eol_or_eoi()]. This function
///   actually runs the specialized parsers.
/// - Which calls this function repeatedly (if the specialized parsers don't match & error
///   out). This serves as a "catch all" parser.
///
/// If these more specialized parsers error out, then this parser will be called again to
/// parse the remainder; this time the input will start with the special character; and in
/// this edge case it will take the input until the first new line; or until the end of
/// the input.
///
/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
/// See: [specialized_parser_delim_matchers::take_starts_with_delim_no_new_line()].
pub fn parse_fragment_plain_text_no_new_line(input: &str) -> IResult<&str, &str> {
    call_if_true!(DEBUG_MD_PARSER_STDOUT, {
        println!("\n{} plain parser, input: {:?}", "██".magenta(), input);
    });

    if check_input_starts_with(input, &get_sp_char_set_2()).is_none() {
        // # Normal case:
        // If the input does not start with any of the special characters above,
        // take till the first special character. And split the input there. This returns the
        // chunk until the first special character as [MdLineFragment::Plain], and the
        // remainder of the input gets a chance to be parsed by the specialized parsers. If
        // they fail, then this function will be called again to parse the remainder, and the
        // special case above will be triggered.

        // `tag_tuple` replaces the following:
        // `( tag(UNDERSCORE), tag(STAR), tag(BACK_TICK), tag(LEFT_IMAGE), tag(LEFT_BRACKET), tag(NEW_LINE) )`
        let tag_vec = get_sp_char_set_3()
            .into_iter()
            .map(tag::<&str, &str, nom::error::Error<&str>>)
            .collect::<Vec<_>>();
        let tag_tuple = {
            assert_eq!(tag_vec.len(), 6);
            tuple6(&tag_vec)
        };

        let it = recognize(
            /* match anychar up until any of the denied strings below is encountered */
            many1(/* match at least 1 char */ preceded(
                /* match anything that isn't in the denied strings list below */
                /* prefix is discarded, it doesn't match anything, only errors out for denied strings */
                not(
                    /* error out if starts w/ denied strings below */
                    alt(tag_tuple),
                ),
                /* output - keep char if it didn't error out above */
                anychar,
            )),
        )(input);
        call_if_true!(DEBUG_MD_PARSER_STDOUT, {
            println!(
                "{} normal case :: {:?}",
                if it.is_err() {
                    "⬢⬢".red()
                } else {
                    "▲▲".blue()
                },
                it
            );
        });
        return it;
    }

    // # Edge case (catch all):
    // If the input starts with any of these special characters, take till the first new
    // line. Since the specialized parsers did not match the input.

    // # Edge case -> Special case:
    // Check for single UNDERSCORE, STAR, BACK_TICK. until the first new line. This is
    // to handle the case with
    // [specialized_parser_delim_matchers::take_starts_with_delim_no_new_line()] where
    // there is no closing delim found.
    if let Some(special_str) = check_input_starts_with(input, &get_sp_char_set_1()) {
        let (count, _, _, _) =
            specialized_parser_delim_matchers::count_delim_occurrences_until_eol(
                input,
                special_str,
            );
        if count == 1 {
            // Split the input, by returning the first part as plain text, and the
            // remainder as the input to be parsed by the specialized parsers.
            let (rem, output) = recognize(many1(tag(special_str)))(input)?;
            call_if_true!(DEBUG_MD_PARSER_STDOUT, {
                println!(
                    "{} edge case -> special case :: rem: {:?}, output: {:?}",
                    "▲▲".blue(),
                    rem,
                    output
                );
            });
            return Ok((rem, output));
        }
        // Otherwise, fall back to the normal case below.
    }

    // # Edge case -> Normal case:
    // Take till the first new line, as [MdLineFragment::Plain], since the specialized
    // parsers did not match the input.
    let it = take_till1(|it: char| it == NEW_LINE_CHAR)(input);
    call_if_true!(DEBUG_MD_PARSER_STDOUT, {
        println!(
            "{} edge case -> normal case :: {:?}",
            if it.is_err() {
                "⬢⬢".red()
            } else {
                "▲▲".blue()
            },
            it
        );
    });
    it
}

/// This is a special set of chars called `set_1`.
///
/// This is used to check if the input starts with any of these special characters, which
/// must have at least 2 occurrences for it to be parsed by the specialized parsers. If
/// only 1 occurrence is found, then this parser's `Edge case -> Special case` will take
/// care of it by splitting the input, and returning the first part as plain text, and the
/// remainder as the input to be parsed by the specialized parsers.
pub fn get_sp_char_set_1<'a>() -> [&'a str; 3] { [UNDERSCORE, STAR, BACK_TICK] }

/// This is a special set of chars called `set_2`.
///
/// This is used to detect the `Edge case -> Normal case` where the input starts with any
/// of these special characters, and the input is taken until the first new line, and
/// return as plain text. Unless both of the following are true:
/// 1. input is in [get_sp_char_set_1()] and,
/// 2. count is 1.
pub fn get_sp_char_set_2<'a>() -> [&'a str; 5] {
    get_sp_char_set_1()
        .iter()
        .chain([LEFT_IMAGE, LEFT_BRACKET].iter())
        .copied()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

/// This is a special set of chars called `set_3`.
///
/// This is used to detect the `Normal case` where the input does not start with any of
/// the special characters in [get_sp_char_set_2()]. The input is taken until the first
/// special character, and split there. This returns the chunk until the first special
/// character as [MdLineFragment::Plain], and the remainder of the input gets a chance to
/// be parsed by the specialized parsers.
pub fn get_sp_char_set_3<'a>() -> [&'a str; 6] {
    get_sp_char_set_2()
        .iter()
        .chain([NEW_LINE].iter())
        .copied()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

pub fn check_input_starts_with<'a>(
    input: &'a str,
    char_set: &[&'a str],
) -> Option<&'a str> {
    char_set
        .iter()
        .find(|&special_str| input.starts_with(special_str))
        .copied()
}

pub fn tuple5<T>(a: &[T]) -> (&T, &T, &T, &T, &T) { (&a[0], &a[1], &a[2], &a[3], &a[4]) }
pub fn tuple6<T>(a: &[T]) -> (&T, &T, &T, &T, &T, &T) {
    (&a[0], &a[1], &a[2], &a[3], &a[4], &a[5])
}

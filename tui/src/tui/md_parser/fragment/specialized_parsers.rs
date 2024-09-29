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

use crossterm::style::Stylize;
use nom::{branch::alt,
          bytes::complete::tag,
          combinator::{map, recognize},
          multi::many0,
          IResult};
use r3bl_core::call_if_true;

use super::specialized_parser_delim_matchers;
use crate::{constants::{BACK_TICK,
                        CHECKED,
                        LEFT_BRACKET,
                        LEFT_IMAGE,
                        LEFT_PARENTHESIS,
                        RIGHT_BRACKET,
                        RIGHT_IMAGE,
                        RIGHT_PARENTHESIS,
                        STAR,
                        UNCHECKED,
                        UNDERSCORE},
            take_text_between_delims_err_on_new_line,
            HyperlinkData,
            DEBUG_MD_PARSER_STDOUT};

pub fn parse_fragment_starts_with_underscore_err_on_new_line(
    input: &str,
) -> IResult<&str, &str> {
    specialized_parser_delim_matchers::take_starts_with_delim_no_new_line(
        input, UNDERSCORE,
    )
}

pub fn parse_fragment_starts_with_star_err_on_new_line(
    input: &str,
) -> IResult<&str, &str> {
    specialized_parser_delim_matchers::take_starts_with_delim_no_new_line(input, STAR)
}

pub fn parse_fragment_starts_with_backtick_err_on_new_line(
    input: &str,
) -> IResult<&str, &str> {
    // Count the number of consecutive backticks. If there are more than 2 backticks,
    // return an error, since this could be a code block.
    let it = recognize(many0(tag(BACK_TICK)))(input);
    if it.is_err() {
        call_if_true!(DEBUG_MD_PARSER_STDOUT, {
            println!(
                "\n{} specialized parser error out with backtick: \ninput: {:?}, delim: {:?}",
                "⬢⬢".red(),
                input,
                BACK_TICK
            );
        });
    }
    let (_, output) = it?;
    if output.len() > 2 {
        call_if_true!(DEBUG_MD_PARSER_STDOUT, {
            println!("{} more than 2 backticks in input:{:?}", "⬢⬢".red(), input);
        });
        return Err(nom::Err::Error(nom::error::Error {
            input: output,
            code: nom::error::ErrorKind::Tag,
        }));
    }

    // Otherwise, return the text between the backticks.
    specialized_parser_delim_matchers::take_starts_with_delim_no_new_line(
        input, BACK_TICK,
    )
}

pub fn parse_fragment_starts_with_left_image_err_on_new_line(
    input: &str,
) -> IResult<&str, HyperlinkData<'_>> {
    // Parse the text between the image tags.
    let result_first =
        take_text_between_delims_err_on_new_line(input, LEFT_IMAGE, RIGHT_IMAGE);
    if result_first.is_err() {
        call_if_true!(DEBUG_MD_PARSER_STDOUT, {
            println!(
                    "\n{} specialized parser error out with image: \ninput: {:?}, delim: {:?}",
                    "⬢⬢".red(),
                    input,
                    LEFT_IMAGE
                );
        });
    }
    let (rem, part_between_image_tags) = result_first?;

    // Parse the text between the parenthesis.
    let result_second = take_text_between_delims_err_on_new_line(
        rem,
        LEFT_PARENTHESIS,
        RIGHT_PARENTHESIS,
    );
    if result_second.is_err() {
        call_if_true!(DEBUG_MD_PARSER_STDOUT, {
            println!(
                    "\n{} specialized parser error out with image: \ninput: {:?}, delim: {:?}",
                    "⬢⬢".red(),
                    rem,
                    LEFT_PARENTHESIS
                );
        });
    }
    let (rem, part_between_parenthesis) = result_second?;

    let it = Ok((
        rem,
        HyperlinkData::from((part_between_image_tags, part_between_parenthesis)),
    ));
    call_if_true!(DEBUG_MD_PARSER_STDOUT, {
        println!(
            "{} specialized parser for image: {:?}",
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

pub fn parse_fragment_starts_with_left_link_err_on_new_line(
    input: &str,
) -> IResult<&str, HyperlinkData<'_>> {
    // Parse the text between the brackets.
    let result_first =
        take_text_between_delims_err_on_new_line(input, LEFT_BRACKET, RIGHT_BRACKET);
    if result_first.is_err() {
        call_if_true!(DEBUG_MD_PARSER_STDOUT, {
            println!(
                "\n{} specialized parser error out with link: \ninput: {:?}, delim: {:?}",
                "⬢⬢".red(),
                input,
                LEFT_BRACKET
            );
        });
    }
    let (rem, part_between_brackets) = result_first?;

    // Parse the text between the parenthesis.
    let result_second = take_text_between_delims_err_on_new_line(
        rem,
        LEFT_PARENTHESIS,
        RIGHT_PARENTHESIS,
    );
    if result_second.is_err() {
        call_if_true!(DEBUG_MD_PARSER_STDOUT, {
            println!(
                "\n{} specialized parser error out with link: \ninput: {:?}, delim: {:?}",
                "⬢⬢".red(),
                rem,
                LEFT_PARENTHESIS
            );
        });
    }
    let (rem, part_between_parenthesis) = result_second?;

    let it = Ok((
        rem,
        HyperlinkData::from((part_between_brackets, part_between_parenthesis)),
    ));
    call_if_true!(DEBUG_MD_PARSER_STDOUT, {
        println!(
            "{} specialized parser for link: {:?}",
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

/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and
/// images.
///
/// So some extra hint is need from the code calling this parser to let it know whether to
/// parse a checkbox into plain text, or into a boolean.
pub fn parse_fragment_starts_with_checkbox_into_str(input: &str) -> IResult<&str, &str> {
    let it = alt((recognize(tag(CHECKED)), recognize(tag(UNCHECKED))))(input);
    call_if_true!(DEBUG_MD_PARSER_STDOUT, {
        println!(
            "{} specialized parser for checkbox: {:?}",
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

/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and
/// images.
///
/// So some extra hint is need from the code calling this parser to let it know whether to
/// parse a checkbox into plain text, or into a boolean.
pub fn parse_fragment_starts_with_checkbox_checkbox_into_bool(
    input: &str,
) -> IResult<&str, bool> {
    let it = alt((map(tag(CHECKED), |_| true), map(tag(UNCHECKED), |_| false)))(input);
    call_if_true!(DEBUG_MD_PARSER_STDOUT, {
        println!(
            "{} specialized parser for checkbox: {:?}",
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

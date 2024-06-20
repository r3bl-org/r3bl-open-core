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

use constants::*;
use nom::{branch::*, bytes::complete::*, combinator::*, multi::*, IResult};

use crate::*;

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
    let (_, output) = recognize(many0(tag(BACK_TICK)))(input)?;
    if output.len() > 2 {
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

#[rustfmt::skip]
pub fn parse_fragment_starts_with_left_image_err_on_new_line(input: &str) -> IResult<&str, HyperlinkData<'_>> {
    let (rem, part_between_image_tags) = take_text_between_delims_err_on_new_line(
        input, LEFT_IMAGE, RIGHT_IMAGE)?;
    let (rem, part_between_parenthesis) = take_text_between_delims_err_on_new_line(
        rem, LEFT_PARENTHESIS, RIGHT_PARENTHESIS)?;
    Ok((rem, HyperlinkData::from((part_between_image_tags, part_between_parenthesis))))
}

#[rustfmt::skip]
pub fn parse_fragment_starts_with_left_link_err_on_new_line(
    input: &str,
) -> IResult<&str, HyperlinkData<'_>> {
    let (rem, part_between_brackets) = take_text_between_delims_err_on_new_line(
        input, LEFT_BRACKET, RIGHT_BRACKET)?;
    let (rem, part_between_parenthesis) = take_text_between_delims_err_on_new_line(
        rem, LEFT_PARENTHESIS, RIGHT_PARENTHESIS)?;
    Ok((rem, HyperlinkData::from((part_between_brackets, part_between_parenthesis))))
}

/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and images.
/// So some extra hint is need from the code calling this parser to let it know whether to parse
/// a checkbox into plain text, or into a boolean.
#[rustfmt::skip]
pub fn parse_fragment_starts_with_checkbox_into_str(input: &str) -> IResult<&str, &str> {
    alt((
        recognize(tag(CHECKED)),
        recognize(tag(UNCHECKED))
    ))(input)
}

#[rustfmt::skip]
/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and images.
/// So some extra hint is need from the code calling this parser to let it know whether to parse
/// a checkbox into plain text, or into a boolean.
pub fn parse_fragment_starts_with_checkbox_checkbox_into_bool(input: &str) -> IResult<&str, bool> {
    alt((
        map(tag(CHECKED), |_| true),
        map(tag(UNCHECKED), |_| false),
    ))(input)
}

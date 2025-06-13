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
use nom::{bytes::complete::tag, combinator::opt, sequence::preceded, IResult, Parser};

use crate::{inline_string,
            md_parser::constants::{COLON, NEW_LINE, SPACE},
            parser_take_text_until_eol_or_eoi_alt,
            AsStrSlice};

/// - Sample parse input: `@title: Something` or `@date: Else`.
/// - There may or may not be a newline at the end. If there is, it is consumed.
/// - Can't nest the `tag_name` within the `output`. So there can only be one `tag_name`
///   in the `output`.
pub fn parse_unique_kv_opt_eol_alt<'a>(
    tag_name: &'a str,
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, Option<AsStrSlice<'a>>> {
    let input_clone = input.clone();

    let (remainder, title_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ parser_take_text_until_eol_or_eoi_alt(),
    )
    .parse(input)?;

    // Can't nest `tag_name` in `output`. Early return in this case.
    // Check if the tag pattern appears in the parsed content or remainder.
    let sub_str = inline_string!("{tag_name}{COLON}{SPACE}");
    if title_text.contains(sub_str.as_str()) | remainder.contains(sub_str.as_str()) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input_clone, // "Can't have more than one tag_name in kv expr.",
            nom::error::ErrorKind::Fail,
        )));
    }

    // If there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE)).parse(remainder)?;

    // Special case: Early return when something like `@title: ` or `@title: \n` is found.
    if title_text.is_empty() {
        Ok((remainder, None))
    }
    // Normal case.
    else {
        Ok((remainder, Some(title_text)))
    }
}

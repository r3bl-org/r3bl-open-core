/*
 *   Copyright (c) 2023 R3BL LLC
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
use nom::{bytes::complete::*, multi::*, sequence::*, IResult};

use crate::{md_parser::parse_element::parse_element_markdown_inline, *};

/// Parse a single line of markdown text [FragmentsInOneLine] terminated by EOL.
#[rustfmt::skip]
pub fn parse_block_markdown_text_until_eol(input: &str) -> IResult<&str, MdLineFragments> {
    parse_until_eol(input)
}

#[rustfmt::skip]
fn parse_until_eol(input: &str) -> IResult<&str, MdLineFragments> {
    let (input, output) = terminated(
        /* output */
        many0(
            |it| parse_element_markdown_inline(it, CheckboxParsePolicy::IgnoreCheckbox)
        ),
        /* ends with (discarded) */ tag(NEW_LINE),
    )(input)?;
    let it = List::from(output);
    Ok((input, it))
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CheckboxParsePolicy {
    IgnoreCheckbox,
    ParseCheckbox,
}

/// Parse a markdown text [FragmentsInOneLine] in the input (no EOL required).
#[rustfmt::skip]
pub fn parse_block_markdown_text_opt_eol_with_checkbox_policy(
    input: &str,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<&str, MdLineFragments> {
    parse_opt_eol(input, checkbox_policy)
}

/// Parse a markdown text [FragmentsInOneLine] in the input (no EOL required).
#[rustfmt::skip]
pub fn parse_block_markdown_text_opt_eol(
    input: &str,
) -> IResult<&str, MdLineFragments> {
    parse_opt_eol(input, CheckboxParsePolicy::IgnoreCheckbox)
}

#[rustfmt::skip]
fn parse_opt_eol(
    input: &str,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<&str, MdLineFragments> {
    let (input, output) = many0(
        |it| parse_element_markdown_inline(it, checkbox_policy)
    )(input)?;
    let it = List::from(output);
    Ok((input, it))
}

#[cfg(test)]
mod test {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_parse_block_markdown_text_no_eol() {
        assert_eq2!(parse_block_markdown_text_opt_eol(""), Ok(("", list![])));

        assert_eq2!(
            parse_block_markdown_text_opt_eol(
                "here is some plaintext *but what if we italicize?"
            ),
            Ok((
                "*but what if we italicize?",
                list![MdLineFragment::Plain("here is some plaintext "),]
            ))
        );
    }

    #[test]
    fn test_parse_block_markdown_text_with_eol() {
        assert_eq2!(parse_block_markdown_text_until_eol("\n"), Ok(("", list![])));
        assert_eq2!(
            parse_block_markdown_text_until_eol("here is some plaintext\n"),
            Ok(("", list![MdLineFragment::Plain("here is some plaintext")]))
        );
        assert_eq2!(
            parse_block_markdown_text_until_eol(
                "here is some plaintext *but what if we bold?*\n"
            ),
            Ok((
                "",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Bold("but what if we bold?"),
                ]
            ))
        );
        assert_eq2!(
            parse_block_markdown_text_until_eol("here is some plaintext *but what if we bold?* I guess it doesn't **matter** in my `code`\n"),
            Ok(
                ("",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Bold("but what if we bold?"),
                    MdLineFragment::Plain(" I guess it doesn't "),
                    MdLineFragment::Bold(""),
                    MdLineFragment::Plain("matter"),
                    MdLineFragment::Bold(""),
                    MdLineFragment::Plain(" in my "),
                    MdLineFragment::InlineCode("code"),
                ])
            )
        );
        assert_eq2!(
            parse_block_markdown_text_until_eol(
                "here is some plaintext _but what if we italic?_\n"
            ),
            Ok((
                "",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Italic("but what if we italic?"),
                ]
            ))
        );
    }
}

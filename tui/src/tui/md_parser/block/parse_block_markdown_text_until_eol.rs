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
    let (input, output) =
        terminated(
            /* output */ many0(parse_element_markdown_inline),
            /* ends with (discarded) */ tag(NEW_LINE),
        )
    (input)?;
    let it = List::from(output);
    Ok((input, it))
}

/// Parse a markdown text [FragmentsInOneLine] in the input (no EOL required).
#[rustfmt::skip]
pub fn parse_block_markdown_text_opt_eol(input: &str) -> IResult<&str, MdLineFragments> {
    parse_opt_eol(input)
}

#[rustfmt::skip]
fn parse_opt_eol(input: &str) -> IResult<&str, MdLineFragments> {
    let (input, output) =
        many0(parse_element_markdown_inline)
    (input)?;
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
            parse_block_markdown_text_opt_eol("here is some plaintext *but what if we italicize?"),
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
                "here is some plaintext *but what if we italicize?*\n"
            ),
            Ok((
                "",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Italic("but what if we italicize?"),
                ]
            ))
        );
        assert_eq2!(
            parse_block_markdown_text_until_eol("here is some plaintext *but what if we italicize?* I guess it doesn't **matter** in my `code`\n"),
            Ok(
                ("",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Italic("but what if we italicize?"),
                    MdLineFragment::Plain(" I guess it doesn't "),
                    MdLineFragment::Bold("matter"),
                    MdLineFragment::Plain(" in my "),
                    MdLineFragment::InlineCode("code"),
                ])
            )
        );
        assert_eq2!(
            parse_block_markdown_text_until_eol(
                "here is some plaintext *but what if we italicize?*\n"
            ),
            Ok((
                "",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Italic("but what if we italicize?"),
                ]
            ))
        );
    }
}

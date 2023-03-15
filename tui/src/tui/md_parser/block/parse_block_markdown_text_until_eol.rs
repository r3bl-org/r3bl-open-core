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

use crate::{md_parser::parser_element::parse_element_markdown_inline, *};

/// Parse a single line of markdown text [FragmentsInOneLine].
#[rustfmt::skip]
pub fn parse_block_markdown_text_until_eol(input: &str) -> IResult<&str, Fragments> {
    parse(input)
}

#[rustfmt::skip]
fn parse(input: &str) -> IResult<&str, Fragments> {
    terminated(
        /* output */ many0(parse_element_markdown_inline),
        /* ends with (discarded) */ tag(NEW_LINE),
    )(input)
}

#[cfg(test)]
mod test {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

    use super::*;

    #[test]
    fn test_parse_block_markdown_text() {
        assert_eq!(parse_block_markdown_text_until_eol("\n"), Ok(("", vec![])));
        assert_eq!(
            parse_block_markdown_text_until_eol("here is some plaintext\n"),
            Ok(("", vec![Fragment::Plain("here is some plaintext")]))
        );
        assert_eq!(
            parse_block_markdown_text_until_eol(
                "here is some plaintext *but what if we italicize?*\n"
            ),
            Ok((
                "",
                vec![
                    Fragment::Plain("here is some plaintext "),
                    Fragment::Italic("but what if we italicize?"),
                ]
            ))
        );
        assert_eq!(
        parse_block_markdown_text_until_eol("here is some plaintext *but what if we italicize?* I guess it doesn't **matter** in my `code`\n"),
        Ok(
            ("",
            vec![
                Fragment::Plain("here is some plaintext "),
                Fragment::Italic("but what if we italicize?"),
                Fragment::Plain(" I guess it doesn't "),
                Fragment::Bold("matter"),
                Fragment::Plain(" in my "),
                Fragment::InlineCode("code"),
            ])
        )
    );
        assert_eq!(
            parse_block_markdown_text_until_eol(
                "here is some plaintext *but what if we italicize?*\n"
            ),
            Ok((
                "",
                vec![
                    Fragment::Plain("here is some plaintext "),
                    Fragment::Italic("but what if we italicize?"),
                ]
            ))
        );
        assert_eq!(
            parse_block_markdown_text_until_eol(
                "here is some plaintext *but what if we italicize?"
            ),
            Err(NomErr::Error(Error {
                input: "*but what if we italicize?",
                code: ErrorKind::Tag
            })) // Ok(("*but what if we italicize?", vec![MarkdownInline::Plaintext(String::from("here is some plaintext "))]))
        );
    }
}

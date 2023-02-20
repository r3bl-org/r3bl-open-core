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

// This module exists so that rustfmt can skip the formatting of the parser code.
#[rustfmt::skip]
pub mod no_rustfmt_block {
    use crate::*;
    use constants::*;
    use nom::{
        branch::*, bytes::complete::*, character::complete::*, combinator::*, multi::*,
        sequence::*, IResult,
    };

    pub fn parse_element_bold_italic(input: &str) -> IResult<&str, &str> {
        alt((
            delimited(/* start */ tag(BITALIC_1), /* output */ is_not(BITALIC_1), /* end */ tag(BITALIC_1)),
            delimited(/* start */ tag(BITALIC_2), /* output */ is_not(BITALIC_2), /* end */ tag(BITALIC_2)),
        ))(input)
    }

    pub fn parse_element_bold(input: &str) -> IResult<&str, &str> {
        alt((
            delimited(/* start */ tag(BOLD_1), /* output */ is_not(BOLD_1), /* end */ tag(BOLD_1)),
            delimited(/* start */ tag(BOLD_2), /* output */ is_not(BOLD_2), /* end */ tag(BOLD_2)),
        ))(input)
    }

    pub fn parse_element_italic(input: &str) -> IResult<&str, &str> {
        alt((
            delimited(/* start */ tag(ITALIC_1), /* output */ is_not(ITALIC_1), /* end */ tag(ITALIC_1)),
            delimited(/* start */ tag(ITALIC_2), /* output */ is_not(ITALIC_2), /* end */ tag(ITALIC_2)),
        ))(input)
    }

    pub fn parse_element_code(input: &str) -> IResult<&str, &str> {
        delimited(/* start */ tag(BACKTICK), /* output */ is_not(BACKTICK), /* end */ tag(BACKTICK))(input)
    }

    pub fn parse_element_link(input: &str) -> IResult<&str, (&str, &str)> {
        pair(
            delimited(/* start */ tag(LEFT_BRACKET), /* output */ is_not(RIGHT_BRACKET), /* end */ tag(RIGHT_BRACKET)),
            delimited(/* start */ tag(LEFT_PAREN), /* output */ is_not(RIGHT_PAREN), /* end */ tag(RIGHT_PAREN)),
        )(input)
    }

    pub fn parse_element_image(input: &str) -> IResult<&str, (&str, &str)> {
        pair(
            delimited(/* start */ tag(LEFT_IMG), /* output */ is_not(RIGHT_IMG), /* end */ tag(RIGHT_IMG)),
            delimited(/* start */ tag(LEFT_PAREN), /* output */ is_not(RIGHT_PAREN), /* end */ tag(RIGHT_PAREN)),
        )(input)
    }

    /// There must be at least one match. We want to match many things that are not any of our
    /// special tags, but since we have no tools available to match and consume in the negative case
    /// (without regex) we need to match against our (start) tags, then consume one char; we repeat
    /// this until we run into one of our special characters (start tags) then we return this slice.
    pub fn parse_element_plaintext(input: &str) -> IResult<&str, &str> {
        recognize(
            many1(
                preceded(
                    /* prefix - discarded */
                    not(
                        /* starts with special characters */
                        alt((
                            tag(BITALIC_1),
                            tag(BITALIC_2),
                            tag(BOLD_1),
                            tag(BOLD_2),
                            tag(ITALIC_1),
                            tag(ITALIC_2),
                            tag(BACKTICK),
                            tag(LEFT_BRACKET),
                            tag(LEFT_IMG),
                            tag(NEW_LINE),
                        ))
                    ),
                    /* output - keep char */
                    anychar,
                )
            )
        )(input)
    }

    /// Parse a single chunk of markdown text [Fragment] in a single line.
    pub fn parse_element_markdown_inline(input: &str) -> IResult<&str, Fragment> {
        alt((
            map(parse_element_italic,       Fragment::Italic),
            map(parse_element_bold,         Fragment::Bold),
            map(parse_element_bold_italic,  Fragment::BoldItalic),
            map(parse_element_code,         Fragment::InlineCode),
            map(parse_element_image,        Fragment::Image),
            map(parse_element_link,         Fragment::Link),
            map(parse_element_plaintext,    Fragment::Plain),
        ))(input)
    }
}
pub use no_rustfmt_block::*;

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

    use super::*;
    use crate::*;

    #[test]
    fn test_parse_element_italic() {
        assert_eq!(
            parse_element_italic("*here is italic*"),
            Ok(("", "here is italic"))
        );

        assert_eq!(
            parse_element_italic("_here is italic_"),
            Ok(("", "here is italic"))
        );

        assert_eq!(
            parse_element_italic("*here is italic"),
            Err(NomErr::Error(Error {
                input: "*here is italic",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_italic("here is italic*"),
            Err(NomErr::Error(Error {
                input: "here is italic*",
                code: ErrorKind::Tag,
            }))
        );

        assert_eq!(
            parse_element_italic("here is italic"),
            Err(NomErr::Error(Error {
                input: "here is italic",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_italic("*"),
            Err(NomErr::Error(Error {
                input: "*",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_italic("**"),
            Err(NomErr::Error(Error {
                input: "**",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_italic(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_italic("**we are doing bold**"),
            Err(NomErr::Error(Error {
                input: "**we are doing bold**",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_bold_italic() {
        assert_eq!(
            parse_element_bold_italic("***here is bitalic***"),
            Ok(("", "here is bitalic"))
        );

        assert_eq!(
            parse_element_bold("***here is bitalic"),
            Err(NomErr::Error(Error {
                input: "***here is bitalic",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold("here is bitalic***"),
            Err(NomErr::Error(Error {
                input: "here is bitalic***",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold_italic("___here is bitalic___"),
            Ok(("", "here is bitalic"))
        );

        assert_eq!(
            parse_element_bold_italic("___here is bitalic"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold_italic("here is bitalic___"),
            Err(NomErr::Error(Error {
                input: "here is bitalic___",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_bold() {
        assert_eq!(
            parse_element_bold("**here is bold**"),
            Ok(("", "here is bold"))
        );

        assert_eq!(
            parse_element_bold("__here is bold__"),
            Ok(("", "here is bold"))
        );

        assert_eq!(
            parse_element_bold("**here is bold"),
            Err(NomErr::Error(Error {
                input: "**here is bold",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold("here is bold**"),
            Err(NomErr::Error(Error {
                input: "here is bold**",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold("here is bold"),
            Err(NomErr::Error(Error {
                input: "here is bold",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold("****"),
            Err(NomErr::Error(Error {
                input: "****",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold("**"),
            Err(NomErr::Error(Error {
                input: "**",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold("*"),
            Err(NomErr::Error(Error {
                input: "*",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );

        assert_eq!(
            parse_element_bold("*this is italic*"),
            Err(NomErr::Error(Error {
                input: "*this is italic*",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_code() {
        assert_eq!(
            parse_element_code("`here is code"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_element_code("here is code`"),
            Err(NomErr::Error(Error {
                input: "here is code`",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_element_code("``"),
            Err(NomErr::Error(Error {
                input: "`",
                code: ErrorKind::IsNot
            }))
        );
        assert_eq!(
            parse_element_code("`"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::IsNot
            }))
        );
        assert_eq!(
            parse_element_code(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_link() {
        assert_eq!(
            parse_element_link("[title](https://www.example.com)"),
            Ok(("", ("title", "https://www.example.com")))
        );
        assert_eq!(
            parse_element_code(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_image() {
        assert_eq!(
            parse_element_image("![alt text](image.jpg)"),
            Ok(("", ("alt text", "image.jpg")))
        );
        assert_eq!(
            parse_element_code(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_plaintext() {
        assert_eq!(
            parse_element_plaintext("1234567890"),
            Ok(("", "1234567890"))
        );
        assert_eq!(
            parse_element_plaintext("oh my gosh!"),
            Ok(("", "oh my gosh!"))
        );
        assert_eq!(
            parse_element_plaintext("oh my gosh!["),
            Ok(("![", "oh my gosh"))
        );
        assert_eq!(
            parse_element_plaintext("oh my gosh!*"),
            Ok(("*", "oh my gosh!"))
        );
        assert_eq!(
            parse_element_plaintext("*bold baby bold*"),
            Err(NomErr::Error(Error {
                input: "*bold baby bold*",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("[link baby](and then somewhat)"),
            Err(NomErr::Error(Error {
                input: "[link baby](and then somewhat)",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("`codeblock for bums`"),
            Err(NomErr::Error(Error {
                input: "`codeblock for bums`",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("![ but wait theres more](jk)"),
            Err(NomErr::Error(Error {
                input: "![ but wait theres more](jk)",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("here is plaintext"),
            Ok(("", "here is plaintext"))
        );
        assert_eq!(
            parse_element_plaintext("here is plaintext!"),
            Ok(("", "here is plaintext!"))
        );
        assert_eq!(
            parse_element_plaintext("here is plaintext![image starting"),
            Ok(("![image starting", "here is plaintext"))
        );
        assert_eq!(
            parse_element_plaintext("here is plaintext\n"),
            Ok(("\n", "here is plaintext"))
        );
        assert_eq!(
            parse_element_plaintext("*here is italic*"),
            Err(NomErr::Error(Error {
                input: "*here is italic*",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("**here is bold**"),
            Err(NomErr::Error(Error {
                input: "**here is bold**",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("`here is code`"),
            Err(NomErr::Error(Error {
                input: "`here is code`",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("[title](https://www.example.com)"),
            Err(NomErr::Error(Error {
                input: "[title](https://www.example.com)",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext("![alt text](image.jpg)"),
            Err(NomErr::Error(Error {
                input: "![alt text](image.jpg)",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_plaintext(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        );
    }

    #[test]
    fn test_parse_element_markdown_inline() {
        assert_eq!(
            parse_element_markdown_inline("*here is italic*"),
            Ok(("", Fragment::Italic("here is italic")))
        );
        assert_eq!(
            parse_element_markdown_inline("**here is bold**"),
            Ok(("", Fragment::Bold("here is bold")))
        );
        assert_eq!(
            parse_element_markdown_inline("`here is code`"),
            Ok(("", Fragment::InlineCode("here is code")))
        );
        assert_eq!(
            parse_element_markdown_inline("[title](https://www.example.com)"),
            Ok(("", (Fragment::Link(("title", "https://www.example.com")))))
        );
        assert_eq!(
            parse_element_markdown_inline("![alt text](image.jpg)"),
            Ok(("", (Fragment::Image(("alt text", "image.jpg")))))
        );
        assert_eq!(
            parse_element_markdown_inline("here is plaintext!"),
            Ok(("", Fragment::Plain("here is plaintext!")))
        );
        assert_eq!(
            parse_element_markdown_inline("here is some plaintext *but what if we italicize?"),
            Ok((
                "*but what if we italicize?",
                Fragment::Plain("here is some plaintext ")
            ))
        );
        assert_eq!(
            parse_element_markdown_inline("here is some plaintext \n*but what if we italicize?"),
            Ok((
                "\n*but what if we italicize?",
                Fragment::Plain("here is some plaintext ")
            ))
        );
        assert_eq!(
            parse_element_markdown_inline("\n"),
            Err(NomErr::Error(Error {
                input: "\n",
                code: ErrorKind::Not
            }))
        );
        assert_eq!(
            parse_element_markdown_inline(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        );
    }
}

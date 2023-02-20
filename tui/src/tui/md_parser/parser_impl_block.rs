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
        branch::*, bytes::complete::*, character::complete::*, combinator::*, multi::*, sequence::*,
        IResult,
    };

/// Parse a single line of markdown text [Line].
pub fn parse_block_markdown_text_until_eol(input: &str) -> IResult<&str, Fragments> {
    terminated(
        /* output */ many0(parse_element_markdown_inline),
        /* ends with (discarded) */ tag(NEW_LINE),
    )(input)
}

/// Matches one or more `#` chars, consumes it, and outputs [Level].
pub fn parse_heading_tag(input: &str) -> IResult<&str, Level> {
    map(
        terminated(
            /* output `#`+ */ take_while1(|it| it == constants::HEADING_CHAR),
            /* ends with (discarded) */ tag(constants::SPACE),
        ),
        |it: &str| Level::from(it.len()),
    )(input)
}

/// This matches the heading tag and text until EOL. Outputs a tuple of [Level] and
/// [Line].
pub fn parse_block_heading(input: &str) -> IResult<&str, (Level, Fragments)> {
    tuple(
        (parse_heading_tag, parse_block_markdown_text_until_eol)
    )(input)
}

/// Matches `- `. Outputs the `-` char.
pub fn parse_unordered_list_tag(input: &str) -> IResult<&str, &str> {
    terminated(
        /* output `-` */ tag(UNORDERED_LIST),
        /* ends with (discarded) */ tag(SPACE),
    )(input)
}

pub fn parse_unordered_list_element(input: &str) -> IResult<&str, Fragments> {
    preceded(
        /* prefix (discarded) */ parse_unordered_list_tag,
        /* output */ parse_block_markdown_text_until_eol,
    )(input)
}

pub fn parse_block_unordered_list(input: &str) -> IResult<&str, Vec<Fragments>> {
    many1(
        parse_unordered_list_element
    )(input)
}

pub fn parse_ordered_list_tag(input: &str) -> IResult<&str, &str> {
    terminated(
        /* output */
        terminated(
            /* output */ digit1,
            /* ends with (discarded) */ tag(PERIOD),
        ),
        /* ends with (discarded) */ tag(SPACE),
    )(input)
}

pub fn parse_ordered_list_element(input: &str) -> IResult<&str, Fragments> {
    preceded(
        /* prefix (discarded) */ parse_ordered_list_tag,
        /* output */ parse_block_markdown_text_until_eol,
    )(input)
}

pub fn parse_block_ordered_list(input: &str) -> IResult<&str, Vec<Fragments>> {
    many1(
        parse_ordered_list_element
    )(input)
}

pub fn parse_block_code(input: &str) -> IResult<&str, CodeBlock> {
    let (input, output) = tuple(
        (parse_code_block_lang, parse_code_block_body)
    )(input)?;
    Ok((input, CodeBlock::from(output)))
}

pub fn parse_code_block_body(input: &str) -> IResult<&str, &str> {
    delimited(
        /* start */ tag(NEW_LINE),
        /* output */ is_not(CODE_BLOCK),
        /* end */ tag(CODE_BLOCK),
    )(input)
}

pub fn parse_code_block_lang(input: &str) -> IResult<&str, &str> {
    alt((
        // Either - Successfully parse both code block language & text.
        preceded(
            /* prefix - discarded */ tag(CODE_BLOCK),
            /* output */ parse_element_plaintext,
        ),
        // Or - Fail to parse language, use unknown language instead.
        map(tag(CODE_BLOCK), |_| constants::UNKNOWN_LANGUAGE),
    ))(input)
}

}
pub use no_rustfmt_block::*;

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

    use super::*;
    use crate::{constants::UNKNOWN_LANGUAGE, *};

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
            Ok(("", vec![
                Fragment::Plain("here is some plaintext "),
                Fragment::Italic("but what if we italicize?"),
                Fragment::Plain(" I guess it doesn't "),
                Fragment::Bold("matter"),
                Fragment::Plain(" in my "),
                Fragment::InlineCode("code"),
            ]))
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

    #[test]
    fn test_parse_header_tag() {
        assert_eq!(parse_heading_tag("# "), Ok(("", 1.into())));
        assert_eq!(parse_heading_tag("### "), Ok(("", 3.into())));
        assert_eq!(parse_heading_tag("# h1"), Ok(("h1", 1.into())));
        assert_eq!(parse_heading_tag("# h1"), Ok(("h1", 1.into())));
        assert_eq!(
            parse_heading_tag(" "),
            Err(NomErr::Error(Error {
                input: " ",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq!(
            parse_heading_tag("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_header() {
        assert_eq!(
            parse_block_heading("# h1\n"),
            Ok(("", (1.into(), vec![Fragment::Plain("h1")])))
        );
        assert_eq!(
            parse_block_heading("## h2\n"),
            Ok(("", (2.into(), vec![Fragment::Plain("h2")])))
        );
        assert_eq!(
            parse_block_heading("###  h3\n"),
            Ok(("", (3.into(), vec![Fragment::Plain(" h3")])))
        );
        assert_eq!(
            parse_block_heading("###h3"),
            Err(NomErr::Error(Error {
                input: "h3",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_heading("###"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_heading(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq!(
            parse_block_heading("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(parse_block_heading("# \n"), Ok(("", (1.into(), vec![]))));
        assert_eq!(
            parse_block_heading("# test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_unordered_list_tag() {
        assert_eq!(parse_unordered_list_tag("- "), Ok(("", "-")));
        assert_eq!(
            parse_unordered_list_tag("- and some more"),
            Ok(("and some more", "-"))
        );
        assert_eq!(
            parse_unordered_list_tag("-"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_tag("-and some more"),
            Err(NomErr::Error(Error {
                input: "and some more",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_tag("--"),
            Err(NomErr::Error(Error {
                input: "-",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_tag(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_unordered_list_element() {
        assert_eq!(
            parse_unordered_list_element("- this is an element\n"),
            Ok(("", vec![Fragment::Plain("this is an element")]))
        );
        assert_eq!(
            parse_unordered_list_element(raw_strings::UNORDERED_LIST_ELEMENT),
            Ok((
                "- this is another element\n",
                vec![Fragment::Plain("this is an element")]
            ))
        );
        assert_eq!(
            parse_unordered_list_element(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(parse_unordered_list_element("- \n"), Ok(("", vec![])));
        assert_eq!(
            parse_unordered_list_element("- "),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_element("- test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_element("-"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_unordered_list() {
        assert_eq!(
            parse_block_unordered_list("- this is an element"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_unordered_list("- this is an element\n"),
            Ok(("", vec![vec![Fragment::Plain("this is an element")]]))
        );
        assert_eq!(
            parse_block_unordered_list(raw_strings::UNORDERED_LIST_ELEMENT),
            Ok((
                "",
                vec![
                    vec![Fragment::Plain("this is an element")],
                    vec![Fragment::Plain("this is another element")]
                ]
            ))
        );
    }

    #[test]
    fn test_parse_ordered_list_tag() {
        assert_eq!(parse_ordered_list_tag("1. "), Ok(("", "1")));
        assert_eq!(parse_ordered_list_tag("1234567. "), Ok(("", "1234567")));
        assert_eq!(
            parse_ordered_list_tag("3. and some more"),
            Ok(("and some more", "3"))
        );
        assert_eq!(
            parse_ordered_list_tag("1"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_ordered_list_tag("1.and some more"),
            Err(NomErr::Error(Error {
                input: "and some more",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_ordered_list_tag("1111."),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_ordered_list_tag(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Digit
            }))
        );
    }

    #[test]
    fn test_parse_ordered_list_element() {
        assert_eq!(
            parse_ordered_list_element("1. this is an element\n"),
            Ok(("", vec![Fragment::Plain("this is an element")]))
        );
        assert_eq!(
            parse_ordered_list_element(raw_strings::ORDERED_LIST_ELEMENT),
            Ok((
                "1. here is another\n",
                vec![Fragment::Plain("this is an element")]
            ))
        );
        assert_eq!(
            parse_ordered_list_element(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Digit
            }))
        );
        assert_eq!(
            parse_ordered_list_element(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Digit
            }))
        );
        assert_eq!(parse_ordered_list_element("1. \n"), Ok(("", vec![])));
        assert_eq!(
            parse_ordered_list_element("1. test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_ordered_list_element("1. "),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_ordered_list_element("1."),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_ordered_list() {
        assert_eq!(
            parse_block_ordered_list("1. this is an element\n"),
            Ok(("", vec![vec![Fragment::Plain("this is an element")]]))
        );
        assert_eq!(
            parse_block_ordered_list("1. test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_ordered_list(raw_strings::ORDERED_LIST_ELEMENT),
            Ok((
                "",
                vec![
                    vec!(Fragment::Plain("this is an element")),
                    vec![Fragment::Plain("here is another")]
                ]
            ))
        );
    }

    #[test]
    fn test_parse_codeblock() {
        assert_eq!(
            parse_block_code(raw_strings::CODE_BLOCK_3_INPUT),
            Ok((
                "",
                CodeBlock::from(("bash", raw_strings::CODE_BLOCK_3_OUTPUT))
            ))
        );
        assert_eq!(
            parse_block_code(raw_strings::CODE_BLOCK_2_INPUT),
            Ok((
                "",
                CodeBlock::from(("python", raw_strings::CODE_BLOCK_2_OUTPUT))
            ))
        );
    }

    #[test]
    fn test_parse_codeblock_no_language() {
        assert_eq!(
            parse_block_code(raw_strings::CODE_BLOCK_1_INPUT),
            Ok((
                "",
                CodeBlock::from((UNKNOWN_LANGUAGE, raw_strings::CODE_BLOCK_1_OUTPUT))
            ))
        );
    }

    #[rustfmt::skip]
    mod raw_strings {
        pub const UNORDERED_LIST_ELEMENT: &str =
r#"- this is an element
- this is another element
"#;
        pub const ORDERED_LIST_ELEMENT: &str =
r#"1. this is an element
1. here is another
"#;

        pub const CODE_BLOCK_1_INPUT: &str =
r#"```
pip install foobar
```"#;

        pub const CODE_BLOCK_1_OUTPUT: &str =
r#"pip install foobar
"#;

        pub const CODE_BLOCK_2_INPUT: &str =
r#"```python
import foobar

foobar.pluralize('word') # returns 'words'
foobar.pluralize('goose') # returns 'geese'
foobar.singularize('phenomena') # returns 'phenomenon'
```"#;

        pub const CODE_BLOCK_2_OUTPUT: &str =
r#"import foobar

foobar.pluralize('word') # returns 'words'
foobar.pluralize('goose') # returns 'geese'
foobar.singularize('phenomena') # returns 'phenomenon'
"#;

        pub const CODE_BLOCK_3_INPUT: &str =
r#"```bash
pip install foobar
```"#;

        pub const CODE_BLOCK_3_OUTPUT: &str =
r#"pip install foobar
"#;

    }
}

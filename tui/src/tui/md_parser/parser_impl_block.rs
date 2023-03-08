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
use nom::{branch::*,
          bytes::complete::*,
          character::complete::*,
          combinator::*,
          multi::*,
          sequence::*,
          IResult};

use crate::*;

// AI: 0. refactor all of this & tests into clean modules

/// Parse a single line of markdown text [Line].
#[rustfmt::skip]
pub fn parse_block_markdown_text_until_eol(input: &str) -> IResult<&str, Fragments> {
    parse_block_markdown_text_until_eol_impl::parse(input)
}

mod parse_block_markdown_text_until_eol_impl {
    use super::*;

    #[rustfmt::skip]
    pub fn parse(input: &str) -> IResult<&str, Fragments> {
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
}

/// Sample inputs:
/// One line:                "```bash\npip install foobar\n```\n"
/// No line:                 "```\n\n```\n"
/// Multi line:              "```bash\npip install foobar\npip install foobar\n```\n"
/// No language:             "```\npip install foobar\n```\n"
/// No language, no line:    "```\n```\n"
/// No language, multi line: "```\npip install foobar\npip install foobar\n```\n"
pub fn parse_block_code(input: &str) -> IResult<&str, CodeBlock> {
    parse_block_code_impl::parse(input)
}

mod parse_block_code_impl {
    use super::*;

    #[rustfmt::skip]
    pub fn parse(input: &str) -> IResult<&str, CodeBlock> {
        use parse_block_code_impl::*;

        let (input, (lang, code)) = tuple(
            (parse_code_block_lang_to_eol, parse_code_block_body_to_code_block_end_to_eol)
        )(input)?;
        let acc = split_by_newline(code);
        return Ok((input, CodeBlock::new(lang, acc)));
    }

    pub fn parse_code_block_lang_to_eol(input: &str) -> IResult<&str, &str> {
        alt((
            // Either - Successfully parse both code block language & text.
            preceded(
                /* prefix - discarded */ tag(CODE_BLOCK_START_PARTIAL),
                /* output */
                terminated(
                    /* match */ is_not(NEW_LINE),
                    /* ends with (discarded) */ tag(NEW_LINE),
                ),
            ),
            // Or - Fail to parse language, use unknown language instead.
            map(
                tuple((tag(CODE_BLOCK_START_PARTIAL), tag(NEW_LINE))),
                |_| constants::UNKNOWN_LANGUAGE,
            ),
        ))(input)
    }

    pub fn parse_code_block_body_to_code_block_end_to_eol(input: &str) -> IResult<&str, &str> {
        terminated(
            take_until(CODE_BLOCK_END),
            /* end (discard) */ tag(CODE_BLOCK_END),
        )(input)
    }

    /// Split a string by newline. The idea is that a line is some text followed by a newline. An
    /// empty line is just a newline character.
    ///
    /// ## Examples:
    /// | input          | output               |
    /// | -------------- | -------------------- |
    /// | "foobar\n"     | `["foobar"]`         |
    /// | "\n"           | `[""] `              |
    /// | ""             | `[] `                |
    /// | "foo\nbar\n"   | `["foo", "bar"] `    |
    /// | "\nfoo\nbar\n" | `["", "foo", "bar"]` |
    pub fn split_by_newline(input: &str) -> Vec<&str> {
        let mut acc: Vec<&str> = input.split('\n').collect();
        if let Some(last_item) = acc.last() {
            if last_item.is_empty() {
                acc.pop();
            }
        }
        acc
    }

    #[cfg(test)]
    mod tests {
        use r3bl_rs_utils_core::*;

        use super::*;
        use crate::constants::UNKNOWN_LANGUAGE;

        #[test]
        fn test_parse_codeblock_split_by_eol() {
            assert_eq2!(split_by_newline("foobar\n"), vec!["foobar"]);
            assert_eq2!(split_by_newline("\n"), vec![""]);
            assert_eq2!(split_by_newline(""), Vec::<&str>::new());
            assert_eq2!(split_by_newline("foo\nbar\n"), vec!["foo", "bar"]);
            assert_eq2!(split_by_newline("\nfoo\nbar\n"), vec!["", "foo", "bar"]);
        }

        #[test]
        fn test_parse_codeblock() {
            // One line: "```bash\npip install foobar\n```\n"
            {
                let lang = "bash";
                let code_lines = vec!["pip install foobar"];
                println!(
                    "{:#?}",
                    (parser_impl_block::raw_strings::CODE_BLOCK_3_INPUT)
                );
                assert_eq2!(
                    parse_block_code(parser_impl_block::raw_strings::CODE_BLOCK_3_INPUT),
                    Ok(("", CodeBlock::new(lang, code_lines)))
                );
            }

            // No line: "```bash\n```\n"
            {
                let lang = "bash";
                let code_lines = vec![];
                assert_eq2!(
                    parse_block_code(parser_impl_block::raw_strings::CODE_BLOCK_0_INPUT),
                    Ok(("", CodeBlock::new(lang, code_lines)))
                );
            }

            // 1 empty line: "```bash\n\n```\n"
            {
                let lang = "bash";
                let code_lines = vec![""];
                assert_eq2!(
                    parse_block_code(parser_impl_block::raw_strings::CODE_BLOCK_1_EMPTY_INPUT),
                    Ok(("", CodeBlock::new(lang, code_lines)))
                );
            }

            // Multiple lines.
            // "import foobar\n\nfoobar.pluralize('word') # returns 'words'\nfoobar.pluralize('goose') # returns 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n```"
            {
                let lang = "python";
                let code_lines = vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ];
                assert_eq2!(
                    parse_block_code(parser_impl_block::raw_strings::CODE_BLOCK_2_INPUT),
                    Ok(("", CodeBlock::new(lang, code_lines)))
                );
            }
        }

        #[test]
        fn test_parse_codeblock_no_language() {
            assert_eq!(
                parse_block_code(parser_impl_block::raw_strings::CODE_BLOCK_1_INPUT),
                Ok((
                    "",
                    CodeBlock::new(UNKNOWN_LANGUAGE, vec!["pip install foobar"])
                ))
            );
        }
    }
}

/// This matches the heading tag and text until EOL. Outputs a tuple of [Level] and [Line].
#[rustfmt::skip]
pub fn parse_block_heading(input: &str) -> IResult<&str, (Level, Fragments)> {
    parse_block_heading_impl::parse(input)
}

mod parse_block_heading_impl {
    use super::*;

    #[rustfmt::skip]
    pub fn parse(input: &str) -> IResult<&str, (Level, Fragments)> {
        tuple(
            (parse_heading_tag, parse_block_markdown_text_until_eol)
        )(input)
    }

    /// Matches one or more `#` chars, consumes it, and outputs [Level].
    #[rustfmt::skip]
    pub fn parse_heading_tag(input: &str) -> IResult<&str, Level> {
        map(
            terminated(
                /* output `#`+ */ take_while1(|it| it == constants::HEADING_CHAR),
                /* ends with (discarded) */ tag(constants::SPACE),
            ),
            |it: &str| Level::from(it.len()),
        )(input)
    }

    #[cfg(test)]
    mod tests {
        use nom::{error::{Error, ErrorKind},
                  Err as NomErr};

        use super::*;

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
    }
}

/// Matches `- `. Outputs the `-` char.
#[rustfmt::skip]
pub fn parse_unordered_list_tag(input: &str) -> IResult<&str, &str> {
    terminated(
        /* output `-` */ tag(UNORDERED_LIST),
        /* ends with (discarded) */ tag(SPACE),
    )(input)
}

#[rustfmt::skip]
pub fn parse_unordered_list_element(input: &str) -> IResult<&str, Fragments> {
    preceded(
        /* prefix (discarded) */ parse_unordered_list_tag,
        /* output */ parse_block_markdown_text_until_eol,
    )(input)
}

#[rustfmt::skip]
pub fn parse_block_unordered_list(input: &str) -> IResult<&str, Vec<Fragments>> {
    many1(
        parse_unordered_list_element
    )(input)
}

#[rustfmt::skip]
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

#[rustfmt::skip]
pub fn parse_ordered_list_element(input: &str) -> IResult<&str, Fragments> {
    preceded(
        /* prefix (discarded) */ parse_ordered_list_tag,
        /* output */ parse_block_markdown_text_until_eol,
    )(input)
}

#[rustfmt::skip]
pub fn parse_block_ordered_list(input: &str) -> IResult<&str, Vec<Fragments>> {
    many1(
        parse_ordered_list_element
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

    use super::*;

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
}

#[rustfmt::skip]
#[cfg(test)]
mod raw_strings {
    pub const UNORDERED_LIST_ELEMENT: &str =
r#"- this is an element
- this is another element
"#;
    pub const ORDERED_LIST_ELEMENT: &str =
r#"1. this is an element
1. here is another
"#;

    pub const CODE_BLOCK_0_INPUT: &str =
r#"```bash
```
"#;

    pub const CODE_BLOCK_1_EMPTY_INPUT: &str =
r#"```bash

```
"#;

    pub const CODE_BLOCK_1_INPUT: &str =
r#"```
pip install foobar
```
"#;

    pub const CODE_BLOCK_2_INPUT: &str =
r#"```python
import foobar

foobar.pluralize('word') # returns 'words'
foobar.pluralize('goose') # returns 'geese'
foobar.singularize('phenomena') # returns 'phenomenon'
```
"#;

    pub const CODE_BLOCK_3_INPUT: &str =
r#"```bash
pip install foobar
```
"#;

}

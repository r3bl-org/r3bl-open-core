/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

//! A single line of Markdown may have many fragments, eg: `**bold**`, `*italic*`,
//! `[link](http://r3bl.com)`, etc.
//!
//! As opposed to markdown [crate::block]s (like code block or smart lists) which may span
//! multiple lines.
//!
//! Fragments may be found in every single line of text. These parsers extract each
//! fragment into either a string slice or some other intermediate representation.
//!
//! To see this in action, set the [crate::DEBUG_MD_PARSER_STDOUT] to true, and run all
//! the tests in this file.

use crossterm::style::Stylize;
use nom::{branch::alt, combinator::map, IResult};
use r3bl_core::{call_if_true, string_storage};

use crate::{parse_fragment_plain_text_no_new_line,
            parse_fragment_starts_with_backtick_err_on_new_line,
            parse_fragment_starts_with_checkbox_checkbox_into_bool,
            parse_fragment_starts_with_checkbox_into_str,
            parse_fragment_starts_with_left_image_err_on_new_line,
            parse_fragment_starts_with_left_link_err_on_new_line,
            parse_fragment_starts_with_star_err_on_new_line,
            parse_fragment_starts_with_underscore_err_on_new_line,
            CheckboxParsePolicy,
            MdLineFragment,
            DEBUG_MD_PARSER};

// XMARK: Parser for a single line of markdown

/// Parse a single chunk of Markdown text (found in a single line of text) into a
/// [MdLineFragment]. If there is no [crate::constants::NEW_LINE] character, then parse
/// the entire input.
///
/// Here's an example of the runtime iterations that may occur, which repeatedly run by
/// [crate::parse_block_markdown_text_with_or_without_new_line()]:
///
/// ```txt
/// input: "foo *bar* _baz_ [link](url) ![image](url)"
/// pass #1: [Plain("foo ")] | "*bar* _baz_ [link](url) ![image](url)"
/// pass #2: [Bold("bar")]   | " _baz_ [link](url) ![image](url)"
/// pass #3: [Plain(" ")]    | "_baz_ [link](url) ![image](url)"
/// pass #4: [Italic("baz")] | " [link](url) ![image](url)"
/// etc.
/// ```
///
/// To see this in action, set the [crate::DEBUG_MD_PARSER_STDOUT] to true, and run all
/// the tests in this file.
#[rustfmt::skip]
pub fn parse_inline_fragments_until_eol_or_eoi(
    input: &str,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<&str, MdLineFragment<'_>> {
    // The order of the following parsers is important. The highest priority parser is at
    // the top. The lowest priority parser is at the bottom. This is because the first
    // parser that matches will be the one that is used.
    let it = match checkbox_policy {
        CheckboxParsePolicy::IgnoreCheckbox => alt((
            map(parse_fragment_starts_with_underscore_err_on_new_line,  MdLineFragment::Italic),
            map(parse_fragment_starts_with_star_err_on_new_line,        MdLineFragment::Bold),
            map(parse_fragment_starts_with_backtick_err_on_new_line,    MdLineFragment::InlineCode),
            map(parse_fragment_starts_with_left_image_err_on_new_line,  MdLineFragment::Image),
            map(parse_fragment_starts_with_left_link_err_on_new_line,   MdLineFragment::Link),
            map(parse_fragment_starts_with_checkbox_into_str,           MdLineFragment::Plain), // This line is different.
            map(parse_fragment_plain_text_no_new_line,                  MdLineFragment::Plain),
        ))(input),
        CheckboxParsePolicy::ParseCheckbox => alt((
            map(parse_fragment_starts_with_underscore_err_on_new_line,  MdLineFragment::Italic),
            map(parse_fragment_starts_with_star_err_on_new_line,        MdLineFragment::Bold),
            map(parse_fragment_starts_with_backtick_err_on_new_line,    MdLineFragment::InlineCode),
            map(parse_fragment_starts_with_left_image_err_on_new_line,  MdLineFragment::Image),
            map(parse_fragment_starts_with_left_link_err_on_new_line,   MdLineFragment::Link),
            map(parse_fragment_starts_with_checkbox_checkbox_into_bool, MdLineFragment::Checkbox), // This line is different.
            map(parse_fragment_plain_text_no_new_line,                  MdLineFragment::Plain),
        ))(input)

    };

    call_if_true!(DEBUG_MD_PARSER, {
        tracing::debug!("\n📣📣📣\n input: {}", string_storage!("{input:?}").green());
        match it {
            Ok(ref element) => {
                tracing::debug!("✅✅✅ OK {}", string_storage!("{element:#?}").magenta());
            },
            Err(ref error) => {
                tracing::debug!("🟥🟥🟥 NO {}", string_storage!("{error:#?}").red());
            },
        }
    });

    it
}

#[cfg(test)]
mod tests_parse_fragment {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};
    use r3bl_core::assert_eq2;

    use super::*;
    use crate::HyperlinkData;

    #[test]
    fn test_parse_plain_text_no_new_line1() {
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("this _bar"),
            Ok((/*rem*/ "_bar", /*output*/ "this "))
        );

        assert_eq2!(
            parse_fragment_plain_text_no_new_line("_bar"),
            Ok(("bar", "_"))
        );

        assert_eq2!(
            parse_fragment_plain_text_no_new_line("bar_"),
            Ok(("_", "bar"))
        );
    }

    #[test]
    fn test_parse_fragment_checkbox_into_str() {
        assert_eq2!(
            parse_fragment_starts_with_checkbox_into_str("[x] here is a checkbox"),
            Ok((/*rem*/ " here is a checkbox", /*output*/ "[x]"))
        );

        assert_eq2!(
            parse_fragment_starts_with_checkbox_into_str("[ ] here is a checkbox"),
            Ok((" here is a checkbox", "[ ]"))
        );
    }

    #[test]
    fn test_parse_fragment_checkbox_into_bool() {
        assert_eq2!(
            parse_fragment_starts_with_checkbox_checkbox_into_bool(
                "[x] here is a checkbox"
            ),
            Ok((/*rem*/ " here is a checkbox", /*output*/ true))
        );

        assert_eq2!(
            parse_fragment_starts_with_checkbox_checkbox_into_bool(
                "[ ] here is a checkbox"
            ),
            Ok((" here is a checkbox", false))
        );
    }

    /// These are tests for underscores.
    #[test]
    fn test_parse_fragment_italic() {
        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line("_here is italic_"),
            Ok((/*rem*/ "", /*output*/ "here is italic"))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line("_here is italic_"),
            Ok(("", "here is italic"))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line("*here is italic"),
            Err(NomErr::Error(Error {
                input: "*here is italic",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line("here is italic*"),
            Err(NomErr::Error(Error {
                input: "here is italic*",
                code: ErrorKind::Fail,
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line("here is italic"),
            Err(NomErr::Error(Error {
                input: "here is italic",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line("*"),
            Err(NomErr::Error(Error {
                input: "*",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line("**"),
            Err(NomErr::Error(Error {
                input: "**",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_underscore_err_on_new_line(
                "**we are doing bold**"
            ),
            Err(NomErr::Error(Error {
                input: "**we are doing bold**",
                code: ErrorKind::Fail
            }))
        );
    }

    /// These are these tests for stars.
    #[test]
    fn test_parse_fragment_bold() {
        assert_eq2!(
            parse_fragment_starts_with_star_err_on_new_line("*here is bold*"),
            Ok((/*rem*/ "", /*output*/ "here is bold"))
        );

        assert_eq2!(
            parse_fragment_starts_with_star_err_on_new_line("*here is bold"),
            Err(NomErr::Error(Error {
                input: "*here is bold",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_star_err_on_new_line("here is bold*"),
            Err(NomErr::Error(Error {
                input: "here is bold*",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_star_err_on_new_line("here is bold"),
            Err(NomErr::Error(Error {
                input: "here is bold",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_star_err_on_new_line("*"),
            Err(NomErr::Error(Error {
                input: "*",
                code: ErrorKind::Fail
            }))
        );

        assert_eq2!(
            parse_fragment_starts_with_star_err_on_new_line(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Fail
            }))
        );
    }

    /// These are tests for backticks.
    #[test]
    fn test_parse_fragment_inline_code() {
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line("`here is code"),
            Err(NomErr::Error(Error {
                input: "`here is code",
                code: ErrorKind::Fail
            }))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line("here is code`"),
            Err(NomErr::Error(Error {
                input: "here is code`",
                code: ErrorKind::Fail
            }))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line("``"),
            Ok((/*rem*/ "", /*output*/ ""))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line("`"),
            Err(NomErr::Error(Error {
                input: "`",
                code: ErrorKind::Fail
            }))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Fail
            }))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line("`abcd`"),
            Ok(("", "abcd"))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line("```"),
            Err(NomErr::Error(Error {
                input: "```",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_fragment_link() {
        assert_eq2!(
            parse_fragment_starts_with_left_link_err_on_new_line(
                "[title](https://www.example.com)"
            ),
            Ok((
                /*rem*/ "",
                /*output*/ HyperlinkData::new("title", "https://www.example.com")
            ))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Fail
            }))
        );
    }

    #[test]
    fn test_parse_fragment_image() {
        assert_eq2!(
            parse_fragment_starts_with_left_image_err_on_new_line(
                "![alt text](image.jpg)"
            ),
            Ok((
                /*rem*/ "",
                /*output*/ HyperlinkData::new("alt text", "image.jpg")
            ))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Fail
            }))
        );
    }

    #[test]
    fn test_parse_fragment_plaintext_unicode() {
        let result = parse_fragment_plain_text_no_new_line("- straight😃\n");
        let remainder = result.as_ref().unwrap().0;
        let output = result.as_ref().unwrap().1;
        assert_eq2!(remainder, "\n");
        assert_eq2!(output, "- straight😃");
    }

    #[test]
    fn test_parse_fragment_plaintext() {
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("1234567890"),
            Ok((/*rem*/ "", /*output*/ "1234567890"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("oh my gosh!"),
            Ok(("", "oh my gosh!"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("oh my gosh!["),
            Ok(("![", "oh my gosh"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("oh my gosh!*"),
            Ok(("*", "oh my gosh!"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("*bold baby bold*"),
            Ok(("", "*bold baby bold*"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("[link baby](and then somewhat)"),
            Ok(("", "[link baby](and then somewhat)"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("`codeblock for bums`"),
            Ok(("", "`codeblock for bums`"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("![ but wait theres more](jk)"),
            Ok(("", "![ but wait theres more](jk)"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("here is plaintext"),
            Ok(("", "here is plaintext"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("here is plaintext!"),
            Ok(("", "here is plaintext!"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("here is plaintext![image starting"),
            Ok(("![image starting", "here is plaintext"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("here is plaintext\n"),
            Ok(("\n", "here is plaintext"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("*here is italic*"),
            Ok(("", "*here is italic*"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("**here is bold**"),
            Ok(("", "**here is bold**"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("`here is code`"),
            Ok(("", "`here is code`"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("[title](https://www.example.com)"),
            Ok(("", "[title](https://www.example.com)"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line("![alt text](image.jpg)"),
            Ok(("", "![alt text](image.jpg)"))
        );
        assert_eq2!(
            parse_fragment_plain_text_no_new_line(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        );
    }

    #[test]
    fn test_parse_fragment_markdown_inline() {
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "here is plaintext!",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                /*rem*/ "",
                /*output*/ MdLineFragment::Plain("here is plaintext!")
            ))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "*here is bold*",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::Bold("here is bold")))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "_here is italic_",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::Italic("here is italic")))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "`here is code`",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::InlineCode("here is code")))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "[title](https://www.example.com)",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "",
                MdLineFragment::Link(HyperlinkData::new(
                    "title",
                    "https://www.example.com"
                ))
            ))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "![alt text](image.jpg)",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "",
                MdLineFragment::Image(HyperlinkData::new("alt text", "image.jpg"))
            ))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "here is plaintext!",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::Plain("here is plaintext!")))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "here is some plaintext *but what if we italicize?",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "*but what if we italicize?",
                MdLineFragment::Plain("here is some plaintext ")
            ))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "here is some plaintext \n*but what if we italicize?",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "\n*but what if we italicize?",
                MdLineFragment::Plain("here is some plaintext ")
            ))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "\n",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Err(NomErr::Error(Error {
                input: "\n",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        );

        // Deal with checkboxes: ignore them.
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "[ ] this is a checkbox",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Plain("[ ]")))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "[x] this is a checkbox",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Plain("[x]")))
        );

        // Deal with checkboxes: parse them.
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "[ ] this is a checkbox",
                CheckboxParsePolicy::ParseCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Checkbox(false)))
        );
        assert_eq2!(
            parse_inline_fragments_until_eol_or_eoi(
                "[x] this is a checkbox",
                CheckboxParsePolicy::ParseCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Checkbox(true)))
        );
    }
}

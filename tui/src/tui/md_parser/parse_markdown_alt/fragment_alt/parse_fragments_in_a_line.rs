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

use nom::{branch::alt, combinator::map, IResult, Parser};

use super::{parse_fragment_plain_text_no_new_line,
            parse_fragment_starts_with_backtick_err_on_new_line,
            parse_fragment_starts_with_checkbox_checkbox_into_bool,
            parse_fragment_starts_with_checkbox_into_str,
            parse_fragment_starts_with_left_image_err_on_new_line,
            parse_fragment_starts_with_left_link_err_on_new_line,
            parse_fragment_starts_with_star_err_on_new_line,
            parse_fragment_starts_with_underscore_err_on_new_line};
use crate::{fg_green,
            fg_red,
            inline_string,
            AsStrSlice,
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
pub fn parse_inline_fragments_until_eol_or_eoi<'a>(
    input: AsStrSlice<'a>,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<AsStrSlice<'a>, MdLineFragment<'a>> {
    // Debug assertion to ensure the input is a single line without newline characters
    debug_assert!(!input.extract_remaining_text_content_in_line().contains('\n'),
                 "Input must be a single line without newline characters");
    // The order of the following parsers is important. The highest priority parser is at
    // the top. The lowest priority parser is at the bottom. This is because the first
    // parser that matches will be the one that is used.

    // Clone the input to avoid ownership issues
    let input_clone = input.clone();

    let it = match checkbox_policy {
        CheckboxParsePolicy::IgnoreCheckbox => alt((
            map(parse_fragment_starts_with_underscore_err_on_new_line,  |s| MdLineFragment::Italic(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_star_err_on_new_line,        |s| MdLineFragment::Bold(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_backtick_err_on_new_line,    |s| MdLineFragment::InlineCode(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_left_image_err_on_new_line,  MdLineFragment::Image),
            map(parse_fragment_starts_with_left_link_err_on_new_line,   MdLineFragment::Link),
            map(parse_fragment_starts_with_checkbox_into_str,           |s| MdLineFragment::Plain(s.extract_remaining_text_content_in_line())), // This line is different.
            map(parse_fragment_plain_text_no_new_line,                  |s| MdLineFragment::Plain(s.extract_remaining_text_content_in_line())),
        )).parse(input_clone.clone()),
        CheckboxParsePolicy::ParseCheckbox => alt((
            map(parse_fragment_starts_with_underscore_err_on_new_line,  |s| MdLineFragment::Italic(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_star_err_on_new_line,       |s| MdLineFragment::Bold(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_backtick_err_on_new_line,    |s| MdLineFragment::InlineCode(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_left_image_err_on_new_line,  MdLineFragment::Image),
            map(parse_fragment_starts_with_left_link_err_on_new_line,   MdLineFragment::Link),
            map(parse_fragment_starts_with_checkbox_checkbox_into_bool, MdLineFragment::Checkbox), // This line is different.
            map(parse_fragment_plain_text_no_new_line,                 |s| MdLineFragment::Plain(s.extract_remaining_text_content_in_line())),
        )).parse(input_clone)
    };

    DEBUG_MD_PARSER.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "📣📣📣 input",
            input = ?input
        );
        match it {
            Ok(ref element) => {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "✅✅✅ OK",
                    element = %fg_green(&inline_string!("{element:#?}"))
                );
            },
            Err(ref error) => {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "🟥🟥🟥 NO",
                    error = %fg_red(&inline_string!("{error:#?}"))
                );
            },
        }
    });

    it
}

#[cfg(test)]
mod tests_parse_fragment {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

    use super::*;
    use crate::{assert_eq2, GCString, HyperlinkData};

    #[test]
    fn test_parse_plain_text_no_new_line1() {
        {
            let input_vec = vec![GCString::new("this _bar")];
            let input = AsStrSlice::from(&input_vec);
            let rem_vec = vec![GCString::new("_bar")];
            let rem = AsStrSlice::from(&rem_vec);
            let output_vec = vec![GCString::new("this ")];
            let output = AsStrSlice::from(&output_vec);

            assert_eq2!(
                parse_fragment_plain_text_no_new_line(input),
                Ok((rem, output))
            );
        }

        {
            let input_vec = vec![GCString::new("_bar")];
            let input = AsStrSlice::from(&input_vec);
            let rem_vec = vec![GCString::new("bar")];
            let rem = AsStrSlice::from(&rem_vec);
            let output_vec = vec![GCString::new("_")];
            let output = AsStrSlice::from(&output_vec);

            assert_eq2!(
                parse_fragment_plain_text_no_new_line(input),
                Ok((rem, output))
            );
        }

        {
            let input_vec = vec![GCString::new("bar_")];
            let input = AsStrSlice::from(&input_vec);
            let rem_vec = vec![GCString::new("_")];
            let rem = AsStrSlice::from(&rem_vec);
            let output_vec = vec![GCString::new("bar")];
            let output = AsStrSlice::from(&output_vec);

            assert_eq2!(
                parse_fragment_plain_text_no_new_line(input),
                Ok((rem, output))
            );
        }
    }

    #[test]
    fn test_parse_fragment_checkbox_into_str() {
        {
            let input_vec = vec![GCString::new("[x] here is a checkbox")];
            let input = AsStrSlice::from(&input_vec);

            let expected_rem_vec = vec![GCString::new(" here is a checkbox")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            let expected_output_vec = vec![GCString::new("[x]")];
            let expected_output = AsStrSlice::from(&expected_output_vec);

            assert_eq2!(
                parse_fragment_starts_with_checkbox_into_str(input),
                Ok((
                    /* rem */ expected_rem,
                    /* output */ expected_output
                ))
            );
        }

        {
            let input_vec = vec![GCString::new("[ ] here is a checkbox")];
            let input = AsStrSlice::from(&input_vec);

            let expected_rem_vec = vec![GCString::new(" here is a checkbox")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            let expected_output_vec = vec![GCString::new("[ ]")];
            let expected_output = AsStrSlice::from(&expected_output_vec);

            assert_eq2!(
                parse_fragment_starts_with_checkbox_into_str(input),
                Ok((expected_rem, expected_output))
            );
        }
    }

    #[test]
    fn test_parse_fragment_checkbox_into_bool() {
        {
            let input_vec = vec![GCString::new("[x] here is a checkbox")];
            let input = AsStrSlice::from(&input_vec);

            let expected_rem_vec = vec![GCString::new(" here is a checkbox")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            assert_eq2!(
                parse_fragment_starts_with_checkbox_checkbox_into_bool(input),
                Ok((/* rem */ expected_rem, /* output */ true))
            );
        }

        {
            let input_vec = vec![GCString::new("[ ] here is a checkbox")];
            let input = AsStrSlice::from(&input_vec);

            let expected_rem_vec = vec![GCString::new(" here is a checkbox")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            assert_eq2!(
                parse_fragment_starts_with_checkbox_checkbox_into_bool(input),
                Ok((expected_rem, false))
            );
        }
    }

    /// These are tests for underscores.
    #[test]
    fn test_parse_fragment_italic() {
        {
            let input_vec = vec![GCString::new("_here is italic_")];
            let input = AsStrSlice::from(&input_vec);

            let expected_output_vec = vec![GCString::new("here is italic")];
            let expected_output = AsStrSlice::from(&expected_output_vec);

            let expected_rem_vec = vec![GCString::new("")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input.clone()),
                Ok((expected_rem, expected_output))
            );
        }

        {
            let input_vec = vec![GCString::new("*here is italic")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("*here is italic")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("here is italic*")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("here is italic*")]),
                    code: ErrorKind::Fail,
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("here is italic")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("here is italic")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("*")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("*")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("**")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("**")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("**we are doing bold**")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_underscore_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new(
                        "**we are doing bold**"
                    )]),
                    code: ErrorKind::Fail
                }))
            );
        }
    }

    /// These are these tests for stars.
    #[test]
    fn test_parse_fragment_bold() {
        {
            let input_vec = vec![GCString::new("*here is bold*")];
            let input = AsStrSlice::from(&input_vec);

            let expected_output_vec = vec![GCString::new("here is bold")];
            let expected_output = AsStrSlice::from(&expected_output_vec);

            let expected_rem_vec = vec![GCString::new("")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            assert_eq2!(
                parse_fragment_starts_with_star_err_on_new_line(input.clone()),
                Ok((expected_rem, expected_output))
            );
        }

        {
            let input_vec = vec![GCString::new("*here is bold")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_star_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("*here is bold")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("here is bold*")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_star_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("here is bold*")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("here is bold")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_star_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("here is bold")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("*")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_star_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("*")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_star_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("")]),
                    code: ErrorKind::Fail
                }))
            );
        }
    }

    /// These are tests for backticks.
    #[test]
    fn test_parse_fragment_inline_code() {
        {
            let input_vec = vec![GCString::new("`here is code")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_backtick_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("`here is code")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("here is code`")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_backtick_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("here is code`")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("``")];
            let input = AsStrSlice::from(&input_vec);

            let expected_output_vec = vec![GCString::new("")];
            let expected_output = AsStrSlice::from(&expected_output_vec);

            let expected_rem_vec = vec![GCString::new("")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            assert_eq2!(
                parse_fragment_starts_with_backtick_err_on_new_line(input),
                Ok((expected_rem, expected_output))
            );
        }

        {
            let input_vec = vec![GCString::new("`")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_backtick_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("`")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_backtick_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("")]),
                    code: ErrorKind::Fail
                }))
            );
        }

        {
            let input_vec = vec![GCString::new("`abcd`")];
            let input = AsStrSlice::from(&input_vec);

            let expected_output_vec = vec![GCString::new("abcd")];
            let expected_output = AsStrSlice::from(&expected_output_vec);

            let expected_rem_vec = vec![GCString::new("")];
            let expected_rem = AsStrSlice::from(&expected_rem_vec);

            assert_eq2!(
                parse_fragment_starts_with_backtick_err_on_new_line(input),
                Ok((expected_rem, expected_output))
            );
        }

        {
            let input_vec = vec![GCString::new("```")];
            let input = AsStrSlice::from(&input_vec);

            assert_eq2!(
                parse_fragment_starts_with_backtick_err_on_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("```")]),
                    code: ErrorKind::Tag
                }))
            );
        }
    }

    #[test]
    fn test_parse_fragment_link() {
        assert_eq2!(
            parse_fragment_starts_with_left_link_err_on_new_line(AsStrSlice::from(
                &vec![GCString::new("[title](https://www.example.com)")]
            )),
            Ok((
                /* rem */ AsStrSlice::from(&vec![GCString::new("")]),
                /* output */ HyperlinkData::new("title", "https://www.example.com")
            ))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line(AsStrSlice::from(&vec![
                GCString::new("")
            ])),
            Err(NomErr::Error(Error {
                input: AsStrSlice::from(&vec![GCString::new("")]),
                code: ErrorKind::Fail
            }))
        );
    }

    #[test]
    fn test_parse_fragment_image() {
        assert_eq2!(
            parse_fragment_starts_with_left_image_err_on_new_line(AsStrSlice::from(
                &vec![GCString::new("![alt text](image.jpg)")]
            )),
            Ok((
                /* rem */ AsStrSlice::from(&vec![GCString::new("")]),
                /* output */ HyperlinkData::new("alt text", "image.jpg")
            ))
        );
        assert_eq2!(
            parse_fragment_starts_with_backtick_err_on_new_line(AsStrSlice::from(&vec![
                GCString::new("")
            ])),
            Err(NomErr::Error(Error {
                input: AsStrSlice::from(&vec![GCString::new("")]),
                code: ErrorKind::Fail
            }))
        );
    }

    #[test]
    fn test_parse_fragment_plaintext_unicode() {
        let input_vec = vec![GCString::new("- straight😃\n")];
        let input = AsStrSlice::from(&input_vec);
        let result = parse_fragment_plain_text_no_new_line(input);
        let remainder = &result.as_ref().unwrap().0;
        let output = &result.as_ref().unwrap().1;
        assert_eq2!(remainder.extract_remaining_text_content_in_line(), "\n");
        assert_eq2!(
            output.extract_remaining_text_content_in_line(),
            "- straight😃"
        );
    }

    #[test]
    fn test_parse_fragment_plaintext() {
        // Test case 1
        {
            let input_vec = vec![GCString::new("1234567890")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("1234567890")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 2
        {
            let input_vec = vec![GCString::new("oh my gosh!")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("oh my gosh!")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 3
        {
            let input_vec = vec![GCString::new("oh my gosh![")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let rem_vec = vec![GCString::new("![")];
            let rem_slice = AsStrSlice::from(&rem_vec);
            let expected_output_vec = vec![GCString::new("oh my gosh")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((rem_slice, expected_output)));
        }

        // Test case 4
        {
            let input_vec = vec![GCString::new("oh my gosh!*")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let rem_vec = vec![GCString::new("*")];
            let rem_slice = AsStrSlice::from(&rem_vec);
            let expected_output_vec = vec![GCString::new("oh my gosh!")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((rem_slice, expected_output)));
        }

        // Test case 5
        {
            let input_vec = vec![GCString::new("*bold baby bold*")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("*bold baby bold*")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 6
        {
            let input_vec = vec![GCString::new("[link baby](and then somewhat)")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec =
                vec![GCString::new("[link baby](and then somewhat)")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 7
        {
            let input_vec = vec![GCString::new("`codeblock for bums`")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("`codeblock for bums`")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 8
        {
            let input_vec = vec![GCString::new("![ but wait theres more](jk)")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("![ but wait theres more](jk)")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 9
        {
            let input_vec = vec![GCString::new("here is plaintext")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("here is plaintext")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 10
        {
            let input_vec = vec![GCString::new("here is plaintext!")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("here is plaintext!")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 11
        {
            let input_vec = vec![GCString::new("here is plaintext![image starting")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let rem_vec = vec![GCString::new("![image starting")];
            let rem_slice = AsStrSlice::from(&rem_vec);
            let expected_output_vec = vec![GCString::new("here is plaintext")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((rem_slice, expected_output)));
        }

        // Test case 12
        {
            let input_vec = vec![GCString::new("here is plaintext\n")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let rem_vec = vec![GCString::new("\n")];
            let rem_slice = AsStrSlice::from(&rem_vec);
            let expected_output_vec = vec![GCString::new("here is plaintext")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((rem_slice, expected_output)));
        }

        // Test case 13
        {
            let input_vec = vec![GCString::new("*here is italic*")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("*here is italic*")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 14
        {
            let input_vec = vec![GCString::new("**here is bold**")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("**here is bold**")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 15
        {
            let input_vec = vec![GCString::new("`here is code`")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("`here is code`")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 16
        {
            let input_vec = vec![GCString::new("[title](https://www.example.com)")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec =
                vec![GCString::new("[title](https://www.example.com)")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 17
        {
            let input_vec = vec![GCString::new("![alt text](image.jpg)")];
            let input = AsStrSlice::from(&input_vec);
            let result = parse_fragment_plain_text_no_new_line(input);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            let expected_output_vec = vec![GCString::new("![alt text](image.jpg)")];
            let expected_output = AsStrSlice::from(&expected_output_vec);
            assert_eq2!(result, Ok((empty_slice, expected_output)));
        }

        // Test case 18
        {
            let input_vec = vec![GCString::new("")];
            let input = AsStrSlice::from(&input_vec);
            assert_eq2!(
                parse_fragment_plain_text_no_new_line(input),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&vec![GCString::new("")]),
                    code: ErrorKind::Eof
                }))
            );
        }
    }

    #[test]
    fn test_parse_fragment_markdown_inline() {
        // Test case 1: Plain text
        {
            let input_vec = vec![GCString::new("here is plaintext!")];
            let input = AsStrSlice::from(&input_vec);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    input,
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((
                    /* rem */ empty_slice,
                    /* output */ MdLineFragment::Plain("here is plaintext!")
                ))
            );
        }

        // Test case 2: Bold text
        {
            let input_vec = vec![GCString::new("*here is bold*")];
            let input = AsStrSlice::from(&input_vec);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    input,
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((empty_slice, MdLineFragment::Bold("here is bold")))
            );
        }

        // Test case 3: Italic text
        {
            let input_vec = vec![GCString::new("_here is italic_")];
            let input = AsStrSlice::from(&input_vec);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    input,
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((empty_slice, MdLineFragment::Italic("here is italic")))
            );
        }

        // Test case 4: Inline code
        {
            let input_vec = vec![GCString::new("`here is code`")];
            let input = AsStrSlice::from(&input_vec);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    input,
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((empty_slice, MdLineFragment::InlineCode("here is code")))
            );
        }

        // Test case 5: Link
        {
            let input_vec = vec![GCString::new("[title](https://www.example.com)")];
            let input = AsStrSlice::from(&input_vec);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    input,
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((
                    empty_slice,
                    MdLineFragment::Link(HyperlinkData::new(
                        "title",
                        "https://www.example.com"
                    ))
                ))
            );
        }

        // Test case 6: Image
        {
            let input_vec = vec![GCString::new("![alt text](image.jpg)")];
            let input = AsStrSlice::from(&input_vec);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    input,
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((
                    empty_slice,
                    MdLineFragment::Image(HyperlinkData::new("alt text", "image.jpg"))
                ))
            );
        }

        // Test case 7: Plain text (duplicate for consistency)
        {
            let input_vec = vec![GCString::new("here is plaintext!")];
            let input = AsStrSlice::from(&input_vec);
            let empty_vec = vec![GCString::new("")];
            let empty_slice = AsStrSlice::from(&empty_vec);
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    input,
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((empty_slice, MdLineFragment::Plain("here is plaintext!")))
            );
        }

        // Test case 8: Partial parsing - plaintext with remaining content
        {
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&vec![GCString::new(
                        "here is some plaintext *but what if we italicize?"
                    )]),
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((
                    AsStrSlice::from(&vec![GCString::new("*but what if we italicize?")]),
                    MdLineFragment::Plain("here is some plaintext ")
                ))
            );
        }

        // Test case 9: Partial parsing with newline - plaintext with remaining content
        {
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&vec![GCString::new(
                        "here is some plaintext \n*but what if we italicize?"
                    )]),
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((
                    AsStrSlice::from(&vec![GCString::new(
                        "\n*but what if we italicize?"
                    )]),
                    MdLineFragment::Plain("here is some plaintext ")
                ))
            );
        }

        // Test case 10: Error case - newline only
        {
            let newline_vec = vec![GCString::new("\n")];
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&newline_vec),
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&newline_vec),
                    code: ErrorKind::Not
                }))
            );
        }

        // Test case 11: Error case - empty string
        {
            let empty_vec = vec![GCString::new("")];
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&empty_vec),
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&empty_vec),
                    code: ErrorKind::Eof
                }))
            );
        }

        // Test case 12: Checkbox parsing - unchecked, ignore policy
        {
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&vec![GCString::new("[ ] this is a checkbox")]),
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((
                    AsStrSlice::from(&vec![GCString::new(" this is a checkbox")]),
                    MdLineFragment::Plain("[ ]")
                ))
            );
        }

        // Test case 13: Checkbox parsing - checked, ignore policy
        {
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&vec![GCString::new("[x] this is a checkbox")]),
                    CheckboxParsePolicy::IgnoreCheckbox
                ),
                Ok((
                    AsStrSlice::from(&vec![GCString::new(" this is a checkbox")]),
                    MdLineFragment::Plain("[x]")
                ))
            );
        }

        // Test case 14: Checkbox parsing - unchecked, parse policy
        {
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&vec![GCString::new("[ ] this is a checkbox")]),
                    CheckboxParsePolicy::ParseCheckbox
                ),
                Ok((
                    AsStrSlice::from(&vec![GCString::new(" this is a checkbox")]),
                    MdLineFragment::Checkbox(false)
                ))
            );
        }

        // Test case 15: Checkbox parsing - checked, parse policy
        {
            assert_eq2!(
                parse_inline_fragments_until_eol_or_eoi(
                    AsStrSlice::from(&vec![GCString::new("[x] this is a checkbox")]),
                    CheckboxParsePolicy::ParseCheckbox
                ),
                Ok((
                    AsStrSlice::from(&vec![GCString::new(" this is a checkbox")]),
                    MdLineFragment::Checkbox(true)
                ))
            );
        }
    }
}

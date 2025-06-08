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

use super::{parse_fragment_plain_text_until_eol_or_eoi_alt,
            parse_fragment_starts_with_backtick_err_on_new_line_alt,
            parse_fragment_starts_with_checkbox_checkbox_into_bool_alt,
            parse_fragment_starts_with_checkbox_into_str_alt,
            parse_fragment_starts_with_left_image_err_on_new_line_alt,
            parse_fragment_starts_with_left_link_err_on_new_line_alt,
            parse_fragment_starts_with_star_err_on_new_line_alt,
            parse_fragment_starts_with_underscore_err_on_new_line_alt};
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
pub fn parse_inline_fragments_until_eol_or_eoi_alt<'a>(
    input: AsStrSlice<'a>,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<AsStrSlice<'a>, MdLineFragment<'a>> {
    // The order of the following parsers is important. The highest priority parser is at
    // the top. The lowest priority parser is at the bottom. This is because the first
    // parser that matches will be the one that is used.

    // Clone the input to avoid ownership issues
    let input_clone = input.clone();

    let it = match checkbox_policy {
        CheckboxParsePolicy::IgnoreCheckbox => alt((
            map(parse_fragment_starts_with_underscore_err_on_new_line_alt,  |s| MdLineFragment::Italic(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_star_err_on_new_line_alt,        |s| MdLineFragment::Bold(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_backtick_err_on_new_line_alt,    |s| MdLineFragment::InlineCode(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_left_image_err_on_new_line_alt,  MdLineFragment::Image),
            map(parse_fragment_starts_with_left_link_err_on_new_line_alt,   MdLineFragment::Link),
            map(parse_fragment_starts_with_checkbox_into_str_alt,           |s| MdLineFragment::Plain(s.extract_remaining_text_content_in_line())), // This line is different.
            map(parse_fragment_plain_text_until_eol_or_eoi_alt,                  |s| MdLineFragment::Plain(s.extract_remaining_text_content_in_line())),
        )).parse(input_clone.clone()),
        CheckboxParsePolicy::ParseCheckbox => alt((
            map(parse_fragment_starts_with_underscore_err_on_new_line_alt,  |s| MdLineFragment::Italic(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_star_err_on_new_line_alt,       |s| MdLineFragment::Bold(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_backtick_err_on_new_line_alt,    |s| MdLineFragment::InlineCode(s.extract_remaining_text_content_in_line())),
            map(parse_fragment_starts_with_left_image_err_on_new_line_alt,  MdLineFragment::Image),
            map(parse_fragment_starts_with_left_link_err_on_new_line_alt,   MdLineFragment::Link),
            map(parse_fragment_starts_with_checkbox_checkbox_into_bool_alt, MdLineFragment::Checkbox), // This line is different.
            map(parse_fragment_plain_text_until_eol_or_eoi_alt,                 |s| MdLineFragment::Plain(s.extract_remaining_text_content_in_line())),
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
            let input_raw = "this _bar";
            let lines = &[GCString::new(input_raw)];
            let input = AsStrSlice::from(lines);

            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            let (rem, out) = res.unwrap();
            assert_eq2!(rem.extract_remaining_text_content_in_line(), "_bar");
            assert_eq2!(out.extract_remaining_text_content_in_line(), "this ");
        }

        {
            let input_raw = "_bar";
            let lines = &[GCString::new(input_raw)];
            let input = AsStrSlice::from(lines);

            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            let (rem, out) = res.unwrap();
            assert_eq2!(rem.extract_remaining_text_content_in_line(), "bar");
            assert_eq2!(out.extract_remaining_text_content_in_line(), "_");
        }

        {
            let input_raw = "bar_";
            let lines = &[GCString::new(input_raw)];
            let input = AsStrSlice::from(lines);

            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            let (rem, out) = res.unwrap();
            assert_eq2!(rem.extract_remaining_text_content_in_line(), "_");
            assert_eq2!(out.extract_remaining_text_content_in_line(), "bar");
        }
    }

    #[test]
    fn test_parse_fragment_plaintext_unicode() {
        let input_raw = "- straight😃\n";
        let lines = &[GCString::new(input_raw)];
        let input = AsStrSlice::from(lines);

        let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
        let (rem, out) = res.unwrap();

        // println!("rem: {:#?}", rem);
        println!(
            "rem.extract_remaining_text_content_in_line(): {:#?}",
            rem.extract_remaining_text_content_in_line()
        );

        // println!("out: {:#?}", out);
        println!(
            "out.extract_remaining_text_content_in_line(): {:#?}",
            out.extract_remaining_text_content_in_line()
        );

        assert_eq2!(rem.lines.len(), 1);
        assert_eq2!(rem.line_index, 0);
        assert_eq2!(rem.extract_remaining_text_content_in_line(), "\n");

        assert_eq2!(out.lines.len(), 1);
        assert_eq2!(out.line_index, 0);
        assert_eq2!(out.extract_remaining_text_content_in_line(), "- straight😃");
    }

    #[test]
    fn test_parse_fragment_plaintext() {
        // "1234567890" -> ok
        {
            let lines = &[GCString::new("1234567890")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "1234567890"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "oh my gosh!" -> ok
        {
            let lines = &[GCString::new("oh my gosh!")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "oh my gosh!"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "oh my gosh![" -> ok
        {
            let lines = &[GCString::new("oh my gosh![")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "![");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "oh my gosh"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "oh my gosh!*" -> ok
        {
            let lines = &[GCString::new("oh my gosh!*")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "*");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "oh my gosh!"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "*bold baby bold*" -> ok
        {
            let lines = &[GCString::new("*bold baby bold*")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "*bold baby bold*"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "[link baby](and then somewhat)" -> ok
        {
            let lines = &[GCString::new("[link baby](and then somewhat)")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "[link baby](and then somewhat)"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "`codeblock for bums`" -> ok
        {
            let lines = &[GCString::new("`codeblock for bums`")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "`codeblock for bums`"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "![ but wait theres more](jk)" -> ok
        {
            let lines = &[GCString::new("![ but wait theres more](jk)")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "![ but wait theres more](jk)"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "here is plaintext" -> ok
        {
            let lines = &[GCString::new("here is plaintext")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "here is plaintext"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "here is plaintext!" -> ok
        {
            let lines = &[GCString::new("here is plaintext!")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "here is plaintext!"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "here is plaintext![image starting" -> ok
        {
            let lines = &[GCString::new("here is plaintext![image starting")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(
                        rem.extract_remaining_text_content_in_line(),
                        "![image starting"
                    );
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "here is plaintext"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "here is plaintext\n" -> ok
        {
            let lines = &[GCString::new("here is plaintext\n")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "\n");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "here is plaintext"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "*here is italic*" -> ok
        {
            let lines = &[GCString::new("*here is italic*")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "*here is italic*"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "**here is bold**" -> ok
        {
            let lines = &[GCString::new("**here is bold**")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "**here is bold**"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "`here is code`" -> ok
        {
            let lines = &[GCString::new("`here is code`")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "`here is code`"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "[title](https://www.example.com)" -> ok
        {
            let lines = &[GCString::new("[title](https://www.example.com)")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "[title](https://www.example.com)"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "![alt text](image.jpg)" -> ok
        {
            let lines = &[GCString::new("![alt text](image.jpg)")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out.extract_remaining_text_content_in_line(),
                        "![alt text](image.jpg)"
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // "" -> error
        {
            let lines = &[GCString::new("")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_plain_text_until_eol_or_eoi_alt(input);
            assert_eq2!(
                res,
                Err(NomErr::Error(Error {
                    input: AsStrSlice::from(&[GCString::new("")]),
                    code: ErrorKind::Eof
                }))
            );
        }
    }

    #[test]
    fn test_parse_fragment_markdown_inline() {
        // Plain text.
        // "here is plaintext!" -> ok
        {
            let lines = &[GCString::new("here is plaintext!")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(out, MdLineFragment::Plain("here is plaintext!"));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Bold text.
        // "*here is bold*" -> ok
        {
            let lines = &[GCString::new("*here is bold*")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(out, MdLineFragment::Bold("here is bold"));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Italic text.
        // "_here is italic_" -> ok
        {
            let lines = &[GCString::new("_here is italic_")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(out, MdLineFragment::Italic("here is italic"));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Inline code.
        // "`here is code`" -> ok
        {
            let lines = &[GCString::new("`here is code`")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(out, MdLineFragment::InlineCode("here is code"));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Link.
        // "[title](https://www.example.com)" -> ok
        {
            let lines = &[GCString::new("[title](https://www.example.com)")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out,
                        MdLineFragment::Link(HyperlinkData {
                            text: "title",
                            url: "https://www.example.com"
                        })
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Image.
        // "![alt text](image.jpg)" -> ok
        {
            let lines = &[GCString::new("![alt text](image.jpg)")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(
                        out,
                        MdLineFragment::Image(HyperlinkData {
                            text: "alt text",
                            url: "image.jpg"
                        })
                    );
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Plain text (duplicate for consistency).
        // "here is plaintext!" -> ok
        {
            let lines = &[GCString::new("here is plaintext!")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(out, MdLineFragment::Plain("here is plaintext!"));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Partial parsing - plaintext with remaining content.
        // "here is some plaintext *but what if we italicize?" -> ok
        {
            let lines = &[GCString::new(
                "here is some plaintext *but what if we italicize?",
            )];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(
                        rem.extract_remaining_text_content_in_line(),
                        "*but what if we italicize?"
                    );
                    assert_eq2!(out, MdLineFragment::Plain("here is some plaintext "));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Partial parsing with newline - plaintext with remaining content.
        // "here is some plaintext \n*but what if we italicize?" -> ok
        {
            let lines = &[GCString::new(
                "here is some plaintext \n*but what if we italicize?",
            )];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(
                        rem.extract_remaining_text_content_in_line(),
                        "\n*but what if we italicize?"
                    );
                    assert_eq2!(out, MdLineFragment::Plain("here is some plaintext "));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Error case - newline only.
        // "\n" -> error
        {
            let lines = &[GCString::new("\n")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err(err) => {
                    // Expected error case
                    assert!(matches!(err, NomErr::Error(_)));
                }
            }
        }

        // Error case - empty string.
        // "" -> error
        {
            let lines = &[GCString::new("")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err(err) => {
                    // Expected error case
                    assert!(matches!(err, NomErr::Error(_)));
                }
            }
        }

        // Checkbox parsing - unchecked, ignore policy.
        // "[ ] this is a checkbox" -> ok
        {
            let lines = &[GCString::new("[ ] this is a checkbox")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(
                        rem.extract_remaining_text_content_in_line(),
                        " this is a checkbox"
                    );
                    assert_eq2!(out, MdLineFragment::Plain("[ ]"));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Checkbox parsing - checked, ignore policy.
        // "[x] this is a checkbox" -> ok
        {
            let lines = &[GCString::new("[x] this is a checkbox")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(
                        rem.extract_remaining_text_content_in_line(),
                        " this is a checkbox"
                    );
                    assert_eq2!(out, MdLineFragment::Plain("[x]"));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Checkbox parsing - unchecked, parse policy.
        // "[ ] this is a checkbox" -> ok
        {
            let lines = &[GCString::new("[ ] this is a checkbox")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::ParseCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(
                        rem.extract_remaining_text_content_in_line(),
                        " this is a checkbox"
                    );
                    assert_eq2!(out, MdLineFragment::Checkbox(false));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }

        // Checkbox parsing - checked, parse policy.
        // "[x] this is a checkbox" -> ok
        {
            let lines = &[GCString::new("[x] this is a checkbox")];
            let input = AsStrSlice::from(lines);
            let res = parse_inline_fragments_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::ParseCheckbox,
            );
            match res {
                Ok((rem, out)) => {
                    assert_eq2!(
                        rem.extract_remaining_text_content_in_line(),
                        " this is a checkbox"
                    );
                    assert_eq2!(out, MdLineFragment::Checkbox(true));
                }
                Err(err) => panic!("Expected Ok, got Err: {:?}", err),
            }
        }
    }
}

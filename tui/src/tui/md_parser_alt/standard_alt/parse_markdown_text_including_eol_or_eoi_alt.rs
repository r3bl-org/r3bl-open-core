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
use nom::{multi::many0, IResult, Parser};

use crate::{constants::NEW_LINE,
            md_parser_types::CheckboxParsePolicy,
            parse_inline_fragments_until_eol_or_eoi_alt,
            AsStrSlice,
            List,
            MdLineFragments,
            NErr,
            NError,
            NErrorKind};

/// Parse a single line of markdown text with inline formatting and automatic line
/// advancement.
///
/// ## Purpose
/// This is the **primary text parser** for single-line markdown content in the R3BL TUI
/// editor. It handles inline markdown formatting (bold, italic, links, code spans, etc.)
/// within a single line and is the fallback parser that processes regular text content
/// when no other specialized parsers (headings, lists, code blocks) match.
///
/// ## Input Format and Validation
/// - Expects input from [`str::lines()`] output (no embedded newlines within the line)
/// - **Rejects empty input** to prevent infinite loops in the main parser chain
/// - **Validates no newline characters** exist in the current line content
/// - Designed to work with [`AsStrSlice`] character-based indexing for Unicode safety
///
/// ## Parser Ordering
/// This parser must come **after** the empty line parser
/// ([`crate::parse_line_empty_advance_alt`]) in the parser chain because
/// this text parser explicitly rejects empty input to prevent infinite loops. The empty
/// line parser handles both completely empty lines and whitespace-only lines, allowing
/// this text parser to focus solely on non-empty content. This ordering ensures proper
/// whitespace handling throughout the document structure.
///
/// ## Inline Formatting Supported
/// Parses and returns [`MdLineFragments`] containing:
/// - **Plain text**: Regular content without formatting
/// - **Bold text**: `**bold**` or `__bold__` syntax
/// - **Italic text**: `*italic*` or `_italic_` syntax
/// - **Code spans**: `` `code` `` syntax
/// - **Links**: `[text](url)` syntax
/// - **Complex combinations**: Mixed formatting within the same line
///
/// ## Line Advancement
/// This is a **single-line parser with automatic advancement**. After successfully
/// parsing the current line's content, it automatically advances to the next line,
/// making it consistent with other auto-advancing parsers in the parser chain.
///
/// ## Parser Chain Integration
/// Works as part of the [`many0(alt(...))`](nom) parser chain in
/// [`crate::parse_markdown_alt()`], typically as one of the last
/// parsers to catch general text content that doesn't match specialized patterns.
///
/// ## Unicode and Performance
/// - **Unicode-safe**: Proper handling of emojis and multi-byte UTF-8 characters
/// - **Zero-allocation parsing**: Uses [`AsStrSlice`] for efficient memory access
/// - **Character-based indexing**: Avoids byte-level operations on UTF-8 strings
///
/// ## Examples
/// ```text
/// "Hello **world** with `code`" → [Plain("Hello "), Bold("world"), Plain(" with "), Code("code")]
/// "Simple text line" → [Plain("Simple text line")]
/// "*italic* and **bold**" → [Italic("italic"), Plain(" and "), Bold("bold")]
/// ```
///
/// ## Error Handling
/// Returns `Err` if:
/// - Input is empty (prevents infinite loops)
/// - Current line contains newline characters (invalid input format)
/// - Internal fragment parsing fails
///
/// ## Related Functions
/// - [`crate::parse_line_empty_advance_alt`]: Handles empty/whitespace-only lines
/// - [`parse_inline_fragments_until_eol_or_eoi_alt`]: Internal fragment parsing logic
/// - [`crate::parse_markdown_alt()`]: Main parser that orchestrates all text parsing
pub fn parse_line_text_advance_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    let current_line_contains_new_line = input.extract_to_line_end().contains(NEW_LINE);
    if current_line_contains_new_line {
        // Throw error for invalid input.
        return Err(NErr::Error(NError::new(input, NErrorKind::CrLf)));
    } else {
        // This checks for empty input.
        inner::without_new_line(input)
    }
}

mod inner {
    use super::*;

    /// Parse a single line of markdown text [MdLineFragments] not terminated by EOL [NEW_LINE].
    #[rustfmt::skip]
    pub fn without_new_line<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
        // Check if input is empty.
        if input.is_empty() {
            return Err(NErr::Error(NError::new(input, NErrorKind::Eof)));
        }

        let (input, output) = many0(
            |it| parse_inline_fragments_until_eol_or_eoi_alt(it, CheckboxParsePolicy::IgnoreCheckbox)
        ).parse(input)?;

        let it = List::from(output);

        Ok((input, it))
    }
}

// XMARK: Great tests to understand how a single line of Markdown text is parsed

#[cfg(test)]
mod tests_inner_without_new_line {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, list, MdLineFragment};

    #[test]
    fn test_parse_multiple_plain_text_fragments_in_single_line() {
        {
            as_str_slice_test_case!(input, "this _bar");
            let result = inner::without_new_line(input);
            println!("result: {result:#?}");

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                output,
                list![
                    MdLineFragment::Plain("this "),
                    MdLineFragment::Plain("_"),
                    MdLineFragment::Plain("bar"),
                ]
            );
        }
    }

    #[test]
    fn test_parse_block_markdown_text_without_eol() {
        {
            as_str_slice_test_case!(input, "here is some plaintext");
            let result = inner::without_new_line(input);

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                output,
                list![MdLineFragment::Plain("here is some plaintext")]
            );
        }

        {
            as_str_slice_test_case!(
                input,
                "here is some plaintext *but what if we bold?*"
            );
            let result = inner::without_new_line(input);

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                output,
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Bold("but what if we bold?"),
                ]
            );
        }

        {
            as_str_slice_test_case!(input, "here is some plaintext *but what if we bold?* I guess it doesn't **matter** in my `code`");
            let result = inner::without_new_line(input);

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                output,
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Bold("but what if we bold?"),
                    MdLineFragment::Plain(" I guess it doesn't "),
                    MdLineFragment::Bold(""),
                    MdLineFragment::Plain("matter"),
                    MdLineFragment::Bold(""),
                    MdLineFragment::Plain(" in my "),
                    MdLineFragment::InlineCode("code"),
                ]
            );
        }

        {
            as_str_slice_test_case!(
                input,
                "here is some plaintext _but what if we italic?_"
            );
            let result = inner::without_new_line(input);

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                output,
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Italic("but what if we italic?"),
                ]
            );
        }

        {
            as_str_slice_test_case!(input, "this!");
            let result = inner::without_new_line(input);

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(output, list![MdLineFragment::Plain("this!")]);
        }
    }

    #[test]
    fn test_empty_input_error() {
        {
            as_str_slice_test_case!(input, "");
            let result = inner::without_new_line(input);

            assert_eq2!(result.is_err(), true);
        }
    }
}

#[cfg(test)]
mod tests_parse_markdown_text_including_eol_or_eoi {
    use super::*;
    use crate::{as_str_slice_test_case,
                assert_eq2,
                list,
                HyperlinkData,
                MdLineFragment};

    #[test]
    fn test_single_line_plain_text() {
        as_str_slice_test_case!(input, "foobar");
        let res = parse_line_text_advance_alt(input);
        let (remainder, fragments) = res.unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(fragments, list![MdLineFragment::Plain("foobar")])
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_1() {
        as_str_slice_test_case!(input, "This is a _hyperlink: [foo](http://google.com).");
        let res = parse_line_text_advance_alt(input);

        let (remainder, fragments) = res.unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            fragments,
            list![
                MdLineFragment::Plain("This is a ",),
                MdLineFragment::Plain("_",),
                MdLineFragment::Plain("hyperlink: ",),
                MdLineFragment::Link(HyperlinkData {
                    text: "foo",
                    url: "http://google.com",
                },),
                MdLineFragment::Plain(".",),
            ]
        );
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_2() {
        as_str_slice_test_case!(input, "This is a *hyperlink: [foo](http://google.com).");
        let res = parse_line_text_advance_alt(input);

        let (remainder, fragments) = res.unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            fragments,
            list![
                MdLineFragment::Plain("This is a ",),
                MdLineFragment::Plain("*",),
                MdLineFragment::Plain("hyperlink: ",),
                MdLineFragment::Link(HyperlinkData {
                    text: "foo",
                    url: "http://google.com",
                },),
                MdLineFragment::Plain(".",),
            ]
        );
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_3_err() {
        as_str_slice_test_case!(input, "this is a * [link](url).\nthis is a monkey");
        let res = parse_line_text_advance_alt(input);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_3_ok() {
        as_str_slice_test_case!(input, "this is a * [link](url).this is a monkey");
        let res = parse_line_text_advance_alt(input);

        let (remainder, fragments) = res.unwrap();

        dbg!(&remainder.is_empty());
        dbg!(&fragments);

        assert!(remainder.is_empty());
        assert_eq2!(
            fragments,
            list![
                MdLineFragment::Plain("this is a ",),
                MdLineFragment::Plain("*",),
                MdLineFragment::Plain(" ",),
                MdLineFragment::Link(HyperlinkData {
                    text: "link",
                    url: "url",
                },),
                MdLineFragment::Plain(".this is a monkey",),
            ]
        );
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_4_err() {
        as_str_slice_test_case!(input, "this is a _ [link](url) *\nthis is a monkey");
        let res = parse_line_text_advance_alt(input);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_4_ok() {
        as_str_slice_test_case!(input, "this is a _ [link](url) *this is a monkey");
        let res = parse_line_text_advance_alt(input);

        let (remainder, fragments) = res.unwrap();

        dbg!(&remainder.is_empty());
        dbg!(&fragments);

        assert!(remainder.is_empty());
        assert_eq2!(
            fragments,
            list![
                MdLineFragment::Plain("this is a ",),
                MdLineFragment::Plain("_",),
                MdLineFragment::Plain(" ",),
                MdLineFragment::Link(HyperlinkData {
                    text: "link",
                    url: "url",
                },),
                MdLineFragment::Plain(" ",),
                MdLineFragment::Plain("*",),
                MdLineFragment::Plain("this is a monkey",),
            ]
        );
    }
}

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

use nom::{bytes::complete::tag, multi::many0, sequence::terminated, IResult, Parser};

use crate::{constants::NEW_LINE,
            md_parser_types::CheckboxParsePolicy,
            parse_inline_fragments_until_eol_or_eoi_alt,
            AsStrSlice,
            List,
            MdLineFragments};

/// Take text until an optional EOL character is found, or end of input is reached.
/// Consumes the [NEW_LINE] if it exists.
pub fn parse_markdown_text_including_eol_or_eoi_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    // Do not use .contains() which materializes the remainder of the entire input.
    // The assumption is that input should be the output of .lines(). In the case
    // there is a "\n" in the input's current line, then this if statement kicks in.
    if input.extract_to_line_end().contains(NEW_LINE) {
        inner::with_new_line(input)
    } else {
        inner::without_new_line(input)
    }
}

mod inner {
    use super::*;

    /// Parse a single line of markdown text [MdLineFragments] terminated by EOL [NEW_LINE].
    #[rustfmt::skip]
    pub fn with_new_line<'a>(
        input: AsStrSlice<'a>,
    ) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
        let (input, output) =
            terminated(
                /* output */
                many0(
                    |it| parse_inline_fragments_until_eol_or_eoi_alt( it, CheckboxParsePolicy::IgnoreCheckbox)
                ),
                /* ends with (discarded) */
                tag(NEW_LINE),
            ).parse(input)?;

        let it = List::from(output);

        Ok((input, it))
    }

    /// Parse a single line of markdown text [MdLineFragments] not terminated by EOL [NEW_LINE].
    #[rustfmt::skip]
    pub fn without_new_line<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
        // Nothing to parse, early return.
        if input.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input, // "Empty input.",
                nom::error::ErrorKind::Fail,
            )));
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
mod tests_inner_with_new_line {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, list, MdLineFragment};

    #[test]
    fn test_parse_multiple_plain_text_fragments_in_single_line() {
        {
            as_str_slice_test_case!(input, "this _bar\n");
            let result = inner::with_new_line(input);
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
    fn test_parse_block_markdown_text_with_eol() {
        {
            as_str_slice_test_case!(input, "\n");
            let result = inner::with_new_line(input);

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(output, list![]);
        }

        {
            as_str_slice_test_case!(input, "here is some plaintext\n");
            let result = inner::with_new_line(input);

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
                "here is some plaintext *but what if we bold?*\n"
            );
            let result = inner::with_new_line(input);

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
            as_str_slice_test_case!(input, "here is some plaintext *but what if we bold?* I guess it doesn't **matter** in my `code`\n");
            let result = inner::with_new_line(input);

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
                "here is some plaintext _but what if we italic?_\n"
            );
            let result = inner::with_new_line(input);

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
            as_str_slice_test_case!(input, "this!\n");
            let result = inner::with_new_line(input);

            let (remainder, output) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(output, list![MdLineFragment::Plain("this!")]);
        }
    }
}

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
    fn test_parse_hyperlink_markdown_text_1() {
        {
            as_str_slice_test_case!(
                input,
                "This is a _hyperlink: [foo](http://google.com)."
            );
            let result = parse_markdown_text_including_eol_or_eoi_alt(input);

            let (remainder, fragments) = result.unwrap();
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
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_2() {
        {
            as_str_slice_test_case!(
                input,
                "This is a *hyperlink: [foo](http://google.com)."
            );
            let result = parse_markdown_text_including_eol_or_eoi_alt(input);

            let (remainder, fragments) = result.unwrap();
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
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_3() {
        {
            as_str_slice_test_case!(
                input,
                "this is a * [link](url).\nthis is a * monkey"
            );
            let result = parse_markdown_text_including_eol_or_eoi_alt(input);

            let (remainder, fragments) = result.unwrap();
            assert_eq2!(
                remainder.extract_to_slice_end().as_ref(),
                "this is a * monkey"
            );
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
                    MdLineFragment::Plain(".",),
                ]
            );
        }
    }

    #[test]
    fn test_parse_hyperlink_markdown_text_4() {
        {
            as_str_slice_test_case!(
                input,
                "this is a _ [link](url) *\nthis is a * monkey"
            );
            let result = parse_markdown_text_including_eol_or_eoi_alt(input);

            let (remainder, fragments) = result.unwrap();
            assert_eq2!(
                remainder.extract_to_slice_end().as_ref(),
                "this is a * monkey"
            );
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
                ]
            );
        }
    }
}

// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{List, MdLineFragments,
            md_parser::constants::{NEW_LINE, NULL_CHAR},
            md_parser_types::CheckboxParsePolicy,
            parse_inline_fragments_until_eol_or_eoi,
            parse_null_padded_line::is};
use nom::{IResult, Parser,
          bytes::complete::{tag, take_while},
          multi::many0,
          sequence::terminated};

/// Parse a markdown text [`crate::FragmentsInOneLine`] in the input (no EOL required).
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0` characters,
/// as provided by `ZeroCopyGapBuffer::as_str()`. The parser handles null padding by terminating
/// fragment parsing at both `\n` and `\0` characters.
///
/// # Errors
///
/// Returns a nom parsing error if the input cannot be parsed as markdown text fragments.
#[rustfmt::skip]
pub fn parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line(
    input: &str,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<&str, MdLineFragments<'_>> {
    let (input, output) = many0(
        |it| parse_inline_fragments_until_eol_or_eoi(it, checkbox_policy)
    ).parse(input)?;

    let it = List::from(output);

    Ok((input, it))
}

/// Parse markdown text blocks with or without new lines.
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. It handles both regular
/// newlines and null-padded lines.
///
/// # Errors
///
/// Returns a nom parsing error if the input cannot be parsed as markdown text.
pub fn parse_block_markdown_text_with_or_without_new_line(
    input: &str,
) -> IResult<&str, MdLineFragments<'_>> {
    if input.contains(NEW_LINE) {
        inner::parse_block_markdown_text_with_new_line(input)
    } else {
        inner::parse_block_markdown_text_without_new_line(input)
    }
}
mod inner {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Parse a single line of markdown text [`crate::FragmentsInOneLine`] terminated by EOL.
    /// # Errors
    ///
    /// Returns a nom parsing error if the input cannot be parsed as markdown text with newline.
    #[rustfmt::skip]
    pub fn parse_block_markdown_text_with_new_line(
        input: &str,
    ) -> IResult<&str, MdLineFragments<'_>> {
        let (input, output) =
            terminated(
                /* output */
                many0(
                    |it| parse_inline_fragments_until_eol_or_eoi( it, CheckboxParsePolicy::IgnoreCheckbox)
                ),
                /* ends with (discarded) */
                (tag(NEW_LINE), /* zero or more */ take_while(is(NULL_CHAR))),
            ).parse(input)?;

        let it = List::from(output);

        Ok((input, it))
    }

    /// Parse a single line of markdown text [`crate::FragmentsInOneLine`] not terminated by EOL.
    #[rustfmt::skip]
    pub fn parse_block_markdown_text_without_new_line(input: &str) -> IResult<&str, MdLineFragments<'_>> {
        // Nothing to parse, early return.
        if input.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                "Empty input.",
                nom::error::ErrorKind::Fail,
            )));
        }

        let (input, output) = many0(
            |it| parse_inline_fragments_until_eol_or_eoi(it, CheckboxParsePolicy::IgnoreCheckbox)
        ).parse(input)?;

        let it = List::from(output);

        Ok((input, it))
    }
}

// XMARK: Great tests to understand how a single line of Markdown text is parsed

#[cfg(test)]
mod tests_parse_block_markdown_text_opt_eol_checkbox_policy {
    use super::*;
    use crate::{MdLineFragment, assert_eq2, list};

    #[test]
    fn test_parse_block_markdown_text_with_checkbox_policy_empty_string() {
        assert_eq2!(
            parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line(
                "",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", list![]))
        );
    }

    #[test]
    fn test_parse_block_markdown_text_with_checkbox_policy_non_empty_string() {
        assert_eq2!(
            parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line(
                "here is some plaintext *but what if we italicize?",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Plain("*"),
                    MdLineFragment::Plain("but what if we italicize?"),
                ]
            ))
        );
    }
}

#[cfg(test)]
mod tests_parse_block_markdown_text_inner {
    use super::*;
    use crate::{MdLineFragment, assert_eq2, list};

    #[test]
    fn test_parse_block_markdown_text_with_new_line() {
        assert_eq2!(
            inner::parse_block_markdown_text_with_new_line("\n"),
            Ok(("", list![]))
        );
        assert_eq2!(
            inner::parse_block_markdown_text_with_new_line("here is some plaintext\n"),
            Ok(("", list![MdLineFragment::Plain("here is some plaintext")]))
        );
        assert_eq2!(
            inner::parse_block_markdown_text_with_new_line(
                "here is some plaintext *but what if we bold?*\n"
            ),
            Ok((
                "",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Bold("but what if we bold?"),
                ]
            ))
        );
        assert_eq2!(
            inner::parse_block_markdown_text_with_new_line(
                "here is some plaintext *but what if we bold?* I guess it doesn't **matter** in my `code`\n"
            ),
            Ok((
                "",
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
            ))
        );
        assert_eq2!(
            inner::parse_block_markdown_text_with_new_line(
                "here is some plaintext _but what if we italic?_\n"
            ),
            Ok((
                "",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Italic("but what if we italic?"),
                ]
            ))
        );
        assert_eq2!(
            inner::parse_block_markdown_text_with_new_line("this!\n"),
            Ok((
                /* remainder */ "",
                /* output */
                list![MdLineFragment::Plain("this!"),]
            ))
        );
    }

    #[test]
    fn test_parse_block_markdown_text_with_multiple_plain_text_fragments() {
        let it = inner::parse_block_markdown_text_with_new_line("this _bar\n");
        println!("it: {it:#?}");
        assert_eq2!(
            it,
            Ok((
                /* remainder */ "",
                /* output */
                list![
                    MdLineFragment::Plain("this "),
                    MdLineFragment::Plain("_"),
                    MdLineFragment::Plain("bar"),
                ]
            ))
        );
    }
}

#[cfg(test)]
mod tests_parse_block_markdown_text {
    use super::*;
    use crate::{HyperlinkData, MdLineFragment, assert_eq2, list};

    #[test]
    fn test_parse_block_markdown_text_with_hyperlink_1() {
        let input = "This is a _hyperlink: [foo](http://google.com).";
        let it = parse_block_markdown_text_with_or_without_new_line(input);
        // println!("it: {:#?}", it);
        assert_eq2!(
            it,
            Ok((
                "",
                list![
                    MdLineFragment::Plain("This is a ",),
                    MdLineFragment::Plain("_",),
                    MdLineFragment::Plain("hyperlink: ",),
                    MdLineFragment::Link(HyperlinkData {
                        text: "foo",
                        url: "http://google.com",
                    },),
                    MdLineFragment::Plain(".",),
                ],
            ))
        );
    }

    #[test]
    fn test_parse_block_markdown_text_with_hyperlink_2() {
        let input = "This is a *hyperlink: [foo](http://google.com).";
        let it = parse_block_markdown_text_with_or_without_new_line(input);
        // println!("it: {:#?}", it);
        assert_eq2!(
            it,
            Ok((
                "",
                list![
                    MdLineFragment::Plain("This is a ",),
                    MdLineFragment::Plain("*",),
                    MdLineFragment::Plain("hyperlink: ",),
                    MdLineFragment::Link(HyperlinkData {
                        text: "foo",
                        url: "http://google.com",
                    },),
                    MdLineFragment::Plain(".",),
                ],
            ))
        );
    }

    #[test]
    fn test_parse_block_markdown_text_with_hyperlink_and_newline_1() {
        let input = "this is a * [link](url).\nthis is a * monkey";
        let it = parse_block_markdown_text_with_or_without_new_line(input);
        // println!("it: {:#?}", it);
        assert_eq2!(
            it,
            Ok((
                "this is a * monkey",
                list![
                    MdLineFragment::Plain("this is a ",),
                    MdLineFragment::Plain("*",),
                    MdLineFragment::Plain(" ",),
                    MdLineFragment::Link(HyperlinkData {
                        text: "link",
                        url: "url",
                    },),
                    MdLineFragment::Plain(".",),
                ],
            ))
        );
    }

    #[test]
    fn test_parse_block_markdown_text_with_hyperlink_and_newline_2() {
        let input = "this is a _ [link](url) *\nthis is a * monkey";
        // println!("input: {:#?}", input);
        let result = parse_block_markdown_text_with_or_without_new_line(input);
        // println!("\n░░ result: {:#?}", result);
        assert_eq2!(
            result,
            Ok((
                "this is a * monkey",
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
                ],
            ))
        );
    }
}

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

use nom::{bytes::complete::tag, multi::many0, sequence::terminated, IResult};

use crate::{constants::NEW_LINE,
            parse_inline_fragments_until_eol_or_eoi,
            List,
            MdLineFragments};

pub fn parse_block_markdown_text_with_or_without_new_line(
    input: &str,
) -> IResult<&str, MdLineFragments<'_>> {
    if input.contains(NEW_LINE) {
        parse_block_markdown_text_with_new_line(input)
    } else {
        parse_block_markdown_text_without_new_line(input)
    }
}

/// Parse a single line of markdown text [crate::FragmentsInOneLine] terminated by EOL.
#[rustfmt::skip]
fn parse_block_markdown_text_with_new_line(
    input: &str,
) -> IResult<&str, MdLineFragments<'_>> {
    let (input, output) =
        terminated(
            /* output */
            many0(
                |it| parse_inline_fragments_until_eol_or_eoi( it, CheckboxParsePolicy::IgnoreCheckbox)
            ),
            /* ends with (discarded) */
            tag(NEW_LINE),
        )(input)?;

    let it = List::from(output);

    Ok((input, it))
}

/// Parse a single line of markdown text [crate::FragmentsInOneLine] not terminated by EOL.
#[rustfmt::skip]
fn parse_block_markdown_text_without_new_line(input: &str) -> IResult<&str, MdLineFragments<'_>> {
    // Nothing to parse, early return.
    if input.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            "Empty input.",
            nom::error::ErrorKind::Fail,
        )));
    }

    let (input, output) = many0(
        |it| parse_inline_fragments_until_eol_or_eoi(it, CheckboxParsePolicy::IgnoreCheckbox)
    )(input)?;

    let it = List::from(output);

    Ok((input, it))
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CheckboxParsePolicy {
    IgnoreCheckbox,
    ParseCheckbox,
}

/// Parse a markdown text [crate::FragmentsInOneLine] in the input (no EOL required).
#[rustfmt::skip]
pub fn parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line(
    input: &str,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<&str, MdLineFragments<'_>> {
    let (input, output) = many0(
        |it| parse_inline_fragments_until_eol_or_eoi(it, checkbox_policy)
    )(input)?;

    let it = List::from(output);

    Ok((input, it))
}

// XMARK: Great tests to understand how a single line of Markdown text is parsed

#[cfg(test)]
mod tests_parse_block_markdown_text_with_or_without_new_line {
    use r3bl_core::assert_eq2;

    use super::*;
    use crate::{list, HyperlinkData, MdLineFragment};

    #[test]
    fn test_parse_hyperlink_markdown_text_1() {
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
    fn test_parse_hyperlink_markdown_text_2() {
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
    fn test_parse_hyperlink_markdown_text_3() {
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
    fn test_parse_hyperlink_markdown_text_4() {
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

#[cfg(test)]
mod tests_parse_block_markdown_text_with_new_line {
    use r3bl_core::assert_eq2;

    use super::*;
    use crate::{list, MdLineFragment};

    #[test]
    fn test_parse_multiple_plain_text_fragments_in_single_line() {
        let it = parse_block_markdown_text_with_new_line("this _bar\n");
        println!("it: {:#?}", it);
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

    #[test]
    fn test_parse_block_markdown_text_with_eol() {
        assert_eq2!(
            parse_block_markdown_text_with_new_line("\n"),
            Ok(("", list![]))
        );
        assert_eq2!(
            parse_block_markdown_text_with_new_line("here is some plaintext\n"),
            Ok(("", list![MdLineFragment::Plain("here is some plaintext")]))
        );
        assert_eq2!(
            parse_block_markdown_text_with_new_line(
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
            parse_block_markdown_text_with_new_line("here is some plaintext *but what if we bold?* I guess it doesn't **matter** in my `code`\n"),
            Ok(
                ("",
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Bold("but what if we bold?"),
                    MdLineFragment::Plain(" I guess it doesn't "),
                    MdLineFragment::Bold(""),
                    MdLineFragment::Plain("matter"),
                    MdLineFragment::Bold(""),
                    MdLineFragment::Plain(" in my "),
                    MdLineFragment::InlineCode("code"),
                ])
            )
        );
        assert_eq2!(
            parse_block_markdown_text_with_new_line(
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
            parse_block_markdown_text_with_new_line("this!\n"),
            Ok((
                /* remainder */ "",
                /* output */
                list![MdLineFragment::Plain("this!"),]
            ))
        );
    }
}

#[cfg(test)]
mod tests_parse_block_markdown_text_opt_eol_with_checkbox_policy {
    use r3bl_core::assert_eq2;

    use super::*;
    use crate::{list, MdLineFragment};

    #[test]
    fn test_ignore_checkbox_empty_string() {
        assert_eq2!(
            parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line(
                "",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", list![]))
        );
    }

    #[test]
    fn test_ignore_checkbox_non_empty_string() {
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

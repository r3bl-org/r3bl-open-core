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

//! An element is a single markdown element. Eg: `**bold**`, `*italic*`, `[link](http://r3bl.com)`,
//! etc. These can be found in every single line of text. These parsers extract each element into
//! either a string slice or some other intermediate representation.

use constants::*;
use crossterm::style::Stylize;
use nom::{branch::*,
          bytes::complete::*,
          character::complete::*,
          combinator::*,
          error::ErrorKind,
          multi::*,
          sequence::*,
          IResult};
use r3bl_rs_utils_core::{call_if_true, log_debug};

use crate::*;

pub fn parse_element_starts_with_star(input: &str) -> IResult<&str, &str> {
    let it = parse_fenced_no_newline(input, BOLD);
    it
}

pub fn parse_element_starts_with_underscore(input: &str) -> IResult<&str, &str> {
    let it = parse_fenced_no_newline(input, ITALIC);
    it
}

#[rustfmt::skip]
pub fn parse_element_starts_with_backtick(input: &str) -> IResult<&str, &str> {
    

    delimited(
        /* start */ tag(BACK_TICK),
        /* output */ is_not(BACK_TICK),
        /* end */ tag(BACK_TICK)
    )(input)
}

#[rustfmt::skip]
pub fn parse_element_starts_with_left_bracket(input: &str) -> IResult<&str, HyperlinkData<'_>> {
    let (input, output) = pair(
        delimited(
            /* start */ tag(LEFT_BRACKET),
            /* output */ is_not(RIGHT_BRACKET),
            /* end */ tag(RIGHT_BRACKET)
        ),
        delimited(
            /* start */ tag(LEFT_PARENTHESIS),
            /* output */ is_not(RIGHT_PARENTHESIS),
            /* end */ tag(RIGHT_PARENTHESIS)
        ),
    )(input)?;
    Ok((input, HyperlinkData::from(output)))
}

/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and images.
/// So some extra hint is need from the code calling this parser to let it know whether to parse
/// a checkbox into plain text, or into a boolean.
#[rustfmt::skip]
pub fn parse_element_checkbox_into_str(input: &str) -> IResult<&str, &str> {
    
    alt((
        recognize(tag(CHECKED)),
        recognize(tag(UNCHECKED))
    ))(input)
}

#[rustfmt::skip]
/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and images.
/// So some extra hint is need from the code calling this parser to let it know whether to parse
/// a checkbox into plain text, or into a boolean.
pub fn parse_element_checkbox_into_bool(input: &str) -> IResult<&str, bool> {
    
    alt((
        map(tag(CHECKED), |_| true),
        map(tag(UNCHECKED), |_| false),
    ))(input)
}

#[rustfmt::skip]
pub fn parse_element_starts_with_left_image(input: &str) -> IResult<&str, HyperlinkData<'_>> {
    let (input, output) =pair(
        delimited(/* start */ tag(LEFT_IMAGE), /* output */ is_not(RIGHT_IMAGE), /* end */ tag(RIGHT_IMAGE)),
        delimited(/* start */ tag(LEFT_PARENTHESIS), /* output */ is_not(RIGHT_PARENTHESIS), /* end */ tag(RIGHT_PARENTHESIS)),
    )(input)?;
    Ok((input, HyperlinkData::from(output)))
}

/// This is the lowest priority parser called by [parse_element_markdown_inline], which
/// itself is called repeatedly in a loop in [parse_block_markdown_text_opt_eol()].
///
/// It will match anything that is not a special character. This is used to parse plain
/// text. However, when it encounters a special character, it will break the input at that
/// character and split the input into two parts: the plain text, and the remainder (after
/// the special character).
///
/// This allows other more specialized parsers to then address these special characters
/// (like italic, bold, links, etc). If these more specialized parsers error out, then
/// this parser will be called again to parse the remainder; this time the input will
/// start with the special character; and in this edge case it will take the input until
/// the first new line; or until the end of the input.
///
/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
#[rustfmt::skip]
pub fn parse_plain_text_no_new_line1(input: &str) -> IResult<&str, &str> {
    // Edge case: If the input starts with [ITALIC, UNDERSCORE, BACKTICK, LEFT_BRACKET,
    // NEW_LINE], then take till the first newline.
    if input.starts_with(ITALIC)
        || input.starts_with(BOLD)
        || input.starts_with(BACK_TICK)
        || input.starts_with(LEFT_BRACKET)
        || input.starts_with(LEFT_IMAGE)
    {
        return take_till1(|c: char|
             c == '\n'
        )(input);
    }

    // Otherwise take till the first special character. And split the input there.
    
    recognize(
        many1(
            preceded(
                /* prefix - discarded */
                not(
                    /* starts with special characters */
                    alt((
                        tag(BOLD),
                        tag(ITALIC),
                        tag(BACK_TICK),
                        tag(LEFT_BRACKET),
                        tag(LEFT_IMAGE),
                        tag(NEW_LINE),
                    ))
                ),
                /* output - keep char */
                anychar,
            )
        )
    )(input)
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
#[rustfmt::skip]
pub fn parse_anychar_in_heading_no_new_line1(input: &str) -> IResult<&str, &str> {
    

    recognize(
        many1(
            preceded(
                /* prefix - discarded */
                not(
                    /* starts with special characters */
                    alt((
                        tag(NEW_LINE),
                    ))
                ),
                /* output - keep char */
                anychar,
            )
        )
    )(input)
}

// BOOKM: parser for a single line of markdown
/// Parse a single chunk of markdown text (found in a single line of text) into a [MdLineFragment].
#[rustfmt::skip]
pub fn parse_element_markdown_inline(
    input: &str,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<&str, MdLineFragment<'_>> {
    let it = match checkbox_policy {
        CheckboxParsePolicy::IgnoreCheckbox => alt((
            map(parse_element_starts_with_underscore, MdLineFragment::Italic),
            map(parse_element_starts_with_star, MdLineFragment::Bold),
            map(parse_element_starts_with_backtick, MdLineFragment::InlineCode),
            map(parse_element_starts_with_left_image, MdLineFragment::Image),
            map(parse_element_starts_with_left_bracket, MdLineFragment::Link),
            map(parse_element_checkbox_into_str, MdLineFragment::Plain),
            map(parse_plain_text_no_new_line1, MdLineFragment::Plain),
        ))(input),
        CheckboxParsePolicy::ParseCheckbox => alt((
            map(parse_element_starts_with_underscore, MdLineFragment::Italic),
            map(parse_element_starts_with_star, MdLineFragment::Bold),
            map(parse_element_starts_with_backtick, MdLineFragment::InlineCode),
            map(parse_element_starts_with_left_image, MdLineFragment::Image),
            map(parse_element_starts_with_left_bracket, MdLineFragment::Link),
            map(parse_element_checkbox_into_bool, MdLineFragment::Checkbox),
            map(parse_plain_text_no_new_line1, MdLineFragment::Plain),
        ))(input)

    };

    call_if_true!(DEBUG_MD_PARSER, {
        log_debug(format!("\n📣📣📣\n input: {:?}", input).green().to_string());
        match it {
            Ok(ref element) => log_debug(format!("✅✅✅ OK {:#?}", element).magenta().to_string()),
            Result::Err(ref error) => log_debug(format!("🟥🟥🟥 NO {:#?}", error)),
        }
    });

    it
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
fn fenced<'a>(
    start: &'a str,
    end: &'a str,
) -> impl FnMut(&'a str) -> IResult<&'a str, &'a str> {
    map(tuple((tag(start), take_until(end), tag(end))), |x| x.1)
}

pub fn parse_fenced_no_newline<'a>(
    input: &'a str,
    delim: &'a str,
) -> IResult<&'a str, &'a str> {
    let it = fenced(delim, delim)(input);
    if let Ok((_, output)) = &it {
        if output.contains(NEW_LINE) {
            return Err(nom::Err::Error(nom::error::Error {
                input: "",
                code: ErrorKind::TakeUntil,
            }));
        };
    }
    it
}

#[cfg(test)]
mod tests_parse_element {
    use nom::{error::Error, Err as NomErr};
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_parse_plain_text_no_new_line1() {
        assert_eq2!(
            parse_plain_text_no_new_line1("this _bar"),
            Ok(("_bar", "this "))
        );

        assert_eq2!(parse_plain_text_no_new_line1("_bar"), Ok(("", "_bar")));

        assert_eq2!(parse_plain_text_no_new_line1("bar_"), Ok(("_", "bar")));
    }

    #[test]
    fn test_parse_fenced_no_newline() {
        let input = "_foo\nbar_";
        let it = parse_fenced_no_newline(input, "_");
        println!("it: {:?}", it);
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeUntil
            }))
        );

        let input = "_foo bar_";
        let it = parse_fenced_no_newline(input, "_");
        println!("it: {:?}", it);
        assert_eq2!(it, Ok(("", "foo bar")));
    }

    #[test]
    fn test_fenced() {
        let input = "_foo bar baz_";
        let it = fenced("_", "_")(input);
        println!("it: {:?}", it);
        assert_eq2!(it, Ok(("", "foo bar baz")));

        let input = "_foo bar baz";
        let it = fenced("_", "_")(input);
        println!("it: {:?}", it);
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: "foo bar baz",
                code: ErrorKind::TakeUntil
            }))
        );

        let input = "foo _bar_ baz";
        let it = fenced("_", "_")(input);
        println!("it: {:?}", it);
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: "foo _bar_ baz",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_checkbox_into_str() {
        assert_eq2!(
            parse_element_checkbox_into_str("[x] here is a checkbox"),
            Ok((" here is a checkbox", "[x]"))
        );

        assert_eq2!(
            parse_element_checkbox_into_str("[ ] here is a checkbox"),
            Ok((" here is a checkbox", "[ ]"))
        );
    }

    #[test]
    fn test_parse_element_checkbox_into_bool() {
        assert_eq2!(
            parse_element_checkbox_into_bool("[x] here is a checkbox"),
            Ok((" here is a checkbox", true))
        );

        assert_eq2!(
            parse_element_checkbox_into_bool("[ ] here is a checkbox"),
            Ok((" here is a checkbox", false))
        );
    }

    #[test]
    fn test_parse_element_italic() {
        assert_eq2!(
            parse_element_starts_with_underscore("_here is italic_"),
            Ok(("", "here is italic"))
        );

        assert_eq2!(
            parse_element_starts_with_underscore("_here is italic_"),
            Ok(("", "here is italic"))
        );

        assert_eq2!(
            parse_element_starts_with_underscore("*here is italic"),
            Err(NomErr::Error(Error {
                input: "*here is italic",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_starts_with_underscore("here is italic*"),
            Err(NomErr::Error(Error {
                input: "here is italic*",
                code: ErrorKind::Tag,
            }))
        );

        assert_eq2!(
            parse_element_starts_with_underscore("here is italic"),
            Err(NomErr::Error(Error {
                input: "here is italic",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_starts_with_underscore("*"),
            Err(NomErr::Error(Error {
                input: "*",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_starts_with_underscore("**"),
            Err(NomErr::Error(Error {
                input: "**",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_starts_with_underscore(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_starts_with_underscore("**we are doing bold**"),
            Err(NomErr::Error(Error {
                input: "**we are doing bold**",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_bold() {
        assert_eq2!(
            parse_element_starts_with_star("*here is bold*"),
            Ok(("", "here is bold"))
        );

        assert_eq2!(
            parse_element_starts_with_star("*here is bold"),
            Err(NomErr::Error(Error {
                input: "here is bold",
                code: ErrorKind::TakeUntil
            }))
        );

        assert_eq2!(
            parse_element_starts_with_star("here is bold*"),
            Err(NomErr::Error(Error {
                input: "here is bold*",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_starts_with_star("here is bold"),
            Err(NomErr::Error(Error {
                input: "here is bold",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_starts_with_star("*"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeUntil
            }))
        );

        assert_eq2!(
            parse_element_starts_with_star(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_code() {
        assert_eq2!(
            parse_element_starts_with_backtick("`here is code"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_element_starts_with_backtick("here is code`"),
            Err(NomErr::Error(Error {
                input: "here is code`",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_element_starts_with_backtick("``"),
            Err(NomErr::Error(Error {
                input: "`",
                code: ErrorKind::IsNot
            }))
        );
        assert_eq2!(
            parse_element_starts_with_backtick("`"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::IsNot
            }))
        );
        assert_eq2!(
            parse_element_starts_with_backtick(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_link() {
        assert_eq2!(
            parse_element_starts_with_left_bracket("[title](https://www.example.com)"),
            Ok(("", HyperlinkData::new("title", "https://www.example.com")))
        );
        assert_eq2!(
            parse_element_starts_with_backtick(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_image() {
        assert_eq2!(
            parse_element_starts_with_left_image("![alt text](image.jpg)"),
            Ok(("", HyperlinkData::new("alt text", "image.jpg")))
        );
        assert_eq2!(
            parse_element_starts_with_backtick(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_plaintext_unicode() {
        let result = parse_plain_text_no_new_line1("- straight😃\n");
        let remainder = result.as_ref().unwrap().0;
        let output = result.as_ref().unwrap().1;
        assert_eq2!(remainder, "\n");
        assert_eq2!(output, "- straight😃");
    }

    #[test]
    fn test_parse_element_plaintext() {
        assert_eq2!(
            parse_plain_text_no_new_line1("1234567890"),
            Ok(("", "1234567890"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("oh my gosh!"),
            Ok(("", "oh my gosh!"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("oh my gosh!["),
            Ok(("![", "oh my gosh"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("oh my gosh!*"),
            Ok(("*", "oh my gosh!"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("*bold baby bold*"),
            Ok(("", "*bold baby bold*"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("[link baby](and then somewhat)"),
            Ok(("", "[link baby](and then somewhat)"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("`codeblock for bums`"),
            Ok(("", "`codeblock for bums`"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("![ but wait theres more](jk)"),
            Ok(("", "![ but wait theres more](jk)"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("here is plaintext"),
            Ok(("", "here is plaintext"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("here is plaintext!"),
            Ok(("", "here is plaintext!"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("here is plaintext![image starting"),
            Ok(("![image starting", "here is plaintext"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("here is plaintext\n"),
            Ok(("\n", "here is plaintext"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("*here is italic*"),
            Ok(("", "*here is italic*"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("**here is bold**"),
            Ok(("", "**here is bold**"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("`here is code`"),
            Ok(("", "`here is code`"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("[title](https://www.example.com)"),
            Ok(("", "[title](https://www.example.com)"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1("![alt text](image.jpg)"),
            Ok(("", "![alt text](image.jpg)"))
        );
        assert_eq2!(
            parse_plain_text_no_new_line1(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        );
    }

    #[test]
    fn test_parse_element_markdown_inline() {
        assert_eq2!(
            parse_element_markdown_inline(
                "*here is bold*",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::Bold("here is bold")))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "_here is italic_",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::Italic("here is italic")))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "`here is code`",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::InlineCode("here is code")))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "[title](https://www.example.com)",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "",
                (MdLineFragment::Link(HyperlinkData::new(
                    "title",
                    "https://www.example.com"
                )))
            ))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "![alt text](image.jpg)",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "",
                (MdLineFragment::Image(HyperlinkData::new("alt text", "image.jpg")))
            ))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "here is plaintext!",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok(("", MdLineFragment::Plain("here is plaintext!")))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "here is some plaintext *but what if we italicize?",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "*but what if we italicize?",
                MdLineFragment::Plain("here is some plaintext ")
            ))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "here is some plaintext \n*but what if we italicize?",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((
                "\n*but what if we italicize?",
                MdLineFragment::Plain("here is some plaintext ")
            ))
        );
        assert_eq2!(
            parse_element_markdown_inline("\n", CheckboxParsePolicy::IgnoreCheckbox),
            Err(NomErr::Error(Error {
                input: "\n",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_markdown_inline("", CheckboxParsePolicy::IgnoreCheckbox),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        );

        // Deal with checkboxes: ignore them.
        assert_eq2!(
            parse_element_markdown_inline(
                "[ ] this is a checkbox",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Plain("[ ]")))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "[x] this is a checkbox",
                CheckboxParsePolicy::IgnoreCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Plain("[x]")))
        );

        // Deal with checkboxes: parse them.
        assert_eq2!(
            parse_element_markdown_inline(
                "[ ] this is a checkbox",
                CheckboxParsePolicy::ParseCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Checkbox(false)))
        );
        assert_eq2!(
            parse_element_markdown_inline(
                "[x] this is a checkbox",
                CheckboxParsePolicy::ParseCheckbox
            ),
            Ok((" this is a checkbox", MdLineFragment::Checkbox(true)))
        );
    }
}

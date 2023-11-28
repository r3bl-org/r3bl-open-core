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

pub fn parse_element_bold(input: &str) -> IResult<&str, &str> {
    parse_fenced_no_newline(input, BOLD)
}

pub fn parse_element_italic(input: &str) -> IResult<&str, &str> {
    parse_fenced_no_newline(input, ITALIC)
}

#[rustfmt::skip]
pub fn parse_element_code(input: &str) -> IResult<&str, &str> {
    delimited(/* start */ tag(BACK_TICK), /* output */ is_not(BACK_TICK), /* end */ tag(BACK_TICK))(input)
}

#[rustfmt::skip]
pub fn parse_element_link(input: &str) -> IResult<&str, HyperlinkData> {
    let (input, output) = pair(
        delimited(/* start */ tag(LEFT_BRACKET), /* output */ is_not(RIGHT_BRACKET), /* end */ tag(RIGHT_BRACKET)),
        delimited(/* start */ tag(LEFT_PARENTHESIS), /* output */ is_not(RIGHT_PARENTHESIS), /* end */ tag(RIGHT_PARENTHESIS)),
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
pub fn parse_element_image(input: &str) -> IResult<&str, HyperlinkData> {
    let (input, output) =pair(
        delimited(/* start */ tag(LEFT_IMAGE), /* output */ is_not(RIGHT_IMAGE), /* end */ tag(RIGHT_IMAGE)),
        delimited(/* start */ tag(LEFT_PARENTHESIS), /* output */ is_not(RIGHT_PARENTHESIS), /* end */ tag(RIGHT_PARENTHESIS)),
    )(input)?;
    Ok((input, HyperlinkData::from(output)))
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
/// There must be at least one match. We want to match many things that are not any of our
/// special tags, but since we have no tools available to match and consume in the negative case
/// (without regex) we need to match against our (start) tags, then consume one char; we repeat
/// this until we run into one of our special characters (start tags) then we return this slice.
#[rustfmt::skip]
pub fn parse_element_plaintext(input: &str) -> IResult<&str, &str> {
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

/// Parse a single chunk of markdown text (found in a single line of text) into a [MdLineFragment].
#[rustfmt::skip]
pub fn parse_element_markdown_inline(
    input: &str,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<&str, MdLineFragment> {
    match checkbox_policy {
        CheckboxParsePolicy::IgnoreCheckbox => alt((
            map(parse_element_italic, MdLineFragment::Italic),
            map(parse_element_bold, MdLineFragment::Bold),
            map(parse_element_code, MdLineFragment::InlineCode),
            map(parse_element_image, MdLineFragment::Image),
            map(parse_element_link, MdLineFragment::Link),
            map(parse_element_checkbox_into_str, MdLineFragment::Plain),
            map(parse_element_plaintext, MdLineFragment::Plain),
        ))(input),
        CheckboxParsePolicy::ParseCheckbox => alt((
            map(parse_element_italic, MdLineFragment::Italic),
            map(parse_element_bold, MdLineFragment::Bold),
            map(parse_element_code, MdLineFragment::InlineCode),
            map(parse_element_image, MdLineFragment::Image),
            map(parse_element_link, MdLineFragment::Link),
            map(parse_element_checkbox_into_bool, MdLineFragment::Checkbox),
            map(parse_element_plaintext, MdLineFragment::Plain),
        ))(input)

    }
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

    call_if_true!(DEBUG_TUI_MOD, {
        log_debug(format!("\n📣📣📣\n input: {:?}", input));
        match it {
            Ok(_) => log_debug(format!("✅✅✅ OK")),
            Result::Err(_) => log_debug(format!("🟥🟥🟥 NO")),
        }
    });

    it
}

#[cfg(test)]
mod tests_parse_element {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

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
            parse_element_italic("_here is italic_"),
            Ok(("", "here is italic"))
        );

        assert_eq2!(
            parse_element_italic("_here is italic_"),
            Ok(("", "here is italic"))
        );

        assert_eq2!(
            parse_element_italic("*here is italic"),
            Err(NomErr::Error(Error {
                input: "*here is italic",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_italic("here is italic*"),
            Err(NomErr::Error(Error {
                input: "here is italic*",
                code: ErrorKind::Tag,
            }))
        );

        assert_eq2!(
            parse_element_italic("here is italic"),
            Err(NomErr::Error(Error {
                input: "here is italic",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_italic("*"),
            Err(NomErr::Error(Error {
                input: "*",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_italic("**"),
            Err(NomErr::Error(Error {
                input: "**",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_italic(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_italic("**we are doing bold**"),
            Err(NomErr::Error(Error {
                input: "**we are doing bold**",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_bold() {
        assert_eq2!(
            parse_element_bold("*here is bold*"),
            Ok(("", "here is bold"))
        );

        assert_eq2!(
            parse_element_bold("*here is bold"),
            Err(NomErr::Error(Error {
                input: "here is bold",
                code: ErrorKind::TakeUntil
            }))
        );

        assert_eq2!(
            parse_element_bold("here is bold*"),
            Err(NomErr::Error(Error {
                input: "here is bold*",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_bold("here is bold"),
            Err(NomErr::Error(Error {
                input: "here is bold",
                code: ErrorKind::Tag
            }))
        );

        assert_eq2!(
            parse_element_bold("*"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeUntil
            }))
        );

        assert_eq2!(
            parse_element_bold(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_code() {
        assert_eq2!(
            parse_element_code("`here is code"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_element_code("here is code`"),
            Err(NomErr::Error(Error {
                input: "here is code`",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_element_code("``"),
            Err(NomErr::Error(Error {
                input: "`",
                code: ErrorKind::IsNot
            }))
        );
        assert_eq2!(
            parse_element_code("`"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::IsNot
            }))
        );
        assert_eq2!(
            parse_element_code(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_link() {
        assert_eq2!(
            parse_element_link("[title](https://www.example.com)"),
            Ok(("", HyperlinkData::new("title", "https://www.example.com")))
        );
        assert_eq2!(
            parse_element_code(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_image() {
        assert_eq2!(
            parse_element_image("![alt text](image.jpg)"),
            Ok(("", HyperlinkData::new("alt text", "image.jpg")))
        );
        assert_eq2!(
            parse_element_code(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_element_plaintext_unicode() {
        let result = parse_element_plaintext("- straight😃\n");
        let remainder = result.as_ref().unwrap().0;
        let output = result.as_ref().unwrap().1;
        assert_eq2!(remainder, "\n");
        assert_eq2!(output, "- straight😃");
    }

    #[test]
    fn test_parse_element_plaintext() {
        assert_eq2!(
            parse_element_plaintext("1234567890"),
            Ok(("", "1234567890"))
        );
        assert_eq2!(
            parse_element_plaintext("oh my gosh!"),
            Ok(("", "oh my gosh!"))
        );
        assert_eq2!(
            parse_element_plaintext("oh my gosh!["),
            Ok(("![", "oh my gosh"))
        );
        assert_eq2!(
            parse_element_plaintext("oh my gosh!*"),
            Ok(("*", "oh my gosh!"))
        );
        assert_eq2!(
            parse_element_plaintext("*bold baby bold*"),
            Err(NomErr::Error(Error {
                input: "*bold baby bold*",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("[link baby](and then somewhat)"),
            Err(NomErr::Error(Error {
                input: "[link baby](and then somewhat)",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("`codeblock for bums`"),
            Err(NomErr::Error(Error {
                input: "`codeblock for bums`",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("![ but wait theres more](jk)"),
            Err(NomErr::Error(Error {
                input: "![ but wait theres more](jk)",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("here is plaintext"),
            Ok(("", "here is plaintext"))
        );
        assert_eq2!(
            parse_element_plaintext("here is plaintext!"),
            Ok(("", "here is plaintext!"))
        );
        assert_eq2!(
            parse_element_plaintext("here is plaintext![image starting"),
            Ok(("![image starting", "here is plaintext"))
        );
        assert_eq2!(
            parse_element_plaintext("here is plaintext\n"),
            Ok(("\n", "here is plaintext"))
        );
        assert_eq2!(
            parse_element_plaintext("*here is italic*"),
            Err(NomErr::Error(Error {
                input: "*here is italic*",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("**here is bold**"),
            Err(NomErr::Error(Error {
                input: "**here is bold**",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("`here is code`"),
            Err(NomErr::Error(Error {
                input: "`here is code`",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("[title](https://www.example.com)"),
            Err(NomErr::Error(Error {
                input: "[title](https://www.example.com)",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext("![alt text](image.jpg)"),
            Err(NomErr::Error(Error {
                input: "![alt text](image.jpg)",
                code: ErrorKind::Not
            }))
        );
        assert_eq2!(
            parse_element_plaintext(""),
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

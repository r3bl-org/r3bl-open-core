// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use nom::{IResult, Parser,
          branch::alt,
          bytes::complete::{tag, take_while, take_while1},
          character::complete::anychar,
          combinator::{map, not, opt, recognize},
          multi::many1,
          sequence::{preceded, terminated}};

use crate::{HeadingData, HeadingLevel,
            md_parser::constants::{self, NEW_LINE, NULL_CHAR, NULL_STR},
            parse_null_padded_line::is};

/// This matches the heading tag and text until EOL. Outputs a tuple of [`HeadingLevel`] and
/// [`crate::FragmentsInOneLine`].
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0` characters,
/// as provided by `ZeroCopyGapBuffer::as_str()`. The parser handles null padding by recognizing
/// `\0` characters as line terminators alongside `\n`.
///
/// # Errors
///
/// Returns a nom parsing error if the input does not match a valid heading pattern.
#[rustfmt::skip]
pub fn parse_heading_in_single_line(input: &str) -> IResult<&str, HeadingData<'_>> {
    let (remainder, output) = parse(input)?;
    Ok((remainder, output))
}

#[rustfmt::skip]
fn parse(input: &str) -> IResult<&str, HeadingData<'_>> {
    let (input, (level, text, _discarded)) = (
        parse_heading_tag,
        parse_anychar_in_heading_no_new_line,
        opt(
            (tag(NEW_LINE), /* zero or more */ take_while(is(NULL_CHAR)))
        ),
    )
        .parse(input)?;
    Ok((input, HeadingData { level, text }))
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
#[rustfmt::skip]
fn parse_anychar_in_heading_no_new_line(input: &str) -> IResult<&str, &str> {
    recognize(
        many1( /* match at least 1 char */
            preceded(
                /* prefix is discarded, it doesn't match anything, only errors out for special chars */
                not( /* error out if starts w/ special chars */
                    alt((
                        tag(NEW_LINE),
                        tag(NULL_STR),
                    ))
                ),
                /* output - keep char */
                anychar,
            )
        )
    ).parse(input)
}

/// Matches one or more `#` chars, consumes it, and outputs [Level].
#[rustfmt::skip]
fn parse_heading_tag(input: &str) -> IResult<&str, HeadingLevel> {
    map(
        terminated(
            /* output `#`+ */ take_while1(|it| it == constants::HEADING_CHAR),
            /* ends with (discarded) */ tag(constants::SPACE),
        ),
        |it: &str|
        HeadingLevel::from(it.len())
    ).parse(input)
}

#[cfg(test)]
mod tests {
    use nom::{Err as NomErr,
              error::{Error, ErrorKind}};

    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_parse_header_tag() {
        assert_eq2!(parse_heading_tag("# "), Ok(("", 1.into())));
        assert_eq2!(parse_heading_tag("### "), Ok(("", 3.into())));
        assert_eq2!(parse_heading_tag("# h1"), Ok(("h1", 1.into())));
        assert_eq2!(parse_heading_tag("# h1"), Ok(("h1", 1.into())));
        assert_eq2!(
            parse_heading_tag(" "),
            Err(NomErr::Error(Error {
                input: " ",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq2!(
            parse_heading_tag("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(parse_heading_tag("####### "), Ok(("", 7.into())));
    }

    #[test]
    fn test_parse_header() {
        assert_eq2!(
            parse_heading_in_single_line("# h1\n"),
            Ok((
                "",
                HeadingData {
                    level: 1.into(),
                    text: "h1",
                }
            ))
        );
        assert_eq2!(
            parse_heading_in_single_line("## h2\n"),
            Ok((
                "",
                HeadingData {
                    level: 2.into(),
                    text: "h2",
                }
            ))
        );
        assert_eq2!(
            parse_heading_in_single_line("###  h3\n"),
            Ok((
                "",
                HeadingData {
                    level: 3.into(),
                    text: " h3",
                }
            ))
        );
        assert_eq2!(
            parse_heading_in_single_line("### h3 *foo* **bar**\n"),
            Ok((
                "",
                HeadingData {
                    level: 3.into(),
                    text: "h3 *foo* **bar**",
                }
            ))
        );
        assert_eq2!(
            parse_heading_in_single_line("###h3"),
            Err(NomErr::Error(Error {
                input: "h3",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_heading_in_single_line("###"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_heading_in_single_line(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq2!(
            parse_heading_in_single_line("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_header_with_new_line() {
        assert_eq2!(
            parse_heading_in_single_line("# \n"),
            Err(NomErr::Error(Error {
                input: "\n",
                code: ErrorKind::Not
            }))
        );
    }

    #[test]
    fn test_parse_header_with_no_new_line() {
        assert_eq2!(
            parse_heading_in_single_line("# test"),
            Ok((
                "",
                HeadingData {
                    level: 1.into(),
                    text: "test",
                }
            ))
        );
    }

    #[test]
    fn test_parse_header_with_null_padding() {
        // Heading with null padding after newline
        assert_eq2!(
            parse_heading_in_single_line("# test\n\0\0\0"),
            Ok((
                "",
                HeadingData {
                    level: 1.into(),
                    text: "test",
                }
            ))
        );

        // Heading without newline but with null after
        assert_eq2!(
            parse_heading_in_single_line("# test\0\0\0"),
            Ok((
                "\0\0\0",
                HeadingData {
                    level: 1.into(),
                    text: "test",
                }
            ))
        );
    }
}

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

use nom::{branch::alt,
          bytes::complete::*,
          character::complete::anychar,
          combinator::*,
          multi::many1,
          sequence::*,
          IResult};

use crate::{constants::NEW_LINE, *};

/// This matches the heading tag and text until EOL. Outputs a tuple of [HeadingLevel] and
/// [FragmentsInOneLine].
#[rustfmt::skip]
pub fn parse_block_heading_opt_eol(input: &str) -> IResult<&str, HeadingData> {
    let (remainder, output) = parse(input)?;

    // 00: eg of _opt_eol

    // Special case: Early return when just a newline after the heading prefix. Eg: `# \n..`.
    if output.text.starts_with(NEW_LINE) {
        if let Some(stripped) = output.text.strip_prefix(NEW_LINE) {
            return Ok((stripped, HeadingData {
                level: output.level,
                text: "",
            }));
        }
    }

    // Normal case: if there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE))(remainder)?;
    Ok((remainder, output))
}

#[rustfmt::skip]
fn parse(input: &str) -> IResult<&str, HeadingData> {
    let (input, (level, text)) = tuple((
        parse_heading_tag,
        alt((
            is_not(NEW_LINE),
            recognize(many1(anychar)),
        ))
    ))(input)?;
    Ok((input, HeadingData { level, text }))
}

/// Matches one or more `#` chars, consumes it, and outputs [Level].
#[rustfmt::skip]
fn parse_heading_tag(input: &str) -> IResult<&str, HeadingLevel> {
    map(
        terminated(
            /* output `#`+ */ take_while1(|it| it == constants::HEADING_CHAR),
            /* ends with (discarded) */ tag(constants::SPACE),
        ),
        |it: &str| HeadingLevel::from(it.len()),
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_parse_header_tag() {
        assert_eq!(parse_heading_tag("# "), Ok(("", 1.into())));
        assert_eq!(parse_heading_tag("### "), Ok(("", 3.into())));
        assert_eq!(parse_heading_tag("# h1"), Ok(("h1", 1.into())));
        assert_eq!(parse_heading_tag("# h1"), Ok(("h1", 1.into())));
        assert_eq!(
            parse_heading_tag(" "),
            Err(NomErr::Error(Error {
                input: " ",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq!(
            parse_heading_tag("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_header() {
        assert_eq!(
            parse_block_heading_opt_eol("# h1\n"),
            Ok((
                "",
                HeadingData {
                    level: 1.into(),
                    text: "h1",
                }
            ))
        );
        assert_eq!(
            parse_block_heading_opt_eol("## h2\n"),
            Ok((
                "",
                HeadingData {
                    level: 2.into(),
                    text: "h2",
                }
            ))
        );
        assert_eq!(
            parse_block_heading_opt_eol("###  h3\n"),
            Ok((
                "",
                HeadingData {
                    level: 3.into(),
                    text: " h3",
                }
            ))
        );
        assert_eq!(
            parse_block_heading_opt_eol("### h3 *foo* **bar**\n"),
            Ok((
                "",
                HeadingData {
                    level: 3.into(),
                    text: "h3 *foo* **bar**",
                }
            ))
        );
        assert_eq!(
            parse_block_heading_opt_eol("###h3"),
            Err(NomErr::Error(Error {
                input: "h3",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_heading_opt_eol("###"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_heading_opt_eol(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq!(
            parse_block_heading_opt_eol("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_header_with_newline() {
        assert_eq!(
            parse_block_heading_opt_eol("# \n"),
            Ok((
                "",
                HeadingData {
                    level: 1.into(),
                    text: "",
                }
            ))
        );
    }

    #[test]
    fn test_parse_header_with_no_newline() {
        assert_eq!(
            parse_block_heading_opt_eol("# test"),
            Ok((
                "",
                HeadingData {
                    level: 1.into(),
                    text: "test",
                }
            ))
        );
    }
}

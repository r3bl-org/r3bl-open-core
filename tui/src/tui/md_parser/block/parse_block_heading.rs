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
          bytes::complete::{tag, take_while1},
          character::complete::anychar,
          combinator::{map, not, opt, recognize},
          multi::many1,
          sequence::{preceded, terminated, tuple},
          IResult};

use crate::{constants::{self, NEW_LINE},
            HeadingData,
            HeadingLevel};

/// This matches the heading tag and text until EOL. Outputs a tuple of [HeadingLevel] and
/// [crate::FragmentsInOneLine].
#[rustfmt::skip]
pub fn parse_block_heading_opt_eol(input: &str) -> IResult<&str, HeadingData<'_>> {
    let (remainder, output) = parse(input)?;
    Ok((remainder, output))
}

#[rustfmt::skip]
fn parse(input: &str) -> IResult<&str, HeadingData<'_>> {
    let (input, (level, text, _)) = tuple((
        parse_heading_tag,
        parse_anychar_in_heading_no_new_line,
        opt(tag(NEW_LINE)),
    ))(input)?;
    Ok((input, HeadingData { heading_level: level, text }))
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
#[rustfmt::skip]
pub fn parse_anychar_in_heading_no_new_line(input: &str) -> IResult<&str, &str> {
    recognize(
        many1( /* match at least 1 char */
            preceded(
                /* prefix is discarded, it doesn't match anything, only errors out for special chars */
                not( /* error out if starts w/ special chars */
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
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

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
            parse_block_heading_opt_eol("# h1\n"),
            Ok((
                "",
                HeadingData {
                    heading_level: 1.into(),
                    text: "h1",
                }
            ))
        );
        assert_eq2!(
            parse_block_heading_opt_eol("## h2\n"),
            Ok((
                "",
                HeadingData {
                    heading_level: 2.into(),
                    text: "h2",
                }
            ))
        );
        assert_eq2!(
            parse_block_heading_opt_eol("###  h3\n"),
            Ok((
                "",
                HeadingData {
                    heading_level: 3.into(),
                    text: " h3",
                }
            ))
        );
        assert_eq2!(
            parse_block_heading_opt_eol("### h3 *foo* **bar**\n"),
            Ok((
                "",
                HeadingData {
                    heading_level: 3.into(),
                    text: "h3 *foo* **bar**",
                }
            ))
        );
        assert_eq2!(
            parse_block_heading_opt_eol("###h3"),
            Err(NomErr::Error(Error {
                input: "h3",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_block_heading_opt_eol("###"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_block_heading_opt_eol(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq2!(
            parse_block_heading_opt_eol("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_header_with_new_line() {
        assert_eq2!(
            parse_block_heading_opt_eol("# \n"),
            Err(NomErr::Error(Error {
                input: "\n",
                code: ErrorKind::Not
            }))
        );
    }

    #[test]
    fn test_parse_header_with_no_new_line() {
        assert_eq2!(
            parse_block_heading_opt_eol("# test"),
            Ok((
                "",
                HeadingData {
                    heading_level: 1.into(),
                    text: "test",
                }
            ))
        );
    }
}

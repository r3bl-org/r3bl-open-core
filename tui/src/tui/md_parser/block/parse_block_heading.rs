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

use nom::{bytes::complete::*, combinator::*, sequence::*, IResult};

use super::*;
use crate::*;

/// This matches the heading tag and text until EOL. Outputs a tuple of [Level] and [Line].
#[rustfmt::skip]
pub fn parse_block_heading(input: &str) -> IResult<&str, (Level, Fragments)> {
    parse(input)
}

#[rustfmt::skip]
fn parse(input: &str) -> IResult<&str, (Level, Fragments)> {
    tuple(
        (parse_heading_tag, parse_block_markdown_text_until_eol)
    )(input)
}

/// Matches one or more `#` chars, consumes it, and outputs [Level].
#[rustfmt::skip]
fn parse_heading_tag(input: &str) -> IResult<&str, Level> {
    map(
        terminated(
            /* output `#`+ */ take_while1(|it| it == constants::HEADING_CHAR),
            /* ends with (discarded) */ tag(constants::SPACE),
        ),
        |it: &str| Level::from(it.len()),
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

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
            parse_block_heading("# h1\n"),
            Ok(("", (1.into(), vec![Fragment::Plain("h1")])))
        );
        assert_eq!(
            parse_block_heading("## h2\n"),
            Ok(("", (2.into(), vec![Fragment::Plain("h2")])))
        );
        assert_eq!(
            parse_block_heading("###  h3\n"),
            Ok(("", (3.into(), vec![Fragment::Plain(" h3")])))
        );
        assert_eq!(
            parse_block_heading("###h3"),
            Err(NomErr::Error(Error {
                input: "h3",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_heading("###"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_heading(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::TakeWhile1
            }))
        );
        assert_eq!(
            parse_block_heading("#"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(parse_block_heading("# \n"), Ok(("", (1.into(), vec![]))));
        assert_eq!(
            parse_block_heading("# test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }
}

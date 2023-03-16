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

use constants::*;
use nom::{bytes::complete::*, multi::*, sequence::*, IResult};

use super::*;
use crate::*;

#[rustfmt::skip]
pub fn parse_block_unordered_list(input: &str) -> IResult<&str, Vec<MdLineFragments>> {
    many1(
        parse_unordered_list_element
    )(input)
}

/// Matches `- `. Outputs the `-` char.
#[rustfmt::skip]
fn parse_unordered_list_tag(input: &str) -> IResult<&str, &str> {
    terminated(
        /* output `-` */ tag(UNORDERED_LIST),
        /* ends with (discarded) */ tag(SPACE),
    )(input)
}

#[rustfmt::skip]
fn parse_unordered_list_element(input: &str) -> IResult<&str, MdLineFragments> {
    preceded(
        /* prefix (discarded) */ parse_unordered_list_tag,
        /* output */ parse_block_markdown_text_until_eol,
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

    use super::*;
    use crate::test_data::raw_strings;

    #[test]
    fn test_parse_unordered_list_tag() {
        assert_eq!(parse_unordered_list_tag("- "), Ok(("", "-")));
        assert_eq!(
            parse_unordered_list_tag("- and some more"),
            Ok(("and some more", "-"))
        );
        assert_eq!(
            parse_unordered_list_tag("-"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_tag("-and some more"),
            Err(NomErr::Error(Error {
                input: "and some more",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_tag("--"),
            Err(NomErr::Error(Error {
                input: "-",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_tag(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_unordered_list_element() {
        assert_eq!(
            parse_unordered_list_element("- this is an element\n"),
            Ok(("", vec![MdLineFragment::Plain("this is an element")]))
        );
        assert_eq!(
            parse_unordered_list_element(raw_strings::UNORDERED_LIST_ELEMENT),
            Ok((
                "- this is another element\n",
                vec![MdLineFragment::Plain("this is an element")]
            ))
        );
        assert_eq!(
            parse_unordered_list_element(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(parse_unordered_list_element("- \n"), Ok(("", vec![])));
        assert_eq!(
            parse_unordered_list_element("- "),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_element("- test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_unordered_list_element("-"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_unordered_list() {
        assert_eq!(
            parse_block_unordered_list("- this is an element"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq!(
            parse_block_unordered_list("- this is an element\n"),
            Ok(("", vec![vec![MdLineFragment::Plain("this is an element")]]))
        );
        assert_eq!(
            parse_block_unordered_list(raw_strings::UNORDERED_LIST_ELEMENT),
            Ok((
                "",
                vec![
                    vec![MdLineFragment::Plain("this is an element")],
                    vec![MdLineFragment::Plain("this is another element")]
                ]
            ))
        );
    }
}

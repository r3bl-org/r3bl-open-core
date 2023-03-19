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
use nom::{bytes::complete::*, character::complete::*, multi::*, sequence::*, IResult};

use super::*;
use crate::*;

#[rustfmt::skip]
pub fn parse_block_ordered_list(input: &str) -> IResult<&str, Vec<MdLineFragments>> {
    many1(
        parse_ordered_list_element
    )(input)
}

#[rustfmt::skip]
fn parse_ordered_list_tag(input: &str) -> IResult<&str, usize> {
    let (input, output) =
    terminated(
        /* output */
        terminated(
            /* output */ digit1,
            /* ends with (discarded) */ tag(PERIOD),
        ),
        /* ends with (discarded) */ tag(SPACE),
    )(input)?;
    let number_output = output.parse::<usize>().unwrap();
    Ok((input, number_output))
}

#[rustfmt::skip]
fn parse_ordered_list_element(input: &str) -> IResult<&str, MdLineFragments> {
    let (input, (number, line)) = tuple((
        /* prefix (discarded) */ parse_ordered_list_tag,
        /* output */ parse_block_markdown_text_until_eol,
    ))(input)?;

    // Insert line number before the line.
    let mut it = vec![MdLineFragment::OrderedListItemNumber(number)];
    it.extend(line);

    Ok((input, it))
}

#[cfg(test)]
mod tests {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::test_data::raw_strings;

    #[test]
    fn test_parse_ordered_list_tag() {
        assert_eq2!(parse_ordered_list_tag("1. "), Ok(("", 1)));
        assert_eq2!(parse_ordered_list_tag("1234567. "), Ok(("", 1234567)));
        assert_eq2!(
            parse_ordered_list_tag("3. and some more"),
            Ok(("and some more", 3))
        );
        assert_eq2!(
            parse_ordered_list_tag("1"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_ordered_list_tag("1.and some more"),
            Err(NomErr::Error(Error {
                input: "and some more",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_ordered_list_tag("1111."),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_ordered_list_tag(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Digit
            }))
        );
    }

    #[test]
    fn test_parse_ordered_list_element() {
        assert_eq2!(
            parse_ordered_list_element("1. this is an element\n"),
            Ok((
                "",
                vec![
                    MdLineFragment::OrderedListItemNumber(1),
                    MdLineFragment::Plain("this is an element")
                ]
            ))
        );
        assert_eq2!(
            parse_ordered_list_element(raw_strings::ORDERED_LIST_ELEMENT),
            Ok((
                "1. here is another\n",
                vec![
                    MdLineFragment::OrderedListItemNumber(1),
                    MdLineFragment::Plain("this is an element")
                ]
            ))
        );
        assert_eq2!(
            parse_ordered_list_element(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Digit
            }))
        );
        assert_eq2!(
            parse_ordered_list_element(""),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Digit
            }))
        );
        assert_eq2!(
            parse_ordered_list_element("1. \n"),
            Ok(("", vec![MdLineFragment::OrderedListItemNumber(1),]))
        );
        assert_eq2!(
            parse_ordered_list_element("1. test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_ordered_list_element("1. "),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_ordered_list_element("1."),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_ordered_list() {
        assert_eq2!(
            parse_block_ordered_list("1. this is an element\n"),
            Ok((
                "",
                vec![vec![
                    MdLineFragment::OrderedListItemNumber(1),
                    MdLineFragment::Plain("this is an element")
                ]]
            ))
        );
        assert_eq2!(
            parse_block_ordered_list("1. test"),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Tag
            }))
        );
        assert_eq2!(
            parse_block_ordered_list(raw_strings::ORDERED_LIST_ELEMENT),
            Ok((
                "",
                vec![
                    vec![
                        MdLineFragment::OrderedListItemNumber(1),
                        MdLineFragment::Plain("this is an element")
                    ],
                    vec![
                        MdLineFragment::OrderedListItemNumber(1),
                        MdLineFragment::Plain("here is another")
                    ],
                ]
            ))
        );
    }
}

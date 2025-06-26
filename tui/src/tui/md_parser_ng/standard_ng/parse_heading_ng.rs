/*
 *   Copyright (c) 2025 R3BL LLC
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
use nom::{bytes::complete::{tag, take_while1},
          combinator::map,
          sequence::terminated,
          IResult,
          Input,
          Parser};

use crate::{md_parser::constants, AsStrSlice, GCString, HeadingData, HeadingLevel};

/// This matches the heading tag and text within the current line only.
/// Line advancement is handled by the infrastructure via ensure_advance_with_parser.
#[rustfmt::skip]
pub fn parse_line_heading_no_advance_ng<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, HeadingData<'a>> {
    let (remainder, output) = parse_impl(input)?;
    Ok((remainder, output))
}

#[rustfmt::skip]
fn parse_impl<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, HeadingData<'a>> {
    // Only parse within the current line - no line advancement concerns
    let current_line = input.extract_to_line_end();
    if current_line.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Parse heading tag (# ## ### etc.)
    let (input, level) = parse_heading_tag_ng(input)?;

    // Parse the rest of the current line as heading text
    let remaining_text = input.extract_to_line_end();

    Ok((input, HeadingData { level, text: remaining_text }))
}

/// Matches one or more `#` chars, consumes it, and outputs [Level].
#[rustfmt::skip]
fn parse_heading_tag_ng<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, HeadingLevel> {
    map(
        terminated(
            /* output `#`+ */ take_while1(|it| it == constants::HEADING_CHAR),
            /* ends with (discarded) */ tag(constants::SPACE),
        ),
        |it: AsStrSlice<'a, GCString>|
        HeadingLevel::from(it.input_len())
    ).parse(input)
}

#[cfg(test)]
mod tests {
    use nom::{error::ErrorKind, Err as NomErr};

    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_parse_header_tag() {
        as_str_slice_test_case!(input1, "# ");
        let (remainder1, level1) = parse_heading_tag_ng(input1).unwrap();
        assert_eq2!(remainder1.to_string(), "");
        assert_eq2!(level1, 1.into());

        as_str_slice_test_case!(input2, "### ");
        let (remainder2, level2) = parse_heading_tag_ng(input2).unwrap();
        assert_eq2!(remainder2.to_string(), "");
        assert_eq2!(level2, 3.into());

        as_str_slice_test_case!(input3, "# h1");
        let (remainder3, level3) = parse_heading_tag_ng(input3).unwrap();
        assert_eq2!(remainder3.to_string(), "h1");
        assert_eq2!(level3, 1.into());

        as_str_slice_test_case!(input5, " ");
        match parse_heading_tag_ng(input5) {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.code, ErrorKind::TakeWhile1);
            }
            _ => panic!("Expected an error"),
        }

        as_str_slice_test_case!(input6, "#");
        match parse_heading_tag_ng(input6) {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected an error"),
        }

        as_str_slice_test_case!(input7, "####### ");
        let (remainder7, level7) = parse_heading_tag_ng(input7).unwrap();
        assert_eq2!(remainder7.to_string(), "");
        assert_eq2!(level7, 7.into());
    }

    #[test]
    fn test_parse_header() {
        {
            as_str_slice_test_case!(input1, "# h1\n");
            let (remainder1, heading_data1) =
                parse_line_heading_no_advance_ng(input1).unwrap();
            assert_eq2!(remainder1.to_string(), "");
            assert_eq2!(heading_data1.level, 1.into());
            assert_eq2!(heading_data1.text, "h1");
        }
        {
            as_str_slice_test_case!(input2, "## h2\n");
            let (remainder2, heading_data2) =
                parse_line_heading_no_advance_ng(input2).unwrap();
            assert_eq2!(remainder2.to_string(), "");
            assert_eq2!(heading_data2.level, 2.into());
            assert_eq2!(heading_data2.text, "h2");
        }
        {
            as_str_slice_test_case!(input3, "###  h3\n");
            let (remainder3, heading_data3) =
                parse_line_heading_no_advance_ng(input3).unwrap();
            assert_eq2!(remainder3.to_string(), "");
            assert_eq2!(heading_data3.level, 3.into());
            assert_eq2!(heading_data3.text, " h3");
        }
        {
            as_str_slice_test_case!(input4, "#### h4\n");
            let (remainder4, heading_data4) =
                parse_line_heading_no_advance_ng(input4).unwrap();
            assert_eq2!(remainder4.to_string(), "");
            assert_eq2!(heading_data4.level, 4.into());
            assert_eq2!(heading_data4.text, "h4");
        }
        {
            as_str_slice_test_case!(input5, "##### h5\n");
            let (remainder5, heading_data5) =
                parse_line_heading_no_advance_ng(input5).unwrap();
            assert_eq2!(remainder5.to_string(), "");
            assert_eq2!(heading_data5.level, 5.into());
            assert_eq2!(heading_data5.text, "h5");
        }
        {
            as_str_slice_test_case!(input6, "###### h6\n");
            let (remainder6, heading_data6) =
                parse_line_heading_no_advance_ng(input6).unwrap();
            assert_eq2!(remainder6.to_string(), "");
            assert_eq2!(heading_data6.level, 6.into());
            assert_eq2!(heading_data6.text, "h6");
        }
        {
            as_str_slice_test_case!(input7, "####### h7\n");
            let (remainder7, heading_data7) =
                parse_line_heading_no_advance_ng(input7).unwrap();
            assert_eq2!(remainder7.to_string(), "");
            assert_eq2!(heading_data7.level, 7.into());
            assert_eq2!(heading_data7.text, "h7");
        }
        {
            as_str_slice_test_case!(input8, "### h3 *foo* **bar**\n");
            let (remainder8, heading_data8) =
                parse_line_heading_no_advance_ng(input8).unwrap();
            assert_eq2!(remainder8.to_string(), "");
            assert_eq2!(heading_data8.level, 3.into());
            assert_eq2!(heading_data8.text, "h3 *foo* **bar**");
        }
        {
            as_str_slice_test_case!(input_err1, "###h3");
            match parse_line_heading_no_advance_ng(input_err1) {
                Err(NomErr::Error(err)) => {
                    assert_eq2!(err.code, ErrorKind::Tag);
                }
                _ => panic!("Expected an error"),
            };
        }
        {
            as_str_slice_test_case!(input_err2, "####h4");
            let result = parse_line_heading_no_advance_ng(input_err2);
            match result {
                Err(NomErr::Error(err)) => {
                    assert_eq2!(err.input.to_string(), "h4");
                    assert_eq2!(err.code, ErrorKind::Tag);
                }
                _ => panic!("Expected an error"),
            }
        }
        {
            as_str_slice_test_case!(input_err3, "###");
            let result = parse_line_heading_no_advance_ng(input_err3);
            match result {
                Err(NomErr::Error(err)) => {
                    assert_eq2!(err.input.to_string(), "");
                    assert_eq2!(err.code, ErrorKind::Tag);
                }
                _ => panic!("Expected an error"),
            }
        }
        {
            as_str_slice_test_case!(input_err4, "");
            let result = parse_line_heading_no_advance_ng(input_err4);
            match result {
                Err(NomErr::Error(err)) => {
                    assert_eq2!(err.input.to_string(), "");
                    assert_eq2!(err.code, ErrorKind::Tag);
                }
                _ => panic!("Expected an error"),
            }
        }
        {
            as_str_slice_test_case!(input_err5, "#");
            let result = parse_line_heading_no_advance_ng(input_err5);
            match result {
                Err(NomErr::Error(err)) => {
                    assert_eq2!(err.input.to_string(), "");
                    assert_eq2!(err.code, ErrorKind::Tag);
                }
                _ => panic!("Expected an error"),
            }
        }
    }

    #[test]
    fn test_parse_header_with_new_line() {
        {
            as_str_slice_test_case!(input1, "# \n");
            let result = parse_line_heading_no_advance_ng(input1);
            match result {
                Err(NomErr::Error(err)) => {
                    assert_eq2!(err.input.to_string(), "\n");
                    assert_eq2!(err.code, ErrorKind::Tag);
                }
                _ => panic!("Expected an error"),
            }
        }
    }

    #[test]
    fn test_parse_header_with_no_new_line() {
        {
            as_str_slice_test_case!(input1, "# test");
            let (remainder, heading_data) =
                parse_line_heading_no_advance_ng(input1).unwrap();
            assert_eq2!(remainder.to_string(), "");
            assert_eq2!(heading_data.level, 1.into());
            assert_eq2!(heading_data.text, "test");
        }
    }
}

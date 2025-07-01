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

use crate::{md_parser::constants,
            AsStrSlice,
            CharLengthExt,
            GCString,
            HeadingData,
            HeadingLevel};

/// This matches the heading tag and text within the current line only.
/// Line advancement is handled by the infrastructure via `ensure_advance_with_parser`.
#[rustfmt::skip]
pub fn parse_line_heading_no_advance_ng<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, HeadingData<'a>> {
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

    // Parse the rest of the current line as heading text.
    let remaining_text = input.extract_to_line_end();

    // Consume the remaining text from the input (make sure to respect unicode characters).
    let consumed_input = input.take_from(remaining_text.len_chars().as_usize());

    Ok((consumed_input, HeadingData { level, text: remaining_text }))
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
    fn test_parse_header_tag_level1() {
        as_str_slice_test_case!(input, "# ");
        let (remainder, level) = parse_heading_tag_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(level, 1.into());
    }

    #[test]
    fn test_parse_header_tag_level3() {
        as_str_slice_test_case!(input, "### ");
        let (remainder, level) = parse_heading_tag_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(level, 3.into());
    }

    #[test]
    fn test_parse_header_tag_with_text() {
        as_str_slice_test_case!(input, "# h1");
        let (remainder, level) = parse_heading_tag_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "h1");
        assert_eq2!(level, 1.into());
    }

    #[test]
    fn test_parse_header_tag_space_only() {
        as_str_slice_test_case!(input, " ");
        match parse_heading_tag_ng(input) {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.code, ErrorKind::TakeWhile1);
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_parse_header_tag_hash_only() {
        as_str_slice_test_case!(input, "#");
        match parse_heading_tag_ng(input) {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_parse_header_tag_level7() {
        as_str_slice_test_case!(input, "####### ");
        let (remainder, level) = parse_heading_tag_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(level, 7.into());
    }

    #[test]
    fn test_parse_header_h1() {
        as_str_slice_test_case!(input, "# h1");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 1.into());
        assert_eq2!(heading_data.text, "h1");
    }

    #[test]
    fn test_parse_header_h2() {
        as_str_slice_test_case!(input, "## h2");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 2.into());
        assert_eq2!(heading_data.text, "h2");
    }

    #[test]
    fn test_parse_header_h3_with_extra_space() {
        as_str_slice_test_case!(input, "###  h3");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 3.into());
        assert_eq2!(heading_data.text, " h3");
    }

    #[test]
    fn test_parse_header_h4() {
        as_str_slice_test_case!(input, "#### h4");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 4.into());
        assert_eq2!(heading_data.text, "h4");
    }

    #[test]
    fn test_parse_header_h5() {
        as_str_slice_test_case!(input, "##### h5");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 5.into());
        assert_eq2!(heading_data.text, "h5");
    }

    #[test]
    fn test_parse_header_h6() {
        as_str_slice_test_case!(input, "###### h6");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 6.into());
        assert_eq2!(heading_data.text, "h6");
    }

    #[test]
    fn test_parse_header_h7() {
        as_str_slice_test_case!(input, "####### h7");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 7.into());
        assert_eq2!(heading_data.text, "h7");
    }

    #[test]
    fn test_parse_header_with_markdown_formatting() {
        as_str_slice_test_case!(input, "### h3 *foo* **bar**");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 3.into());
        assert_eq2!(heading_data.text, "h3 *foo* **bar**");
    }

    #[test]
    fn test_parse_header_error_no_space_after_hash() {
        as_str_slice_test_case!(input, "###h3");
        match parse_line_heading_no_advance_ng(input) {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_parse_header_error_no_space_after_hash_h4() {
        as_str_slice_test_case!(input, "####h4");
        let result = parse_line_heading_no_advance_ng(input);
        match result {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.input.to_string(), "h4");
                assert_eq2!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_parse_header_error_hash_only() {
        as_str_slice_test_case!(input, "###");
        let result = parse_line_heading_no_advance_ng(input);
        match result {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.input.to_string(), "");
                assert_eq2!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_parse_header_error_empty_input() {
        as_str_slice_test_case!(input, "");
        let result = parse_line_heading_no_advance_ng(input);
        match result {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.input.to_string(), "");
                assert_eq2!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_parse_header_error_single_hash() {
        as_str_slice_test_case!(input, "#");
        let result = parse_line_heading_no_advance_ng(input);
        match result {
            Err(NomErr::Error(err)) => {
                assert_eq2!(err.input.to_string(), "");
                assert_eq2!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_parse_header_with_no_new_line() {
        as_str_slice_test_case!(input, "# test");
        let (remainder, heading_data) = parse_line_heading_no_advance_ng(input).unwrap();
        assert_eq2!(remainder.to_string(), "");
        assert_eq2!(heading_data.level, 1.into());
        assert_eq2!(heading_data.text, "test");
    }
}

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

use nom::{bytes::complete::{tag, take_while, take_while1},
          combinator::{opt, verify},
          multi::many0,
          sequence::preceded,
          IResult,
          Parser as _};

use crate::{as_str_slice_test_case,
            constants::{COMMA_CHAR, NEW_LINE_CHAR, SPACE_CHAR},
            inline_vec,
            list,
            md_parser::constants::{COLON, COMMA, NEW_LINE, SPACE},
            parser_take_text_until_eol_or_eoi_alt::parser_take_text_until_eol_or_eoi_alt,
            AsStrSlice,
            InlineVec,
            List};

/// - Sample parse input:
///   - `@tags: tag1, tag2, tag3`
///   - `@tags: tag1, tag2, tag3\n`
///   - `@authors: me, myself, i`
///   - `@authors: me, myself, i\n`
/// - There may or may not be a newline at the end. If there is, it is consumed.
pub fn parse_csv_opt_eol_alt<'a>(
    tag_name: &str,
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, List<AsStrSlice<'a>>> {
    // Parse the tag name, colon, and space
    let (rem, _) = (tag(tag_name), tag(COLON), tag(SPACE)).parse(input)?;

    // Get the text content until end of line or end of input
    let (rem, tags_text) = parser_take_text_until_eol_or_eoi_alt().parse(rem)?;

    // If there is a newline, consume it.
    let (rem_new, _) = opt(tag(NEW_LINE)).parse(rem)?;

    // Special case: Early return when just a `@tags: ` is found.
    if tags_text.is_empty() {
        Ok((rem_new, list![]))
    }
    // Normal case.
    else {
        // At this point, `output` can have something like: `tag1, tag2, tag3`.
        let (_, vec_tags_text) = parse_comma_separated_list_alt(tags_text)?;
        Ok((rem_new, List::from(vec_tags_text)))
    }
}

/// | input                | rem     |  output                           |
/// | -------------------- | ------- | --------------------------------- |
/// | `"tag1, tag2, tag3"` | `""`    | `vec!(["tag1", "tag2", "tag3"])`  |
fn parse_comma_separated_list_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, InlineVec<AsStrSlice<'a>>> {
    // Handle empty input.
    if input.is_empty() {
        return Ok((input, inline_vec![]));
    }

    // Parser for the first item (must not start with space, stops at comma or end).
    let mut first_item = verify(
        take_while1(|c: char| c != COMMA_CHAR && c != NEW_LINE_CHAR),
        |item: &AsStrSlice<'_>| !item.starts_with(SPACE),
    );

    // Parser for subsequent items (after comma, must start with at least one space, then
    // trim all leading spaces).
    let subsequent_item = preceded(
        tag(COMMA),
        preceded(
            verify(
                take_while1(|c: char| c == SPACE_CHAR),
                |spaces: &AsStrSlice<'_>| !spaces.is_empty(),
            ),
            take_while1(|c: char| c != COMMA_CHAR && c != NEW_LINE_CHAR),
        ),
    );

    // Parse first item.
    let (remaining, first) = first_item.parse(input)?;

    // Parse remaining items.
    let (remaining, rest) = many0(subsequent_item).parse(remaining)?;

    // Check if there's any remaining input that contains a comma.
    // This ensures we reject inputs like "tag1,tag2" (without a space after the comma).
    if remaining.contains(COMMA) {
        return Err(nom::Err::Error(nom::error::Error::new(
            remaining,
            nom::error::ErrorKind::Verify,
        )));
    }

    // Build result vector.
    let mut result = InlineVec::with_capacity(1 + rest.len());
    result.push(first);
    result.extend(rest);

    Ok((remaining, result))
}

#[cfg(test)]
mod test_parse_tags_opt_eol {
    use super::*;
    use crate::{assert_eq2, md_parser::constants::TAGS, GCString};

    #[test]
    fn test_not_quoted_no_eol() {
        as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3");

        let (input, output) = super::parse_csv_opt_eol_alt(TAGS, input).unwrap();
        assert_eq2!(input.extract_to_slice_end(), "");

        // Create expected output with AsStrSlice values
        let expected_tag1 = &[GCString::new("tag1")];
        let expected_tag2 = &[GCString::new("tag2")];
        let expected_tag3 = &[GCString::new("tag3")];
        let expected = list![
            AsStrSlice::from(expected_tag1),
            AsStrSlice::from(expected_tag2),
            AsStrSlice::from(expected_tag3)
        ];

        // Compare the string representations for easier debugging
        assert_eq2!(
            output
                .iter()
                .map(|s| s.extract_to_slice_end())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|s| s.extract_to_slice_end())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_quoted_no_eol_err_whitespace() {
        // First fragment mustn't have any space prefix.
        as_str_slice_test_case!(input1, "@tags:  tag1, tag2, tag3");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input1).is_err(), true,);

        // 2nd fragment onwards must have a single space prefix.
        as_str_slice_test_case!(input2, "@tags: tag1,tag2, tag3");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input2).is_err(), true,);

        as_str_slice_test_case!(input3, "@tags: tag1,  tag2,tag3");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input3).is_err(), true,);

        as_str_slice_test_case!(input4, "@tags: tag1, tag2,tag3");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input4).is_err(), true,);

        // It is ok to have more than 1 prefix space for 2nd fragment onwards.
        as_str_slice_test_case!(input5, "@tags: tag1, tag2,  tag3");
        let result = parse_csv_opt_eol_alt(TAGS, input5).unwrap();
        assert_eq2!(result.0.extract_to_slice_end(), "",);

        // Create expected output with AsStrSlice values
        let expected_tag1 = &[GCString::new("tag1")];
        let expected_tag2 = &[GCString::new("tag2")];
        let expected_tag3 = &[GCString::new("tag3")];
        let expected = list![
            AsStrSlice::from(expected_tag1),
            AsStrSlice::from(expected_tag2),
            AsStrSlice::from(expected_tag3)
        ];

        // Compare the string representations for easier debugging
        assert_eq2!(
            result
                .1
                .iter()
                .map(|s| s.extract_to_slice_end())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|s| s.extract_to_slice_end())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_quoted_with_eol() {
        // Valid.
        {
            as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3\n");

            let (input, output) = parse_csv_opt_eol_alt(TAGS, input).unwrap();
            assert_eq2!(input.extract_to_slice_end(), "");

            // Create expected output with AsStrSlice values
            let expected_tag1 = &[GCString::new("tag1")];
            let expected_tag2 = &[GCString::new("tag2")];
            let expected_tag3 = &[GCString::new("tag3")];
            let expected = list![
                AsStrSlice::from(expected_tag1),
                AsStrSlice::from(expected_tag2),
                AsStrSlice::from(expected_tag3)
            ];

            // Compare the string representations for easier debugging
            assert_eq2!(
                output
                    .iter()
                    .map(|s| s.extract_to_slice_end())
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|s| s.extract_to_slice_end())
                    .collect::<Vec<_>>()
            );
        }

        {
            as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3\n]\n");

            let result = parse_csv_opt_eol_alt(TAGS, input);
            assert_eq2!(result.is_err(), false);
        }

        {
            as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3");

            let result = parse_csv_opt_eol_alt(TAGS, input);
            assert_eq2!(result.is_err(), false);
        }
    }

    #[test]
    fn test_not_quoted_with_eol_whitespace() {
        // First fragment mustn't have any space prefix.
        as_str_slice_test_case!(input1, "@tags:  tag1, tag2, tag3\n");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input1).is_err(), true,);

        // 2nd fragment onwards must have a single space prefix.
        as_str_slice_test_case!(input2, "@tags: tag1,tag2, tag3\n");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input2).is_err(), true,);

        as_str_slice_test_case!(input3, "@tags: tag1,  tag2,tag3\n");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input3).is_err(), true,);

        as_str_slice_test_case!(input4, "@tags: tag1, tag2,tag3\n");
        assert_eq2!(parse_csv_opt_eol_alt(TAGS, input4).is_err(), true,);

        // It is ok to have more than 1 prefix space for 2nd fragment onwards.
        as_str_slice_test_case!(input5, "@tags: tag1, tag2,  tag3\n");
        let result = parse_csv_opt_eol_alt(TAGS, input5).unwrap();
        assert_eq2!(result.0.extract_to_slice_end(), "",);

        // Create expected output with AsStrSlice values
        let expected_tag1 = &[GCString::new("tag1")];
        let expected_tag2 = &[GCString::new("tag2")];
        let expected_tag3 = &[GCString::new("tag3")];
        let expected = list![
            AsStrSlice::from(expected_tag1),
            AsStrSlice::from(expected_tag2),
            AsStrSlice::from(expected_tag3)
        ];

        // Compare the string representations for easier debugging
        assert_eq2!(
            result
                .1
                .iter()
                .map(|s| s.extract_to_slice_end())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|s| s.extract_to_slice_end())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_quoted_with_postfix_content() {
        as_str_slice_test_case!(input, "@tags: \nfoo\nbar");

        println!("Input: {:?}", input.extract_to_slice_end());
        let (input, output) = parse_csv_opt_eol_alt(TAGS, input).unwrap();
        println!("Remainder: {:?}", input.extract_to_slice_end());
        println!("Output: {:?}", output);

        assert_eq2!(input.extract_to_slice_end(), "foo\nbar");

        // Empty list case
        let expected = list![];
        assert_eq2!(output, expected);
    }
}

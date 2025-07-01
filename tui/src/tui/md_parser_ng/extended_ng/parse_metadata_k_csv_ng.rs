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
          combinator::verify,
          multi::many0,
          sequence::preceded,
          IResult,
          Parser as _};

use crate::{constants::{COMMA_CHAR, NEW_LINE_CHAR, SPACE_CHAR},
            inline_vec,
            list,
            md_parser::constants::{COLON, COMMA, SPACE},
            parser_take_text_until_eol_or_eoi_ng::parser_take_line_text_ng,
            AsStrSlice,
            InlineVec,
            List};

/// Parse tags metadata from a line like `@tags: rust, parsing, markdown, documentation`
/// or `@authors: author1, author2, author3`.
///
/// ## Input format
/// Expects a line starting with [`crate::constants::TAGS`] + colon + space followed by a
/// comma-separated list of tag names. Whitespace around tag names is trimmed. Tags
/// typically represent categories, keywords or topics. The line may end with a newline
/// or be at end-of-input.
///
/// ## Line advancement
/// This is a **single-line parser with NO advancement**. It only parses the current line
/// content and does NOT consume trailing newlines. Line advancement is handled by the
/// infrastructure (`ensure_advance_with_parser`), following the same architectural
/// pattern as the heading and key-value parsers.
///
/// ## Returns
/// - Either `Ok((remaining_input, List<AsStrSlice>))` with the list of tag names on
///   success.
/// - Or `Err` if the line doesn't start with [`crate::constants::TAGS`] + colon + space
///   or has invalid format.
///
/// ## Example
/// - `"@authors: Alice, Bob, Charlie"` → `["Alice", "Bob", "Charlie"]`
/// - `"@tags: rust, parsing, nom"` → `["rust", "parsing", "nom"]`
/// - `"@tags: tag1, tag2, tag3"`
/// - `"@authors: me, myself, i"`
pub fn parse_line_csv_no_advance_ng<'a>(
    tag_name: &str,
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, List<AsStrSlice<'a>>> {
    // Parse only the current line content, without newline handling
    let (rem_new, tags_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ parser_take_line_text_ng(),
    )
    .parse(input)?;

    // Special case: Early return when just a `@tags: ` is found.
    if tags_text.is_empty() {
        Ok((rem_new, list![]))
    }
    // Normal case.
    else {
        // At this point, `output` can have something like: `tag1, tag2, tag3`.
        let (_, vec_tags_text) = parse_comma_separated_list_ng(tags_text)?;
        Ok((rem_new, List::from(vec_tags_text)))
    }
}

/// | input                | rem     |  output                           |
/// | -------------------- | ------- | --------------------------------- |
/// | `"tag1, tag2, tag3"` | `""`    | `vec!(["tag1", "tag2", "tag3"])`  |
fn parse_comma_separated_list_ng<'a>(
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
    if remaining.contains_in_current_line(COMMA) {
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
    use crate::{as_str_slice_test_case,
                assert_eq2,
                md_parser::constants::TAGS,
                AsStrSlice,
                GCString};

    #[test]
    fn test_not_quoted_no_eol() {
        as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3");

        let (input, output) = super::parse_line_csv_no_advance_ng(TAGS, input).unwrap();
        assert_eq2!(input.extract_to_slice_end().as_ref(), "");

        // Create expected output with AsStrSlice values.
        let expected_tag1 = &[GCString::new("tag1")];
        let expected_tag2 = &[GCString::new("tag2")];
        let expected_tag3 = &[GCString::new("tag3")];
        let expected = list![
            AsStrSlice::from(expected_tag1),
            AsStrSlice::from(expected_tag2),
            AsStrSlice::from(expected_tag3)
        ];

        // Compare the string representations for easier debugging.
        assert_eq2!(
            output
                .iter()
                .map(AsStrSlice::extract_to_slice_end)
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(AsStrSlice::extract_to_slice_end)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_quoted_no_eol_err_whitespace() {
        // First fragment mustn't have any space prefix.
        as_str_slice_test_case!(input1, "@tags:  tag1, tag2, tag3");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input1).is_err(), true,);

        // 2nd fragment onwards must have a single space prefix.
        as_str_slice_test_case!(input2, "@tags: tag1,tag2, tag3");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input2).is_err(), true,);

        as_str_slice_test_case!(input3, "@tags: tag1,  tag2,tag3");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input3).is_err(), true,);

        as_str_slice_test_case!(input4, "@tags: tag1, tag2,tag3");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input4).is_err(), true,);

        // It is ok to have more than 1 prefix space for 2nd fragment onwards.
        as_str_slice_test_case!(input5, "@tags: tag1, tag2,  tag3");
        let result = parse_line_csv_no_advance_ng(TAGS, input5).unwrap();
        assert_eq2!(result.0.extract_to_slice_end().as_ref(), "");

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
                .map(AsStrSlice::extract_to_slice_end)
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(AsStrSlice::extract_to_slice_end)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_quoted_with_eol() {
        // Valid.
        {
            as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3\n");

            let (input, output) = parse_line_csv_no_advance_ng(TAGS, input).unwrap();
            assert_eq2!(input.extract_to_slice_end().as_ref(), "\n");

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
                    .map(AsStrSlice::extract_to_slice_end)
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(AsStrSlice::extract_to_slice_end)
                    .collect::<Vec<_>>()
            );
        }

        {
            as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3\n]\n");

            let result = parse_line_csv_no_advance_ng(TAGS, input);
            assert_eq2!(result.is_err(), false);
        }

        {
            as_str_slice_test_case!(input, "@tags: tag1, tag2, tag3");

            let result = parse_line_csv_no_advance_ng(TAGS, input);
            assert_eq2!(result.is_err(), false);
        }
    }

    #[test]
    fn test_not_quoted_with_eol_whitespace() {
        // First fragment mustn't have any space prefix.
        as_str_slice_test_case!(input1, "@tags:  tag1, tag2, tag3\n");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input1).is_err(), true,);

        // 2nd fragment onwards must have a single space prefix.
        as_str_slice_test_case!(input2, "@tags: tag1,tag2, tag3\n");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input2).is_err(), true,);

        as_str_slice_test_case!(input3, "@tags: tag1,  tag2,tag3\n");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input3).is_err(), true,);

        as_str_slice_test_case!(input4, "@tags: tag1, tag2,tag3\n");
        assert_eq2!(parse_line_csv_no_advance_ng(TAGS, input4).is_err(), true,);

        // It is ok to have more than 1 prefix space for 2nd fragment onwards.
        as_str_slice_test_case!(input5, "@tags: tag1, tag2,  tag3\n");
        let result = parse_line_csv_no_advance_ng(TAGS, input5).unwrap();
        assert_eq2!(result.0.extract_to_slice_end().as_ref(), "\n");

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
                .map(AsStrSlice::extract_to_slice_end)
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(AsStrSlice::extract_to_slice_end)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_quoted_with_postfix_content() {
        as_str_slice_test_case!(input, "@tags: \nfoo\nbar");

        println!("Input: {:?}", input.extract_to_slice_end());
        let (input, output) = parse_line_csv_no_advance_ng(TAGS, input).unwrap();
        println!("Remainder: {:?}", input.extract_to_slice_end());
        println!("Output: {output:?}");

        assert_eq2!(input.extract_to_slice_end().as_ref(), "\nfoo\nbar");

        // Empty list case
        let expected = list![];
        assert_eq2!(output, expected);
    }
}

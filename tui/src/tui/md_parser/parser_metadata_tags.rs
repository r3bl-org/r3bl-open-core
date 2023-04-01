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
use nom::{branch::*,
          bytes::complete::*,
          character::complete::*,
          combinator::*,
          multi::*,
          sequence::*,
          IResult};

use crate::*;

/// - Parse input: `@tags: [tag1, tag2, tag3]`.
/// - There may or may not be a newline at the end.
#[rustfmt::skip]
pub fn parse_tags_opt_eol(input: &str) -> IResult<&str, List<&str>> {
    // BM: eg of _opt_eol
    let (input, output) = preceded(
        /* start */ tuple((tag(TAGS), tag(COLON), tag(SPACE))),
        /* output */ alt((
            is_not(NEW_LINE),
            recognize(many1(anychar)),
        )),
    )(input)?;

    // At this point, `output` can have something like: `[tag1, tag2, tag3]`.
    let (_, output) = parse_tag_list_enclosed_in_brackets(output)?;

    // At this point, `output` can have something like: `tag1, tag2, tag3`.
    let (_, output) = parse_tag_list_comma_separated(output)?;

    // If there is a newline, consume it since there may or may not be a newline at the end.
    let (input, _) = opt(tag(NEW_LINE))(input)?;

    return Ok((input, List::from(output)));

    /// Parse input: `[tag1, tag2, tag3]`.
    fn parse_tag_list_enclosed_in_brackets(input: &str) -> IResult<&str, &str> {
        let (input, output) = delimited(
            /* start */ tag(LEFT_BRACKET),
            /* output */ alt((
                is_not(RIGHT_BRACKET),
                is_not(LEFT_BRACKET)
            )),
            /* end */ tag(RIGHT_BRACKET),
        )(input)?;
        Ok((input, output))
    }

    /// Parse input: `tag1, tag2, tag3`.
    fn parse_tag_list_comma_separated(input: &str) -> IResult<&str, Vec<&str>> {
        let acc: Vec<&str> = input.split(COMMA).collect();
        let mut trimmed_acc: Vec<&str> = Vec::with_capacity(acc.len());

        // Verify whitespace prefix rules.
        match acc.len() {
            0 => {
                // Empty. Nothing to do here.
            }
            1 => {
                // Only one item. Must not be prefixed with a space.
                let only_item = &acc[0];
                if only_item.starts_with(SPACE) {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        "Only item must not start with space.",
                        nom::error::ErrorKind::Fail,
                    )));
                }
                else {
                    trimmed_acc.push(only_item);
                }
            },
            _ => {
                // More than one item:
                // 1. 1st item must not be prefixed with a space.
                // 2. 2nd item onwards must be prefixed by at least 1 space, may have more.
                let mut my_iter = acc.iter();

                let first_item = my_iter.next().unwrap();

                // First item must not be prefixed with a space.
                if first_item.starts_with(SPACE) {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        "First item must not start with space.",
                        nom::error::ErrorKind::Fail,
                    )));
                }else {
                    trimmed_acc.push(first_item);
                }

                // Rest of items must be prefixed with a space.
                for rest_item in my_iter {
                    if !rest_item.starts_with(SPACE) {
                        return Err(nom::Err::Error(nom::error::Error::new(
                            "Non-first item must start with space.",
                            nom::error::ErrorKind::Fail,
                        )));
                    }
                    // Can only trim 1 space from start of rest_item.
                    trimmed_acc.push(&rest_item[1..]);
                }
            },
        }

        Ok((input, trimmed_acc))
    }

}

#[cfg(test)]
mod test_parse_tags_opt_eol {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_not_quoted_no_eol() {
        let input = "@tags: [tag1, tag2, tag3]";
        let (input, output) = super::parse_tags_opt_eol(input).unwrap();
        assert_eq2!(input, "");
        assert_eq2!(output, list!["tag1", "tag2", "tag3"]);
    }

    #[test]
    fn test_not_quoted_no_eol_err_whitespace() {
        // First element mustn't have any space prefix.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [ tag1, tag2, tag3]").is_err(),
            true,
        );

        // 2nd element onwards must have a single space prefix.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1,tag2, tag3]").is_err(),
            true,
        );
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1,  tag2,tag3]").is_err(),
            true,
        );
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1, tag2,tag3]").is_err(),
            true,
        );

        // It is ok to have more than 1 prefix space for 2nd element onwards.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1, tag2,  tag3]").unwrap(),
            ("", list!["tag1", "tag2", " tag3"]),
        );
    }

    #[test]
    fn test_not_quoted_with_eol() {
        // Valid.
        {
            let input = "@tags: [tag1, tag2, tag3]\n";
            let (input, output) = parse_tags_opt_eol(input).unwrap();
            assert_eq2!(input, "");
            assert_eq2!(output, list!["tag1", "tag2", "tag3"]);
        }

        // Invalid: opening `[` must be terminated w/ `]`.
        {
            let input = "@tags: [tag1, tag2, tag3\n]\n";
            let result = parse_tags_opt_eol(input);
            assert_eq2!(result.is_err(), true);
        }
    }

    #[test]
    fn test_not_quoted_with_eol_whitespace() {
        // First element mustn't have any space prefix.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [ tag1, tag2, tag3]\n").is_err(),
            true,
        );

        // 2nd element onwards must have a single space prefix.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1,tag2, tag3]\n").is_err(),
            true,
        );
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1,  tag2,tag3]\n").is_err(),
            true,
        );
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1, tag2,tag3]\n").is_err(),
            true,
        );

        // It is ok to have more than 1 prefix space for 2nd element onwards.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1, tag2,  tag3]\n").unwrap(),
            ("", list!["tag1", "tag2", " tag3"]),
        );
    }
}

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
// 00: rename file to `parser_metadata_tags_opt_eol.rs`
// 00: use this instead of parse_tags()
pub fn parse_tags_opt_eol(input: &str) -> IResult<&str, List<&str>> {
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

    // 00: verify this works
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
                        only_item,
                        nom::error::ErrorKind::Tag,
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
                if first_item.starts_with(SPACE) {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        first_item,
                        nom::error::ErrorKind::Tag,
                    )));
                }else {
                    trimmed_acc.push(first_item);
                }

                let mut maybe_misfit = None;
                for rest_item in my_iter {
                    if !rest_item.starts_with(SPACE) {
                        maybe_misfit = Some(rest_item);
                        break;
                    }
                    // Can only trim 1 space from start of rest_item.
                    trimmed_acc.push(&rest_item[1..]);
                }

                if let Some(misfit) = maybe_misfit {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        misfit,
                        nom::error::ErrorKind::Tag,
                    )));
                }
            },
        }

        Ok((input, trimmed_acc))
    }

}

#[cfg(test)]
mod test_parse_tags_opt_eol {
    use r3bl_rs_utils_core::*;

    use super::*;

    #[test]
    fn test_not_quoted_no_eol() {
        // 00: impl this
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
            true
        );

        // 2nd element onwards must have a single space prefix.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1,tag2, tag3]").is_err(),
            true
        );
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1,  tag2,tag3]").is_err(),
            true
        );
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1, tag2,tag3]").is_err(),
            true
        );

        // It is ok to have more than 1 prefix space for 2nd element onwards.
        assert_eq2!(
            parse_tags_opt_eol("@tags: [tag1, tag2,  tag3]").unwrap(),
            ("", list!["tag1", "tag2", " tag3"])
        );
    }

    #[test]
    fn test_not_quoted_with_eol() {
        // 00: impl this
        let input = "@tags: [tag1, tag2, tag3]\n";
        let (input, output) = super::parse_tags_opt_eol(input).unwrap();
        assert_eq2!(input, "");
        assert_eq2!(output, list!["tag1", "tag2", "tag3"]);
    }
}

// 00: remove all the stuff below
// REFACTOR: make the requirement for `[` and `]` optional. Update tests.

/// Parse input: `@tags: [tag1, tag2, tag3]\n` or `tags: ["tag1", "tag2", "tag3"]\n`.
#[rustfmt::skip]
pub fn parse_tags(input: &str) -> IResult<&str, List<&str>> {
    let (input, output) = delimited(
        /* start */ tuple((tag(TAGS), tag(COLON), space0)),
        /* output */ is_not(NEW_LINE),
        /* end */ tag(NEW_LINE),
    )(input)?;

    // At this point, `output` can have something like: `[tag1, tag2, tag3]`.
    let (_, output) = parse_tag_list_enclosed_in_brackets(output)?;

    // At this point, `output` can have something like: `tag1, tag2, tag3`.
    let (_, output) = parse_tag_list_comma_separated(output)?;

    let it = List::from(output);

    return Ok((input, it));

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
        let (input, output) = separated_list1(
            /* separator */ tag(COMMA),
            /* output */ preceded(
                space0,
                is_not(COMMA),
            ),
        )(input)?;

        // At this point, `output` can have something like:
        // `tag1, tag2, tag3` or `"tag1", "tag2", "tag3"`.

        // Try and strip any quotes from the output. Also trim whitespace.
        let mut output_trimmed_unquoted = vec![];
        // AB: remove trim()
        for item in output.iter() {
            if let Ok((_, output)) = parse_quoted(item) {
                output_trimmed_unquoted.push(output.trim())
            } else {
                output_trimmed_unquoted.push(item.trim())
            }
        }
        Ok((input, output_trimmed_unquoted))
    }

    fn parse_quoted(input: &str) -> IResult<&str, &str> {
        // Make sure there are no new lines in the output.
        delimited(
            /* start */ tag(QUOTE),
            /* output */ is_not(QUOTE),
            /* end */ tag(QUOTE),
        )(input)
    }
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::assert_eq2;
    use raw_strings::*;

    use super::*;

    #[test]
    fn test_parse_metadata_tags() {
        let output = parse_tags(TAG_STRING_1);
        assert_eq2!(output, Ok(("", list!["tag 1", "tag 2", "tag3"])));

        let output = parse_tags(TAG_STRING_2);
        assert_eq2!(output, Ok(("", list!["tag 1", "tag 2", "tag 3"])));
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod raw_strings {
pub const TAG_STRING_1: &str =
r#"@tags: [tag 1 ,    tag 2, tag3]
"#;
pub const TAG_STRING_2: &str =
r#"@tags: ["tag 1 ", "  tag 2", "tag 3"]
"#;
}

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

// This module exists so that rustfmt can skip the formatting of the parser code.
#[rustfmt::skip]
pub mod no_rustfmt_block {
    use crate::*;
    use constants::*;
    use nom::{
        branch::*, bytes::complete::*, character::complete::*, combinator::*, multi::*, sequence::*,
        IResult,
    };

    /// Parse input: `@title: "Something"\n` or `@title: Something\n`.
    pub fn parse_title(input: &str) -> IResult<&str, &str> {
        return terminated(
            alt((
                parse_title_quoted,
                parse_title_not_quoted
            )),
            tag(NEW_LINE)
        )(input);

        /// Eg: `@title: "Something"\n`. The quotes are optional & EOL terminates input.
        fn parse_title_quoted(input: &str) -> IResult<&str, &str> {
            map(
                preceded(
                    /* start */ tuple((tag(TITLE), tag(COLON), space0)),
                    /* output */ parse_quoted,
                ),
                |it: &str| it.trim()
            )(input)
        }

        /// Eg: `@title: "Something"\n`. The quotes are optional & EOL terminates input.
        fn parse_title_not_quoted(input: &str) -> IResult<&str, &str> {
            map(
                preceded(
                    /* start */ tuple((tag(TITLE), tag(COLON), space0)),
                    /* output */ is_not(NEW_LINE),
                ),
                |it: &str| it.trim()
            )(input)
        }

    }

    pub fn parse_quoted(input: &str) -> IResult<&str, &str> {
        delimited(
            /* start */ tag(QUOTE),
            /* output */ is_not(QUOTE),
            /* end */ tag(QUOTE),
        )(input)
    }

    // TODO: make the requirement for `[` and `]` optional. Update tests.

    /// Parse input: `@tags: [tag1, tag2, tag3]\n` or `tags: ["tag1", "tag2", "tag3"]\n`.
    pub fn parse_tags(input: &str) -> IResult<&str, Vec<&str>> {
        let (input, output) = delimited(
            /* start */ tuple((tag(TAGS), tag(COLON), space0)),
            /* output */ is_not(NEW_LINE),
            /* end */ tag(NEW_LINE),
        )(input)?;

        // At this point, `output` can have something like: `[tag1, tag2, tag3]`.
        let (_, output) = parse_tag_list_enclosed_in_brackets(output)?;

        // At this point, `output` can have something like: `tag1, tag2, tag3`.
        let (_, output) = parse_tag_list_comma_separated(output)?;

        return Ok((input, output));

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
            for item in output.iter() {
                if let Ok((_, output)) = parse_quoted(item) {
                    output_trimmed_unquoted.push(output.trim())
                } else {
                    output_trimmed_unquoted.push(item.trim())
                }
            }
            Ok((input, output_trimmed_unquoted))
        }
    }
}
pub use no_rustfmt_block::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_impl_metadata::tests::raw_strings::*;

    #[test]
    fn test_parse_metadata_tags() {
        let output = parse_tags(TAG_STRING_1);
        assert_eq!(output, Ok(("", vec!["tag 1", "tag 2", "tag3"])));

        let output = parse_tags(TAG_STRING_2);
        assert_eq!(output, Ok(("", vec!["tag 1", "tag 2", "tag 3"])));
    }

    #[test]
    fn test_parse_metadata_title() {
        let output = parse_title(raw_strings::TITLE_STRING_1);
        assert_eq!(output, Ok(("", "Some title")));

        let output = parse_title(raw_strings::TITLE_STRING_2);
        assert_eq!(output, Ok(("", "Some title")));
    }

    #[rustfmt::skip]
    mod raw_strings {
pub const TITLE_STRING_1: &str =
r#"@title: Some title
"#;
pub const TITLE_STRING_2: &str =
r#"@title: "Some title  "
"#;
pub const TAG_STRING_1: &str =
r#"@tags: [tag 1 ,    tag 2, tag3]
"#;
pub const TAG_STRING_2: &str =
r#"@tags: ["tag 1 ", "  tag 2", "tag 3"]
"#;
    }
}

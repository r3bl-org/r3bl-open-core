// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use nom::{IResult, Parser, bytes::complete::tag, sequence::preceded};

use crate::{InlineVec, List, list,
            md_parser::constants::{COLON, COMMA, SPACE},
            parse_null_padded_line::trim_optional_leading_newline_and_nulls,
            take_text_in_single_line};

/// Parse comma-separated value metadata pairs.
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. The parser handles null
/// padding by consuming both newline and null characters at the end of the line.
///
/// - Sample parse input: `@tags: tag1, tag2, tag3`, `@tags: tag1, tag2, tag3\n`, or
///   `@authors: me, myself, i`, `@authors: me, myself, i\n`.
/// - There may or may not be a newline at the end. If there is, it is consumed.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't match the expected format.
pub fn parse_csv_opt_eol<'a>(
    tag_name: &'a str,
    input: &'a str,
) -> IResult<&'a str, List<&'a str>> {
    let (remainder, tags_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ take_text_in_single_line(),
    )
    .parse(input)?;

    // If there is a newline, consume it along with any null padding that follows.
    // to handle the ZeroCopyGapBuffer null padding invariant
    let remainder = trim_optional_leading_newline_and_nulls(remainder);

    // Special case: Early return when just a `@tags: ` or `@tags: \n` is found.
    if tags_text.is_empty() {
        Ok((remainder, list![]))
    }
    // Normal case.
    else {
        // At this point, `output` can have something like: `tag1, tag2, tag3`.
        let (_, vec_tags_text) = parse_comma_separated_list(tags_text)?;
        Ok((remainder, List::from(vec_tags_text)))
    }
}

/// | input                | rem     |  output                           |
/// | -------------------- | ------- | --------------------------------- |
/// | `"tag1, tag2, tag3"` | `""`    | `vec!(["tag1", "tag2", "tag3"])`  |
fn parse_comma_separated_list(input: &str) -> IResult<&str, InlineVec<&str>> {
    let acc: InlineVec<&str> = input.split(COMMA).collect();
    let mut trimmed_acc: InlineVec<&str> = InlineVec::with_capacity(acc.len());

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
            trimmed_acc.push(only_item);
        }
        _ => {
            // More than one item.
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
            }
            trimmed_acc.push(first_item);

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
        }
    }

    Ok((input, trimmed_acc))
}

#[cfg(test)]
mod test_parse_tags_opt_eol {
    use super::*;
    use crate::{assert_eq2, md_parser::constants::TAGS};

    #[test]
    fn test_not_quoted_no_eol() {
        let input = "@tags: tag1, tag2, tag3";
        let (input, output) = super::parse_csv_opt_eol(TAGS, input).unwrap();
        assert_eq2!(input, "");
        assert_eq2!(output, list!["tag1", "tag2", "tag3"]);
    }

    #[test]
    fn test_not_quoted_no_eol_err_whitespace() {
        // First fragment mustn't have any space prefix.
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags:  tag1, tag2, tag3").is_err(),
            true,
        );

        // 2nd fragment onwards must have a single space prefix.
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1,tag2, tag3").is_err(),
            true,
        );
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1,  tag2,tag3").is_err(),
            true,
        );
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1, tag2,tag3").is_err(),
            true,
        );

        // It is ok to have more than 1 prefix space for 2nd fragment onwards.
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1, tag2,  tag3").unwrap(),
            ("", list!["tag1", "tag2", " tag3"]),
        );
    }

    #[test]
    fn test_not_quoted_with_eol() {
        // Valid.
        {
            let input = "@tags: tag1, tag2, tag3\n";
            let (input, output) = parse_csv_opt_eol(TAGS, input).unwrap();
            assert_eq2!(input, "");
            assert_eq2!(output, list!["tag1", "tag2", "tag3"]);
        }

        {
            let input = "@tags: tag1, tag2, tag3\n]\n";
            let result = parse_csv_opt_eol(TAGS, input);
            assert_eq2!(result.is_err(), false);
        }

        {
            let input = "@tags: tag1, tag2, tag3";
            let result = parse_csv_opt_eol(TAGS, input);
            assert_eq2!(result.is_err(), false);
        }
    }

    #[test]
    fn test_not_quoted_with_eol_whitespace() {
        // First fragment mustn't have any space prefix.
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags:  tag1, tag2, tag3\n").is_err(),
            true,
        );

        // 2nd fragment onwards must have a single space prefix.
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1,tag2, tag3\n").is_err(),
            true,
        );
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1,  tag2,tag3\n").is_err(),
            true,
        );
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1, tag2,tag3\n").is_err(),
            true,
        );

        // It is ok to have more than 1 prefix space for 2nd fragment onwards.
        assert_eq2!(
            parse_csv_opt_eol(TAGS, "@tags: tag1, tag2,  tag3\n").unwrap(),
            ("", list!["tag1", "tag2", " tag3"]),
        );
    }

    #[test]
    fn test_not_quoted_with_postfix_content() {
        let input = "@tags: \nfoo\nbar";
        let (input, output) = parse_csv_opt_eol(TAGS, input).unwrap();
        assert_eq2!(input, "foo\nbar");
        assert_eq2!(output, list![]);
    }
}

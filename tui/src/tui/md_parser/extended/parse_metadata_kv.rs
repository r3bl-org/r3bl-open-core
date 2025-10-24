// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{md_parser::md_parser_constants::{COLON, SPACE},
            parse_null_padded_line::trim_optional_leading_newline_and_nulls,
            take_text_in_single_line, tiny_inline_string};
use nom::{IResult, Parser, bytes::complete::tag, sequence::preceded};

/// Parse key-value metadata pairs.
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0` characters,
/// as provided by `ZeroCopyGapBuffer::as_str()`. The parser handles null padding by consuming
/// both newline and null characters at the end of the line.
///
/// - Sample parse input: `@title: Something` or `@date: Else`.
/// - There may or may not be a newline at the end. If there is, it is consumed.
/// - Can't nest the `tag_name` within the `output`. So there can only be one `tag_name`
///   in the `output`.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't match the expected format.
#[rustfmt::skip]
pub fn parse_unique_kv_opt_eol<'a>(
    tag_name: &'a str,
    input: &'a str,
) -> IResult<&'a str, &'a str> {
    let (remainder, title_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ take_text_in_single_line(),
    )
    .parse(input)?;

    // Can't nest `tag_name` in `output`. Early return in this case.
    let tag_fragment = tiny_inline_string!("{tag_name}{COLON}{SPACE}");
    if title_text.contains(tag_fragment.as_str())
        | remainder.contains(tag_fragment.as_str())
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            "Can't have more than one tag_name in kv expr.",
            nom::error::ErrorKind::Fail,
        )));
    }

    // If there is a newline, consume it along with any null padding that follows
    // to handle the ZeroCopyGapBuffer null padding invariant.
    let remainder = trim_optional_leading_newline_and_nulls(remainder);

    // Special case: Early return when something like `@title: ` or `@title: \n` is found.
    if title_text.is_empty() {
        Ok((remainder, ""))
    }
    // Normal case.
    else {
        Ok((remainder, title_text))
    }
}

#[cfg(test)]
mod test_parse_title_no_eol {
    use super::*;
    use crate::{assert_eq2, fg_black, inline_string, md_parser::md_parser_constants::TITLE};

    #[test]
    fn test_not_quoted_no_eol() {
        let input = "@title: Something";
        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_not_quoted_with_eol() {
        let input = "@title: Something\n";
        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_no_quoted_no_eol_nested_title() {
        let input = "@title: Something @title: Something";
        let it = parse_unique_kv_opt_eol(TITLE, input);

        assert_eq2!(it.is_err(), true);
        if let Err(nom::Err::Error(ref e)) = it {
            assert_eq2!(e.input, "Can't have more than one tag_name in kv expr.");
            assert_eq2!(e.code, nom::error::ErrorKind::Fail);
        }

        println!(
            "err: '{}'",
            fg_black(&inline_string!("{:?}", it.err().unwrap())).bg_yellow(),
        );
    }

    #[test]
    fn test_no_quoted_no_eol_multiple_title_tags() {
        let input = "@title: Something\n@title: Else\n";
        let it = parse_unique_kv_opt_eol(TITLE, input);

        assert_eq2!(it.is_err(), true);
        if let Err(nom::Err::Error(ref e)) = it {
            assert_eq2!(e.input, "Can't have more than one tag_name in kv expr.");
            assert_eq2!(e.code, nom::error::ErrorKind::Fail);
        }

        println!(
            "err: '{}'",
            fg_black(&inline_string!("{:?}", it.err().unwrap())).bg_yellow(),
        );
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_1() {
        let input = "@title: \nfoo\nbar";
        println!("input: '{}'", fg_black(input).bg_cyan(),);

        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "foo\nbar");
        assert_eq2!(output, "");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_2() {
        let input = "@title:  a\nfoo\nbar";
        println!("input: '{}'", fg_black(input).bg_cyan(),);

        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "foo\nbar");
        assert_eq2!(output, " a");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_3() {
        let input = "@title: \n\n# heading1\n## heading2";
        println!("❯ input: \n'{}'", fg_black(input).bg_cyan(),);

        let (remainder, title) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "❯ remainder: \n'{r}'\n❯ title: \n'{t}'",
            r = fg_black(remainder).bg_yellow(),
            t = fg_black(title).bg_green(),
        );
        assert_eq2!(remainder, "\n# heading1\n## heading2");
        assert_eq2!(title, "");
    }
}

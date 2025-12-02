// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words aaabbb

use crate::md_parser::md_parser_constants::{NEW_LINE, NEWLINE_OR_NULL, NULL_CHAR};
use nom::{IResult, Parser,
          bytes::complete::{is_not, tag, take_while},
          combinator::opt,
          sequence::terminated};

/// Helper function that creates a predicate for matching a specific character.
/// This is the opposite of nom's `is_not()` - it matches only the specified char.
///
/// # Example
/// ```
/// use nom::{bytes::complete::take_while, IResult};
/// use r3bl_tui::is;
///
/// fn parser(input: &str) -> IResult<&str, &str> {
///     take_while(is('a'))(input)  // Takes all 'a' characters
/// }
///
/// let result = parser("aaabbb");
/// assert_eq!(result, Ok(("bbb", "aaa")));
/// ```
pub fn is(target: char) -> impl Fn(char) -> bool { move |c| c == target }

/// Helper function that creates a predicate for matching any of the specified characters.
/// This is useful for `take_till` and `take_while` when checking multiple characters.
///
/// # Example
/// ```
/// use nom::{bytes::complete::take_till1, IResult};
/// use r3bl_tui::is_any_of;
///
/// fn parser(input: &str) -> IResult<&str, &str> {
///     take_till1(is_any_of(&['\n', '\0']))(input)  // Takes until newline or null
/// }
///
/// let result = parser("hello\nworld");
/// assert_eq!(result, Ok(("\nworld", "hello")));
/// ```
pub fn is_any_of(targets: &'static [char]) -> impl Fn(char) -> bool {
    move |c| targets.contains(&c)
}

/// Helper function to trim an optional leading newline followed by any null padding.
///
/// This is commonly used after parsing content to handle the `ZeroCopyGapBuffer` null
/// padding invariant. The function trims from the start of the input:
/// - An optional newline character ('\n')
/// - Followed by zero or more null characters ('\0')
///
/// This function always succeeds and returns the remainder after optionally
/// trimming the leading newline and null padding pattern. If no such pattern is found
/// at the beginning of the input, it returns the original input unchanged.
///
/// # Examples
/// ```
/// use r3bl_tui::trim_optional_leading_newline_and_nulls;
///
/// // With newline and null padding at start
/// let remainder = trim_optional_leading_newline_and_nulls("\n\0\0world");
/// assert_eq!(remainder, "world");
///
/// // With just newline at start
/// let remainder = trim_optional_leading_newline_and_nulls("\nhello");
/// assert_eq!(remainder, "hello");
///
/// // Without pattern at start (returns unchanged)
/// let remainder = trim_optional_leading_newline_and_nulls("hello\n\0\0world");
/// assert_eq!(remainder, "hello\n\0\0world");
/// ```
#[must_use]
pub fn trim_optional_leading_newline_and_nulls(input: &str) -> &str {
    let result: IResult<&str, Option<(&str, &str)>> = opt((
        tag(NEW_LINE),
        /* zero or more */ take_while(is(NULL_CHAR)),
    ))
    .parse(input);

    match result {
        Ok((remainder, _)) => remainder,
        Err(_) => input, // This should never happen with opt(), but be safe
    }
}
/// Parse a line that ends with '\n' followed by optional '\0' padding.
///
/// Assumes input comes from `ZeroCopyGapBuffer` with its invariants:
/// - Non-null chars are content
/// - Lines end with \n followed by optional \0 padding
/// - If no \n found and we run out of input = EOI
///
/// # Examples
/// - "hello\n\0\0\0world" -> ("world", "hello")
/// - "hello" -> ("", "hello")
/// - "hello\n\0\0\0" -> ("", "hello")
///
/// # Errors
///
/// Returns a nom error if the parser fails to match any of the expected patterns.
/// This should not happen with valid `ZeroCopyGapBuffer` input.
#[rustfmt::skip]
pub fn parse_null_padded_line(input: &str) -> IResult<&str, &str> {
    use nom::branch::alt;
    use nom::combinator::value;

    alt((
        // Handle empty line case (starts with newline)
        value(
            "",
            (
                tag(NEW_LINE),
                take_while(is(NULL_CHAR))
            )
        ),
        // Normal case: content followed by optional newline and null padding
        terminated(
            // Take content (non-null, non-newline chars)
            is_not(NEWLINE_OR_NULL),
            // Consume newline + null padding if present.
            opt(
                (
                    tag(NEW_LINE),
                    take_while(is(NULL_CHAR))
                )
            )
        )
    )).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_is_helpers() {
        // Test is()
        let pred = is('a');
        assert!(pred('a'));
        assert!(!pred('b'));

        // Test is_any_of()
        let pred = is_any_of(&['a', 'b', 'c']);
        assert!(pred('a'));
        assert!(pred('b'));
        assert!(pred('c'));
        assert!(!pred('d'));
    }

    #[test]
    fn test_trim_optional_leading_newline_and_nulls() {
        // No newline
        {
            let input = "hello";
            let remainder = trim_optional_leading_newline_and_nulls(input);
            assert_eq2!(remainder, "hello");
        }

        // Newline only
        {
            let input = "\nhello";
            let remainder = trim_optional_leading_newline_and_nulls(input);
            assert_eq2!(remainder, "hello");
        }

        // Newline with null padding.
        {
            let input = "\n\0\0\0hello";
            let remainder = trim_optional_leading_newline_and_nulls(input);
            assert_eq2!(remainder, "hello");
        }

        // Just null chars (no newline) - should not consume
        {
            let input = "\0\0\0hello";
            let remainder = trim_optional_leading_newline_and_nulls(input);
            assert_eq2!(remainder, "\0\0\0hello");
        }

        // Empty input
        {
            let input = "";
            let remainder = trim_optional_leading_newline_and_nulls(input);
            assert_eq2!(remainder, "");
        }
    }

    #[test]
    fn test_parse_null_padded_line() {
        // Line with newline and null padding.
        {
            let input = "hello\n\0\0\0world";
            let (remainder, content) = parse_null_padded_line(input).unwrap();
            assert_eq2!(content, "hello");
            assert_eq2!(remainder, "world");
        }

        // Line with just newline, no null padding.
        {
            let input = "hello\nworld";
            let (remainder, content) = parse_null_padded_line(input).unwrap();
            assert_eq2!(content, "hello");
            assert_eq2!(remainder, "world");
        }

        // Line without newline (EOI)
        {
            let input = "hello";
            let (remainder, content) = parse_null_padded_line(input).unwrap();
            assert_eq2!(content, "hello");
            assert_eq2!(remainder, "");
        }

        // Empty line with null padding.
        {
            let input = "\n\0\0\0next";
            let (remainder, content) = parse_null_padded_line(input).unwrap();
            assert_eq2!(content, "");
            assert_eq2!(remainder, "next");
        }

        // Unicode content
        {
            let input = "Hello 👋 世界\n\0\0\0";
            let (remainder, content) = parse_null_padded_line(input).unwrap();
            assert_eq2!(content, "Hello 👋 世界");
            assert_eq2!(remainder, "");
        }

        // Multiple newlines (edge case)
        {
            let input = "line1\n\0\0\0line2\n\0\0\0";
            let (remainder, content) = parse_null_padded_line(input).unwrap();
            assert_eq2!(content, "line1");
            assert_eq2!(remainder, "line2\n\0\0\0");
        }
    }
}

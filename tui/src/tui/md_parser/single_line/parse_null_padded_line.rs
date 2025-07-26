/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use nom::{
    IResult, Parser,
    bytes::complete::{is_not, tag, take_while},
    combinator::opt,
    sequence::terminated,
};
use crate::md_parser::constants::{NEW_LINE, NULL_CHAR, NEWLINE_OR_NULL};

/// Helper function that creates a predicate for matching a specific character.
/// This is the opposite of nom's `is_not()` - it matches only the specified char.
/// 
/// # Example
/// ```
/// take_while(is('a'))  // Takes all 'a' characters
/// ```
pub fn is(target: char) -> impl Fn(char) -> bool {
    move |c| c == target
}

/// Helper function that creates a predicate for matching any of the specified characters.
/// This is useful for `take_till` and `take_while` when checking multiple characters.
/// 
/// # Example
/// ```
/// take_till1(is_any_of(&['\n', '\0']))  // Takes until newline or null
/// ```
pub fn is_any_of(targets: &'static [char]) -> impl Fn(char) -> bool {
    move |c| targets.contains(&c)
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
            // Consume newline + null padding if present
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
    fn test_parse_null_padded_line() {
        // Line with newline and null padding
        {
            let input = "hello\n\0\0\0world";
            let (remainder, content) = parse_null_padded_line(input).unwrap();
            assert_eq2!(content, "hello");
            assert_eq2!(remainder, "world");
        }

        // Line with just newline, no null padding
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

        // Empty line with null padding
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
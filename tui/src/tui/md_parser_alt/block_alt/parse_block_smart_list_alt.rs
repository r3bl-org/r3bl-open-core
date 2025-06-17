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

use nom::{branch::alt,
          bytes::complete::{is_not, tag},
          character::complete::{anychar, digit1, space0},
          combinator::{map, opt, recognize, verify},
          multi::{many0, many1},
          sequence::{preceded, terminated},
          IResult,
          Input,
          Parser};
use smallvec::smallvec;

use crate::{list,
            md_parser::constants::{CHECKED,
                                   LIST_PREFIX_BASE_WIDTH,
                                   NEW_LINE,
                                   ORDERED_LIST_PARTIAL_PREFIX,
                                   SPACE,
                                   SPACE_CHAR,
                                   UNCHECKED,
                                   UNORDERED_LIST_PREFIX},
            parse_inline_fragments_until_eol_or_eoi_alt,
            tiny_inline_string,
            AsStrSlice,
            BulletKind,
            CheckboxParsePolicy,
            InlineVec,
            Lines,
            List,
            MdLineFragment,
            MdLineFragments,
            SmartListIR,
            SmartListLine};

// TODO: parse_smart_list_content_lines_alt()
// TODO: mod tests_parse_list_item
// TODO: mod tests_list_item_lines
// TODO: mod tests_bullet_kinds
// TODO: mod tests_parse_indents
// TODO: parse_smart_list_alt()
// TODO: mod tests_parse_block_smart_list
// TODO: mod tests_parse_smart_lists_in_markdown

mod verify_rest {
    use super::*;

    /// Return true if:
    /// - No ul items (at any indent).
    /// - No other ol items with same indent + number.
    /// - No other ol items with any indent or number.
    pub fn list_contents_does_not_start_with_list_prefix<'a>(
        input: AsStrSlice<'a>,
    ) -> bool {
        let trimmed_input = input.trim_start_current_line();

        // Check for unordered list prefix
        if trimmed_input.starts_with(UNORDERED_LIST_PREFIX) {
            return false;
        }

        // Check for ordered list prefix (digit(s) followed by ". ")
        let input_str = trimmed_input.extract_to_line_end();

        // Find the position of the first non-digit character
        let mut digit_end = 0;
        for (i, c) in input_str.char_indices() {
            if !c.is_digit(10) {
                digit_end = i;
                break;
            }
            // If we reach the end of the string, all characters are digits
            if i == input_str.len() - 1 {
                digit_end = input_str.len();
                break;
            }
        }

        // If we found at least one digit, check if it's followed by ". "
        if digit_end > 0 {
            let rest = &input_str[digit_end..];
            if rest.starts_with(ORDERED_LIST_PARTIAL_PREFIX) {
                return false;
            }
        }

        // No list prefix found
        true
    }

    /// Verifies that a line starts with exactly the correct number of spaces
    /// for continuation of a list item. The number of spaces should match
    /// the length of the bullet string (which includes the bullet marker and following
    /// space).
    ///
    /// # Examples
    /// - `"  content"` with bullet `"- "` (len=2) => true (2 spaces match bullet length)
    /// - `"    content"` with bullet `"1. "` (len=3) => false (4 spaces != 3)
    /// - `" content"` with bullet `"- "` (len=2) => false (1 space != 2)
    /// - `"content"` with bullet `"- "` (len=2) => false (0 spaces != 2)
    pub fn must_start_with_correct_num_of_spaces<'a>(
        input: AsStrSlice<'a>,
        my_bullet: AsStrSlice<'a>,
    ) -> bool {
        let it_spaces_at_start = count_whitespace_at_start(input);
        let expected_spaces = my_bullet.input_len();
        it_spaces_at_start == expected_spaces
    }

    pub fn count_whitespace_at_start<'a>(input: AsStrSlice<'a>) -> usize {
        // Use position() to find the first non-space character
        // If found, that position is the count of leading spaces
        // If not found (all spaces or empty), return the total length
        input
            .position(|c| c != SPACE_CHAR)
            .unwrap_or(input.input_len())
    }
}

#[cfg(test)]
mod tests_verify_rest {
    use super::*;
    use crate::as_str_slice_test_case;

    #[test]
    fn test_list_contents_does_not_start_with_list_prefix() {
        // Test content that does NOT start with list prefixes (should return true)

        // Regular text content
        {
            as_str_slice_test_case!(input, "regular text content");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Content starting with spaces but no list prefix
        {
            as_str_slice_test_case!(input, "  indented content");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Content with dash but not followed by space
        {
            as_str_slice_test_case!(input, "-notalist");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Content with number but not followed by ". "
        {
            as_str_slice_test_case!(input, "123notalist");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Content with number followed by dot but no space
        {
            as_str_slice_test_case!(input, "1.notalist");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Empty string
        {
            as_str_slice_test_case!(input, "");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Only spaces
        {
            as_str_slice_test_case!(input, "   ");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Test content that DOES start with list prefixes (should return false)

        // Unordered list prefix at start
        {
            as_str_slice_test_case!(input, "- this is a list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Unordered list prefix with leading spaces
        {
            as_str_slice_test_case!(input, "  - indented list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Ordered list prefix at start - single digit
        {
            as_str_slice_test_case!(input, "1. this is ordered list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Ordered list prefix at start - multiple digits
        {
            as_str_slice_test_case!(input, "123. this is ordered list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Ordered list prefix with leading spaces
        {
            as_str_slice_test_case!(input, "    2. indented ordered list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Just the unordered list prefix
        {
            as_str_slice_test_case!(input, "- ");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Just the ordered list prefix
        {
            as_str_slice_test_case!(input, "42. ");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Multiple spaces before unordered list prefix
        {
            as_str_slice_test_case!(input, "      - deeply indented list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Multiple spaces before ordered list prefix
        {
            as_str_slice_test_case!(input, "        99. deeply indented ordered");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Edge case: zero as ordered list number
        {
            as_str_slice_test_case!(input, "0. zero numbered list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Edge case: very large number
        {
            as_str_slice_test_case!(input, "999999. large numbered list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Mixed whitespace before list prefix (tabs and spaces)
        {
            as_str_slice_test_case!(input, " \t - mixed whitespace");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, false);
        }

        // Test edge cases with special characters

        // Content starting with asterisk (not unordered list prefix)
        {
            as_str_slice_test_case!(input, "*not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Content starting with plus (not unordered list prefix)
        {
            as_str_slice_test_case!(input, "+not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // Content with Unicode characters
        {
            as_str_slice_test_case!(input, "😀 emoji content");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // List prefix in middle of line (should not be detected)
        {
            as_str_slice_test_case!(input, "some text - not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }

        // List prefix in middle of line (ordered)
        {
            as_str_slice_test_case!(input, "text 1. not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert_eq!(result, true);
        }
    }

    #[test]
    fn test_count_whitespace_at_start() {
        // Test with no leading spaces
        {
            as_str_slice_test_case!(input, "hello world");
            let count = verify_rest::count_whitespace_at_start(input);
            assert_eq!(count, 0);
        }

        // Test with leading spaces
        {
            as_str_slice_test_case!(input, "   hello world");
            let count = verify_rest::count_whitespace_at_start(input);
            assert_eq!(count, 3);
        }

        // Test with all spaces
        {
            as_str_slice_test_case!(input, "    ");
            let count = verify_rest::count_whitespace_at_start(input);
            assert_eq!(count, 4);
        }

        // Test with empty string
        {
            as_str_slice_test_case!(input, "");
            let count = verify_rest::count_whitespace_at_start(input);
            assert_eq!(count, 0);
        }

        // Test with single space
        {
            as_str_slice_test_case!(input, " ");
            let count = verify_rest::count_whitespace_at_start(input);
            assert_eq!(count, 1);
        }

        // Test with tabs and spaces (should only count spaces)
        {
            as_str_slice_test_case!(input, "  \thello");
            let count = verify_rest::count_whitespace_at_start(input);
            assert_eq!(count, 2); // Only spaces, not tabs
        }

        // Test with non-space whitespace at start
        {
            as_str_slice_test_case!(input, "\t  hello");
            let count = verify_rest::count_whitespace_at_start(input);
            assert_eq!(count, 0); // Tab is not a space character
        }
    }

    #[test]
    fn test_must_start_with_correct_num_of_spaces() {
        // Test case: 2 spaces with bullet "- " (length 2) => should be true
        {
            as_str_slice_test_case!(content, "  some content");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, true);
        }

        // Test case: 3 spaces with bullet "1. " (length 3) => should be true
        {
            as_str_slice_test_case!(content, "   more content");
            as_str_slice_test_case!(bullet, "1. ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, true);
        }

        // Test case: 4 spaces with bullet "1. " (length 3) => should be false
        {
            as_str_slice_test_case!(content, "    too many spaces");
            as_str_slice_test_case!(bullet, "1. ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, false);
        }

        // Test case: 1 space with bullet "- " (length 2) => should be false
        {
            as_str_slice_test_case!(content, " not enough spaces");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, false);
        }

        // Test case: 0 spaces with bullet "- " (length 2) => should be false
        {
            as_str_slice_test_case!(content, "no spaces");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, false);
        }

        // Test case: exact match with longer bullet "10. " (length 4)
        {
            as_str_slice_test_case!(content, "    content here");
            as_str_slice_test_case!(bullet, "10. ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, true);
        }

        // Test case: empty content with empty bullet
        {
            as_str_slice_test_case!(content, "");
            as_str_slice_test_case!(bullet, "");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, true);
        }

        // Test case: content with only spaces matching bullet length
        {
            as_str_slice_test_case!(content, "  ");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, true);
        }

        // Test case: content with only spaces not matching bullet length
        {
            as_str_slice_test_case!(content, "   ");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, false);
        }

        // Test case: content with mixed whitespace at start (only spaces should count)
        {
            as_str_slice_test_case!(content, " \tcontent");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, false); // Only 1 space, not 2
        }
    }

    #[test]
    fn test_must_start_with_correct_num_of_spaces_edge_cases() {
        // Test with Unicode characters in content (should not affect space counting)
        {
            as_str_slice_test_case!(content, "  😀 emoji content");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, true);
        }

        // Test with Unicode characters in bullet
        {
            as_str_slice_test_case!(content, "   content");
            as_str_slice_test_case!(bullet, "● "); // bullet character + space = 2 chars
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, false); // 3 spaces != 2 char bullet length
        }

        // Test with very long bullet
        {
            as_str_slice_test_case!(content, "      content");
            as_str_slice_test_case!(bullet, "100. "); // 5 characters
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, false); // 6 spaces != 5 char bullet length
        }

        // Test with very long bullet - correct match
        {
            as_str_slice_test_case!(content, "     content");
            as_str_slice_test_case!(bullet, "100. "); // 5 characters
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert_eq!(result, true); // 5 spaces == 5 char bullet length
        }
    }
}

/// Parse markdown text with a specific checkbox policy until the end of line or input.
/// This function is used as a utility for parsing markdown text that may contain checkboxes.
/// It returns a list of markdown line fragments [MdLineFragments].
///
/// Does not consume the end of line character if it exists. If an EOL character
/// [crate::constants::NEW_LINE] is found:
/// - The EOL character is not included in the output.
/// - The EOL character is not consumed, and is part of the remainder.
#[rustfmt::skip]
pub fn parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt<'a>(
    input: AsStrSlice<'a>,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    let (input, output) = many0(
        |it| parse_inline_fragments_until_eol_or_eoi_alt(it, checkbox_policy)
    ).parse(input)?;

    let it = List::from(output);

    Ok((input, it))
}

#[cfg(test)]
mod tests_checkbox_policy {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, list, MdLineFragment};

    #[test]
    fn test_ignore_checkbox_empty_string() {
        {
            as_str_slice_test_case!(input, "");
            let result = parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );

            let (remaining, fragments) = result.unwrap();
            assert_eq2!(remaining.is_empty(), true);
            assert_eq2!(fragments, list![]);
        }
    }

    #[test]
    fn test_ignore_checkbox_non_empty_string() {
        {
            as_str_slice_test_case!(
                input,
                "here is some plaintext *but what if we italicize?"
            );
            let result = parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );

            let (remaining, fragments) = result.unwrap();
            assert_eq2!(remaining.is_empty(), true);
            assert_eq2!(
                fragments,
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Plain("*"),
                    MdLineFragment::Plain("but what if we italicize?"),
                ]
            );
        }
    }
}

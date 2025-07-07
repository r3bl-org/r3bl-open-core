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

use std::fmt::Write as _;

use crate::{AsStrSlice, CodeBlockLine, CodeBlockLineContent, List, ParserByteCache,
            PARSER_BYTE_CACHE_PAGE_SIZE};

impl AsStrSlice<'_> {
    /// Write the content of this slice to a byte cache.
    ///
    /// This is for compatibility with the legacy markdown parser, which expects a [&str]
    /// input with trailing [`crate::constants::NEW_LINE`].
    ///
    /// ## Newline behavior
    ///
    /// - It adds a trailing [`crate::constants::NEW_LINE`] to the end of the `acc` in
    ///   case there is more than one line in `lines` field of [`AsStrSlice`].
    /// - For a single line, no trailing newline is added.
    /// - Empty lines are preserved with newlines.
    ///
    /// ## Incompatibility with [`str::lines()`]
    ///
    /// **Important**: This behavior is intentionally different from [`str::lines()`].
    /// When there are multiple lines and the last line is empty, this method will add
    /// a trailing newline, whereas [`str::lines()`] would not.
    ///
    /// This behavior is what was used in the legacy parser which takes [&str] as input,
    /// rather than [`AsStrSlice`].
    pub fn write_to_byte_cache_compat(
        &self,
        size_hint: usize,
        acc: &mut ParserByteCache,
    ) {
        // Clear the cache before writing to it. And size it correctly.
        acc.clear();
        let amount_to_reserve = {
            // Increase the capacity of the acc if necessary by rounding up to the
            // nearest PARSER_BYTE_CACHE_PAGE_SIZE.
            let page_size = PARSER_BYTE_CACHE_PAGE_SIZE;
            let current_capacity = acc.capacity();
            if size_hint > current_capacity {
                let bytes_needed: usize = size_hint - current_capacity;
                // Round up bytes_needed to the nearest page_size.
                let pages_needed = bytes_needed.div_ceil(page_size);
                pages_needed * page_size
            } else {
                0
            }
        };
        acc.reserve(amount_to_reserve);

        if self.lines.is_empty() {
            return;
        }

        // Use the Display implementation which already handles the correct newline
        // behavior.
        // We don't care about the result of this operation.
        write!(acc, "{self}").ok();
    }
}

#[allow(clippy::doc_markdown)]
/// Shared function used by both old and new code block parsers.
///
/// At a minimum, a [`CodeBlockLine`] will be 2 lines of text.
/// 1. The first line will be the language of the code block, eg: "```rs\n" or "```\n".
/// 2. The second line will be the end of the code block, eg: "```\n" Then there may be
///    some number of lines of text in the middle. These lines are stored in the
///    [content](CodeBlockLine.content) field.
#[must_use]
pub fn convert_into_code_block_lines<'a>(
    lang: Option<&'a str>,
    lines: Vec<&'a str>,
) -> List<CodeBlockLine<'a>> {
    let mut acc = List::with_capacity(lines.len() + 2);

    acc += CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::StartTag,
    };

    for line in lines {
        acc += CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::Text(line),
        };
    }

    acc += CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::EndTag,
    };

    acc
}

/// These tests ensure compatibility with how [`AsStrSlice::write_to_byte_cache_compat()`]
/// works. And ensuring that the [`AsStrSlice`] methods that are used to implement the
/// [`Display`] trait do in fact make it behave like a "virtual" array or slice of strings
/// that matches the behavior of [`AsStrSlice::write_to_byte_cache_compat()`].
///
/// This breaks compatibility with [`str::lines()`] behavior, but matches the behavior of
/// [`AsStrSlice::write_to_byte_cache_compat()`] which adds trailing newlines for multiple
/// lines.
#[cfg(test)]
mod tests_write_to_byte_cache_compat_behavior {
    use crate::{as_str_slice_test_case, AsStrSlice, GCString, ParserByteCache};

    #[test]
    fn test_empty_string() {
        // Empty lines behavior.
        {
            let lines: Vec<GCString> = vec![];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), "");
            assert_eq!(slice.lines.len(), 0);
        }
    }

    #[test]
    fn test_single_char_no_newline() {
        // Single line behavior - no trailing newline for single lines.
        {
            as_str_slice_test_case!(slice, "a");
            assert_eq!(slice.to_inline_string(), "a");
            assert_eq!(slice.lines.len(), 1);
        }
    }

    #[test]
    fn test_two_chars_with_trailing_newline() {
        // Multiple lines behavior - adds trailing newline for multiple lines.
        {
            as_str_slice_test_case!(slice, "a", "b");
            assert_eq!(slice.to_inline_string(), "a\nb\n"); // Trailing \n added
            assert_eq!(slice.lines.len(), 2);
        }
    }

    #[test]
    fn test_three_chars_with_trailing_newline() {
        // Multiple lines behavior - adds trailing newline for multiple lines.
        {
            as_str_slice_test_case!(slice, "a", "b", "c");
            assert_eq!(slice.to_inline_string(), "a\nb\nc\n"); // Trailing \n added
            assert_eq!(slice.lines.len(), 3);
        }
    }

    #[test]
    fn test_empty_lines_with_trailing_newline() {
        // Empty lines are preserved with newlines, plus trailing newline.
        {
            as_str_slice_test_case!(slice, "", "a", "");
            assert_eq!(slice.to_inline_string(), "\na\n\n"); // Each line followed by \n
            assert_eq!(slice.lines.len(), 3);
        }
    }

    #[test]
    fn test_only_empty_lines() {
        // Multiple empty lines get trailing newline.
        {
            as_str_slice_test_case!(slice, "", "");
            assert_eq!(slice.to_inline_string(), "\n\n"); // Two newlines plus trailing
            assert_eq!(slice.lines.len(), 2);
        }
    }

    #[test]
    fn test_single_empty_line() {
        // Single empty line gets no trailing newline.
        {
            as_str_slice_test_case!(slice, "");
            assert_eq!(slice.to_inline_string(), ""); // No trailing newline for single line
            assert_eq!(slice.lines.len(), 1);
        }
    }

    #[test]
    fn test_verify_write_to_byte_cache_compat_consistency() {
        let test_helper = |slice: AsStrSlice<'_>| {
            let slice_result = slice.to_inline_string();

            // Get write_to_byte_cache_compat result
            let mut cache = ParserByteCache::new();
            slice.write_to_byte_cache_compat(slice_result.len() + 10, &mut cache);
            let cache_result = cache.as_str();

            // They should match exactly
            assert_eq!(
                slice_result, cache_result,
                "Mismatch: AsStrSlice produced {slice_result:?}, write_to_byte_cache_compat produced {cache_result:?}"
            );
        };

        // Empty
        {
            let slice = AsStrSlice::from(&[]);
            test_helper(slice);
        }

        // Single line
        {
            as_str_slice_test_case!(slice, "single");
            test_helper(slice);
        }

        // Two lines
        {
            as_str_slice_test_case!(slice, "a", "b");
            test_helper(slice);
        }

        // With empty lines
        {
            as_str_slice_test_case!(slice, "", "middle", "");
            test_helper(slice);
        }

        // Only empty lines
        {
            as_str_slice_test_case!(slice, "", "");
            test_helper(slice);
        }
    }

    #[test]
    fn test_compare_with_str_lines() {
        // This test explicitly demonstrates the incompatibility with str::lines()
        // when there are multiple lines and the last line is empty.

        // Case 1: Multiple lines with empty last line
        {
            // Create a string with multiple lines and empty last line
            let str_with_empty_last_line = "line1\nline2\n";

            // Using str::lines()
            let str_lines: Vec<&str> = str_with_empty_last_line.lines().collect();
            assert_eq!(str_lines, vec!["line1", "line2"]); // str::lines() ignores the empty last line

            // Using AsStrSlice
            as_str_slice_test_case!(slice, "line1", "line2");
            let slice_result = slice.to_inline_string();
            assert_eq!(slice_result.as_str(), "line1\nline2\n"); // AsStrSlice preserves the trailing newline

            // Demonstrate the difference
            let reconstructed_from_str_lines = str_lines.join("\n");
            assert_eq!(reconstructed_from_str_lines, "line1\nline2"); // No trailing newline
            assert_ne!(reconstructed_from_str_lines, slice_result.as_str()); // Different from AsStrSlice
        }

        // Case 2: Multiple lines with non-empty last line
        {
            // Create a string with multiple lines and non-empty last line
            let str_with_non_empty_last_line = "line1\nline2";

            // Using str::lines()
            let str_lines: Vec<&str> = str_with_non_empty_last_line.lines().collect();
            assert_eq!(str_lines, vec!["line1", "line2"]);

            // Using AsStrSlice
            as_str_slice_test_case!(slice, "line1", "line2");
            let slice_result = slice.to_inline_string();
            assert_eq!(slice_result.as_str(), "line1\nline2\n"); // AsStrSlice adds a trailing newline

            // Demonstrate the difference
            let reconstructed_from_str_lines = str_lines.join("\n");
            assert_eq!(reconstructed_from_str_lines, "line1\nline2"); // No trailing newline
            assert_ne!(reconstructed_from_str_lines, slice_result.as_str()); // Different from AsStrSlice
        }
    }
}

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

use crate::{as_str_slice::AsStrSlice,
            constants::{SPACE_CHAR, TAB_CHAR},
            core::units::{len, Length}};

impl AsStrSlice<'_> {
    /// Internal helper function that trims specified whitespace characters from the start
    /// of the current line. This only operates on the contents of the current line.
    ///
    /// ⚠️ **Important: ASCII-Only Whitespace Trimming**
    ///
    /// This method **only trims specified ASCII characters**, not Unicode whitespace
    /// characters. This is the correct behavior for Markdown parsing, as the Markdown
    /// specification typically only recognizes ASCII spaces for list indentation and
    /// text formatting.
    ///
    /// # Parameters
    /// - `whitespace_chars`: A slice of characters to be considered as whitespace
    ///
    /// # Returns
    /// A tuple containing:
    /// - The number of characters trimmed
    /// - The trimmed `AsStrSlice` instance
    #[must_use]
    pub fn trim_whitespace_chars_start_current_line(
        &self,
        whitespace_chars: &[char],
    ) -> (Length, Self) {
        let mut result = self.clone();
        let mut chars_trimmed = len(0);

        // Use advance() instead of manual manipulation.
        loop {
            match result.current_char() {
                Some(ch) if whitespace_chars.contains(&ch) => {
                    result.advance(); // This properly handles both char_index and max_len
                    chars_trimmed += len(1);
                }
                _ => break,
            }
        }

        (chars_trimmed, result)
    }

    /// Remove leading whitespace from the start of the slice. This only operates on the
    /// contents of the current line. Whitespace includes [`SPACE_CHAR`] and [`TAB_CHAR`].
    ///
    /// ⚠️ **Important: ASCII-Only Whitespace Trimming**
    ///
    /// This method **only trims ASCII spaces (U+0020) and ASCII tabs (U+0009)**, not
    /// Unicode whitespace characters like em space (U+2003), non-breaking space
    /// (U+00A0), or other Unicode whitespace. This is the correct behavior for
    /// Markdown parsing, as the Markdown specification typically only recognizes
    /// ASCII spaces for list indentation and text formatting.
    ///
    /// # Examples
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// // ASCII whitespace - will be trimmed
    /// as_str_slice_test_case!(slice, "  \thello");
    /// let trimmed = slice.trim_start_current_line();
    /// assert_eq!(trimmed.extract_to_line_end(), "hello");
    ///
    /// // Unicode whitespace - will NOT be trimmed
    /// as_str_slice_test_case!(slice2, " \u{2003}hello");  // space + em space
    /// let trimmed = slice2.trim_start_current_line();
    /// assert_eq!(trimmed.extract_to_line_end(), "\u{2003}hello");  // em space preserved
    /// ```
    ///
    /// This behavior ensures consistent Markdown parsing where Unicode spaces don't count
    /// as valid indentation for lists and other block elements.
    #[must_use]
    pub fn trim_start_current_line(&self) -> Self {
        let (_, trimmed_slice) =
            self.trim_whitespace_chars_start_current_line(&[SPACE_CHAR, TAB_CHAR]);
        trimmed_slice
    }

    /// Handy check to verify that if the leading whitespace are trimmed from the start
    /// of the current line, it is just an empty string. Needed for some parsers in
    /// smart lists.
    ///
    /// ⚠️ **Note**: This method only considers ASCII spaces and tabs as whitespace
    /// (via `trim_start_current_line()`), not Unicode whitespace characters. This
    /// ensures consistent Markdown parsing behavior.
    #[must_use]
    pub fn trim_start_current_line_is_empty(&self) -> bool {
        self.trim_start_current_line()
            .extract_to_line_end()
            .is_empty()
    }

    /// Similar to [`Self::trim_start_current_line()`], but it trims leading spaces
    /// and returns the number of space characters trimmed from the start
    /// and the trimmed [`AsStrSlice`] instance.
    #[must_use]
    pub fn trim_spaces_start_current_line(&self) -> (Length, Self) {
        self.trim_whitespace_chars_start_current_line(&[SPACE_CHAR])
    }
}

#[cfg(test)]
mod tests_trim_whitespace_chars_start_current_line {
    use nom::Input;

    use crate::{as_str_slice_test_case, assert_eq2, idx, len, AsStrSlice, GCString};

    #[test]
    fn test_no_whitespace_to_trim() {
        as_str_slice_test_case!(slice, "hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_single_space() {
        as_str_slice_test_case!(slice, " hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(1));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_multiple_spaces() {
        as_str_slice_test_case!(slice, "   hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_mixed_whitespace() {
        as_str_slice_test_case!(slice, " \t  hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(4));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_only_specific_whitespace_chars() {
        as_str_slice_test_case!(slice, " \t\nhello world", "second line");
        let whitespace_chars = [' ', '\t']; // Don't include '\n'

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2)); // Should stop at '\n'
        assert_eq2!(result.current_char(), Some('\n'));
    }

    #[test]
    fn test_trim_entire_line_of_whitespace() {
        as_str_slice_test_case!(slice, "   \t  ", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(6));
        assert_eq2!(result.current_char(), Some('\n')); // At synthetic newline
    }

    #[test]
    fn test_trim_with_unicode_content() {
        as_str_slice_test_case!(slice, "  😀hello🌟world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2));
        assert_eq2!(result.current_char(), Some('😀'));
        assert_eq2!(result.extract_to_line_end(), "😀hello🌟world");
    }

    #[test]
    fn test_trim_unicode_whitespace() {
        // Test with Unicode whitespace characters
        as_str_slice_test_case!(slice, "\u{2000}\u{2001}hello", "second line"); // en-space, em-space
        let whitespace_chars = ['\u{2000}', '\u{2001}', ' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello");
    }

    #[test]
    fn test_trim_with_max_len_limit() {
        as_str_slice_test_case!(slice, limit: 10, "   hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.max_len, Some(len(7))); // Original 10 - 3 trimmed = 7
    }

    #[test]
    fn test_trim_with_max_len_exhausted_by_whitespace() {
        as_str_slice_test_case!(slice, limit: 3, "   hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), None); // Max length reached
        assert_eq2!(result.max_len, Some(len(0))); // All consumed
    }

    #[test]
    fn test_trim_with_max_len_partial_whitespace() {
        as_str_slice_test_case!(slice, limit: 3, "     hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3)); // Can only trim 3 due to max_len
        assert_eq2!(result.current_char(), None); // Hit max_len limit
        assert_eq2!(result.max_len, Some(len(0)));
    }

    #[test]
    fn test_trim_starting_mid_line() {
        as_str_slice_test_case!(slice_orig, "abc   def", "second line");
        let slice = slice_orig.take_from(3); // Start at the spaces
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('d'));
        assert_eq2!(result.extract_to_line_end(), "def");
    }

    #[test]
    fn test_trim_empty_line() {
        as_str_slice_test_case!(slice, "", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), Some('\n')); // At synthetic newline
                                                        // immediately
    }

    #[test]
    fn test_trim_single_line_input() {
        as_str_slice_test_case!(slice, "  hello");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello");
    }

    #[test]
    fn test_trim_does_not_cross_line_boundaries() {
        as_str_slice_test_case!(slice, "hello", "  world");
        let slice = slice.take_from(5); // Position at synthetic newline after "hello"
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        // Should advance past the synthetic newline and then trim spaces on the next line
        assert_eq2!(chars_trimmed, len(3)); // Newline + 2 spaces
        assert_eq2!(result.line_index, idx(1)); // Moved to second line
        assert_eq2!(result.char_index, idx(2)); // After spaces
    }

    #[test]
    fn test_trim_with_custom_whitespace_set() {
        as_str_slice_test_case!(slice, ".,!hello world", "second line");
        let custom_chars = ['.', ',', '!'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&custom_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_no_matching_chars() {
        as_str_slice_test_case!(slice, "hello world", "second line");
        let custom_chars = ['.', ',', '!'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&custom_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result, slice); // Should be unchanged
    }

    #[test]
    fn test_trim_position_consistency() {
        as_str_slice_test_case!(slice, "   hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        // Verify that the result is consistent with manual advancement
        let mut manual_result = slice.clone();
        for _ in 0..chars_trimmed.as_usize() {
            manual_result.advance();
        }

        assert_eq2!(result.line_index, manual_result.line_index);
        assert_eq2!(result.char_index, manual_result.char_index);
        assert_eq2!(result.current_char(), manual_result.current_char());
    }

    #[test]
    fn test_trim_with_very_long_whitespace() {
        let long_whitespace = " ".repeat(100);
        let line = format!("{long_whitespace}hello");
        let line_1 = GCString::new(line);
        let line_2 = GCString::new("second line");
        let lines = vec![line_1, line_2];
        let slice = AsStrSlice::from(&lines);
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(100));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello");
    }

    #[test]
    fn test_trim_edge_case_at_line_end() {
        as_str_slice_test_case!(slice, "hello   ", "second line");
        let slice = slice.take_from(5); // Position at first space after "hello"
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('\n')); // At synthetic newline
    }

    #[test]
    fn test_trim_preserves_current_taken_tracking() {
        as_str_slice_test_case!(slice, "  hello world", "second line");
        let original_taken = slice.current_taken;
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(result.current_taken, original_taken + chars_trimmed);
    }

    #[test]
    fn test_trim_with_zero_length_input() {
        as_str_slice_test_case!(slice, limit: 0, "hello");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), None);
        assert_eq2!(result, slice); // Should be unchanged
    }
}

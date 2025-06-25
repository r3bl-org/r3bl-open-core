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

use nom::Input;

use crate::{as_str_slice::AsStrSlice, core::tui_core::units::Length, CharacterIndex};

/// Character-based range methods for safe Unicode/UTF-8 text processing.
///
/// These methods provide convenient range operations that work with character positions
/// instead of byte positions, ensuring proper Unicode handling.
impl<'a> AsStrSlice<'a> {
    /// Character-based range [start..end] - safe for Unicode/UTF-8 text.
    ///
    /// Returns a slice containing characters from `start` to `end` (exclusive).
    /// This is equivalent to `self.take_from(start).take(end - start)` but with
    /// bounds checking and clearer semantics.
    ///
    /// # ⚠️ Critical: Character-Based Indexing
    ///
    /// This method uses **character positions**, not byte positions. This is essential
    /// for proper Unicode/UTF-8 support.
    ///
    /// # Examples
    ///
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case, assert_eq2};
    /// as_str_slice_test_case!(slice, "😀hello world");
    /// let range = slice.char_range(1, 6); // "hello" (5 characters after emoji)
    /// assert_eq2!(range.extract_to_line_end(), "hello");
    /// ```
    ///
    /// # Invalid start and end index
    /// This method will return an empty `AsStrSlice` if `start > end`.
    ///
    /// # Parameters
    /// - `start`: The starting character position (inclusive)
    /// - `end`: The ending character position (exclusive)
    pub fn char_range(
        &self,
        arg_start: impl Into<CharacterIndex>,
        arg_end: impl Into<CharacterIndex>,
    ) -> Self {
        let start = arg_start.into().as_usize();
        let end = arg_end.into().as_usize();

        if start > end {
            // Return empty slice.
            return Self::with_limit(
                self.lines,
                self.line_index,
                self.char_index,
                Some(Length::from(0)),
            );
        }

        self.take_from(start).take(end - start)
    }

    /// Character-based range [start..] - safe for Unicode/UTF-8 text.
    ///
    /// Returns a slice starting from the specified character position to the end
    /// of the slice. This is equivalent to `self.take_from(start)`.
    ///
    /// # ⚠️ Critical: Character-Based Indexing
    ///
    /// This method uses **character positions**, not byte positions.
    ///
    /// # Examples
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case, assert_eq2};
    /// as_str_slice_test_case!(slice, "😀hello world");
    /// let from_second = slice.char_from(1); // "hello world" (after emoji)
    /// assert_eq2!(from_second.extract_to_line_end(), "hello world");
    /// ```
    ///
    /// # Parameters
    /// - `start`: The starting character position (inclusive)
    pub fn char_from(&self, arg_start: impl Into<CharacterIndex>) -> Self {
        let start = arg_start.into().as_usize();
        self.take_from(start)
    }

    /// Character-based range [..end] - safe for Unicode/UTF-8 text.
    ///
    /// Returns a slice from the beginning to the specified character position
    /// (exclusive). This is equivalent to `self.take(end)`.
    ///
    /// # ⚠️ Critical: Character-Based Indexing
    ///
    /// This method uses **character positions**, not byte positions.
    ///
    /// # Examples
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// as_str_slice_test_case!(slice, "😀hello world");
    /// let first_six = slice.char_to(6); // "😀hello" (emoji + 5 chars)
    /// assert_eq!(first_six.extract_to_line_end(), "😀hello");
    /// ```
    ///
    /// # Parameters
    /// - `end`: The ending character position (exclusive)
    pub fn char_to(&self, arg_end: impl Into<CharacterIndex>) -> Self {
        let end = arg_end.into().as_usize();
        self.take(end)
    }

    /// Character-based range [start..=end] - safe for Unicode/UTF-8 text.
    ///
    /// Returns a slice containing characters from `start` to `end` (inclusive).
    /// This is equivalent to `self.take_from(start).take(end - start + 1)` but with
    /// bounds checking and clearer semantics.
    ///
    /// # ⚠️ Critical: Character-Based Indexing
    ///
    /// This method uses **character positions**, not byte positions.
    ///
    /// # Examples
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// as_str_slice_test_case!(slice, "😀hello world");
    /// let range = slice.char_range_inclusive(1, 5); // "hello" (inclusive range)
    /// assert_eq!(range.extract_to_line_end(), "hello");
    /// ```
    ///
    /// # Invalid start and end index
    /// This method will return an empty `AsStrSlice` if `start > end`.
    ///
    /// # Parameters
    /// - `start`: The starting character position (inclusive)
    /// - `end`: The ending character position (inclusive)
    pub fn char_range_inclusive(
        &self,
        arg_start: impl Into<CharacterIndex>,
        arg_end: impl Into<CharacterIndex>,
    ) -> Self {
        let start = arg_start.into().as_usize();
        let end = arg_end.into().as_usize();

        if start > end {
            // Return empty slice.
            return Self::with_limit(
                self.lines,
                self.line_index,
                self.char_index,
                Some(Length::from(0)),
            );
        }

        self.take_from(start).take(end - start + 1)
    }
}

#[cfg(test)]
mod tests_character_based_range_methods {
    use crate::{as_str_slice_test_case, assert_eq2, len};

    #[test]
    fn test_char_range() {
        // Test basic ASCII range
        {
            as_str_slice_test_case!(slice, "hello world");
            let range = slice.char_range(0, 5); // "hello"
            assert_eq2!(range.extract_to_line_end(), "hello");
        }

        // Test range with Unicode characters
        {
            as_str_slice_test_case!(slice, "😀hello world");
            let range = slice.char_range(1, 6); // "hello" (after emoji)
            assert_eq2!(range.extract_to_line_end(), "hello");
        }

        // Test range in middle of text
        {
            as_str_slice_test_case!(slice, "😀hello world🎉");
            let range = slice.char_range(6, 12); // " world"
            assert_eq2!(range.extract_to_line_end(), " world");
        }

        // Test empty range (start == end)
        {
            as_str_slice_test_case!(slice, "hello");
            let range = slice.char_range(2, 2); // empty
            assert_eq2!(range.extract_to_line_end(), "");
        }

        // Test range at start
        {
            as_str_slice_test_case!(slice, "😀🎉hello");
            let range = slice.char_range(0, 2); // "😀🎉"
            assert_eq2!(range.extract_to_line_end(), "😀🎉");
        }

        // Test range at end
        {
            as_str_slice_test_case!(slice, "hello😀🎉");
            let range = slice.char_range(5, 7); // "😀🎉"
            assert_eq2!(range.extract_to_line_end(), "😀🎉");
        }

        // Test multiline content
        {
            as_str_slice_test_case!(slice, "😀hello", "world🎉");
            let range = slice.char_range(1, 6); // "hello" from first line
            assert_eq2!(range.extract_to_line_end(), "hello");
        }
    }

    #[test]
    fn test_char_range_invalid_start_greater_than_end() {
        // This test should now verify that char_range returns an empty slice
        // instead of panicking when start > end
        as_str_slice_test_case!(input, "Hello", "World");
        let result = input.char_range(5, 3);
        assert_eq2!(result.is_empty(), true);
        assert_eq2!(result.len_chars(), len(0));
    }

    #[test]
    fn test_char_from() {
        // Test basic ASCII
        {
            as_str_slice_test_case!(slice, "hello world");
            let from_five = slice.char_from(5); // " world"
            assert_eq2!(from_five.extract_to_line_end(), " world");
        }

        // Test with Unicode characters
        {
            as_str_slice_test_case!(slice, "😀hello world");
            let from_one = slice.char_from(1); // "hello world" (after emoji)
            assert_eq2!(from_one.extract_to_line_end(), "hello world");
        }

        // Test from start
        {
            as_str_slice_test_case!(slice, "😀hello");
            let from_zero = slice.char_from(0); // "😀hello"
            assert_eq2!(from_zero.extract_to_line_end(), "😀hello");
        }

        // Test from end
        {
            as_str_slice_test_case!(slice, "hello😀");
            let from_six = slice.char_from(6); // ""
            assert_eq2!(from_six.extract_to_line_end(), "");
        }

        // Test multiline content
        {
            as_str_slice_test_case!(slice, "😀hello", "world🎉");
            let from_six = slice.char_from(6); // Should start from newline between lines
                                               // This extracts across lines with synthetic newlines
            let content = from_six.extract_to_slice_end();
            assert!(content.as_ref().contains("world🎉"));
        }
    }

    #[test]
    fn test_char_to() {
        // Test basic ASCII
        {
            as_str_slice_test_case!(slice, "hello world");
            let to_five = slice.char_to(5); // "hello"
            assert_eq2!(to_five.extract_to_line_end(), "hello");
        }

        // Test with Unicode characters
        {
            as_str_slice_test_case!(slice, "😀hello world");
            let to_six = slice.char_to(6); // "😀hello"
            assert_eq2!(to_six.extract_to_line_end(), "😀hello");
        }

        // Test to start (empty)
        {
            as_str_slice_test_case!(slice, "😀hello");
            let to_zero = slice.char_to(0); // ""
            assert_eq2!(to_zero.extract_to_line_end(), "");
        }

        // Test to end
        {
            as_str_slice_test_case!(slice, "hello😀");
            let to_six = slice.char_to(6); // "hello😀"
            assert_eq2!(to_six.extract_to_line_end(), "hello😀");
        }

        // Test beyond end (should be limited)
        {
            as_str_slice_test_case!(slice, "hi");
            let to_ten = slice.char_to(10); // "hi" (limited to actual length)
            assert_eq2!(to_ten.extract_to_line_end(), "hi");
        }

        // Test multiline content
        {
            as_str_slice_test_case!(slice, "😀hello", "world🎉");
            let to_six = slice.char_to(6); // Should get "😀hello" from first line
            assert_eq2!(to_six.extract_to_line_end(), "😀hello");
        }
    }

    #[test]
    fn test_char_range_inclusive() {
        // Test basic ASCII range
        {
            as_str_slice_test_case!(slice, "hello world");
            let range = slice.char_range_inclusive(0, 4); // "hello"
            assert_eq2!(range.extract_to_line_end(), "hello");
        }

        // Test range with Unicode characters
        {
            as_str_slice_test_case!(slice, "😀hello world");
            let range = slice.char_range_inclusive(1, 5); // "hello" (after emoji)
            assert_eq2!(range.extract_to_line_end(), "hello");
        }

        // Test single character range
        {
            as_str_slice_test_case!(slice, "😀hello");
            let range = slice.char_range_inclusive(0, 0); // "😀"
            assert_eq2!(range.extract_to_line_end(), "😀");
        }

        // Test range in middle of text
        {
            as_str_slice_test_case!(slice, "😀hello world🎉");
            let range = slice.char_range_inclusive(6, 11); // " world"
            assert_eq2!(range.extract_to_line_end(), " world");
        }

        // Test range at end
        {
            as_str_slice_test_case!(slice, "hello😀🎉");
            let range = slice.char_range_inclusive(5, 6); // "😀🎉"
            assert_eq2!(range.extract_to_line_end(), "😀🎉");
        }

        // Test multiline content
        {
            as_str_slice_test_case!(slice, "😀hello", "world🎉");
            let range = slice.char_range_inclusive(1, 5); // "hello" from first line
            assert_eq2!(range.extract_to_line_end(), "hello");
        }
    }

    #[test]
    fn test_char_range_inclusive_invalid_start_greater_than_end() {
        // This test should now verify that char_range_inclusive returns an empty slice
        // instead of panicking when start > end
        as_str_slice_test_case!(input, "Hello", "World");
        let result = input.char_range_inclusive(5, 3);
        assert_eq2!(result.is_empty(), true);
        assert_eq2!(result.len_chars(), len(0));
    }

    #[test]
    fn test_char_range_methods_equivalence() {
        // Test that char_range(a, b) = = char_from(a).char_to(b-a)
        {
            as_str_slice_test_case!(slice, "😀hello world🎉");
            let range1 = slice.char_range(2, 8);
            let range2 = slice.char_from(2).char_to(6);

            assert_eq2!(range1.extract_to_line_end(), range2.extract_to_line_end());
        }

        // Test that char_range_inclusive(a, b) == char_range(a, b+1)
        {
            as_str_slice_test_case!(slice, "😀hello world🎉");
            let range1 = slice.char_range_inclusive(2, 6);
            let range2 = slice.char_range(2, 7);

            assert_eq2!(range1.extract_to_line_end(), range2.extract_to_line_end());
        }

        // Test that char_from(a) == char_range(a, len)
        {
            as_str_slice_test_case!(slice, "😀hello");
            let from_two = slice.char_from(2);
            let range_two = slice.char_range(2, 6); // 6 is the total length

            assert_eq2!(
                from_two.extract_to_line_end(),
                range_two.extract_to_line_end()
            );
        }

        // Test that char_to(a) == char_range(0, a)
        {
            as_str_slice_test_case!(slice, "😀hello");
            let to_three = slice.char_to(3);
            let range_three = slice.char_range(0, 3);

            assert_eq2!(
                to_three.extract_to_line_end(),
                range_three.extract_to_line_end()
            );
        }
    }

    #[test]
    fn test_char_range_methods_with_empty_slice() {
        // Test with empty slice
        {
            as_str_slice_test_case!(slice, "");

            let range = slice.char_range(0, 0);
            assert_eq2!(range.extract_to_line_end(), "");

            let from_zero = slice.char_from(0);
            assert_eq2!(from_zero.extract_to_line_end(), "");

            let to_zero = slice.char_to(0);
            assert_eq2!(to_zero.extract_to_line_end(), "");

            let range_inc = slice.char_range_inclusive(0, 0);
            // This might be empty or have different behavior with empty slice
            // The important thing is it doesn't panic
            let _content = range_inc.extract_to_line_end();
        }
    }

    #[test]
    fn test_char_range_methods_edge_cases() {
        // Test with very large indices (should be clamped)
        {
            as_str_slice_test_case!(slice, "hi😀");

            let from_large = slice.char_from(1000);
            assert_eq2!(from_large.extract_to_line_end(), "");

            let to_large = slice.char_to(1000);
            // Should get the whole slice
            assert_eq2!(to_large.extract_to_line_end(), "hi😀");

            let range_large = slice.char_range(1, 1000);
            assert_eq2!(range_large.extract_to_line_end(), "i😀");
        }

        // Test consistency across operations
        {
            as_str_slice_test_case!(slice, "😀hello world🎉test");

            // These should all give the same result
            let method1 = slice.char_range(2, 8);
            let method2 = slice.char_from(2).char_to(6);
            let method3 = slice.char_range_inclusive(2, 7);

            let content1 = method1.extract_to_line_end();
            let content2 = method2.extract_to_line_end();
            let content3 = method3.extract_to_line_end();

            assert_eq2!(content1, content2);
            assert_eq2!(content2, content3);
        }
    }
}

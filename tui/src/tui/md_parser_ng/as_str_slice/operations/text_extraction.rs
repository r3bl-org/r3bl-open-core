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

use nom::{FindSubstring, Input};

use crate::{bounds_check,
            AsStrSlice,
            BoundsCheck,
            CharLengthExt as _,
            DocumentStorage,
            InlineString,
            InlineStringCow,
            Length};

/// Text extraction methods for `AsStrSlice`.
///
/// These methods provide various ways to extract text content from an `AsStrSlice`,
/// including single-line extraction, multi-line extraction, and string conversion
/// utilities.
impl<'a> AsStrSlice<'a> {
    /// This does not materialize the `AsStrSlice`. And it does not look at the entire
    /// slice, but only at the current line. `AsStrSlice` is designed to work with output
    /// from [`str::lines()`], which means it should not contain any
    /// [`crate::constants::NEW_LINE`] characters in the `lines` slice.
    ///
    /// This method extracts the current line up to the end of the line, which is defined
    /// as the end of the current line or the end of the slice, whichever comes first.
    #[must_use]
    pub fn contains_in_current_line(&self, sub_str: &str) -> bool {
        self.extract_to_line_end().contains(sub_str)
    }

    /// Use [`nom::FindSubstring`] to implement this function to check if a substring
    /// exists. This will try not to materialize the `AsStrSlice` if it can avoid it,
    /// but there are situations where it may have to (and allocate memory).
    #[must_use]
    pub fn contains(&self, sub_str: &str) -> bool {
        self.find_substring(sub_str).is_some()
    }

    /// Returns the number of characters remaining in this slice.
    ///
    /// ⚠️ **Character-Based Length**: This method returns the count of **characters**,
    /// not bytes. This is essential for proper Unicode/UTF-8 support where characters
    /// can be 1-4 bytes long.
    ///
    /// This method provides the same character-based counting that should be used
    /// instead of `output.len()` and `rem.len()` in nom parsers when working with
    /// `&str` results that need to be converted back to `AsStrSlice` positions.
    ///
    /// # Examples
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case, len};
    /// as_str_slice_test_case!(slice, "😀hello");
    /// assert_eq!(slice.len_chars(), len(6)); // 1 emoji + 5 ASCII chars = 6 characters
    ///
    /// // Compare with &str.len() which returns byte count
    /// let text = "😀hello";
    /// assert_eq!(text.len(), 9); // 4 bytes (emoji) + 5 bytes (ASCII) = 9 bytes
    /// assert_eq!(text.chars().count(), 6); // Same as slice.len_chars() - 6 characters
    /// ```
    ///
    /// # Use in nom Parsers
    /// When converting `&str` lengths back to `AsStrSlice` positions, use this pattern:
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case, len};
    /// as_str_slice_test_case!(input, "😀hello world");
    ///
    /// // ❌ WRONG - using byte length from &str
    /// let text = input.extract_to_line_end();
    /// let byte_len = text.len(); // This is BYTE count (dangerous for Unicode)
    ///
    /// // ✅ CORRECT - using character count
    /// let char_count = text.chars().count(); // This is CHARACTER count (safe)
    /// // Or even better, use the AsStrSlice len_chars() method:
    /// let char_count_better = input.len_chars(); // CHARACTER count, not bytes
    ///
    /// assert_eq!(len(char_count), char_count_better); // Both should be equal
    /// assert_eq!(char_count, 12); // 1 emoji + 11 ASCII chars
    /// ```
    ///
    /// This method does not materialize the `AsStrSlice` content - it calculates
    /// length efficiently without allocating strings.
    #[must_use]
    pub fn len_chars(&self) -> Length { self.remaining_len() }

    /// This does not materialize the `AsStrSlice`.
    #[must_use]
    pub fn starts_with(&self, sub_str: &str) -> bool {
        self.extract_to_line_end().starts_with(sub_str)
    }

    /// Use the [`std::fmt::Display`] implementation to materialize the
    /// [`DocumentStorage`] content. Returns a string representation of the slice.
    ///
    /// ## Newline Behavior
    ///
    /// This method follows the same newline handling rules as described in the struct
    /// documentation:
    ///
    /// - For multiple lines, a trailing newline is added after the last line.
    /// - For a single line, no trailing newline is added.
    /// - Empty lines are preserved with newlines.
    ///
    /// ## Incompatibility with [`str::lines()`]
    ///
    /// **Important**: This behavior is intentionally different from [`str::lines()`].
    /// When there are multiple lines and the last line is empty, this method will add
    /// a trailing newline, whereas [`str::lines()`] would not.
    #[must_use]
    pub fn to_inline_string(&self) -> DocumentStorage {
        let mut acc = DocumentStorage::new();
        _ = write!(acc, "{self}");
        acc
    }

    /// Extracts text content from the current position (`line_index`, `char_index`) to
    /// the end of the line (optionally limited by `max_len`).
    ///
    /// ⚠️ **Character-Based Extraction**: This method extracts content starting from the
    /// current **character position** (not byte position) to the end of the line.
    /// This is safe for Unicode/UTF-8 text and will never split multi-byte characters.
    ///
    /// Only use this over [`Self::extract_to_slice_end()`] if you need to extract the
    /// remaining text in the current line (but not the entire slice).
    ///
    /// It handles various edge cases like:
    /// - Being at the end of a line.
    /// - Length limitations.
    /// - Lines with embedded newline characters.
    /// - Fallback to empty string for invalid positions.
    ///
    /// Returns a string reference to the slice content that is guaranteed to contain
    /// valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// # use r3bl_tui::{GCString, AsStrSlice};
    /// # use nom::Input;
    /// let lines = &[GCString::new("😀hello world"), GCString::new("Second line")];
    /// let slice = AsStrSlice::from(lines);
    ///
    /// // Extract from beginning of first line.
    /// let content = slice.extract_to_line_end();
    /// assert_eq!(content, "😀hello world");
    ///
    /// // Extract with character position offset (safe for Unicode).
    /// let slice_offset = slice.take_from(1); // Start after emoji character
    /// assert_eq!(slice_offset.extract_to_line_end(), "hello world");
    /// ```
    ///
    /// # Edge Cases
    ///
    /// - **Empty lines**: Returns empty string for empty lines
    /// - **Out of bounds**: Returns empty string when `line_index >= lines.len()`
    /// - **Character index beyond line**: Clamps `char_index` to line length
    /// - **Zero `max_len`**: When `max_len` is `Some(0)`, returns empty string
    /// - **Embedded newlines**: Don't do any special handling or processing of
    ///   [`crate::constants::NEW_LINE`] chars inside the current line.
    #[must_use]
    pub fn extract_to_line_end(&self) -> &'a str {
        use crate::core::tui_core::units::len;

        // Early returns for edge cases.
        {
            if self.lines.is_empty() {
                return "";
            }

            bounds_check!(self.line_index, self.lines.len(), {
                return "";
            });

            if let Some(max_len) = self.max_len {
                if max_len == len(0) {
                    return "";
                }
            }
        }

        // Early return if current_line does not exist.
        let Some(current_line) = self.lines.get(self.line_index.as_usize()) else {
            return "";
        };

        let current_line = current_line.as_ref();

        // ⚠️ CRITICAL: Convert character index to byte index for safe slicing
        // char_index represents CHARACTER position, but we need BYTE position for slicing
        let char_position = self.char_index.as_usize();

        // Convert character position to byte position
        let safe_start_byte_index = if char_position == 0 {
            0
        } else {
            // Find the byte position of the char_position-th character
            current_line
                .char_indices()
                .nth(char_position)
                .map_or(current_line.len(), |(byte_idx, _)| byte_idx) // If beyond end, use end of string
        };

        // If we're past the end of the line, return empty.
        if safe_start_byte_index >= current_line.len() {
            return "";
        }

        let eol = current_line.len();
        let safe_end_byte_index = match self.max_len {
            None => eol,
            Some(max_len) => {
                // Convert max_len (character count) to byte position
                let max_chars = char_position + max_len.as_usize();
                current_line
                    .char_indices()
                    .nth(max_chars)
                    .map_or(eol, |(byte_idx, _)| byte_idx) // If beyond end, use end of string
            }
        };

        &current_line[safe_start_byte_index..safe_end_byte_index]
    }

    /// Creates a new `AsStrSlice` with `max_len` set to the length of content that
    /// `extract_to_line_end()` would return. This effectively limits the slice to
    /// only include the characters from the current position to the end of the current
    /// line.
    ///
    /// This is useful when you want to create a slice that represents only the remaining
    /// content in the current line, which can then be used with other methods while
    /// maintaining the character-based limitation.
    ///
    /// # Returns
    /// A new `AsStrSlice` with the same position but with `max_len` set to the character
    /// count of the content from current position to end of line.
    ///
    /// # Examples
    /// ```
    /// # use r3bl_tui::{GCString, AsStrSlice};
    /// # use nom::Input;
    /// let lines = &[GCString::new("hello world"), GCString::new("second line")];
    /// let slice = AsStrSlice::from(lines);
    ///
    /// // Get slice limited to current line content
    /// let line_limited = slice.limit_to_line_end();
    /// assert_eq!(line_limited.extract_to_line_end(), "hello world");
    ///
    /// // After taking some characters, limit to remaining line content
    /// let advanced = slice.take_from(6); // Start from "world"
    /// let limited = advanced.limit_to_line_end();
    /// assert_eq!(limited.extract_to_line_end(), "world");
    /// ```
    #[must_use]
    pub fn limit_to_line_end(&self) -> Self {
        let line = self.extract_to_line_end();
        let line_char_count = line.len_chars().as_usize();
        self.take(line_char_count)
    }

    /// Extracts text content from the current position (`line_index`, `char_index`) to
    /// the end of the slice, respecting the `max_len` limit. It allocates for multiline
    /// `lines`, but not for single line content.
    ///
    /// This is used mostly for tests. Be aware (in the tests) that this method
    /// adds an extra [`crate::constants::NEW_LINE`] at the end of the content if there
    /// are multiple lines in the slice (to mimic the opposite behavior of
    /// [`str::lines()`], which strips trailing new line if it exists).
    ///
    /// ## Allocation Behavior
    ///
    /// For multiline content this will allocate, since there is no contiguous chunk of
    /// memory that has `\n` in them, since these new lines are generated
    /// synthetically when iterating this struct. Thus it is impossible to take
    /// chunks from [`Self::lines`] and then "join" them with `\n` in between lines,
    /// WITHOUT allocating.
    ///
    /// In the case there is only one line, this method will NOT allocate. This is why
    /// [`InlineStringCow`] is used. If you are sure that you will always have a single
    /// line, you can use [`Self::extract_to_line_end()`] instead, which does not
    /// allocate.
    ///
    /// For multiline content this will allocate, since there is no contiguous chunk of
    /// memory that has `\n` in them, since these new lines are generated
    /// synthetically when iterating this struct. Thus it is impossible to take
    /// chunks from [`Self::lines`] and then "join" them with `\n` in between lines,
    /// WITHOUT allocating.
    ///
    /// In the case there is only one line, this method will NOT allocate. This is why
    /// [`InlineStringCow`] is used.
    ///
    /// This method behaves similarly to the [`std::fmt::Display`] trait implementation
    /// but respects the current position (`line_index`, `char_index`) and `max_len`
    /// limit.
    #[must_use]
    pub fn extract_to_slice_end(&self) -> InlineStringCow<'a> {
        // Early return for invalid line_index (it has gone beyond the available lines in
        // the slice).
        bounds_check!(self.line_index, self.lines.len(), {
            return InlineStringCow::Borrowed("");
        });

        // For single line case, we can potentially return borrowed content.
        if self.lines.len() == 1 {
            let current_line = &self.lines[0].string;
            let current_line: &str = current_line.as_ref();

            // Check if we're already at the end.
            // ⚠️ CRITICAL: char_index represents CHARACTER position, use chars().count()
            let line_char_count = current_line.len_chars();
            bounds_check!(self.char_index, line_char_count, {
                return InlineStringCow::Borrowed("");
            });

            // ⚠️ **Unicode check**
            // Get the start index, ensuring it's at a valid char boundary.
            let start_col_index = self.char_index.as_usize();
            if !current_line.is_char_boundary(start_col_index) {
                // If not at a valid boundary, use a safe approach: collect chars and
                // rejoin.
                let mut acc = InlineString::new();
                for ch in current_line.chars().skip(start_col_index) {
                    acc.push(ch);
                }
                return InlineStringCow::Owned(acc);
            }

            let eol = current_line.len();
            let end_col_index = match self.max_len {
                None => eol,
                Some(max_len) => {
                    let limit = start_col_index + max_len.as_usize();
                    (eol).min(limit)
                }
            };

            // ⚠️ **Unicode check**
            // Ensure the end index is also at a valid char boundary.
            if !current_line.is_char_boundary(end_col_index) {
                // If not at a valid boundary, use a safe approach: collect chars and
                // rejoin. This approach accumulates the chars into a String and not
                // InlineString.
                let mut acc = InlineString::new();
                for ch in current_line
                    .chars()
                    .skip(start_col_index)
                    .take(end_col_index - start_col_index)
                {
                    acc.push(ch);
                }
                return InlineStringCow::Owned(acc);
            }

            return InlineStringCow::Borrowed(
                &current_line[start_col_index..end_col_index],
            );
        }

        // Multi-line case: need to allocate and use synthetic newlines.
        let mut acc = InlineString::new();
        let mut self_clone = self.clone();

        while let Some(ch) = self_clone.current_char() {
            acc.push(ch);
            self_clone.advance();
        }

        if acc.is_empty() {
            InlineStringCow::Borrowed("")
        } else {
            InlineStringCow::Owned(acc)
        }
    }
}

#[cfg(test)]
mod tests_limit_to_line_end {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, len};

    #[test]
    fn test_limit_to_line_end_basic() {
        // Single line - limit to entire line
        {
            as_str_slice_test_case!(slice, "hello world");
            let limited = slice.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "hello world");
            assert_eq2!(limited.max_len, Some(len(11))); // "hello world" = 11 chars

            // Should be equivalent to original extract_to_line_end()
            assert_eq2!(limited.extract_to_line_end(), slice.extract_to_line_end());
        }

        // Multiple lines - limit to first line only
        {
            as_str_slice_test_case!(slice, "first line", "second line", "third line");
            let limited = slice.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "first line");
            assert_eq2!(limited.max_len, Some(len(10))); // "first line" = 10 chars

            // Should be equivalent to original extract_to_line_end()
            assert_eq2!(limited.extract_to_line_end(), slice.extract_to_line_end());
        }
    }

    #[test]
    fn test_limit_to_line_end_with_position_offset() {
        // Test with character offset in the middle of a line
        {
            as_str_slice_test_case!(slice, "hello world", "second line");
            let advanced = slice.take_from(6); // Start from "world"
            let limited = advanced.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "world");
            assert_eq2!(limited.max_len, Some(len(5))); // "world" = 5 chars

            // Should be equivalent to original extract_to_line_end()
            assert_eq2!(
                limited.extract_to_line_end(),
                advanced.extract_to_line_end()
            );
        }

        // Test at the beginning of second line
        {
            as_str_slice_test_case!(slice, "first", "second line");
            let advanced = slice.take_from(6); // Move to second line (5 chars + 1 newline)
            let limited = advanced.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "second line");
            assert_eq2!(limited.max_len, Some(len(11))); // "second line" = 11 chars

            // Should be equivalent to advanced slice's extract_to_line_end()
            assert_eq2!(
                limited.extract_to_line_end(),
                advanced.extract_to_line_end()
            );
        }
    }

    #[test]
    fn test_limit_to_line_end_unicode() {
        // Test with Unicode characters including emojis
        {
            as_str_slice_test_case!(slice, "😀hello 🌍world", "next line");
            let limited = slice.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "😀hello 🌍world");
            assert_eq2!(limited.max_len, Some(len(13))); // 😀 + hello + space + 🌍 +
                                                         // world = 13 chars
        }

        // Test with Unicode and position offset
        {
            as_str_slice_test_case!(slice, "😀hello 🌍world");
            let advanced = slice.take_from(1); // Start after emoji
            let limited = advanced.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "hello 🌍world");
            assert_eq2!(limited.max_len, Some(len(12))); // hello + space + 🌍 + world =
                                                         // 12 chars
        }
    }

    #[test]
    fn test_limit_to_line_end_edge_cases() {
        // Empty line
        {
            as_str_slice_test_case!(slice, "");
            let limited = slice.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "");
            assert_eq2!(limited.max_len, Some(len(0)));
        }

        // Empty line in the middle
        {
            as_str_slice_test_case!(slice, "first", "", "third");
            let advanced = slice.take_from(6); // Move to empty line (5 chars + 1 newline)
            let limited = advanced.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "");
            assert_eq2!(limited.max_len, Some(len(0)));
        }

        // At end of line
        {
            as_str_slice_test_case!(slice, "hello");
            let advanced = slice.take_from(5); // Move to end of line
            let limited = advanced.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "");
            assert_eq2!(limited.max_len, Some(len(0)));
        }

        // Beyond end of line (should be handled gracefully)
        {
            as_str_slice_test_case!(slice, "hello");
            let advanced = slice.take_from(10); // Beyond end
            let limited = advanced.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "");
            assert_eq2!(limited.max_len, Some(len(0)));
        }
    }

    #[test]
    fn test_limit_to_line_end_with_existing_max_len() {
        // Test when slice already has a max_len that's larger than line content
        {
            as_str_slice_test_case!(slice, limit: 20, "hello world");
            let limited = slice.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "hello world");
            assert_eq2!(limited.max_len, Some(len(11))); // Should be line length, not
                                                         // original max_len
        }

        // Test when slice already has a max_len that's smaller than line content
        {
            as_str_slice_test_case!(slice, limit: 5, "hello world");
            let limited = slice.limit_to_line_end();

            assert_eq2!(limited.extract_to_line_end(), "hello");
            assert_eq2!(limited.max_len, Some(len(5))); // Should be actual extracted
                                                        // length
        }
    }

    #[test]
    fn test_limit_to_line_end_preserves_other_fields() {
        // Verify that other fields are preserved correctly
        {
            as_str_slice_test_case!(slice, "first line", "second line");
            let advanced = slice.take_from(3); // Move to position 3 in first line
            let limited = advanced.limit_to_line_end();

            // Check that position fields are preserved
            assert_eq2!(limited.lines, advanced.lines);
            assert_eq2!(limited.line_index, advanced.line_index);
            assert_eq2!(limited.char_index, advanced.char_index);
            assert_eq2!(limited.total_size, advanced.total_size);
            assert_eq2!(limited.current_taken, advanced.current_taken);

            // Only max_len should be different
            assert_eq2!(limited.max_len, Some(len(7))); // "st line" = 7 chars
        }
    }

    #[test]
    fn test_limit_to_line_end_equivalence_with_take() {
        // Verify that limit_to_line_end() produces same result as manual take()
        {
            as_str_slice_test_case!(slice, "hello world", "second line");

            let line_content = slice.extract_to_line_end();
            let char_count = line_content.chars().count();
            let manual_limited = slice.take(char_count);
            let auto_limited = slice.limit_to_line_end();

            assert_eq2!(
                auto_limited.extract_to_line_end(),
                manual_limited.extract_to_line_end()
            );
            assert_eq2!(auto_limited.max_len, manual_limited.max_len);
        }

        // Test with position offset
        {
            as_str_slice_test_case!(slice, "hello world", "second line");
            let advanced = slice.take_from(6);

            let line_content = advanced.extract_to_line_end();
            let char_count = line_content.chars().count();
            let manual_limited = advanced.take(char_count);
            let auto_limited = advanced.limit_to_line_end();

            assert_eq2!(
                auto_limited.extract_to_line_end(),
                manual_limited.extract_to_line_end()
            );
            assert_eq2!(auto_limited.max_len, manual_limited.max_len);
        }
    }

    #[test]
    fn test_limit_to_line_end_multiple_calls() {
        // Test that calling limit_to_line_end() multiple times is idempotent
        {
            as_str_slice_test_case!(slice, "hello world");
            let limited1 = slice.limit_to_line_end();
            let limited2 = limited1.limit_to_line_end();

            assert_eq2!(
                limited1.extract_to_line_end(),
                limited2.extract_to_line_end()
            );
            assert_eq2!(limited1.max_len, limited2.max_len);
        }
    }
}

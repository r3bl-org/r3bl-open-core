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

use crate::{as_str_slice_mod::AsStrSlice,
            core::tui_core::units::{len, Index, Length},
            GCString};

impl<'a> AsStrSlice<'a> {
    /// This does not materialize the `AsStrSlice`.
    ///
    /// For [nom::Input] trait compatibility, this should return true when no more
    /// characters can be consumed from the current position, taking into account
    /// any `max_len` constraints.
    pub fn is_empty(&self) -> bool {
        // This code is simply the following, however, it is fast.
        // self.remaining_len() == len(0)

        // If max_len is set and is 0, we're empty.
        if let Some(max_len) = self.max_len {
            if max_len == len(0) {
                return true;
            }
        }

        // Check if we've consumed all available characters
        if self.current_taken >= self.total_size {
            return true;
        }

        // Check if we're beyond the available lines.
        if self.line_index.as_usize() >= self.lines.len() {
            return true;
        }

        // For empty lines array.
        if self.lines.is_empty() {
            return true;
        }

        // Use current_char() to determine emptiness - if no char available, we're empty.
        self.current_char().is_none()
    }

    /// Create a new slice with a maximum length limit.
    pub fn with_limit(
        lines: &'a [GCString],
        arg_start_line: impl Into<Index>,
        arg_start_char: impl Into<Index>,
        max_len: Option<Length>,
    ) -> Self {
        let start_line: Index = arg_start_line.into();
        let start_char: Index = arg_start_char.into();
        let total_size = Self::calculate_total_size(lines);
        let current_taken = Self::calculate_current_taken(lines, start_line, start_char);

        Self {
            lines,
            line_index: start_line,
            char_index: start_char,
            max_len,
            total_size,
            current_taken,
        }
    }

    /// Skip characters then take a limited number of characters within the current line.
    ///
    /// This method is optimized for **single-line operations** where you need to skip
    /// some characters and then take a specific number of characters from the same
    /// line. It's an atomic operation that's more efficient than chaining
    /// `take_from(skip_count).take(take_count)` for single-line text processing.
    ///
    /// ## When to use this method
    ///
    /// ✅ **Use `skip_take_in_current_line()` when:**
    /// - Working with text within a single line boundary
    /// - Parsing inline elements (like emphasis, links, code spans)
    /// - Processing results from `extract_to_line_end()` or similar single-line
    ///   operations
    /// - Need atomic skip+take operation for performance
    ///
    /// ## When to Use `take_from(skip_count).take(take_count)` instead
    ///
    /// ✅ **Use `input.take_from(skip_count).take(take_count)` when:**
    /// - Working with multiline content that spans line boundaries
    /// - Processing continuous text from functions like
    ///   `parse_code_block_body_including_code_block_end_alt`
    /// - Need to handle line transitions and character offsets across multiple lines
    /// - Splitting multiline content into segments
    ///
    /// ## Technical details
    ///
    /// This method works by directly manipulating `char_index` within the current line
    /// context. It doesn't call `advance()` internally, which means it doesn't handle
    /// line boundary transitions. This makes it fast for single-line operations but
    /// unsuitable for multiline content where character positions may cross line
    /// boundaries.
    ///
    /// See [`crate::split_by_new_line_alt`] for an example of why multiline processing
    /// requires `take_from` and `take` (part of the [nom::Input] impl) instead of this
    /// method.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use r3bl_tui::{as_str_slice_test_case, len};
    /// // ✅ Good: Single-line processing
    /// as_str_slice_test_case!(input, "Hello, World!");
    /// let result = input.skip_take_in_current_line(7, 5); // Skip "Hello, ", take "World"
    /// assert_eq!(result.extract_to_line_end(), "World");
    ///
    /// // ❌ Avoid: Multiline content - use take_from().take() instead
    /// as_str_slice_test_case!(multiline, "Line 1", "Line 2", "Line 3");
    /// // Don't use skip_take_in_current_line() for this - it won't handle line boundaries
    /// ```
    pub fn skip_take_in_current_line(
        &self,
        arg_skip_count: impl Into<Length>,
        arg_take_count: impl Into<Length>,
    ) -> Self {
        let skip_count: Length = arg_skip_count.into();
        let take_count: Length = arg_take_count.into();

        Self {
            lines: self.lines,
            line_index: self.line_index,
            char_index:
            // Can't exceed the end of the slice. Set the new char_index
            // based on the current char_index and skip_count.
            {
                let new_start_index_after_skip = self.char_index + skip_count;
                let max_index = self.total_size.convert_to_index();
                new_start_index_after_skip.min(max_index)
            },
            max_len:
            // Can't exceed the end of the slice. This is the maximum length
            // that can be taken after the new char_index is set.
            {
                let consumed_after_skip = self.current_taken + skip_count;
                let available_space_after_skip = self.total_size - consumed_after_skip;
                Some(
                    take_count.min(available_space_after_skip)
                )
            },
            total_size: self.total_size,
            current_taken: self.current_taken,
        }
    }

    /// Creates a new `AsStrSlice` that limits content consumption to a maximum character
    /// count.
    ///
    /// This method creates a truncated view of the current slice by setting a character
    /// limit at the specified `end_index`. The resulting slice will consume
    /// characters from the current position up to (but not including) the `end_index`
    /// character position.
    pub fn take_until(&self, arg_end_index: impl Into<Index>) -> Self {
        let end_index: Index = arg_end_index.into();
        let new_char_index = self.char_index.min(end_index);

        Self {
            lines: self.lines,
            line_index: self.line_index,
            char_index: new_char_index,
            max_len: {
                let max_until_end = *end_index - *new_char_index;
                Some(len(max_until_end))
            },
            total_size: self.total_size,
            current_taken: self.current_taken,
        }
    }
}

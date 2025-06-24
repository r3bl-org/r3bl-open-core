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

use crate::{as_str_slice_mod::AsStrSlice,
            core::tui_core::units::len,
            CharacterIndexNomCompat,
            StringCharIndices,
            StringChars};

impl<'a> Input for AsStrSlice<'a> {
    type Item = char;
    type Iter = StringChars<'a>;
    type IterIndices = StringCharIndices<'a>;

    fn input_len(&self) -> usize { self.remaining_len().as_usize() }

    /// Returns a slice containing the first `count` characters from the current position.
    /// This is a character index due to the [Self::Item] assignment to [char].
    ///
    /// ⚠️ **Character-Based Operation**: This method takes `count` **characters**, not
    /// bytes. This is safe for Unicode/UTF-8 text including emojis and multi-byte
    /// characters.
    ///
    /// # Example
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// # use nom::Input;
    /// as_str_slice_test_case!(slice, "😀hello");
    /// let first_two = slice.take(2); // Gets "😀h" (2 characters)
    /// // NOT "😀" + partial byte sequence which would panic
    /// ```
    ///
    /// This works with the `max_len` field of [AsStrSlice].
    fn take(&self, count: CharacterIndexNomCompat) -> Self {
        // take() should return a slice containing the first 'count' characters.
        // Create a slice that starts at current position with max_len = count.
        //
        // If count is 0, we should return an empty slice that doesn't interfere with
        // further parsing, rather than a slice that blocks all parsing.
        if count == 0 {
            // Return a slice that represents "empty content" but doesn't block parsing
            // by setting max_len to 0 but keeping the same position.
            Self::with_limit(self.lines, self.line_index, self.char_index, Some(len(0)))
        } else {
            Self::with_limit(
                self.lines,
                self.line_index,
                self.char_index,
                Some(len(count)),
            )
        }
    }

    /// Returns a slice starting from the `start` character position. This is a character
    /// index due to the [Self::Item] assignment to [char].
    ///
    /// ⚠️ **Character-Based Operation**: The `start` parameter is a **character offset**,
    /// not a byte offset. This is critical for Unicode/UTF-8 safety - never pass byte
    /// positions from operations like `find_substring()` directly to this method.
    ///
    /// # Example
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// # use nom::Input;
    /// as_str_slice_test_case!(slice, "😀hello");
    /// let from_second = slice.take_from(1); // Starts from "hello" (after the emoji)
    /// // NOT from a byte position that might split the emoji
    /// ```
    ///
    /// # Converting Byte Positions to Character Positions
    ///
    /// If you have a byte position (e.g., from `find_substring()`), convert it first:
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// # use nom::FindSubstring;
    /// # use nom::Input;
    /// as_str_slice_test_case!(slice, "hello world");
    /// let byte_pos = slice.find_substring("world").unwrap();
    /// let prefix = slice.take(byte_pos); // Use byte position with take()
    /// let char_count = prefix.extract_to_line_end().chars().count(); // Convert to chars
    /// let from_char = slice.take_from(char_count); // Use char count here
    /// ```
    fn take_from(&self, start: CharacterIndexNomCompat) -> Self {
        let mut result = self.clone();
        let actual_advance = start.min(self.remaining_len().as_usize());

        // Advance to the start position.
        for _ in 0..actual_advance {
            result.advance();
        }

        // If we had a max_len limit, adjust it to account for the advanced position.
        if let Some(max_len) = self.max_len {
            if actual_advance >= max_len.as_usize() {
                // We've advanced past the original limit, so the remaining slice is
                // empty.
                result.max_len = Some(len(0));
            } else {
                // Reduce the max_len by the amount we advanced.
                result.max_len = Some(max_len - len(actual_advance));
            }
        }

        result
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let taken = self.take(count);
        let remaining = self.take_from(count);
        (taken, remaining)
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        let mut pos = 0;
        let mut current = self.clone();

        while let Some(ch) = current.current_char() {
            if predicate(ch) {
                return Some(pos);
            }
            current.advance();
            pos += 1;
        }

        None
    }

    /// Returns an iterator over the characters in the slice.
    fn iter_elements(&self) -> Self::Iter { StringChars::new(self.clone()) }

    /// Returns an iterator over the characters in the slice with their indices.
    fn iter_indices(&self) -> Self::IterIndices { StringCharIndices::new(self.clone()) }

    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        let remaining = self.remaining_len().as_usize();
        if count <= remaining {
            Ok(count)
        } else {
            Err(nom::Needed::new(count - remaining))
        }
    }
}

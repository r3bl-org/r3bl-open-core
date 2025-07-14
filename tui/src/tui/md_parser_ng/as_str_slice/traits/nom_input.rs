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

//! Implementation of `nom::Input` trait for `AsStrSlice`.
//!
//! # Character-Based Indexing
//!
//! **Important**: All `usize` values in this implementation represent **character
//! positions** and **character counts**, not byte positions or byte counts. This is
//! crucial for handling Unicode text correctly, including multi-byte characters like
//! emojis.
//!
//! Since [`AsStrSlice`] works with [char] (see [`Iterator::Item`]), all `usize` values in
//! the interface represent character-based indices and offsets. However, since we can't
//! change the `nom::Input` trait signature, we use type aliases to clarify this:
//!
//! - [`CharacterIndexNomCompat`]: Character index, not byte index
//! - [`CharacterCountNomCompat`]: Character count, not byte count
//!
//! ## Converting Between Byte and Character Positions
//!
//! If you have byte positions (e.g., from `find_substring()`), you must convert them to
//! character positions before using methods like `take_from()`:
//!
//! ```rust
//! # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
//! # use nom::{FindSubstring, Input};
//! as_str_slice_test_case!(slice, "hello world");
//! let byte_pos = slice.find_substring("world").unwrap();
//! let prefix = slice.take(byte_pos); // take() accepts byte positions
//! let char_count = prefix.extract_to_line_end().chars().count(); // Convert to chars
//! let from_char = slice.take_from(char_count); // take_from() needs char positions
//! ```

use nom::Input;

use crate::{as_str_slice::AsStrSlice,
            core::units::{len, Index},
            CharacterCountNomCompat, CharacterIndexNomCompat, StringCharIndices,
            StringChars};

/// Implementation of `nom::Input` trait for `AsStrSlice`.
impl<'a> Input for AsStrSlice<'a> {
    type Item = char;
    type Iter = StringChars<'a>;
    type IterIndices = StringCharIndices<'a>;

    fn input_len(&self) -> usize { self.remaining_len().as_usize() }

    /// Returns a slice containing the first `count` characters from the current position.
    ///
    /// # Example
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// # use nom::Input;
    /// as_str_slice_test_case!(slice, "😀hello");
    /// let first_two = slice.take(2); // Gets "😀h" (2 characters)
    /// // NOT "😀" + partial byte sequence which would panic
    /// ```
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

    /// Returns a slice starting from the `start` character position.
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

        if actual_advance == 0 {
            return result;
        }

        // Calculate the global character position we want to reach
        let target_global_pos = self.current_taken.as_usize() + actual_advance;

        // Use cache to find the target line and character position
        match self
            .cache
            .get()
            .line_metadata
            .char_pos_to_line_char(target_global_pos)
        {
            Some((target_line, char_within_line)) => {
                result.line_index = Index::from(target_line);
                result.char_index = Index::from(char_within_line);
                result.current_taken = len(target_global_pos);

                // If we had a max_len limit, adjust it to account for the advanced
                // position.
                if let Some(max_len) = self.max_len {
                    if actual_advance >= max_len.as_usize() {
                        result.max_len = Some(len(0));
                    } else {
                        result.max_len = Some(max_len - len(actual_advance));
                    }
                }
            }
            _ => {
                // Fallback to character-by-character advance if cache lookup fails
                for _ in 0..actual_advance {
                    result.advance();
                }
            }
        }

        result
    }

    fn take_split(&self, count: CharacterCountNomCompat) -> (Self, Self) {
        let taken = self.take(count);
        let remaining = self.take_from(count);
        (taken, remaining)
    }

    fn position<P>(&self, predicate: P) -> Option<CharacterIndexNomCompat>
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

    fn slice_index(
        &self,
        count: CharacterCountNomCompat,
    ) -> Result<CharacterIndexNomCompat, nom::Needed> {
        let remaining = self.remaining_len().as_usize();
        if count <= remaining {
            Ok(count)
        } else {
            Err(nom::Needed::new(count - remaining))
        }
    }
}

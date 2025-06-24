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

use std::{convert::AsRef, fmt::Display};

use nom::{Compare, CompareResult, FindSubstring, IResult, Input, Offset, Parser};

use crate::{as_str_slice_mod::{AsStrSlice},
            core::tui_core::units::{idx, len, Index, Length},
            CharacterIndex,
            CharacterIndexNomCompat,
            CharacterLength,
            GCString,
            InlineVec,
            List,
            NErr,
            NError,
            NErrorKind};

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

/// The `Compare` trait in nom is not symmetric - you need to implement it in both
/// directions if you want to use both types interchangeably with the `tag` function.
impl<'a> Compare<&str> for AsStrSlice<'a> {
    fn compare(&self, t: &str) -> CompareResult {
        let mut current = self.clone();
        let mut target_chars = t.chars();

        loop {
            match (current.current_char(), target_chars.next()) {
                (Some(a), Some(b)) if a == b => {
                    current.advance();
                }
                (Some(_), Some(_)) => return CompareResult::Error,
                (None, Some(_)) => return CompareResult::Incomplete,
                (Some(_), None) => return CompareResult::Ok,
                (None, None) => return CompareResult::Ok,
            }
        }
    }

    fn compare_no_case(&self, t: &str) -> CompareResult {
        let mut current = self.clone();
        let mut target_chars = t.chars();

        loop {
            match (current.current_char(), target_chars.next()) {
                (Some(a), Some(b)) if a.to_lowercase().eq(b.to_lowercase()) => {
                    current.advance();
                }
                (Some(_), Some(_)) => return CompareResult::Error,
                (None, Some(_)) => return CompareResult::Incomplete,
                (Some(_), None) => return CompareResult::Ok,
                (None, None) => return CompareResult::Ok,
            }
        }
    }
}

/// The `Compare` trait in nom is not symmetric - you need to implement it in both
/// directions if you want to use both types interchangeably with the `tag` function.
impl<'a> Compare<AsStrSlice<'a>> for &str {
    fn compare(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert AsStrSlice to string and compare with self
        let t_str = t.extract_to_slice_end();
        self.compare(t_str.as_ref())
    }

    fn compare_no_case(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert AsStrSlice to string and compare with self (case insensitive)
        let t_str = t.extract_to_slice_end();
        self.compare_no_case(t_str.as_ref())
    }
}

/// The `Compare` trait needs to be implemented to compare `AsStrSlice` with each other
/// for the `tag` function.
impl<'a> Compare<AsStrSlice<'a>> for AsStrSlice<'a> {
    fn compare(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert both AsStrSlice instances to strings and compare
        let self_str = self.extract_to_slice_end();
        let t_str = t.extract_to_slice_end();
        self_str.as_ref().compare(t_str.as_ref())
    }

    fn compare_no_case(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert both AsStrSlice instances to strings and compare (case insensitive)
        let self_str = self.extract_to_slice_end();
        let t_str = t.extract_to_slice_end();
        self_str.as_ref().compare_no_case(t_str.as_ref())
    }
}

/// Integrate with [crate::List] so that `List::from()` will work for
/// `InlineVec<AsStrSlice>`.
impl<'a> From<InlineVec<AsStrSlice<'a>>> for List<AsStrSlice<'a>> {
    fn from(other: InlineVec<AsStrSlice<'a>>) -> Self {
        let mut it = List::with_capacity(other.len());
        it.extend(other);
        it
    }
}

/// Implement [Offset] trait for [AsStrSlice]. This is required for the
/// [nom::combinator::recognize] parser to work.
impl<'a> Offset for AsStrSlice<'a> {
    fn offset(&self, second: &Self) -> usize {
        // Calculate the character offset between two AsStrSlice instances.
        // The second slice must be a part of self (advanced from self).

        // If they point to different line arrays, we can't calculate a meaningful offset.
        if !std::ptr::eq(self.lines.as_ptr(), second.lines.as_ptr()) {
            return 0;
        }

        // If second is before self, return 0 (invalid case).
        if second.line_index.as_usize() < self.line_index.as_usize()
            || (second.line_index == self.line_index
                && second.char_index.as_usize() < self.char_index.as_usize())
        {
            return 0;
        }

        let mut offset = 0;

        // Count characters from self's position to second's position.
        let mut current_line = self.line_index.as_usize();
        let mut current_char = self.char_index.as_usize();

        while current_line < second.line_index.as_usize()
            || (current_line == second.line_index.as_usize()
                && current_char < second.char_index.as_usize())
        {
            if current_line >= self.lines.len() {
                break;
            }

            let line = &self.lines[current_line].string;

            if current_line < second.line_index.as_usize() {
                // Count remaining characters in current line
                if current_char < line.len() {
                    offset += line.len() - current_char;
                }
                // Add synthetic newline if not the last line
                if current_line < self.lines.len() - 1 {
                    offset += 1;
                }
                // Move to next line
                current_line += 1;
                current_char = 0;
            } else {
                // We're on the same line as second, count up to second's char_index
                let end_char = second.char_index.as_usize().min(line.len());
                if current_char < end_char {
                    offset += end_char - current_char;
                }
                break;
            }
        }

        offset
    }
}

/// Implement [Display] trait for [AsStrSlice].
impl<'a> Display for AsStrSlice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Materialize the text by collecting all characters from the current position.
        let mut current = self.clone();
        while let Some(ch) = current.current_char() {
            write!(f, "{ch}")?;
            current.advance();
        }
        Ok(())
    }
}

/// Implement [FindSubstring] trait for [AsStrSlice]. This is required by the
/// [nom::bytes::complete::take_until] parser function.
impl<'a> FindSubstring<&str> for AsStrSlice<'a> {
    fn find_substring(&self, sub_str: &str) -> Option<usize> {
        // Convert the AsStrSlice to a string representation.
        let full_text = self.extract_to_slice_end();

        // Find the substring in the full text.
        full_text.as_ref().find(sub_str)
    }
}

/// Iterator over the characters in an [AsStrSlice].
pub struct StringChars<'a> {
    slice: AsStrSlice<'a>,
}

impl<'a> StringChars<'a> {
    /// Creates a new iterator over the characters in the given slice.
    fn new(slice: AsStrSlice<'a>) -> Self { Self { slice } }
}

impl<'a> Iterator for StringChars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.slice.current_char();
        if ch.is_some() {
            self.slice.advance();
        }
        ch
    }
}

/// Iterator over the characters in an [AsStrSlice] with their indices.
pub struct StringCharIndices<'a> {
    slice: AsStrSlice<'a>,
    position: Index,
}

impl<'a> StringCharIndices<'a> {
    fn new(slice: AsStrSlice<'a>) -> Self {
        Self {
            slice,
            position: idx(0),
        }
    }
}

impl<'a> Iterator for StringCharIndices<'a> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.slice.current_char()?;
        let pos = self.position.as_usize();
        self.slice.advance();
        self.position += idx(1);
        Some((pos, ch))
    }
}

/// Represents the overall input state for parsing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputState {
    /// Input has been exhausted - no more content to parse
    AtEndOfInput,
    /// Input still has content available for parsing
    HasMoreContent,
}

/// Represents the advancement state after a parser operation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdvancementState {
    /// Parser advanced to a new line (ideal case)
    AdvancedToNewLine,
    /// Parser made progress within the current line
    MadeCharProgress,
    /// Parser successfully handled an empty line
    HandledEmptyLine,
    /// Parser made no progress at all
    NoProgress,
}

/// Captures the initial position state before parsing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InitialParsePosition {
    pub line_index: CharacterIndex,
    pub char_index: CharacterIndex,
    pub current_taken: CharacterLength,
}

impl<'a> AsStrSlice<'a> {
    /// Ensures parser advancement with fail-safe line progression for `AsStrSlice` input.
    ///
    /// This method guarantees that parsing always makes progress by advancing to the next
    /// line when a parser succeeds but doesn't naturally advance lines. It prevents
    /// infinite loops in parsing by implementing a fail-safe advancement mechanism.
    ///
    /// # How it works
    ///
    /// 1. **Input validation**: Checks if input is exhausted before attempting to parse
    /// 2. **Parser application**: Applies the provided parser to a clone of the current
    ///    input
    /// 3. **Advancement analysis**: Determines what type of advancement occurred:
    ///    - `AdvancedToNewLine`: Parser naturally advanced to next line (ideal case)
    ///    - `MadeCharProgress`: Parser advanced within current line
    ///    - `HandledEmptyLine`: Parser handled an empty/whitespace-only line
    ///    - `NoProgress`: Parser made no advancement at all
    /// 4. **Fail-safe handling**: For cases where parser didn't advance lines, manually
    ///    advances to the beginning of the next line to ensure progress
    ///
    /// # State Management
    ///
    /// Uses clean enum-based state tracking:
    /// - `InputState`: Distinguishes between exhausted input and available content
    /// - `AdvancementState`: Categorizes different types of parser advancement
    /// - `InitialParsePosition`: Captures position before parsing for comparison
    ///
    /// # Error Handling
    ///
    /// - Returns `Eof` error when input is exhausted
    /// - Returns `Verify` error when parser makes no progress (prevents infinite loops)
    /// - Propagates parser-specific errors unchanged
    ///
    /// # Usage Pattern
    ///
    /// This method is designed to be called within closure-based parser alternatives,
    /// typically used with [`nom::branch::alt()`]:
    ///
    /// ```
    /// # use r3bl_tui::*;
    /// # use nom::{branch::alt, combinator::map, IResult};
    /// # use nom::Parser as _;
    /// #
    /// # fn some_parser_function<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    /// #     nom::bytes::complete::tag("test")(input)
    /// # }
    /// # fn another_parser_function<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    /// #     nom::bytes::complete::tag("other")(input)
    /// # }
    /// # fn transform_output(s: AsStrSlice<'_>) -> String { s.extract_to_line_end().to_string() }
    /// # fn another_transform(s: AsStrSlice<'_>) -> String { format!("transformed: {}", s.extract_to_line_end()) }
    ///
    /// // Example usage with a single parser
    /// fn example_parser(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     input.ensure_advance_with_parser(&mut map(
    ///         some_parser_function,
    ///         transform_output,
    ///     ))
    /// }
    ///
    /// // Helper functions for alt() usage (avoids closure lifetime issues)
    /// fn parser_branch_1(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     input.ensure_advance_with_parser(&mut map(
    ///         some_parser_function,
    ///         transform_output,
    ///     ))
    /// }
    ///
    /// fn parser_branch_2(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     input.ensure_advance_with_parser(&mut map(
    ///         another_parser_function,
    ///         another_transform,
    ///     ))
    /// }
    ///
    /// // Example usage in alt() chain
    /// fn parse_alternatives(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     let mut parser = alt([parser_branch_1, parser_branch_2]);
    ///     parser.parse(input)
    /// }
    /// ```
    ///
    /// # Parameters
    ///
    /// * `parser` - A mutable reference to a nom parser that operates on `AsStrSlice`
    ///   input. The mutable reference is required by nom's `Parser` trait implementation.
    ///
    /// # Returns
    ///
    /// * `Ok((remainder, output))` - Parser succeeded with guaranteed line advancement
    /// * `Err(nom::Err)` - Parser failed or input was exhausted
    ///
    /// # See Also
    ///
    /// * `determine_input_state` - Input exhaustion detection
    /// * `handle_parser_advancement` - Core advancement logic
    /// * [`crate::ensure_advance_fail_safe_alt`] - Legacy wrapper function for backward
    ///   compatibility (deprecated in favor of this method)
    pub fn ensure_advance_with_parser<F, O>(
        &self,
        parser: &mut F,
    ) -> IResult<AsStrSlice<'a>, O>
    where
        F: Parser<AsStrSlice<'a>, Output = O, Error = nom::error::Error<AsStrSlice<'a>>>,
    {
        // Check input state before attempting to parse.
        let input_state = self.determine_input_state();
        if let InputState::AtEndOfInput = input_state {
            return Err(NErr::Error(NError::new(self.clone(), NErrorKind::Eof)));
        }

        // Capture initial state and apply parser.
        let initial_position = self.capture_initial_position();
        let result = parser.parse(self.clone());

        match result {
            Ok((remainder, output)) => {
                let advancement_result =
                    self.handle_parser_advancement(initial_position, remainder)?;
                Ok((advancement_result, output))
            }
            Err(e) => Err(e),
        }
    }

    /// Determines if input has been exhausted.
    fn determine_input_state(&self) -> InputState {
        if self.line_index >= self.lines.len().into()
            || self.current_taken >= self.total_size
        {
            InputState::AtEndOfInput
        } else {
            InputState::HasMoreContent
        }
    }

    /// Captures the current position state before parsing.
    fn capture_initial_position(&self) -> InitialParsePosition {
        InitialParsePosition {
            line_index: self.line_index,
            char_index: self.char_index,
            current_taken: self.current_taken,
        }
    }

    /// Determines what type of advancement occurred after parsing.
    fn determine_advancement_state(
        &self,
        initial_position: InitialParsePosition,
        remainder: &AsStrSlice<'a>,
    ) -> AdvancementState {
        // Check if parser advanced to a new line (ideal case).
        if remainder.line_index > initial_position.line_index {
            return AdvancementState::AdvancedToNewLine;
        }

        // Check if parser made progress within the current line.
        let made_char_progress = remainder.current_taken > initial_position.current_taken
            || remainder.char_index > initial_position.char_index;

        if made_char_progress {
            return AdvancementState::MadeCharProgress;
        }

        // Check if we're dealing with an empty line.
        let current_line = remainder
            .lines
            .get(remainder.line_index.as_usize())
            .map(|line| line.as_ref())
            .unwrap_or("");

        if current_line.trim().is_empty() {
            return AdvancementState::HandledEmptyLine;
        }

        AdvancementState::NoProgress
    }

    /// Handles the advancement logic based on parser results.
    fn handle_parser_advancement(
        &self,
        initial_position: InitialParsePosition,
        remainder: AsStrSlice<'a>,
    ) -> Result<AsStrSlice<'a>, NErr<NError<AsStrSlice<'a>>>> {
        let advancement_state =
            self.determine_advancement_state(initial_position, &remainder);

        match advancement_state {
            AdvancementState::AdvancedToNewLine => {
                // Parser already made proper line advancement.
                Ok(remainder)
            }
            AdvancementState::MadeCharProgress | AdvancementState::HandledEmptyLine => {
                // Need to manually advance to next line.
                self.advance_to_next_line(remainder)
            }
            AdvancementState::NoProgress => {
                // Check if we're at end of input.
                if remainder.determine_input_state() == InputState::AtEndOfInput {
                    Err(NErr::Error(NError::new(self.clone(), NErrorKind::Eof)))
                } else {
                    // No progress made - return error to break parsing loop.
                    Err(NErr::Error(NError::new(self.clone(), NErrorKind::Verify)))
                }
            }
        }
    }

    /// Advances the slice to the beginning of the next line.
    fn advance_to_next_line(
        &self,
        mut remainder: AsStrSlice<'a>,
    ) -> Result<AsStrSlice<'a>, NErr<NError<AsStrSlice<'a>>>> {
        // Ensure we're within valid line bounds.
        if remainder.line_index >= remainder.lines.len().into() {
            return Err(NErr::Error(NError::new(self.clone(), NErrorKind::Eof)));
        }

        // Get current line length.
        let current_line_len = remainder
            .lines
            .get(remainder.line_index.as_usize())
            .map(|line| line.as_ref().chars().count())
            .unwrap_or(0);

        // Advance to end of current line if not already there.
        if remainder.char_index.as_usize() < current_line_len {
            let chars_to_advance = current_line_len - remainder.char_index.as_usize();
            for _ in 0..chars_to_advance {
                remainder.advance();
            }
        }

        // Check if we can advance to the next line.
        if remainder.line_index.as_usize() < remainder.lines.len() - 1 {
            // Create a fresh AsStrSlice at the next line with no max_len constraint.
            let next_line_index = remainder.line_index + crate::idx(1);
            remainder = AsStrSlice::with_limit(
                remainder.lines,
                next_line_index,
                crate::idx(0), // Start at beginning of next line.
                None,          // Remove max_len constraint
            );
        }

        Ok(remainder)
    }

    /// Helper method to check if the current line is empty or whitespace-only.
    pub fn is_current_line_empty_or_whitespace(&self) -> bool {
        self.lines
            .get(self.line_index.as_usize())
            .map(|line| line.as_ref().trim().is_empty())
            .unwrap_or(true)
    }

    /// Helper method to get the current line as a string reference.
    pub fn get_current_line(&self) -> Option<&str> {
        self.lines
            .get(self.line_index.as_usize())
            .map(|line| line.as_ref())
    }
}

#[cfg(test)]
mod tests_ensure_advance_with_parser {
    use nom::{bytes::complete::tag, IResult};

    use super::*;
    use crate::{assert_eq2, GCString};

    fn simple_parser(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, AsStrSlice<'_>> {
        tag("test")(input)
    }

    fn empty_line_parser(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, ()> {
        if input.is_current_line_empty_or_whitespace() {
            Ok((input, ()))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    }

    #[test]
    fn test_parser_advances_to_new_line() {
        let lines = [GCString::new("test"), GCString::new("next")];
        let input = AsStrSlice::from(&lines);

        let result = input.ensure_advance_with_parser(&mut simple_parser);
        assert!(result.is_ok());

        let (remainder, _) = result.unwrap();
        assert_eq2!(remainder.line_index, crate::idx(1));
        assert_eq2!(remainder.char_index, crate::idx(0));
    }

    #[test]
    fn test_parser_handles_empty_line() {
        let lines = [GCString::new(""), GCString::new("next")];
        let input = AsStrSlice::from(&lines);

        let result = input.ensure_advance_with_parser(&mut empty_line_parser);
        assert!(result.is_ok());

        let (remainder, _) = result.unwrap();
        assert_eq2!(remainder.line_index, crate::idx(1));
    }

    #[test]
    fn test_parser_at_end_of_input() {
        let lines = [GCString::new("test")];
        let mut input = AsStrSlice::from(&lines);
        input.line_index = crate::idx(1); // Beyond available lines

        let result = input.ensure_advance_with_parser(&mut simple_parser);
        assert!(result.is_err());

        if let Err(nom::Err::Error(error)) = result {
            assert_eq2!(error.code, nom::error::ErrorKind::Eof);
        }
    }

    #[test]
    fn test_determine_input_state() {
        let lines = [GCString::new("test")];
        let input = AsStrSlice::from(&lines);

        assert_eq2!(input.determine_input_state(), InputState::HasMoreContent);

        let mut exhausted_input = input;
        exhausted_input.line_index = crate::idx(1);
        assert_eq2!(
            exhausted_input.determine_input_state(),
            InputState::AtEndOfInput
        );
    }

    #[test]
    fn test_capture_initial_position() {
        let lines = [GCString::new("test")];
        let input = AsStrSlice::from(&lines);

        let position = input.capture_initial_position();
        assert_eq2!(position.line_index, input.line_index);
        assert_eq2!(position.char_index, input.char_index);
        assert_eq2!(position.current_taken, input.current_taken);
    }
}

#[cfg(test)]
mod tests_as_str_slice_test_case {
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_as_str_slice_creation() {
        // Single string.
        as_str_slice_test_case!(input, "@title: Something");
        assert_eq2!(input.lines.len(), 1);
        assert_eq2!(input.lines[0].as_ref(), "@title: Something");

        // Multiple strings.
        as_str_slice_test_case!(input, "@title: Something", "more content", "even more");
        assert_eq2!(input.lines.len(), 3);
        assert_eq2!(input.lines[0].as_ref(), "@title: Something");
        assert_eq2!(input.lines[1].as_ref(), "more content");
        assert_eq2!(input.lines[2].as_ref(), "even more");

        // With trailing comma (optional).
        as_str_slice_test_case!(input, "@title: Something",);
        assert_eq2!(input.lines.len(), 1);
        assert_eq2!(input.lines[0].as_ref(), "@title: Something");
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

#[cfg(test)]
mod tests_compat_with_unicode_grapheme_cluster_segment_boundary {
    use super::*;
    use crate::{assert_eq2, CharLengthExt as _};

    const EMOJI_CHAR: char = '\u{1F600}'; // 😀
    const INPUT_RAW: &str = "a😀b😀c";
    const EMOJI_AS_BYTES: [u8; 4] = [240, 159, 152, 128];

    #[test]
    fn test_utf8_encoding_char_string() {
        // "😀", `char` is 4 bytes, this "😀" len_utf8() is 4. chars().count() is 1.
        {
            // Memory size of the char type itself (always 4 bytes in Rust).
            assert_eq2!(std::mem::size_of::<char>(), 4);

            // UTF-8 encoding length (how many bytes when encoded as UTF-8).
            let utf8_len = EMOJI_CHAR.len_utf8();
            assert_eq2!(utf8_len, 4);

            // Put emoji in an Vec of u8.
            let mut buffer: [u8; 4] = [0; 4];
            EMOJI_CHAR.encode_utf8(&mut buffer);
            assert_eq2!(buffer, EMOJI_AS_BYTES);

            // Character count vs byte count.
            let emoji_string = EMOJI_CHAR.to_string();
            let emoji_str: &str = emoji_string.as_ref();
            let byte_count = emoji_str.len();
            assert_eq2!(byte_count, 4);
            let char_count = emoji_str.len_chars(); // aka chars().count()
            assert_eq2!(char_count, len(1));
        }

        // "a", char is 4 bytes, this "a" len_utf8() is 1. chars().count() is also 1.
        {
            // Test with a simple ASCII character 'a'
            const ASCII_CHAR: char = 'a';
            const ASCII_AS_BYTES: [u8; 1] = [97]; // ASCII value of 'a'

            // Memory size of the char type itself (always 4 bytes in Rust).
            assert_eq2!(std::mem::size_of::<char>(), 4);

            // UTF-8 encoding length (how many bytes when encoded as UTF-8).
            let utf8_len = ASCII_CHAR.len_utf8();
            assert_eq2!(utf8_len, 1);

            // Put ASCII char in a Vec of u8.
            let mut buffer: [u8; 4] = [0; 4];
            ASCII_CHAR.encode_utf8(&mut buffer);
            // Only the first byte should match, rest should be 0
            assert_eq2!(buffer[0], ASCII_AS_BYTES[0]);
            assert_eq2!(buffer[1..], [0, 0, 0]);

            // Character count vs byte count.
            let ascii_string = ASCII_CHAR.to_string();
            let ascii_str: &str = ascii_string.as_ref();
            let byte_count = ascii_str.len();
            assert_eq2!(byte_count, 1);
            let char_count = ascii_str.len_chars(); // aka chars().count()
            assert_eq2!(char_count, len(1));
        }

        // "1", char is 4 bytes, this "1" len_utf8() is 1. chars().count() is also 1.
        {
            // Test with a simple ASCII digit '1'
            const DIGIT_CHAR: char = '1';
            const DIGIT_AS_BYTES: [u8; 1] = [49]; // ASCII value of '1'

            // Memory size of the char type itself (always 4 bytes in Rust).
            assert_eq2!(std::mem::size_of::<char>(), 4);

            // UTF-8 encoding length (how many bytes when encoded as UTF-8).
            let utf8_len = DIGIT_CHAR.len_utf8();
            assert_eq2!(utf8_len, 1);

            // Put digit in a Vec of u8.
            let mut buffer: [u8; 4] = [0; 4];
            DIGIT_CHAR.encode_utf8(&mut buffer);
            // Only the first byte should match, rest should be 0
            assert_eq2!(buffer[0], DIGIT_AS_BYTES[0]);
            assert_eq2!(buffer[1..], [0, 0, 0]);

            // Character count vs byte count.
            let digit_string = DIGIT_CHAR.to_string();
            let digit_str: &str = digit_string.as_ref();
            let byte_count = digit_str.len();
            assert_eq2!(byte_count, 1);
            let char_count = digit_str.len_chars(); // aka chars().count()
            assert_eq2!(char_count, len(1));
        }
    }

    #[test]
    fn test_index_str_byte_count_vs_char_count() {
        let input_string = format!("a{EMOJI_CHAR}b{EMOJI_CHAR}c");
        let input_str = input_string.as_str();
        assert_eq2!(input_str, INPUT_RAW);

        // Size in memory.
        let byte_count = input_str.len();
        assert_eq2!(byte_count, 11);

        // UTF-8 encoded chars in the string.
        let char_count = input_str.len_chars(); // aka chars().count()
        assert_eq2!(char_count, len(5));

        // Index bytes in the input_str.
        let input_str_bytes = input_str.as_bytes();
        assert_eq2!(input_str_bytes[0], b'a');
        assert_eq2!(input_str_bytes[1..=4], EMOJI_AS_BYTES);
        assert_eq2!(input_str_bytes[5], b'b');
        assert_eq2!(input_str_bytes[6..=9], EMOJI_AS_BYTES);
        assert_eq2!(input_str_bytes[10], b'c');

        // Index chars in the input_str using Chars iterator.
        let mut input_str_chars = input_str.chars();
        assert_eq2!(input_str_chars.next(), Some('a'));
        assert_eq2!(input_str_chars.next(), Some('😀'));
        assert_eq2!(input_str_chars.next(), Some('b'));
        assert_eq2!(input_str_chars.next(), Some('😀'));
        assert_eq2!(input_str_chars.next(), Some('c'));
        assert_eq2!(input_str_chars.next(), None);
    }

    #[test]
    fn test_input_contains_emoji() {
        let lines: Vec<GCString> =
            vec![GCString::from(INPUT_RAW), GCString::from(INPUT_RAW)];
        let slice = AsStrSlice::from(&lines);
        assert_eq2!(slice.lines.len(), 2);
        assert_eq2!(
            slice.to_inline_string(),
            format!("{INPUT_RAW}\n{INPUT_RAW}\n")
        );
        assert_eq2!(slice.lines[0].as_ref(), INPUT_RAW);
        assert_eq2!(slice.lines[1].as_ref(), INPUT_RAW);
        assert_eq2!(slice.lines[0].string, INPUT_RAW);
        assert_eq2!(slice.lines[1].string, INPUT_RAW);
        assert_eq2!(slice.lines[0].string.len(), INPUT_RAW.len());
        assert_eq2!(slice.lines[1].string.len(), INPUT_RAW.len());
    }
}

/// Unit tests for the [AsStrSlice] struct and its methods.
#[cfg(test)]
mod tests_as_str_slice_basic_functionality {
    use nom::Input;

    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, idx, len};

    #[test]
    fn test_gc_string_slice_basic_functionality() {
        as_str_slice_test_case!(slice, "Hello world", "This is a test", "Third line");

        // Test that we can iterate through characters.
        let mut chars: Vec<char> = vec![];
        let mut current = slice;
        while let Some(ch) = current.current_char() {
            chars.push(ch);
            current.advance();
        }

        let expected = "Hello world\nThis is a test\nThird line\n"; // Trailing newline for multiple lines
        let result: String = chars.into_iter().collect();
        std::assert_eq!(result, expected);
    }

    #[test]
    fn test_nom_input_position() {
        as_str_slice_test_case!(slice, "hello", "world");

        // Test position finding
        let pos = slice.position(|c| c == 'w');
        std::assert_eq!(pos, Some(6)); // "hello\n" = 6 chars, then 'w'

        let pos = slice.position(|c| c == 'z');
        std::assert_eq!(pos, None); // 'z' not found
    }

    pub mod fixtures {
        use crate::GCString;

        pub fn create_test_lines() -> Vec<GCString> {
            vec![
                GCString::new("Hello world"),
                GCString::new("Second line"),
                GCString::new("Third line"),
                GCString::new(""),
                GCString::new("Fifth line"),
            ]
        }

        pub fn create_simple_lines() -> Vec<GCString> {
            vec![GCString::new("abc"), GCString::new("def")]
        }
    }

    // Test From trait implementations
    #[test]
    fn test_from_slice() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(lines.as_slice());

        assert_eq!(slice.line_index, idx(0));
        assert_eq!(slice.char_index, idx(0));
        assert_eq!(slice.max_len, None);
        assert_eq!(slice.lines.len(), 5);
    }

    #[test]
    fn test_from_vec() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(&lines);

        assert_eq!(slice.line_index, idx(0));
        assert_eq!(slice.char_index, idx(0));
        assert_eq!(slice.max_len, None);
        assert_eq!(slice.lines.len(), 5);
    }

    // Test Clone and PartialEq traits
    #[test]
    fn test_clone_and_partial_eq() {
        let lines = fixtures::create_test_lines();
        let slice1 = AsStrSlice::from(lines.as_slice());
        let slice2 = slice1.clone();

        assert_eq!(slice1, slice2);

        let slice3 = slice1.take_from(1);
        assert_ne!(slice1, slice3);
    }

    // Test with_limit constructor and behavior.
    #[test]
    fn test_with_limit() {
        let lines = fixtures::create_test_lines();

        // Basic constructor test
        let slice = AsStrSlice::with_limit(&lines, idx(1), idx(3), Some(len(5)));
        assert_eq!(slice.line_index, idx(1));
        assert_eq!(slice.char_index, idx(3));
        assert_eq!(slice.max_len, Some(len(5)));

        // Test behavior with limit
        let content = slice.extract_to_line_end();
        assert_eq!(content, "ond l"); // "Second line" starting at index 3 with max 5 chars

        // Test with limit spanning multiple lines
        let multi_line_slice =
            AsStrSlice::with_limit(&lines, idx(0), idx(6), Some(len(15)));
        let result = multi_line_slice.to_inline_string();
        assert_eq!(result, "world\nSecond li"); // 15 chars total

        // Test with no limit
        let no_limit_slice = AsStrSlice::with_limit(&lines, idx(0), idx(6), None);
        let result = no_limit_slice.to_inline_string();
        assert_eq!(result, "world\nSecond line\nThird line\n\nFifth line\n");

        // Test with zero limit
        let zero_limit_slice =
            AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(0)));
        assert_eq!(zero_limit_slice.current_char(), None);
        assert_eq!(zero_limit_slice.input_len(), 0);
        assert_eq!(zero_limit_slice.to_inline_string(), "");

        // Test with out-of-bounds line index
        let oob_slice = AsStrSlice::with_limit(&lines, idx(10), idx(0), None);
        assert_eq!(oob_slice.current_char(), None);
        assert_eq!(oob_slice.input_len(), 0);
        assert_eq!(oob_slice.to_inline_string(), "");

        // Test with out-of-bounds char index
        let oob_char_slice = AsStrSlice::with_limit(&lines, idx(0), idx(100), None);
        assert_eq!(oob_char_slice.current_char(), None);
        assert_eq!(oob_char_slice.to_inline_string(), "");
    }

    // Test extract_remaining_text_content_in_line
    #[test]
    fn test_extract_remaining_text_content_in_line() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(lines.as_slice());

        // From beginning of first line.
        assert_eq!(slice.extract_to_line_end(), "Hello world");

        // From middle of first line.
        let slice_offset = slice.take_from(6);
        assert_eq!(slice_offset.extract_to_line_end(), "world");

        // From empty line
        let slice_empty = AsStrSlice::with_limit(&lines, idx(3), idx(0), None);
        assert_eq!(slice_empty.extract_to_line_end(), "");

        // With max_len limit
        let slice_limited = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(5)));
        assert_eq!(slice_limited.extract_to_line_end(), "Hello");

        // Out of bounds
        let slice_oob = AsStrSlice::with_limit(&lines, idx(10), idx(0), None);
        assert_eq!(slice_oob.extract_to_line_end(), "");
    }

    // Test current_char and advance
    #[test]
    fn test_current_char_and_advance() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let mut slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef\n" (synthetic \n added between lines + trailing \n)
        // Positions: a(0), b(1), c(2), \n(3), d(4), e(5), f(6), \n(7)

        // Test normal characters
        assert_eq!(slice.current_char(), Some('a'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('b'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('c'));
        slice.advance();

        // Test synthetic newline between lines
        assert_eq!(slice.current_char(), Some('\n'));
        slice.advance();

        // Test second line
        assert_eq!(slice.current_char(), Some('d'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('e'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('f'));
        slice.advance();

        // Test trailing newline for multiple lines.
        assert_eq!(slice.current_char(), Some('\n'));
        slice.advance();

        // Test end of input.
        assert_eq!(slice.current_char(), None);
    }

    #[test]
    fn test_advance_with_max_len_zero_at_end_of_line() {
        // This test specifically covers the scenario that was causing the parser to hang:
        // When max_len=0 and we're at the end of a line, advance() should still move to
        // the next line.

        as_str_slice_test_case!(slice, "short line", "next line");
        let mut slice = slice;

        // Advance to end of first line
        for _ in 0..10 {
            // "short line" has 10 characters
            slice.advance();
        }

        // At this point we should be at the end of the first line
        assert_eq2!(slice.line_index, idx(0));
        assert_eq2!(slice.char_index, idx(10));

        // Now set max_len to 0 to simulate the problematic condition
        slice.max_len = Some(len(0));

        // The advance() should still work and move us to the next line
        slice.advance();

        // We should now be at the beginning of the second line
        assert_eq2!(slice.line_index, idx(1));
        assert_eq2!(slice.char_index, idx(0));
    }

    #[test]
    fn test_advance_through_multiline_content() {
        // Test advancing through multiple lines to ensure proper line transitions
        as_str_slice_test_case!(slice, "ab", "cd", "ef");
        let mut slice = slice;

        let expected_positions = vec![
            // First line: "ab"
            (0, 0, Some('a')),
            (0, 1, Some('b')),
            (0, 2, Some('\n')), // Synthetic newline
            // Second line: "cd"
            (1, 0, Some('c')),
            (1, 1, Some('d')),
            (1, 2, Some('\n')), // Synthetic newline
            // Third line: "ef"
            (2, 0, Some('e')),
            (2, 1, Some('f')),
            (2, 2, Some('\n')), // Trailing synthetic newline
            (2, 3, None),       // Past end
        ];

        for (expected_line, expected_char, expected_current_char) in expected_positions {
            assert_eq2!(slice.line_index.as_usize(), expected_line);
            assert_eq2!(slice.char_index.as_usize(), expected_char);
            assert_eq2!(slice.current_char(), expected_current_char);

            if slice.current_char().is_some() {
                slice.advance();
            }
        }
    }

    #[test]
    fn test_advance_with_max_len_constraint() {
        // Test that advance() respects max_len constraints
        as_str_slice_test_case!(slice, "hello world", "second line");
        let mut limited_slice = slice.take(5); // Only "hello"

        // Should be able to advance 5 times to consume "hello"
        for i in 0..5 {
            assert_eq2!(limited_slice.char_index.as_usize(), i);
            limited_slice.advance();
        }

        // At this point, max_len should be 0 and we should be at position 5
        assert_eq2!(limited_slice.char_index.as_usize(), 5);
        assert_eq2!(limited_slice.max_len, Some(len(0)));

        // Further advances should not move the position when max_len is exhausted
        // (unless we're at end of line transitioning to next line)
        let original_position = (limited_slice.line_index, limited_slice.char_index);
        limited_slice.advance();
        // Position should remain the same since we're in the middle of a line with
        // max_len=0
        assert_eq2!(
            (limited_slice.line_index, limited_slice.char_index),
            original_position
        );
    }

    #[test]
    fn test_advance_single_line_behavior() {
        // Test advance behavior with single line (no trailing newline)
        as_str_slice_test_case!(slice, "hello");
        let mut slice = slice;

        // Advance through all characters
        for i in 0..5 {
            assert_eq2!(slice.char_index.as_usize(), i);
            slice.advance();
        }

        // After consuming all characters, we should be at the end
        assert_eq2!(slice.char_index.as_usize(), 5);
        assert_eq2!(slice.current_char(), None); // No trailing newline for single line

        // Further advances should be no-ops
        let final_position = (slice.line_index, slice.char_index);
        slice.advance();
        assert_eq2!((slice.line_index, slice.char_index), final_position);
    }

    #[test]
    fn test_advance_empty_lines() {
        // Test advance behavior with empty lines
        as_str_slice_test_case!(slice, "", "content", "");
        let mut slice = slice;

        let expected_sequence = vec![
            (0, 0, Some('\n')), // Empty first line -> synthetic newline
            (1, 0, Some('c')),  // Start of "content"
            (1, 1, Some('o')),
            (1, 2, Some('n')),
            (1, 3, Some('t')),
            (1, 4, Some('e')),
            (1, 5, Some('n')),
            (1, 6, Some('t')),
            (1, 7, Some('\n')), // End of "content" -> synthetic newline
            (2, 0, Some('\n')), // Empty last line -> trailing newline
            (2, 1, None),       // Past end
        ];

        for (expected_line, expected_char, expected_current_char) in expected_sequence {
            assert_eq2!(slice.line_index.as_usize(), expected_line);
            assert_eq2!(slice.char_index.as_usize(), expected_char);
            assert_eq2!(slice.current_char(), expected_current_char);

            if slice.current_char().is_some() {
                slice.advance();
            }
        }
    }
}

/// Tests for the `is_empty()` method to ensure it correctly identifies when no more
/// characters can be consumed from the current position.
#[cfg(test)]
mod tests_is_empty {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_is_empty_with_max_len_zero() {
        // Test when max_len is set to 0
        as_str_slice_test_case!(slice, "hello");
        let slice_with_max_len_zero = slice.take(0);

        assert_eq2!(slice_with_max_len_zero.max_len, Some(len(0)));
        assert_eq2!(slice_with_max_len_zero.is_empty(), true);
    }

    #[test]
    fn test_is_empty_all_chars_consumed() {
        // Test when all available characters have been consumed
        as_str_slice_test_case!(slice, "hello");
        let mut consumed_slice = slice;

        // Advance through all characters
        for _ in 0..5 {
            consumed_slice.advance();
        }

        // At this point, current_taken should equal total_size
        assert_eq2!(consumed_slice.current_taken, consumed_slice.total_size);
        assert_eq2!(consumed_slice.is_empty(), true);
    }

    #[test]
    fn test_is_empty_beyond_available_lines() {
        // Test when we're beyond the available lines
        as_str_slice_test_case!(slice, "line1", "line2");

        // Create a slice with line_index beyond the available lines
        let mut beyond_lines = slice;
        beyond_lines.line_index = idx(2); // Only 2 lines (indices 0 and 1) exist

        assert_eq2!(beyond_lines.is_empty(), true);
    }

    #[test]
    fn test_is_empty_empty_lines_array() {
        // Test when the lines array is empty
        let empty_lines: Vec<GCString> = vec![];
        let slice = AsStrSlice::from(&empty_lines);

        assert_eq2!(slice.lines.is_empty(), true);
        assert_eq2!(slice.is_empty(), true);
    }

    #[test]
    fn test_is_empty_no_current_char() {
        // Test when there's no current character available
        as_str_slice_test_case!(slice, "hello");
        let mut no_char_slice = slice;

        // Move to a position where current_char() returns None
        no_char_slice.char_index = idx(5); // Beyond the end of "hello"

        assert_eq2!(no_char_slice.current_char(), None);
        assert_eq2!(no_char_slice.is_empty(), true);
    }

    #[test]
    fn test_is_not_empty() {
        // Test cases where is_empty() should return false

        // Regular non-empty slice
        as_str_slice_test_case!(slice1, "hello");
        assert_eq2!(slice1.is_empty(), false);

        // Slice with content after the current position
        as_str_slice_test_case!(slice2, "hello", "world");
        let mut middle_slice = slice2;
        middle_slice.line_index = idx(0);
        middle_slice.char_index = idx(2); // At 'l' in "hello"

        assert_eq2!(middle_slice.is_empty(), false);

        // Slice with max_len greater than 0
        as_str_slice_test_case!(slice3, "hello");
        let limited_slice = slice3.take(3); // Only "hel"
        assert_eq2!(limited_slice.max_len, Some(len(3)));
        assert_eq2!(limited_slice.is_empty(), false);
    }
}

#[cfg(test)]
mod tests_is_empty_character_exhaustion {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, len};

    #[test]
    fn test_is_empty_basic_behavior() {
        // Empty slice should be empty
        {
            let empty_lines: &[GCString] = &[];
            let slice = AsStrSlice::from(empty_lines);
            assert_eq2!(slice.is_empty(), true);
        }

        // Non-empty slice at start should not be empty
        {
            as_str_slice_test_case!(slice, "hello");
            assert_eq2!(slice.is_empty(), false);
        }
    }

    #[test]
    fn test_is_empty_when_current_taken_equals_total_size() {
        // Test the new behavior: is_empty() returns true when current_taken >= total_size
        {
            as_str_slice_test_case!(slice, "hello", "world");
            let mut slice = slice;

            // Initially not empty
            assert_eq2!(slice.is_empty(), false);
            assert_eq2!(slice.current_taken < slice.total_size, true);

            // Advance through all characters
            while slice.current_char().is_some() {
                slice.advance();
            }

            // Now should be empty because current_taken >= total_size
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_taken >= slice.total_size, true);
        }
    }

    #[test]
    fn test_is_empty_with_max_len_zero() {
        // max_len = 0 should make slice empty regardless of content
        {
            as_str_slice_test_case!(slice, limit: 0, "hello world");
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.max_len, Some(len(0)));
        }
    }

    #[test]
    fn test_is_empty_past_available_lines() {
        // When line_index >= lines.len(), should be empty
        {
            as_str_slice_test_case!(slice, "hello");
            let past_end = AsStrSlice::with_limit(
                slice.lines,
                idx(1), // Past the only line (index 0)
                idx(0),
                None,
            );
            assert_eq2!(past_end.is_empty(), true);
        }
    }

    #[test]
    fn test_is_empty_single_line_exhausted() {
        // Single line: when all characters consumed, should be empty
        {
            as_str_slice_test_case!(slice, "hi");
            let mut slice = slice;

            // Advance to end of line
            slice.advance(); // 'h'
            slice.advance(); // 'i'

            // Now at end of single line, should be empty
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_char(), None);
        }
    }

    #[test]
    fn test_is_empty_multiline_exhausted() {
        // Multiple lines: when all characters consumed, should be empty
        {
            as_str_slice_test_case!(slice, "a", "b");
            let mut slice = slice;

            // Total: "a" (1) + "\n" (1) + "b" (1) + "\n" (1) = 4 chars
            let expected_total = 4;
            assert_eq2!(slice.total_size.as_usize(), expected_total);

            // Advance through all characters
            for _ in 0..expected_total {
                assert_eq2!(slice.is_empty(), false);
                slice.advance();
            }

            // Now should be empty
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_taken.as_usize(), expected_total);
        }
    }

    #[test]
    fn test_is_empty_with_unicode() {
        // Test with Unicode characters
        {
            as_str_slice_test_case!(slice, "😀hello");
            let mut slice = slice;

            // Should not be empty initially
            assert_eq2!(slice.is_empty(), false);

            // Advance through all characters: 😀(1) + hello(5) = 6 chars
            for _ in 0..6 {
                assert_eq2!(slice.is_empty(), false);
                slice.advance();
            }

            // Now should be empty
            assert_eq2!(slice.is_empty(), true);
        }
    }

    #[test]
    fn test_is_empty_empty_lines_in_multiline() {
        // Test with empty lines in multiline content
        {
            as_str_slice_test_case!(slice, "", "content", "");
            let mut slice = slice;

            // Initially not empty
            assert_eq2!(slice.is_empty(), false);

            // Advance through all: "" + "\n" + "content" + "\n" + "" + "\n" = 10 chars
            let expected_chars = ['\n', 'c', 'o', 'n', 't', 'e', 'n', 't', '\n', '\n'];

            for expected_char in expected_chars {
                assert_eq2!(slice.is_empty(), false);
                assert_eq2!(slice.current_char(), Some(expected_char));
                slice.advance();
            }

            // Now should be empty
            assert_eq2!(slice.is_empty(), true);
        }
    }

    #[test]
    fn test_is_empty_respects_max_len() {
        // When max_len limits available characters, is_empty should respect that
        {
            as_str_slice_test_case!(slice, limit: 3, "hello world");
            let mut slice = slice;

            // Initially not empty
            assert_eq2!(slice.is_empty(), false);

            // Advance 3 characters (the limit)
            for _ in 0..3 {
                assert_eq2!(slice.is_empty(), false);
                slice.advance();
            }

            // Now should be empty due to max_len limit, even though there's more content
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.max_len, Some(len(0))); // Should be exhausted
        }
    }

    #[test]
    fn test_is_empty_consistency_with_current_char() {
        // is_empty() should be consistent with current_char() returning None
        {
            as_str_slice_test_case!(slice, "test");
            let mut slice = slice;

            while !slice.is_empty() {
                assert_eq2!(slice.current_char().is_some(), true);
                slice.advance();
            }

            // When empty, current_char should return None
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_char(), None);
        }
    }

    #[test]
    fn test_is_empty_edge_case_character_exhaustion_vs_line_exhaustion() {
        // Test the edge case where we're at an empty line but have consumed all chars
        {
            as_str_slice_test_case!(slice, "text", "");
            let mut slice = slice;

            // Advance through "text" + synthetic newline = 5 chars
            for _ in 0..5 {
                slice.advance();
            }

            // Now we're at the start of the empty line, line_index=1, char_index=0
            assert_eq2!(slice.line_index.as_usize(), 1);
            assert_eq2!(slice.char_index.as_usize(), 0);

            // But we haven't consumed all characters yet (empty line has a trailing
            // newline)
            assert_eq2!(slice.is_empty(), false);
            assert_eq2!(slice.current_char(), Some('\n')); // Synthetic newline from empty line

            // Advance one more to consume the trailing newline
            slice.advance();

            // Now should be empty (all characters consumed)
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_taken >= slice.total_size, true);
        }
    }
}

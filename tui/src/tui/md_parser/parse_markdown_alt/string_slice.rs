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
use std::{borrow::Cow, fmt::Display};

use nom::{Compare, CompareResult, FindSubstring, Input, Offset};

use crate::{constants::{NEW_LINE, NEW_LINE_CHAR},
            DocumentStorage,
            GCString,
            ParserByteCache,
            PARSER_BYTE_CACHE_PAGE_SIZE};

/// Wrapper type that implements [nom::Input] for &[GCString] or **any other type** that
/// implements [AsRef<str>]. The [Clone] operations on this struct are really cheap.
///
/// This struct generates synthetic new lines when it's [nom::Input] methods are used
/// to manipulate it. This ensures that it can make the underline `line` struct "act" like
/// it is a contiguous array of chars.
///
/// Since this struct implements [nom::Input], it can be used in any function that can
/// receive an argument that implements it. So you have flexibility in using the
/// [AsStrSlice] type or the [nom::Input] type where appropriate.
///
/// Also it is preferable to use the following function signature:
///
/// ```
/// # use r3bl_tui::AsStrSlice;
/// # use nom::IResult;
/// fn f<'a>(i: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> { unimplemented!() }
/// ```
///
/// Instead of the overly generic and difficult to work with (this type of signature makes
/// sense for the built-in parsers which are expected to work with any slice, but our use
/// case is anchored in [AsStrSlice], which itself is very flexible):
///
/// ```
/// # use r3bl_tui::AsStrSlice;
/// # use nom::{IResult, Input, Compare, Offset, AsChar};
/// # use std::fmt::Debug;
/// fn f<'a, I>(input: I) -> IResult<I, I>
/// where
///       I: Input + Clone + Compare<&'a str> + Offset + Debug,
///       I::Item: AsChar + Copy
/// { unimplemented!() }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AsStrSlice<'a, T: AsRef<str> = GCString>
where
    &'a [T]: Copy,
{
    pub lines: &'a [T],
    // Position tracking: (line_index, char_index_within_line).
    // Special case: if char_index == line.len(), we're at the synthetic newline.
    pub line_index: usize,
    pub char_index: usize,
    // Optional maximum length limit for the slice.
    pub max_len: Option<usize>,
}

/// Implement [From] trait to allow automatic conversion from &[GCString] to
/// [AsStrSlice].
impl<'a> From<&'a [GCString]> for AsStrSlice<'a> {
    fn from(lines: &'a [GCString]) -> Self {
        Self {
            lines,
            line_index: 0,
            char_index: 0,
            max_len: None,
        }
    }
}

/// Implement [From] trait to allow automatic conversion from &[[GCString]; N] to
/// [AsStrSlice]. Primary use case is for tests where the inputs are hardcoded as
/// fixed-size arrays.
impl<'a, const N: usize> From<&'a [GCString; N]> for AsStrSlice<'a> {
    fn from(lines: &'a [GCString; N]) -> Self {
        Self {
            lines: lines.as_slice(),
            line_index: 0,
            char_index: 0,
            max_len: None,
        }
    }
}

/// Implement [From] trait to allow automatic conversion from &[Vec<GCString>] to
/// [AsStrSlice].
impl<'a> From<&'a Vec<GCString>> for AsStrSlice<'a> {
    fn from(lines: &'a Vec<GCString>) -> Self {
        Self {
            lines,
            line_index: 0,
            char_index: 0,
            max_len: None,
        }
    }
}

impl<'a> AsStrSlice<'a> {
    /// Use the Display implementation to materialize the [DocumentStorage] content.
    pub fn to_inline_string(&self) -> DocumentStorage {
        let mut acc = DocumentStorage::new();
        use std::fmt::Write as _;
        _ = write!(acc, "{}", self);
        acc
    }

    pub fn write_to_byte_cache(
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

        // Write the content into the cache.
        use std::fmt::Write as _;
        _ = write!(acc, "{}", self);
    }

    // Create a new slice with a maximum length limit
    pub fn with_limit(
        lines: &'a [GCString],
        start_line: usize,
        start_char: usize,
        max_len: Option<usize>,
    ) -> Self {
        Self {
            lines,
            line_index: start_line,
            char_index: start_char,
            max_len,
        }
    }

    /// Extracts text content from the current position (`line_index`, `char_index`) to
    /// the end of the line (optionally limited `by max_len`).
    ///
    /// It handles various edge cases like:
    /// - Being at the end of a line.
    /// - Length limitations.
    /// - Optimizations for lines without newlines.
    /// - Fallback to empty string for invalid positions.
    ///
    /// Returns a string reference to the slice content.
    ///
    /// # Examples
    ///
    /// ```
    /// # use r3bl_tui::{GCString, AsStrSlice};
    /// # use nom::Input;
    /// let lines = vec![GCString::new("Hello world"), GCString::new("Second line")];
    /// let slice = AsStrSlice::new(&lines);
    ///
    /// // Extract from beginning of first line.
    /// let content = slice.extract_remaining_text_content_in_line();
    /// assert_eq!(content, "Hello world");
    ///
    /// // Extract with position offset.
    /// let slice_offset = slice.take_from(6); // Start from "world".
    /// assert_eq!(slice_offset.extract_remaining_text_content_in_line(), "world");
    /// ```
    ///
    /// # Edge Cases
    ///
    /// - **Empty lines**: Returns empty string for empty lines
    /// - **Out of bounds**: Returns empty string when `line_index >= lines.len()`
    /// - **Character index beyond line**: Clamps `char_index` to line length
    /// - **Zero max_len**: When `max_len` is `Some(0)`, returns empty string
    pub fn extract_remaining_text_content_in_line(&self) -> &'a str {
        // If we're looking at a slice with a valid line index, we can extract text from
        // that line
        if self.line_index < self.lines.len() {
            let line = &self.lines[self.line_index].string;
            let start = self.char_index.min(line.len());

            // If we have a max_len limit, we need to respect it
            if let Some(max_len) = self.max_len {
                let end = (start + max_len).min(line.len());
                return &line[start..end];
            }

            // If we're at the beginning of a line, return the whole line
            // This handles the case in test_parse_unique_kv_generic where we need to
            // extract "gc_value"
            if start == 0 {
                return line;
            }

            // Optimization from extract_text_content: special case for the last line
            if self.line_index == self.lines.len() - 1
                && !line[start..].contains(NEW_LINE)
            {
                return &line[start..];
            }

            // If we're on a line and there are no newlines, return a direct reference
            if !line[start..].contains(NEW_LINE) {
                return &line[start..];
            }
        }

        ""
    }

    /// For multiline content this will allocate, since there is no contiguous chunk of
    /// memory that has `\n` in them, since these new lines are generated
    /// synthetically when iterating this struct. Thus it is not possible to take
    /// chunks from [Self::lines] and then "join" them with `\n` in between lines, WITHOUT
    /// allocating.
    ///
    /// In the case there is only one line, this method will NOT allocate. This is why
    /// [Cow] is used.
    pub fn extract_remaining_text_content_to_end(&self) -> Cow<'a, str> {
        if self.line_index >= self.lines.len() {
            return Cow::Borrowed("");
        }

        let current_line = self.lines[self.line_index].as_ref();
        let start_char = std::cmp::min(self.char_index, current_line.len());

        // Calculate how much content we can actually return based on max_len
        let max_chars = self.remaining_len();

        // If we're on the last line, just return the remaining content from current line
        if self.line_index == self.lines.len() - 1 {
            let remaining_in_line = &current_line[start_char..];
            let content = if max_chars < remaining_in_line.len() {
                &remaining_in_line[..max_chars]
            } else {
                remaining_in_line
            };
            return Cow::Borrowed(content);
        }

        // Multi-line case: need to allocate and join with newlines
        let mut result = String::new();
        let mut chars_added = 0;

        // Add remaining content from current line
        let remaining_in_current = &current_line[start_char..];
        let chars_to_add = std::cmp::min(remaining_in_current.len(), max_chars);
        result.push_str(&remaining_in_current[..chars_to_add]);
        chars_added += chars_to_add;

        if chars_added >= max_chars {
            return Cow::Owned(result);
        }

        // Add subsequent lines with newlines
        for line_idx in (self.line_index + 1)..self.lines.len() {
            // Add newline
            if chars_added >= max_chars {
                break;
            }
            result.push('\n');
            chars_added += 1;

            if chars_added >= max_chars {
                break;
            }

            // Add line content
            let line_content = self.lines[line_idx].as_ref();
            let remaining_chars = max_chars - chars_added;
            let chars_to_add = std::cmp::min(line_content.len(), remaining_chars);

            result.push_str(&line_content[..chars_to_add]);
            chars_added += chars_to_add;

            if chars_added >= max_chars {
                break;
            }
        }

        Cow::Owned(result)
    }

    // Get the current character without materializing the full string
    pub fn current_char(&self) -> Option<char> {
        // Check if we've hit the max_len limit
        if let Some(max_len) = self.max_len {
            if max_len == 0 {
                return None;
            }
        }

        if self.line_index >= self.lines.len() {
            return None;
        }

        let line = &self.lines[self.line_index].string;

        if self.char_index < line.len() {
            // We're within the line content
            line.chars().nth(self.char_index)
        } else if self.char_index == line.len() && self.line_index < self.lines.len() - 1
        {
            // We're at the synthetic newline (between lines)
            Some(NEW_LINE_CHAR)
        } else {
            // End of input
            None
        }
    }

    // Advance position by one character
    pub fn advance(&mut self) {
        // Check if we've hit the max_len limit
        if let Some(max_len) = self.max_len {
            if max_len == 0 {
                return;
            }
            // Decrement max_len as we advance
            self.max_len = Some(max_len - 1);
        }

        if self.line_index >= self.lines.len() {
            return;
        }

        let line = &self.lines[self.line_index].string;

        if self.char_index < line.len() {
            // Move to next character within the line
            self.char_index += 1;
        } else if self.char_index == line.len() && self.line_index < self.lines.len() - 1
        {
            // We're at the synthetic newline, move to next line
            self.line_index += 1;
            self.char_index = 0;
        }
        // If we're at the end, don't advance further
    }

    // Get remaining length without materializing string
    fn remaining_len(&self) -> usize {
        if self.line_index >= self.lines.len() {
            return 0;
        }

        let mut total = 0;

        // Count remaining chars in current line
        let current_line = &self.lines[self.line_index].string;
        if self.char_index < current_line.len() {
            total += current_line.len() - self.char_index;
        }

        // Add synthetic newline if not at last line and we haven't passed the end of
        // current line
        if self.line_index < self.lines.len() - 1 && self.char_index <= current_line.len()
        {
            total += 1; // synthetic newline
        }

        // Add all subsequent lines plus their synthetic newlines
        for i in (self.line_index + 1)..self.lines.len() {
            total += self.lines[i].string.len();
            if i < self.lines.len() - 1 {
                total += 1; // synthetic newline
            }
        }

        // Apply max_len limit if set
        if let Some(max_len) = self.max_len {
            total.min(max_len)
        } else {
            total
        }
    }
}

impl<'a> Input for AsStrSlice<'a> {
    type Item = char;
    type Iter = StringChars<'a>;
    type IterIndices = StringCharIndices<'a>;

    fn iter_indices(&self) -> Self::IterIndices { StringCharIndices::new(self.clone()) }

    fn iter_elements(&self) -> Self::Iter { StringChars::new(self.clone()) }

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

    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        let remaining = self.remaining_len();
        if count <= remaining {
            Ok(count)
        } else {
            Err(nom::Needed::new(count - remaining))
        }
    }

    fn input_len(&self) -> usize { self.remaining_len() }

    fn take(&self, count: usize) -> Self {
        // take() should return a slice containing the first 'count' characters
        // Create a slice that starts at current position with max_len = count
        Self::with_limit(self.lines, self.line_index, self.char_index, Some(count))
    }

    fn take_from(&self, start: usize) -> Self {
        let mut result = self.clone();

        // Advance to the start position
        for _ in 0..start.min(self.remaining_len()) {
            result.advance();
        }

        // Reset max_len since we're creating a new slice from the advanced position
        result.max_len = None;

        result
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let taken = self.take(count);
        let remaining = self.take_from(count);
        (taken, remaining)
    }
}

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

/// Implement [Offset] trait for [AsStrSlice].
impl<'a> Offset for AsStrSlice<'a> {
    fn offset(&self, second: &Self) -> usize {
        // Calculate the character offset between two AsStrSlice instances
        // The second slice must be a part of self (advanced from self)

        // If they point to different line arrays, we can't calculate a meaningful offset
        if !std::ptr::eq(self.lines.as_ptr(), second.lines.as_ptr()) {
            return 0;
        }

        // If second is before self, return 0 (invalid case)
        if second.line_index < self.line_index
            || (second.line_index == self.line_index
                && second.char_index < self.char_index)
        {
            return 0;
        }

        let mut offset = 0;

        // Count characters from self's position to second's position
        let mut current_line = self.line_index;
        let mut current_char = self.char_index;

        while current_line < second.line_index
            || (current_line == second.line_index && current_char < second.char_index)
        {
            if current_line >= self.lines.len() {
                break;
            }

            let line = &self.lines[current_line].string;

            if current_line < second.line_index {
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
                let end_char = second.char_index.min(line.len());
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
        // Materialize the text by collecting all characters from the current position
        let mut current = self.clone();
        while let Some(ch) = current.current_char() {
            write!(f, "{}", ch)?;
            current.advance();
        }
        Ok(())
    }
}

/// Implement [FindSubstring] trait for [AsStrSlice].
impl<'a> FindSubstring<&str> for AsStrSlice<'a> {
    fn find_substring(&self, sub_str: &str) -> Option<usize> {
        // Convert the AsStrSlice to a string representation
        let full_text = self.extract_remaining_text_content_to_end();

        // Find the substring in the full text
        full_text.find(sub_str)
    }
}

pub struct StringChars<'a> {
    slice: AsStrSlice<'a>,
}

impl<'a> StringChars<'a> {
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

pub struct StringCharIndices<'a> {
    slice: AsStrSlice<'a>,
    position: usize,
}

impl<'a> StringCharIndices<'a> {
    fn new(slice: AsStrSlice<'a>) -> Self { Self { slice, position: 0 } }
}

impl<'a> Iterator for StringCharIndices<'a> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.slice.current_char()?;
        let pos = self.position;
        self.slice.advance();
        self.position += 1;
        Some((pos, ch))
    }
}

/// Unit tests for the [AsStrSlice] struct and its methods.
#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use nom::{Compare, CompareResult, Input, Offset};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_gc_string_slice_basic_functionality() {
        let lines = vec![
            GCString::new("Hello world"),
            GCString::new("This is a test"),
            GCString::new("Third line"),
        ];

        let slice = AsStrSlice::from(&lines);

        // Test that we can iterate through characters
        let mut chars: Vec<char> = vec![];
        let mut current = slice;
        while let Some(ch) = current.current_char() {
            chars.push(ch);
            current.advance();
        }

        let expected = "Hello world\nThis is a test\nThird line";
        let result: String = chars.into_iter().collect();
        std::assert_eq!(result, expected);
    }

    #[test]
    fn test_nom_input_position() {
        let lines = vec![GCString::new("hello"), GCString::new("world")];

        let slice = AsStrSlice::from(&lines);

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

        pub fn create_three_lines() -> Vec<GCString> {
            vec![
                GCString::from("First line"),
                GCString::from("Second line"),
                GCString::from("Third line"),
            ]
        }
    }

    // Test From trait implementations
    #[test]
    fn test_from_slice() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(lines.as_slice());

        assert_eq!(slice.line_index, 0);
        assert_eq!(slice.char_index, 0);
        assert_eq!(slice.max_len, None);
        assert_eq!(slice.lines.len(), 5);
    }

    #[test]
    fn test_from_vec() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(&lines);

        assert_eq!(slice.line_index, 0);
        assert_eq!(slice.char_index, 0);
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

    // Test Debug trait
    #[test]
    fn test_debug() {
        let lines = fixtures::create_simple_lines();
        let slice = AsStrSlice::from(lines.as_slice());
        let debug_str = format!("{:?}", slice);

        assert!(debug_str.contains("AsStrSlice"));
        assert!(debug_str.contains("line_index: 0"));
        assert!(debug_str.contains("char_index: 0"));
    }

    // Test with_limit constructor
    #[test]
    fn test_with_limit() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::with_limit(&lines, 1, 3, Some(5));

        assert_eq!(slice.line_index, 1);
        assert_eq!(slice.char_index, 3);
        assert_eq!(slice.max_len, Some(5));
    }

    // Test extract_remaining_text_content_in_line
    #[test]
    fn test_extract_remaining_text_content_in_line() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(lines.as_slice());

        // From beginning of first line
        assert_eq!(
            slice.extract_remaining_text_content_in_line(),
            "Hello world"
        );

        // From middle of first line
        let slice_offset = slice.take_from(6);
        assert_eq!(
            slice_offset.extract_remaining_text_content_in_line(),
            "world"
        );

        // From empty line
        let slice_empty = AsStrSlice::with_limit(&lines, 3, 0, None);
        assert_eq!(slice_empty.extract_remaining_text_content_in_line(), "");

        // With max_len limit
        let slice_limited = AsStrSlice::with_limit(&lines, 0, 0, Some(5));
        assert_eq!(
            slice_limited.extract_remaining_text_content_in_line(),
            "Hello"
        );

        // Out of bounds
        let slice_oob = AsStrSlice::with_limit(&lines, 10, 0, None);
        assert_eq!(slice_oob.extract_remaining_text_content_in_line(), "");
    }

    // Test current_char and advance
    #[test]
    fn test_current_char_and_advance() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let mut slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (synthetic \n added between lines)
        // Positions: a(0), b(1), c(2), \n(3), d(4), e(5), f(6)

        // Test normal characters
        assert_eq!(slice.current_char(), Some('a'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('b'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('c'));
        slice.advance();

        // Test synthetic newline
        assert_eq!(slice.current_char(), Some('\n'));
        slice.advance();

        // Test second line
        assert_eq!(slice.current_char(), Some('d'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('e'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('f'));
        slice.advance();

        // Test end of input
        assert_eq!(slice.current_char(), None);
    }

    #[test]
    fn test_advance_with_max_len() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let mut slice = AsStrSlice::with_limit(&lines, 0, 0, Some(2));
        // Input appears as: "abc\ndef" but limited to first 2 chars: "ab"

        assert_eq!(slice.current_char(), Some('a'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('b'));
        slice.advance();
        assert_eq!(slice.current_char(), None); // Hit max_len limit
    }

    // Test Input trait implementation
    #[test]
    fn test_input_len() {
        let lines = fixtures::create_simple_lines(); // "abc", "def" = 6 chars + 1 newline = 7
        let slice = AsStrSlice::from(lines.as_slice());
        assert_eq!(slice.input_len(), 7);

        let slice_offset = slice.take_from(2);
        assert_eq!(slice_offset.input_len(), 5); // "c\ndef" (from position 2 to end)

        // With max_len
        let slice_limited = AsStrSlice::with_limit(&lines, 0, 0, Some(3));
        assert_eq!(slice_limited.input_len(), 3);
    }

    #[test]
    fn test_input_take() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (7 total chars)

        let taken = slice.take(3);
        assert_eq!(taken.max_len, Some(3));
        assert_eq!(taken.input_len(), 3); // Takes first 3 chars: "abc"
    }

    #[test]
    fn test_input_take_from() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (positions: a(0), b(1), c(2), \n(3), d(4), e(5),
        // f(6))

        let from_offset = slice.take_from(2);
        assert_eq!(from_offset.line_index, 0);
        assert_eq!(from_offset.char_index, 2); // Advanced to position 2: 'c'
        assert_eq!(from_offset.max_len, None);
    }

    #[test]
    fn test_input_take_split() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (synthetic \n added between lines)
        // Positions: a(0), b(1), c(2), \n(3), d(4), e(5), f(6)

        let (taken, remaining) = slice.take_split(3);
        assert_eq!(taken.max_len, Some(3));
        assert_eq!(taken.input_len(), 3);
        assert_eq!(remaining.char_index, 3); // Advanced by 3: 'a'(0), 'b'(1), 'c'(2) ->
                                             // now at '\n'(3)
    }

    #[test]
    fn test_input_position() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (positions: a(0), b(1), c(2), \n(3), d(4), e(5),
        // f(6))

        // Find newline
        let pos = slice.position(|c| c == '\n');
        assert_eq!(pos, Some(3)); // Synthetic newline at position 3

        // Find 'd'
        let pos = slice.position(|c| c == 'd');
        assert_eq!(pos, Some(4)); // 'd' at position 4 (after synthetic newline)

        // Find non-existent character
        let pos = slice.position(|c| c == 'z');
        assert_eq!(pos, None);
    }

    #[test]
    fn test_input_slice_index() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (7 total chars)

        // Valid count
        assert_eq!(slice.slice_index(3), Ok(3));
        assert_eq!(slice.slice_index(7), Ok(7)); // Full length

        // Invalid count
        let result = slice.slice_index(10);
        assert!(result.is_err());
        if let Err(nom::Needed::Size(size)) = result {
            assert_eq!(size.get(), 3); // 10 - 7 = 3 (need 3 more chars)
        }
    }

    // Test iterators
    #[test]
    fn test_iter_elements() {
        let lines = vec![GCString::new("ab"), GCString::new("cd")]; // Creates ["ab", "cd"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "ab\ncd" (synthetic \n added between lines)

        let chars: Vec<char> = slice.iter_elements().collect();
        assert_eq!(chars, vec!['a', 'b', '\n', 'c', 'd']); // Note synthetic newline
    }

    #[test]
    fn test_iter_indices() {
        let lines = vec![GCString::new("ab"), GCString::new("cd")]; // Creates ["ab", "cd"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "ab\ncd" (synthetic \n added between lines)

        let indexed_chars: Vec<(usize, char)> = slice.iter_indices().collect();
        assert_eq!(
            indexed_chars,
            vec![(0, 'a'), (1, 'b'), (2, '\n'), (3, 'c'), (4, 'd')] /* Note synthetic
                                                                     * newline at
                                                                     * index 2 */
        );
    }

    // Test Compare trait implementation
    #[test]
    fn test_compare() {
        let lines = vec![GCString::new("Hello"), GCString::new("World")];
        let slice = AsStrSlice::from(lines.as_slice());

        // Exact match
        assert_eq!(slice.compare("Hello"), CompareResult::Ok);

        // Partial match (target is longer)
        assert_eq!(slice.compare("Hello\nWorld"), CompareResult::Ok);

        // Mismatch
        assert_eq!(slice.compare("Hi"), CompareResult::Error);

        // Target longer than available input
        assert_eq!(
            slice.compare("Hello\nWorld\nExtra"),
            CompareResult::Incomplete
        );

        // Empty string
        assert_eq!(slice.compare(""), CompareResult::Ok);
    }

    #[test]
    fn test_compare_no_case() {
        let lines = vec![GCString::new("Hello"), GCString::new("World")];
        let slice = AsStrSlice::from(lines.as_slice());

        // Case insensitive match
        assert_eq!(slice.compare_no_case("hello"), CompareResult::Ok);
        assert_eq!(slice.compare_no_case("HELLO"), CompareResult::Ok);
        assert_eq!(slice.compare_no_case("HeLLo"), CompareResult::Ok);

        // Mismatch
        assert_eq!(slice.compare_no_case("hi"), CompareResult::Error);

        // Target longer than available input
        assert_eq!(
            slice.compare_no_case("hello\nworld\nextra"),
            CompareResult::Incomplete
        );
    }

    // Test Offset trait implementation
    #[test]
    fn test_offset() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice1 = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (positions: a(0), b(1), c(2), \n(3), d(4), e(5),
        // f(6))
        let slice2 = slice1.take_from(3); // After "abc" -> at position 3 ('\n')
        let slice3 = slice1.take_from(5); // After "abc\nde" -> at position 5 ('f')

        assert_eq!(slice1.offset(&slice1), 0);
        assert_eq!(slice1.offset(&slice2), 3); // 3 positions advanced
        assert_eq!(slice1.offset(&slice3), 5); // 5 positions advanced
        assert_eq!(slice2.offset(&slice3), 2); // 2 positions from '\n' to 'f'

        // Invalid case: second is before first
        assert_eq!(slice2.offset(&slice1), 0);
    }

    #[test]
    fn test_offset_different_arrays() {
        let lines1 = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let lines2 = fixtures::create_test_lines();
        let slice1 = AsStrSlice::from(lines1.as_slice());
        let slice2 = AsStrSlice::from(lines2.as_slice());

        // Different arrays should return 0
        assert_eq!(slice1.offset(&slice2), 0);
    }

    // Edge cases and error conditions
    #[test]
    fn test_empty_lines() {
        let lines: Vec<GCString> = vec![];
        let slice = AsStrSlice::from(lines.as_slice());

        assert_eq!(slice.current_char(), None);
        assert_eq!(slice.input_len(), 0);
        assert_eq!(slice.extract_remaining_text_content_in_line(), "");
    }

    #[test]
    fn test_single_empty_line() {
        let lines = vec![GCString::new("")];
        let slice = AsStrSlice::from(lines.as_slice());

        assert_eq!(slice.current_char(), None);
        assert_eq!(slice.input_len(), 0);
        assert_eq!(slice.extract_remaining_text_content_in_line(), "");
    }

    #[test]
    fn test_max_len_zero() {
        let lines = fixtures::create_simple_lines();
        let slice = AsStrSlice::with_limit(&lines, 0, 0, Some(0));

        assert_eq!(slice.current_char(), None);
        assert_eq!(slice.input_len(), 0);
        assert_eq!(slice.extract_remaining_text_content_in_line(), "");
    }

    #[test]
    fn test_advance_beyond_bounds() {
        let lines = vec![GCString::new("a")];
        let mut slice = AsStrSlice::from(lines.as_slice());

        assert_eq!(slice.current_char(), Some('a'));
        slice.advance(); // Now at end
        assert_eq!(slice.current_char(), None);
        slice.advance(); // Should not panic or change state
        assert_eq!(slice.current_char(), None);
    }

    #[test]
    fn test_with_newlines_in_content() {
        let lines = vec![GCString::new("line1\nembedded"), GCString::new("line2")];
        let slice = AsStrSlice::from(lines.as_slice());

        // Should handle embedded newlines in line content
        let content = slice.extract_remaining_text_content_in_line();
        assert_eq!(content, "line1\nembedded");
    }

    #[test]
    fn test_char_index_beyond_line_length() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, 0, 10, None); // char_index > line.len()
                                                                 // Input appears as: "abc\ndef" but starting at invalid position 10

        // Should clamp to line length
        assert_eq!(slice.extract_remaining_text_content_in_line(), "");
    }

    // Test Display trait implementation
    #[test]
    fn test_display_full_content() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef" (synthetic \n added between lines)

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "abc\ndef"); // Shows synthetic newline in output
    }

    #[test]
    fn test_display_from_offset() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice()).take_from(2);
        // Input appears as: "abc\ndef", starting from position 2 ('c')

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "c\ndef"); // From 'c' through synthetic newline to end
    }

    #[test]
    fn test_display_with_limit() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, 0, 0, Some(4));
        // Input appears as: "abc\ndef" but limited to first 4 chars: "abc\n"

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "abc\n"); // First 4 chars including synthetic newline
    }

    #[test]
    fn test_display_empty_slice() {
        let lines: Vec<GCString> = vec![];
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "");
    }

    #[test]
    fn test_display_single_line() {
        let lines = vec![GCString::new("hello")];
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "hello");
    }

    #[test]
    fn test_display_empty_lines() {
        let lines = vec![
            GCString::new(""),
            GCString::new("middle"),
            GCString::new(""),
        ];
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "\nmiddle\n");
    }

    #[test]
    fn test_display_with_embedded_newlines() {
        let lines = vec![GCString::new("line1\nembedded"), GCString::new("line2")];
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "line1\nembedded\nline2");
    }

    #[test]
    fn test_display_max_len_zero() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, 0, 0, Some(0));
        // Input appears as: "abc\ndef" but limited to 0 chars

        let displayed = format!("{}", slice);
        assert_eq!(displayed, ""); // No characters displayed due to max_len = 0
    }

    #[test]
    fn test_display_at_end_position() {
        let lines = vec![GCString::new("abc")];
        let slice = AsStrSlice::with_limit(&lines, 0, 3, None); // At end of line

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "");
    }

    #[test]
    fn test_display_multiline_complex() {
        let lines = fixtures::create_test_lines(); // Multiple lines including empty
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{}", slice);
        assert_eq!(
            displayed,
            "Hello world\nSecond line\nThird line\n\nFifth line"
        );
    }

    #[test]
    fn test_display_partial_from_middle() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::with_limit(&lines, 1, 7, Some(10)); // From "line" in "Second line"

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "line\nThird");
    }

    #[test]
    fn test_extract_remaining_text_content_to_end() {
        // Test single line - should return Cow::Borrowed
        {
            let lines = fixtures::create_simple_lines(); // ["abc", "def"]
            let mut slice = AsStrSlice::from(&lines);
            // Advance to position 2 in first line (after "ab")
            slice.advance();
            slice.advance();

            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "c\ndef");
            // Should be owned since it spans multiple lines
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test single line from start - should return Cow::Owned for multi-line
        {
            let lines = fixtures::create_simple_lines(); // ["abc", "def"]
            let slice = AsStrSlice::from(&lines);
            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "abc\ndef");
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test single line remaining content only
        {
            let lines = vec![GCString::from("Hello, World!")];
            let mut slice = AsStrSlice::from(&lines);
            // Advance to position 7 (after "Hello, ")
            for _ in 0..7 {
                slice.advance();
            }

            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "World!");
            // Should be borrowed since it's single line content
            assert!(matches!(result, Cow::Borrowed(_)));
        }

        // Test from start of single line
        {
            let lines = vec![GCString::from("Hello, World!")];
            let slice = AsStrSlice::from(&lines);
            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "Hello, World!");
            assert!(matches!(result, Cow::Borrowed(_)));
        }

        // Test at end of single line
        {
            let lines = fixtures::create_simple_lines(); // ["abc", "def"]
            let mut slice = AsStrSlice::from(&lines);
            // Advance to end
            for _ in 0..13 {
                slice.advance();
            }
            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "");
            assert!(matches!(result, Cow::Borrowed(_)));
        }

        // Test multi-line from middle
        {
            let lines = fixtures::create_test_lines(); // More complex test data
            let mut slice = AsStrSlice::from(&lines);
            // Advance a few characters into first line
            slice.advance();
            slice.advance();

            let result = slice.extract_remaining_text_content_to_end();
            // Result should contain remaining content from current position to end
            assert!(!result.is_empty());
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test with max_len limit on single line
        {
            let lines = vec![GCString::from("Hello, World!")];
            let slice = AsStrSlice::with_limit(&lines, 0, 7, Some(3)); // Start at pos 7, limit 3 chars

            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "Wor"); // Only 3 chars due to limit
            assert!(matches!(result, Cow::Borrowed(_)));
        }

        // Test with max_len limit on multi-line
        {
            let lines = vec![
                GCString::from("First"),
                GCString::from("Second"),
                GCString::from("Third"),
            ];
            let slice = AsStrSlice::with_limit(&lines, 0, 3, Some(7)); // Start at pos 3, limit 7 chars

            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "st\nSeco"); // "st" + "\n" + "Seco" = 7 chars
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test with max_len that cuts off at newline
        {
            let lines = vec![GCString::from("First"), GCString::from("Second")];
            let slice = AsStrSlice::with_limit(&lines, 0, 3, Some(4)); // Start at pos 3, limit 4 chars

            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "st\nS"); // "st" + "\n" + "S" = 4 chars
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test empty lines
        {
            let lines = vec![GCString::from(""), GCString::from(""), GCString::from("")];
            let slice = AsStrSlice::from(&lines);
            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "\n\n");
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test beyond end
        {
            let lines = vec![GCString::from("Hello")];
            let slice = AsStrSlice::with_limit(&lines, 10, 0, None); // Start beyond available lines
            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "");
            assert!(matches!(result, Cow::Borrowed(_)));
        }

        // Test starting from second line
        {
            let lines = fixtures::create_three_lines();
            let slice = AsStrSlice::with_limit(&lines, 1, 0, None); // Start at beginning of second line
            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "Second line\nThird line");
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test starting from middle of second line
        {
            let lines = fixtures::create_three_lines();
            let slice = AsStrSlice::with_limit(&lines, 1, 7, None); // Start at "line" in "Second line"
            let result = slice.extract_remaining_text_content_to_end();
            assert_eq!(result, "line\nThird line");
            assert!(matches!(result, Cow::Owned(_)));
        }
    }

    #[test]
    fn test_find_substring() {
        // Test basic substring finding in a single line
        let lines = vec![GCString::from("Hello, World!")];
        let slice = AsStrSlice::from(&lines);

        // Find substring in the middle
        let pos = slice.find_substring("World");
        assert_eq!(pos, Some(7));

        // Find substring at the beginning
        let pos = slice.find_substring("Hello");
        assert_eq!(pos, Some(0));

        // Find substring at the end
        let pos = slice.find_substring("!");
        assert_eq!(pos, Some(12));

        // Substring not found
        let pos = slice.find_substring("Rust");
        assert_eq!(pos, None);

        // Empty substring (should match at position 0)
        let pos = slice.find_substring("");
        assert_eq!(pos, Some(0));

        // Test finding substring across multiple lines
        let lines = vec![
            GCString::from("First line"),
            GCString::from("Second line"),
            GCString::from("Third line"),
        ];
        let slice = AsStrSlice::from(&lines);

        // Find substring that spans a newline
        let pos = slice.find_substring("line\nSecond");
        assert_eq!(pos, Some(6));

        // Find substring in the middle line
        let pos = slice.find_substring("Second");
        assert_eq!(pos, Some(11));

        // Test with offset position
        let slice = AsStrSlice::with_limit(&lines, 1, 0, None); // Start at beginning of second line
        let pos = slice.find_substring("Second");
        assert_eq!(pos, Some(0));

        // Test with max_len limit
        let slice = AsStrSlice::with_limit(&lines, 0, 0, Some(15)); // Limit to first 15 chars
        let pos = slice.find_substring("Second");
        assert_eq!(pos, None); // Should not find "Second" as it's beyond the limit

        // Test with empty lines
        let lines = vec![
            GCString::from(""),
            GCString::from(""),
            GCString::from("Content"),
        ];
        let slice = AsStrSlice::from(&lines);
        let pos = slice.find_substring("Content");
        assert_eq!(pos, Some(2)); // 2 newlines before "Content"
    }
}

/// These are tests to ensure that [AsStrSlice] works correctly in integration with
/// nom parsers.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::tui::md_parser::parse_markdown_alt::fragment_alt::take_text_between_generic;

    /// When the parser succeeds, it should return the remaining input and the
    /// extracted text.
    #[test]
    fn test_take_text_between_ok() {
        let lines = [GCString::new("_foo bar baz_bar")];
        let input = AsStrSlice::from(&lines);

        // Add assertions for the input slice before parsing.
        assert_eq!(input.char_index, 0);
        assert_eq!(input.input_len(), 16);
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.line_index, 0);
        assert_eq!(
            input.extract_remaining_text_content_in_line(),
            "_foo bar baz_bar"
        );

        // Extract the result for comparison.
        let it = take_text_between_generic(input, "_", "_");
        let (rem, output) = it.unwrap();

        // Make sure the remainder (remaining input) slice is correct.
        assert_eq!(rem.char_index, 13);
        assert_eq!(rem.input_len(), 3);
        assert_eq!(rem.lines.len(), 1);
        assert_eq!(rem.line_index, 0);
        assert_eq!(rem.extract_remaining_text_content_in_line(), "bar");

        // Check that extracted contains "foo bar baz". Use the Display trait impl to
        // convert it to a string.
        assert_eq!(output.to_string(), "foo bar baz");
        assert_eq!(output.char_index, 1);
        assert_eq!(output.input_len(), 11);
        assert_eq!(output.lines.len(), 1);
        assert_eq!(output.line_index, 0);
        assert_eq!(
            output.extract_remaining_text_content_in_line(),
            "foo bar baz"
        );
    }

    /// When a parser fails, it should return an error containing a copy of the "input" in
    /// the correct position.
    #[test]
    fn test_take_text_between_error() {
        let lines = [GCString::new("_foo bar baz")];
        let input = AsStrSlice::from(&lines);
        assert_eq!(input.char_index, 0);

        let res = take_text_between_generic(input, "_", "_");

        match res {
            Ok(_) => panic!("Expected an error, but got Ok"),
            Err(nom::Err::Error(error)) => {
                // Add more rigorous assertions for `error.input`.
                assert_eq!(error.code, nom::error::ErrorKind::TakeUntil);
                // `tag("_")` moved this forward by 1. it is no longer equal to `input`.
                assert_eq!(error.input.char_index, 1);
                assert_eq!(error.input.input_len(), 11); // "foo bar baz" remaining
                assert_eq!(error.input.lines.len(), 1);
                assert_eq!(error.input.line_index, 0);
                assert_eq!(error.input.max_len, None);
                assert_eq!(
                    error.input.extract_remaining_text_content_in_line(),
                    "foo bar baz"
                );
            }
            Err(other_err) => panic!("Expected Error variant, but got: {:?}", other_err),
        }
    }
}

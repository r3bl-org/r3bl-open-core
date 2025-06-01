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

use nom::{Compare, CompareResult, Input};

use crate::{constants::{NEW_LINE, NEW_LINE_CHAR},
            GCString};

/// Wrapper type that implements [nom::Input] for &[GCString]. The [Clone] operations on
/// this struct are really cheap, and it implements [Copy].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GCStringSlice<'a> {
    lines: &'a [GCString],
    // Position tracking: (line_index, char_index_within_line).
    // Special case: if char_index == line.len(), we're at the synthetic newline.
    line_index: usize,
    char_index: usize,
    // Optional maximum length limit for the slice.
    max_len: Option<usize>,
}

/// Implement [From] trait to allow automatic conversion from &[GCString] to
/// [GCStringSlice].
impl<'a> From<&'a [GCString]> for GCStringSlice<'a> {
    fn from(lines: &'a [GCString]) -> Self { Self::new(lines) }
}

impl<'a> GCStringSlice<'a> {
    pub fn new(lines: &'a [GCString]) -> Self {
        Self {
            lines,
            line_index: 0,
            char_index: 0,
            max_len: None,
        }
    }

    /// Convert from a slice reference to a GCStringSlice
    pub fn from_slice(lines: &'a [GCString]) -> Self { Self::new(lines) }

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
    /// # use r3bl_tui::{GCString, GCStringSlice};
    /// # use nom::Input;
    /// let lines = vec![GCString::new("Hello world"), GCString::new("Second line")];
    /// let slice = GCStringSlice::new(&lines);
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

impl<'a> Input for GCStringSlice<'a> {
    type Item = char;
    type Iter = GCStringChars<'a>;
    type IterIndices = GCStringCharIndices<'a>;

    fn iter_indices(&self) -> Self::IterIndices { GCStringCharIndices::new(*self) }

    fn iter_elements(&self) -> Self::Iter { GCStringChars::new(*self) }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        let mut pos = 0;
        let mut current = *self;

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
        let mut result = *self;

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

impl<'a> Compare<&str> for GCStringSlice<'a> {
    fn compare(&self, t: &str) -> CompareResult {
        let mut current = *self;
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
        let mut current = *self;
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

// Iterator implementations
pub struct GCStringChars<'a> {
    slice: GCStringSlice<'a>,
}

impl<'a> GCStringChars<'a> {
    fn new(slice: GCStringSlice<'a>) -> Self { Self { slice } }
}

impl<'a> Iterator for GCStringChars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.slice.current_char();
        if ch.is_some() {
            self.slice.advance();
        }
        ch
    }
}

pub struct GCStringCharIndices<'a> {
    slice: GCStringSlice<'a>,
    position: usize,
}

impl<'a> GCStringCharIndices<'a> {
    fn new(slice: GCStringSlice<'a>) -> Self { Self { slice, position: 0 } }
}

impl<'a> Iterator for GCStringCharIndices<'a> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.slice.current_char()?;
        let pos = self.position;
        self.slice.advance();
        self.position += 1;
        Some((pos, ch))
    }
}

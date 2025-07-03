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

//! # Character advancement and synthetic newline generation
//!
//! This module implements the core character advancement logic for [`AsStrSlice`], which
//! provides a memory-efficient way to iterate over multiline text content without
//! allocating contiguous strings with embedded newlines.
//!
//! ## Overview
//!
//! The [`AsStrSlice`] struct represents a slice of multiline text stored as separate
//! lines (`Vec<GCString>`). Since the original text doesn't contain actual `\n`
//! characters between lines, this module implements **synthetic newline generation** to
//! present a unified view of the text as if it were a single string with proper line
//! separators.
//!
//! ## Character advancement strategy
//!
//! The `advance()` method implements a state-machine approach to character advancement:
//!
//! ### Position states
//!
//! 1. **`WithinLineContent`**: Character index is within the current line's content
//!    - Simply increment `char_index` to move to the next character
//!    - This is the most common case for normal text processing
//!
//! 2. **`AtEndOfLine`**: Character index is at the end of the current line
//!    - For multiline documents: inject a synthetic `\n` character
//!    - Advance to the next line or handle trailing newlines appropriately
//!    - For single-line documents: no synthetic newline is added
//!
//! 3. **`PastEndOfLine`**: Character index exceeds the current line's length
//!    - No-op case - don't advance further to prevent invalid states
//!
//! ## Synthetic newline generation rules
//!
//! The synthetic newline behavior follows these rules based on document structure:
//!
//! ### Single line documents
//! ```text
//! Input: ["hello world"]
//! Output: "hello world" (no trailing newline)
//! ```
//!
//! ### Multiple line documents
//! ```text
//! Input: ["line1", "line2", "line3"]
//! Output: "line1\nline2\nline3\n" (newlines between + trailing newline)
//! ```
//!
//! ### Decision matrix
//!
//! | Position State        | Document Type   | Line Location        | Action                                   |
//! |-----------------------|-----------------|----------------------|------------------------------------------|
//! | `AtEndOfLine`         | `MultipleLines` | `HasMoreLinesAfter`  | Add synthetic `\n`, advance to next line |
//! | `AtEndOfLine`         | `MultipleLines` | `LastLine`           | Add trailing synthetic `\n`              |
//! | `AtEndOfLine`         | `SingleLine`    | -                    | No synthetic newline                     |
//! | `WithinLineContent`   | Any             | Any                  | Advance `char_index` within line         |
//! | `PastEndOfLine`       | Any             | Any                  | No-op                                    |
//!
//! ## Unicode safety
//!
//! ⚠️ **Critical**: All character operations use **character positions**, not byte
//! positions:
//! - `char_index` represents CHARACTER position within a line
//! - `advance()` moves by one CHARACTER, safely handling multi-byte UTF-8 sequences
//! - Length calculations use `str.len_chars()` for character counts
//! - This ensures safe handling of emojis, accented characters, and other Unicode content
//!
//! ## Memory efficiency
//!
//! The synthetic newline approach provides several benefits:
//! - **Zero allocation** for character advancement
//! - **Lazy evaluation** - newlines are generated on-demand during iteration
//! - **Memory efficient** - avoids creating large concatenated strings
//! - **Cache friendly** - original line data remains in separate allocations
//!
//! ## Integration with nom parser framework
//!
//! This module enables [`AsStrSlice`] to work seamlessly with the nom parser combinator
//! library:
//! - Implements `nom::Input` trait for nom compatibility.
//! - Supports `take()`, `take_from()`, and other nom operations.
//! - Maintains parser position state across multiline boundaries.
//! - Handles `max_len` limits for bounded parsing operations.
//!
//! ## Example usage
//!
//! ```rust
//! use r3bl_tui::{AsStrSlice, GCString};
//!
//! let lines = vec![
//!     GCString::from("Hello"),
//!     GCString::from("World"),
//! ];
//! let mut slice = AsStrSlice::from(&lines[..]);
//!
//! // Iterate through characters - synthetic newlines appear automatically
//! let chars: Vec<char> = std::iter::from_fn(|| {
//!     let ch = slice.current_char();
//!     if ch.is_some() { slice.advance(); }
//!     ch
//! }).collect();
//!
//! assert_eq!(chars, vec!['H', 'e', 'l', 'l', 'o', '\n', 'W', 'o', 'r', 'l', 'd', '\n']);
//! ```
//!
//! ## Related functions
//!
//! - `remaining_len()`: Calculate remaining characters without materializing string
//! - `calculate_total_size()`: Calculate total document size including synthetic newlines
//! - `calculate_current_taken()`: Calculate consumed characters up to current position
//! - `current_char()`: Get current character (including synthetic newlines)
//!
//! This implementation is fundamental to the markdown parsing system's ability to process
//! multiline content efficiently while maintaining compatibility with text processing
//! libraries that expect contiguous string-like interfaces.
//!
//! ## Enhancements for compatibility
//!
//! This module was enhanced as part of achieving true drop-in replacement compatibility
//! between the NG parser and legacy parser. Key improvements include:
//!
//! ### Enhanced state machine logic
//! - **Improved edge case handling**: Better detection of end-of-line vs end-of-input
//! - **Robust Unicode support**: Safe character advancement for multi-byte sequences
//! - **Consistent newline behavior**: Ensures synthetic newlines match legacy parser
//!
//! ### Integration with line advancement infrastructure
//! This module works closely with the `line_advancement.rs` infrastructure:
//! - **Character-level advancement**: Handles within-line character movement
//! - **Line boundary detection**: Signals when synthetic newlines are needed
//! - **State consistency**: Maintains accurate position tracking for parsers
//!
//! ### Compatibility testing
//! The synthetic newline generation was validated against 45+ compatibility test cases
//! to ensure identical behavior with the legacy parser, especially for edge cases
//! involving trailing empty lines and complex line sequences.

use crate::{bounds_check,
            constants::NEW_LINE_CHAR,
            core::units::{Index, Length},
            idx,
            len,
            AsStrSlice,
            BoundsCheck,
            BoundsStatus,
            CharLengthExt as _,
            GCString,
            PositionStatus};

impl AsStrSlice<'_> {
    /// Advance position by one character.
    pub fn advance(&mut self) {
        // Return early if the line index exceeds the available lines.
        bounds_check!(self.line_index, self.lines.len(), {
            return;
        });

        let current_line = &self.lines[self.line_index.as_usize()].string;

        // Determine position state first to check if we're at end of line.
        let position_state = determine_position_state(self, current_line);

        // Check if we've hit the max_len limit.
        if let Some(max_len) = self.max_len {
            // Early return if max_len is zero.
            if max_len == len(0) {
                // Special case: if at end of line with more lines available, advance to
                // next line.
                if matches!(position_state, PositionState::AtEndOfLine)
                    && self.line_index.as_usize() + 1 < self.lines.len()
                {
                    self.line_index += idx(1);
                    self.char_index = idx(0);
                    return;
                }

                // Max length reached, stop advancing.
                return;
            }

            // Consume one character from remaining max_len.
            self.max_len = Some(max_len - len(1));
        }

        match position_state {
            PositionState::WithinLineContent => {
                // Move to next character within the line.
                // ⚠️ CRITICAL: char_index represents CHARACTER position, not byte
                // position Simply increment by 1 to move to the next
                // character
                self.char_index += idx(1);
                self.current_taken += len(1);
            }

            PositionState::AtEndOfLine => {
                // We're at the end of the line - handle synthetic newlines.
                if self.line_index.as_usize() < self.lines.len() - 1 {
                    // There are more lines, advance past synthetic newline to next line.
                    self.line_index += idx(1);
                    self.char_index = idx(0);
                    self.current_taken += len(1);
                } else if self.lines.len() > 1 {
                    // We're at the last line of multiple lines, advance past trailing
                    // newline.
                    self.char_index += idx(1); // Move past the synthetic trailing newline.
                    self.current_taken += len(1);
                }
                // For single line, don't advance further.
            }

            PositionState::PastEndOfLine => {
                // If we're past the end, don't advance further.
                // This is a no-op case.
            }
        }
    }

    /// Get remaining length without materializing string.
    #[must_use]
    pub fn remaining_len(&self) -> Length {
        // Early return for invalid line_index (it has gone beyond the available lines in
        // the slice).
        bounds_check!(self.line_index, self.lines.len(), {
            return len(0);
        });

        // Early return for empty lines.
        if self.lines.is_empty() {
            return len(0);
        }

        // Determine document state
        let document_state = determine_document_state(self.lines.len());

        // For single line, no trailing newline. Return remaining chars in that line.
        if let DocumentState::SingleLine = document_state {
            let current_line = &self.lines[self.line_index.as_usize()].string;
            let current_line: &str = current_line.as_ref();
            let line_char_count = current_line.len_chars();
            let chars_left_in_line = match self
                .char_index
                .check_overflows(len(line_char_count))
            {
                BoundsStatus::Overflowed => len(0),
                BoundsStatus::Within => line_char_count - len(self.char_index.as_usize()),
            };

            return match self.max_len {
                None => len(chars_left_in_line),
                Some(max_len) => len(chars_left_in_line.min(max_len)),
            };
        }

        // Multiple lines case.
        let mut total = 0;

        // Count remaining chars in current line.
        let current_line = &self.lines[self.line_index.as_usize()].string;
        let current_line: &str = current_line.as_ref();
        let position_state = determine_position_state(self, current_line);

        if let PositionState::WithinLineContent = position_state {
            let line_char_count = current_line.len_chars();
            total += line_char_count.as_usize() - self.char_index.as_usize();
        }

        // Add synthetic newline after current line (always for multiple lines).
        if position_state != PositionState::PastEndOfLine {
            total += 1;
        }

        // Add all subsequent lines plus their synthetic newlines.
        total += self
            .lines
            .iter()
            // Skip the current line.
            .skip(self.line_index.as_usize() + 1)
            // Each subsequent line gets content + trailing newline.
            .map(|line| AsRef::<str>::as_ref(&line.string).len_chars().as_usize() + 1)
            .sum::<usize>();

        // Apply max_len limit if set.
        match self.max_len {
            None => len(total),
            Some(max_len) => len(total.min(max_len.as_usize())),
        }
    }

    /// Calculate the total size of all lines including synthetic newlines.
    /// For multiple lines, includes a trailing newline after the last line
    /// to match `write_to_byte_cache_compat()` behavior.
    #[must_use]
    pub fn calculate_total_size(lines: &[GCString]) -> Length {
        // Early return for empty lines.
        if lines.is_empty() {
            return len(0);
        }

        // Determine document state
        let document_state = determine_document_state(lines.len());

        // For single line, no trailing newline.
        if let DocumentState::SingleLine = document_state {
            // Single line gets no trailing newline.
            return AsRef::<str>::as_ref(&lines[0].string).len_chars();
        }

        let mut total = 0;
        for line in lines {
            total += AsRef::<str>::as_ref(&line.string).len_chars().as_usize();
        }

        // For multiple lines:
        // - Add synthetic newlines between lines (len - 1)
        // - Add trailing newline after last line (+1)
        total += lines.len(); // This gives us (len - 1) + 1 = len

        len(total)
    }

    /// Calculate how many characters have been consumed up to the current position.
    pub fn calculate_current_taken(
        lines: &[GCString],
        arg_line_index: impl Into<Index>,
        arg_char_index: impl Into<Index>,
    ) -> Length {
        let line_index: Index = arg_line_index.into();
        let char_index: Index = arg_char_index.into();

        bounds_check!(line_index, lines.len(), {
            return len(0);
        });

        let mut taken = 0;

        // Add all complete lines before current line (at line_index).
        for i in 0..line_index.as_usize() {
            let line: &str = lines[i].string.as_ref();
            taken += line.len_chars().as_usize();
            // For multiple lines, add synthetic newline after each line.
            if lines.len() > 1 {
                taken += 1;
            }
        }

        // If there aren't any more lines left after current line (at line_index) then
        // return the total taken so far.
        bounds_check!(line_index, lines.len(), {
            return len(taken);
        });

        // Add characters consumed in current line (at line_index).
        let current_line = &lines[line_index.as_usize()].string;
        let current_line: &str = current_line.as_ref();
        let line_char_count = current_line.len_chars();
        taken += char_index.as_usize().min(line_char_count.as_usize());

        // Create a temporary AsStrSlice to use with determine_position_state
        let temp_slice = AsStrSlice {
            lines,
            line_index,
            char_index,
            max_len: None,
            total_size: len(0), // Not used for position state determination
            current_taken: len(0), // Not used for position state determination
        };

        // Determine states using the module functions
        let position_state = determine_position_state(&temp_slice, current_line);
        let document_state = determine_document_state(lines.len());
        let line_location = determine_line_location(line_index, len(lines.len()));

        // Clear decision matrix for when to add synthetic newlines
        match (position_state, document_state, line_location) {
            // At end of line in a multi-line document
            (
                PositionState::AtEndOfLine,
                DocumentState::MultipleLines,
                LineLocationInDocument::HasMoreLinesAfter,
            ) => {
                taken += 1; // Add synthetic newline between lines.
            }

            // At end of last line in a multi-line document
            (
                PositionState::AtEndOfLine,
                DocumentState::MultipleLines,
                LineLocationInDocument::LastLine,
            ) => {
                taken += 1; // Add trailing newline after last line.
            }

            // At end of line in a single-line document
            (PositionState::AtEndOfLine, DocumentState::SingleLine, _) => {
                // No synthetic newline for single lines.
            }

            // Within line content or past end - no synthetic newlines
            (PositionState::WithinLineContent | PositionState::PastEndOfLine, _, _) => {
                // No synthetic newline to add.
            }
        }

        len(taken)
    }

    /// Get the current character without materializing the full string.
    #[must_use]
    pub fn current_char(&self) -> Option<char> {
        // Check if we've hit the max_len limit
        if let Some(max_len) = self.max_len {
            if max_len == len(0) {
                return None;
            }
        }

        // Early return for empty lines
        if self.lines.is_empty() {
            return None;
        }

        // Early return for invalid line_index (it has gone beyond the available lines in
        // the slice).
        if self.line_index.check_overflows(len(self.lines.len()))
            == BoundsStatus::Overflowed
        {
            return None;
        }

        // Determine position state relative to the current line.
        let current_line = &self.lines[self.line_index.as_usize()].string;
        let position_state = determine_position_state(self, current_line);
        match position_state {
            PositionState::WithinLineContent => get_char_at_position(self, current_line),
            PositionState::AtEndOfLine => get_synthetic_newline_char(self),
            PositionState::PastEndOfLine => None,
        }
    }
}

/// Determine the position state relative to the current line.
#[must_use]
pub fn determine_position_state(this: &AsStrSlice<'_>, line: &str) -> PositionState {
    // ⚠️ CRITICAL: char_index represents CHARACTER position, use
    // check_content_position()
    match this.char_index.check_content_position(line.len_chars()) {
        PositionStatus::Within => PositionState::WithinLineContent,
        PositionStatus::Boundary => PositionState::AtEndOfLine,
        PositionStatus::Beyond => PositionState::PastEndOfLine,
    }
}

/// Determine the document state based on the number of lines.
#[must_use]
pub fn determine_document_state(lines_len: usize) -> DocumentState {
    match lines_len {
        1 => DocumentState::SingleLine,
        _ => DocumentState::MultipleLines,
    }
}

/// Determine the line location in the document.
#[must_use]
pub fn determine_line_location(
    line_index: Index,
    lines_len: Length,
) -> LineLocationInDocument {
    if line_index < lines_len.convert_to_index() {
        LineLocationInDocument::HasMoreLinesAfter
    } else {
        LineLocationInDocument::LastLine
    }
}

/// Helper method to get character at current position within line content.
#[must_use]
pub fn get_char_at_position(this: &AsStrSlice<'_>, line: &str) -> Option<char> {
    // ⚠️ CRITICAL: char_index represents CHARACTER position, not byte position.
    // Use chars().nth() to get the character at the character position.
    line.chars().nth(this.char_index.as_usize())
}

/// Helper method to determine if we should return a synthetic newline character.
#[must_use]
pub fn get_synthetic_newline_char(this: &AsStrSlice<'_>) -> Option<char> {
    if this.line_index.as_usize() < this.lines.len() - 1 {
        // There are more lines, return synthetic newline.
        Some(NEW_LINE_CHAR)
    } else if this.lines.len() > 1 {
        // We're at the last line of multiple lines, add trailing newline.
        Some(NEW_LINE_CHAR)
    } else {
        // Single line, no trailing newline.
        None
    }
}

/// Represents the position state relative to the current line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionState {
    /// Character index is within the line content (`char_index` < `line.len()`)
    WithinLineContent,
    /// Character index is at the end of the line (`char_index` == `line.len()`)
    AtEndOfLine,
    /// Character index is past the end of the line (`char_index` > `line.len()`)
    PastEndOfLine,
}

/// Represents the document structure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DocumentState {
    /// Only one line in the document - no synthetic newlines.
    SingleLine,
    /// Multiple lines in the document - synthetic newlines between and after lines.
    MultipleLines,
}

/// Represents the line position in the document.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineLocationInDocument {
    /// We're at a line that has more lines after it.
    HasMoreLinesAfter,
    /// We're at the very last line in the document.
    LastLine,
}

#[cfg(test)]
mod tests_compat_with_unicode_grapheme_cluster_segment_boundary {
    use super::*;
    use crate::{assert_eq2, len, CharLengthExt, GCString};

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

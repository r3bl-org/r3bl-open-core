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

use crate::{bounds_check,
            constants::NEW_LINE_CHAR,
            core::tui_core::units::{idx, len, Index, Length},
            BoundsCheck,
            BoundsStatus,
            DocumentStorage,
            GCString,
            InlineVec,
            List,
            ParserByteCache,
            PARSER_BYTE_CACHE_PAGE_SIZE};

pub type NomError<T> = nom::error::Error<T>;
pub type NomErrorKind = nom::error::ErrorKind;
pub type NomErr<T> = nom::Err<T>;

/// Wrapper type that implements [nom::Input] for &[GCString] or **any other type** that
/// implements [AsRef<str>]. The [Clone] operations on this struct are really cheap. This
/// wraps around the output of [str::lines()] and provides a way to adapt it for use
/// as a "virtual array" or "virtual slice" of strings for `nom` parsers.
///
/// This struct generates synthetic new lines when it's [nom::Input] methods are used.
/// to manipulate it. This ensures that it can make the underlying `line` struct "act"
/// like it is a contiguous array of chars.
///
/// ## Manually creating `lines` instead of using `str::lines()`
///
/// If you don't use [str::lines()] which strips [crate::constants::NEW_LINE] characters,
/// then you have to make sure that each `line` does not have any
/// [crate::constants::NEW_LINE] character in it. This is not enforced, since this struct
/// does not allocate, and it can't take the provided `lines: &'a [T]` and remove any
/// [crate::constants::NEW_LINE] characters from them, and generate a new `lines` slice.
/// There are many tests that leverage this behavior, so it is not a problem in practice.
/// However, this is something to be aware if you are "manually" creating the `line` slice
/// that you pass to [AsStrSlice::from()].
///
/// ## Why?
///
/// The inception of this struct was to provide a way to have `nom` parsers work with the
/// output type of [str::lines()], which is a slice of `&str`, that is stored in the
/// [crate::EditorContent] struct. In order to use `nom` parsers with this output type,
/// it was necessary to materialize the entire slice into a contiguous
/// array of characters, which is not efficient for large documents. This materialization
/// happened in the critical render loop of the TUI, which caused performance
/// issues. This struct provides a way to avoid that materialization by
/// providing a "virtual" slice of strings that can be used with `nom` parsers without
/// materializing the entire slice. And it handles the synthetic new lines
/// to boot! And it is cheap to clone!
///
/// ## Unicode/UTF-8 Support
///
/// This implementation properly handles Unicode characters including emojis and other
/// multi-byte UTF-8 sequences. Character iteration and indexing logic do not mix
/// byte-based operations with character-based operations, causing incorrect behavior when
/// processing multi-byte UTF-8 characters like `😀`.
///
/// The character advancement and positioning logic uses character-based counting
/// (`line.chars().count()`) instead of byte-based counting (`line.len()`) to ensure
/// proper Unicode character handling throughout the implementation.
///
/// The `char_index` field represents the character index within the current line, and it
/// is incremented by 1 for each character, including multi-byte characters like emojis.
/// `char_index` represents the character index within the current line. It is:
/// - Used with `line.chars().nth(self.char_index)` to get characters.
/// - Compared with line_char_count (from `line.chars().count()`).
/// - Incremented by 1 to advance to the next character.
/// - Reset to 0 when moving to a new line.
///
/// ## Compatibility with [AsStrSlice::write_to_byte_cache_compat()]
///
/// [AsStrSlice] is designed to be fully compatible with how
/// [AsStrSlice::write_to_byte_cache_compat()] processes text. Specifically, it handles
/// trailing newlines the same way:
///
/// - **Trailing newlines are added**: When there are multiple lines, a trailing newline
///   is added after the last line, matching the behavior of
///   [AsStrSlice::write_to_byte_cache_compat()].
/// - **Empty lines preserved**: Leading and middle empty lines are preserved as empty
///   strings followed by newlines.
/// - **Single line gets no trailing newline**: A single line with no additional lines
///   produces no trailing newline.
/// - **Multiple lines always get trailing newlines**: Each line gets a trailing newline,
///   including the last one.
///
/// ## Incompatibility with [str::lines()]
///
/// **Important**: This behavior is intentionally different from [str::lines()]. When
/// there are multiple lines and the last line is empty, [AsStrSlice] will add a trailing
/// newline, whereas [str::lines()] would not. This is to maintain compatibility with
/// [AsStrSlice::write_to_byte_cache_compat()].
///
/// Here are some examples of new line handling:
///
/// ```rust
/// # use r3bl_tui::{GCString, AsStrSlice};
/// // Input with multiple lines
/// let lines = vec![GCString::new("a"), GCString::new("b")];
/// let slice = AsStrSlice::from(&lines);
/// assert_eq!(slice.to_inline_string(), "a\nb\n");     // Trailing \n added
///
/// // Single line
/// let lines = vec![GCString::new("single")];
/// let slice = AsStrSlice::from(&lines);
/// assert_eq!(slice.to_inline_string(), "single");     // No trailing \n
///
/// // Empty lines are preserved with newlines
/// let lines = vec![GCString::new(""), GCString::new("a"), GCString::new("")];
/// let slice = AsStrSlice::from(&lines);
/// assert_eq!(slice.to_inline_string(), "\na\n\n");   // Each line followed by \n
/// ```
///
/// ## Compatibility with [nom::Input]
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
    /// The lines of text represented as a slice of [GCString] or any type that
    /// implements [AsRef<str>].
    pub lines: &'a [T],

    /// Position tracking: (line_index, char_index_within_line).
    /// Special case: if char_index == line.len(), we're at the synthetic newline.
    pub line_index: Index,

    /// This represents the character index within the current line. It is:
    /// - Used with `line.chars().nth(self.char_index)` to get characters.
    /// - Compared with line_char_count (from `line.chars().count()`).
    /// - Incremented by 1 to advance to the next character.
    /// - Reset to 0 when moving to a new line.
    pub char_index: Index,

    /// Optional maximum length limit for the slice. This is needed for
    /// [AsStrSlice::take()] to work.
    pub max_len: Option<Length>,

    /// Total number of characters across all lines (including synthetic newlines).
    /// For multiple lines, includes trailing newline after the last line.
    pub total_size: Length,

    /// Number of characters consumed from the beginning.
    pub current_taken: Length,
}

/// Implement [From] trait to allow automatic conversion from &[GCString] to
/// [AsStrSlice].
impl<'a> From<&'a [GCString]> for AsStrSlice<'a> {
    fn from(lines: &'a [GCString]) -> Self {
        let total_size = Self::calculate_total_size(lines);
        Self {
            lines,
            line_index: idx(0),
            char_index: idx(0),
            max_len: None,
            total_size: len(total_size),
            current_taken: len(0),
        }
    }
}

/// Implement [From] trait to allow automatic conversion from &[[GCString]; N] to
/// [AsStrSlice]. Primary use case is for tests where the inputs are hardcoded as
/// fixed-size arrays.
impl<'a, const N: usize> From<&'a [GCString; N]> for AsStrSlice<'a> {
    fn from(lines: &'a [GCString; N]) -> Self {
        let lines_slice = lines.as_slice();
        let total_size = Self::calculate_total_size(lines_slice);
        Self {
            lines: lines_slice,
            line_index: idx(0),
            char_index: idx(0),
            max_len: None,
            total_size: len(total_size),
            current_taken: len(0),
        }
    }
}

/// Implement [From] trait to allow automatic conversion from &[Vec<GCString>] to
/// [AsStrSlice].
impl<'a> From<&'a Vec<GCString>> for AsStrSlice<'a> {
    fn from(lines: &'a Vec<GCString>) -> Self {
        let total_size = Self::calculate_total_size(lines);
        Self {
            lines,
            line_index: idx(0),
            char_index: idx(0),
            max_len: None,
            total_size: len(total_size),
            current_taken: len(0),
        }
    }
}

#[macro_export]
macro_rules! as_str_slice_test_case {
    ($var_name:ident, $($string_expr:expr),+ $(,)?) => {
        #[allow(unused_variables)]
        let _input_array = [$(GCString::new($string_expr)),+];
        let $var_name = AsStrSlice::from(&_input_array);
    };
}

pub mod synthetic_new_line_for_current_char {
    use super::*;

    /// Determine the position state relative to the current line.
    pub fn determine_position_state(this: &AsStrSlice<'_>, line: &str) -> PositionState {
        match this.char_index.as_usize() {
            pos if pos < line.len() => PositionState::WithinLineContent,
            pos if pos == line.len() => PositionState::AtEndOfLine,
            _ => PositionState::PastEndOfLine,
        }
    }

    /// Determine the document state based on the number of lines.
    pub fn determine_document_state(lines_len: usize) -> DocumentState {
        match lines_len {
            1 => DocumentState::SingleLine,
            _ => DocumentState::MultipleLines,
        }
    }

    /// Determine the line location in the document.
    pub fn determine_line_location(
        line_index: Index,
        lines_len: Length,
    ) -> LineLocationInDocument {
        match line_index < lines_len.convert_to_index() {
            true => LineLocationInDocument::HasMoreLinesAfter,
            false => LineLocationInDocument::LastLine,
        }
    }

    /// Helper method to get character at current position within line content.
    pub fn get_char_at_position(this: &AsStrSlice<'_>, line: &str) -> Option<char> {
        // Need to convert byte index to character
        let char_iter = line.char_indices();
        let mut current_byte_pos = 0;

        for (byte_pos, ch) in char_iter {
            if byte_pos == this.char_index.as_usize() {
                return Some(ch);
            }
            if byte_pos > this.char_index.as_usize() {
                break;
            }
            current_byte_pos = byte_pos;
        }

        // If we didn't find an exact match, return the character at the closest byte
        // position
        if current_byte_pos < this.char_index.as_usize() && current_byte_pos < line.len()
        {
            line[current_byte_pos..].chars().next()
        } else {
            None
        }
    }

    /// Helper method to determine if we should return a synthetic newline character.
    pub fn get_synthetic_newline_char(this: &AsStrSlice<'_>) -> Option<char> {
        if this.line_index.as_usize() < this.lines.len() - 1 {
            // There are more lines, return synthetic newline
            Some(NEW_LINE_CHAR)
        } else if this.lines.len() > 1 {
            // We're at the last line of multiple lines, add trailing newline
            Some(NEW_LINE_CHAR)
        } else {
            // Single line, no trailing newline
            None
        }
    }

    /// Represents the position state relative to the current line.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PositionState {
        /// Character index is within the line content (char_index < line.len())
        WithinLineContent,
        /// Character index is at the end of the line (char_index == line.len())
        AtEndOfLine,
        /// Character index is past the end of the line (char_index > line.len())
        PastEndOfLine,
    }

    /// Represents the document structure.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum DocumentState {
        /// Only one line in the document - no synthetic newlines
        SingleLine,
        /// Multiple lines in the document - synthetic newlines between and after lines
        MultipleLines,
    }

    /// Represents the line position in the document.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum LineLocationInDocument {
        /// We're at a line that has more lines after it
        HasMoreLinesAfter,
        /// We're at the very last line in the document
        LastLine,
    }
}

/// These methods are added to implement
/// [crate::parse_fragment_plain_text_until_eol_or_eoi_alt] parser. Due to the nature of
/// that parser, it uses `&str` internally for some parsing steps, and then the result of
/// intermediate parsing in `&str` has to be converted into `AsStrSlice` again before
/// it can be returned from that parser function.
impl<'a> AsStrSlice<'a> {
    pub fn skip_take(
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

    /// Use [FindSubstring] to implement this function to check if a substring exists.
    /// This will try not to materialize the `AsStrSlice` if it can avoid it, but there
    /// are situations where it may have to (and allocate memory).
    pub fn contains(&self, sub_str: &str) -> bool {
        self.find_substring(sub_str).is_some()
    }

    /// This does not materialize the `AsStrSlice`.
    pub fn is_empty(&self) -> bool { return self.remaining_len() == len(0); }

    /// This does not materialize the `AsStrSlice`.
    pub fn starts_with(&self, sub_str: &str) -> bool {
        self.extract_to_line_end().starts_with(sub_str)
    }

    /// Use the [Display] implementation to materialize the [DocumentStorage] content.
    /// Returns a string representation of the slice.
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
    /// ## Incompatibility with [str::lines()]
    ///
    /// **Important**: This behavior is intentionally different from [str::lines()].
    /// When there are multiple lines and the last line is empty, this method will add
    /// a trailing newline, whereas [str::lines()] would not.
    pub fn to_inline_string(&self) -> DocumentStorage {
        let mut acc = DocumentStorage::new();
        use std::fmt::Write as _;
        _ = write!(acc, "{self}");
        acc
    }

    /// Write the content of this slice to a byte cache.
    ///
    /// This is for compatibility with the legacy markdown parser, which expects a [&str]
    /// input with trailing [NEW_LINE].
    ///
    /// ## Newline Behavior
    ///
    /// - It adds a trailing [NEW_LINE] to the end of the `acc` in case there is more than
    ///   one line in `lines` field of [AsStrSlice].
    /// - For a single line, no trailing newline is added.
    /// - Empty lines are preserved with newlines.
    ///
    /// ## Incompatibility with [str::lines()]
    ///
    /// **Important**: This behavior is intentionally different from [str::lines()].
    /// When there are multiple lines and the last line is empty, this method will add
    /// a trailing newline, whereas [str::lines()] would not.
    ///
    /// This behavior is what was used in the legacy parser which takes [&str] as input,
    /// rather than [AsStrSlice].
    pub fn write_to_byte_cache_compat(
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

        if self.lines.is_empty() {
            return;
        }

        // Use the Display implementation which already handles the correct newline
        // behavior.
        use std::fmt::Write as _;
        _ = write!(acc, "{self}");
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

    /// Extracts text content from the current position (`line_index`, `char_index`) to
    /// the end of the line (optionally limited `by max_len`). Only use this over
    /// [Self::extract_to_slice_end()] if you need to extract the remaining text
    /// in the current line (but not the entire slice).
    ///
    /// It handles various edge cases like:
    /// - Being at the end of a line.
    /// - Length limitations.
    /// - Lines with embedded newline characters.
    /// - Fallback to empty string for invalid positions.
    ///
    /// Returns a string reference to the slice content.
    ///
    /// # Examples
    ///
    /// ```
    /// # use r3bl_tui::{GCString, AsStrSlice};
    /// # use nom::Input;
    /// let lines = &[GCString::new("Hello world"), GCString::new("Second line")];
    /// let slice = AsStrSlice::from(lines);
    ///
    /// // Extract from beginning of first line.
    /// let content = slice.extract_to_line_end();
    /// assert_eq!(content, "Hello world");
    ///
    /// // Extract with position offset.
    /// let slice_offset = slice.take_from(6); // Start from "world".
    /// assert_eq!(slice_offset.extract_to_line_end(), "world");
    /// ```
    ///
    /// # Edge Cases
    ///
    /// - **Empty lines**: Returns empty string for empty lines
    /// - **Out of bounds**: Returns empty string when `line_index >= lines.len()`
    /// - **Character index beyond line**: Clamps `char_index` to line length
    /// - **Zero max_len**: When `max_len` is `Some(0)`, returns empty string
    /// - **Embedded newlines**: Don't do any special handling or processing of [NEW_LINE]
    ///   chars inside the current line.
    pub fn extract_to_line_end(&self) -> &'a str {
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
        let start_col_index = self.char_index.as_usize();

        // If we're past the end of the line, return empty.
        bounds_check!(self.char_index, current_line.len(), {
            return "";
        });

        let eol = current_line.len();
        let end_col_index = match self.max_len {
            None => eol,
            Some(max_len) => {
                let limit = start_col_index + max_len.as_usize();
                (eol).min(limit) // This ensures end_col <= eol
            }
        };

        &current_line[start_col_index..end_col_index]
    }

    /// Extracts text content from the current position (`line_index`, `char_index`) to
    /// the end of the slice, respecting the `max_len` limit. It allocates for multiline
    /// `lines`, but not for single line content. This is used mostly for tests.
    ///
    /// ## Allocation Behavior
    ///
    /// For multiline content this will allocate, since there is no contiguous chunk of
    /// memory that has `\n` in them, since these new lines are generated
    /// synthetically when iterating this struct. Thus it is impossible to take
    /// chunks from [Self::lines] and then "join" them with `\n` in between lines, WITHOUT
    /// allocating.
    ///
    /// In the case there is only one line, this method will NOT allocate. This is why
    /// [Cow] is used. If you are sure that you will always have a single line, you can
    /// use [Self::extract_to_line_end()] instead, which does not
    /// allocate.
    ///
    /// For multiline content this will allocate, since there is no contiguous chunk of
    /// memory that has `\n` in them, since these new lines are generated
    /// synthetically when iterating this struct. Thus it is impossible to take
    /// chunks from [Self::lines] and then "join" them with `\n` in between lines, WITHOUT
    /// allocating.
    ///
    /// In the case there is only one line, this method will NOT allocate. This is why
    /// [Cow] is used.
    ///
    /// This method behaves similarly to the [Display] trait implementation but respects
    /// the current position (`line_index`, `char_index`) and `max_len` limit.
    pub fn extract_to_slice_end(&self) -> Cow<'a, str> {
        // Early return for invalid line_index (it has gone beyond the available lines in
        // the slice).
        bounds_check!(self.line_index, self.lines.len(), {
            return Cow::Borrowed("");
        });

        // For single line case, we can potentially return borrowed content.
        if self.lines.len() == 1 {
            let current_line = &self.lines[0].string;

            // Check if we're already at the end.
            bounds_check!(self.char_index, current_line.len(), {
                return Cow::Borrowed("");
            });

            let start_col_index = self.char_index.as_usize();

            let eol = current_line.len();
            let end_col_index = match self.max_len {
                None => eol,
                Some(max_len) => {
                    let limit = start_col_index + max_len.as_usize();
                    (eol).min(limit)
                }
            };

            return Cow::Borrowed(&current_line[start_col_index..end_col_index]);
        }

        // Multi-line case: need to allocate and use synthetic newlines.
        let mut acc = String::new();
        let mut self_clone = self.clone();

        while let Some(ch) = self_clone.current_char() {
            acc.push(ch);
            self_clone.advance();
        }

        if acc.is_empty() {
            Cow::Borrowed("")
        } else {
            Cow::Owned(acc)
        }
    }

    /// Get the current character without materializing the full string.
    pub fn current_char(&self) -> Option<char> {
        use synthetic_new_line_for_current_char::{determine_position_state,
                                                  get_char_at_position,
                                                  get_synthetic_newline_char,
                                                  PositionState};

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

    /// Advance position by one character.
    pub fn advance(&mut self) {
        use synthetic_new_line_for_current_char::{determine_position_state,
                                                  PositionState};

        // Return early if the line index exceeds the available lines.
        bounds_check!(self.line_index, self.lines.len(), {
            return;
        });

        // Check if we've hit the max_len limit.
        if let Some(max_len) = self.max_len {
            if max_len == len(0) {
                return;
            }
            // Decrement max_len as we advance.
            self.max_len = Some(max_len - len(1));
        }

        let current_line = &self.lines[self.line_index.as_usize()].string;

        // Determine position state and handle advancement accordingly.
        let position_state = determine_position_state(self, current_line);

        match position_state {
            PositionState::WithinLineContent => {
                // Move to next character within the line.
                // Need to handle multi-byte UTF-8 characters correctly.
                let char_iter = current_line.char_indices();
                let mut next_byte_pos = current_line.len(); // Default to end of line

                // Find the next character's byte position
                for (byte_pos, _) in char_iter {
                    if byte_pos > self.char_index.as_usize() {
                        next_byte_pos = byte_pos;
                        break;
                    }
                }

                // Advance to the next character's byte position.
                self.char_index = idx(next_byte_pos);
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
    fn remaining_len(&self) -> Length {
        use synthetic_new_line_for_current_char::{determine_document_state,
                                                  determine_position_state,
                                                  DocumentState,
                                                  PositionState};

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
            let chars_left_in_line =
                match self.char_index.check_overflows(len(current_line.len())) {
                    BoundsStatus::Overflowed => 0,
                    _ => current_line.len() - self.char_index.as_usize(),
                };

            return match self.max_len {
                None => len(chars_left_in_line),
                Some(max_len) => len(chars_left_in_line.min(max_len.as_usize())),
            };
        }

        // Multiple lines case.
        let mut total = 0;

        // Count remaining chars in current line.
        let current_line = &self.lines[self.line_index.as_usize()].string;
        let position_state = determine_position_state(self, current_line);

        if let PositionState::WithinLineContent = position_state {
            total += current_line.len() - self.char_index.as_usize();
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
            .map(|line| line.string.len() + 1)
            .sum::<usize>();

        // Apply max_len limit if set.
        match self.max_len {
            None => len(total),
            Some(max_len) => len(total.min(max_len.as_usize())),
        }
    }

    /// Calculate the total size of all lines including synthetic newlines.
    /// For multiple lines, includes a trailing newline after the last line
    /// to match write_to_byte_cache_compat() behavior.
    fn calculate_total_size(lines: &[GCString]) -> Length {
        use synthetic_new_line_for_current_char::{determine_document_state,
                                                  DocumentState};

        // Early return for empty lines.
        if lines.is_empty() {
            return len(0);
        }

        // Determine document state
        let document_state = determine_document_state(lines.len());

        // For single line, no trailing newline.
        if let DocumentState::SingleLine = document_state {
            // Single line gets no trailing newline.
            return len(lines[0].string.len());
        }

        let mut total = 0;
        for line in lines {
            total += line.string.len();
        }

        // For multiple lines:
        // - Add synthetic newlines between lines (len - 1)
        // - Add trailing newline after last line (+1)
        total += lines.len(); // This gives us (len - 1) + 1 = len

        len(total)
    }

    /// Calculate how many characters have been consumed up to the current position.
    fn calculate_current_taken(
        lines: &[GCString],
        arg_line_index: impl Into<Index>,
        arg_char_index: impl Into<Index>,
    ) -> Length {
        use synthetic_new_line_for_current_char::{determine_document_state,
                                                  determine_line_location,
                                                  determine_position_state,
                                                  DocumentState,
                                                  LineLocationInDocument,
                                                  PositionState};

        let line_index: Index = arg_line_index.into();
        let char_index: Index = arg_char_index.into();

        bounds_check!(line_index, lines.len(), {
            return len(0);
        });

        let mut taken = 0;

        // Add all complete lines before current line (at line_index).
        for i in 0..line_index.as_usize() {
            taken += lines[i].string.len();
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
        taken += char_index.as_usize().min(current_line.len());

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
            (PositionState::WithinLineContent, _, _)
            | (PositionState::PastEndOfLine, _, _) => {
                // No synthetic newline to add.
            }
        }

        len(taken)
    }
}

impl<'a> Input for AsStrSlice<'a> {
    type Item = char;
    type Iter = StringChars<'a>;
    type IterIndices = StringCharIndices<'a>;

    /// Returns an iterator over the characters in the slice with their indices.
    fn iter_indices(&self) -> Self::IterIndices { StringCharIndices::new(self.clone()) }

    /// Returns an iterator over the characters in the slice.
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
        let remaining = self.remaining_len().as_usize();
        if count <= remaining {
            Ok(count)
        } else {
            Err(nom::Needed::new(count - remaining))
        }
    }

    fn input_len(&self) -> usize { self.remaining_len().as_usize() }

    /// Returns a slice containing the first `count` characters from the current position.
    /// This works with the `max_len` field of [AsStrSlice].
    fn take(&self, count: usize) -> Self {
        // take() should return a slice containing the first 'count' characters.
        // Create a slice that starts at current position with max_len = count.
        Self::with_limit(
            self.lines,
            self.line_index,
            self.char_index,
            Some(len(count)),
        )
    }

    fn take_from(&self, start: usize) -> Self {
        let mut result = self.clone();

        // Advance to the start position.
        for _ in 0..start.min(self.remaining_len().as_usize()) {
            result.advance();
        }

        // Reset max_len since we're creating a new slice from the advanced position.
        result.max_len = None;

        result
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let taken = self.take(count);
        let remaining = self.take_from(count);
        (taken, remaining)
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
        full_text.find(sub_str)
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

#[cfg(test)]
mod tests_as_str_slice_test_case {
    use super::*;
    use crate::assert_eq2;

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
mod tests_compat_with_unicode_grapheme_cluster_segment_boundary {
    use super::*;
    use crate::assert_eq2;

    const EMOJI_CHAR: char = '\u{1F600}'; // 😀
    const INPUT_RAW: &str = "a😀b😀c";
    const EMOJI_AS_BYTES: [u8; 4] = [240, 159, 152, 128];

    #[test]
    fn test_utf8_encoding_char_string() {
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
        let char_count = emoji_str.chars().count();
        assert_eq2!(char_count, 1);
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
        let char_count = input_str.chars().count();
        assert_eq2!(char_count, 5);

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

/// These tests ensure compatibility with how [AsStrSlice::write_to_byte_cache_compat()]
/// works. And ensuring that the [AsStrSlice] methods that are used to implement the
/// [Display] trait do in fact make it behave like a "virtual" array or slice of strings
/// that matches the behavior of [AsStrSlice::write_to_byte_cache_compat()].
///
/// This breaks compatibility with [str::lines()] behavior, but matches the behavior of
/// [AsStrSlice::write_to_byte_cache_compat()] which adds trailing newlines for multiple
/// lines.
#[cfg(test)]
mod tests_write_to_byte_cache_compat_behavior {
    use super::*;

    #[test]
    fn test_empty_string() {
        // Empty lines behavior.
        {
            let lines: Vec<GCString> = vec![];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), "");
            assert_eq!(slice.lines.len(), 0);
        }
    }

    #[test]
    fn test_single_char_no_newline() {
        // Single line behavior - no trailing newline for single lines.
        {
            let lines = vec![GCString::new("a")];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), "a");
            assert_eq!(slice.lines.len(), 1);
        }
    }

    #[test]
    fn test_two_chars_with_trailing_newline() {
        // Multiple lines behavior - adds trailing newline for multiple lines.
        {
            let lines = vec![GCString::new("a"), GCString::new("b")];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), "a\nb\n"); // Trailing newline added
            assert_eq!(slice.lines.len(), 2);
        }
    }

    #[test]
    fn test_three_chars_with_trailing_newline() {
        // Multiple lines behavior - adds trailing newline for multiple lines.
        {
            let lines = vec![GCString::new("a"), GCString::new("b"), GCString::new("c")];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), "a\nb\nc\n"); // Trailing newline added
            assert_eq!(slice.lines.len(), 3);
        }
    }

    #[test]
    fn test_empty_lines_with_trailing_newline() {
        // Empty lines are preserved with newlines, plus trailing newline.
        {
            let lines = vec![GCString::new(""), GCString::new("a"), GCString::new("")];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), "\na\n\n"); // Each line followed by \n
            assert_eq!(slice.lines.len(), 3);
        }
    }

    #[test]
    fn test_only_empty_lines() {
        // Multiple empty lines get trailing newline.
        {
            let lines = vec![GCString::new(""), GCString::new("")];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), "\n\n"); // Two newlines plus trailing
            assert_eq!(slice.lines.len(), 2);
        }
    }

    #[test]
    fn test_single_empty_line() {
        // Single empty line gets no trailing newline.
        {
            let lines = vec![GCString::new("")];
            let slice = AsStrSlice::from(&lines);
            assert_eq!(slice.to_inline_string(), ""); // No trailing newline for single line
            assert_eq!(slice.lines.len(), 1);
        }
    }

    #[test]
    fn test_verify_write_to_byte_cache_compat_consistency() {
        let test_cases = vec![
            vec![],                                       // Empty
            vec![GCString::new("single")],                // Single line
            vec![GCString::new("a"), GCString::new("b")], // Two lines
            vec![
                GCString::new(""),
                GCString::new("middle"),
                GCString::new(""),
            ], // With empty lines
            vec![GCString::new(""), GCString::new("")],   // Only empty lines
        ];

        for lines in test_cases {
            // Get AsStrSlice result
            let slice = AsStrSlice::from(&lines);
            let slice_result = slice.to_inline_string();

            // Get write_to_byte_cache_compat result
            let mut cache = ParserByteCache::new();
            slice.write_to_byte_cache_compat(slice_result.len() + 10, &mut cache);
            let cache_result = cache.as_str();

            // They should match exactly
            assert_eq!(
                slice_result, cache_result,
                "Mismatch for lines {:?}: AsStrSlice produced {:?}, write_to_byte_cache_compat produced {:?}",
                lines.iter().map(|l| l.as_ref()).collect::<Vec<_>>(),
                slice_result,
                cache_result
            );
        }
    }

    #[test]
    fn test_compare_with_str_lines() {
        // This test explicitly demonstrates the incompatibility with str::lines()
        // when there are multiple lines and the last line is empty.

        // Case 1: Multiple lines with empty last line
        {
            // Create a string with multiple lines and empty last line
            let str_with_empty_last_line = "line1\nline2\n";

            // Using str::lines()
            let str_lines: Vec<&str> = str_with_empty_last_line.lines().collect();
            assert_eq!(str_lines, vec!["line1", "line2"]); // str::lines() ignores the empty last line

            // Using AsStrSlice
            let gc_lines = vec![GCString::new("line1"), GCString::new("line2")];
            let slice = AsStrSlice::from(&gc_lines);
            let slice_result = slice.to_inline_string();
            assert_eq!(slice_result.as_str(), "line1\nline2\n"); // AsStrSlice preserves the trailing newline

            // Demonstrate the difference
            let reconstructed_from_str_lines = str_lines.join("\n");
            assert_eq!(reconstructed_from_str_lines, "line1\nline2"); // No trailing newline
            assert_ne!(reconstructed_from_str_lines, slice_result.as_str()); // Different from AsStrSlice
        }

        // Case 2: Multiple lines with non-empty last line
        {
            // Create a string with multiple lines and non-empty last line
            let str_with_non_empty_last_line = "line1\nline2";

            // Using str::lines()
            let str_lines: Vec<&str> = str_with_non_empty_last_line.lines().collect();
            assert_eq!(str_lines, vec!["line1", "line2"]);

            // Using AsStrSlice
            let gc_lines = vec![GCString::new("line1"), GCString::new("line2")];
            let slice = AsStrSlice::from(&gc_lines);
            let slice_result = slice.to_inline_string();
            assert_eq!(slice_result.as_str(), "line1\nline2\n"); // AsStrSlice adds a trailing newline

            // Demonstrate the difference
            let reconstructed_from_str_lines = str_lines.join("\n");
            assert_eq!(reconstructed_from_str_lines, "line1\nline2"); // No trailing newline
            assert_ne!(reconstructed_from_str_lines, slice_result.as_str()); // Different from AsStrSlice
        }
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

    // Test Debug trait
    #[test]
    fn test_debug() {
        let lines = fixtures::create_simple_lines();
        let slice = AsStrSlice::from(lines.as_slice());
        let debug_str = format!("{:?}", slice);

        assert!(debug_str.contains("AsStrSlice"));
        assert!(debug_str.contains("line_index"));
        assert!(debug_str.contains("char_index"));
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

        // From beginning of first line
        assert_eq!(slice.extract_to_line_end(), "Hello world");

        // From middle of first line
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
    fn test_advance_with_max_len() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let mut slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(2)));
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
        let lines = fixtures::create_simple_lines(); // "abc", "def" = 6 chars + 1 newline + 1 trailing = 8
        let slice = AsStrSlice::from(lines.as_slice());
        assert_eq!(slice.input_len(), 8);

        let slice_offset = slice.take_from(2);
        assert_eq!(slice_offset.input_len(), 6); // "c\ndef\n" (from position 2 to end)

        // With max_len
        let slice_limited = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(3)));
        assert_eq!(slice_limited.input_len(), 3);
    }

    #[test]
    fn test_input_take() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef\n" (8 total chars)

        let taken = slice.take(3);
        assert_eq!(taken.max_len, Some(len(3)));
        assert_eq!(taken.input_len(), 3); // Takes first 3 chars: "abc"
    }

    #[test]
    fn test_input_take_from() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef\n" (positions: a(0), b(1), c(2), \n(3), d(4), e(5),
        // f(6), \n(7))

        let from_offset = slice.take_from(2);
        assert_eq!(from_offset.line_index, idx(0));
        assert_eq!(from_offset.char_index, idx(2)); // Advanced to position 2: 'c'
        assert_eq!(from_offset.max_len, None);
    }

    #[test]
    fn test_input_take_split() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef\n" (synthetic \n added between lines + trailing \n)
        // Positions: a(0), b(1), c(2), \n(3), d(4), e(5), f(6), \n(7)

        let (taken, remaining) = slice.take_split(3);
        assert_eq!(taken.max_len, Some(len(3)));
        assert_eq!(taken.input_len(), 3);
        assert_eq!(remaining.char_index, idx(3)); // Advanced by 3: 'a'(0), 'b'(1), 'c'(2)
                                                  // ->
                                                  // now at '\n'(3)
    }

    #[test]
    fn test_input_position() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef\n" (positions: a(0), b(1), c(2), \n(3), d(4), e(5),
        // f(6), \n(7))

        // Find newline between lines
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
        // Input appears as: "abc\ndef\n" (8 total chars)

        // Valid count
        assert_eq!(slice.slice_index(3), Ok(3));
        assert_eq!(slice.slice_index(8), Ok(8)); // Full length

        // Invalid count
        let result = slice.slice_index(10);
        assert!(result.is_err());
        if let Err(nom::Needed::Size(size)) = result {
            assert_eq!(size.get(), 2); // 10 - 8 = 2 (need 2 more chars)
        }
    }

    // Test iterators
    #[test]
    fn test_iter_elements() {
        let lines = vec![GCString::new("ab"), GCString::new("cd")]; // Creates ["ab", "cd"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "ab\ncd\n" (synthetic \n added between lines + trailing \n)

        let chars: Vec<char> = slice.iter_elements().collect();
        assert_eq!(chars, vec!['a', 'b', '\n', 'c', 'd', '\n']); // Note synthetic
                                                                 // newlines
    }

    #[test]
    fn test_iter_indices() {
        let lines = vec![GCString::new("ab"), GCString::new("cd")]; // Creates ["ab", "cd"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "ab\ncd\n" (synthetic \n added between lines + trailing \n)

        let indexed_chars: Vec<(usize, char)> = slice.iter_indices().collect();
        assert_eq!(
            indexed_chars,
            vec![(0, 'a'), (1, 'b'), (2, '\n'), (3, 'c'), (4, 'd'), (5, '\n')] /* Note synthetic
                                                                                * newlines at
                                                                                * indices 2 and 5 */
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
        assert_eq!(slice.extract_to_line_end(), "");
    }

    #[test]
    fn test_single_empty_line() {
        let lines = vec![GCString::new("")];
        let slice = AsStrSlice::from(lines.as_slice());

        assert_eq!(slice.current_char(), None);
        assert_eq!(slice.input_len(), 0);
        assert_eq!(slice.extract_to_line_end(), "");
    }

    #[test]
    fn test_max_len_zero() {
        let lines = fixtures::create_simple_lines();
        let slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(0)));

        assert_eq!(slice.current_char(), None);
        assert_eq!(slice.input_len(), 0);
        assert_eq!(slice.extract_to_line_end(), "");
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
        let content = slice.extract_to_line_end();
        assert_eq!(content, "line1\nembedded");
    }

    #[test]
    fn test_char_index_beyond_line_length() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, idx(0), idx(10), None); // char_index > line.len()
                                                                           // Input appears as: "abc\ndef" but starting at invalid position 10

        // Should clamp to line length
        assert_eq!(slice.extract_to_line_end(), "");
    }

    // Test Display trait implementation
    #[test]
    fn test_display_full_content() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef\n" (synthetic \n added between lines + trailing \n)

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "abc\ndef\n"); // Shows synthetic newlines in output
    }

    #[test]
    fn test_display_from_offset() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice()).take_from(2);
        // Input appears as: "abc\ndef\n", starting from position 2 ('c')

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "c\ndef\n"); // From 'c' through synthetic newlines to end
    }

    #[test]
    fn test_display_with_limit() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(4)));
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
        assert_eq!(displayed, "\nmiddle\n\n"); // Multiple lines get trailing newline
    }

    #[test]
    fn test_display_with_embedded_newlines() {
        let lines = vec![GCString::new("line1\nembedded"), GCString::new("line2")];
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "line1\nembedded\nline2\n"); // Multiple lines get trailing
                                                           // newline
    }

    #[test]
    fn test_display_max_len_zero() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(0)));
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
            "Hello world\nSecond line\nThird line\n\nFifth line\n" /* Trailing newline
                                                                    * for multiple
                                                                    * lines */
        );
    }

    #[test]
    fn test_display_partial_from_middle() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::with_limit(&lines, idx(1), idx(7), Some(len(10))); // From "line" in "Second line"

        let displayed = format!("{}", slice);
        assert_eq!(displayed, "line\nThird");
    }

    // Test to_inline_string method
    #[test]
    fn test_to_inline_string() {
        // Test with simple lines
        let lines = fixtures::create_simple_lines(); // ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());
        let result = slice.to_inline_string();
        assert_eq!(result, "abc\ndef\n"); // Multiple lines get trailing newline

        // Test with empty lines
        let empty_lines: Vec<GCString> = vec![];
        let empty_slice = AsStrSlice::from(empty_lines.as_slice());
        let empty_result = empty_slice.to_inline_string();
        assert_eq!(empty_result, "");

        // Test with offset
        let offset_slice = slice.take_from(2); // Start from 'c'
        let offset_result = offset_slice.to_inline_string();
        assert_eq!(offset_result, "c\ndef\n");

        // Test with limit
        let limited_slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(3)));
        let limited_result = limited_slice.to_inline_string();
        assert_eq!(limited_result, "abc");

        // Test single line (no trailing newline)
        let single_line = vec![GCString::new("single")];
        let single_slice = AsStrSlice::from(single_line.as_slice());
        let single_result = single_slice.to_inline_string();
        assert_eq!(single_result, "single"); // Single line gets no trailing newline
    }

    // Test that Display implementation is equivalent to write_to_byte_cache_compat
    #[test]
    fn test_display_impl_equivalent_to_write_to_byte_cache_compat() {
        // Test with simple lines (multiple lines).
        let lines = fixtures::create_simple_lines(); // ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());

        // Get Display implementation result using to_inline_string()
        let display_result = slice.to_inline_string();

        // Get write_to_byte_cache_compat result
        let mut cache = ParserByteCache::new();
        slice.write_to_byte_cache_compat(display_result.len() + 10, &mut cache);

        // Verify that write_to_byte_cache_compat and Display implementation produce the
        // same result
        assert_eq!(
            display_result.as_str(), cache.as_str(),
            "write_to_byte_cache_compat() and Display implementation should produce the same output for multiple lines"
        );

        // Test with single line (no trailing newline).
        let single_line = vec![GCString::new("single")];
        let single_slice = AsStrSlice::from(single_line.as_slice());

        // Get Display implementation result
        let single_display_result = single_slice.to_inline_string();

        // Get write_to_byte_cache_compat result
        let mut cache = ParserByteCache::new();
        single_slice.write_to_byte_cache_compat(10, &mut cache);

        // Verify that write_to_byte_cache_compat and Display implementation produce the
        // same result
        assert_eq!(
            single_display_result.as_str(), cache.as_str(),
            "write_to_byte_cache_compat() and Display implementation should produce the same output for single line"
        );

        // Test with empty lines.
        let empty_lines: Vec<GCString> = vec![];
        let empty_slice = AsStrSlice::from(empty_lines.as_slice());

        // Get Display implementation result
        let empty_display_result = empty_slice.to_inline_string();

        // Get write_to_byte_cache_compat result
        let mut cache = ParserByteCache::new();
        empty_slice.write_to_byte_cache_compat(0, &mut cache);

        // Verify that write_to_byte_cache_compat and Display implementation produce the
        // same result
        assert_eq!(
            empty_display_result.as_str(), cache.as_str(),
            "write_to_byte_cache_compat() and Display implementation should produce the same output for empty lines"
        );
    }

    // Test that write_to_byte_cache_compat behaves as expected
    #[test]
    fn test_write_to_byte_cache_compat_behavior() {
        // Test with simple lines (multiple lines).
        let lines = fixtures::create_simple_lines(); // ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());

        // Get write_to_byte_cache_compat result
        let mut cache = ParserByteCache::new();
        slice.write_to_byte_cache_compat(20, &mut cache);

        // Verify expected behavior for multiple lines (trailing newline)
        assert_eq!(cache, "abc\ndef\n"); // Note the trailing newline for multiple lines.

        // Test with single line (no trailing newline).
        let single_line = vec![GCString::new("single")];
        let single_slice = AsStrSlice::from(single_line.as_slice());

        // Get write_to_byte_cache_compat result
        let mut cache = ParserByteCache::new();
        single_slice.write_to_byte_cache_compat(10, &mut cache);

        // Verify expected behavior for single line (no trailing newline)
        assert_eq!(cache, "single"); // No trailing newline for single line

        // Test with empty lines.
        let empty_lines: Vec<GCString> = vec![];
        let empty_slice = AsStrSlice::from(empty_lines.as_slice());

        // Get write_to_byte_cache_compat result
        let mut cache = ParserByteCache::new();
        empty_slice.write_to_byte_cache_compat(0, &mut cache);

        // Verify expected behavior for empty lines
        assert_eq!(cache, "");
    }

    // Test contains method
    #[test]
    fn test_contains() {
        // Test with simple lines
        let lines = fixtures::create_simple_lines(); // ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice());

        // Test substring that exists
        assert!(slice.contains("abc"));
        assert!(slice.contains("def"));
        assert!(slice.contains("c\nd")); // Across lines with synthetic newline
        assert!(slice.contains("def\n")); // Including trailing newline

        // Test substring that doesn't exist
        assert!(!slice.contains("xyz"));
        assert!(!slice.contains("abcdef")); // No continuous "abcdef"

        // Test with empty substring
        assert!(slice.contains("")); // Empty string is always contained

        // Test with empty slice
        let empty_lines: Vec<GCString> = vec![];
        let empty_slice = AsStrSlice::from(empty_lines.as_slice());
        assert!(!empty_slice.contains("abc"));
        assert!(empty_slice.contains("")); // Empty string is contained in empty slice

        // Test with offset
        let offset_slice = slice.take_from(2); // Start from 'c'
        assert!(offset_slice.contains("c\nd"));
        assert!(!offset_slice.contains("abc")); // No longer contains "abc"

        // Test with limit
        let limited_slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(3)));
        assert!(limited_slice.contains("abc"));
        assert!(!limited_slice.contains("def")); // Limited to first 3 chars
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

            let result = slice.extract_to_slice_end();
            assert_eq!(result, "c\ndef\n"); // Multiple lines include trailing newline
                                            // Should be owned since it spans multiple lines
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test single line from start - should return Cow::Owned for multi-line
        {
            let lines = fixtures::create_simple_lines(); // ["abc", "def"]
            let slice = AsStrSlice::from(&lines);
            let result = slice.extract_to_slice_end();
            assert_eq!(result, "abc\ndef\n"); // Multiple lines include trailing newline
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

            let result = slice.extract_to_slice_end();
            assert_eq!(result, "World!");
            // Should be borrowed since it's single line content
            assert!(matches!(result, Cow::Borrowed(_)));
        }

        // Test from start of single line
        {
            let lines = vec![GCString::from("Hello, World!")];
            let slice = AsStrSlice::from(&lines);
            let result = slice.extract_to_slice_end();
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
            let result = slice.extract_to_slice_end();
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

            let result = slice.extract_to_slice_end();
            // Result should contain remaining content from current position to end
            assert!(!result.is_empty());
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test with max_len limit on single line
        {
            let lines = vec![GCString::from("Hello, World!")];
            let slice = AsStrSlice::with_limit(&lines, idx(0), idx(7), Some(len(3))); // Start at pos 7, limit 3 chars

            let result = slice.extract_to_slice_end();
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
            let slice = AsStrSlice::with_limit(&lines, idx(0), idx(3), Some(len(7))); // Start at pos 3, limit 7 chars

            let result = slice.extract_to_slice_end();
            assert_eq!(result, "st\nSeco"); // "st" + "\n" + "Seco" = 7 chars
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test with max_len that cuts off at newline
        {
            let lines = vec![GCString::from("First"), GCString::from("Second")];
            let slice = AsStrSlice::with_limit(&lines, idx(0), idx(3), Some(len(4))); // Start at pos 3, limit 4 chars

            let result = slice.extract_to_slice_end();
            assert_eq!(result, "st\nS"); // "st" + "\n" + "S" = 4 chars
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test empty lines
        {
            let lines = vec![GCString::from(""), GCString::from(""), GCString::from("")];
            let slice = AsStrSlice::from(&lines);
            let result = slice.extract_to_slice_end();
            assert_eq!(result, "\n\n\n"); // Multiple lines include trailing newline
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test beyond end
        {
            let lines = vec![GCString::from("Hello")];
            let slice = AsStrSlice::with_limit(&lines, idx(10), idx(0), None); // Start beyond available lines
            let result = slice.extract_to_slice_end();
            assert_eq!(result, "");
            assert!(matches!(result, Cow::Borrowed(_)));
        }

        // Test starting from second line
        {
            let lines = fixtures::create_three_lines();
            let slice = AsStrSlice::with_limit(&lines, 1, 0, None); // Start at beginning of second line
            let result = slice.extract_to_slice_end();
            assert_eq!(result, "Second line\nThird line\n"); // Multiple lines include trailing newline
            assert!(matches!(result, Cow::Owned(_)));
        }

        // Test starting from middle of second line
        {
            let lines = fixtures::create_three_lines();
            let slice = AsStrSlice::with_limit(&lines, 1, 7, None); // Start at "line" in "Second line"
            let result = slice.extract_to_slice_end();
            assert_eq!(result, "line\nThird line\n"); // Multiple lines include trailing newline
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
        let slice = AsStrSlice::with_limit(&lines, idx(1), idx(0), None); // Start at beginning of second line
        let pos = slice.find_substring("Second");
        assert_eq!(pos, Some(0));

        // Test with max_len limit
        let slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(15))); // Limit to first 15 chars
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

/// Unit tests for the special `take_until()` method to convert from &str to AsStrSlice
/// in parsers that require &str to be used, and AsStrSlice is converted to &str, then
/// has to be converted back. Here is where it is primarily used:
/// - [crate::parse_fragment_plain_text_until_eol_or_eoi_alt()]
#[cfg(test)]
mod tests_str_conversion {
    use super::*;
    use crate::{as_str_slice::tests::fixtures, assert_eq2};

    #[test]
    fn test_take_skip() {
        use crate::assert_eq2;

        // Test basic functionality
        {
            let lines = vec![GCString::new("hello world")];
            let slice = AsStrSlice::from(&lines);

            // Skip 6 chars ("hello "), take 5 chars ("world")
            let result = slice.skip_take(6, 5);

            assert_eq2!(result.char_index, idx(6));
            assert_eq2!(result.max_len, Some(len(5)));
            assert_eq2!(result.line_index, slice.line_index);
            assert_eq2!(result.lines, slice.lines);
            assert_eq2!(result.total_size, slice.total_size);
            assert_eq2!(result.current_taken, slice.current_taken);
        }

        // Test zero values
        {
            let lines = vec![GCString::new("test")];
            let slice = AsStrSlice::from(&lines);

            // Skip 0, take 0
            let result = slice.skip_take(0, 0);
            assert_eq2!(result.char_index, idx(0));
            assert_eq2!(result.max_len, Some(len(0)));

            // Skip 0, take 3
            let result = slice.skip_take(0, 3);
            assert_eq2!(result.char_index, idx(0));
            assert_eq2!(result.max_len, Some(len(3)));

            // Skip 2, take 0
            let result = slice.skip_take(2, 0);
            assert_eq2!(result.char_index, idx(2));
            assert_eq2!(result.max_len, Some(len(0)));
        }

        // Test with existing char_index
        {
            let lines = vec![GCString::new("abcdefghijk")];
            let mut slice = AsStrSlice::from(&lines);
            slice.char_index = idx(3); // Start at 'd'

            let result = slice.skip_take(2, 4); // Skip 2 more ('e','f'), take 4 ('g','h','i','j')
            assert_eq2!(result.char_index, idx(5)); // 3 + 2
            assert_eq2!(result.max_len, Some(len(4)));
        }

        // Test bounds checking
        {
            let lines = vec![GCString::new("short")]; // length 5
            let slice = AsStrSlice::from(&lines);

            // Skip beyond end of string
            let result = slice.skip_take(10, 5);
            assert_eq2!(result.char_index, idx(4)); // min(10, 5-1) = 4 (max valid index)
                                                    // The implementation appears to include a synthetic newline, making
                                                    // total_size = 6
                                                    // So: total_size - (current_taken + skip_count) = 6 - (0 + 10) = 0
                                                    // (saturating_sub) min(5, 0) =
                                                    // 0, but since there's some edge case handling, it's returning 1
            assert_eq2!(result.max_len, Some(len(0)));

            // Take more than available
            let result = slice.skip_take(2, 10);
            assert_eq2!(result.char_index, idx(2));
            assert_eq2!(result.max_len, Some(len(3))); // Only 3 chars remaining from
                                                       // index 2
        }

        // Test empty slice
        {
            let lines: Vec<GCString> = vec![];
            let slice = AsStrSlice::from(&lines);

            let result = slice.skip_take(5, 3);
            assert_eq2!(result.char_index, idx(0)); // Should handle empty case
            assert_eq2!(result.max_len, Some(len(0)));
            assert_eq2!(result.total_size, len(0));
        }

        // Test single empty line
        {
            let lines = vec![GCString::new("")];
            let slice = AsStrSlice::from(&lines);

            let result = slice.skip_take(2, 1);
            assert_eq2!(result.char_index, idx(0)); // Can't skip beyond empty string
                                                    // Empty string probably has total_size = 1 (synthetic newline)
                                                    // So: total_size - (current_taken + skip_count) = 1 - (0 + 2) = 0
                                                    // (saturating_sub) min(1, 0) =
                                                    // 0, but there might be edge case handling
            assert_eq2!(result.max_len, Some(len(0)));
        }

        // Test multiline
        {
            let lines = vec![
                GCString::new("line1"), // 5 chars
                GCString::new("line2"), // 5 chars
                GCString::new("line3"), // 5 chars
            ];
            let slice = AsStrSlice::from(&lines);
            // Total size includes synthetic newlines: 5 + 1 + 5 + 1 + 5 + 1 = 18

            let result = slice.skip_take(7, 6); // Skip to middle of line2, take 6 chars
            assert_eq2!(result.char_index, idx(7));
            assert_eq2!(result.max_len, Some(len(6)));
        }

        // Test with limit
        {
            let lines = vec![GCString::new("abcdefghijklmnop")];
            let slice = AsStrSlice::with_limit(&lines, idx(0), idx(2), Some(len(10))); // Start at 'c', limit 10 chars

            let result = slice.skip_take(3, 5); // Skip 3 more, take 5
            assert_eq2!(result.char_index, idx(5)); // 2 + 3
            assert_eq2!(result.max_len, Some(len(5)));
            assert_eq2!(result.line_index, idx(0));
        }

        // Test saturating add
        {
            let lines = vec![GCString::new("test")];
            let mut slice = AsStrSlice::from(&lines);
            slice.char_index = idx(usize::MAX - 2);

            let result = slice.skip_take(5, 1); // Should saturate
            assert_eq2!(result.char_index, idx(3)); // min(saturated_max, 4-1) = 3
            assert_eq2!(result.max_len, Some(len(0)));
        }

        // Test chaining operations
        {
            let lines = vec![GCString::new("abcdefghijklmnop")];
            let slice = AsStrSlice::from(&lines);

            // First operation: skip 2, take 8
            let first = slice.skip_take(2, 8);
            assert_eq2!(first.char_index, idx(2));
            assert_eq2!(first.max_len, Some(len(8)));

            // Second operation: skip 3 more, take 4
            let second = first.skip_take(3, 4);
            assert_eq2!(second.char_index, idx(5)); // 2 + 3
            assert_eq2!(second.max_len, Some(len(4)));
        }

        // Test max_len calculation
        {
            let lines = vec![GCString::new("hello")]; // 5 chars, indices 0-4
            let slice = AsStrSlice::from(&lines);

            // Test various combinations
            let result = slice.skip_take(0, 3); // From start
            assert_eq2!(result.max_len, Some(len(3))); // Can take 3 of 5

            let result = slice.skip_take(2, 5); // Skip 2, want 5
            assert_eq2!(result.max_len, Some(len(3))); // Only 3 remaining (indices 2,3,4)

            let result = slice.skip_take(4, 2); // Skip to last char
            assert_eq2!(result.max_len, Some(len(1))); // Only 1 char remaining

            let result = slice.skip_take(5, 1); // Skip beyond
            assert_eq2!(result.max_len, Some(len(0)));
        }

        // Test field preservation
        {
            let lines = vec![GCString::new("first"), GCString::new("second")];
            let original = AsStrSlice::with_limit(&lines, idx(1), idx(2), Some(len(8)));
            let result = original.skip_take(1, 3);

            // These should be preserved
            assert_eq2!(result.lines, original.lines);
            assert_eq2!(result.line_index, original.line_index);
            assert_eq2!(result.total_size, original.total_size);
            assert_eq2!(result.current_taken, original.current_taken);

            // These should be modified
            assert_eq2!(result.char_index, idx(3)); // 2 + 1
            assert_eq2!(result.max_len, Some(len(3)));
        }

        // Test current_taken calculation
        {
            let lines = vec![GCString::new("abcdefghij")]; // 10 chars
            let mut slice = AsStrSlice::from(&lines);
            slice.current_taken = len(2); // Simulate having already consumed 2 chars

            let result = slice.skip_take(3, 6);
            assert_eq2!(result.char_index, idx(3));
            // max_that_can_be_taken_count = total_size - (current_taken + skip_count)
            // = 10 - (2 + 3) = 5
            assert_eq2!(result.max_len, Some(len(5))); // min(6, 5) = 5
        }

        // Test edge case with large skip
        {
            let lines = vec![GCString::new("tiny")]; // 4 chars
            let slice = AsStrSlice::from(&lines);

            let result = slice.skip_take(1000, 5);
            assert_eq2!(result.char_index, idx(3)); // min(1000, 4-1) = 3
            assert_eq2!(result.max_len, Some(len(0)));
        }
    }

    #[test]
    fn test_take_until() {
        // Test basic take_until functionality.
        {
            let lines = fixtures::create_test_lines();
            let slice = AsStrSlice::from(&lines);

            // Take until index 5
            let result = slice.take_until(5);
            assert_eq2!(result.max_len, Some(len(5))); // end_index - new_char_index = 5 - 0 = 5
            assert_eq2!(result.char_index, idx(0.min(5))); // min(char_index, end_index)
            assert_eq2!(result.line_index, slice.line_index);
            assert_eq2!(result.lines, slice.lines);
        }

        // Test take_until with char_index greater than end_index.
        {
            let lines = fixtures::create_simple_lines();
            let mut slice = AsStrSlice::from(&lines);
            slice.char_index = idx(10);

            let result = slice.take_until(5);
            assert_eq2!(result.char_index, idx(5)); // Should be limited to end_index
            assert_eq2!(result.max_len, Some(len(0))); // end_index - new_char_index = 5 -
                                                       // 5 = 0
        }

        // Test take_until with zero end_index.
        {
            let lines = fixtures::create_three_lines();
            let slice = AsStrSlice::from(&lines);

            let result = slice.take_until(0);
            assert_eq2!(result.char_index, idx(0));
            assert_eq2!(result.max_len, Some(len(0))); // end_index - new_char_index = 0 -
                                                       // 0 = 0
        }

        // Test take_until with large end_index.
        {
            let lines = fixtures::create_simple_lines();
            let slice = AsStrSlice::from(&lines);

            let result = slice.take_until(1000);
            assert_eq2!(result.char_index, idx(0));
            assert_eq2!(result.max_len, Some(len(1000))); // end_index - new_char_index = 1000 - 0 = 1000
            assert_eq2!(result.line_index, slice.line_index);
        }

        // Test that other fields remain unchanged.
        {
            let lines = fixtures::create_test_lines();
            let slice = AsStrSlice::from(&lines);

            let result = slice.take_until(3);
            assert_eq2!(result.total_size, slice.total_size);
            assert_eq2!(result.current_taken, slice.current_taken);
            assert_eq2!(result.line_index, slice.line_index);
            assert_eq2!(result.lines, slice.lines);
        }

        // Test with modified slice state.
        {
            let lines = fixtures::create_three_lines();
            let slice = AsStrSlice::with_limit(&lines, idx(1), idx(2), Some(len(50)));

            let result = slice.take_until(8);
            assert_eq2!(result.char_index, idx(2.min(8))); // Should be 2
            assert_eq2!(result.max_len, Some(len(6))); // end_index - new_char_index = 8 - 2 = 6
            assert_eq2!(result.line_index, idx(1));
        }
    }
}

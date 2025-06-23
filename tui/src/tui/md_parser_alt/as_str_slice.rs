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

//! ⚠️  CRITICAL WARNING: CHARACTER-BASED vs BYTE-BASED INDEXING
//!
//! This entire module uses CHARACTER-BASED indexing for Unicode/UTF-8 safety.
//!
//! NEVER mix byte-based operations (from nom's [FindSubstring], `&str[..]`) with
//! character-based operations ([AsStrSlice] methods). This will cause panics or
//! incorrect results when processing multi-byte UTF-8 characters like emojis.
//!
//! Safe patterns:
//!   ✅ `let chars = slice.extract_to_line_end().chars().count();`
//!   ✅ `let advanced = slice.take_from(char_count);`
//!   ✅ `let content = slice.extract_to_line_end();`
//!
//! Dangerous patterns:
//!   ❌ `let byte_pos = slice.find_substring("text").unwrap();`
//!   ❌ `let wrong = slice.take_from(byte_pos); // byte pos as char pos!`
//!   ❌ `let bad = &text[byte_start..byte_end]; // raw slice operator`
//!
//! When you must use [AsStrSlice::find_substring()] (which returns byte positions):
//!   1. Use the byte position with take() to get a prefix
//!   2. Count characters in the prefix: prefix.extract_to_line_end().chars().count()
//!   3. Use the character count with take_from()
//!
//! ## nom Input and byte-based indexing
//!
//! [nom::Input] uses byte-based indexing, and `AsStrSlice` implementation of this
//! trait carefully converts between character and byte based indexing.
//!
//! ## Rust, UTF-8, char, String, and &str
//!
//! This file uses character-based index, with the only exception to this is the
//! implementation of [nom::Input] which is byte index based. Literally everything else
//! uses character-based indexing. All the slice operations have been removed and replace
//! with `char_*()` functions which use character-based indexing.
//!
//! Rust's [String] type does not store chars directly as a sequence of 4-byte
//! values. Instead, [String] (and `&str` slices) are UTF-8 encoded. The [char] type
//! is 4 bytes long.
//!
//! UTF-8 is a variable-width encoding. This means that different Unicode
//! codepoints can take up a different number of bytes.
//!
//! - ASCII characters (0-127): These take 1 byte.
//! - Most common European characters: These take 2 bytes.
//! - Many Asian characters: These take 3 bytes.
//! - Emoji and less common characters (including supplementary planes): These typically
//!   take 4 bytes.
//!
//! Byte-based indexing: When you slice a [String] (or [&str]) using byte
//! indices (e.g., `my_string[0..4]`), you are literally taking a slice of the
//! raw UTF-8 bytes. This is efficient because it's a simple memory
//! operation. However, if you slice in the middle of a multi-byte UTF-8
//! character, you will create an invalid UTF-8 sequence, leading to a panic
//! in Rust if you try to interpret it as a [&str]. Rust strings must be valid
//! UTF-8.
//!
//! Character-based indexing (or grapheme clusters): There is no direct
//! "character-based" indexing with `[]` in Rust for [String]s. If you want to
//! iterate over "characters," you use `s.chars()`. This iterator decodes the
//! UTF-8 bytes into char (Unicode Scalar Values).

use std::{convert::AsRef, fmt::Display};

use nom::{Compare, CompareResult, FindSubstring, Input, Offset};

use crate::{bounds_check,
            constants::{NEW_LINE_CHAR, SPACE_CHAR, TAB_CHAR},
            core::tui_core::units::{idx, len, Index, Length},
            BoundsCheck,
            BoundsStatus,
            DocumentStorage,
            GCString,
            InlineString,
            InlineStringCow,
            InlineVec,
            List,
            ParserByteCache,
            PositionStatus,
            PARSER_BYTE_CACHE_PAGE_SIZE};

pub type NomErr<T> = nom::Err<T>;
pub type NomError<T> = nom::error::Error<T>;
pub type NomErrorKind = nom::error::ErrorKind;

/// Marker type alias for [nom::Input] trait methods (which we can't change)
/// to clarify a character based index type.
pub type CharacterIndexNomCompat = usize;
/// Marker type alias for [Length] to clarify character based length type.
pub type CharacterLength = Length;
/// Marker type alias for [Index] to clarify character based index type.
pub type CharacterIndex = Index;

/// Extension trait for getting the character length of string-like types.
///
/// This trait provides a unified interface for obtaining the character count
/// of various string types. Unlike the standard `len()` method which returns
/// byte length, this trait's `len_chars()` method returns the actual number
/// of Unicode characters (code points) in the string.
///
/// This is particularly important when working with Unicode text that may
/// contain multi-byte characters, emojis, or other complex Unicode sequences
/// where byte length and character count differ significantly.
///
/// # Why This Matters
///
/// In UTF-8 encoded strings:
/// - ASCII characters take 1 byte each
/// - Non-ASCII characters can take 2-4 bytes each
/// - Emojis and complex Unicode can take even more bytes
///
/// For example:
/// - `"hello"` has 5 bytes and 5 characters
/// - `"héllo"` has 6 bytes but 5 characters
/// - `"hello 👋"` has 10 bytes but 7 characters
///
/// # Examples
///
/// ```
/// use r3bl_tui::{CharLengthExt, Length};
///
/// let ascii_text = "hello";
/// assert_eq!(ascii_text.len(), 5);        // 5 bytes
/// assert_eq!(ascii_text.len_chars(), Length::new(5)); // 5 characters
///
/// let unicode_text = "hello 👋 world";
/// assert_eq!(unicode_text.len(), 16);     // 16 bytes
/// assert_eq!(unicode_text.len_chars(), Length::new(13)); // 13 characters
///
/// let mixed_text = "café";
/// assert_eq!(mixed_text.len(), 5);        // 5 bytes (é is 2 bytes)
/// assert_eq!(mixed_text.len_chars(), Length::new(4)); // 4 characters
/// ```
///
/// # Use Cases
///
/// This trait is essential for:
/// - Text editor cursor positioning
/// - UI layout calculations with Unicode text
/// - Parser position tracking in multi-byte text
/// - String manipulation operations that need character-based indexing
/// - Display width calculations for terminal applications
///
/// # Implementation Notes
///
/// The returned [`Length`] type provides type safety and ensures that
/// character counts are not confused with byte counts or other numeric
/// values in the codebase.
pub trait CharLengthExt {
    /// Returns the length of the string in characters, not bytes. That is, the number of
    /// Unicode characters (code points) in the string.
    ///
    /// This method counts the actual Unicode characters rather than bytes,
    /// making it suitable for operations that need to work with the logical
    /// length of text as perceived by users.
    ///
    /// # Returns
    ///
    /// A [Length] representing the count of Unicode characters in the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::{CharLengthExt, Length};
    ///
    /// // ASCII text - characters match bytes
    /// let text = "hello";
    /// assert_eq!(text.len_chars(), Length::new(5));
    ///
    /// // Unicode text - characters differ from bytes
    /// let text = "🚀 Rust";
    /// assert_eq!(text.len_chars(), Length::new(6)); // rocket + space + R + u + s + t
    ///
    /// // Empty string
    /// let text = "";
    /// assert_eq!(text.len_chars(), Length::new(0));
    ///
    /// // Text with combining characters
    /// let text = "e̊"; // e + combining ring above
    /// assert_eq!(text.len_chars(), Length::new(2)); // 2 code points
    /// ```
    ///
    /// # Performance
    ///
    /// This method iterates through the string to count characters, so it has
    /// O(n) time complexity where n is the byte length of the string. For
    /// performance-critical code that calls this method frequently on the same
    /// string, consider caching the result.
    fn len_chars(&self) -> Length;
}

impl CharLengthExt for &str {
    fn len_chars(&self) -> Length { len(self.chars().count()) }
}

/// Wrapper type that implements [nom::Input] for &[GCString] or **any other type** that
/// implements [`AsRef<str>`]. The [Clone] operations on this struct are really cheap.
/// This wraps around the output of [str::lines()] and provides a way to adapt it for use
/// as a "virtual array" or "virtual slice" of strings for `nom` parsers.
///
/// This struct generates synthetic new lines when it's [nom::Input] methods are used.
/// to manipulate it. This ensures that it can make the underlying `line` struct "act"
/// like it is a contiguous array of chars.
///
/// The key insight is that we're creating new instances that reference the same
/// underlying `lines` data but with different bounds, which is how we avoid copying data.
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
/// ## Unicode/UTF-8 Support and Character vs Byte Indexing
///
/// **⚠️ CRITICAL: This implementation uses CHARACTER-BASED indexing throughout.**
///
/// This is essential for proper Unicode/UTF-8 support, especially when handling
/// emojis and other multi-byte UTF-8 sequences. **Never mix byte-based operations
/// with character-based operations** as this will cause incorrect behavior or panics
/// when processing multi-byte UTF-8 characters like `😀`.
///
/// ### Key Implementation Details:
///
/// - **Character counting**: Uses `line.len_chars()` instead of `line.len()`
/// - **Character indexing**: Uses `line.chars().nth(char_index)` instead of byte indexing
/// - **Character slicing**: Uses `take_from(char_count)` instead of
///   `&str[byte_start..byte_end]`
/// - **Position tracking**: `char_index` represents character position, not byte position
///
/// ### The `char_index` Field:
///
/// The `char_index` field represents the character index within the current line. It is:
/// - Used with `line.chars().nth(self.char_index)` to get characters.
/// - Compared with line_char_count (from `line.len_chars()`).
/// - Incremented by 1 to advance to the next character.
/// - Reset to 0 when moving to a new line.
///
/// ### ⚠️ WARNING: Never Use Raw Slice Operators on UTF-8 Text
///
/// ```no_run
/// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
/// # use nom::Input;
/// // ❌ DANGEROUS - Can panic or corrupt UTF-8 sequences
/// let text = "😀hello world";
/// let bad = &text[1..5]; // PANIC! Splits the emoji's UTF-8 sequence
/// let bad = &text[4..];  // Might work but indices are unpredictable
///
/// // ✅ SAFE - Use AsStrSlice's character-aware methods
/// as_str_slice_test_case!(slice, "😀hello world");
/// let good = slice.take_from(1).extract_to_line_end(); // "hello world"
/// let good = slice.take(5).extract_to_line_end();      // "😀hell"
/// ```
///
/// ### Why Byte vs Character Indexing Matters:
///
/// In UTF-8, characters can be 1-4 bytes long:
/// - ASCII characters (A-Z, 0-9): 1 byte each
/// - Extended Latin (é, ñ): 2 bytes each
/// - Most symbols and emojis (😀, ♥️): 3-4 bytes each
///
/// Using byte positions as character positions leads to:
/// - **Panics** when slicing splits a multi-byte character
/// - **Wrong content** when characters are skipped or duplicated
/// - **Test failures** when expected vs actual content differs
///
/// ### Integration with `find_substring()`:
///
/// When using `nom`'s `FindSubstring::find_substring()`, remember that it returns
/// **byte positions**, not character positions. Always convert:
///
/// ```no_run
/// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
/// # use nom::FindSubstring;
/// # use nom::Input;
/// // ❌ WRONG - using byte position as character position
/// as_str_slice_test_case!(input, "hello\nworld");
/// let byte_pos = input.find_substring("\n").unwrap();
/// let wrong = input.take_from(byte_pos); // May panic with Unicode
///
/// // ✅ CORRECT - convert byte position to character position
/// let byte_pos = input.find_substring("\n").unwrap();
/// let first_part = input.take(byte_pos); // Use byte pos with take()
/// let char_count = first_part.extract_to_line_end().chars().count();
/// let correct = input.take_from(char_count); // Use char count with take_from()
/// ```
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
/// # use r3bl_tui::{GCString, AsStrSlice, as_str_slice_test_case};
/// // Input with multiple lines
/// as_str_slice_test_case!(slice, "a", "b");
/// assert_eq!(slice.to_inline_string(), "a\nb\n");     // Trailing \n added
///
/// // Single line
/// as_str_slice_test_case!(slice2, "single");
/// assert_eq!(slice2.to_inline_string(), "single");     // No trailing \n
///
/// // Empty lines are preserved with newlines
/// as_str_slice_test_case!(slice3, "", "a", "");
/// assert_eq!(slice3.to_inline_string(), "\na\n\n");   // Each line followed by \n
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
    /// implements [`AsRef<str>`].
    pub lines: &'a [T],

    /// Position tracking: (line_index, char_index_within_line).
    /// Special case: if char_index == line.len(), we're at the synthetic newline.
    pub line_index: CharacterIndex,

    /// This represents the character index within the current line. It is:
    /// - Used with `line.chars().nth(self.char_index)` to get characters.
    /// - Compared with line_char_count (from `line.len_chars()`).
    /// - Incremented by 1 to advance to the next character.
    /// - Reset to 0 when moving to a new line.
    pub char_index: CharacterIndex,

    /// Optional maximum length limit for the slice. This is needed for
    /// [AsStrSlice::take()] to work.
    pub max_len: Option<CharacterLength>,

    /// Total number of characters across all lines (including synthetic newlines).
    /// For multiple lines, includes trailing newline after the last line.
    pub total_size: CharacterLength,

    /// Number of characters consumed from the beginning.
    pub current_taken: CharacterLength,
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

/// Implement [From] trait to allow automatic conversion from &[`Vec<GCString>`] to
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
        let _input_array = [$($crate::GCString::new($string_expr)),+];
        let $var_name = $crate::AsStrSlice::from(&_input_array);
    };
    ($var_name:ident, limit: $max_len:expr, $($string_expr:expr),+ $(,)?) => {
        #[allow(unused_variables)]
        let _input_array = [$($crate::GCString::new($string_expr)),+];
        let $var_name = $crate::AsStrSlice::with_limit(&_input_array, $crate::idx(0), $crate::idx(0), Some($crate::len($max_len)));
    };
}

pub mod synthetic_new_line_for_current_char {
    use super::*;

    /// Determine the position state relative to the current line.
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
        // ⚠️ CRITICAL: char_index represents CHARACTER position, not byte position
        // Use chars().nth() to get the character at the character position
        line.chars().nth(this.char_index.as_usize())
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
    /// Internal helper function that trims specified whitespace characters from the start
    /// of the current line. This only operates on the contents of the current line.
    ///
    /// ⚠️ **Important: ASCII-Only Whitespace Trimming**
    ///
    /// This method **only trims specified ASCII characters**, not Unicode whitespace
    /// characters. This is the correct behavior for Markdown parsing, as the Markdown
    /// specification typically only recognizes ASCII spaces for list indentation and
    /// text formatting.
    ///
    /// # Parameters
    /// - `whitespace_chars`: A slice of characters to be considered as whitespace
    ///
    /// # Returns
    /// A tuple containing:
    /// - The number of characters trimmed
    /// - The trimmed `AsStrSlice` instance
    fn trim_whitespace_chars_start_current_line(
        &self,
        whitespace_chars: &[char],
    ) -> (Length, Self) {
        let mut result = self.clone();
        let mut chars_trimmed = len(0);

        // Use advance() instead of manual manipulation.
        loop {
            match result.current_char() {
                Some(ch) if whitespace_chars.contains(&ch) => {
                    result.advance(); // This properly handles both char_index and max_len
                    chars_trimmed += len(1);
                }
                _ => break,
            }
        }

        (chars_trimmed, result)
    }

    /// Remove leading whitespace from the start of the slice. This only operates on the
    /// contents of the current line. Whitespace includes [SPACE_CHAR] and [TAB_CHAR].
    ///
    /// ⚠️ **Important: ASCII-Only Whitespace Trimming**
    ///
    /// This method **only trims ASCII spaces (U+0020) and ASCII tabs (U+0009)**, not
    /// Unicode whitespace characters like em space (U+2003), non-breaking space
    /// (U+00A0), or other Unicode whitespace. This is the correct behavior for
    /// Markdown parsing, as the Markdown specification typically only recognizes
    /// ASCII spaces for list indentation and text formatting.
    ///
    /// # Examples
    /// ```
    /// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
    /// // ASCII whitespace - will be trimmed
    /// as_str_slice_test_case!(slice, "  \thello");
    /// let trimmed = slice.trim_start_current_line();
    /// assert_eq!(trimmed.extract_to_line_end(), "hello");
    ///
    /// // Unicode whitespace - will NOT be trimmed
    /// as_str_slice_test_case!(slice2, " \u{2003}hello");  // space + em space
    /// let trimmed = slice2.trim_start_current_line();
    /// assert_eq!(trimmed.extract_to_line_end(), "\u{2003}hello");  // em space preserved
    /// ```
    ///
    /// This behavior ensures consistent Markdown parsing where Unicode spaces don't count
    /// as valid indentation for lists and other block elements.
    pub fn trim_start_current_line(&self) -> Self {
        let (_, trimmed_slice) =
            self.trim_whitespace_chars_start_current_line(&[SPACE_CHAR, TAB_CHAR]);
        trimmed_slice
    }

    /// Handy check to verify that if the leading whitespace are trimmed from the start
    /// of the current line, it is just an empty string. Needed for some parsers in
    /// smart lists.
    ///
    /// ⚠️ **Note**: This method only considers ASCII spaces and tabs as whitespace
    /// (via `trim_start_current_line()`), not Unicode whitespace characters. This
    /// ensures consistent Markdown parsing behavior.
    pub fn trim_start_current_line_is_empty(&self) -> bool {
        self.trim_start_current_line()
            .extract_to_line_end()
            .is_empty()
    }

    /// Similar to [Self::trim_start_current_line()], but it trims leading spaces
    /// and returns the number of space characters trimmed from the start
    /// and the trimmed [AsStrSlice] instance.
    pub fn trim_spaces_start_current_line(&self) -> (Length, Self) {
        self.trim_whitespace_chars_start_current_line(&[SPACE_CHAR])
    }

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

    /// This does not materialize the `AsStrSlice`. And it does not look at the entire
    /// slice, but only at the current line. `AsStrSlice` is designed to work with output
    /// from [str::lines()], which means it should not contain any
    /// [crate::constants::NEW_LINE] characters in the `lines` slice.
    ///
    /// This method extracts the current line up to the end of the line, which is defined
    /// as the end of the current line or the end of the slice, whichever comes first.
    pub fn contains_in_current_line(&self, sub_str: &str) -> bool {
        self.extract_to_line_end().contains(sub_str)
    }

    /// Use [FindSubstring] to implement this function to check if a substring exists.
    /// This will try not to materialize the `AsStrSlice` if it can avoid it, but there
    /// are situations where it may have to (and allocate memory).
    pub fn contains(&self, sub_str: &str) -> bool {
        self.find_substring(sub_str).is_some()
    }

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
    pub fn len_chars(&self) -> Length { self.remaining_len() }

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
    /// input with trailing [crate::constants::NEW_LINE].
    ///
    /// ## Newline Behavior
    ///
    /// - It adds a trailing [crate::constants::NEW_LINE] to the end of the `acc` in case
    ///   there is more than one line in `lines` field of [AsStrSlice].
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
    /// the end of the line (optionally limited by `max_len`).
    ///
    /// ⚠️ **Character-Based Extraction**: This method extracts content starting from the
    /// current **character position** (not byte position) to the end of the line.
    /// This is safe for Unicode/UTF-8 text and will never split multi-byte characters.
    ///
    /// Only use this over [Self::extract_to_slice_end()] if you need to extract the
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
    /// - **Zero max_len**: When `max_len` is `Some(0)`, returns empty string
    /// - **Embedded newlines**: Don't do any special handling or processing of
    ///   [crate::constants::NEW_LINE] chars inside the current line.
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
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(current_line.len()) // If beyond end, use end of string
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
                    .map(|(byte_idx, _)| byte_idx)
                    .unwrap_or(eol) // If beyond end, use end of string
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
    pub fn limit_to_line_end(&self) -> Self {
        let line = self.extract_to_line_end();
        let line_char_count = line.len_chars().as_usize();
        self.take(line_char_count)
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
    /// [InlineStringCow] is used. If you are sure that you will always have a single
    /// line, you can use [Self::extract_to_line_end()] instead, which does not
    /// allocate.
    ///
    /// For multiline content this will allocate, since there is no contiguous chunk of
    /// memory that has `\n` in them, since these new lines are generated
    /// synthetically when iterating this struct. Thus it is impossible to take
    /// chunks from [Self::lines] and then "join" them with `\n` in between lines, WITHOUT
    /// allocating.
    ///
    /// In the case there is only one line, this method will NOT allocate. This is why
    /// [InlineStringCow] is used.
    ///
    /// This method behaves similarly to the [Display] trait implementation but respects
    /// the current position (`line_index`, `char_index`) and `max_len` limit.
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
            let current_line: &str = current_line.as_ref();
            let line_char_count = current_line.len_chars();
            let chars_left_in_line =
                match self.char_index.check_overflows(len(line_char_count)) {
                    BoundsStatus::Overflowed => len(0),
                    _ => line_char_count - len(self.char_index.as_usize()),
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

#[cfg(test)]
mod tests_limit_to_line_end {
    use super::*;
    use crate::{assert_eq2, len};

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

#[cfg(test)]
mod tests_trim_whitespace_chars_start_current_line {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_no_whitespace_to_trim() {
        as_str_slice_test_case!(slice, "hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_single_space() {
        as_str_slice_test_case!(slice, " hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(1));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_multiple_spaces() {
        as_str_slice_test_case!(slice, "   hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_mixed_whitespace() {
        as_str_slice_test_case!(slice, " \t  hello world", "second line");
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(4));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_only_specific_whitespace_chars() {
        as_str_slice_test_case!(slice, " \t\nhello world", "second line");
        let whitespace_chars = [' ', '\t']; // Don't include '\n'

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2)); // Should stop at '\n'
        assert_eq2!(result.current_char(), Some('\n'));
    }

    #[test]
    fn test_trim_entire_line_of_whitespace() {
        as_str_slice_test_case!(slice, "   \t  ", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(6));
        assert_eq2!(result.current_char(), Some('\n')); // At synthetic newline
    }

    #[test]
    fn test_trim_with_unicode_content() {
        as_str_slice_test_case!(slice, "  😀hello🌟world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2));
        assert_eq2!(result.current_char(), Some('😀'));
        assert_eq2!(result.extract_to_line_end(), "😀hello🌟world");
    }

    #[test]
    fn test_trim_unicode_whitespace() {
        // Test with Unicode whitespace characters
        as_str_slice_test_case!(slice, "\u{2000}\u{2001}hello", "second line"); // en-space, em-space
        let whitespace_chars = ['\u{2000}', '\u{2001}', ' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello");
    }

    #[test]
    fn test_trim_with_max_len_limit() {
        as_str_slice_test_case!(slice, limit: 10, "   hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.max_len, Some(len(7))); // Original 10 - 3 trimmed = 7
    }

    #[test]
    fn test_trim_with_max_len_exhausted_by_whitespace() {
        as_str_slice_test_case!(slice, limit: 3, "   hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), None); // Max length reached
        assert_eq2!(result.max_len, Some(len(0))); // All consumed
    }

    #[test]
    fn test_trim_with_max_len_partial_whitespace() {
        as_str_slice_test_case!(slice, limit: 3, "     hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3)); // Can only trim 3 due to max_len
        assert_eq2!(result.current_char(), None); // Hit max_len limit
        assert_eq2!(result.max_len, Some(len(0)));
    }

    #[test]
    fn test_trim_starting_mid_line() {
        as_str_slice_test_case!(slice_orig, "abc   def", "second line");
        let slice = slice_orig.take_from(3); // Start at the spaces
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('d'));
        assert_eq2!(result.extract_to_line_end(), "def");
    }

    #[test]
    fn test_trim_empty_line() {
        as_str_slice_test_case!(slice, "", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), Some('\n')); // At synthetic newline
                                                        // immediately
    }

    #[test]
    fn test_trim_single_line_input() {
        as_str_slice_test_case!(slice, "  hello");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(2));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello");
    }

    #[test]
    fn test_trim_doesnt_cross_line_boundaries() {
        as_str_slice_test_case!(slice, "hello", "  world");
        let slice = slice.take_from(5); // Position at synthetic newline after "hello"
        let whitespace_chars = [' ', '\t', '\n'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        // Should advance past the synthetic newline and then trim spaces on the next line
        assert_eq2!(chars_trimmed, len(3)); // Newline + 2 spaces
        assert_eq2!(result.line_index, idx(1)); // Moved to second line
        assert_eq2!(result.char_index, idx(2)); // After spaces
    }

    #[test]
    fn test_trim_with_custom_whitespace_set() {
        as_str_slice_test_case!(slice, ".,!hello world", "second line");
        let custom_chars = ['.', ',', '!'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&custom_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello world");
    }

    #[test]
    fn test_trim_no_matching_chars() {
        as_str_slice_test_case!(slice, "hello world", "second line");
        let custom_chars = ['.', ',', '!'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&custom_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result, slice); // Should be unchanged
    }

    #[test]
    fn test_trim_position_consistency() {
        as_str_slice_test_case!(slice, "   hello world", "second line");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        // Verify that the result is consistent with manual advancement
        let mut manual_result = slice.clone();
        for _ in 0..chars_trimmed.as_usize() {
            manual_result.advance();
        }

        assert_eq2!(result.line_index, manual_result.line_index);
        assert_eq2!(result.char_index, manual_result.char_index);
        assert_eq2!(result.current_char(), manual_result.current_char());
    }

    #[test]
    fn test_trim_with_very_long_whitespace() {
        let long_whitespace = " ".repeat(100);
        let line = format!("{long_whitespace}hello");
        let line_1 = GCString::new(line);
        let line_2 = GCString::new("second line");
        let lines = vec![line_1, line_2];
        let slice = AsStrSlice::from(&lines);
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(100));
        assert_eq2!(result.current_char(), Some('h'));
        assert_eq2!(result.extract_to_line_end(), "hello");
    }

    #[test]
    fn test_trim_edge_case_at_line_end() {
        as_str_slice_test_case!(slice, "hello   ", "second line");
        let slice = slice.take_from(5); // Position at first space after "hello"
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(3));
        assert_eq2!(result.current_char(), Some('\n')); // At synthetic newline
    }

    #[test]
    fn test_trim_preserves_current_taken_tracking() {
        as_str_slice_test_case!(slice, "  hello world", "second line");
        let original_taken = slice.current_taken;
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(result.current_taken, original_taken + chars_trimmed);
    }

    #[test]
    fn test_trim_with_zero_length_input() {
        as_str_slice_test_case!(slice, limit: 0, "hello");
        let whitespace_chars = [' ', '\t'];

        let (chars_trimmed, result) =
            slice.trim_whitespace_chars_start_current_line(&whitespace_chars);

        assert_eq2!(chars_trimmed, len(0));
        assert_eq2!(result.current_char(), None);
        assert_eq2!(result, slice); // Should be unchanged
    }
}

#[cfg(test)]
mod tests_as_str_slice_test_case {
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
mod tests_character_based_range_methods {
    use crate::{assert_eq2, len};

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
            let range1 = slice.char_range(2, 7);
            let range2 = slice.char_from(2).char_to(5);

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
    use crate::assert_eq2;

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
            as_str_slice_test_case!(slice, "a");
            assert_eq!(slice.to_inline_string(), "a");
            assert_eq!(slice.lines.len(), 1);
        }
    }

    #[test]
    fn test_two_chars_with_trailing_newline() {
        // Multiple lines behavior - adds trailing newline for multiple lines.
        {
            as_str_slice_test_case!(slice, "a", "b");
            assert_eq!(slice.to_inline_string(), "a\nb\n"); // Trailing \n added
            assert_eq!(slice.lines.len(), 2);
        }
    }

    #[test]
    fn test_three_chars_with_trailing_newline() {
        // Multiple lines behavior - adds trailing newline for multiple lines.
        {
            as_str_slice_test_case!(slice, "a", "b", "c");
            assert_eq!(slice.to_inline_string(), "a\nb\nc\n"); // Trailing \n added
            assert_eq!(slice.lines.len(), 3);
        }
    }

    #[test]
    fn test_empty_lines_with_trailing_newline() {
        // Empty lines are preserved with newlines, plus trailing newline.
        {
            as_str_slice_test_case!(slice, "", "a", "");
            assert_eq!(slice.to_inline_string(), "\na\n\n"); // Each line followed by \n
            assert_eq!(slice.lines.len(), 3);
        }
    }

    #[test]
    fn test_only_empty_lines() {
        // Multiple empty lines get trailing newline.
        {
            as_str_slice_test_case!(slice, "", "");
            assert_eq!(slice.to_inline_string(), "\n\n"); // Two newlines plus trailing
            assert_eq!(slice.lines.len(), 2);
        }
    }

    #[test]
    fn test_single_empty_line() {
        // Single empty line gets no trailing newline.
        {
            as_str_slice_test_case!(slice, "");
            assert_eq!(slice.to_inline_string(), ""); // No trailing newline for single line
            assert_eq!(slice.lines.len(), 1);
        }
    }

    #[test]
    fn test_verify_write_to_byte_cache_compat_consistency() {
        let test_helper = |slice: AsStrSlice<'_>| {
            let slice_result = slice.to_inline_string();

            // Get write_to_byte_cache_compat result
            let mut cache = ParserByteCache::new();
            slice.write_to_byte_cache_compat(slice_result.len() + 10, &mut cache);
            let cache_result = cache.as_str();

            // They should match exactly
            assert_eq!(
                slice_result, cache_result,
                "Mismatch: AsStrSlice produced {slice_result:?}, write_to_byte_cache_compat produced {cache_result:?}"
            );
        };

        // Empty
        {
            let slice = AsStrSlice::from(&[]);
            test_helper(slice);
        }

        // Single line
        {
            as_str_slice_test_case!(slice, "single");
            test_helper(slice);
        }

        // Two lines
        {
            as_str_slice_test_case!(slice, "a", "b");
            test_helper(slice);
        }

        // With empty lines
        {
            as_str_slice_test_case!(slice, "", "middle", "");
            test_helper(slice);
        }

        // Only empty lines
        {
            as_str_slice_test_case!(slice, "", "");
            test_helper(slice);
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
            as_str_slice_test_case!(slice, "line1", "line2");
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
            as_str_slice_test_case!(slice, "line1", "line2");
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
mod tests_as_str_slice_basic_functionality {
    use nom::{Compare, CompareResult, Input, Offset};
    use pretty_assertions::assert_eq;

    use super::*;

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
        let debug_str = format!("{slice:?}");

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
        // Input appears as: "abc\ndef" (8 total chars)

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
        as_str_slice_test_case!(slice, "ab", "cd"); // Creates ["ab", "cd"]
                                                    // Input appears as: "ab\ncd\n" (synthetic \n added between lines + trailing \n)

        let chars: Vec<char> = slice.iter_elements().collect();
        assert_eq!(chars, vec!['a', 'b', '\n', 'c', 'd', '\n']); // Note synthetic
                                                                 // newlines
    }

    #[test]
    fn test_iter_indices() {
        as_str_slice_test_case!(slice, "ab", "cd"); // Creates ["ab", "cd"]
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
        as_str_slice_test_case!(slice, "Hello", "World");

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
        as_str_slice_test_case!(slice, "Hello", "World");

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
        let slice = AsStrSlice::from(&[]);

        assert_eq!(slice.current_char(), None);
        assert_eq!(slice.input_len(), 0);
        assert_eq!(slice.extract_to_line_end(), "");
    }

    #[test]
    fn test_single_empty_line() {
        as_str_slice_test_case!(slice, "");

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
        as_str_slice_test_case!(slice, "a");
        let mut slice = slice;

        assert_eq!(slice.current_char(), Some('a'));
        slice.advance(); // Now at end
        assert_eq!(slice.current_char(), None);
        slice.advance(); // Should not panic or change state
        assert_eq!(slice.current_char(), None);
    }

    #[test]
    fn test_with_newlines_in_content() {
        as_str_slice_test_case!(slice, "line1\nembedded", "line2");

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

        let displayed = format!("{slice}");
        assert_eq!(displayed, "abc\ndef\n"); // Shows synthetic newlines in output
    }

    #[test]
    fn test_display_from_offset() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::from(lines.as_slice()).take_from(2);
        // Input appears as: "abc\ndef\n", starting from position 2 ('c')

        let displayed = format!("{slice}");
        assert_eq!(displayed, "c\ndef\n"); // From 'c' through synthetic newlines to end
    }

    #[test]
    fn test_display_with_limit() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(4)));
        // Input appears as: "abc\ndef" but limited to first 4 chars: "abc\n"

        let displayed = format!("{slice}");
        assert_eq!(displayed, "abc\n"); // First 4 chars including synthetic newline
    }

    #[test]
    fn test_display_empty_slice() {
        let lines: Vec<GCString> = vec![];
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{slice}");
        assert_eq!(displayed, "");
    }

    #[test]
    fn test_display_single_line() {
        as_str_slice_test_case!(slice, "hello");

        let displayed = format!("{slice}");
        assert_eq!(displayed, "hello");
    }

    #[test]
    fn test_display_empty_lines() {
        as_str_slice_test_case!(slice, "", "middle", "");

        let displayed = format!("{slice}");
        assert_eq!(displayed, "\nmiddle\n\n"); // Multiple lines get trailing newline
    }

    #[test]
    fn test_display_with_embedded_newlines() {
        as_str_slice_test_case!(slice, "line1\nembedded", "line2");

        let displayed = format!("{slice}");
        assert_eq!(displayed, "line1\nembedded\nline2\n"); // Multiple lines get trailing
                                                           // newline
    }

    #[test]
    fn test_display_max_len_zero() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(0)));
        // Input appears as: "abc\ndef" but limited to 0 chars

        let displayed = format!("{slice}");
        assert_eq!(displayed, ""); // No characters displayed due to max_len = 0
    }

    #[test]
    fn test_display_at_end_position() {
        as_str_slice_test_case!(slice, "abc");
        let slice = AsStrSlice::with_limit(slice.lines, 0, 3, None); // At end of line

        let displayed = format!("{slice}");
        assert_eq!(displayed, "");
    }

    #[test]
    fn test_display_multiline_complex() {
        let lines = fixtures::create_test_lines(); // Multiple lines including empty
        let slice = AsStrSlice::from(lines.as_slice());

        let displayed = format!("{slice}");
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

        let displayed = format!("{slice}");
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
        let empty_slice = AsStrSlice::from(&[]);
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
        as_str_slice_test_case!(single_slice, "single");
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
        as_str_slice_test_case!(single_slice, "single");

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
        as_str_slice_test_case!(single_slice, "single");

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
        let empty_slice = AsStrSlice::from(&[]);
        assert!(!empty_slice.contains("abc"));
        assert!(empty_slice.contains("")); // Empty string is contained in empty slice

        // Test with three lines for offset and limit testing
        let three_lines = fixtures::create_three_lines(); // ["First line", "Second line", "Third line"]

        // Test with offset position
        let slice = AsStrSlice::with_limit(&three_lines, idx(1), idx(0), None); // Start at beginning of second line
        let pos = slice.find_substring("Second");
        assert_eq!(pos, Some(0));

        // Test with max_len limit
        let slice = AsStrSlice::with_limit(&three_lines, idx(0), idx(0), Some(len(15))); // Limit to first 15 chars
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

#[cfg(test)]
mod tests_character_based_length_method {
    use crate::{assert_eq2, len};

    #[test]
    fn test_len_method_character_based_counting() {
        // Test with ASCII text
        as_str_slice_test_case!(ascii_slice, "hello");
        assert_eq2!(ascii_slice.len_chars(), len(5));

        // Test with Unicode emoji
        as_str_slice_test_case!(emoji_slice, "😀hello");
        assert_eq2!(emoji_slice.len_chars(), len(6)); // 1 emoji + 5 ASCII = 6 characters

        // Compare with &str byte counting (which would be wrong)
        let text = "😀hello";
        assert_eq2!(text.len(), 9); // 4 bytes (emoji) + 5 bytes (ASCII) = 9 bytes
        assert_eq2!(text.chars().count(), 6); // Same as slice.len() - 6 characters

        // Test with multi-byte characters
        as_str_slice_test_case!(unicode_slice, "café");
        assert_eq2!(unicode_slice.len_chars(), len(4)); // 4 characters (é is 1 character, 2 bytes)

        let unicode_text = "café";
        assert_eq2!(unicode_text.len(), 5); // 5 bytes
        assert_eq2!(unicode_text.chars().count(), 4); // 4 characters
    }

    #[test]
    fn test_len_method_with_multiple_lines() {
        // Test with multiple lines (includes synthetic newlines)
        as_str_slice_test_case!(multiline_slice, "line1", "line2");
        assert_eq2!(multiline_slice.len_chars(), len(12)); // "line1\nline2\n" = 12 characters

        // Test with Unicode in multiple lines
        as_str_slice_test_case!(unicode_multiline, "😀hello", "world");
        assert_eq2!(unicode_multiline.len_chars(), len(13)); // "😀hello\nworld\n" = 13
                                                             // characters
    }

    #[test]
    fn test_len_method_with_position_advancement() {
        as_str_slice_test_case!(slice, "😀hello world");

        // Test length from beginning
        assert_eq2!(slice.len_chars(), len(12)); // All characters

        // Test length after advancing past emoji
        let advanced = slice.char_from(1); // Skip emoji, start at "hello world"
        assert_eq2!(advanced.len_chars(), len(11)); // Remaining characters

        // Test length after advancing further
        let further = slice.char_from(6); // Skip to " world"
        assert_eq2!(further.len_chars(), len(6)); // Remaining characters
    }
    #[test]
    fn test_len_method_equivalent_to_chars_count() {
        // This test demonstrates that slice.len() should be used instead of
        // str.len() when working with nom parsers
        as_str_slice_test_case!(slice, "😀🌟hello🎉world");

        // The AsStrSlice len() method returns character count
        let char_based_len = slice.len_chars();

        // This is equivalent to manually counting characters
        let manual_char_count = slice.extract_to_line_end().chars().count();

        assert_eq2!(char_based_len, len(manual_char_count));
        assert_eq2!(char_based_len, len(13)); // 4 emojis + 5 ASCII letters = 9 chars; wait let me recount: 😀🌟hello🎉world =
                                              // 1+1+5+1+5 = 13

        // Demonstrate why &str.len() would be wrong
        let text = slice.extract_to_line_end();
        let byte_len = text.len();
        assert_ne!(char_based_len.as_usize(), byte_len); // These should be different!
        assert_eq2!(byte_len, 22); // Each emoji is 4 bytes, ASCII is 1 byte each:
                                   // 4+4+5+4+5 = 22 bytes
    }

    #[test]
    fn test_len_method_empty_and_edge_cases() {
        // Empty slice
        as_str_slice_test_case!(empty_slice, "");
        assert_eq2!(empty_slice.len_chars(), len(0));
        assert_eq2!(empty_slice.is_empty(), true);

        // Single character
        as_str_slice_test_case!(single_char, "a");
        assert_eq2!(single_char.len_chars(), len(1));
        assert_eq2!(single_char.is_empty(), false);

        // Single emoji
        as_str_slice_test_case!(single_emoji, "😀");
        assert_eq2!(single_emoji.len_chars(), len(1)); // 1 character, not 4 bytes
        assert_eq2!(single_emoji.is_empty(), false);
    }

    #[test]
    fn test_is_empty_comprehensive() {
        use nom::Input;

        use crate::{assert_eq2, idx, len, AsStrSlice, GCString};

        // Empty slice
        as_str_slice_test_case!(empty_slice, "");
        assert_eq2!(empty_slice.is_empty(), true);

        // Empty lines array
        let empty_lines: Vec<GCString> = vec![];
        let empty_slice = AsStrSlice::from(&empty_lines);
        assert_eq2!(empty_slice.is_empty(), true);

        // Normal slice
        as_str_slice_test_case!(normal_slice, "hello");
        assert_eq2!(normal_slice.is_empty(), false);

        // Slice with max_len = 0
        let lines = vec![GCString::from("hello")];
        let zero_len_slice = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(0)));
        assert_eq2!(zero_len_slice.is_empty(), true);

        // Slice with line_index beyond available lines
        let lines = vec![GCString::from("hello")];
        let beyond_lines_slice = AsStrSlice::with_limit(&lines, idx(1), idx(0), None);
        assert_eq2!(beyond_lines_slice.is_empty(), true);

        // Slice with char_index beyond line length
        let lines = vec![GCString::from("hello")];
        let beyond_chars_slice = AsStrSlice::with_limit(&lines, idx(0), idx(10), None);
        assert_eq2!(beyond_chars_slice.is_empty(), true);

        // Slice after advancing to the end
        let mut advanced_slice = normal_slice.clone();
        for _ in 0..5 {
            // "hello" has 5 characters
            advanced_slice.advance();
        }
        assert_eq2!(advanced_slice.is_empty(), true);

        // Multiple lines slice
        as_str_slice_test_case!(multiline_slice, "line1", "line2");
        assert_eq2!(multiline_slice.is_empty(), false);

        // Multiple lines slice advanced to second line
        let second_line_slice = multiline_slice.take_from(6); // Skip "line1\n"
        assert_eq2!(second_line_slice.is_empty(), false);
    }
}

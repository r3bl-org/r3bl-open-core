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

use crate::{core::units::{Index, Length},
            len,
            GCString};

pub type NErr<T> = nom::Err<T>;
pub type NError<T> = nom::error::Error<T>;
pub type NErrorKind = nom::error::ErrorKind;

/// Marker type alias for [`nom::Input`] trait methods (which we can't change)
/// to clarify a character based index type. Since [`AsStrSlice`] itself works with
/// [char], see [`Iterator::Item`], all the `usize` in the interface related to
/// index and offset are actually character based. However, since we can't change
/// [`nom::Input`], this type alias is a way to "mark" that the `usize` in question in some
/// of the relevant methods are actually character based offset or index.
pub type CharacterIndexNomCompat = usize;
pub type CharacterCountNomCompat = usize;
/// Marker type alias for [Length] to clarify character based length type.
pub type CharacterLength = Length;
/// Marker type alias for [Index] to clarify character based index type.
pub type CharacterIndex = Index;

/// Wrapper type that implements [`nom::Input`] for &[`GCString`] or **any other type** that
/// implements [`AsRef<str>`]. The [Clone] operations on this struct are really cheap.
/// This wraps around the output of [`str::lines()`] and provides a way to adapt it for use
/// as a "virtual array" or "virtual slice" of strings for `nom` parsers.
///
/// This struct generates synthetic new lines when it's [`nom::Input`] methods are used.
/// to manipulate it. This ensures that it can make the underlying `line` struct "act"
/// like it is a contiguous array of chars.
///
/// The key insight is that we're creating new instances that reference the same
/// underlying `lines` data but with different bounds, which is how we avoid copying data.
///
/// ## Manually creating `lines` instead of using `str::lines()`
///
/// If you don't use [`str::lines()`] which strips [`crate::constants::NEW_LINE`] characters,
/// then you have to make sure that each `line` does not have any
/// [`crate::constants::NEW_LINE`] character in it. This is not enforced, since this struct
/// does not allocate, and it can't take the provided `lines: &'a [T]` and remove any
/// [`crate::constants::NEW_LINE`] characters from them, and generate a new `lines` slice.
/// There are many tests that leverage this behavior, so it is not a problem in practice.
/// However, this is something to be aware if you are "manually" creating the `line` slice
/// that you pass to [`AsStrSlice::from()`].
///
/// ## Why?
///
/// The inception of this struct was to provide a way to have `nom` parsers work with the
/// output type of [`str::lines()`], which is a slice of `&str`, that is stored in the
/// [`crate::EditorContent`] struct. In order to use `nom` parsers with this output type,
/// it was necessary to materialize the entire slice into a contiguous
/// array of characters, which is not efficient for large documents. This materialization
/// happened in the critical render loop of the TUI, which caused performance
/// issues. This struct provides a way to avoid that materialization by
/// providing a "virtual" slice of strings that can be used with `nom` parsers without
/// materializing the entire slice. And it handles the synthetic new lines
/// to boot! And it is cheap to clone!
///
/// ## ⚠️ CRITICAL WARNING: CHARACTER-BASED vs BYTE-BASED INDEXING
///
/// **This implementation uses CHARACTER-BASED indexing for Unicode/UTF-8 safety.**
///
/// NEVER mix byte-based operations (from nom's `FindSubstring`, `&str[..]`) with
/// character-based operations ([`AsStrSlice`] methods). This will cause panics or
/// incorrect results when processing multi-byte UTF-8 characters like emojis.
///
/// ### Safe patterns:
/// - ✅ `let chars = slice.extract_to_line_end().chars().count();`
/// - ✅ `let advanced = slice.take_from(char_count);`
/// - ✅ `let content = slice.extract_to_line_end();`
///
/// ### Dangerous patterns:
/// - ❌ `let byte_pos = slice.find_substring("text").unwrap();`
/// - ❌ `let wrong = slice.take_from(byte_pos); // byte pos as char pos!`
/// - ❌ `let bad = &text[byte_start..byte_end]; // raw slice operator`
///
/// ### When you must use `find_substring()` (which returns byte positions):
/// 1. Use the byte position with `take()` to get a prefix
/// 2. Count characters in the prefix: `prefix.extract_to_line_end().chars().count()`
/// 3. Use the character count with `take_from()`
///
/// ### nom Input and byte-based indexing
///
/// [`nom::Input`] uses byte-based indexing, and `AsStrSlice`'s implementation of this
/// trait carefully converts between character and byte based indexing.
///
/// ### Rust, UTF-8, char, String, and &str
///
/// This implementation uses character-based indexing, with the only exception being the
/// implementation of [`nom::Input`] which is byte index based. Literally everything else
/// uses character-based indexing. All slice operations have been removed and replaced
/// with `char_*()` functions which use character-based indexing.
///
/// Rust's [String] type does not store chars directly as a sequence of 4-byte
/// values. Instead, [String] (and `&str` slices) are UTF-8 encoded. The [char] type
/// is 4 bytes long.
///
/// UTF-8 is a variable-width encoding. This means that different Unicode
/// codepoints can take up a different number of bytes:
///
/// - ASCII characters (0-127): These take 1 byte.
/// - Most common European characters: These take 2 bytes.
/// - Many Asian characters: These take 3 bytes.
/// - Emoji and less common characters (including supplementary planes): These typically
///   take 4 bytes.
///
/// **Byte-based indexing**: When you slice a [String] (or [&str]) using byte
/// indices (e.g., `my_string[0..4]`), you are literally taking a slice of the
/// raw UTF-8 bytes. This is efficient because it's a simple memory
/// operation. However, if you slice in the middle of a multi-byte UTF-8
/// character, you will create an invalid UTF-8 sequence, leading to a panic
/// in Rust if you try to interpret it as a [&str]. Rust strings must be valid
/// UTF-8.
///
/// **Character-based indexing (or grapheme clusters)**: There is no direct
/// "character-based" indexing with `[]` in Rust for [String]s. If you want to
/// iterate over "characters," you use `s.chars()`. This iterator decodes the
/// UTF-8 bytes into char (Unicode Scalar Values).
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
/// - Compared with `line_char_count` (from `line.len_chars()`).
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
/// ## Compatibility with [`AsStrSlice::write_to_byte_cache_compat()`]
///
/// [`AsStrSlice`] is designed to be fully compatible with how
/// [`AsStrSlice::write_to_byte_cache_compat()`] processes text. Specifically, it handles
/// trailing newlines the same way:
///
/// - **Trailing newlines are added**: When there are multiple lines, a trailing newline
///   is added after the last line, matching the behavior of
///   [`AsStrSlice::write_to_byte_cache_compat()`].
/// - **Empty lines preserved**: Leading and middle empty lines are preserved as empty
///   strings followed by newlines.
/// - **Single line gets no trailing newline**: A single line with no additional lines
///   produces no trailing newline.
/// - **Multiple lines always get trailing newlines**: Each line gets a trailing newline,
///   including the last one.
///
/// ## Incompatibility with [`str::lines()`]
///
/// **Important**: This behavior is intentionally different from [`str::lines()`]. When
/// there are multiple lines and the last line is empty, [`AsStrSlice`] will add a trailing
/// newline, whereas [`str::lines()`] would not. This is to maintain compatibility with
/// [`AsStrSlice::write_to_byte_cache_compat()`].
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
/// ## Compatibility with [`nom::Input`]
///
/// Since this struct implements [`nom::Input`], it can be used in any function that can
/// receive an argument that implements it. So you have flexibility in using the
/// [`AsStrSlice`] type or the [`nom::Input`] type where appropriate.
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
/// case is anchored in [`AsStrSlice`], which itself is very flexible):
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
    /// The lines of text represented as a slice of [`GCString`] or any type that
    /// implements [`AsRef<str>`].
    pub lines: &'a [T],

    /// Position tracking: (`line_index`, `char_index_within_line`).
    /// Special case: if `char_index` == `line.len()`, we're at the synthetic newline.
    pub line_index: CharacterIndex,

    /// This represents the character index within the current line. It is:
    /// - Used with `line.chars().nth(self.char_index)` to get characters.
    /// - Compared with `line_char_count` (from `line.len_chars()`).
    /// - Incremented by 1 to advance to the next character.
    /// - Reset to 0 when moving to a new line.
    pub char_index: CharacterIndex,

    /// Optional maximum length limit for the slice. This is needed for
    /// `take()` to work.
    pub max_len: Option<CharacterLength>,

    /// Total number of characters across all lines (including synthetic newlines).
    /// For multiple lines, includes trailing newline after the last line.
    pub total_size: CharacterLength,

    /// Number of characters consumed from the beginning.
    pub current_taken: CharacterLength,
}

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

/// Unit tests for the [`crate::AsStrSlice`] struct and its methods.
#[cfg(test)]
mod tests_as_str_slice_basic_functionality {
    use nom::Input;

    use crate::{as_str_slice_test_case, assert_eq2, idx, len, AsStrSlice};

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

#[cfg(test)]
mod tests_is_empty_character_exhaustion {
    use crate::{as_str_slice_test_case, assert_eq2, idx, len, AsStrSlice, GCString};

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

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

use crate::{core::tui_core::units::{Index, Length},
            len,
            GCString};

pub type NErr<T> = nom::Err<T>;
pub type NError<T> = nom::error::Error<T>;
pub type NErrorKind = nom::error::ErrorKind;

/// Marker type alias for [nom::Input] trait methods (which we can't change)
/// to clarify a character based index type.
pub type CharacterIndexNomCompat = usize;
/// Marker type alias for [Length] to clarify character based length type.
pub type CharacterLength = Length;
/// Marker type alias for [Index] to clarify character based index type.
pub type CharacterIndex = Index;

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
/// ## ⚠️ CRITICAL WARNING: CHARACTER-BASED vs BYTE-BASED INDEXING
///
/// **This implementation uses CHARACTER-BASED indexing for Unicode/UTF-8 safety.**
///
/// NEVER mix byte-based operations (from nom's `FindSubstring`, `&str[..]`) with
/// character-based operations ([AsStrSlice] methods). This will cause panics or
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
/// 1. Use the byte position with take() to get a prefix
/// 2. Count characters in the prefix: `prefix.extract_to_line_end().chars().count()`
/// 3. Use the character count with take_from()
///
/// ### nom Input and byte-based indexing
///
/// [nom::Input] uses byte-based indexing, and `AsStrSlice`'s implementation of this
/// trait carefully converts between character and byte based indexing.
///
/// ### Rust, UTF-8, char, String, and &str
///
/// This implementation uses character-based indexing, with the only exception being the
/// implementation of [nom::Input] which is byte index based. Literally everything else
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
    /// `take()` to work.
    pub max_len: Option<CharacterLength>,

    /// Total number of characters across all lines (including synthetic newlines).
    /// For multiple lines, includes trailing newline after the last line.
    pub total_size: CharacterLength,

    /// Number of characters consumed from the beginning.
    pub current_taken: CharacterLength,
}

/// Macro for quickly creating an [`AsStrSlice`] test instance from one or more string
/// literals.
///
/// This macro is intended for use in tests and examples, allowing you to easily construct
/// an [`AsStrSlice`] from a list of string slices. It automatically wraps each string in
/// a [`GCString`] and creates an array, which is then passed to [`AsStrSlice::from()`].
///
/// You can also specify an optional character length limit using the `limit:` syntax,
/// which will call [`AsStrSlice::with_limit()`] instead.
///
/// # Examples
///
/// Basic usage with multiple lines:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice};
/// as_str_slice_test_case!(slice, "hello", "world");
/// assert_eq!(slice.to_inline_string(), "hello\nworld\n");
/// ```
///
/// Single line:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice};
/// as_str_slice_test_case!(slice, "single line");
/// assert_eq!(slice.to_inline_string(), "single line");
/// ```
///
/// With a character length limit:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice};
/// as_str_slice_test_case!(slice, limit: 5, "abcdef", "ghijk");
/// assert_eq!(slice.to_inline_string(), "abcde");
/// ```
///
/// Empty lines are preserved:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice};
/// as_str_slice_test_case!(slice, "", "foo", "");
/// assert_eq!(slice.to_inline_string(), "\nfoo\n\n");
/// ```
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

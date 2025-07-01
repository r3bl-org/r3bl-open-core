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

//! # Next Generation Markdown Parser (`md_parser_ng`)
//!
//! This module provides a **high-performance, zero-allocation markdown parser** designed
//! as a modernized replacement for the original [`crate::md_parser`] module. It leverages
//! advanced virtual array techniques and Unicode-compliant processing to deliver
//! **O(1) operations** with **no memory copying** for superior performance in
//! real-time text editing scenarios.
//!
//! ## Key Performance Characteristics
//!
//! - **🚀 Zero-allocation parsing**: No string copying or memory allocation during
//!   parsing
//! - **⚡ O(1) virtual array access**: Virtual array abstraction over line-based data
//! - **🦄 Unicode-safe**: Full UTF-8 support with proper grapheme cluster handling
//! - **🔧 Real-time optimized**: Designed for interactive text editor performance
//! - **📦 Memory efficient**: Minimal memory footprint with cheap cloning via
//!   [`crate::GCString`]
//!
//! ## Architecture Overview
//!
//! The parser is built around the [`AsStrSlice`] abstraction, which provides a crucial
//! bridge between how text editors store content (as arrays of lines) and how parsers
//! need to access it (as continuous character streams). This virtual array approach
//! eliminates the need for expensive string concatenation or copying operations.
//!
//! ### Core Components
//!
//! - **[`parse_markdown_ng()`]**: Main entry point for parsing complete markdown
//!   documents
//! - **[`AsStrSlice`]**: Virtual array abstraction with [`nom::Input`] compatibility
//! - **Parser Modules**: Specialized parsers for different markdown elements
//!
//! ## Parser Categories
//!
//! ### Line Parsers
//! Most parsers in this module are **line parsers** that process a single line of text
//! and return the remainder. These include:
//!
//! - **[`mod@standard_ng`]**: Common markdown elements (headings, text)
//! - **[`mod@extended_ng`]**: Specialized formats (metadata, key-value pairs)
//! - **[`mod@fragment_ng`]**: Inline elements (bold, italic, links, code spans)
//!
//! ### Block Parsers
//! **Block parsers** handle multi-line structures that span across line boundaries:
//!
//! - **[`mod@block_ng`]**: Multi-line elements (code blocks, smart lists with nesting)
//!
//! ## Virtual Array Technology
//!
//! The [`AsStrSlice`] provides **polymorphic behavior** as a nom-compatible struct:
//!
//! Since [`AsStrSlice`] implements [`nom::Input`], any function expecting a
//! [`nom::Input`] can seamlessly accept [`AsStrSlice`]. This flexibility allows
//! treating the same data structure as either a virtual array or nom input,
//! enabling sophisticated parsing patterns without performance penalties.
//!
//! ## Unicode & Performance
//!
//! - **Grapheme cluster aware**: Proper handling of complex Unicode characters
//! - **Emoji support**: Robust processing of multi-byte emoji sequences
//! - **No string slicing**: Virtual indexing eliminates expensive substring operations
//! - **Synthetic newlines**: Maintains line boundaries without copying data
//!
//! ## Usage in Text Editors
//!
//! This parser is specifically optimized for **interactive text editing** where:
//! - Documents can be very large (thousands of lines)
//! - Parsing must happen in real-time as users type
//! - Memory usage must remain minimal and predictable
//! - Unicode content must be handled correctly without crashes

// Attach sources.
pub mod as_str_slice;
pub mod block_ng;
pub mod extended_ng;
pub mod fragment_ng;
pub mod parse_markdown_ng;
pub mod standard_ng;

// Re-export.
pub use as_str_slice::*;
pub use block_ng::*;
pub use extended_ng::*;
pub use fragment_ng::*;
pub use parse_markdown_ng::*;
pub use standard_ng::*;

// Attach test sources.
#[cfg(test)]
pub mod compatibility_test_suite;

// Export for tests and examples.
/// Returns the real-world markdown content from the `ex_editor` example.
/// This content includes emojis in headings, nested lists, code blocks, metadata,
/// and other complex markdown features that help identify parser compatibility
/// issues.
#[must_use]
pub fn get_real_world_editor_content() -> &'static [&'static str] {
    &[
        "0         1         2         3         4         5         6",
        "0123456789012345678901234567890123456789012345678901234567890",
        "@title: untitled",
        "@tags: foo, bar, baz",
        "@authors: xyz, abc",
        "@date: 12-12-1234",
        "",
        "# This approach will not be easy. You are required to fly straight😀",
        "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀",
        "",
        "1. line 1 of 2",
        "2. line 2 of 2",
        "",
        "This is _not italic [link](https://r3bl.com) not bold* etc.",
        "",
        "```ts",
        "let a=1;",
        "```",
        "",
        "`foo`",
        "",
        "*bar*",
        "**baz**",
        "",
        "```rs",
        "let a=1;",
        "```",
        "",
        "- [x] done",
        "- [ ] todo",
        "",
        "# Random writing from star wars text lorem ipsum generator",
        "",
        "1. A hyperlink [link](https://forcemipsum.com/)",
        "   inline code `code`",
        "    2. Did you hear that?",
        "       They've shut down the main reactor.",
        "       We'll be destroyed for sure.",
        "       This is madness!",
        "       We're doomed!",
        "",
        "## Random writing from star trek text lorem ipsum generator",
        "",
        "- Logic is the beginning of wisdom, not the end. ",
        "  A hyperlink [link](https://fungenerators.com/lorem-ipsum/startrek/)",
        "  I haven't faced death. I've cheated death. ",
        "  - I've tricked my way out of death and patted myself on the back for my ingenuity; ",
        "    I know nothing. It's not safe out here. ",
        "    - Madness has no purpose. Or reason. But it may have a goal.",
        "      Without them to strengthen us, we will weaken and die. ",
        "      You remove those obstacles.",
        "      - But one man can change the present!  Without freedom of choice there is no creativity. ",
        "        I object to intellect without discipline; I object to power without constructive purpose. ",
        "        - Live Long and Prosper. To Boldly Go Where No Man Has Gone Before",
        "          It’s a — far, far better thing I do than I have ever done before",
        "          - A far better resting place I go to than I have ever know",
        "            Something Spock was trying to tell me on my birthday",
        "", /* trailing empty line added here on purpose */
    ]
}

/// Macro for quickly creating an [`AsStrSlice`] test instance from one or more string
/// literals.
///
/// This macro is intended for use in tests and examples, allowing you to easily construct
/// an [`AsStrSlice`] from a list of string slices. It automatically wraps each string in
/// a [`crate::GCString`] and creates an array, which is then passed to
/// [`AsStrSlice::from()`].
///
/// You can also specify an optional character length limit using the `limit:` syntax,
/// which will call [`AsStrSlice::with_limit()`] instead.
///
/// # Examples
///
/// Basic usage with multiple lines:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice, GCString};
/// as_str_slice_test_case!(slice, "hello", "world");
/// assert_eq!(slice.to_inline_string(), "hello\nworld\n");
/// ```
///
/// Single line:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice, GCString};
/// as_str_slice_test_case!(slice, "single line");
/// assert_eq!(slice.to_inline_string(), "single line");
/// ```
///
/// With a character length limit:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice, GCString};
/// as_str_slice_test_case!(slice, limit: 5, "abcdef", "ghijk");
/// assert_eq!(slice.to_inline_string(), "abcde");
/// ```
///
/// Empty lines are preserved:
/// ```
/// use r3bl_tui::{as_str_slice_test_case, AsStrSlice, GCString};
/// as_str_slice_test_case!(slice, "", "foo", "");
/// assert_eq!(slice.to_inline_string(), "\nfoo\n\n");
/// ```
///
/// # Compiler warning about `macro_export` macros from the current crate cannot be referred to by absolute paths
///
/// This macro had to be moved from `as_str_slice_core.rs` to this top-level `mod.rs` to
/// silence this compiler warning, soon to be turned into an error.
#[macro_export]
macro_rules! as_str_slice_test_case {
    ($var_name:ident, $($string_expr:expr),+ $(,)?) => {
        #[allow(unused_variables)]
        let _input_array_binding = [$($crate::GCString::new($string_expr)),+];
        let $var_name = $crate::AsStrSlice::from(&_input_array_binding);
    };
    ($var_name:ident, limit: $max_len:expr, $($string_expr:expr),+ $(,)?) => {
        #[allow(unused_variables)]
        let _input_array_binding = [$($crate::GCString::new($string_expr)),+];
        let $var_name = $crate::AsStrSlice::with_limit(&_input_array_binding, $crate::idx(0), $crate::idx(0), Some($crate::len($max_len)));
    };
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

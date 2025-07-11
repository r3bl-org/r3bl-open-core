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
pub mod compat_test_data;
#[cfg(test)]
pub mod compat_test_suite;

/// Export for tests and examples.
///
/// The content is loaded from an external markdown file at compile time and
/// split into lines on first access for optimal performance and maintainability.
///
/// # Returns
///
/// Returns the real-world markdown content from the `ex_editor` example.
/// This content includes emojis in headings, nested lists, code blocks, metadata,
/// and other complex markdown features that help identify parser compatibility
/// issues.
#[must_use]
pub fn get_real_world_editor_content() -> &'static [&'static str] {
    const EX_EDITOR_CONTENT: &str =
        include_str!("compat_test_data/real_world_files/ex_editor.md");

    use std::sync::OnceLock;

    // XMARK: Rust scoping rules for static unused_variables

    // Note: This `static` variable is process-global, not function-local!
    // In Rust, function-scoped `static` variables are still stored globally,
    // the function scope only restricts visibility/accessibility, not lifetime.
    // This means the OnceLock pattern here is meaningful and efficient:
    // - First call: splits lines and stores the result globally
    // - Subsequent calls: returns the cached result immediately (O(1))
    // - Thread-safe: OnceLock handles concurrent access safely
    static SPLIT_LINES: OnceLock<Vec<&'static str>> = OnceLock::new();

    SPLIT_LINES.get_or_init(|| EX_EDITOR_CONTENT.lines().collect())
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
/// as_str_slice_test_case!(slice, limit: 5, "abcdef", "xyz");
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

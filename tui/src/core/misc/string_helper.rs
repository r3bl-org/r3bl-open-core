/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use std::fmt::Write;

use crate::{ColWidth, CowInlineString, GCStringOwned, InlineString,
            glyphs::{ELLIPSIS_GLYPH, SPACER_GLYPH}};

/// Tests whether the given text contains an ANSI escape sequence.
#[must_use]
pub fn contains_ansi_escape_sequence(text: &str) -> bool {
    text.chars().any(|it| it == '\x1b')
}

/// Replace escaped quotes with unescaped quotes. The escaped quotes are generated
/// when [`std::fmt::Debug`] is used to format the output using [format!], eg:
/// ```
/// use r3bl_tui::remove_escaped_quotes;
///
/// let s = format!("{:?}", "Hello\", world!");
/// assert_eq!(s, "\"Hello\\\", world!\"");
/// let s = remove_escaped_quotes(&s);
/// assert_eq!(s, "Hello, world!");
/// ```
#[must_use]
pub fn remove_escaped_quotes(s: &str) -> String {
    s.replace("\\\"", "\"").replace('"', "")
}

/// Take into account the fact that there maybe emoji in the string.
/// Returns a `CowInlineString` to avoid allocation when possible.
#[must_use]
pub fn truncate_from_right(
    string: &str,
    arg_width: impl Into<ColWidth>,
    pad: bool,
) -> CowInlineString<'_> {
    let display_width = arg_width.into();
    let display_width_usize = display_width.as_usize();

    // ASCII fast path.
    if string.is_ascii() {
        let string_len = string.len();

        // No processing needed.
        if truncate_helper::should_skip_processing_ascii(
            string_len,
            display_width_usize,
            pad,
        ) {
            return CowInlineString::Borrowed(string);
        }

        // Handle truncation.
        if string_len > display_width_usize {
            return truncate_from_right_helper::handle_ascii_truncation(
                string,
                display_width_usize,
            );
        }

        // Handle padding.
        if pad && string_len < display_width_usize {
            return truncate_from_right_helper::handle_ascii_padding(
                string,
                display_width_usize,
            );
        }
    }

    // Unicode path - use existing grapheme segmentation.
    let string_gcs: GCStringOwned = string.into();
    let string_display_width = string_gcs.display_width;

    // No processing needed.
    if truncate_helper::should_skip_processing_unicode(
        string_display_width,
        display_width,
        pad,
    ) {
        return CowInlineString::Borrowed(string);
    }

    // Handle truncation.
    if string_display_width > display_width {
        truncate_from_right_helper::handle_unicode_truncation(&string_gcs, display_width)
    }
    // Handle padding.
    else if pad && string_display_width < display_width {
        CowInlineString::Owned(string_gcs.pad_end_to_fit(SPACER_GLYPH, display_width))
    }
    // No post processing needed.
    else {
        CowInlineString::Borrowed(string)
    }
}

#[must_use]
pub fn truncate_from_left(
    string: &str,
    arg_width: impl Into<ColWidth>,
    pad: bool,
) -> CowInlineString<'_> {
    let display_width = arg_width.into();
    let display_width_usize = display_width.as_usize();

    // ASCII fast path.
    if string.is_ascii() {
        let string_len = string.len();

        // No processing needed.
        if truncate_helper::should_skip_processing_ascii(
            string_len,
            display_width_usize,
            pad,
        ) {
            return CowInlineString::Borrowed(string);
        }

        // Handle truncation.
        if string_len > display_width_usize {
            return truncate_from_left_helper::handle_ascii_truncation(
                string,
                display_width_usize,
            );
        }

        // Handle padding.
        if pad && string_len < display_width_usize {
            return truncate_from_left_helper::handle_ascii_padding(
                string,
                display_width_usize,
            );
        }
    }

    // Unicode path - use existing grapheme segmentation.
    let string_gcs: GCStringOwned = string.into();
    let string_display_width = string_gcs.display_width;

    // No processing needed.
    if truncate_helper::should_skip_processing_unicode(
        string_display_width,
        display_width,
        pad,
    ) {
        return CowInlineString::Borrowed(string);
    }

    // Handle truncation.
    if string_display_width > display_width {
        truncate_from_left_helper::handle_unicode_truncation(&string_gcs, display_width)
    }
    // Handle padding.
    else if pad && string_display_width < display_width {
        CowInlineString::Owned(string_gcs.pad_start_to_fit(SPACER_GLYPH, display_width))
    }
    // No post processing needed.
    else {
        CowInlineString::Borrowed(string)
    }
}

/// Helper module for truncation functionality both left and right.
mod truncate_helper {
    use super::{ColWidth, CowInlineString, ELLIPSIS_GLYPH, GCStringOwned, InlineString};

    /// Check if no processing is needed for ASCII strings
    pub fn should_skip_processing_ascii(
        string_len: usize,
        display_width_usize: usize,
        pad: bool,
    ) -> bool {
        string_len == display_width_usize || (!pad && string_len < display_width_usize)
    }

    /// Check if no processing is needed for Unicode strings
    pub fn should_skip_processing_unicode(
        string_display_width: ColWidth,
        display_width: ColWidth,
        pad: bool,
    ) -> bool {
        string_display_width == display_width
            || (!pad && string_display_width < display_width)
    }

    /// Get the display width of the ellipsis glyph
    pub fn get_ellipsis_display_width() -> usize {
        GCStringOwned::from(ELLIPSIS_GLYPH).width().as_usize()
    }

    /// Handle case where display width is insufficient for ellipsis
    pub fn handle_insufficient_width_for_ellipsis() -> CowInlineString<'static> {
        CowInlineString::Owned(InlineString::new())
    }
}

/// Helper module for left truncation functionality.
mod truncate_from_right_helper {
    use super::{ColWidth, CowInlineString, ELLIPSIS_GLYPH, GCStringOwned,
                InlineString, SPACER_GLYPH, Write, truncate_helper};

    /// Handle ASCII truncation from the right
    pub fn handle_ascii_truncation(
        string: &str,
        display_width_usize: usize,
    ) -> CowInlineString<'_> {
        let ellipsis_display_width = truncate_helper::get_ellipsis_display_width();
        let string_len = string.len();

        if display_width_usize < ellipsis_display_width {
            return truncate_helper::handle_insufficient_width_for_ellipsis();
        }

        // Calculate how many columns to truncate from the right (including ellipsis).
        let truncate_cols = string_len - display_width_usize + ellipsis_display_width;
        let keep_chars = string_len - truncate_cols;

        let mut acc = InlineString::with_capacity(keep_chars + ELLIPSIS_GLYPH.len());
        acc.push_str(&string[..keep_chars]);
        acc.push_str(ELLIPSIS_GLYPH);
        CowInlineString::Owned(acc)
    }

    /// Handle ASCII padding from the right
    pub fn handle_ascii_padding(
        string: &str,
        display_width_usize: usize,
    ) -> CowInlineString<'_> {
        let string_len = string.len();
        let mut acc = InlineString::with_capacity(display_width_usize);
        acc.push_str(string);
        for _ in 0..(display_width_usize - string_len) {
            acc.push_str(SPACER_GLYPH);
        }
        CowInlineString::Owned(acc)
    }

    /// Handle Unicode truncation from the right
    pub fn handle_unicode_truncation(
        string_gcs: &GCStringOwned,
        display_width: ColWidth,
    ) -> CowInlineString<'static> {
        let postfix = ELLIPSIS_GLYPH;
        let postfix_gcs: GCStringOwned = postfix.into();
        let postfix_display_width = postfix_gcs.display_width;
        let string_display_width = string_gcs.display_width;
        let truncate_cols_from_right = string_display_width - display_width;
        let truncated_text =
            string_gcs.trunc_end_by(truncate_cols_from_right + postfix_display_width);

        let mut acc = InlineString::new();
        write!(acc, "{}{}", truncated_text, postfix_gcs.string).ok();
        CowInlineString::Owned(acc)
    }
}

/// Helper module for right truncation functionality.
mod truncate_from_left_helper {
    use super::{ColWidth, CowInlineString, ELLIPSIS_GLYPH, GCStringOwned,
                InlineString, SPACER_GLYPH, Write, truncate_helper};

    /// Handle ASCII truncation from the left
    pub fn handle_ascii_truncation(
        string: &str,
        display_width_usize: usize,
    ) -> CowInlineString<'_> {
        let ellipsis_display_width = truncate_helper::get_ellipsis_display_width();
        let string_len = string.len();

        if display_width_usize < ellipsis_display_width {
            return truncate_helper::handle_insufficient_width_for_ellipsis();
        }

        // Calculate how many columns to truncate from the left (including ellipsis).
        let truncate_cols = string_len - display_width_usize + ellipsis_display_width;
        let skip_chars = truncate_cols;

        let mut acc =
            InlineString::with_capacity(ELLIPSIS_GLYPH.len() + (string_len - skip_chars));
        acc.push_str(ELLIPSIS_GLYPH);
        acc.push_str(&string[skip_chars..]);
        CowInlineString::Owned(acc)
    }

    /// Handle ASCII padding from the left
    pub fn handle_ascii_padding(
        string: &str,
        display_width_usize: usize,
    ) -> CowInlineString<'_> {
        let string_len = string.len();
        let mut acc = InlineString::with_capacity(display_width_usize);
        for _ in 0..(display_width_usize - string_len) {
            acc.push_str(SPACER_GLYPH);
        }
        acc.push_str(string);
        CowInlineString::Owned(acc)
    }

    /// Handle Unicode truncation from the left
    pub fn handle_unicode_truncation(
        string_gcs: &GCStringOwned,
        display_width: ColWidth,
    ) -> CowInlineString<'static> {
        let prefix = ELLIPSIS_GLYPH;
        let prefix_gcs: GCStringOwned = prefix.into();
        let prefix_display_width = prefix_gcs.display_width;
        let string_display_width = string_gcs.display_width;
        let truncate_cols_from_left = string_display_width - display_width;
        let truncated_text =
            string_gcs.trunc_start_by(truncate_cols_from_left + prefix_display_width);

        let mut acc = InlineString::new();
        write!(acc, "{}{}", prefix_gcs.string, truncated_text).ok();
        CowInlineString::Owned(acc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast, new_style, tui_color, width};

    #[test]
    fn test_contains_ansi_escape_sequence() {
        use crate::assert_eq2;

        assert_eq2!(
            contains_ansi_escape_sequence(
                "\x1b[31mThis is red text.\x1b[0m And this is normal text."
            ),
            true
        );

        assert_eq2!(contains_ansi_escape_sequence("This is normal text."), false);

        assert_eq2!(
        contains_ansi_escape_sequence(
            &ast(
                "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
                new_style!(
                    bold italic underline
                    color_fg: {tui_color!(50, 50, 50)}
                    color_bg: {tui_color!(100, 200, 1)}
                ),
            )
            .to_string()
        ),
        true
    );
    }

    #[test]
    fn test_truncate_or_pad_from_right() {
        let long_string = "Hello, world!";
        let short_string = "Hi!";
        let width = width(10);

        // Test ASCII truncation.
        assert_eq!(
            truncate_from_right(long_string, width, true).as_ref(),
            "Hello, wo…"
        );
        assert_eq!(
            truncate_from_right(short_string, width, true).as_ref(),
            "Hi!       "
        );

        assert_eq!(
            truncate_from_right(long_string, width, false).as_ref(),
            "Hello, wo…"
        );
        assert_eq!(
            truncate_from_right(short_string, width, false).as_ref(),
            "Hi!"
        );

        // Test that no allocation occurs when no processing needed.
        let result = truncate_from_right(short_string, width, false);
        assert!(matches!(result, CowInlineString::Borrowed(_)));

        // Test Unicode truncation.
        let unicode_string = "Hello, 世界!";
        let result = truncate_from_right(unicode_string, width, false);
        assert_eq!(result.as_ref(), "Hello, 世…");
    }

    #[test]
    fn test_truncate_or_pad_from_left() {
        let long_string = "Hello, world!";
        let short_string = "Hi!";
        let width = width(10);

        // Test ASCII truncation.
        assert_eq!(
            truncate_from_left(long_string, width, true).as_ref(),
            "…o, world!"
        );
        assert_eq!(
            truncate_from_left(short_string, width, true).as_ref(),
            "       Hi!"
        );

        assert_eq!(
            truncate_from_left(long_string, width, false).as_ref(),
            "…o, world!"
        );
        assert_eq!(
            truncate_from_left(short_string, width, false).as_ref(),
            "Hi!"
        );

        // Test that no allocation occurs when no processing needed.
        let result = truncate_from_left(short_string, width, false);
        assert!(matches!(result, CowInlineString::Borrowed(_)));

        // Test Unicode truncation.
        let unicode_string = "Hello, 世界!";
        let result = truncate_from_left(unicode_string, width, false);
        assert_eq!(result.as_ref(), "…lo, 世界!");
    }
}

#[cfg(test)]
mod bench_tests {
    extern crate test;
    use test::Bencher;

    use super::*;
    use crate::width;

    #[bench]
    fn bench_truncate_ascii_no_truncation_no_pad(b: &mut Bencher) {
        let text = "Hello, world!";
        let display_width = width(20);
        b.iter(|| {
            let result = truncate_from_right(text, display_width, false);
            test::black_box(result);
        });
    }

    #[bench]
    fn bench_truncate_ascii_with_truncation(b: &mut Bencher) {
        let text = "Hello, world! This is a long string that needs truncation";
        let display_width = width(20);
        b.iter(|| {
            let result = truncate_from_right(text, display_width, false);
            test::black_box(result);
        });
    }

    #[bench]
    fn bench_truncate_ascii_with_padding(b: &mut Bencher) {
        let text = "Hi!";
        let display_width = width(20);
        b.iter(|| {
            let result = truncate_from_right(text, display_width, true);
            test::black_box(result);
        });
    }

    #[bench]
    fn bench_truncate_unicode_no_truncation(b: &mut Bencher) {
        let text = "Hello, 世界!";
        let display_width = width(20);
        b.iter(|| {
            let result = truncate_from_right(text, display_width, false);
            test::black_box(result);
        });
    }

    #[bench]
    fn bench_truncate_unicode_with_truncation(b: &mut Bencher) {
        let text =
            "Hello, 世界! This is a long string with unicode that needs truncation";
        let display_width = width(20);
        b.iter(|| {
            let result = truncate_from_right(text, display_width, false);
            test::black_box(result);
        });
    }

    #[test]
    fn test_zero_copy_optimization() {
        // Test that we get a borrowed reference when no processing is needed
        let text = "Hello, world!";
        let display_width = width(13); // Exact length, no padding
        let result = truncate_from_right(text, display_width, false);

        match result {
            CowInlineString::Borrowed(_) => {
                // Good, zero-copy optimization is working
            }
            CowInlineString::Owned(_) => {
                panic!("Expected borrowed reference for zero-copy optimization");
            }
        }
    }
}

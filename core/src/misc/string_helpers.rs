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

use crate::{ColWidth,
            GCStringExt as _,
            InlineString,
            glyphs::{ELLIPSIS_GLYPH, SPACER_GLYPH}};

/// Tests whether the given text contains an ANSI escape sequence.
pub fn contains_ansi_escape_sequence(text: &str) -> bool {
    text.chars().any(|it| it == '\x1b')
}

#[test]
fn test_contains_ansi_escape_sequence() {
    use crate::{ASTColor, ASTStyle, AnsiStyledText, assert_eq2};

    assert_eq2!(
        contains_ansi_escape_sequence(
            "\x1b[31mThis is red text.\x1b[0m And this is normal text."
        ),
        true
    );

    assert_eq2!(contains_ansi_escape_sequence("This is normal text."), false);

    assert_eq2!(
          contains_ansi_escape_sequence(
              &AnsiStyledText {
                  text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
                  style: smallvec::smallvec![
                      ASTStyle::Bold,
                      ASTStyle::Italic,
                      ASTStyle::Underline,
                      ASTStyle::Foreground(ASTColor::Rgb(50, 50, 50)),
                      ASTStyle::Background(ASTColor::Rgb(100, 200, 1)),
                  ],
              }
              .to_string()
          ),
          true
      );
}

/// Replace escaped quotes with unescaped quotes. The escaped quotes are generated
/// when [std::fmt::Debug] is used to format the output using [format!], eg:
/// ```
/// use r3bl_core::remove_escaped_quotes;
///
/// let s = format!("{:?}", "Hello\", world!");
/// assert_eq!(s, "\"Hello\\\", world!\"");
/// let s = remove_escaped_quotes(&s);
/// assert_eq!(s, "Hello, world!");
/// ```
pub fn remove_escaped_quotes(s: &str) -> String {
    s.replace("\\\"", "\"").replace("\"", "")
}

/// Take into account the fact that there maybe emoji in the string.
pub fn truncate_from_right(
    string: &str,
    arg_width: impl Into<ColWidth>,
    pad: bool,
) -> InlineString {
    let display_width = arg_width.into();
    let string_gcs = string.grapheme_string();
    let string_display_width = string_gcs.display_width;

    // Handle truncation.
    if string_display_width > display_width {
        let postfix = ELLIPSIS_GLYPH;
        let postfix_gcs = postfix.grapheme_string();
        let postfix_display_width = postfix_gcs.display_width;
        let truncate_cols_from_right = string_display_width - display_width;
        let truncated_text =
            string_gcs.trunc_end_by(truncate_cols_from_right + postfix_display_width);

        let mut acc = InlineString::new();
        _ = write!(acc, "{}{}", truncated_text, postfix_gcs.string);
        acc
    }
    // Handle padding.
    else if pad {
        use crate::glyphs::SPACER_GLYPH as SPACER;
        string_gcs.pad_end_to_fit(SPACER, display_width)
    }
    // No post processing needed.
    else {
        string.into()
    }
}

pub fn truncate_from_left(
    string: &str,
    arg_width: impl Into<ColWidth>,
    pad: bool,
) -> InlineString {
    let display_width = arg_width.into();
    let string_gcs = string.grapheme_string();
    let string_display_width = string_gcs.display_width;

    // Handle truncation.
    if string_display_width > display_width {
        let prefix = ELLIPSIS_GLYPH;
        let prefix_gcs = prefix.grapheme_string();
        let prefix_display_width = prefix_gcs.display_width;
        let truncate_cols_from_left = string_display_width - display_width;
        let truncated_text =
            string_gcs.trunc_start_by(truncate_cols_from_left + prefix_display_width);

        let mut acc = InlineString::new();
        _ = write!(acc, "{}{}", prefix_gcs.string, truncated_text);
        acc
    }
    // Handle padding.
    else if pad {
        string_gcs.pad_start_to_fit(SPACER_GLYPH, display_width)
    } else {
        string.into()
    }
}

#[cfg(test)]
mod tests_truncate_or_pad {
    use super::*;
    use crate::width;

    #[test]
    fn test_truncate_or_pad_from_right() {
        let long_string = "Hello, world!";
        let short_string = "Hi!";
        let width = width(10);

        assert_eq!(truncate_from_right(long_string, width, true), "Hello, wo…");
        assert_eq!(truncate_from_right(short_string, width, true), "Hi!       ");

        assert_eq!(truncate_from_right(long_string, width, false), "Hello, wo…");
        assert_eq!(truncate_from_right(short_string, width, false), "Hi!");
    }

    #[test]
    fn test_truncate_or_pad_from_left() {
        let long_string = "Hello, world!";
        let short_string = "Hi!";
        let width = width(10);

        assert_eq!(truncate_from_left(long_string, width, true), "…o, world!");
        assert_eq!(truncate_from_left(short_string, width, true), "       Hi!");

        assert_eq!(truncate_from_left(long_string, width, false), "…o, world!");
        assert_eq!(truncate_from_left(short_string, width, false), "Hi!");
    }
}

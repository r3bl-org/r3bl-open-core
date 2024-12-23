/*
 *   Copyright (c) 2024 R3BL LLC
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

use crate::{ChUnit,
            ELLIPSIS_GLYPH,
            UnicodeStringExt,
            ch,
            f64,
            glyphs::SPACER_GLYPH as SPACER,
            usize};

/// Tests whether the given text contains an ANSI escape sequence.
pub fn contains_ansi_escape_sequence(text: &str) -> bool {
    text.chars().any(|it| it == '\x1b')
}

#[test]
fn test_contains_ansi_escape_sequence() {
    use r3bl_ansi_color::{AnsiStyledText, Color, Style};

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
            &AnsiStyledText {
                text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
                style: &[
                    Style::Bold,
                    Style::Italic,
                    Style::Underline,
                    Style::Foreground(Color::Rgb(50, 50, 50)),
                    Style::Background(Color::Rgb(100, 200, 1)),
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
    display_width: impl Into<ChUnit>,
    pad: bool,
) -> String {
    use crate::glyphs::SPACER_GLYPH as SPACER;

    let display_width = display_width.into();
    let string = string.unicode_string();
    let string_display_width = string.display_width;

    // Handle truncation.
    if string_display_width > display_width {
        let suffix = ELLIPSIS_GLYPH.unicode_string();
        let suffix_display_width = suffix.display_width;

        let trunc_string = string.truncate_to_fit_size(crate::size!(
            col_count: display_width - suffix_display_width,
            row_count: 1
        ));
        // The above statement is Equivalent to:
        // let trunc_string =
        //     string.truncate_end_by_n_col(display_width - suffix_display_width - 1);

        format!("{}{}", trunc_string, suffix.string)
    }
    // Handle padding.
    else if pad {
        let mut padded_string = string.to_string();
        let display_width_to_pad = display_width - string_display_width;
        let display_width_to_pad = f64(display_width_to_pad);
        let spacer_display_width = SPACER.unicode_string().display_width;
        let spacer_display_width = f64(spacer_display_width);
        let repeat_count = (display_width_to_pad / spacer_display_width).ceil() as usize;
        padded_string.push_str(&SPACER.repeat(repeat_count));
        padded_string
    }
    // No post processing needed.
    else {
        string.to_string()
    }
}

pub fn truncate_from_left(text: &str, display_width: usize, pad: bool) -> String {
    let display_width = ch(display_width);
    let text = text.unicode_string();
    let text_width = text.display_width;

    if text_width > display_width {
        let suffix = ELLIPSIS_GLYPH.unicode_string();
        let suffix_width = suffix.display_width;

        let truncate_cols_from_left = text_width - display_width;
        let truncated_text =
            text.truncate_start_by_n_col(truncate_cols_from_left + suffix_width);

        format!("{}{}", suffix.string, truncated_text)
    } else if pad {
        let mut padded_text = text.to_string();
        let width_to_pad = display_width - text_width;
        let spacer_width = SPACER.unicode_string().display_width;
        let repeat_count = (f64(width_to_pad) / f64(spacer_width)).ceil();
        let repeat_count = ch(repeat_count);
        padded_text.insert_str(0, &SPACER.repeat(usize(repeat_count)));
        padded_text
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests_truncate_or_pad {
    use super::*;

    #[test]
    fn test_truncate_or_pad_from_right() {
        let long_string = "Hello, world!";
        let short_string = "Hi!";
        let width = 10;

        assert_eq!(truncate_from_right(long_string, width, true), "Hello, wo…");
        assert_eq!(truncate_from_right(short_string, width, true), "Hi!       ");

        assert_eq!(truncate_from_right(long_string, width, false), "Hello, wo…");
        assert_eq!(truncate_from_right(short_string, width, false), "Hi!");
    }

    #[test]
    fn test_truncate_or_pad_from_left() {
        let long_string = "Hello, world!";
        let short_string = "Hi!";
        let width = 10;

        assert_eq!(truncate_from_left(long_string, width, true), "…o, world!");
        assert_eq!(truncate_from_left(short_string, width, true), "       Hi!");

        assert_eq!(truncate_from_left(long_string, width, false), "…o, world!");
        assert_eq!(truncate_from_left(short_string, width, false), "Hi!");
    }
}
